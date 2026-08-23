use std::{fs, path::Path};

use assert_cmd::Command;
use project_ledger_application::{
    CommandDraft, CommandProcessor, CreateProjectHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command as LedgerCommand, CommandId, CommandResultStatus,
    CreateProjectPayload, LedgerTimestamp, Project, ProjectId, ProjectLifecycleStatus,
    SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcRequest, JsonRpcResponse, decode_frame, encode_frame,
};
use serde_json::{Value, json};
use tempfile::TempDir;

const PROFILE: &str = "gskills-context-provider/0.3.0";
const PROJECT_ID: &str = "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const COMMAND_UUID: &str = "0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

#[test]
fn cli_sink_executes_validate_submit_replay_status_and_reconcile_on_real_sqlite() {
    let fixture = SinkFixture::new("TRUSTED_FOR_RECORDING");
    let command = fixture.command();
    let input = framed_requests(&command);

    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args([
            "serve",
            "sink",
            "--stdio",
            "--profile",
            PROFILE,
            "--database",
            fixture.database.to_str().unwrap(),
            "--project-ref",
            PROJECT_ID,
            "--gskills-project-ref",
            "project:mempilot",
            "--policy",
            fixture.policy.to_str().unwrap(),
            "--observed-at",
            "2026-08-23T10:02:00-04:00",
        ])
        .write_stdin(input)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = decode_frames(&output.stdout);
    assert_eq!(responses.len(), 5);
    assert_eq!(responses[0].result.as_ref().unwrap()["status"], "VALID");
    assert_eq!(responses[1].result.as_ref().unwrap()["status"], "ACCEPTED");
    assert_eq!(
        responses[2].result.as_ref().unwrap()["status"],
        "DUPLICATE_REPLAY"
    );
    assert_eq!(responses[3].result.as_ref().unwrap()["status"], "FOUND");
    assert_eq!(responses[4].result.as_ref().unwrap()["status"], "FOUND");
    assert_eq!(
        responses[3].result.as_ref().unwrap()["result"]["native_result_digest"],
        responses[1].result.as_ref().unwrap()["native_result_digest"]
    );
    assert_eq!(
        responses[4].result.as_ref().unwrap()["result"]["native_result_digest"],
        responses[1].result.as_ref().unwrap()["native_result_digest"]
    );

    let store = SqliteEventStore::open(&fixture.database).unwrap();
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
}

