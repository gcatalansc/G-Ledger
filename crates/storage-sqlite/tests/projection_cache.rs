use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{
    CommandCommit, CommandDraft, CommandResultDraft, EventDraft, EventStore, ProjectStateSnapshot,
    ProjectionSnapshotLoad, ProjectionSnapshotStore, build_command_result, build_event,
    prepare_command,
};
use project_ledger_projection_engine::{PROJECTOR_VERSION, ProjectStateProjector};
use project_ledger_storage_sqlite::SqliteEventStore;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn snapshot_cache_roundtrips_and_overwrites_without_touching_events() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    let commit = create_commit();
    let head = commit.events[0].event_digest;
    store.commit_command(commit).unwrap();
    let state = ProjectStateProjector::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();

    assert!(matches!(
        store.load_project_state_snapshot(&project_id()).unwrap(),
        ProjectionSnapshotLoad::Missing
    ));
    store
        .store_project_state_snapshot(&ProjectStateSnapshot {
            state: state.clone(),
            source_head_event_digest: head,
            projector_version: PROJECTOR_VERSION,
        })
        .unwrap();
    assert_eq!(
        store.load_project_state_snapshot(&project_id()).unwrap(),
        ProjectionSnapshotLoad::Snapshot(Box::new(ProjectStateSnapshot {
            state: state.clone(),
            source_head_event_digest: head,
            projector_version: PROJECTOR_VERSION,
        }))
    );

    store
        .store_project_state_snapshot(&ProjectStateSnapshot {
            state: state.clone(),
            source_head_event_digest: head,
            projector_version: PROJECTOR_VERSION + 1,
        })
        .unwrap();
    let ProjectionSnapshotLoad::Snapshot(overwritten) =
        store.load_project_state_snapshot(&project_id()).unwrap()
    else {
        panic!("snapshot must remain readable after overwrite");
    };
    assert_eq!(overwritten.projector_version, PROJECTOR_VERSION + 1);
    assert_eq!(store.current_version(&project_id()).unwrap(), 1);
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

#[test]
fn malformed_or_metadata_divergent_snapshot_is_classified_as_corrupt() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    let commit = create_commit();
    let head = commit.events[0].event_digest;
    store.commit_command(commit).unwrap();
    let state = ProjectStateProjector::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    store
        .store_project_state_snapshot(&ProjectStateSnapshot {
            state: state.clone(),
            source_head_event_digest: head,
            projector_version: PROJECTOR_VERSION,
        })
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE project_state_snapshots SET state_json = X'7B7D' WHERE project_ref = ?1",
        [project_id().as_str()],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        store.load_project_state_snapshot(&project_id()).unwrap(),
        ProjectionSnapshotLoad::Corrupt
    ));

    store
        .store_project_state_snapshot(&ProjectStateSnapshot {
            state,
            source_head_event_digest: head,
            projector_version: PROJECTOR_VERSION,
        })
        .unwrap();

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE project_state_snapshots SET state_digest =
         'sha256:0000000000000000000000000000000000000000000000000000000000000000'
         WHERE project_ref = ?1",
        [project_id().as_str()],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        store.load_project_state_snapshot(&project_id()).unwrap(),
        ProjectionSnapshotLoad::Corrupt
    ));
}

fn create_commit() -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("projection-cache:create").unwrap(),
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
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            correlation_ref: StableId::parse("correlation:projection-cache").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    let result = build_command_result(CommandResultDraft {
        command_result_id: StableId::parse(&format!("result:{}", command.command_id.uuid()))
            .unwrap(),
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
        title: "Projection cache".to_owned(),
        summary: "Derived and discardable".to_owned(),
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
