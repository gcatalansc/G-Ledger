use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDigestObservation, AuthorityDocument, AuthorityDocumentStatus,
    AuthorityPolicy, BindRepositoryPayload, Command, CommandEnvelope, CommandId,
    CommandResultStatus, CreateProjectPayload, DefineGatePayload, DefineRoadmapPayload,
    EvidenceKind, EvidenceMetrics, EvidencePrivacy, EvidenceRecord, EvidenceResult, EvidenceSource,
    EvidenceSourceType, EvidenceStatus, FreshnessPolicy, GateCompletionOperator, GateCriticality,
    GateDefinition, GateStatus, GateWaiverPolicy, InvalidateEvidencePayload, LedgerTimestamp,
    PredicateAst, Project, ProjectId, ProjectLifecycleStatus, RecordVerificationResultPayload,
    RefreshAuthorityDocumentDigestPayload, RefreshRepositoryBindingPayload,
    RegisterAuthorityDocumentPayload, RepositoryBinding, RepositoryBindingStatus,
    RepositoryDependencies, RepositoryIdentityMethod, RepositorySnapshot,
    RequestGateEvaluationPayload, RequestGateRevalidationPayload, RequestPhaseCompletionPayload,
    RequiredEvidenceRule, ResourceRef, RevalidationTrigger, Roadmap, RoadmapCompletionPolicy,
    RoadmapExecutionModel, ScalarValue, SchemaVersion, StableId, StartPhasePayload, StorageProfile,
    TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde::Serialize;
use tempfile::tempdir;

