mod gsp_support;

use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, LedgerTimestamp, Project, ProjectLifecycleStatus, ResourceRef,
    SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_interop_gsp::{GspBridge, GspBridgeError};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::json;

#[test]
fn exact_six_contract_result_is_deterministic_and_cross_artifact_consistent() {
    let corpus = gsp_support::corpus();
    let raw = gsp_support::exact_result(&corpus);
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();

    let first = bridge.validate_result(&corpus, raw.clone()).unwrap();
    let second = bridge.validate_result(&corpus, raw).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.report.exact_contract_count, 6);
    assert_eq!(
        first.receipt.request_id,
        corpus.evidence_bundle["request_id"]
    );
    assert_eq!(first.receipt.source_refs, vec!["authority:spec"]);
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../integrations/gsp/schemas/ledger-gsp-compilation-receipt.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&serde_json::to_value(&first.receipt).unwrap())
        .unwrap();
}

#[test]
fn tampered_or_resealed_corpus_manifest_cannot_influence_the_receipt() {
    let corpus = gsp_support::corpus();
    let raw = gsp_support::exact_result(&corpus);
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();

    let mut tampered = corpus.clone();
    tampered.manifest.project_state_version += 1;
    assert!(
        bridge.validate_result(&tampered, raw.clone()).is_err(),
        "a manifest whose canonical digest no longer matches must be rejected"
    );

    let mut resealed = corpus;
    resealed.manifest.project_state_version += 1;
    resealed.manifest.corpus_digest = Sha256Digest::from([0; 32]);
    resealed.manifest.corpus_digest = sha256_digest(&resealed.manifest).unwrap();
    assert!(
        bridge.validate_result(&resealed, raw).is_err(),
        "a resealed manifest must still match the exact EvidenceBundle it describes"
    );
}

#[test]
fn reduced_mismatched_or_overclaimed_results_fail_closed() {
    let corpus = gsp_support::corpus();
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();

    let mut reduced = gsp_support::exact_result(&corpus);
    reduced
        .as_object_mut()
        .unwrap()
        .remove("task_projection_bundle");
    assert!(matches!(
        bridge.validate_result(&corpus, reduced),
        Err(GspBridgeError::MalformedResult(_))
    ));

    let mut mismatch = gsp_support::exact_result(&corpus);
    mismatch["candidate_bundle"]["request_id"] = json!("018f5d0a-9999-7999-8999-999999999999");
    assert!(matches!(
        bridge.validate_result(&corpus, mismatch),
        Err(GspBridgeError::RequestIdMismatch { .. })
    ));

    let mut overclaim = gsp_support::exact_result(&corpus);
    overclaim["validation"]["contracts_checked"] = json!(["evidence.bundle"]);
    assert!(matches!(
        bridge.validate_result(&corpus, overclaim),
        Err(GspBridgeError::InvalidRuntimeValidation { .. })
    ));

    let mut lifecycle_field = gsp_support::exact_result(&corpus);
    lifecycle_field["complete_phase"] = json!("phase:implementation");
    assert!(matches!(
        bridge.validate_result(&corpus, lifecycle_field),
        Err(GspBridgeError::MalformedResult(_))
    ));
}

#[test]
fn explicit_native_link_appends_one_evidence_event_without_lifecycle_authority() {
    let corpus = gsp_support::corpus();
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();
    let output = bridge
        .validate_result(&corpus, gsp_support::exact_result(&corpus))
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());

    let create = command(
        0,
        "gsp:link:create",
        Command::CreateProject(CreateProjectPayload { project: project() }),
    );
    let created = CommandProcessor::new(&mut store, &clock)
        .execute(create, &CoreCommandHandler)
        .unwrap();
    assert_eq!(created.status, CommandResultStatus::Accepted);
    let before = ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap())
        .unwrap()
        .state()
        .project
        .lifecycle_status;

    let link = command(
        1,
        "gsp:link:receipt",
        Command::LinkExternalEvidence(
            output
                .receipt
                .link_payload(ResourceRef::parse("artifact://gsp/receipt.json").unwrap()),
        ),
    );
    let linked = CommandProcessor::new(&mut store, &clock)
        .execute(link.clone(), &CoreCommandHandler)
        .unwrap();
    let replay = CommandProcessor::new(&mut store, &clock)
        .execute(link, &CoreCommandHandler)
        .unwrap();

    assert_eq!(linked.status, CommandResultStatus::Accepted);
    assert_eq!(linked.emitted_event_refs.len(), 1);
    assert_eq!(replay.status, CommandResultStatus::DuplicateReplay);
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    assert_eq!(projection.state().project.lifecycle_status, before);
    assert!(projection.state().active_phase_refs.is_empty());
    assert!(projection.state().verified_gate_refs.is_empty());
}

fn command(expected: u64, key: &str, payload: Command) -> project_ledger_domain::CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(key),
        actor: Actor {
            actor_id: id("actor:gsp-link"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: at(),
    })
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "GSP link boundary".to_owned(),
        summary: "No lifecycle authority".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn project_id() -> project_ledger_domain::ProjectId {
    project_ledger_domain::ProjectId::parse("project:0198cf00-0000-7000-8000-000000000021").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}
