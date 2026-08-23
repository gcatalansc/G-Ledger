use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, Blocker, BlockerOpenedEvent, BlockerResolvedEvent, BlockerStatus, Command,
    CommandId, ConflictKind, ConflictOpenedEvent, ConflictRecord, ConflictResolvedEvent,
    ConflictStatus, CreateProjectPayload, Decision, DecisionAcceptedEvent, DecisionProposedEvent,
    DecisionStatus, DecisionSupersededEvent, Event, EventId, LedgerTimestamp, LikelihoodClass,
    NextAction, NextActionClassification, NextActionCompletedEvent, NextActionSetEvent,
    PhaseStartedEvent, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectStatusDerivedEvent, ResourceRef, Risk, RiskClosedEvent, RiskOpenedEvent, RiskStatus,
    Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent, RoadmapExecutionModel, SchemaVersion,
    ScopeKind, Severity, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{ProjectStateProjection, ProjectionError};
use serde::Serialize;

#[test]
fn accepted_decision_supersession_preserves_history_and_selects_only_the_replacement() {
    let mut stream = active_stream();
    let first = decision("decision:storage-v1", 1, None);
    let replacement = decision("decision:storage-v2", 2, Some(id("decision:storage-v1")));
    stream.push(Event::DecisionProposed(DecisionProposedEvent {
        decision: first.clone(),
    }));
    stream.push(Event::DecisionAccepted(DecisionAcceptedEvent {
        decision_ref: first.decision_id.clone(),
        authority_ref: id("authority:canonical"),
    }));
    stream.push(Event::DecisionProposed(DecisionProposedEvent {
        decision: replacement.clone(),
    }));
    stream.push(Event::DecisionAccepted(DecisionAcceptedEvent {
        decision_ref: replacement.decision_id.clone(),
        authority_ref: id("authority:canonical"),
    }));
    stream.push(Event::DecisionSuperseded(DecisionSupersededEvent {
        decision_ref: first.decision_id.clone(),
        replacement_ref: replacement.decision_id.clone(),
    }));

    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();

    assert_eq!(
        projection.state().current_decision_refs,
        vec![replacement.decision_id.clone()]
    );
    assert_eq!(
        projection.decision(&first.decision_id).unwrap().status,
        DecisionStatus::Superseded
    );
    assert_eq!(
        projection
            .decision(&replacement.decision_id)
            .unwrap()
            .status,
        DecisionStatus::Accepted
    );
}

#[test]
fn blockers_risks_actions_and_conflicts_reduce_without_losing_history() {
    let mut stream = active_stream();
    let risk = risk("risk:locking");
    let blocker = blocker("blocker:dependency");
    let conflict = conflict("conflict:authority");

    stream.push(Event::RiskOpened(RiskOpenedEvent { risk: risk.clone() }));
    stream.push(Event::BlockerOpened(BlockerOpenedEvent {
        blocker: blocker.clone(),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Blocked,
        reason_codes: vec!["OPEN_PROJECT_BLOCKER".to_owned()],
    }));
    stream.push(Event::BlockerResolved(BlockerResolvedEvent {
        blocker_ref: blocker.blocker_id.clone(),
        resolution_summary: "dependency restored".to_owned(),
        evidence_refs: vec![id("evidence:dependency-restored")],
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Blocked,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["LAST_BLOCKER_RESOLVED".to_owned()],
    }));

    for action in [
        action("action:proposed", NextActionClassification::Proposed, None),
        action(
            "action:recommended",
            NextActionClassification::Recommended,
            None,
        ),
        action(
            "action:authorized",
            NextActionClassification::Authorized,
            Some(id("authority:canonical")),
        ),
        action("action:eligible", NextActionClassification::Eligible, None),
    ] {
        stream.push(Event::NextActionSet(NextActionSetEvent { action }));
    }
    stream.push(Event::NextActionCompleted(NextActionCompletedEvent {
        action_ref: id("action:authorized"),
        evidence_refs: vec![id("evidence:action-complete")],
    }));
    stream.push(Event::ConflictOpened(ConflictOpenedEvent {
        conflict: conflict.clone(),
    }));
    stream.push(Event::ConflictResolved(ConflictResolvedEvent {
        conflict_ref: conflict.conflict_id.clone(),
        resolution_ref: ResourceRef::parse("project-ledger://resolutions/authority").unwrap(),
        reason: "canonical source confirmed".to_owned(),
    }));
    stream.push(Event::RiskClosed(RiskClosedEvent {
        risk_ref: risk.risk_id.clone(),
        note: "lock strategy verified".to_owned(),
        evidence_refs: vec![id("evidence:locking")],
    }));

    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let state = projection.state();

    assert!(state.open_blocker_refs.is_empty());
    assert!(state.open_risk_refs.is_empty());
    assert!(state.open_conflict_refs.is_empty());
    assert_eq!(
        state.project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );
    assert_eq!(
        state.ordered_next_action_refs,
        vec![
            id("action:eligible"),
            id("action:recommended"),
            id("action:proposed")
        ]
    );
    assert_eq!(state.preferred_next_action_ref, Some(id("action:eligible")));
    assert_eq!(
        projection.blocker(&blocker.blocker_id).unwrap().status,
        BlockerStatus::Resolved
    );
    assert_eq!(
        projection.risk(&risk.risk_id).unwrap().status,
        RiskStatus::Closed
    );
    assert_eq!(
        projection
            .next_action(&id("action:authorized"))
            .unwrap()
            .classification,
        NextActionClassification::Completed
    );
    assert_eq!(
        projection.conflict(&conflict.conflict_id).unwrap().status,
        ConflictStatus::Resolved
    );
}

