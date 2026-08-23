use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, ClaimClassification, ClaimRecord, Command, CommandEnvelope, CommandId,
    CreateProjectPayload, DefineGatePayload, EngineIdentity, Event, EventId, EvidenceKind,
    EvidenceMetrics, EvidencePrivacy, EvidenceRecord, EvidenceResult, EvidenceSource,
    EvidenceSourceType, EvidenceStatus, FreshnessPolicy, GateCompletionOperator, GateCriticality,
    GateDefinition, GateEvaluationResult, GateStatus, GateWaiver, GateWaiverPolicy,
    LedgerTimestamp, PhaseStartedEvent, PredicateAst, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectStatusDerivedEvent, RecordClaimPayload,
    RecordVerificationResultPayload, RepositoryDependencies, RepositorySnapshot,
    RequestGateEvaluationPayload, RequestPhaseCompletionPayload, RequiredEvidenceRule, ResourceRef,
    RevalidationTrigger, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, RuleResult, ScalarValue, SchemaVersion, StableId, StorageProfile,
    TrustLevel, UpdateClaimStatusPayload, WaiveGatePayload,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_gate_engine::{GateEngine, GateEngineError};
use serde::Serialize;

#[test]
fn green_test_evidence_is_only_evidence_and_missing_build_keeps_gate_incomplete() {
    let mut stream = active_phase_stream();
    let gate = gate(true);
    let define = command(
        20,
        stream.events.len() as u64,
        Command::DefineGate(DefineGatePayload { gate: gate.clone() }),
        ActorKind::Human,
    );
    let defined = GateEngine::handle(&define, &stream.events).unwrap();
    assert!(matches!(defined[0].event, Event::GateDefined(_)));
    stream.append(&define, defined);

    let evidence = evidence();
    let record = command(
        21,
        stream.events.len() as u64,
        Command::RecordVerificationResult(RecordVerificationResultPayload {
            evidence: evidence.clone(),
        }),
        ActorKind::System,
    );
    let recorded = GateEngine::handle(&record, &stream.events).unwrap();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0].event, Event::VerificationRecorded(_)));
    stream.append(&record, recorded);

    let evaluate = command(
        22,
        stream.events.len() as u64,
        Command::RequestGateEvaluation(RequestGateEvaluationPayload {
            gate_ref: gate.gate_id.clone(),
        }),
        ActorKind::Human,
    );
    let first = GateEngine::handle(&evaluate, &stream.events).unwrap();
    let second = GateEngine::handle(&evaluate, &stream.events).unwrap();
    assert_drafts_eq(&first, &second);
    assert_eq!(first.len(), 2);
    let Event::GateEvaluated(evaluated) = &first[1].event else {
        panic!("second event must be GATE_EVALUATED");
    };
    assert_eq!(
        evaluated.evaluation.result,
        GateEvaluationResult::Incomplete
    );
    assert_eq!(
        evaluated.evaluation.proposed_gate_status,
        GateStatus::Pending
    );
    assert_eq!(
        evaluated
            .evaluation
            .rule_results
            .iter()
            .map(|result| result.result)
            .collect::<Vec<_>>(),
        vec![RuleResult::Missing, RuleResult::Pass]
    );
    stream.append(&evaluate, first);

    let close = command(
        23,
        stream.events.len() as u64,
        Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
            phase_ref: id("phase:implementation"),
        }),
        ActorKind::Human,
    );
    assert!(matches!(
        GateEngine::handle(&close, &stream.events),
        Err(GateEngineError::InvalidCommand { .. })
    ));
}

#[test]
fn any_completion_accepts_one_pass_rule_while_preserving_the_missing_rule() {
    let mut stream = active_phase_stream();
    let mut any_gate = gate(true);
    any_gate.completion_rule = GateCompletionOperator::Any;
    any_gate.digest = zero();
    any_gate = with_digest(any_gate, |value, digest| value.digest = digest);

    for (number, command_payload, actor_kind) in [
        (
            60,
            Command::DefineGate(DefineGatePayload {
                gate: any_gate.clone(),
            }),
            ActorKind::Human,
        ),
        (
            61,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence(),
            }),
            ActorKind::System,
        ),
    ] {
        let command = command(
            number,
            stream.events.len() as u64,
            command_payload,
            actor_kind,
        );
        let drafts = GateEngine::handle(&command, &stream.events).unwrap();
        stream.append(&command, drafts);
    }

    let evaluate = command(
        62,
        stream.events.len() as u64,
        Command::RequestGateEvaluation(RequestGateEvaluationPayload {
            gate_ref: any_gate.gate_id,
        }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&evaluate, &stream.events).unwrap();
    let Event::GateEvaluated(evaluated) = &drafts[1].event else {
        panic!("second event must be GATE_EVALUATED");
    };
    assert_eq!(evaluated.evaluation.result, GateEvaluationResult::Pass);
    assert_eq!(
        evaluated
            .evaluation
            .rule_results
            .iter()
            .map(|result| result.result)
            .collect::<Vec<_>>(),
        vec![RuleResult::Missing, RuleResult::Pass]
    );
    assert!(matches!(drafts[2].event, Event::GateVerified(_)));
}

