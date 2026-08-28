use std::{fs, path::Path};

use assert_cmd::Command;
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, CheckpointCreatedEvent, Command as LedgerCommand, CommandId,
    ContinuityCheckpoint, CreateProjectPayload, Decision, DecisionProposedEvent, DecisionStatus,
    Event, EventId, EvidenceKind, FreshnessPolicy, GateCompletionOperator, GateCriticality,
    GateDefinedEvent, GateDefinition, GateWaiverPolicy, LedgerTimestamp, PredicateAst, Project,
    ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, RepositoryDependencies,
    RequiredEvidenceRule, ResourceRef, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, SchemaVersion, StableId, StorageProfile, TrustLevel,
    WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{
    CommandDraft, EventDraft, EventImportStore, EventStore, build_event, prepare_command,
};
use project_ledger_interop_gsp::{GspCorpus, GspCorpusBudget, GspCorpusRequest};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::TempDir;

#[path = "gsp_live/mod.rs"]
mod live_sidecar;

#[path = "gsp_live/supervised.rs"]
mod supervised;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

#[test]
fn cli_verifies_exact_gsp_contracts() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "gsp", "verify-contracts"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "gsp contracts MATCH registry=28 schemas=31 fixtures=140",
        ));
}

#[test]
#[allow(clippy::too_many_lines)] // One test narrates the complete read/read/explicit-write SQLite workflow.
fn cli_exports_validates_and_explicitly_links_without_hidden_mutation() {
    let fixture = Fixture::new();
    let before = event_count(&fixture.database);

    cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&fixture.corpus)
        .assert()
        .success();
    assert_eq!(event_count(&fixture.database), before);
    let corpus: GspCorpus = read_json(&fixture.corpus);
    assert_eq!(corpus.manifest.files.len(), 5);

    fs::write(
        &fixture.result,
        serde_json::to_vec(&exact_result(&corpus)).unwrap(),
    )
    .unwrap();
    cli()
        .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
        .arg(&fixture.corpus)
        .arg("--result")
        .arg(&fixture.result)
        .arg("--output")
        .arg(&fixture.validated)
        .assert()
        .success();
    assert_eq!(event_count(&fixture.database), before);
    let validated: Value = read_json(&fixture.validated);
    assert_eq!(validated["report"]["authority_elevation_count"], 0);
    fs::write(
        &fixture.receipt,
        serde_json::to_vec(&validated["receipt"]).unwrap(),
    )
    .unwrap();

    let link_project = project_id();
    let link_args = [
        "integration",
        "gsp",
        "link-result",
        "--project-ref",
        link_project.as_str(),
        "--receipt-uri",
        "artifact://gsp/validated-receipt.json",
        "--expected-state-version",
        "7",
        "--idempotency-key",
        "gsp:cli:link",
        "--actor-id",
        "actor:gsp-cli",
        "--observed-at",
        "2026-08-23T13:00:00-04:00",
    ];

    let forged_receipt = fixture.root.path().join("forged-receipt.json");
    let mut forged = validated["receipt"].clone();
    forged["project_state_digest"] = json!(Sha256Digest::from([99; 32]));
    let mut unsigned = forged.clone();
    unsigned.as_object_mut().unwrap().remove("receipt_digest");
    forged["receipt_digest"] = json!(sha256_digest(&unsigned).unwrap());
    fs::write(&forged_receipt, serde_json::to_vec(&forged).unwrap()).unwrap();
    cli()
        .args([
            "integration",
            "gsp",
            "link-result",
            "--project-ref",
            link_project.as_str(),
            "--receipt-uri",
            "artifact://gsp/forged-receipt.json",
            "--expected-state-version",
            "7",
            "--idempotency-key",
            "gsp:cli:forged-state",
            "--actor-id",
            "actor:gsp-cli",
            "--observed-at",
            "2026-08-23T13:00:00-04:00",
        ])
        .arg("--database")
        .arg(&fixture.database)
        .arg("--receipt")
        .arg(&forged_receipt)
        .assert()
        .failure()
        .stderr(predicates::str::contains("receipt state binding differs"));
    assert_eq!(event_count(&fixture.database), before);

    cli()
        .args(link_args)
        .arg("--database")
        .arg(&fixture.database)
        .arg("--receipt")
        .arg(&fixture.receipt)
        .assert()
        .success()
        .stdout(predicates::str::contains("\"status\":\"ACCEPTED\""));
    assert_eq!(event_count(&fixture.database), before + 1);
    let projection = projection(&fixture.database);
    assert_eq!(
        projection.state().project.lifecycle_status,
        ProjectLifecycleStatus::Initializing
    );
    assert!(projection.state().active_phase_refs.is_empty());
    assert!(projection.state().verified_gate_refs.is_empty());

    cli()
        .args(link_args)
        .arg("--database")
        .arg(&fixture.database)
        .arg("--receipt")
        .arg(&fixture.receipt)
        .assert()
        .success()
        .stdout(predicates::str::contains("\"status\":\"DUPLICATE_REPLAY\""));
    assert_eq!(event_count(&fixture.database), before + 1);

    cli()
        .args([
            "integration",
            "gsp",
            "link-result",
            "--project-ref",
            link_project.as_str(),
            "--receipt-uri",
            "artifact://gsp/validated-receipt.json",
            "--expected-state-version",
            "6",
            "--idempotency-key",
            "gsp:cli:stale",
            "--actor-id",
            "actor:gsp-cli",
            "--observed-at",
            "2026-08-23T13:00:00-04:00",
        ])
        .arg("--database")
        .arg(&fixture.database)
        .arg("--receipt")
        .arg(&fixture.receipt)
        .assert()
        .failure()
        .stderr(predicates::str::contains("receipt state version differs"));
    assert_eq!(event_count(&fixture.database), before + 1);
}

