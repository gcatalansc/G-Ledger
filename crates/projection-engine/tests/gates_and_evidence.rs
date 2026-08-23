use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, Command, CommandId, CreateProjectPayload, EngineIdentity, Event, EventId,
    EvidenceInvalidatedEvent, EvidenceKind, EvidenceMetrics, EvidencePrivacy, EvidenceRecord,
    EvidenceResult, EvidenceSource, EvidenceSourceType, EvidenceStatus, FreshnessPolicy,
    GateCompletionOperator, GateCriticality, GateDefinedEvent, GateDefinition, GateEvaluatedEvent,
    GateEvaluation, GateEvaluationRequestedEvent, GateEvaluationResult, GateMarkedStaleEvent,
    GateRevalidationRequiredEvent, GateRuleResult, GateStatus, GateVerifiedEvent, GateWaiverPolicy,
    LedgerTimestamp, PhaseCompletedEvent, PhaseReopenedEvent, PhaseStartedEvent, PhaseStatus,
    PredicateAst, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectStatusDerivedEvent, RepositoryDependencies, RequiredEvidenceRule, ResourceRef,
    RevalidationTrigger, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, RuleResult, ScalarValue, SchemaVersion, StableId, StorageProfile,
    TrustLevel, VerificationRecordedEvent,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectStateProjector, ProjectionError,
};
use serde::Serialize;

#[test]
fn verified_gate_requires_accepted_evidence_and_matching_immutable_evaluation() {
    let mut stream = active_phase_stream();
    let gate = gate();
    let evidence = evidence();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate.clone() }));
    stream.push(Event::VerificationRecorded(VerificationRecordedEvent {
        evidence: evidence.clone(),
    }));
    stream.push(Event::GateEvaluationRequested(
        GateEvaluationRequestedEvent {
            gate_ref: gate.gate_id.clone(),
        },
    ));
    let evaluation = evaluation(stream.events.len() as u64 - 1, &gate, &evidence);
    stream.push(Event::GateEvaluated(GateEvaluatedEvent {
        evaluation: evaluation.clone(),
    }));
    stream.push(Event::GateVerified(GateVerifiedEvent {
        gate_ref: gate.gate_id.clone(),
        evaluation_ref: evaluation.gate_evaluation_id.clone(),
    }));
    stream.push(Event::PhaseCompleted(PhaseCompletedEvent {
        phase_ref: id("phase:implementation"),
        gate_evaluation_refs: vec![evaluation.gate_evaluation_id.clone()],
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Completed,
        reason_codes: vec!["REQUEST_PHASE_COMPLETION".to_owned()],
    }));

    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();

    assert_eq!(projection.gate(&gate.gate_id), Some(&gate));
    assert_eq!(projection.evidence(&evidence.evidence_id), Some(&evidence));
    assert_eq!(
        projection.gate_status(&gate.gate_id),
        Some(GateStatus::Verified)
    );
    assert_eq!(
        projection.gate_evaluation(&evaluation.gate_evaluation_id),
        Some(&evaluation)
    );
    assert_eq!(projection.state().verified_gate_refs, vec![gate.gate_id]);
    assert!(projection.state().pending_gate_refs.is_empty());
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(PhaseStatus::Completed)
    );
    assert_eq!(
        ProjectStateProjector::rebuild(&stream.events).unwrap(),
        projection.into_state()
    );
}

