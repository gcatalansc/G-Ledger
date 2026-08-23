use project_ledger_application::{
    CommandDraft, CommandResultDraft, EventDraft, ProjectStateQuery, ProjectionSource,
    build_command_result, build_event, prepare_command,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel, UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{CommandCommit, EventStore};
use project_ledger_storage_sqlite::SqliteEventStore;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn ac_007_and_pl_09_rebuild_then_serve_one_hundred_nonmutating_queries() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    store.commit_command(create_commit()).unwrap();
    let before_version = store.current_version(&project_id()).unwrap();
    let before_count = store.event_count(&project_id()).unwrap();

    let rebuilt = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();
    assert_eq!(rebuilt.source, ProjectionSource::Rebuilt);
    assert_eq!(rebuilt.state.state_version, 1);

    for _ in 0..100 {
        let cached = ProjectStateQuery::new(&mut store)
            .load(&project_id())
            .unwrap();
        assert_eq!(cached.source, ProjectionSource::Snapshot);
        assert_eq!(cached.state, rebuilt.state);
    }
    assert_eq!(
        store.current_version(&project_id()).unwrap(),
        before_version
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), before_count);
}

#[test]
fn stale_snapshot_is_rebuilt_from_the_new_canonical_head() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let first = create_commit();
    let first_digest = first.events[0].event_digest;
    store.commit_command(first).unwrap();
    ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();

    store.commit_command(metadata_commit(first_digest)).unwrap();
    let recovered = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();

    assert_eq!(recovered.source, ProjectionSource::Rebuilt);
    assert_eq!(recovered.state.state_version, 2);
    assert_eq!(recovered.state.project.title, "Updated from event");
    assert_eq!(store.current_version(&project_id()).unwrap(), 2);
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
    assert_eq!(
        ProjectStateQuery::new(&mut store)
            .load(&project_id())
            .unwrap()
            .source,
        ProjectionSource::Snapshot
    );
}

#[test]
fn ac_009_and_ac_010_corrupt_snapshot_is_discarded_rebuilt_and_repaired() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(create_commit()).unwrap();
    let expected = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap()
        .state;

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE project_state_snapshots SET state_json = X'7B7D' WHERE project_ref = ?1",
        [project_id().as_str()],
    )
    .unwrap();
    drop(raw);
    let before_version = store.current_version(&project_id()).unwrap();
    let before_count = store.event_count(&project_id()).unwrap();

    let recovered = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();
    assert_eq!(recovered.source, ProjectionSource::Rebuilt);
    assert_eq!(recovered.state, expected);
    assert_eq!(
        store.current_version(&project_id()).unwrap(),
        before_version
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), before_count);
    assert_eq!(
        ProjectStateQuery::new(&mut store)
            .load(&project_id())
            .unwrap()
            .source,
        ProjectionSource::Snapshot
    );
}

fn create_commit() -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("query:create").unwrap(),
        actor: actor(),
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp("2026-08-23T12:00:00-04:00"),
    })
    .unwrap();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            correlation_ref: StableId::parse("correlation:query-create").unwrap(),
            occurred_at: timestamp("2026-08-23T12:00:00-04:00"),
            recorded_at: timestamp("2026-08-23T12:00:00-04:00"),
        },
    )
    .unwrap();
    accepted_commit(command, vec![event])
}

fn metadata_commit(prior: project_ledger_canonical_json::Sha256Digest) -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: 1,
        idempotency_key: StableId::parse("query:metadata").unwrap(),
        actor: actor(),
        command: Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some("Updated from event".to_owned()),
            summary: None,
        }),
        submitted_at: timestamp("2026-08-23T12:05:00-04:00"),
    })
    .unwrap();
    let event = build_event(
        &command,
        2,
        Some(prior),
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                title: Some("Updated from event".to_owned()),
                summary: None,
            }),
            correlation_ref: StableId::parse("correlation:query-metadata").unwrap(),
            occurred_at: timestamp("2026-08-23T12:05:00-04:00"),
            recorded_at: timestamp("2026-08-23T12:05:00-04:00"),
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
        processed_at: command.submitted_at.clone(),
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
        title: "State query".to_owned(),
        summary: "Canonical rebuild".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp("2026-08-23T12:00:00-04:00"),
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

fn timestamp(value: &str) -> LedgerTimestamp {
    LedgerTimestamp::parse(value).unwrap()
}