#[test]
fn cli_rejects_unknown_scope_overwrite_reduced_result_and_stale_link() {
    let fixture = Fixture::new();
    let mut request: GspCorpusRequest = read_json(&fixture.request);
    request.authority_refs = vec![id("authority:missing")];
    fs::write(&fixture.request, serde_json::to_vec(&request).unwrap()).unwrap();
    cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&fixture.corpus)
        .assert()
        .failure();
    assert!(!fixture.corpus.exists());

    let fixture = Fixture::new();
    fs::write(&fixture.corpus, b"already exists").unwrap();
    cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&fixture.corpus)
        .assert()
        .failure();
    assert_eq!(fs::read(&fixture.corpus).unwrap(), b"already exists");

    let exported = fixture.root.path().join("fresh-corpus.json");
    cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&exported)
        .assert()
        .success();
    let corpus: GspCorpus = read_json(&exported);
    let mut reduced = exact_result(&corpus);
    reduced.as_object_mut().unwrap().remove("uncertainty_map");
    fs::write(&fixture.result, serde_json::to_vec(&reduced).unwrap()).unwrap();
    cli()
        .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
        .arg(&exported)
        .arg("--result")
        .arg(&fixture.result)
        .arg("--output")
        .arg(&fixture.validated)
        .assert()
        .failure();
    assert!(!fixture.validated.exists());
    assert_eq!(event_count(&fixture.database), 7);

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "gsp", "link-result"])
        .assert()
        .failure();
}

struct Fixture {
    root: TempDir,
    database: std::path::PathBuf,
    request: std::path::PathBuf,
    corpus: std::path::PathBuf,
    result: std::path::PathBuf,
    validated: std::path::PathBuf,
    receipt: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let database = root.path().join("ledger.db");
        let request = root.path().join("request.json");
        let corpus = root.path().join("corpus.json");
        let result = root.path().join("result.json");
        let validated = root.path().join("validated.json");
        let receipt = root.path().join("receipt.json");
        let (events, refs) = stream();
        let mut store = SqliteEventStore::open(&database).unwrap();
        store
            .append_imported_events(&project_id(), 0, &events)
            .unwrap();
        let request_value = GspCorpusRequest {
            query: "What remains before release?".to_owned(),
            project_ref: project_id(),
            authority_refs: vec![refs.authority],
            decision_refs: vec![refs.decision],
            gate_refs: vec![refs.gate],
            checkpoint_refs: vec![refs.checkpoint],
            material_event_refs: vec![refs.work_event],
            observed_at: timestamp(10),
            budget: GspCorpusBudget {
                max_items: 5,
                max_text_bytes: 16_384,
            },
        };
        fs::write(&request, serde_json::to_vec(&request_value).unwrap()).unwrap();
        Self {
            root,
            database,
            request,
            corpus,
            result,
            validated,
            receipt,
        }
    }
}

struct Refs {
    authority: StableId,
    decision: StableId,
    gate: StableId,
    checkpoint: StableId,
    work_event: EventId,
}