#[test]
fn same_snapshot_policy_records_mixed_selected_snapshots_as_conflicted() {
    let mut stream = active_phase_stream();
    let mut snapshot_gate = gate(true);
    snapshot_gate
        .freshness_policy
        .evidence_must_match_same_repository_snapshot = true;
    snapshot_gate.digest = zero();
    snapshot_gate = with_digest(snapshot_gate, |value, digest| value.digest = digest);

    let mut test_evidence = evidence();
    test_evidence.repository_binding = Some(snapshot("snapshot:one", 1));
    test_evidence.payload_digest = zero();
    test_evidence = with_digest(test_evidence, |value, digest| {
        value.payload_digest = digest;
    });
    let mut build_evidence = evidence();
    build_evidence.evidence_id = id("evidence:build");
    build_evidence.kind = EvidenceKind::BuildResult;
    build_evidence.repository_binding = Some(snapshot("snapshot:two", 2));
    build_evidence.payload_digest = zero();
    build_evidence = with_digest(build_evidence, |value, digest| {
        value.payload_digest = digest;
    });

    for (number, command_payload, actor_kind) in [
        (
            70,
            Command::DefineGate(DefineGatePayload {
                gate: snapshot_gate.clone(),
            }),
            ActorKind::Human,
        ),
        (
            71,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: test_evidence,
            }),
            ActorKind::System,
        ),
        (
            72,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: build_evidence,
            }),
            ActorKind::System,
        ),
    ] {
        let command = command(
            number,
            stream.events.len() as u64,
            command_payload,
            actor_kind,
        );
        let drafts = GateEngine::handle(&command, &stream.events).unwrap();
        stream.append(&command, drafts);
    }

    let evaluate = command(
        73,
        stream.events.len() as u64,
        Command::RequestGateEvaluation(RequestGateEvaluationPayload {
            gate_ref: snapshot_gate.gate_id,
        }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&evaluate, &stream.events).unwrap();
    assert_eq!(drafts.len(), 2);
    let Event::GateEvaluated(evaluated) = &drafts[1].event else {
        panic!("second event must be GATE_EVALUATED");
    };
    assert_eq!(
        evaluated.evaluation.result,
        GateEvaluationResult::Conflicted
    );
    assert!(evaluated.evaluation.repository_binding.is_none());
    assert!(
        evaluated
            .evaluation
            .explanation_codes
            .contains(&"REPOSITORY_SNAPSHOT_CONFLICT".to_owned())
    );
}

#[test]
fn pass_evaluation_is_the_only_path_to_verified_and_explicit_phase_completion() {
    let mut stream = active_phase_stream();
    let gate = gate(false);
    for (number, command_payload, actor_kind) in [
        (
            30,
            Command::DefineGate(DefineGatePayload { gate: gate.clone() }),
            ActorKind::Human,
        ),
        (
            31,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence(),
            }),
            ActorKind::System,
        ),
    ] {
        let command = command(
            number,
            stream.events.len() as u64,
            command_payload,
            actor_kind,
        );
        let drafts = GateEngine::handle(&command, &stream.events).unwrap();
        stream.append(&command, drafts);
    }

    let evaluate = command(
        32,
        stream.events.len() as u64,
        Command::RequestGateEvaluation(RequestGateEvaluationPayload {
            gate_ref: gate.gate_id.clone(),
        }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&evaluate, &stream.events).unwrap();
    assert_eq!(drafts.len(), 3);
    assert!(matches!(drafts[0].event, Event::GateEvaluationRequested(_)));
    assert!(matches!(drafts[1].event, Event::GateEvaluated(_)));
    assert!(matches!(drafts[2].event, Event::GateVerified(_)));
    stream.append(&evaluate, drafts);

    let close = command(
        33,
        stream.events.len() as u64,
        Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
            phase_ref: id("phase:implementation"),
        }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&close, &stream.events).unwrap();
    assert_eq!(drafts.len(), 2);
    assert!(matches!(drafts[0].event, Event::PhaseCompleted(_)));
    assert!(matches!(drafts[1].event, Event::ProjectStatusDerived(_)));

    let agent_close = command(
        34,
        stream.events.len() as u64,
        Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
            phase_ref: id("phase:implementation"),
        }),
        ActorKind::Agent,
    );
    assert!(matches!(
        GateEngine::handle(&agent_close, &stream.events),
        Err(GateEngineError::InvalidCommand { .. })
    ));
}

