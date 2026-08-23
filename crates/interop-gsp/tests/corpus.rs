use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, CheckpointCreatedEvent, Command, CommandId, ContinuityCheckpoint,
    CreateProjectPayload, Decision, DecisionProposedEvent, DecisionStatus, Event, EventId,
    EvidenceKind, FreshnessPolicy, GateCompletionOperator, GateCriticality, GateDefinedEvent,
    GateDefinition, GateWaiverPolicy, LedgerTimestamp, PhaseStartedEvent, PredicateAst, Project,
    ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, ProjectStatusDerivedEvent,
    RepositoryDependencies, RequiredEvidenceRule, ResourceRef, Roadmap, RoadmapCompletionPolicy,
    RoadmapDefinedEvent, RoadmapExecutionModel, SchemaVersion, StableId, StorageProfile,
    TrustLevel, WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_interop_gsp::{
    GspCorpusBudget, GspCorpusError, GspCorpusRequest, build_minimized_corpus,
};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;

#[test]
fn explicit_bounded_scope_builds_exact_evidence_bundle_without_private_paths() {
    let (projection, events, refs) = fixture();
    let request = request(&projection, &refs);

    let corpus = build_minimized_corpus(&projection, &events, &request).unwrap();
    let repeated = build_minimized_corpus(&projection, &events, &request).unwrap();

    assert_eq!(corpus, repeated);
    assert_eq!(corpus.manifest.profile, "GSP_MINIMIZED_CORPUS_V1");
    assert_eq!(corpus.manifest.files.len(), 5);
    assert!(!corpus.manifest.full_timeline_included);
    assert!(!corpus.manifest.raw_evidence_output_included);
    assert!(!corpus.manifest.workspace_paths_included);
    assert_eq!(
        corpus.evidence_bundle["evidence_items"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        corpus.evidence_bundle["route_taken"],
        serde_json::json!(["project-ledger-explicit-scope"])
    );

    let rendered = serde_json::to_string(&corpus).unwrap();
    assert!(!rendered.contains("C:\\\\private\\\\PROJECT_LEDGER_SPEC.md"));
    assert!(!rendered.contains("file:///C:/private/PROJECT_LEDGER_SPEC.md"));
    assert!(!rendered.contains("workspace://secrets/build.log"));
    assert!(rendered.contains("source_ref_digest"));

    let contracts = project_ledger_interop_gsp::GspContractCatalog::load_vendored(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )
    .unwrap();
    contracts
        .validate("evidence.bundle", &corpus.evidence_bundle)
        .unwrap();

    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../integrations/gsp/schemas/ledger-gsp-corpus-manifest.schema.json"
    ))
    .unwrap();
    let manifest_value = serde_json::to_value(&corpus.manifest).unwrap();
    jsonschema::validator_for(&manifest_schema)
        .unwrap()
        .validate(&manifest_value)
        .unwrap();
}

#[test]
fn missing_unallowlisted_and_over_budget_selections_fail_closed() {
    let (projection, events, refs) = fixture();

    let mut missing = request(&projection, &refs);
    missing.authority_refs = vec![id("authority:missing")];
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &missing),
        Err(GspCorpusError::MissingSelection { .. })
    ));

    let mut unallowlisted = request(&projection, &refs);
    unallowlisted.material_event_refs = vec![events[0].event_id.clone()];
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &unallowlisted),
        Err(GspCorpusError::EventTypeNotAllowed { .. })
    ));

    let mut over_budget = request(&projection, &refs);
    over_budget.budget.max_items = 4;
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &over_budget),
        Err(GspCorpusError::BudgetExceeded { .. })
    ));

    let mut duplicate = request(&projection, &refs);
    duplicate.authority_refs.push(refs.authority.clone());
    duplicate.budget.max_items = 6;
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &duplicate),
        Err(GspCorpusError::DuplicateSelection { .. })
    ));

    let mut empty = request(&projection, &refs);
    empty.authority_refs.clear();
    empty.decision_refs.clear();
    empty.gate_refs.clear();
    empty.checkpoint_refs.clear();
    empty.material_event_refs.clear();
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &empty),
        Err(GspCorpusError::EmptySelection)
    ));

    let mut wrong_project = request(&projection, &refs);
    wrong_project.project_ref =
        ProjectId::parse("project:0198cf00-0000-7000-8000-000000000012").unwrap();
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &wrong_project),
        Err(GspCorpusError::ProjectMismatch)
    ));

    let mut text_budget = request(&projection, &refs);
    text_budget.budget.max_text_bytes = 1;
    assert!(matches!(
        build_minimized_corpus(&projection, &events, &text_budget),
        Err(GspCorpusError::BudgetExceeded {
            field: "max_text_bytes",
            ..
        })
    ));
}