#[test]
#[allow(clippy::too_many_lines)] // The test intentionally narrates one complete real-SQLite transaction.
fn real_sqlite_keeps_green_evidence_separate_then_verifies_gate_and_completes_phase() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());

    for (number, payload, actor_kind) in [
        (
            60,
            Command::CreateProject(CreateProjectPayload { project: project() }),
            ActorKind::System,
        ),
        (
            61,
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
            ActorKind::Human,
        ),
        (
            62,
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
            ActorKind::Human,
        ),
        (
            63,
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
        (
            64,
            Command::DefineGate(DefineGatePayload { gate: gate() }),
            ActorKind::Human,
        ),
        (
            65,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence(),
            }),
            ActorKind::System,
        ),
    ] {
        let expected = store.current_version(&project_id()).unwrap();
        let result = execute(
            &mut store,
            &clock,
            command(number, expected, payload, actor_kind),
        );
        assert_eq!(result.status, CommandResultStatus::Accepted);
    }

    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::Pending)
    );
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(project_ledger_domain::PhaseStatus::Active)
    );

    let version = store.current_version(&project_id()).unwrap();
    let premature = execute(
        &mut store,
        &clock,
        command(
            66,
            version,
            Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(premature.status, CommandResultStatus::ValidationFailed);
    assert_eq!(store.current_version(&project_id()).unwrap(), version);

    let evaluated = execute(
        &mut store,
        &clock,
        command(
            67,
            version,
            Command::RequestGateEvaluation(RequestGateEvaluationPayload {
                gate_ref: id("gate:tests"),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(evaluated.status, CommandResultStatus::Accepted);
    assert_eq!(evaluated.new_state_version, version + 3);

    let closed = execute(
        &mut store,
        &clock,
        command(
            68,
            evaluated.new_state_version,
            Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(closed.status, CommandResultStatus::Accepted);
    assert_eq!(closed.new_state_version, evaluated.new_state_version + 2);

    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::Verified)
    );
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(project_ledger_domain::PhaseStatus::Completed)
    );
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Completed
    );
    store.verify_event_chain(&project_id()).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)] // The full freshness timeline is easier to audit as one scenario.
fn invalidated_selected_evidence_reopens_then_explicit_revalidation_repairs_the_phase() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());
    let setup = [
        (
            70,
            Command::CreateProject(CreateProjectPayload { project: project() }),
            ActorKind::System,
        ),
        (
            71,
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
            ActorKind::Human,
        ),
        (
            72,
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
            ActorKind::Human,
        ),
        (
            73,
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
        (
            74,
            Command::DefineGate(DefineGatePayload { gate: gate() }),
            ActorKind::Human,
        ),
        (
            75,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence(),
            }),
            ActorKind::System,
        ),
        (
            76,
            Command::RequestGateEvaluation(RequestGateEvaluationPayload {
                gate_ref: id("gate:tests"),
            }),
            ActorKind::Human,
        ),
        (
            77,
            Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
    ];
    for (number, payload, actor_kind) in setup {
        let expected = store.current_version(&project_id()).unwrap();
        let result = execute(
            &mut store,
            &clock,
            command(number, expected, payload, actor_kind),
        );
        assert_eq!(result.status, CommandResultStatus::Accepted);
    }

    let before = store.current_version(&project_id()).unwrap();
    let invalidated = execute(
        &mut store,
        &clock,
        command(
            78,
            before,
            Command::InvalidateEvidence(InvalidateEvidencePayload {
                evidence_ref: id("evidence:tests"),
                reason: "repository snapshot changed".to_owned(),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(invalidated.status, CommandResultStatus::Accepted);
    assert_eq!(invalidated.new_state_version, before + 5);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.evidence(&id("evidence:tests")).unwrap().status,
        EvidenceStatus::Invalidated
    );
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::RevalidationRequired)
    );
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(project_ledger_domain::PhaseStatus::RevalidationRequired)
    );
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );

    let replacement = execute(
        &mut store,
        &clock,
        command(
            79,
            invalidated.new_state_version,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence_named("evidence:tests-replacement"),
            }),
            ActorKind::System,
        ),
    );
    assert_eq!(replacement.status, CommandResultStatus::Accepted);
    let revalidated = execute(
        &mut store,
        &clock,
        command(
            80,
            replacement.new_state_version,
            Command::RequestGateRevalidation(RequestGateRevalidationPayload {
                gate_ref: id("gate:tests"),
                signal_ref: Some(ResourceRef::parse("project-ledger://signals/retry").unwrap()),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(revalidated.status, CommandResultStatus::Accepted);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::Verified)
    );
    assert_eq!(
        projection
            .current_gate_evaluation(&id("gate:tests"))
            .unwrap()
            .selected_evidence_refs,
        vec![id("evidence:tests-replacement")]
    );

    let closed = execute(
        &mut store,
        &clock,
        command(
            81,
            revalidated.new_state_version,
            Command::RequestPhaseCompletion(RequestPhaseCompletionPayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(closed.status, CommandResultStatus::Accepted);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.phase_status(&id("phase:implementation")),
        Some(project_ledger_domain::PhaseStatus::Completed)
    );
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Completed
    );
    store.verify_event_chain(&project_id()).unwrap();
}

#[test]
fn pl_05_authority_digest_change_marks_a_verified_active_gate_for_revalidation() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());
    let setup = [
        (
            90,
            Command::CreateProject(CreateProjectPayload { project: project() }),
            ActorKind::System,
        ),
        (
            91,
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
            ActorKind::Human,
        ),
        (
            92,
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
            ActorKind::Human,
        ),
        (
            93,
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
        (
            94,
            Command::DefineGate(DefineGatePayload { gate: gate() }),
            ActorKind::Human,
        ),
        (
            95,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: evidence(),
            }),
            ActorKind::System,
        ),
        (
            96,
            Command::RequestGateEvaluation(RequestGateEvaluationPayload {
                gate_ref: id("gate:tests"),
            }),
            ActorKind::Human,
        ),
    ];
    for (number, payload, actor_kind) in setup {
        let expected = store.current_version(&project_id()).unwrap();
        let result = execute(
            &mut store,
            &clock,
            command(number, expected, payload, actor_kind),
        );
        assert_eq!(result.status, CommandResultStatus::Accepted);
    }
    let before = store.current_version(&project_id()).unwrap();
    let refreshed = execute(
        &mut store,
        &clock,
        command(
            97,
            before,
            Command::RefreshAuthorityDocumentDigest(RefreshAuthorityDocumentDigestPayload {
                document_ref: id("authority:canonical"),
                observation: AuthorityDigestObservation::Mismatch {
                    observed_digest: Sha256Digest::from([9_u8; 32]),
                },
            }),
            ActorKind::Human,
        ),
    );

    assert_eq!(refreshed.status, CommandResultStatus::Accepted);
    assert_eq!(refreshed.new_state_version, before + 4);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::RevalidationRequired)
    );
    assert_eq!(
        projection
            .authority_document(&id("authority:canonical"))
            .unwrap()
            .digest,
        Sha256Digest::from([1_u8; 32])
    );
    assert_eq!(
        projection
            .authority_document(&id("authority:canonical"))
            .unwrap()
            .status,
        AuthorityDocumentStatus::DigestMismatch
    );
}