#[test]
fn human_waiver_preserves_actor_reason_scope_review_authority_and_gate_version() {
    let mut stream = active_phase_stream();
    let gate = gate(false);
    let define = command(
        40,
        stream.events.len() as u64,
        Command::DefineGate(DefineGatePayload { gate: gate.clone() }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&define, &stream.events).unwrap();
    stream.append(&define, drafts);
    let waiver = with_digest(
        GateWaiver {
            schema_version: SchemaVersion::V0_2_0,
            waiver_id: id("waiver:quality"),
            gate_ref: gate.gate_id.clone(),
            gate_definition_version: gate.definition_version,
            gate_definition_digest: gate.digest,
            actor_ref: id("actor:test"),
            reason: "approved risk acceptance".to_owned(),
            scope: "phase:implementation".to_owned(),
            expires_at: None,
            review_condition: "review before release".to_owned(),
            authority_ref: id("authority:spec"),
            created_at: timestamp(41),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    );
    let waive = command(
        41,
        stream.events.len() as u64,
        Command::WaiveGate(WaiveGatePayload {
            waiver: waiver.clone(),
        }),
        ActorKind::Human,
    );
    let drafts = GateEngine::handle(&waive, &stream.events).unwrap();
    assert_eq!(drafts.len(), 1);
    assert!(matches!(drafts[0].event, Event::GateWaived(_)));
    stream.append(&waive, drafts);
    let projection =
        project_ledger_projection_engine::ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(projection.gate_waiver(&gate.gate_id), Some(&waiver));
    assert_eq!(
        projection.gate_status(&gate.gate_id),
        Some(GateStatus::Waived)
    );

    let close = command(
        42,
        stream.events.len() as u64,
        Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
            phase_ref: gate.phase_ref,
        }),
        ActorKind::Human,
    );
    assert!(matches!(
        GateEngine::handle(&close, &stream.events).unwrap()[0].event,
        Event::PhaseCompleted(_)
    ));

    let mut wrong_actor = waiver;
    wrong_actor.waiver_id = id("waiver:wrong-actor");
    wrong_actor.actor_ref = id("actor:other");
    wrong_actor.digest = zero();
    wrong_actor = with_digest(wrong_actor, |value, digest| value.digest = digest);
    let rejected = command(
        43,
        stream.events.len() as u64,
        Command::WaiveGate(WaiveGatePayload {
            waiver: wrong_actor,
        }),
        ActorKind::Human,
    );
    assert!(matches!(
        GateEngine::handle(&rejected, &stream.events),
        Err(GateEngineError::InvalidCommand { .. })
    ));
}

