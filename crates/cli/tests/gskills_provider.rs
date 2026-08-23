use std::{collections::BTreeMap, fs, path::Path};

use assert_cmd::Command;
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, Command as LedgerCommand, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Phase, PhaseKind, PhaseStartedEvent, PhaseStatus, Project,
    ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, ProjectStatusDerivedEvent, ResourceRef,
    Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent, RoadmapExecutionModel, SchemaVersion,
    StableId, StorageProfile, TrustLevel, WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{
    CommandDraft, EventDraft, EventImportStore, build_event, prepare_command,
};
use project_ledger_interop_gskills::{GSkillsProvider, ProviderContext};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcRequest, JsonRpcResponse, decode_frame, encode_frame,
};
use serde::Serialize;
use serde_json::{Value, json};
use tempfile::TempDir;

const PROFILE: &str = "gskills-context-provider/0.3.0";
const FIELDS: &[&str] = &[
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

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

#[test]
fn cli_verifies_exact_gskills_contracts_and_rejects_profile_aliases() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "gskills", "verify-contracts"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "gskills contracts MATCH schemas=6 fixtures=12",
        ));

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args([
            "serve",
            "provider",
            "--stdio",
            "--profile",
            "0.3",
            "--database",
            "missing.db",
            "--project-ref",
            project_id().as_str(),
            "--gskills-project-ref",
            "project:mempilot",
            "--phase-catalog",
            "missing.json",
            "--observed-at",
            timestamp(6).as_str(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported provider profile"));
}

#[test]
fn cli_serves_one_real_sqlite_backed_framed_projection() {
    let fixture = CliFixture::new();
    let limits = FrameLimits::default();
    let request = projection_request(&fixture.provider);
    let expected = fixture.provider.project(&request).unwrap();
    let rpc = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!("cli:e2e"),
        method: "ledger.project_continuity".to_owned(),
        params: request,
    };
    let frame = encode_frame(&rpc, limits).unwrap();

    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args([
            "serve",
            "provider",
            "--stdio",
            "--profile",
            PROFILE,
            "--database",
            fixture.database.to_str().unwrap(),
            "--project-ref",
            project_id().as_str(),
            "--gskills-project-ref",
            "project:mempilot",
            "--phase-catalog",
            fixture.phase_catalog.to_str().unwrap(),
            "--observed-at",
            timestamp(6).as_str(),
        ])
        .write_stdin(frame)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: JsonRpcResponse = decode_frame(&output.stdout, limits).unwrap();
    assert!(response.error.is_none());
    assert_eq!(
        response.result,
        Some(json!({
            "envelope": expected.envelope,
            "freshness": expected.freshness,
            "projection": expected.payload,
        }))
    );
}

struct CliFixture {
    _root: TempDir,
    database: std::path::PathBuf,
    phase_catalog: std::path::PathBuf,
    provider: GSkillsProvider,
}

impl CliFixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let database = root.path().join("ledger.db");
        let phase_catalog = root.path().join("phases.json");
        let events = stream();
        let phase = phase();
        let mut store = SqliteEventStore::open(&database).unwrap();
        store
            .append_imported_events(&project_id(), 0, &events)
            .unwrap();
        fs::write(
            &phase_catalog,
            serde_json::to_vec(&vec![phase.clone()]).unwrap(),
        )
        .unwrap();
        let projection = ProjectStateProjection::rebuild(&events).unwrap();
        let provider = GSkillsProvider::new(
            projection,
            ProviderContext {
                gskills_project_ref: "project:mempilot".to_owned(),
                external_project_ref: ResourceRef::parse(&format!(
                    "project-ledger://projects/{}",
                    project_id().uuid()
                ))
                .unwrap(),
                phase_catalog: BTreeMap::from([(phase.phase_id.clone(), phase)]),
                process_instance_id: id("provider-process:cli"),
                observed_at: timestamp(6),
            },
        )
        .unwrap();
        Self {
            _root: root,
            database,
            phase_catalog,
            provider,
        }
    }
}