struct FixtureRefs {
    authority: StableId,
    decision: StableId,
    gate: StableId,
    checkpoint: StableId,
    work_event: EventId,
}

fn fixture() -> (
    ProjectStateProjection,
    Vec<project_ledger_domain::EventEnvelope>,
    FixtureRefs,
) {
    let mut stream = Stream::genesis();
    let authority = authority();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority.clone(),
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
    let decision = decision();
    stream.push(Event::DecisionProposed(DecisionProposedEvent {
        decision: decision.clone(),
    }));
    let gate = gate();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate.clone() }));
    let work_event = EventId::new();
    stream.push_with_id(
        work_event.clone(),
        Event::WorkProgressRecorded(WorkProgressRecordedEvent {
            work_record: work(),
        }),
    );
    let before_checkpoint = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let checkpoint_event = EventId::new();
    let checkpoint = checkpoint(&before_checkpoint, checkpoint_event.clone());
    stream.push_with_id(
        checkpoint_event,
        Event::CheckpointCreated(CheckpointCreatedEvent {
            checkpoint: checkpoint.clone(),
        }),
    );
    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();
    (
        projection,
        stream.events,
        FixtureRefs {
            authority: authority.document_id,
            decision: decision.decision_id,
            gate: gate.gate_id,
            checkpoint: checkpoint.checkpoint_id,
            work_event,
        },
    )
}

fn request(projection: &ProjectStateProjection, refs: &FixtureRefs) -> GspCorpusRequest {
    GspCorpusRequest {
        query: "What remains before release?".to_owned(),
        project_ref: projection.state().project.project_id.clone(),
        authority_refs: vec![refs.authority.clone()],
        decision_refs: vec![refs.decision.clone()],
        gate_refs: vec![refs.gate.clone()],
        checkpoint_refs: vec![refs.checkpoint.clone()],
        material_event_refs: vec![refs.work_event.clone()],
        observed_at: timestamp(10),
        budget: GspCorpusBudget {
            max_items: 5,
            max_text_bytes: 16_384,
        },
    }
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
                EventId::new(),
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        self.push_with_id(EventId::new(), payload);
    }

    fn push_with_id(&mut self, event_id: EventId, payload: Event) {
        self.events.push(event(
            self.events.len() as u64 + 1,
            self.events.last().map(|value| value.event_digest),
            event_id,
            payload,
        ));
    }
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    event_id: EventId,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("gsp:corpus:{stream_version}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(stream_version),
    })
    .unwrap();
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id,
            event: payload,
            correlation_ref: id("correlation:gsp-corpus"),
            occurred_at: timestamp(stream_version),
            recorded_at: timestamp(stream_version),
        },
    )
    .unwrap()
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:spec"),
        uri: ResourceRef::parse("file:///C:/private/PROJECT_LEDGER_SPEC.md").unwrap(),
        digest: Sha256Digest::from([1; 32]),
        version: "0.2.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: timestamp(2),
        supersedes_ref: None,
    }
}

fn roadmap() -> Roadmap {
    Roadmap {
        schema_version: SchemaVersion::V0_2_0,
        roadmap_id: id("roadmap:main"),
        title: "Main roadmap".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:spec"),
    }
}

