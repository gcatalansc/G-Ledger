use project_ledger_application::{
    CommandDraft, CommandProcessor, CreateProjectHandler, ExternalSubmissionDraft, FixedClock,
    ReconciliationOutcome, SubmissionError, SubmissionService, build_external_submission,
    prepare_command,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CreateProjectPayload,
    ExternalSubmissionRecord, ExternalSubmissionState, LedgerTimestamp, Project, ProjectId,
    ProjectLifecycleStatus, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn pl_08_unknown_outcome_is_persisted_without_fabricating_a_server_result() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let command = create_command(0, "command:unknown", "key:unknown");
    let submission = submitted_record(&command, "submission:unknown");
    let mut service = SubmissionService::new(&mut store);
    service.record(submission).unwrap();

    let unknown = service
        .mark_outcome_unknown(
            &StableId::parse("submission:unknown").unwrap(),
            "TIMEOUT_AFTER_SEND",
            &timestamp(),
        )
        .unwrap();

    assert_eq!(unknown.state, ExternalSubmissionState::OutcomeUnknown);
    assert_eq!(
        unknown.last_transport_error.as_deref(),
        Some("TIMEOUT_AFTER_SEND")
    );
    assert_eq!(unknown.known_result_ref, None);
    assert!(
        service
            .command_result(&command.command_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        states(service.submission_history(&unknown.submission_id).unwrap()),
        vec![
            ExternalSubmissionState::Submitted,
            ExternalSubmissionState::OutcomeUnknown
        ]
    );
}

#[test]
fn pl_20_reconciliation_maps_durable_accepted_and_rejected_results() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(timestamp());

    let accepted_command = create_command(0, "command:accepted", "key:accepted");
    CommandProcessor::new(&mut store, &clock)
        .execute(accepted_command.clone(), &CreateProjectHandler)
        .unwrap();

    let rejected_command = create_command(0, "command:rejected", "key:rejected");
    CommandProcessor::new(&mut store, &clock)
        .execute(rejected_command.clone(), &CreateProjectHandler)
        .unwrap();

    let mut service = SubmissionService::new(&mut store);
    service
        .record(unknown_record(&accepted_command, "submission:accepted"))
        .unwrap();
    service
        .record(unknown_record(&rejected_command, "submission:rejected"))
        .unwrap();

    let accepted = service
        .reconcile(
            &StableId::parse("submission:accepted").unwrap(),
            &timestamp(),
        )
        .unwrap();
    let rejected = service
        .reconcile(
            &StableId::parse("submission:rejected").unwrap(),
            &timestamp(),
        )
        .unwrap();

    assert!(matches!(accepted, ReconciliationOutcome::Accepted { .. }));
    assert!(matches!(rejected, ReconciliationOutcome::Rejected { .. }));
    assert_eq!(
        service
            .submission(&StableId::parse("submission:accepted").unwrap())
            .unwrap()
            .unwrap()
            .state,
        ExternalSubmissionState::ReconciledAccepted
    );
    assert_eq!(
        service
            .submission(&StableId::parse("submission:rejected").unwrap())
            .unwrap()
            .unwrap()
            .state,
        ExternalSubmissionState::ReconciledRejected
    );
}

#[test]
fn not_found_leaves_unknown_and_does_not_append_history() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let command = create_command(0, "command:not-found", "key:not-found");
    let id = StableId::parse("submission:not-found").unwrap();
    let mut service = SubmissionService::new(&mut store);
    service
        .record(unknown_record(&command, id.as_str()))
        .unwrap();

    assert_eq!(
        service.reconcile(&id, &timestamp()).unwrap(),
        ReconciliationOutcome::NotFound
    );
    assert_eq!(
        service.submission(&id).unwrap().unwrap().state,
        ExternalSubmissionState::OutcomeUnknown
    );
    assert_eq!(service.submission_history(&id).unwrap().len(), 1);
}