#[test]
fn cli_sink_requires_explicit_recording_policy_and_never_derives_write_trust() {
    let fixture = SinkFixture::new("TRUSTED_FOR_PROJECTION");

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args([
            "serve",
            "sink",
            "--stdio",
            "--profile",
            PROFILE,
            "--database",
            fixture.database.to_str().unwrap(),
            "--project-ref",
            PROJECT_ID,
            "--gskills-project-ref",
            "project:mempilot",
            "--policy",
            fixture.policy.to_str().unwrap(),
            "--observed-at",
            "2026-08-23T10:02:00-04:00",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("TRUSTED_FOR_RECORDING"));

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args([
            "serve",
            "sink",
            "--stdio",
            "--profile",
            PROFILE,
            "--database",
            fixture.database.to_str().unwrap(),
            "--project-ref",
            PROJECT_ID,
            "--gskills-project-ref",
            "project:mempilot",
            "--observed-at",
            "2026-08-23T10:02:00-04:00",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--policy"));

    let store = SqliteEventStore::open(&fixture.database).unwrap();
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

struct SinkFixture {
    _root: TempDir,
    database: std::path::PathBuf,
    policy: std::path::PathBuf,
}

impl SinkFixture {
    fn new(trust: &str) -> Self {
        let root = TempDir::new().unwrap();
        let database = root.path().join("ledger.db");
        let policy = root.path().join("record-sink-policy.json");
        let mut store = SqliteEventStore::open(&database).unwrap();
        let created = CommandProcessor::new(
            &mut store,
            &FixedClock::new(timestamp("2026-08-23T10:00:00-04:00")),
        )
        .execute(
            prepare_command(CommandDraft {
                schema_version: SchemaVersion::V0_2_0,
                command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df50")
                    .unwrap(),
                project_ref: project_id(),
                expected_state_version: 0,
                idempotency_key: id("cli:sink:create"),
                actor: Actor {
                    actor_id: id("human:local-user"),
                    actor_kind: ActorKind::Human,
                    trust_level: TrustLevel::LocalUser,
                    display_name: None,
                },
                command: LedgerCommand::CreateProject(CreateProjectPayload { project: project() }),
                submitted_at: timestamp("2026-08-23T10:00:00-04:00"),
            })
            .unwrap(),
            &CreateProjectHandler,
        )
        .unwrap();
        assert_eq!(created.status, CommandResultStatus::Accepted);
        fs::write(
            &policy,
            serde_json::to_vec_pretty(&policy_value(trust)).unwrap(),
        )
        .unwrap();
        Self {
            _root: root,
            database,
            policy,
        }
    }

    fn command(&self) -> Value {
        let store = SqliteEventStore::open(&self.database).unwrap();
        let stream = store.load_stream(&project_id()).unwrap();
        let projection = ProjectStateProjection::rebuild(&stream).unwrap();
        external_command(
            projection.state().state_version,
            projection.state().material_state_version,
            projection.state().state_digest,
        )
    }
}

fn framed_requests(command: &Value) -> Vec<u8> {
    let limits = FrameLimits::default();
    [
        ("ledger.command_validate", json!({"command": command})),
        ("ledger.command_submit", json!({"command": command})),
        ("ledger.command_submit", json!({"command": command})),
        ("ledger.command_status", json!({"command": command})),
        ("ledger.command_reconcile", json!({"command": command})),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (method, params))| {
        encode_frame(
            &JsonRpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: json!(index + 1),
                method: method.to_owned(),
                params,
            },
            limits,
        )
        .unwrap()
    })
    .collect()
}

fn decode_frames(bytes: &[u8]) -> Vec<JsonRpcResponse> {
    let limits = FrameLimits::default();
    let mut responses = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let relative = bytes[offset..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let body_offset = offset + relative + 4;
        let header = std::str::from_utf8(&bytes[offset..offset + relative]).unwrap();
        let length = header
            .strip_prefix("Content-Length: ")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let end = body_offset + length;
        responses.push(decode_frame(&bytes[offset..end], limits).unwrap());
        offset = end;
    }
    responses
}

fn policy_value(trust: &str) -> Value {
    let mut value = json!({
        "schema_version":"0.2.0",
        "policy_id":"policy:gskills-record-sink",
        "sink_id":"project-ledger",
        "profile":PROFILE,
        "gskills_project_ref":"project:mempilot",
        "external_project_ref":"project-ledger://projects/mempilot",
        "native_project_ref":PROJECT_ID,
        "external_actor":{"type":"agent_harness","id":"codex-session-44","on_behalf_of":"human:local-user"},
        "native_actor":{"actor_id":"integration:gskills","actor_kind":"INTEGRATION","trust_level":trust,"display_name":"G-Skills RecordSink"},
        "actor_trust_domain":"agent_harness:on_behalf_of_local_user",
        "allowed_command_kinds":["RECORD_WORK_PROGRESS","RECORD_VERIFICATION_RESULT","REQUEST_GATE_EVALUATION","REQUEST_GATE_REVALIDATION","CREATE_CONTINUITY_CHECKPOINT"],
        "approval_requirement":"USER_OR_POLICY",
        "provider_manifest_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "provider_executable_digest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "compatibility_lock_digest":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        "identity_map_digest":"sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "created_at":"2026-08-23T10:00:00-04:00",
        "digest":zero()
    });
    refresh_digest(&mut value);
    value
}

fn external_command(state: u64, material: u64, state_digest: Sha256Digest) -> Value {
    let policy = policy_value("TRUSTED_FOR_RECORDING");
    let mut work = json!({
        "schema_version":"0.2.0",
        "work_record_id":"work:cli-sink-e2e",
        "summary":"CLI exact RecordSink E2E",
        "phase_ref":null,
        "task_ref":"task:s10",
        "files_or_artifacts":[],
        "commit_refs":[],
        "claim_refs":[],
        "evidence_refs":[],
        "actor_ref":"integration:gskills",
        "influence":null,
        "occurred_at":"2026-08-23T10:01:00-04:00",
        "recorded_at":"2026-08-23T10:01:00-04:00",
        "provenance_ref":"gskills://provenance/cli-sink-e2e",
        "digest":zero()
    });
    refresh_digest(&mut work);
    let mut value = json!({
        "schema_version":"0.3.0",
        "external_record_command_id":format!("external-command:{COMMAND_UUID}"),
        "command_kind":"RECORD_WORK_PROGRESS",
        "sink_id":"project-ledger",
        "profile":PROFILE,
        "project_ref":"project:mempilot",
        "external_project_ref":"project-ledger://projects/mempilot",
        "actor":{"type":"agent_harness","id":"codex-session-44","on_behalf_of":"human:local-user"},
        "session_ref":"session:cli-sink",
        "native_command":{"command_type":"RECORD_WORK_PROGRESS","command_schema_ref":"urn:project-ledger:command:record-work-progress:0.2.0","command_schema_ref_status":"EXACT","expected_state_version":state},
        "expected_state":{"project_state_version":state,"material_state_version":material,"project_state_digest":state_digest},
        "payload":{"work_record":work},
        "idempotency_key":"gskills:cli:sink:e2e",
        "idempotency_scope":{"sink_id":"project-ledger","external_project_ref":"project-ledger://projects/mempilot","native_command_type":"RECORD_WORK_PROGRESS","actor_trust_domain":"agent_harness:on_behalf_of_local_user"},
        "preparation":{
            "prepared_by":"gskills",
            "prepared_at":"2026-08-23T10:01:00-04:00",
            "prepared_command_digest":zero(),
            "preview_digest":zero(),
            "provider_manifest_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "provider_executable_digest":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "compatibility_lock_digest":"sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "policy_digest":policy["digest"],
            "identity_map_digest":"sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "expected_state_digest":state_digest,
            "approval_requirement":"USER_OR_POLICY",
            "approval_expires_at":"2026-08-23T10:06:00-04:00"
        },
        "privacy":{"raw_output_included":false,"evidence_refs_only":true},
        "provenance_ref":"gskills://provenance/external-command-cli-sink",
        "digest":zero()
    });
    let prepared = prepared_digest(&value);
    value["preparation"]["prepared_command_digest"] = json!(prepared.to_string());
    value["preparation"]["preview_digest"] = json!(prepared.to_string());
    refresh_digest(&mut value);
    value
}

fn prepared_digest(value: &Value) -> Sha256Digest {
    let mut core = value.clone();
    let object = core.as_object_mut().unwrap();
    object.remove("digest");
    let preparation = object["preparation"].as_object_mut().unwrap();
    for field in [
        "prepared_command_digest",
        "preview_digest",
        "provider_manifest_digest",
        "provider_executable_digest",
        "compatibility_lock_digest",
        "policy_digest",
        "identity_map_digest",
        "expected_state_digest",
    ] {
        preparation.remove(field);
    }
    sha256_digest(&core).unwrap()
}

fn refresh_digest(value: &mut Value) {
    let object = value.as_object_mut().unwrap();
    object.remove("digest");
    let digest = sha256_digest(&*object).unwrap();
    object.insert("digest".to_owned(), json!(digest.to_string()));
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "CLI RecordSink".to_owned(),
        summary: "S10 CLI E2E".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp("2026-08-23T10:00:00-04:00"),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse(PROJECT_ID).unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn timestamp(value: &str) -> LedgerTimestamp {
    LedgerTimestamp::parse(value).unwrap()
}

fn zero() -> String {
    Sha256Digest::from([0; 32]).to_string()
}
