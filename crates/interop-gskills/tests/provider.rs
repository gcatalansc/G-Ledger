use std::collections::BTreeMap;

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, Blocker, BlockerOpenedEvent, BlockerStatus, CheckpointCreatedEvent, Command,
    CommandId, ConflictKind, ConflictOpenedEvent, ConflictRecord, ConflictStatus,
    ContinuityCheckpoint, CreateProjectPayload, Decision, DecisionAcceptedEvent,
    DecisionProposedEvent, DecisionStatus, Event, EventId, EvidenceKind, FreshnessPolicy,
    GateCompletionOperator, GateCriticality, GateDefinition, GateMarkedStaleEvent,
    GateRevalidationRequiredEvent, GateStatus, GateWaivedEvent, GateWaiver, GateWaiverPolicy,
    LedgerTimestamp, LikelihoodClass, NextAction, NextActionClassification, NextActionSetEvent,
    Phase, PhaseKind, PhaseStartedEvent, PhaseStatus, PredicateAst, Project, ProjectCreatedEvent,
    ProjectId, ProjectLifecycleStatus, ProjectStatusDerivedEvent, RepositoryBinding,
    RepositoryBindingStatus, RepositoryBoundEvent, RepositoryDependencies,
    RepositoryIdentityMethod, RepositorySnapshot, RequiredEvidenceRule, ResourceRef, Risk,
    RiskOpenedEvent, RiskStatus, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, SchemaVersion, ScopeKind, Severity, StableId, StorageProfile,
    TrustLevel, UnknownCriticality, UnknownRecord, UnknownRecordedEvent, UnknownStatus,
    WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_interop_gskills::{
    GSkillsProvider, ProviderContext, ProviderError, validate_read_contract,
};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;
use serde_json::{Value, json};

const REQUESTED_FIELDS: &[&str] = &[
    "project_state_version",
    "material_state_version",
    "project_state_digest",
    "authority_documents",
    "active_phases",
    "primary_active_phase_ref",
    "verified_completed_phases",
    "current_decisions",
    "open_blockers",
    "open_risks",
    "pending_gates",
    "revalidation_required_gates",
    "stale_gates",
    "next_actions",
    "checkpoint",
    "repository_binding",
    "recent_material_events",
    "last_material_activity_at",
    "unknowns",
    "conflicts",
];