#[test]
#[allow(clippy::too_many_lines)] // One scenario preserves the repository snapshot freshness timeline.
fn pl_05_repository_snapshot_change_marks_a_verified_gate_for_revalidation() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());
    let mut snapshot_gate = gate();
    snapshot_gate
        .freshness_policy
        .evidence_must_match_same_repository_snapshot = true;
    snapshot_gate
        .revalidate_on
        .push(RevalidationTrigger::RepositoryDependencyChanged);
    snapshot_gate.digest = zero();
    snapshot_gate = with_digest(snapshot_gate, |record, digest| record.digest = digest);
    let mut snapshot_evidence = evidence();
    snapshot_evidence.repository_binding = Some(snapshot("snapshot:one", 1));
    snapshot_evidence.payload_digest = zero();
    snapshot_evidence = with_digest(snapshot_evidence, |record, digest| {
        record.payload_digest = digest;
    });
    let setup = [
        (
            110,
            Command::CreateProject(CreateProjectPayload { project: project() }),
            ActorKind::System,
        ),
        (
            111,
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
            ActorKind::Human,
        ),
        (
            112,
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
            ActorKind::Human,
        ),
        (
            113,
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
            ActorKind::Human,
        ),
        (
            114,
            Command::BindRepository(BindRepositoryPayload {
                binding: repository_binding(),
            }),
            ActorKind::Human,
        ),
        (
            115,
            Command::DefineGate(DefineGatePayload {
                gate: snapshot_gate,
            }),
            ActorKind::Human,
        ),
        (
            116,
            Command::RecordVerificationResult(RecordVerificationResultPayload {
                evidence: snapshot_evidence,
            }),
            ActorKind::System,
        ),
        (
            117,
            Command::RequestGateEvaluation(RequestGateEvaluationPayload {
                gate_ref: id("gate:tests"),
            }),
            ActorKind::Human,
        ),
    ];
    for (number, payload, actor_kind) in setup {
        let expected = store.current_version(&project_id()).unwrap();
        let result = execute(
            &mut store,
            &clock,
            command(number, expected, payload, actor_kind),
        );
        assert_eq!(result.status, CommandResultStatus::Accepted);
    }

    let before = store.current_version(&project_id()).unwrap();
    let refreshed = execute(
        &mut store,
        &clock,
        command(
            118,
            before,
            Command::RefreshRepositoryBinding(RefreshRepositoryBindingPayload {
                binding_ref: id("binding:main"),
                snapshot: snapshot("snapshot:two", 2),
            }),
            ActorKind::Human,
        ),
    );
    assert_eq!(refreshed.status, CommandResultStatus::Accepted);
    assert_eq!(refreshed.new_state_version, before + 3);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(
        projection.gate_status(&id("gate:tests")),
        Some(GateStatus::RevalidationRequired)
    );
    assert_eq!(
        projection
            .repository_binding(&id("binding:main"))
            .unwrap()
            .current_snapshot_ref,
        Some(id("snapshot:two"))
    );
}

fn execute(
    store: &mut SqliteEventStore,
    clock: &FixedClock,
    command: CommandEnvelope,
) -> project_ledger_domain::CommandResult {
    CommandProcessor::new(store, clock)
        .execute(command, &CoreCommandHandler)
        .unwrap()
}

fn command(
    number: u8,
    expected_state_version: u64,
    payload: Command,
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
        idempotency_key: id(&format!("application:gate:{number}")),
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

fn gate() -> GateDefinition {
    with_digest(
        GateDefinition {
            schema_version: SchemaVersion::V0_2_0,
            gate_id: id("gate:tests"),
            definition_version: 1,
            title: "Tests".to_owned(),
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
            revalidate_on: vec![
                RevalidationTrigger::EvidenceInvalidated,
                RevalidationTrigger::AuthorityDocumentChanged,
            ],
            created_at: at(),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn evidence() -> EvidenceRecord {
    evidence_named("evidence:tests")
}

fn evidence_named(evidence_id: &str) -> EvidenceRecord {
    with_digest(
        EvidenceRecord {
            schema_version: SchemaVersion::V0_2_0,
            evidence_id: id(evidence_id),
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
            observed_at: at(),
            payload_digest: zero(),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/tests")
                .unwrap(),
        },
        |record, digest| record.payload_digest = digest,
    )
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:canonical"),
        uri: ResourceRef::parse("file:///authority/spec.md").unwrap(),
        digest: Sha256Digest::from([1_u8; 32]),
        version: "0.2.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: at(),
        supersedes_ref: None,
    }
}

fn repository_binding() -> RepositoryBinding {
    with_digest(
        RepositoryBinding {
            schema_version: SchemaVersion::V0_2_0,
            repository_binding_id: id("binding:main"),
            repository_ref: id("repository:main"),
            workspace_root_ref: ResourceRef::parse("workspace://project/").unwrap(),
            identity_method: RepositoryIdentityMethod::UserConfirmed,
            remote_identity_digest: None,
            current_snapshot_ref: Some(id("snapshot:one")),
            current_snapshot_digest: Some(Sha256Digest::from([1_u8; 32])),
            status: RepositoryBindingStatus::Current,
            confirmed_by: Some(id("actor:test")),
            created_at: at(),
            updated_at: at(),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/repository/main")
                .unwrap(),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn snapshot(snapshot_ref: &str, digest: u8) -> RepositorySnapshot {
    RepositorySnapshot {
        repository_ref: id("repository:main"),
        snapshot_ref: id(snapshot_ref),
        snapshot_digest: Sha256Digest::from([digest; 32]),
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
        title: "Gate transaction".to_owned(),
        summary: "S5 real SQLite".to_owned(),
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

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T15:00:00-04:00").unwrap()
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
