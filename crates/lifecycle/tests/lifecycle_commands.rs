use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDigestObservation, AuthorityDocument,
    AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus, AuthorityPolicy, Blocker,
    BlockerStatus, Command, CommandEnvelope, CommandId, CreateProjectPayload, Event, EventId,
    EventType, LedgerTimestamp, OpenBlockerPayload, OpenRiskPayload, PauseProjectPayload, Project,
    ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, RefreshAuthorityDocumentDigestPayload,
    ResourceRef, Risk, RiskStatus, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, SchemaVersion, ScopeKind, Severity, StableId, StartPhasePayload,
    StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_lifecycle::LifecycleEngine;
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;

#[test]
fn start_phase_emits_a_deterministic_transition_and_status_derivation() {
    let stream = roadmap_stream();
    let command = command(
        Command::StartPhase(StartPhasePayload {
            phase_ref: id("phase:implementation"),
        }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df61",
    );

    let first = LifecycleEngine::handle(&command, &stream).unwrap();
    let replay = LifecycleEngine::handle(&command, &stream).unwrap();

    assert_eq!(
        event_types(&first),
        vec![EventType::PhaseStarted, EventType::ProjectStatusDerived]
    );
    assert_eq!(
        first
            .iter()
            .map(|draft| draft.event_id.as_str())
            .collect::<Vec<_>>(),
        replay
            .iter()
            .map(|draft| draft.event_id.as_str())
            .collect::<Vec<_>>()
    );
    let resulting_stream = append_drafts(stream, &command, first);
    let projection = ProjectStateProjection::rebuild(&resulting_stream).unwrap();
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );
    assert_eq!(
        projection.state().active_phase_refs,
        vec![id("phase:implementation")]
    );
}

#[test]
fn pause_is_human_only_and_archival_style_status_is_derived_not_set_directly() {
    let stream = active_stream();
    let agent_command = command(
        Command::PauseProject(PauseProjectPayload {
            reason: "agent cannot pause".to_owned(),
        }),
        stream.len() as u64,
        ActorKind::Agent,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df62",
    );
    assert!(LifecycleEngine::handle(&agent_command, &stream).is_err());

    let human_command = command(
        Command::PauseProject(PauseProjectPayload {
            reason: "operator maintenance".to_owned(),
        }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df63",
    );
    let drafts = LifecycleEngine::handle(&human_command, &stream).unwrap();
    assert_eq!(
        event_types(&drafts),
        vec![EventType::ProjectPaused, EventType::ProjectStatusDerived]
    );

    let projection =
        ProjectStateProjection::rebuild(&append_drafts(stream, &human_command, drafts)).unwrap();
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Paused
    );
    assert_eq!(
        projection.state().project.paused_reason.as_deref(),
        Some("operator maintenance")
    );
}

#[test]
fn a_risk_never_blocks_implicitly_but_a_project_blocker_derives_blocked() {
    let stream = active_stream();
    let risk_command = command(
        Command::OpenRisk(OpenRiskPayload { risk: risk() }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df64",
    );
    let risk_drafts = LifecycleEngine::handle(&risk_command, &stream).unwrap();
    assert_eq!(event_types(&risk_drafts), vec![EventType::RiskOpened]);
    let with_risk = append_drafts(stream, &risk_command, risk_drafts);
    assert_eq!(
        ProjectStateProjection::rebuild(&with_risk)
            .unwrap()
            .state()
            .project
            .lifecycle_status,
        ProjectLifecycleStatus::Active
    );

    let blocker_command = command(
        Command::OpenBlocker(OpenBlockerPayload { blocker: blocker() }),
        with_risk.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df65",
    );
    let blocker_drafts = LifecycleEngine::handle(&blocker_command, &with_risk).unwrap();
    assert_eq!(
        event_types(&blocker_drafts),
        vec![EventType::BlockerOpened, EventType::ProjectStatusDerived]
    );
    let projection = ProjectStateProjection::rebuild(&append_drafts(
        with_risk,
        &blocker_command,
        blocker_drafts,
    ))
    .unwrap();
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Blocked
    );
    assert_eq!(projection.state().open_risk_refs, vec![id("risk:locking")]);
    assert_eq!(
        projection.state().open_blocker_refs,
        vec![id("blocker:dependency")]
    );
}

