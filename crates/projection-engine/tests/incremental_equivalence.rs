use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectMetadataUpdatedEvent, ProjectPausedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{ProjectStateProjector, ProjectionError};

#[test]
fn incremental_metadata_projection_equals_a_full_rebuild() {
    let genesis = event(
        project_id(),
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        "2026-08-23T12:00:00-04:00",
    );
    let metadata = event(
        project_id(),
        2,
        Some(genesis.event_digest),
        Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
            title: Some("Projected title".to_owned()),
            summary: Some("Projected summary".to_owned()),
        }),
        "2026-08-23T12:05:00-04:00",
    );

    let genesis_state = ProjectStateProjector::rebuild(std::slice::from_ref(&genesis)).unwrap();
    let incremental =
        ProjectStateProjector::apply(&genesis_state, genesis.event_digest, &metadata).unwrap();
    let rebuilt = ProjectStateProjector::rebuild(&[genesis, metadata.clone()]).unwrap();

    assert_eq!(incremental, rebuilt);
    assert_eq!(rebuilt.project.title, "Projected title");
    assert_eq!(rebuilt.project.summary, "Projected summary");
    assert_eq!(rebuilt.state_version, 2);
    assert_eq!(rebuilt.material_state_version, 2);
    assert_eq!(rebuilt.last_material_event_ref, Some(metadata.event_id));
    assert_eq!(
        rebuilt.last_material_activity_at,
        Some(metadata.occurred_at)
    );
    assert_eq!(rebuilt.updated_at, metadata.recorded_at);
}

#[test]
fn incremental_projection_rejects_untrusted_or_noncontiguous_inputs() {
    let genesis = event(
        project_id(),
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        "2026-08-23T12:00:00-04:00",
    );
    let state = ProjectStateProjector::rebuild(std::slice::from_ref(&genesis)).unwrap();
    let metadata = event(
        project_id(),
        2,
        Some(genesis.event_digest),
        Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
            title: Some("next".to_owned()),
            summary: None,
        }),
        "2026-08-23T12:05:00-04:00",
    );

    let mut corrupt_state = state.clone();
    corrupt_state.project.title = "digest no longer covers this".to_owned();
    assert!(matches!(
        ProjectStateProjector::apply(&corrupt_state, genesis.event_digest, &metadata),
        Err(ProjectionError::StateDigestMismatch)
    ));

    let gap = event(
        project_id(),
        3,
        Some(genesis.event_digest),
        Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
            title: None,
            summary: Some("gap".to_owned()),
        }),
        "2026-08-23T12:06:00-04:00",
    );
    assert!(matches!(
        ProjectStateProjector::apply(&state, genesis.event_digest, &gap),
        Err(ProjectionError::InvalidSequence { .. })
    ));

    assert!(matches!(
        ProjectStateProjector::apply(&state, Sha256Digest::from([7_u8; 32]), &metadata),
        Err(ProjectionError::InvalidSequence { .. })
    ));
}

#[test]
fn incremental_projection_rejects_cross_project_duplicate_genesis_and_later_slice_events() {
    let genesis = event(
        project_id(),
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        "2026-08-23T12:00:00-04:00",
    );
    let state = ProjectStateProjector::rebuild(std::slice::from_ref(&genesis)).unwrap();

    let other_project = ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df6e").unwrap();
    let cross_project = event(
        other_project,
        2,
        Some(genesis.event_digest),
        Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
            title: Some("foreign".to_owned()),
            summary: None,
        }),
        "2026-08-23T12:05:00-04:00",
    );
    assert!(matches!(
        ProjectStateProjector::apply(&state, genesis.event_digest, &cross_project),
        Err(ProjectionError::ProjectMismatch)
    ));

    let duplicate_genesis = event(
        project_id(),
        2,
        Some(genesis.event_digest),
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        "2026-08-23T12:05:00-04:00",
    );
    assert!(matches!(
        ProjectStateProjector::apply(&state, genesis.event_digest, &duplicate_genesis),
        Err(ProjectionError::DuplicateGenesis)
    ));

    let lifecycle_event = event(
        project_id(),
        2,
        Some(genesis.event_digest),
        Event::ProjectPaused(ProjectPausedEvent {
            reason: "belongs to S4".to_owned(),
        }),
        "2026-08-23T12:05:00-04:00",
    );
    assert!(matches!(
        ProjectStateProjector::apply(&state, genesis.event_digest, &lifecycle_event),
        Err(ProjectionError::UnsupportedEvent)
    ));
}

fn event(
    project_ref: ProjectId,
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
    at: &str,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref,
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: StableId::parse(&format!("projection:version:{stream_version}")).unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:test").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: LedgerTimestamp::parse(at).unwrap(),
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
            occurred_at: LedgerTimestamp::parse(at).unwrap(),
            recorded_at: LedgerTimestamp::parse(at).unwrap(),
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
        created_at: LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
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
