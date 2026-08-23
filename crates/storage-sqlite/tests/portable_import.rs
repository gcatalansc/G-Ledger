use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile, TrustLevel,
    UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{
    CommandDraft, EventDraft, EventStore, build_event, prepare_command,
};
use project_ledger_portable_bundle::{PortableImportOutcome, PortableImportPlan, apply_import};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;

#[test]
fn ac_005_new_project_and_fast_forward_roundtrip_through_real_sqlite() {
    let source = stream();
    let directory = tempfile::tempdir().unwrap();
    let mut fresh = SqliteEventStore::open(&directory.path().join("fresh.db")).unwrap();
    let new_plan = PortableImportPlan::NewProject {
        project_ref: project_id(),
        events: source.clone(),
    };

    assert!(matches!(
        apply_import(&mut fresh, &new_plan).unwrap(),
        PortableImportOutcome::NewProject {
            appended_events: 2,
            ..
        }
    ));
    assert_eq!(fresh.load_stream(&project_id()).unwrap(), source);

    let mut prefix = SqliteEventStore::open(&directory.path().join("prefix.db")).unwrap();
    apply_import(
        &mut prefix,
        &PortableImportPlan::NewProject {
            project_ref: project_id(),
            events: source[..1].to_vec(),
        },
    )
    .unwrap();
    let fast_forward = PortableImportPlan::FastForward {
        project_ref: project_id(),
        expected_version: 1,
        events: source[1..].to_vec(),
    };
    assert!(matches!(
        apply_import(&mut prefix, &fast_forward).unwrap(),
        PortableImportOutcome::FastForward {
            appended_events: 1,
            ..
        }
    ));
    let rebuilt =
        ProjectStateProjection::rebuild(&prefix.load_stream(&project_id()).unwrap()).unwrap();
    let expected = ProjectStateProjection::rebuild(&source).unwrap();
    assert_eq!(rebuilt.state(), expected.state());
}

#[test]
fn ac_035_store_revalidates_suffix_and_rolls_back_every_invalid_import() {
    let source = stream();
    let directory = tempfile::tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    apply_import(
        &mut store,
        &PortableImportPlan::NewProject {
            project_ref: project_id(),
            events: source[..1].to_vec(),
        },
    )
    .unwrap();

    let invalid = PortableImportPlan::FastForward {
        project_ref: project_id(),
        expected_version: 0,
        events: source[1..].to_vec(),
    };
    assert!(apply_import(&mut store, &invalid).is_err());
    assert_eq!(store.current_version(&project_id()).unwrap(), 1);
    assert_eq!(store.load_stream(&project_id()).unwrap(), source[..1]);

    let identical = PortableImportPlan::IdenticalChain {
        project_ref: project_id(),
        state_version: 1,
        chain_digest: source[0].event_digest,
    };
    assert!(matches!(
        apply_import(&mut store, &identical).unwrap(),
        PortableImportOutcome::IdenticalChain {
            state_version: 1,
            ..
        }
    ));
    assert_eq!(store.current_version(&project_id()).unwrap(), 1);
}

fn stream() -> Vec<project_ledger_domain::EventEnvelope> {
    let project = project();
    let create = command(
        1,
        0,
        Command::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
    );
    let first = build_event(
        &create,
        1,
        None,
        EventDraft {
            event_id: event_id(1),
            event: Event::ProjectCreated(ProjectCreatedEvent { project }),
            correlation_ref: id("correlation:portable-sqlite"),
            occurred_at: timestamp(1),
            recorded_at: timestamp(1),
        },
    )
    .unwrap();
    let update = command(
        2,
        1,
        Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some("Imported title".to_owned()),
            summary: None,
        }),
    );
    let second = build_event(
        &update,
        2,
        Some(first.event_digest),
        EventDraft {
            event_id: event_id(2),
            event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                title: Some("Imported title".to_owned()),
                summary: None,
            }),
            correlation_ref: id("correlation:portable-sqlite"),
            occurred_at: timestamp(2),
            recorded_at: timestamp(2),
        },
    )
    .unwrap();
    vec![first, second]
}

fn command(number: u8, expected: u64, payload: Command) -> project_ledger_domain::CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(&format!(
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
        ))
        .unwrap(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(&format!("idempotency:portable-sqlite:{number}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: timestamp(u64::from(number)),
    })
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Portable SQLite".to_owned(),
        summary: "S7 import".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(1),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T18:{minute:02}:00-04:00")).unwrap()
}

fn event_id(number: u8) -> EventId {
    EventId::parse(&format!(
        "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
    ))
    .unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
