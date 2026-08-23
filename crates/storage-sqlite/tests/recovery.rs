use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{
    CommandCommit, CommandDraft, CommandResultDraft, EventDraft, EventStore, StoreError,
    build_command_result, build_event, prepare_command,
};
use project_ledger_storage_sqlite::SqliteEventStore;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn wal_rollback_and_reopen_preserve_the_last_committed_truth() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();
    drop(store);

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(
        "BEGIN IMMEDIATE;
         INSERT INTO commands(
             command_id, project_ref, command_type, actor_id, trust_domain,
             idempotency_key, expected_state_version, command_digest, command_json, submitted_at
         ) VALUES (
             'command:rollback', 'project:rollback', 'CREATE_PROJECT', 'actor:test',
             'LOCAL_USER', 'rollback', 0,
             'sha256:0000000000000000000000000000000000000000000000000000000000000000',
             X'7B7D', '2026-08-23T12:00:00-04:00'
         );
         ROLLBACK;",
    )
    .unwrap();
    drop(raw);

    let reopened = SqliteEventStore::open(&path).unwrap();
    let report = reopened.recovery_report(&project_id()).unwrap();
    assert_eq!(report.quick_check, "ok");
    assert!(report.chain_verified);
    assert_eq!(report.current_version, 1);
    assert_eq!(report.event_count, 1);
    assert_eq!(report.command_count, 1);
    assert_eq!(report.command_result_count, 1);
    assert_eq!(report.unresolved_submission_count, 0);
}

#[test]
fn recovery_fails_closed_when_canonical_event_bytes_are_corrupt() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();
    drop(store);

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TRIGGER events_forbid_update")
        .unwrap();
    raw.execute("UPDATE events SET event_json = X'7B7D'", [])
        .unwrap();
    drop(raw);

    let reopened = SqliteEventStore::open(&path).unwrap();
    assert!(matches!(
        reopened.recovery_report(&project_id()),
        Err(StoreError::CorruptStream { .. })
    ));
}

#[test]
fn recovery_fails_closed_when_a_command_result_no_longer_matches_its_digest() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();
    drop(store);

    let raw = Connection::open(&path).unwrap();
    let bytes: Vec<u8> = raw
        .query_row("SELECT result_json FROM command_results", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    result["new_state_version"] = serde_json::json!(9);
    raw.execute(
        "UPDATE command_results SET result_json = ?1",
        [serde_json::to_vec(&result).unwrap()],
    )
    .unwrap();
    drop(raw);

    let reopened = SqliteEventStore::open(&path).unwrap();
    assert!(reopened.recovery_report(&project_id()).is_err());
}

fn create_commit() -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("recovery:create").unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:test").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(),
    })
    .unwrap();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            correlation_ref: StableId::parse("correlation:recovery").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    let result = build_command_result(CommandResultDraft {
        command_result_id: StableId::parse("result:recovery").unwrap(),
        command_ref: command.command_id.clone(),
        status: CommandResultStatus::Accepted,
        previous_state_version: 0,
        new_state_version: 1,
        emitted_event_refs: vec![event.event_id.clone()],
        resulting_checkpoint_ref: None,
        rejection: None,
        replayed: false,
        processed_at: timestamp(),
    })
    .unwrap();
    CommandCommit {
        command,
        events: vec![event],
        result,
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Recovery".to_owned(),
        summary: "WAL rollback".to_owned(),
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