#[test]
fn phase_completion_with_an_unsatisfied_required_gate_fails_closed() {
    let mut stream = active_phase_stream();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate() }));
    stream.push(Event::PhaseCompleted(PhaseCompletedEvent {
        phase_ref: id("phase:implementation"),
        gate_evaluation_refs: Vec::new(),
    }));

    assert!(matches!(
        ProjectStateProjection::rebuild(&stream.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

#[test]
fn forged_gate_verified_without_current_pass_evaluation_fails_closed() {
    let mut stream = active_phase_stream();
    let gate = gate();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate.clone() }));
    stream.push(Event::GateVerified(GateVerifiedEvent {
        gate_ref: gate.gate_id,
        evaluation_ref: id("gate-eval:forged"),
    }));

    assert!(matches!(
        ProjectStateProjection::rebuild(&stream.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

#[test]
fn pass_evaluation_with_a_non_pass_rule_result_is_not_valid_evidence_of_verification() {
    let mut stream = active_phase_stream();
    let gate = gate();
    let evidence = evidence();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate.clone() }));
    stream.push(Event::VerificationRecorded(VerificationRecordedEvent {
        evidence: evidence.clone(),
    }));
    stream.push(Event::GateEvaluationRequested(
        GateEvaluationRequestedEvent {
            gate_ref: gate.gate_id.clone(),
        },
    ));
    let mut forged = evaluation(stream.events.len() as u64 - 1, &gate, &evidence);
    forged.rule_results[0].result = RuleResult::Missing;
    forged.digest = zero();
    forged = with_digest(forged, |record, digest| record.digest = digest);
    stream.push(Event::GateEvaluated(GateEvaluatedEvent {
        evaluation: forged,
    }));

    assert!(matches!(
        ProjectStateProjection::rebuild(&stream.events),
        Err(ProjectionError::LifecycleInvariant { .. })
    ));
}

#[test]
fn invalidated_selected_evidence_revalidates_gate_and_reopens_completed_phase() {
    let (mut stream, gate, evidence, evaluation) = completed_gate_stream();
    let invalidated = stream.push(Event::EvidenceInvalidated(EvidenceInvalidatedEvent {
        evidence_ref: evidence.evidence_id.clone(),
        reason: "repository snapshot changed".to_owned(),
    }));
    let mut incremental =
        ProjectStateProjection::rebuild(&stream.events[..stream.events.len() - 1])
            .unwrap()
            .apply(
                stream.events[stream.events.len() - 2].event_digest,
                &invalidated,
            )
            .unwrap();
    let stale = stream.push(Event::GateMarkedStale(GateMarkedStaleEvent {
        gate_ref: gate.gate_id.clone(),
        reason_codes: vec!["EVIDENCE_INVALIDATED".to_owned()],
    }));
    incremental = incremental.apply(invalidated.event_digest, &stale).unwrap();
    let required = stream.push(Event::GateRevalidationRequired(
        GateRevalidationRequiredEvent {
            gate_ref: gate.gate_id.clone(),
            prior_status: GateStatus::Stale,
            reason_codes: vec!["EVIDENCE_INVALIDATED".to_owned()],
        },
    ));
    incremental = incremental.apply(stale.event_digest, &required).unwrap();
    let reopened = stream.push(Event::PhaseReopened(PhaseReopenedEvent {
        phase_ref: gate.phase_ref.clone(),
        resulting_status: PhaseStatus::RevalidationRequired,
        reason: format!("evidence invalidated: {}", evidence.evidence_id),
    }));
    incremental = incremental.apply(required.event_digest, &reopened).unwrap();
    let status = stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Completed,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["INVALIDATE_EVIDENCE".to_owned()],
    }));
    incremental = incremental.apply(reopened.event_digest, &status).unwrap();

    let rebuilt = ProjectStateProjection::rebuild(&stream.events).unwrap();

    assert_eq!(incremental.state(), rebuilt.state());
    assert_eq!(
        rebuilt.evidence(&evidence.evidence_id).unwrap().status,
        EvidenceStatus::Invalidated
    );
    assert_eq!(
        rebuilt.gate_status(&gate.gate_id),
        Some(GateStatus::RevalidationRequired)
    );
    assert_eq!(
        rebuilt.state().revalidation_required_gate_refs,
        vec![gate.gate_id.clone()]
    );
    assert_eq!(
        rebuilt.phase_status(&gate.phase_ref),
        Some(PhaseStatus::RevalidationRequired)
    );
    assert_eq!(
        rebuilt
            .gate_evaluation(&evaluation.gate_evaluation_id)
            .unwrap()
            .selected_evidence_refs,
        vec![evidence.evidence_id]
    );
}

fn completed_gate_stream() -> (Stream, GateDefinition, EvidenceRecord, GateEvaluation) {
    let mut stream = active_phase_stream();
    let gate = gate();
    let evidence = evidence();
    stream.push(Event::GateDefined(GateDefinedEvent { gate: gate.clone() }));
    stream.push(Event::VerificationRecorded(VerificationRecordedEvent {
        evidence: evidence.clone(),
    }));
    stream.push(Event::GateEvaluationRequested(
        GateEvaluationRequestedEvent {
            gate_ref: gate.gate_id.clone(),
        },
    ));
    let evaluation = evaluation(stream.events.len() as u64 - 1, &gate, &evidence);
    stream.push(Event::GateEvaluated(GateEvaluatedEvent {
        evaluation: evaluation.clone(),
    }));
    stream.push(Event::GateVerified(GateVerifiedEvent {
        gate_ref: gate.gate_id.clone(),
        evaluation_ref: evaluation.gate_evaluation_id.clone(),
    }));
    stream.push(Event::PhaseCompleted(PhaseCompletedEvent {
        phase_ref: gate.phase_ref.clone(),
        gate_evaluation_refs: vec![evaluation.gate_evaluation_id.clone()],
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Active,
        new_status: ProjectLifecycleStatus::Completed,
        reason_codes: vec!["REQUEST_PHASE_COMPLETION".to_owned()],
    }));
    (stream, gate, evidence, evaluation)
}