fn stream() -> (Vec<project_ledger_domain::EventEnvelope>, Refs) {
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
    let before = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let checkpoint_event = EventId::new();
    let checkpoint = checkpoint(&before, checkpoint_event.clone());
    stream.push_with_id(
        checkpoint_event,
        Event::CheckpointCreated(CheckpointCreatedEvent {
            checkpoint: checkpoint.clone(),
        }),
    );
    (
        stream.events,
        Refs {
            authority: authority.document_id,
            decision: decision.decision_id,
            gate: gate.gate_id,
            checkpoint: checkpoint.checkpoint_id,
            work_event,
        },
    )
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
        idempotency_key: id(&format!("gsp:cli:fixture:{stream_version}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: LedgerCommand::CreateProject(CreateProjectPayload { project: project() }),
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
            correlation_ref: id("correlation:gsp-cli"),
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
        uri: ResourceRef::parse("file:///C:/private/spec.md").unwrap(),
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
            alternatives: vec!["Memory".to_owned()],
            status: DecisionStatus::Proposed,
            version: 1,
            authority_ref: None,
            evidence_refs: Vec::new(),
            supersedes_ref: None,
            created_at: timestamp(4),
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
            created_at: timestamp(5),
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
            summary: "Prepare exact GSP bridge".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            task_ref: Some(id("task:gsp")),
            files_or_artifacts: Vec::new(),
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(6),
            recorded_at: timestamp(6),
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
            created_at: timestamp(7),
            checkpoint_digest: zero(),
        },
        |value, digest| value.checkpoint_digest = digest,
    )
}

fn exact_result(corpus: &GspCorpus) -> Value {
    let request_id = corpus.evidence_bundle["request_id"].as_str().unwrap();
    let evidence_id = corpus.evidence_bundle["evidence_items"][0]["evidence_id"]
        .as_str()
        .unwrap();
    let mut candidate = fixture_payload("candidate_bundle__valid_min.yaml");
    let mut compilation = fixture_payload("compilation_bundle__valid_min.yaml");
    let mut report = fixture_payload("validation_report__valid_min.yaml");
    let mut uncertainty = fixture_payload("uncertainty_map__valid_min.yaml");
    let mut task = fixture_payload("task_projection_bundle__valid_min.yaml");
    for value in [
        &mut candidate,
        &mut compilation,
        &mut report,
        &mut uncertainty,
        &mut task,
    ] {
        replace_string(value, "request_id", request_id);
        replace_exact(value, "ev-001", evidence_id);
    }
    json!({
        "evidence_bundle": corpus.evidence_bundle,
        "candidate_bundle": candidate,
        "compilation_bundle": compilation,
        "validation_report": report,
        "uncertainty_map": uncertainty,
        "task_projection_bundle": task,
        "validation": {
            "status": "pass",
            "authority": "gsp-spec@gsp-exact-2026.04.12",
            "contracts_checked": ["evidence.bundle", "candidate.bundle", "compilation.bundle", "validation.report", "uncertainty.map", "task.projection.bundle"]
        }
    })
}

fn fixture_payload(name: &str) -> Value {
    let path = repository_root()
        .join("integrations/gsp/contracts/gsp-exact-2026.04.12/fixtures/contracts")
        .join(name);
    let value: Value = serde_yaml::from_slice(&fs::read(path).unwrap()).unwrap();
    value["payload"].clone()
}

fn replace_string(value: &mut Value, field: &str, replacement: &str) {
    match value {
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| replace_string(value, field, replacement)),
        Value::Object(fields) => fields.iter_mut().for_each(|(name, value)| {
            if name == field {
                *value = json!(replacement);
            } else {
                replace_string(value, field, replacement);
            }
        }),
        _ => {}
    }
}

fn replace_exact(value: &mut Value, expected: &str, replacement: &str) {
    match value {
        Value::String(text) if text == expected => replacement.clone_into(text),
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| replace_exact(value, expected, replacement)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|value| replace_exact(value, expected, replacement)),
        _ => {}
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "GSP CLI".to_owned(),
        summary: "Exact bridge fixture".to_owned(),
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

fn projection(database: &Path) -> ProjectStateProjection {
    let store = SqliteEventStore::open(database).unwrap();
    ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap()
}

fn event_count(database: &Path) -> u64 {
    SqliteEventStore::open(database)
        .unwrap()
        .event_count(&project_id())
        .unwrap()
}

fn cli() -> Command {
    let mut command = std::env::var_os("PROJECT_LEDGER_PACKAGE_BINARY").map_or_else(
        || Command::cargo_bin("project-ledger").unwrap(),
        Command::new,
    );
    command.current_dir(std::env::temp_dir());
    command
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198cf00-0000-7000-8000-000000000031").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T12:{minute:02}:00-04:00")).unwrap()
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
    set(&mut value, sha256_digest(&json).unwrap());
    value
}
