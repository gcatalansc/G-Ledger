use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, ResourceRef,
    SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_project_catalog::{CatalogError, KnownProject, ProjectCatalog};
use project_ledger_projection_engine::ProjectStateProjector;

#[test]
fn rebuild_maps_and_orders_explicit_known_projects_independent_of_input_order() {
    let alpha = known_project(
        "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df91",
        "Alpha",
        "file:///ledgers/alpha.db",
    );
    let beta = known_project(
        "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df92",
        "Beta",
        "file:///ledgers/beta.db",
    );

    let forward = ProjectCatalog::rebuild(&[alpha.clone(), beta.clone()]).unwrap();
    let reverse = ProjectCatalog::rebuild(&[beta, alpha.clone()]).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.len(), 2);
    assert_eq!(
        forward[0].project_ref.as_str(),
        "project-ledger://projects/0198f8c8-7417-7fb0-b88f-0fe0c0a7df91"
    );
    assert_eq!(forward[0].canonical_title, "Alpha");
    assert_eq!(
        forward[0].ledger_location_ref.as_str(),
        "file:///ledgers/alpha.db"
    );
    assert_eq!(
        forward[0].last_seen_state_version,
        alpha.state.state_version
    );
    assert_eq!(forward[0].last_seen_state_digest, alpha.state.state_digest);
    assert_eq!(forward[0].schema_version, SchemaVersion::V0_2_0);
}

#[test]
fn rebuild_rejects_invalid_state_and_duplicate_project_identity() {
    let known = known_project(
        "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df91",
        "Alpha",
        "file:///ledgers/alpha.db",
    );
    let mut invalid = known.clone();
    invalid.state.project.title = "digest corruption".to_owned();
    assert!(matches!(
        ProjectCatalog::rebuild(&[invalid]),
        Err(CatalogError::InvalidState { .. })
    ));

    let duplicate_location = KnownProject {
        state: known.state.clone(),
        ledger_location_ref: ResourceRef::parse("file:///ledgers/duplicate.db").unwrap(),
    };
    assert!(matches!(
        ProjectCatalog::rebuild(&[known, duplicate_location]),
        Err(CatalogError::DuplicateProject { .. })
    ));
}

fn known_project(project_id: &str, title: &str, location: &str) -> KnownProject {
    let project_id = ProjectId::parse(project_id).unwrap();
    let project = Project {
        project_id: project_id.clone(),
        title: title.to_owned(),
        summary: "Catalog fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    };
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id,
        expected_state_version: 0,
        idempotency_key: StableId::parse(&format!("catalog:{title}")).unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:test").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
        submitted_at: at(),
    })
    .unwrap();
    let genesis = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project }),
            correlation_ref: StableId::parse("correlation:catalog").unwrap(),
            occurred_at: at(),
            recorded_at: at(),
        },
    )
    .unwrap();
    KnownProject {
        state: ProjectStateProjector::rebuild(&[genesis]).unwrap(),
        ledger_location_ref: ResourceRef::parse(location).unwrap(),
    }
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T17:00:00-04:00").unwrap()
}