#[test]
fn pl_10_exact_mapping_preserves_every_normative_continuity_field() {
    let (projection, context) = provider_fixture(false);
    let provider = GSkillsProvider::new(projection, context).unwrap();
    let request = full_request(&provider);

    let result = provider.project(&request).unwrap();

    validate_read_contract("project-continuity-projection", &result.payload).unwrap();
    validate_read_contract("provider-freshness-record", &result.freshness).unwrap();
    validate_read_contract("context-projection-envelope", &result.envelope).unwrap();
    assert_eq!(result.payload["project"]["state_version"], 22);
    assert_eq!(result.payload["project"]["material_state_version"], 21);
    assert_eq!(
        result.payload["project"]["state_digest"],
        provider.observable_state().state_digest.to_string()
    );
    assert_eq!(
        result.payload["authority_documents"][0]["ref"],
        "authority:spec"
    );
    assert_eq!(
        result.payload["active_phases"][0]["ref"],
        "phase:implementation"
    );
    assert_eq!(result.payload["active_phases"][0]["kind"], "IMPLEMENTATION");
    assert_eq!(result.payload["active_phases"][0]["status"], "ACTIVE");
    assert_eq!(
        result.payload["primary_active_phase_ref"],
        "phase:implementation"
    );
    assert_eq!(
        result.payload["current_decisions"][0]["ref"],
        "decision:sqlite"
    );
    assert_eq!(
        result.payload["current_decisions"][0]["decision_version"],
        1
    );
    assert_eq!(
        result.payload["open_blockers"][0]["ref"],
        "blocker:dependency"
    );
    assert_eq!(result.payload["open_risks"][0]["ref"], "risk:locking");
    assert_eq!(result.payload["gates"]["pending"][0]["ref"], "gate:pending");
    assert_eq!(
        result.payload["gates"]["revalidation_required"][0]["ref"],
        "gate:revalidation"
    );
    assert_eq!(result.payload["gates"]["stale"][0]["ref"], "gate:stale");
    assert_eq!(
        result.payload["next_actions"]["ordered"][0]["ref"],
        "action:eligible"
    );
    assert_eq!(
        result.payload["next_actions"]["ordered"][0]["classification"],
        "ELIGIBLE"
    );
    assert_eq!(
        result.payload["next_actions"]["ordered"][0]["status"],
        "ELIGIBLE"
    );
    assert_eq!(result.payload["checkpoint"]["captured_state_version"], 21);
    assert_eq!(
        result.payload["checkpoint"]["captured_material_state_version"],
        21
    );
    assert_eq!(result.payload["checkpoint"]["checkpoint_event_version"], 22);
    assert_eq!(result.payload["repository_binding"]["ref"], "binding:main");
    assert_eq!(
        result.payload["repository_binding"]["repository_snapshot_ref"],
        "snapshot:main"
    );
    assert_eq!(result.payload["unknowns"][0]["ref"], "unknown:repository");
    assert_eq!(result.payload["conflict_refs"][0], "conflict:source");
    assert_eq!(
        result.payload["recent_material_events"][0]["kind"],
        "WORK_PROGRESS_RECORDED"
    );
    assert_eq!(result.envelope["coverage"]["returned_fields"], 20);
    assert_eq!(result.envelope["coverage"]["omitted_fields"], json!([]));
    assert!(result.envelope["loss_report_ref"].is_null());
    assert_eq!(result.envelope["effective_authority_domains"], json!([]));
}

#[test]
fn provider_fails_closed_on_missing_phase_metadata_or_ambiguous_repository_binding() {
    let (projection, mut context) = provider_fixture(false);
    context.phase_catalog.clear();
    assert!(matches!(
        GSkillsProvider::new(projection, context),
        Err(ProviderError::PhaseMetadataMissing { .. })
    ));

    let (projection, context) = provider_fixture(true);
    assert!(matches!(
        GSkillsProvider::new(projection, context),
        Err(ProviderError::AmbiguousRepositoryBinding { count: 2 })
    ));
}

#[test]
fn request_identity_privacy_budget_and_state_consistency_are_enforced() {
    let (projection, context) = provider_fixture(false);
    let provider = GSkillsProvider::new(projection, context).unwrap();

    let mut wrong_identity = full_request(&provider);
    wrong_identity["identity"]["project_ref"] = json!("project:other");
    refresh_digest(&mut wrong_identity);
    assert!(matches!(
        provider.project(&wrong_identity),
        Err(ProviderError::IdentityMismatch { .. })
    ));

    let mut raw_events = full_request(&provider);
    raw_events["privacy"]["raw_events"] = json!(true);
    refresh_digest(&mut raw_events);
    assert!(matches!(
        provider.project(&raw_events),
        Err(ProviderError::PrivacyViolation { .. })
    ));

    let mut too_small = full_request(&provider);
    too_small["budget"]["max_items"] = json!(1);
    refresh_digest(&mut too_small);
    assert!(matches!(
        provider.project(&too_small),
        Err(ProviderError::BudgetExceeded {
            dimension: "items",
            ..
        })
    ));

    let mut stale = full_request(&provider);
    stale["consistency"]["expected_project_state_version"] = json!(21);
    refresh_digest(&mut stale);
    let result = provider.project(&stale).unwrap();
    assert_eq!(result.freshness["status"], "SUPERSEDED_BY_NEWER_CHECKPOINT");
    assert_eq!(result.envelope["delivery_status"], "DELIVERED");
}