fn projection_request(provider: &GSkillsProvider) -> Value {
    let state = provider.observable_state();
    let mut request = json!({
        "schema_version": "0.3.0",
        "context_projection_request_id": "projection-request:cli-e2e",
        "request_id": "req:cli-e2e",
        "traceparent": "00-00000000000000000000000000000003-0000000000000003-01",
        "provider_id": "project-ledger",
        "projection_kind": "PROJECT_CONTINUITY",
        "stage": "TASK_CONTEXT",
        "purpose": "Verify real CLI composition",
        "identity": {
            "project_ref": "project:mempilot",
            "session_ref": "session:cli-e2e",
            "task_ref": "task:provider-cli"
        },
        "structural_scope": {"path_refs": [], "symbol_refs": [], "impact_ref": null},
        "requested_fields": FIELDS,
        "budget": {"max_items": 24, "max_tokens": 350, "max_output_bytes": 65536, "timeout_ms": 1000},
        "privacy": {
            "allowed_fields": ["ids", "titles", "statuses", "digests", "bounded_summaries"],
            "source_bodies": false,
            "raw_events": false
        },
        "consistency": {
            "expected_project_state_version": state.state_version,
            "expected_material_state_version": state.material_state_version,
            "expected_project_state_digest": state.state_digest.to_string(),
            "require_freshness": true,
            "allow_partial": true
        },
        "write_capability": "NONE",
        "requested_at": timestamp(6),
        "digest": zero().to_string()
    });
    refresh_digest(&mut request);
    request
}

fn stream() -> Vec<project_ledger_domain::EventEnvelope> {
    let payloads = [
        Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
        Event::AuthorityDocumentRegistered(AuthorityDocumentRegisteredEvent {
            document: authority(),
        }),
        Event::RoadmapDefined(RoadmapDefinedEvent { roadmap: roadmap() }),
        Event::PhaseStarted(PhaseStartedEvent {
            phase_ref: id("phase:implementation"),
        }),
        Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
            previous_status: ProjectLifecycleStatus::Initializing,
            new_status: ProjectLifecycleStatus::Active,
            reason_codes: vec!["PHASE_STARTED".to_owned()],
        }),
        Event::WorkProgressRecorded(WorkProgressRecordedEvent {
            work_record: work(),
        }),
    ];
    let mut events = Vec::new();
    for payload in payloads {
        let version = events.len() as u64 + 1;
        events.push(event(
            version,
            events
                .last()
                .map(|value: &project_ledger_domain::EventEnvelope| value.event_digest),
            payload,
        ));
    }
    events
}

fn event(
    version: u64,
    previous: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: version - 1,
        idempotency_key: id(&format!("cli:gskills:{version}")),
        actor: actor(),
        command: LedgerCommand::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(version),
    })
    .unwrap();
    build_event(
        &command,
        version,
        previous,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:cli-gskills"),
            occurred_at: timestamp(version),
            recorded_at: timestamp(version),
        },
    )
    .unwrap()
}

fn phase() -> Phase {
    Phase {
        schema_version: SchemaVersion::V0_2_0,
        phase_id: id("phase:implementation"),
        title: "Implementation".to_owned(),
        ordinal: 1,
        kind: PhaseKind::Implementation,
        status: PhaseStatus::Active,
        required_gate_refs: Vec::new(),
        optional_gate_refs: Vec::new(),
        started_at: Some(timestamp(4)),
        completed_at: None,
        reopened_at: None,
    }
}

fn work() -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:cli-e2e"),
            summary: "CLI projection fixture ready".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            task_ref: Some(id("task:provider-cli")),
            files_or_artifacts: Vec::new(),
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(6),
            recorded_at: timestamp(6),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/cli-e2e").unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:spec"),
        uri: ResourceRef::parse("workspace://PROJECT_LEDGER_SPEC_v0.2.0.md").unwrap(),
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

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "MemPilot".to_owned(),
        summary: "CLI fixture".to_owned(),
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

fn refresh_digest(value: &mut Value) {
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("digest");
    value["digest"] = json!(sha256_digest(&unsigned).unwrap().to_string());
}

fn with_digest<T: Clone + Serialize>(mut value: T, set: impl Fn(&mut T, Sha256Digest)) -> T {
    set(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    json.as_object_mut().unwrap().remove("digest");
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

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T10:{minute:02}:00-04:00")).unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}
