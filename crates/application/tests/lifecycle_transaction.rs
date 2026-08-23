use project_ledger_application::{
    CommandDraft, CommandProcessor, CreateProjectHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentStatus, AuthorityPolicy, Command,
    CommandEnvelope, CommandId, CommandResultStatus, CreateProjectPayload, DefineRoadmapPayload,
    LedgerTimestamp, Project, ProjectId, ProjectLifecycleStatus, RegisterAuthorityDocumentPayload,
    ResourceRef, Roadmap, RoadmapCompletionPolicy, RoadmapExecutionModel, SchemaVersion, StableId,
    StartPhasePayload, StorageProfile, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_lifecycle::LifecycleEngine;
use project_ledger_projection_engine::ProjectStateProjector;
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn lifecycle_commands_use_the_verified_stream_and_commit_one_atomic_transition() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());

    let created = CommandProcessor::new(&mut store, &clock)
        .execute(create_command(), &CreateProjectHandler)
        .unwrap();
    assert_eq!(created.new_state_version, 1);

    let authority = execute(
        &mut store,
        &clock,
        command(
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
            1,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df71",
        ),
    );
    assert_eq!(authority.status, CommandResultStatus::Accepted);
    assert_eq!(authority.new_state_version, 2);

    let roadmap = execute(
        &mut store,
        &clock,
        command(
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
            2,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df72",
        ),
    );
    assert_eq!(roadmap.new_state_version, 3);

    let started = execute(
        &mut store,
        &clock,
        command(
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            3,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df73",
        ),
    );
    assert_eq!(started.status, CommandResultStatus::Accepted);
    assert_eq!(started.previous_state_version, 3);
    assert_eq!(started.new_state_version, 5);
    assert_eq!(started.emitted_event_refs.len(), 2);

    let invalid_repeat = execute(
        &mut store,
        &clock,
        command(
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            5,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df74",
        ),
    );
    assert_eq!(invalid_repeat.status, CommandResultStatus::ValidationFailed);
    assert_eq!(invalid_repeat.previous_state_version, 5);
    assert_eq!(invalid_repeat.new_state_version, 5);
    assert!(invalid_repeat.emitted_event_refs.is_empty());
    assert_eq!(store.event_count(&project_id()).unwrap(), 5);

    let state = ProjectStateProjector::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        state.project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );
    assert_eq!(state.active_phase_refs, vec![id("phase:implementation")]);
}

fn execute(
    store: &mut SqliteEventStore,
    clock: &FixedClock,
    command: CommandEnvelope,
) -> project_ledger_domain::CommandResult {
    CommandProcessor::new(store, clock)
        .execute(command, &LifecycleEngine)
        .unwrap()
}

fn create_command() -> CommandEnvelope {
    command(
        Command::CreateProject(CreateProjectPayload { project: project() }),
        0,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df70",
    )
}

fn command(payload: Command, expected: u64, command_id: &str) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(command_id).unwrap(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(&format!("lifecycle:transaction:{expected}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: at(),
    })
    .unwrap()
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:canonical"),
        uri: ResourceRef::parse("file:///authority/spec.md").unwrap(),
        digest: Sha256Digest::from([1_u8; 32]),
        version: "1.0.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: at(),
        supersedes_ref: None,
    }
}

fn roadmap() -> Roadmap {
    Roadmap {
        schema_version: SchemaVersion::V0_2_0,
        roadmap_id: id("roadmap:main"),
        title: "Implementation".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:canonical"),
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Lifecycle transaction".to_owned(),
        summary: "Verified stream integration".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T15:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