#[test]
fn missing_decision_replacement_and_unauthorized_authorized_action_fail_closed() {
    let mut missing_replacement = active_stream();
    let first = decision("decision:storage-v1", 1, None);
    missing_replacement.push(Event::DecisionProposed(DecisionProposedEvent {
        decision: first.clone(),
    }));
    missing_replacement.push(Event::DecisionAccepted(DecisionAcceptedEvent {
        decision_ref: first.decision_id.clone(),
        authority_ref: id("authority:canonical"),
    }));
    missing_replacement.push(Event::DecisionSuperseded(DecisionSupersededEvent {
        decision_ref: first.decision_id,
        replacement_ref: id("decision:missing"),
    }));
    assert!(matches!(
        ProjectStateProjection::rebuild(&missing_replacement.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));

    let mut unauthorized = active_stream();
    unauthorized.push(Event::NextActionSet(NextActionSetEvent {
        action: action(
            "action:invalid-authorized",
            NextActionClassification::Authorized,
            None,
        ),
    }));
    assert!(matches!(
        ProjectStateProjection::rebuild(&unauthorized.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

struct Stream {
    events: Vec<project_ledger_domain::EventEnvelope>,
}

impl Stream {
    fn genesis() -> Self {
        Self {
            events: vec![event(
                1,
                None,
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        self.events.push(event(
            self.events.len() as u64 + 1,
            self.events.last().map(|event| event.event_digest),
            payload,
        ));
    }
}

fn active_stream() -> Stream {
    let mut stream = Stream::genesis();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority(),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:implementation"),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Initializing,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_STARTED".to_owned()],
    }));
    stream
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let at = format!("2026-08-23T13:{:02}:00-04:00", stream_version - 1);
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("projection:records:{stream_version}")),
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
            correlation_ref: id("correlation:continuity-records"),
            occurred_at: LedgerTimestamp::parse(&at).unwrap(),
            recorded_at: LedgerTimestamp::parse(&at).unwrap(),
        },
    )
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

fn decision(decision_id: &str, version: u64, supersedes_ref: Option<StableId>) -> Decision {
    let mut decision = Decision {
        schema_version: SchemaVersion::V0_2_0,
        decision_id: id(decision_id),
        title: "Storage authority".to_owned(),
        domain: "storage".to_owned(),
        scope: "project".to_owned(),
        context: "durable state".to_owned(),
        decision: format!("storage decision version {version}"),
        rationale: "deterministic recovery".to_owned(),
        alternatives: vec!["memory".to_owned()],
        status: DecisionStatus::Proposed,
        version,
        authority_ref: None,
        evidence_refs: Vec::new(),
        supersedes_ref,
        created_at: at(),
        accepted_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/decision").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    decision.digest = digest_without(&decision);
    decision
}

fn blocker(blocker_id: &str) -> Blocker {
    let mut blocker = Blocker {
        schema_version: SchemaVersion::V0_2_0,
        blocker_id: id(blocker_id),
        title: "External dependency".to_owned(),
        description: "required service unavailable".to_owned(),
        scope_kind: ScopeKind::Project,
        scope_ref: id("scope:project"),
        severity: Severity::High,
        status: BlockerStatus::Open,
        source_kind: "OBSERVED".to_owned(),
        source_ref: None,
        owner_ref: None,
        evidence_refs: Vec::new(),
        opened_at: at(),
        updated_at: at(),
        resolved_at: None,
        resolution_summary: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/blocker").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    blocker.digest = digest_without(&blocker);
    blocker
}

fn risk(risk_id: &str) -> Risk {
    let mut risk = Risk {
        schema_version: SchemaVersion::V0_2_0,
        risk_id: id(risk_id),
        title: "SQLite locking".to_owned(),
        description: "concurrent writers may contend".to_owned(),
        scope_kind: ScopeKind::Project,
        scope_ref: id("scope:project"),
        severity: Severity::Critical,
        likelihood_class: LikelihoodClass::Possible,
        status: RiskStatus::Open,
        owner_ref: None,
        trigger_conditions: vec!["concurrent writer".to_owned()],
        mitigation_actions: vec!["busy timeout".to_owned()],
        evidence_refs: Vec::new(),
        opened_at: at(),
        updated_at: at(),
        closed_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/risk").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    risk.digest = digest_without(&risk);
    risk
}

fn action(
    action_id: &str,
    classification: NextActionClassification,
    authority_ref: Option<StableId>,
) -> NextAction {
    let mut action = NextAction {
        schema_version: SchemaVersion::V0_2_0,
        next_action_id: id(action_id),
        title: action_id.to_owned(),
        description: "deterministic next work".to_owned(),
        phase_ref: Some(id("phase:implementation")),
        gate_ref: None,
        classification,
        authority_ref,
        precondition_refs: Vec::new(),
        blocked_by_refs: Vec::new(),
        owner_ref: None,
        created_at: at(),
        updated_at: at(),
        completed_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/action").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    action.digest = digest_without(&action);
    action
}

fn conflict(conflict_id: &str) -> ConflictRecord {
    ConflictRecord {
        schema_version: SchemaVersion::V0_2_0,
        conflict_id: id(conflict_id),
        kind: ConflictKind::AuthorityDocumentDivergence,
        severity: Severity::Critical,
        domain: "authority".to_owned(),
        subject_refs: vec![id("authority:canonical")],
        source_refs: vec![ResourceRef::parse("file:///authority/spec.md").unwrap()],
        status: ConflictStatus::Open,
        required_action: "confirm canonical source".to_owned(),
        opened_at: at(),
        resolved_at: None,
        resolution_ref: None,
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Continuity records".to_owned(),
        summary: "S4 deterministic fixture".to_owned(),
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

fn digest_without<T: Serialize>(record: &T) -> Sha256Digest {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    sha256_digest(&value).unwrap()
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T13:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
