use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel, UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{
    CommandCommit, CommandDraft, CommandResultDraft, CommitDisposition, EventDraft, EventStore,
    StoreError, build_command_result, build_event, prepare_command,
};
use project_ledger_storage_sqlite::SqliteEventStore;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn genesis_and_second_event_roundtrip_as_a_gap_free_chain() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();

    let first = create_commit();
    let first_digest = first.events[0].event_digest;
    assert!(matches!(
        store.commit_command(first).unwrap(),
        CommitDisposition::Committed(_)
    ));

    let second = update_commit(1, Some(first_digest));
    assert!(matches!(
        store.commit_command(second).unwrap(),
        CommitDisposition::Committed(_)
    ));

    let events = store.load_stream(&project_id()).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].stream_version, 1);
    assert_eq!(events[0].previous_event_digest, None);
    assert_eq!(events[1].stream_version, 2);
    assert_eq!(events[1].previous_event_digest, Some(first_digest));
    assert_eq!(store.current_version(&project_id()).unwrap(), 2);
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
    store.verify_event_chain(&project_id()).unwrap();

    // Reads and audits must not mutate canonical truth.
    store.load_stream(&project_id()).unwrap();
    store.verify_event_chain(&project_id()).unwrap();
    assert_eq!(store.current_version(&project_id()).unwrap(), 2);
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
}

#[test]
fn append_rejects_missing_genesis_gap_wrong_prior_digest_and_duplicate_version() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();

    let missing_genesis = update_commit(0, None);
    assert!(matches!(
        store.commit_command(missing_genesis),
        Err(StoreError::InvalidEventSequence { .. })
    ));

    let first = create_commit();
    let first_digest = first.events[0].event_digest;
    store.commit_command(first).unwrap();

    let gap = update_commit(2, Some(first_digest));
    assert!(matches!(
        store.commit_command(gap),
        Err(StoreError::StreamVersionConflict {
            expected: 2,
            actual: 1
        })
    ));

    let wrong_prior = update_commit(
        1,
        Some(project_ledger_canonical_json::Sha256Digest::from([7; 32])),
    );
    assert!(matches!(
        store.commit_command(wrong_prior),
        Err(StoreError::InvalidEventSequence { .. })
    ));

    let duplicate_version = update_commit(0, None);
    assert!(matches!(
        store.commit_command(duplicate_version),
        Err(StoreError::StreamVersionConflict {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

#[test]
fn defensive_triggers_reject_mutation_and_verification_fails_after_controlled_tampering() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();

    let raw = Connection::open(&path).unwrap();
    assert!(
        raw.execute("UPDATE events SET recorded_at = 'tampered'", [])
            .is_err()
    );
    assert!(raw.execute("DELETE FROM events", []).is_err());

    // Controlled corruption simulates damaged storage below the public API.
    raw.execute_batch("DROP TRIGGER events_forbid_update")
        .unwrap();
    raw.execute("UPDATE events SET event_json = X'7B7D'", [])
        .unwrap();
    drop(raw);

    assert!(matches!(
        store.verify_event_chain(&project_id()),
        Err(StoreError::CorruptStream { .. })
    ));
}

#[test]
fn verification_detects_tampering_in_redundant_indexed_event_metadata() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("DROP TRIGGER events_forbid_update")
        .unwrap();
    raw.execute(
        "UPDATE events SET payload_digest =
         'sha256:0000000000000000000000000000000000000000000000000000000000000000'",
        [],
    )
    .unwrap();
    drop(raw);

    assert!(matches!(
        store.verify_event_chain(&project_id()),
        Err(StoreError::CorruptStream { .. })
    ));
}

fn create_commit() -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("chain:create").unwrap(),
        actor: actor(),
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(),
    })
    .unwrap();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            correlation_ref: StableId::parse("correlation:create").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    accepted_commit(command, vec![event])
}

fn update_commit(
    expected_version: u64,
    prior: Option<project_ledger_canonical_json::Sha256Digest>,
) -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: expected_version,
        idempotency_key: StableId::parse(&format!(
            "chain:update:{expected_version}:{}",
            prior.is_some()
        ))
        .unwrap(),
        actor: actor(),
        command: Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some("Updated".to_owned()),
            summary: None,
        }),
        submitted_at: timestamp(),
    })
    .unwrap();
    let event = build_event(
        &command,
        expected_version + 1,
        prior,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                title: Some("Updated".to_owned()),
                summary: None,
            }),
            correlation_ref: StableId::parse("correlation:update").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    accepted_commit(command, vec![event])
}

fn accepted_commit(
    command: project_ledger_domain::CommandEnvelope,
    events: Vec<project_ledger_domain::EventEnvelope>,
) -> CommandCommit {
    let result = build_command_result(CommandResultDraft {
        command_result_id: StableId::parse(&format!("result:{}", command.command_id.uuid()))
            .unwrap(),
        command_ref: command.command_id.clone(),
        status: CommandResultStatus::Accepted,
        previous_state_version: command.expected_state_version,
        new_state_version: command.expected_state_version + events.len() as u64,
        emitted_event_refs: events.iter().map(|event| event.event_id.clone()).collect(),
        resulting_checkpoint_ref: None,
        rejection: None,
        replayed: false,
        processed_at: timestamp(),
    })
    .unwrap();
    CommandCommit {
        command,
        events,
        result,
    }
}

fn actor() -> Actor {
    Actor {
        actor_id: StableId::parse("actor:test").unwrap(),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Ledger test".to_owned(),
        summary: "Event chain".to_owned(),
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