fn full_request(provider: &GSkillsProvider) -> Value {
    let observable = provider.observable_state();
    let mut request = json!({
        "schema_version": "0.3.0",
        "context_projection_request_id": "projection-request:ledger-s9",
        "request_id": "req:ledger-s9",
        "traceparent": "00-00000000000000000000000000000001-0000000000000001-01",
        "provider_id": "project-ledger",
        "projection_kind": "PROJECT_CONTINUITY",
        "stage": "TASK_CONTEXT",
        "purpose": "Continue exact S9 implementation",
        "identity": {
            "project_ref": "project:mempilot",
            "repository_ref": "repository:main",
            "repository_snapshot_ref": "snapshot:main",
            "session_ref": "session:s9",
            "task_ref": "task:gskills-provider"
        },
        "structural_scope": {"path_refs": [], "symbol_refs": [], "impact_ref": null},
        "requested_fields": REQUESTED_FIELDS,
        "budget": {"max_items": 24, "max_tokens": 350, "max_output_bytes": 65536, "timeout_ms": 1000},
        "privacy": {
            "allowed_fields": ["ids", "titles", "statuses", "digests", "bounded_summaries"],
            "source_bodies": false,
            "raw_events": false
        },
        "consistency": {
            "expected_project_state_version": observable.state_version,
            "expected_material_state_version": observable.material_state_version,
            "expected_project_state_digest": observable.state_digest.to_string(),
            "require_freshness": true,
            "allow_partial": true
        },
        "write_capability": "NONE",
        "requested_at": "2026-08-23T08:22:00-04:00",
        "digest": Sha256Digest::from([0; 32]).to_string()
    });
    refresh_digest(&mut request);
    request
}

#[allow(clippy::too_many_lines)] // The literal event stream is the audited end-to-end source fixture.
fn provider_fixture(ambiguous_repository: bool) -> (ProjectStateProjection, ProviderContext) {
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
    let binding = repository_binding("main", 7);
    stream.push(Event::RepositoryBound(RepositoryBoundEvent {
        binding: binding.clone(),
    }));
    if ambiguous_repository {
        stream.push(Event::RepositoryBound(RepositoryBoundEvent {
            binding: repository_binding("secondary", 8),
        }));
    }
    for name in ["pending", "revalidation", "stale"] {
        stream.push(Event::GateDefined(
            project_ledger_domain::GateDefinedEvent { gate: gate(name) },
        ));
    }
    stream.push(Event::GateRevalidationRequired(
        GateRevalidationRequiredEvent {
            gate_ref: id("gate:revalidation"),
            prior_status: GateStatus::Pending,
            reason_codes: vec!["AUTHORITY_CHANGED".to_owned()],
        },
    ));
    let stale_gate = gate("stale");
    stream.push(Event::GateWaived(GateWaivedEvent {
        waiver: waiver(&stale_gate),
    }));
    stream.push(Event::GateMarkedStale(GateMarkedStaleEvent {
        gate_ref: id("gate:stale"),
        reason_codes: vec!["REPOSITORY_CHANGED".to_owned()],
    }));
    let decision = decision();
    stream.push(Event::DecisionProposed(DecisionProposedEvent {
        decision: decision.clone(),
    }));
    stream.push(Event::DecisionAccepted(DecisionAcceptedEvent {
        decision_ref: decision.decision_id,
        authority_ref: id("authority:spec"),
    }));
    stream.push(Event::BlockerOpened(BlockerOpenedEvent {
        blocker: blocker(),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Blocked,
        reason_codes: vec!["OPEN_PROJECT_BLOCKER".to_owned()],
    }));
    stream.push(Event::RiskOpened(RiskOpenedEvent { risk: risk() }));
    stream.push(Event::NextActionSet(NextActionSetEvent {
        action: action(),
    }));
    stream.push(Event::UnknownRecorded(UnknownRecordedEvent {
        unknown: unknown(),
    }));
    stream.push(Event::ConflictOpened(ConflictOpenedEvent {
        conflict: conflict(),
    }));
    let work_event_minute = stream.events.len() as u64 + 1;
    stream.push(Event::WorkProgressRecorded(WorkProgressRecordedEvent {
        work_record: work(work_event_minute),
    }));

    if !ambiguous_repository {
        let before_checkpoint = ProjectStateProjection::rebuild(&stream.events).unwrap();
        let checkpoint_event_ref = event_id(222);
        let checkpoint = checkpoint(&before_checkpoint, checkpoint_event_ref.clone(), &binding);
        stream.push_with_id(
            checkpoint_event_ref,
            Event::CheckpointCreated(CheckpointCreatedEvent { checkpoint }),
        );
    }
    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let phase = Phase {
        schema_version: SchemaVersion::V0_2_0,
        phase_id: id("phase:implementation"),
        title: "Persistence".to_owned(),
        ordinal: 1,
        kind: PhaseKind::Implementation,
        status: PhaseStatus::Active,
        required_gate_refs: vec![id("gate:pending"), id("gate:revalidation")],
        optional_gate_refs: vec![id("gate:stale")],
        started_at: Some(timestamp(4)),
        completed_at: None,
        reopened_at: None,
    };
    let context = ProviderContext {
        gskills_project_ref: "project:mempilot".to_owned(),
        external_project_ref: ResourceRef::parse("project-ledger://projects/mempilot").unwrap(),
        phase_catalog: BTreeMap::from([(phase.phase_id.clone(), phase)]),
        process_instance_id: id("provider-process:s9"),
        observed_at: timestamp(22),
    };
    (projection, context)
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
        idempotency_key: id(&format!("gskills:s9:{stream_version}")),
        actor: actor(),
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
            correlation_ref: id("correlation:gskills-s9"),
            occurred_at: timestamp(stream_version),
            recorded_at: timestamp(stream_version),
        },
    )
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "MemPilot".to_owned(),
        summary: "Exact provider fixture".to_owned(),
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

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:spec"),
        uri: ResourceRef::parse("workspace://MEMPILOT_SPEC_v0.2.0.md").unwrap(),
        digest: Sha256Digest::from([1; 32]),
        version: "0.2.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: timestamp(1),
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

