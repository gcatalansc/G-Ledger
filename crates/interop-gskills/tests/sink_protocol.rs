mod sink_support;

use project_ledger_interop_gskills::{
    GSkillsRecordSink, RecordSinkError, validate_gskills_contract,
};

use sink_support::{
    ProcessingHost, change_work_summary_and_resign, policy, unknown_command, work_progress_command,
};

#[test]
fn submit_replay_status_and_reconcile_preserve_one_native_effect() {
    let host = ProcessingHost::new();
    let command = work_progress_command(host.current_state());
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let baseline_events = host.event_count();

    let first = sink.submit(&command).unwrap();
    let replay = sink.submit(&command).unwrap();

    validate_gskills_contract("external-record-result", &first).unwrap();
    validate_gskills_contract("external-record-result", &replay).unwrap();
    assert_eq!(first["status"], "ACCEPTED");
    assert_eq!(replay["status"], "DUPLICATE_REPLAY");
    assert_eq!(first["idempotency"]["replayed"], false);
    assert_eq!(replay["idempotency"]["replayed"], true);
    assert_eq!(host.event_count(), baseline_events + 1);
    assert_eq!(host.execute_calls(), 2);

    let status = sink.status(&command).unwrap();
    let reconciled = sink.reconcile(&command).unwrap();
    assert_eq!(status["status"], "FOUND");
    assert_eq!(reconciled["status"], "FOUND");
    assert_eq!(
        reconciled["result"]["native_result_digest"],
        first["native_result_digest"]
    );
    assert_eq!(host.execute_calls(), 2);
    assert_eq!(host.event_count(), baseline_events + 1);
}

#[test]
fn same_scope_key_with_changed_command_is_rejected_without_duplicate_effect() {
    let host = ProcessingHost::new();
    let mut command = work_progress_command(host.current_state());
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let baseline_events = host.event_count();
    assert_eq!(sink.submit(&command).unwrap()["status"], "ACCEPTED");

    change_work_summary_and_resign(
        &mut command,
        "Different command under the same scope and key",
    );
    let conflict = sink.submit(&command).unwrap();

    assert_eq!(conflict["status"], "REJECTED");
    assert_eq!(conflict["rejection"]["code"], "IDEMPOTENCY_CONFLICT");
    assert_eq!(host.event_count(), baseline_events + 1);
}

#[test]
fn lost_submit_response_is_reconciled_from_durable_result_without_retry() {
    let host = ProcessingHost::new();
    let command = work_progress_command(host.current_state());
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let baseline_events = host.event_count();
    host.lose_next_response();

    assert!(matches!(
        sink.submit(&command),
        Err(RecordSinkError::Host(_))
    ));
    assert_eq!(host.event_count(), baseline_events + 1);
    assert_eq!(host.execute_calls(), 1);

    let reconciled = sink.reconcile(&command).unwrap();
    assert_eq!(reconciled["status"], "FOUND");
    assert_eq!(reconciled["result"]["status"], "ACCEPTED");
    assert_eq!(host.execute_calls(), 1);
    assert_eq!(host.event_count(), baseline_events + 1);
}

#[test]
fn status_and_reconcile_not_found_never_submit() {
    let host = ProcessingHost::new();
    let command = unknown_command(host.current_state());
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let baseline_events = host.event_count();

    assert_eq!(sink.status(&command).unwrap()["status"], "NOT_FOUND");
    assert_eq!(sink.reconcile(&command).unwrap()["status"], "NOT_FOUND");
    assert_eq!(host.execute_calls(), 0);
    assert_eq!(host.event_count(), baseline_events);
}

#[test]
fn version_race_returns_native_conflict_without_overwrite() {
    let host = ProcessingHost::new();
    let prepared_state = host.current_state();
    let command = work_progress_command(prepared_state);
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();

    host.advance_out_of_band();
    host.override_reported_state(prepared_state);
    let before = host.event_count();
    let conflict = sink.submit(&command).unwrap();
    host.clear_reported_state_override();

    assert_eq!(conflict["status"], "VERSION_CONFLICT");
    assert_eq!(host.event_count(), before);
}