#[test]
fn transitions_are_compare_and_set_and_exact_replay_does_not_duplicate_history() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let command = create_command(0, "command:cas", "key:cas");
    let submitted_id = StableId::parse("submission:cas").unwrap();
    let prepared_id = StableId::parse("submission:prepared").unwrap();
    let mut service = SubmissionService::new(&mut store);
    service
        .record(submitted_record(&command, submitted_id.as_str()))
        .unwrap();
    service
        .record(record_with_state(
            &command,
            prepared_id.as_str(),
            ExternalSubmissionState::Prepared,
        ))
        .unwrap();

    service
        .mark_outcome_unknown(&submitted_id, "TIMEOUT_AFTER_SEND", &timestamp())
        .unwrap();
    service
        .mark_outcome_unknown(&submitted_id, "TIMEOUT_AFTER_SEND", &timestamp())
        .unwrap();
    assert_eq!(service.submission_history(&submitted_id).unwrap().len(), 2);

    assert!(matches!(
        service.mark_outcome_unknown(&submitted_id, "DIFFERENT", &later_timestamp()),
        Err(SubmissionError::StateConflict { .. })
    ));
    assert!(matches!(
        service.mark_outcome_unknown(&prepared_id, "TIMEOUT", &timestamp()),
        Err(SubmissionError::IllegalTransition { .. })
    ));
}

fn submitted_record(command: &CommandEnvelope, submission_id: &str) -> ExternalSubmissionRecord {
    record_with_state(command, submission_id, ExternalSubmissionState::Submitted)
}

fn unknown_record(command: &CommandEnvelope, submission_id: &str) -> ExternalSubmissionRecord {
    record_with_state(
        command,
        submission_id,
        ExternalSubmissionState::OutcomeUnknown,
    )
}

fn record_with_state(
    command: &CommandEnvelope,
    submission_id: &str,
    state: ExternalSubmissionState,
) -> ExternalSubmissionRecord {
    build_external_submission(ExternalSubmissionDraft {
        submission_id: StableId::parse(submission_id).unwrap(),
        command_ref: command.command_id.clone(),
        idempotency_key: command.idempotency_key.clone(),
        state,
        attempt_count: u32::from(matches!(
            state,
            ExternalSubmissionState::Submitted | ExternalSubmissionState::OutcomeUnknown
        )),
        last_transport_error: (state == ExternalSubmissionState::OutcomeUnknown)
            .then(|| "TIMEOUT_AFTER_SEND".to_owned()),
        known_result_ref: None,
        prepared_at: timestamp(),
        submitted_at: matches!(
            state,
            ExternalSubmissionState::Submitted | ExternalSubmissionState::OutcomeUnknown
        )
        .then(timestamp),
        reconciled_at: None,
    })
    .unwrap()
}

fn states(records: Vec<ExternalSubmissionRecord>) -> Vec<ExternalSubmissionState> {
    records.into_iter().map(|record| record.state).collect()
}

fn create_command(expected_version: u64, command_id: &str, key: &str) -> CommandEnvelope {
    let command_id = match command_id {
        "command:accepted" => "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "command:rejected" => "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e",
        "command:unknown" => "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5f",
        "command:not-found" => "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df60",
        "command:cas" => "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df61",
        other => other,
    };
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(command_id).unwrap(),
        project_ref: project_id(),
        expected_state_version: expected_version,
        idempotency_key: StableId::parse(key).unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:test").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload {
            project: Project {
                project_id: project_id(),
                title: "Ledger".to_owned(),
                summary: "Submission reconciliation".to_owned(),
                lifecycle_status: ProjectLifecycleStatus::Initializing,
                created_at: timestamp(),
                timezone: "UTC".to_owned(),
                authority_policy: AuthorityPolicy::SpecFirst,
                storage_profile: StorageProfile::UserData,
                repository_binding_refs: Vec::new(),
                paused_reason: None,
                archived_at: None,
            },
        }),
        submitted_at: timestamp(),
    })
    .unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}

fn later_timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:01:00-04:00").unwrap()
}