#[test]
fn derived_event_ids_are_namespaced_from_adjacent_command_ids() {
    let stream = roadmap_stream();
    let start = command(
        Command::StartPhase(StartPhasePayload {
            phase_ref: id("phase:implementation"),
        }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df90",
    );
    let start_drafts = LifecycleEngine::handle(&start, &stream).unwrap();
    let with_started_phase = append_drafts(stream, &start, start_drafts.clone());
    let adjacent = command(
        Command::OpenRisk(OpenRiskPayload { risk: risk() }),
        with_started_phase.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df91",
    );
    let adjacent_drafts = LifecycleEngine::handle(&adjacent, &with_started_phase).unwrap();

    assert_ne!(start_drafts[1].event_id, adjacent_drafts[0].event_id);
}

#[test]
fn a_critical_authority_conflict_prevents_phase_start() {
    let stream = roadmap_stream();
    let mismatch = command(
        Command::RefreshAuthorityDocumentDigest(RefreshAuthorityDocumentDigestPayload {
            document_ref: id("authority:canonical"),
            observation: AuthorityDigestObservation::Mismatch {
                observed_digest: Sha256Digest::from([9_u8; 32]),
            },
        }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7dfa0",
    );
    let mismatch_drafts = LifecycleEngine::handle(&mismatch, &stream).unwrap();
    assert_eq!(
        event_types(&mismatch_drafts),
        vec![
            EventType::AuthorityDocumentDigestMismatch,
            EventType::ConflictOpened
        ]
    );
    let conflicted = append_drafts(stream, &mismatch, mismatch_drafts);
    let start = command(
        Command::StartPhase(StartPhasePayload {
            phase_ref: id("phase:implementation"),
        }),
        conflicted.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7dfa1",
    );

    assert!(LifecycleEngine::handle(&start, &conflicted).is_err());
}

fn roadmap_stream() -> Vec<project_ledger_domain::EventEnvelope> {
    let mut stream = vec![raw_event(
        1,
        None,
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
    )];
    push_raw(
        &mut stream,
        Event::AuthorityDocumentRegistered(AuthorityDocumentRegisteredEvent {
            document: authority(),
        }),
    );
    push_raw(
        &mut stream,
        Event::RoadmapDefined(RoadmapDefinedEvent { roadmap: roadmap() }),
    );
    stream
}

fn active_stream() -> Vec<project_ledger_domain::EventEnvelope> {
    let stream = roadmap_stream();
    let command = command(
        Command::StartPhase(StartPhasePayload {
            phase_ref: id("phase:implementation"),
        }),
        stream.len() as u64,
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df60",
    );
    let drafts = LifecycleEngine::handle(&command, &stream).unwrap();
    append_drafts(stream, &command, drafts)
}

fn append_drafts(
    mut stream: Vec<project_ledger_domain::EventEnvelope>,
    command: &CommandEnvelope,
    drafts: Vec<EventDraft>,
) -> Vec<project_ledger_domain::EventEnvelope> {
    let mut head = stream.last().map(|event| event.event_digest);
    for draft in drafts {
        let event = build_event(command, stream.len() as u64 + 1, head, draft).unwrap();
        head = Some(event.event_digest);
        stream.push(event);
    }
    stream
}

fn push_raw(stream: &mut Vec<project_ledger_domain::EventEnvelope>, payload: Event) {
    stream.push(raw_event(
        stream.len() as u64 + 1,
        stream.last().map(|event| event.event_digest),
        payload,
    ));
}

fn raw_event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = command(
        Command::CreateProject(CreateProjectPayload { project: project() }),
        stream_version.saturating_sub(1),
        ActorKind::Human,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
    );
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:lifecycle-test"),
            occurred_at: at(),
            recorded_at: at(),
        },
    )
    .unwrap()
}

fn command(
    payload: Command,
    expected_state_version: u64,
    actor_kind: ActorKind,
    command_id: &str,
) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(command_id).unwrap(),
        project_ref: project_id(),
        expected_state_version,
        idempotency_key: id(&format!("lifecycle:command:{command_id}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind,
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

fn blocker() -> Blocker {
    let mut blocker = Blocker {
        schema_version: SchemaVersion::V0_2_0,
        blocker_id: id("blocker:dependency"),
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
    blocker.digest = record_digest(&blocker);
    blocker
}

fn risk() -> Risk {
    let mut risk = Risk {
        schema_version: SchemaVersion::V0_2_0,
        risk_id: id("risk:locking"),
        title: "SQLite locking".to_owned(),
        description: "concurrent writers may contend".to_owned(),
        scope_kind: ScopeKind::Project,
        scope_ref: id("scope:project"),
        severity: Severity::Critical,
        likelihood_class: project_ledger_domain::LikelihoodClass::Possible,
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
    risk.digest = record_digest(&risk);
    risk
}

fn record_digest<T: Serialize>(record: &T) -> Sha256Digest {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    sha256_digest(&value).unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Lifecycle commands".to_owned(),
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

fn event_types(drafts: &[EventDraft]) -> Vec<EventType> {
    drafts
        .iter()
        .map(|draft| draft.event.event_type())
        .collect()
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T14:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