#[test]
fn ac_020_agent_claim_stays_asserted_and_cannot_become_verified_without_evidence() {
    let mut stream = active_phase_stream();
    let claim = ClaimRecord {
        schema_version: SchemaVersion::V0_2_0,
        claim_id: id("claim:implementation"),
        statement: "implementation is complete".to_owned(),
        classification: ClaimClassification::Asserted,
        subject_refs: vec![id("phase:implementation")],
        evidence_refs: Vec::new(),
        authority_ref: None,
    };
    let record = command(
        50,
        stream.events.len() as u64,
        Command::RecordClaim(RecordClaimPayload {
            claim: claim.clone(),
        }),
        ActorKind::Agent,
    );
    let drafts = GateEngine::handle(&record, &stream.events).unwrap();
    assert_eq!(drafts.len(), 1);
    assert!(matches!(drafts[0].event, Event::ClaimRecorded(_)));
    stream.append(&record, drafts);
    let projection =
        project_ledger_projection_engine::ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(projection.claim(&claim.claim_id), Some(&claim));

    let verify = command(
        51,
        stream.events.len() as u64,
        Command::UpdateClaimStatus(UpdateClaimStatusPayload {
            claim_ref: claim.claim_id,
            classification: ClaimClassification::Verified,
            evidence_refs: Vec::new(),
        }),
        ActorKind::Human,
    );
    assert!(matches!(
        GateEngine::handle(&verify, &stream.events),
        Err(GateEngineError::InvalidCommand { .. })
    ));
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

fn gate(include_build: bool) -> GateDefinition {
    let mut rules = vec![RequiredEvidenceRule {
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
    }];
    if include_build {
        rules.push(RequiredEvidenceRule {
            rule_id: id("rule:build-pass"),
            kind: EvidenceKind::BuildResult,
            predicate: PredicateAst::Equals {
                path: "result.exit_code".to_owned(),
                value: ScalarValue::Integer(0),
            },
        });
        rules.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    }
    with_digest(
        GateDefinition {
            schema_version: SchemaVersion::V0_2_0,
            gate_id: id("gate:quality"),
            definition_version: 1,
            title: "Quality".to_owned(),
            phase_ref: id("phase:implementation"),
            criticality: GateCriticality::Required,
            required_evidence_rules: rules,
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
            created_at: timestamp(10),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn evidence() -> EvidenceRecord {
    with_digest(
        EvidenceRecord {
            schema_version: SchemaVersion::V0_2_0,
            evidence_id: id("evidence:tests"),
            project_ref: project_id(),
            kind: EvidenceKind::TestResult,
            status: EvidenceStatus::VerifiedSource,
            source: EvidenceSource {
                source_type: EvidenceSourceType::CommandExecution,
                tool: "cargo".to_owned(),
                command_display: Some("cargo test".to_owned()),
                working_directory_ref: Some(ResourceRef::parse("workspace://project/").unwrap()),
                runner_version: "project-ledger/0.2.0".to_owned(),
            },
            result: EvidenceResult {
                exit_code: Some(0),
                duration_ms: 20,
                metrics: EvidenceMetrics {
                    passed: Some(10),
                    failed: Some(0),
                    skipped: Some(0),
                    errors: Some(0),
                },
                summary: "10 passed".to_owned(),
            },
            repository_binding: None,
            artifacts: Vec::new(),
            privacy: EvidencePrivacy {
                raw_output_stored: false,
                redaction_applied: true,
            },
            observed_at: timestamp(11),
            payload_digest: zero(),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/tests")
                .unwrap(),
        },
        |value, digest| value.payload_digest = digest,
    )
}

fn snapshot(snapshot_ref: &str, digest: u8) -> RepositorySnapshot {
    RepositorySnapshot {
        repository_ref: id("repository:main"),
        snapshot_ref: id(snapshot_ref),
        snapshot_digest: Sha256Digest::from([digest; 32]),
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
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        let envelope = event(
            self.events.len() as u64 + 1,
            self.events.last().map(|event| event.event_digest),
            payload,
        );
        self.events.push(envelope);
    }

    fn append(&mut self, command: &CommandEnvelope, drafts: Vec<EventDraft>) {
        let mut previous = self.events.last().map(|event| event.event_digest);
        for draft in drafts {
            let envelope =
                build_event(command, self.events.len() as u64 + 1, previous, draft).unwrap();
            previous = Some(envelope.event_digest);
            self.events.push(envelope);
        }
    }
}

fn command(
    number: u8,
    expected_state_version: u64,
    command: Command,
    actor_kind: ActorKind,
) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(&format!(
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
        ))
        .unwrap(),
        project_ref: project_id(),
        expected_state_version,
        idempotency_key: id(&format!("idempotency:gate:{number}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command,
        submitted_at: timestamp(u64::from(number)),
    })
    .unwrap()
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = command(
        u8::try_from(stream_version).unwrap(),
        stream_version.saturating_sub(1),
        Command::CreateProject(CreateProjectPayload { project: project() }),
        ActorKind::Human,
    );
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:gate-commands"),
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
        title: "Gate commands".to_owned(),
        summary: "S5".to_owned(),
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

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T12:{:02}:00-04:00", minute % 60)).unwrap()
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

fn assert_drafts_eq(left: &[EventDraft], right: &[EventDraft]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        assert_eq!(left.event_id, right.event_id);
        assert_eq!(left.event, right.event);
        assert_eq!(left.correlation_ref, right.correlation_ref);
        assert_eq!(left.occurred_at, right.occurred_at);
        assert_eq!(left.recorded_at, right.recorded_at);
    }
}

#[allow(dead_code)]
fn engine_identity() -> EngineIdentity {
    EngineIdentity {
        engine: "project-ledger-gate-engine".to_owned(),
        version: "0.2.0".to_owned(),
    }
}
