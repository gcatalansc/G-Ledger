mod sink_support;

use project_ledger_interop_gskills::{
    GSkillsRecordSink, GSkillsRecordSinkConformanceSuite, run_gskills_record_sink_conformance,
};

use sink_support::{ProcessingHost, policy, unknown_command, work_progress_command};

#[test]
fn exact_mutable_conformance_proves_no_duplicate_effect_or_blind_retry() {
    let host = ProcessingHost::new();
    let state = host.current_state();
    let suite = GSkillsRecordSinkConformanceSuite {
        command: work_progress_command(state),
        unknown_command: unknown_command(state),
    };
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();

    let report = run_gskills_record_sink_conformance(&sink, &suite).unwrap();

    assert!(report.checks.iter().all(|check| check.passed));
    assert_eq!(report.submissions, 2);
    assert_eq!(report.durable_lookups, 3);
    assert_eq!(report.duplicate_effects, 0);
    assert_eq!(report.blind_retries, 0);
    assert_eq!(report.transport_outcomes_in_domain_results, 0);
    assert_eq!(host.execute_calls(), 2);
    assert_eq!(host.event_count(), state.state_version + 1);
}