fn gate(name: &str) -> GateDefinition {
    with_digest(
        GateDefinition {
            schema_version: SchemaVersion::V0_2_0,
            gate_id: id(&format!("gate:{name}")),
            definition_version: 1,
            title: format!("Gate {name}"),
            phase_ref: id("phase:implementation"),
            criticality: GateCriticality::Required,
            required_evidence_rules: vec![RequiredEvidenceRule {
                rule_id: id(&format!("rule:{name}")),
                kind: EvidenceKind::TestResult,
                predicate: PredicateAst::Exists {
                    path: "result.exit_code".to_owned(),
                },
            }],
            repository_dependencies: RepositoryDependencies {
                paths: vec![format!("crates/{name}/**")],
                external_refs: Vec::new(),
            },
            completion_rule: GateCompletionOperator::All,
            waiver_policy: GateWaiverPolicy::HumanApproval,
            freshness_policy: FreshnessPolicy {
                evidence_must_match_same_repository_snapshot: true,
            },
            revalidate_on: Vec::new(),
            created_at: timestamp(7),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn waiver(gate: &GateDefinition) -> GateWaiver {
    with_digest(
        GateWaiver {
            schema_version: SchemaVersion::V0_2_0,
            waiver_id: id("waiver:stale"),
            gate_ref: gate.gate_id.clone(),
            gate_definition_version: gate.definition_version,
            gate_definition_digest: gate.digest,
            actor_ref: id("actor:test"),
            reason: "bounded fixture waiver".to_owned(),
            scope: "TEST_ONLY".to_owned(),
            expires_at: None,
            review_condition: "repository change".to_owned(),
            authority_ref: id("authority:spec"),
            created_at: timestamp(11),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn decision() -> Decision {
    with_digest(
        Decision {
            schema_version: SchemaVersion::V0_2_0,
            decision_id: id("decision:sqlite"),
            title: "SQLite persistence".to_owned(),
            domain: "storage".to_owned(),
            scope: "project".to_owned(),
            context: "durable local state".to_owned(),
            decision: "Use SQLite".to_owned(),
            rationale: "deterministic recovery".to_owned(),
            alternatives: vec!["memory".to_owned()],
            status: DecisionStatus::Proposed,
            version: 1,
            authority_ref: None,
            evidence_refs: Vec::new(),
            supersedes_ref: None,
            created_at: timestamp(13),
            accepted_at: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/decision/sqlite")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn blocker() -> Blocker {
    with_digest(
        Blocker {
            schema_version: SchemaVersion::V0_2_0,
            blocker_id: id("blocker:dependency"),
            title: "Dependency unavailable".to_owned(),
            description: "External fixture dependency".to_owned(),
            scope_kind: ScopeKind::Project,
            scope_ref: id("scope:project"),
            severity: Severity::High,
            status: BlockerStatus::Open,
            source_kind: "OBSERVED".to_owned(),
            source_ref: None,
            owner_ref: None,
            evidence_refs: Vec::new(),
            opened_at: timestamp(15),
            updated_at: timestamp(15),
            resolved_at: None,
            resolution_summary: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/blocker/dependency")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn risk() -> Risk {
    with_digest(
        Risk {
            schema_version: SchemaVersion::V0_2_0,
            risk_id: id("risk:locking"),
            title: "SQLite locking".to_owned(),
            description: "Concurrent writer contention".to_owned(),
            scope_kind: ScopeKind::Project,
            scope_ref: id("scope:project"),
            severity: Severity::Medium,
            likelihood_class: LikelihoodClass::Possible,
            status: RiskStatus::Open,
            owner_ref: None,
            trigger_conditions: vec!["concurrent writer".to_owned()],
            mitigation_actions: vec!["bounded busy timeout".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: timestamp(17),
            updated_at: timestamp(17),
            closed_at: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/risk/locking").unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn action() -> NextAction {
    with_digest(
        NextAction {
            schema_version: SchemaVersion::V0_2_0,
            next_action_id: id("action:eligible"),
            title: "Implement SQLite repository".to_owned(),
            description: "Continue S9 through RED-GREEN".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            gate_ref: Some(id("gate:pending")),
            classification: NextActionClassification::Eligible,
            authority_ref: None,
            precondition_refs: Vec::new(),
            blocked_by_refs: Vec::new(),
            owner_ref: None,
            created_at: timestamp(18),
            updated_at: timestamp(18),
            completed_at: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/action/eligible")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn unknown() -> UnknownRecord {
    with_digest(
        UnknownRecord {
            schema_version: SchemaVersion::V0_2_0,
            unknown_id: id("unknown:repository"),
            subject_kind: "REPOSITORY".to_owned(),
            subject_ref: id("repository:main"),
            field_path: "remote".to_owned(),
            statement: "Remote availability is not verified".to_owned(),
            criticality: UnknownCriticality::Medium,
            status: UnknownStatus::Open,
            blocks: Vec::new(),
            required_evidence: vec!["remote observation".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: timestamp(19),
            resolved_at: None,
            resolution_ref: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/unknown/repository")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn conflict() -> ConflictRecord {
    ConflictRecord {
        schema_version: SchemaVersion::V0_2_0,
        conflict_id: id("conflict:source"),
        kind: ConflictKind::SourceDivergence,
        severity: Severity::High,
        domain: "repository".to_owned(),
        subject_refs: vec![id("repository:main")],
        source_refs: vec![ResourceRef::parse("project-ledger://source/repository/main").unwrap()],
        status: ConflictStatus::Open,
        required_action: "inspect source".to_owned(),
        opened_at: timestamp(20),
        resolved_at: None,
        resolution_ref: None,
    }
}

fn work(event_minute: u64) -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:s9-mapping"),
            summary: "Exact S9 mapping tests written".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            task_ref: Some(id("task:gskills-provider")),
            files_or_artifacts: Vec::new(),
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(event_minute),
            recorded_at: timestamp(event_minute),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/s9-mapping")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn repository_binding(name: &str, digest_byte: u8) -> RepositoryBinding {
    with_digest(
        RepositoryBinding {
            schema_version: SchemaVersion::V0_2_0,
            repository_binding_id: id(&format!("binding:{name}")),
            repository_ref: id(&format!("repository:{name}")),
            workspace_root_ref: ResourceRef::parse(&format!("workspace://repository/{name}"))
                .unwrap(),
            identity_method: RepositoryIdentityMethod::ExplicitId,
            remote_identity_digest: None,
            current_snapshot_ref: Some(id(&format!("snapshot:{name}"))),
            current_snapshot_digest: Some(Sha256Digest::from([digest_byte; 32])),
            status: RepositoryBindingStatus::Current,
            confirmed_by: Some(id("actor:test")),
            created_at: timestamp(6),
            updated_at: timestamp(6),
            provenance_ref: ResourceRef::parse(&format!(
                "project-ledger://provenance/repository/{name}"
            ))
            .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn checkpoint(
    projection: &ProjectStateProjection,
    checkpoint_event_ref: EventId,
    binding: &RepositoryBinding,
) -> ContinuityCheckpoint {
    let state = projection.state();
    with_digest(
        ContinuityCheckpoint {
            schema_version: SchemaVersion::V0_2_0,
            checkpoint_id: id("checkpoint:s9"),
            project_ref: state.project.project_id.clone(),
            captured_state_version: state.state_version,
            captured_material_state_version: state.material_state_version,
            captured_state_digest: state.state_digest,
            checkpoint_event_version: state.state_version + 1,
            checkpoint_event_ref,
            authority_document_refs: vec![id("authority:spec")],
            active_phase_refs: vec![id("phase:implementation")],
            primary_active_phase_ref: Some(id("phase:implementation")),
            verified_completed_phase_refs: Vec::new(),
            pending_gate_refs: state.pending_gate_refs.clone(),
            current_decision_refs: state.current_decision_refs.clone(),
            open_blocker_refs: state.open_blocker_refs.clone(),
            open_risk_refs: state.open_risk_refs.clone(),
            next_action_refs: state.ordered_next_action_refs.clone(),
            last_material_event_refs: state.last_material_event_ref.iter().cloned().collect(),
            repository_binding: Some(RepositorySnapshot {
                repository_ref: binding.repository_ref.clone(),
                snapshot_ref: binding.current_snapshot_ref.clone().unwrap(),
                snapshot_digest: binding.current_snapshot_digest.unwrap(),
            }),
            unknown_refs: projection.open_unknown_refs(),
            conflict_refs: state.open_conflict_refs.clone(),
            created_by: id("actor:test"),
            created_at: timestamp(22),
            checkpoint_digest: zero(),
        },
        |value, digest| value.checkpoint_digest = digest,
    )
}

fn refresh_digest(value: &mut Value) {
    value["digest"] = json!(zero().to_string());
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("digest");
    value["digest"] = json!(sha256_digest(&unsigned).unwrap().to_string());
}

fn with_digest<T: Clone + Serialize>(mut value: T, set: impl Fn(&mut T, Sha256Digest)) -> T {
    set(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("digest");
    object.remove("checkpoint_digest");
    set(&mut value, sha256_digest(&json).unwrap());
    value
}

fn actor() -> Actor {
    Actor {
        actor_id: id("actor:test"),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn event_id(suffix: u16) -> EventId {
    EventId::parse(&format!(
        "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7{suffix:04x}"
    ))
    .unwrap()
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T08:{minute:02}:00-04:00")).unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}