fn active_phase_stream() -> Stream {
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

fn gate() -> GateDefinition {
    with_digest(
        GateDefinition {
            schema_version: SchemaVersion::V0_2_0,
            gate_id: id("gate:tests"),
            definition_version: 1,
            title: "Tests pass".to_owned(),
            phase_ref: id("phase:implementation"),
            criticality: GateCriticality::Required,
            required_evidence_rules: vec![RequiredEvidenceRule {
                rule_id: id("rule:tests-pass"),
                kind: EvidenceKind::TestResult,
                predicate: PredicateAst::All {
                    predicates: vec![
                        PredicateAst::Equals {
                            path: "result.exit_code".to_owned(),
                            value: ScalarValue::Integer(0),
                        },
                        PredicateAst::Equals {
                            path: "result.metrics.failed".to_owned(),
                            value: ScalarValue::Unsigned(0),
                        },
                    ],
                },
            }],
            repository_dependencies: RepositoryDependencies {
                paths: vec!["crates/**".to_owned()],
                external_refs: Vec::new(),
            },
            completion_rule: GateCompletionOperator::All,
            waiver_policy: GateWaiverPolicy::HumanApproval,
            freshness_policy: FreshnessPolicy {
                evidence_must_match_same_repository_snapshot: false,
            },
            revalidate_on: vec![RevalidationTrigger::EvidenceInvalidated],
            created_at: timestamp(5),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn evidence() -> EvidenceRecord {
    with_digest(
        EvidenceRecord {
            schema_version: SchemaVersion::V0_2_0,
            evidence_id: id("evidence:tests-1"),
            project_ref: project_id(),
            kind: EvidenceKind::TestResult,
            status: EvidenceStatus::VerifiedSource,
            source: EvidenceSource {
                source_type: EvidenceSourceType::CommandExecution,
                tool: "cargo".to_owned(),
                command_display: Some("cargo test --workspace".to_owned()),
                working_directory_ref: Some(
                    ResourceRef::parse("workspace://project-ledger/").unwrap(),
                ),
                runner_version: "project-ledger/0.2.0".to_owned(),
            },
            result: EvidenceResult {
                exit_code: Some(0),
                duration_ms: 123,
                metrics: EvidenceMetrics {
                    passed: Some(10),
                    failed: Some(0),
                    skipped: Some(0),
                    errors: Some(0),
                },
                summary: "10 passed, 0 failed".to_owned(),
            },
            repository_binding: None,
            artifacts: Vec::new(),
            privacy: EvidencePrivacy {
                raw_output_stored: false,
                redaction_applied: true,
            },
            observed_at: timestamp(6),
            payload_digest: zero(),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/tests-1")
                .unwrap(),
        },
        |record, digest| record.payload_digest = digest,
    )
}

fn evaluation(
    project_state_version: u64,
    gate: &GateDefinition,
    evidence: &EvidenceRecord,
) -> GateEvaluation {
    with_digest(
        GateEvaluation {
            schema_version: SchemaVersion::V0_2_0,
            gate_evaluation_id: id("gate-eval:tests-1"),
            gate_ref: gate.gate_id.clone(),
            gate_definition_version: gate.definition_version,
            gate_definition_digest: gate.digest,
            project_state_version,
            repository_binding: None,
            selected_evidence_refs: vec![evidence.evidence_id.clone()],
            rejected_evidence: Vec::new(),
            rule_results: vec![GateRuleResult {
                rule_id: id("rule:tests-pass"),
                result: RuleResult::Pass,
            }],
            result: GateEvaluationResult::Pass,
            proposed_gate_status: GateStatus::Verified,
            explanation_codes: Vec::new(),
            evaluated_by: EngineIdentity {
                engine: "project-ledger-gate-engine".to_owned(),
                version: "0.2.0".to_owned(),
            },
            evaluated_at: timestamp(8),
            provenance_ref: ResourceRef::parse(
                "project-ledger://provenance/gate-evaluations/tests-1",
            )
            .unwrap(),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn with_digest<T: Clone + Serialize>(mut value: T, set_digest: impl Fn(&mut T, Sha256Digest)) -> T {
    set_digest(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("digest");
    object.remove("payload_digest");
    let digest = sha256_digest(&json).unwrap();
    set_digest(&mut value, digest);
    value
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

    fn push(&mut self, payload: Event) -> project_ledger_domain::EventEnvelope {
        let envelope = event(
            self.events.len() as u64 + 1,
            self.events.last().map(|event| event.event_digest),
            payload,
        );
        self.events.push(envelope.clone());
        envelope
    }
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("projection:gates:{stream_version}")),
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
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:gates-projection"),
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
        uri: ResourceRef::parse("file:///authority/spec.md").unwrap(),
        digest: Sha256Digest::from([1_u8; 32]),
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
        title: "Main".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:spec"),
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Gates projection".to_owned(),
        summary: "S5 deterministic fixture".to_owned(),
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
    LedgerTimestamp::parse(&format!("2026-08-23T12:{minute:02}:00-04:00")).unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0_u8; 32])
}
