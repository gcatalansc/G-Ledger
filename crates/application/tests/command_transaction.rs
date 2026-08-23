use project_ledger_application::{
    CommandDraft, CommandHandler, CommandHandlerError, CommandProcessor, CreateProjectHandler,
    EventDraft, FixedClock, prepare_command,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult,
    CommandResultStatus, CreateProjectPayload, ErrorCode, Event, EventId, LedgerTimestamp, Project,
    ProjectId, ProjectLifecycleStatus, ProjectPausedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn ac_036_same_scope_key_and_digest_has_one_effect() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(timestamp());
    let command = create_command("Ledger", 0, "idempotency:create");

    let first = CommandProcessor::new(&mut store, &clock)
        .execute(command.clone(), &CreateProjectHandler)
        .unwrap();
    let replay = CommandProcessor::new(&mut store, &clock)
        .execute(command, &CreateProjectHandler)
        .unwrap();

    assert_eq!(first.status, CommandResultStatus::Accepted);
    assert_eq!(first.previous_state_version, 0);
    assert_eq!(first.new_state_version, 1);
    assert_eq!(first.emitted_event_refs.len(), 1);
    assert!(!first.idempotency.replayed);
    assert_eq!(replay.status, CommandResultStatus::DuplicateReplay);
    assert!(replay.idempotency.replayed);
    assert_eq!(replay.emitted_event_refs, first.emitted_event_refs);
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

#[test]
fn ac_037_same_scope_key_with_different_digest_fails_closed() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(timestamp());
    let first = create_command("Ledger", 0, "idempotency:conflict");
    let mut different = create_command("Different", 1, "idempotency:conflict");
    different.command_id = first.command_id.clone();
    different = prepare_from_envelope(different);

    CommandProcessor::new(&mut store, &clock)
        .execute(first, &CreateProjectHandler)
        .unwrap();
    let conflict = CommandProcessor::new(&mut store, &clock)
        .execute(different, &CreateProjectHandler)
        .unwrap();

    assert_eq!(conflict.status, CommandResultStatus::Rejected);
    assert_eq!(
        conflict.rejection.as_ref().unwrap().code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

#[test]
fn ac_038_stale_expected_version_returns_conflict_without_append() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(timestamp());
    CommandProcessor::new(&mut store, &clock)
        .execute(
            create_command("Ledger", 0, "idempotency:create"),
            &CreateProjectHandler,
        )
        .unwrap();

    let stale = CommandProcessor::new(&mut store, &clock)
        .execute(
            create_command("Ledger again", 0, "idempotency:stale"),
            &CreateProjectHandler,
        )
        .unwrap();

    assert_eq!(stale.status, CommandResultStatus::VersionConflict);
    assert_eq!(
        stale.rejection.as_ref().unwrap().code,
        ErrorCode::VersionConflict
    );
    assert_eq!(stale.previous_state_version, 1);
    assert_eq!(stale.new_state_version, 1);
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
    assert_eq!(
        store
            .command_result(&stale.command_ref)
            .unwrap()
            .unwrap()
            .status,
        CommandResultStatus::VersionConflict
    );
}

#[test]
fn ac_039_forbidden_command_event_pair_is_rejected_before_storage() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(timestamp());
    let result = CommandProcessor::new(&mut store, &clock)
        .execute(
            create_command("Ledger", 0, "idempotency:forbidden"),
            &ForbiddenCreateHandler,
        )
        .unwrap();

    assert_eq!(result.status, CommandResultStatus::IncompatibleContract);
    assert_eq!(
        result.rejection.as_ref().unwrap().code,
        ErrorCode::CommandEventContractViolation
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), 0);
}

#[test]
fn ac_040_transport_unknown_is_not_a_server_command_result_status() {
    let json = serde_json::json!({
        "schema_version": "0.2.0",
        "command_result_id": "result:test",
        "command_ref": "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "status": "OUTCOME_UNKNOWN",
        "previous_state_version": 0,
        "new_state_version": 0,
        "emitted_event_refs": [],
        "resulting_checkpoint_ref": null,
        "rejection": null,
        "idempotency": { "replayed": false },
        "processed_at": "2026-08-23T12:00:00-04:00",
        "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    });

    assert!(serde_json::from_value::<CommandResult>(json).is_err());
}

#[derive(Debug)]
struct ForbiddenCreateHandler;

impl CommandHandler for ForbiddenCreateHandler {
    fn handle(&self, command: &CommandEnvelope) -> Result<Vec<EventDraft>, CommandHandlerError> {
        Ok(vec![EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectPaused(ProjectPausedEvent {
                reason: "forbidden".to_owned(),
            }),
            correlation_ref: StableId::parse("correlation:forbidden").unwrap(),
            occurred_at: command.submitted_at.clone(),
            recorded_at: command.submitted_at.clone(),
        }])
    }
}

fn create_command(title: &str, expected_version: u64, key: &str) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: expected_version,
        idempotency_key: StableId::parse(key).unwrap(),
        actor: actor(),
        command: Command::CreateProject(CreateProjectPayload {
            project: project(title),
        }),
        submitted_at: timestamp(),
    })
    .unwrap()
}

fn prepare_from_envelope(command: CommandEnvelope) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: command.schema_version,
        command_id: command.command_id,
        project_ref: command.project_ref,
        expected_state_version: command.expected_state_version,
        idempotency_key: command.idempotency_key,
        actor: command.actor,
        command: command.command,
        submitted_at: command.submitted_at,
    })
    .unwrap()
}

fn actor() -> Actor {
    Actor {
        actor_id: StableId::parse("actor:test").unwrap(),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn project(title: &str) -> Project {
    Project {
        project_id: project_id(),
        title: title.to_owned(),
        summary: "Command transaction".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}
