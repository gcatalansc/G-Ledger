use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentDigestMismatchEvent,
    AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus, AuthorityPolicy, Command, CommandId,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, PhaseCompletedEvent, PhaseReopenedEvent,
    PhaseSkippedEvent, PhaseStartedEvent, PhaseStatus, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectStatusDerivedEvent, ResourceRef, Roadmap,
    RoadmapCompletionPolicy, RoadmapDefinedEvent, RoadmapExecutionModel, SchemaVersion, StableId,
    StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectStateProjector, ProjectionError,
};

#[test]
fn authority_roadmap_and_phase_lifecycle_rebuild_deterministically() {
    let mut stream = Stream::genesis();
    let lower = authority("authority:secondary", 20, [2_u8; 32]);
    let canonical = authority("authority:canonical", 10, [1_u8; 32]);
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: lower.clone(),
        },
    ));
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: canonical.clone(),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::AuthorityDocumentDigestMismatch(
        AuthorityDocumentDigestMismatchEvent {
            document_ref: canonical.document_id.clone(),
            expected_digest: canonical.digest,
            observed_digest: Sha256Digest::from([9_u8; 32]),
        },
    ));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:discovery"),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Initializing,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_STARTED".to_owned()],
    }));
    stream.push(Event::PhaseCompleted(PhaseCompletedEvent {
        phase_ref: id("phase:discovery"),
        gate_evaluation_refs: Vec::new(),
    }));
    stream.push(Event::PhaseSkipped(PhaseSkippedEvent {
        phase_ref: id("phase:implementation"),
        authority_ref: canonical.document_id.clone(),
        reason: "waived by authority".to_owned(),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Completed,
        reason_codes: vec!["ROADMAP_COMPLETED".to_owned()],
    }));

    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let state = projection.state();

    assert_eq!(
        state
            .authority_documents
            .iter()
            .map(|document| document.document_id.as_str())
            .collect::<Vec<_>>(),
        vec!["authority:canonical", "authority:secondary"]
    );
    assert_eq!(state.authority_documents[0].digest, canonical.digest);
    assert_eq!(
        state.authority_documents[0].status,
        AuthorityDocumentStatus::DigestMismatch
    );
    assert_eq!(state.roadmap, Some(roadmap()));
    assert!(state.active_phase_refs.is_empty());
    assert_eq!(state.completed_phase_refs, vec![id("phase:discovery")]);
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(PhaseStatus::Skipped)
    );
    assert_eq!(
        state.project.lifecycle_status,
        ProjectLifecycleStatus::Completed
    );
    assert_eq!(state.state_version, stream.events.len() as u64);
    assert_eq!(state.material_state_version, stream.events.len() as u64);
}

#[test]
fn reopening_a_completed_phase_restores_active_status_and_incremental_equals_rebuild() {
    let mut stream = completed_stream();
    let mut incremental = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let reopened = stream.push(Event::PhaseReopened(PhaseReopenedEvent {
        phase_ref: id("phase:discovery"),
        resulting_status: PhaseStatus::Active,
        reason: "new authority input".to_owned(),
    }));
    incremental = incremental
        .apply(
            stream.events[stream.events.len() - 2].event_digest,
            &reopened,
        )
        .unwrap();
    let status = stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Completed,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_REOPENED".to_owned()],
    }));
    incremental = incremental.apply(reopened.event_digest, &status).unwrap();

    let rebuilt = ProjectStateProjection::rebuild(&stream.events).unwrap();

    assert_eq!(incremental.state(), rebuilt.state());
    assert_eq!(
        rebuilt.state().active_phase_refs,
        vec![id("phase:discovery")]
    );
    assert!(rebuilt.state().completed_phase_refs.is_empty());
    assert_eq!(
        rebuilt.phase_status(&id("phase:discovery")),
        Some(PhaseStatus::Active)
    );
    assert_eq!(
        rebuilt.state().project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );
    assert_eq!(
        ProjectStateProjector::rebuild(&stream.events).unwrap(),
        rebuilt.into_state()
    );
}

#[test]
fn a_second_active_phase_fails_closed() {
    let mut stream = Stream::genesis();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority("authority:canonical", 10, [1_u8; 32]),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:discovery"),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:implementation"),
    }));

    assert!(matches!(
        ProjectStateProjection::rebuild(&stream.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

#[test]
fn a_complete_stream_cannot_end_with_a_stale_derived_project_status() {
    let mut stream = Stream::genesis();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority("authority:canonical", 10, [1_u8; 32]),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:discovery"),
    }));

    assert!(matches!(
        ProjectStateProjection::rebuild(&stream.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

fn completed_stream() -> Stream {
    let mut stream = Stream::genesis();
    let authority = authority("authority:canonical", 10, [1_u8; 32]);
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority.clone(),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:discovery"),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Initializing,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_STARTED".to_owned()],
    }));
    stream.push(Event::PhaseCompleted(PhaseCompletedEvent {
        phase_ref: id("phase:discovery"),
        gate_evaluation_refs: Vec::new(),
    }));
    stream.push(Event::PhaseSkipped(PhaseSkippedEvent {
        phase_ref: id("phase:implementation"),
        authority_ref: authority.document_id,
        reason: "waived by authority".to_owned(),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Completed,
        reason_codes: vec!["ROADMAP_COMPLETED".to_owned()],
    }));
    stream
}

struct Stream {
    events: Vec<project_ledger_domain::EventEnvelope>,
}

impl Stream {
    fn genesis() -> Self {
        let genesis = event(
            1,
            None,
            Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        );
        Self {
            events: vec![genesis],
        }
    }

    fn push(&mut self, payload: Event) -> project_ledger_domain::EventEnvelope {
        let event = event(
            self.events.len() as u64 + 1,
            self.events.last().map(|event| event.event_digest),
            payload,
        );
        self.events.push(event.clone());
        event
    }
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let at = format!("2026-08-23T12:{:02}:00-04:00", stream_version - 1);
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("projection:lifecycle:{stream_version}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: LedgerTimestamp::parse(&at).unwrap(),
    })
    .unwrap();
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:lifecycle-projection"),
            occurred_at: LedgerTimestamp::parse(&at).unwrap(),
            recorded_at: LedgerTimestamp::parse(&at).unwrap(),
        },
    )
    .unwrap()
}

fn authority(document_id: &str, authority_rank: u32, digest: [u8; 32]) -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id(document_id),
        uri: ResourceRef::parse(&format!("file:///authority/{document_id}.md")).unwrap(),
        digest: Sha256Digest::from(digest),
        version: "1.0.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank,
        effective_from: LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
        supersedes_ref: None,
    }
}

fn roadmap() -> Roadmap {
    Roadmap {
        schema_version: SchemaVersion::V0_2_0,
        roadmap_id: id("roadmap:main"),
        title: "Sequential delivery".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:discovery"), id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:canonical"),
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Lifecycle projection".to_owned(),
        summary: "S4 deterministic fixture".to_owned(),
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

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