fn decision() -> Decision {
    with_digest(
        Decision {
            schema_version: SchemaVersion::V0_2_0,
            decision_id: id("decision:storage"),
            title: "Storage authority".to_owned(),
            domain: "storage".to_owned(),
            scope: "project".to_owned(),
            context: "Durable state".to_owned(),
            decision: "SQLite owns the event stream".to_owned(),
            rationale: "Deterministic recovery".to_owned(),
            alternatives: vec!["Memory only".to_owned()],
            status: DecisionStatus::Proposed,
            version: 1,
            authority_ref: None,
            evidence_refs: Vec::new(),
            supersedes_ref: None,
            created_at: timestamp(6),
            accepted_at: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/decision").unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn gate() -> GateDefinition {
    with_digest(
        GateDefinition {
            schema_version: SchemaVersion::V0_2_0,
            gate_id: id("gate:quality"),
            definition_version: 1,
            title: "Quality".to_owned(),
            phase_ref: id("phase:implementation"),
            criticality: GateCriticality::Required,
            required_evidence_rules: vec![RequiredEvidenceRule {
                rule_id: id("rule:tests"),
                kind: EvidenceKind::TestResult,
                predicate: PredicateAst::Exists {
                    path: "result.exit_code".to_owned(),
                },
            }],
            repository_dependencies: RepositoryDependencies {
                paths: Vec::new(),
                external_refs: Vec::new(),
            },
            completion_rule: GateCompletionOperator::All,
            waiver_policy: GateWaiverPolicy::Forbidden,
            freshness_policy: FreshnessPolicy {
                evidence_must_match_same_repository_snapshot: false,
            },
            revalidate_on: Vec::new(),
            created_at: timestamp(7),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn work() -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:gsp"),
            summary: "Exact GSP boundary under test".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            task_ref: Some(id("task:gsp")),
            files_or_artifacts: vec![ResourceRef::parse("workspace://secrets/build.log").unwrap()],
            commit_refs: vec!["deadbeef".to_owned()],
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(8),
            recorded_at: timestamp(8),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work").unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn checkpoint(
    projection: &ProjectStateProjection,
    checkpoint_event_ref: EventId,
) -> ContinuityCheckpoint {
    let state = projection.state();
    with_digest(
        ContinuityCheckpoint {
            schema_version: SchemaVersion::V0_2_0,
            checkpoint_id: id("checkpoint:gsp"),
            project_ref: state.project.project_id.clone(),
            captured_state_version: state.state_version,
            captured_material_state_version: state.material_state_version,
            captured_state_digest: state.state_digest,
            checkpoint_event_version: state.state_version + 1,
            checkpoint_event_ref,
            authority_document_refs: state
                .authority_documents
                .iter()
                .map(|document| document.document_id.clone())
                .collect(),
            active_phase_refs: state.active_phase_refs.clone(),
            primary_active_phase_ref: state.primary_active_phase_ref.clone(),
            verified_completed_phase_refs: state.completed_phase_refs.clone(),
            pending_gate_refs: state.pending_gate_refs.clone(),
            current_decision_refs: state.current_decision_refs.clone(),
            open_blocker_refs: state.open_blocker_refs.clone(),
            open_risk_refs: state.open_risk_refs.clone(),
            next_action_refs: state.ordered_next_action_refs.clone(),
            last_material_event_refs: state.last_material_event_ref.iter().cloned().collect(),
            repository_binding: None,
            unknown_refs: Vec::new(),
            conflict_refs: state.open_conflict_refs.clone(),
            created_by: id("actor:test"),
            created_at: timestamp(9),
            checkpoint_digest: zero(),
        },
        |value, digest| value.checkpoint_digest = digest,
    )
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "G-Ledger".to_owned(),
        summary: "GSP corpus fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(1),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198cf00-0000-7000-8000-000000000011").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn timestamp(index: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T10:{index:02}:00-04:00")).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}

fn with_digest<T: Serialize>(mut value: T, mut set: impl FnMut(&mut T, Sha256Digest)) -> T {
    set(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("digest");
    object.remove("payload_digest");
    object.remove("checkpoint_digest");
    let digest = sha256_digest(&json).unwrap();
    set(&mut value, digest);
    value
}
