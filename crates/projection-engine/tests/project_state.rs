use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{
    ProjectStateProjector, ProjectionError, validate_state_integrity,
};
use serde::Serialize;

#[test]
fn genesis_projects_an_exact_deterministic_state_and_digest() {
    let event = event(
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
    );

    let first = ProjectStateProjector::rebuild(std::slice::from_ref(&event)).unwrap();
    let second = ProjectStateProjector::rebuild(std::slice::from_ref(&event)).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.schema_version, SchemaVersion::V0_2_0);
    assert_eq!(
        first.project_state_id,
        StableId::parse("state:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
    );
    assert_eq!(first.project, project());
    assert_eq!(first.state_version, 1);
    assert_eq!(first.material_state_version, 1);
    assert_eq!(first.last_material_event_ref, Some(event.event_id));
    assert_eq!(first.last_material_activity_at, Some(event.occurred_at));
    assert_eq!(first.updated_at, event.recorded_at);
    assert!(first.authority_documents.is_empty());
    assert!(first.roadmap.is_none());
    assert!(first.active_phase_refs.is_empty());
    assert!(first.primary_active_phase_ref.is_none());
    assert!(first.completed_phase_refs.is_empty());
    assert!(first.pending_gate_refs.is_empty());
    assert!(first.verified_gate_refs.is_empty());
    assert!(first.revalidation_required_gate_refs.is_empty());
    assert!(first.current_decision_refs.is_empty());
    assert!(first.open_blocker_refs.is_empty());
    assert!(first.open_risk_refs.is_empty());
    assert!(first.ordered_next_action_refs.is_empty());
    assert!(first.preferred_next_action_ref.is_none());
    assert!(first.checkpoint_ref.is_none());
    assert!(first.open_conflict_refs.is_empty());
    assert_eq!(first.state_digest, digest_without(&first, "state_digest"));
    validate_state_integrity(&first).unwrap();
}

#[test]
fn rebuild_requires_project_created_as_the_only_genesis() {
    assert!(matches!(
        ProjectStateProjector::rebuild(&[]),
        Err(ProjectionError::EmptyStream)
    ));

    let metadata_genesis = event(
        1,
        None,
        Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
            title: Some("too early".to_owned()),
            summary: None,
        }),
    );
    assert!(matches!(
        ProjectStateProjector::rebuild(&[metadata_genesis]),
        Err(ProjectionError::InvalidGenesis)
    ));
}

#[test]
fn rebuild_rejects_the_radar_profile_contract_version() {
    let event = event_with_schema(
        SchemaVersion::V0_1_1,
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
    );

    assert!(matches!(
        ProjectStateProjector::rebuild(&[event]),
        Err(ProjectionError::UnsupportedSchemaVersion)
    ));
}

#[test]
fn state_validation_rejects_a_self_consistent_but_wrong_derived_identity() {
    let event = event(
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
    );
    let mut state = ProjectStateProjector::rebuild(&[event]).unwrap();
    state.project_state_id = StableId::parse("state:wrong").unwrap();
    state.state_digest = digest_without(&state, "state_digest");

    assert!(matches!(
        validate_state_integrity(&state),
        Err(ProjectionError::StateIdentityMismatch)
    ));
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<project_ledger_canonical_json::Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    event_with_schema(
        SchemaVersion::V0_2_0,
        stream_version,
        previous_event_digest,
        payload,
    )
}

fn event_with_schema(
    schema_version: SchemaVersion,
    stream_version: u64,
    previous_event_digest: Option<project_ledger_canonical_json::Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: StableId::parse("projection:test").unwrap(),
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
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: StableId::parse("correlation:projection").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Ledger projection".to_owned(),
        summary: "Deterministic fixture".to_owned(),
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

fn digest_without<T: Serialize>(
    value: &T,
    field: &str,
) -> project_ledger_canonical_json::Sha256Digest {
    let mut value = serde_json::to_value(value).unwrap();
    value.as_object_mut().unwrap().remove(field);
    sha256_digest(&value).unwrap()
}
