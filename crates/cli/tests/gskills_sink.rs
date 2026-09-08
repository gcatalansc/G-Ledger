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
fn cli_sink_stale_submit_is_durable_after_restart_without_domain_events() {
    let fixture = SinkFixture::new("TRUSTED_FOR_RECORDING");
    let command = fixture.command();
    let original = command.clone();
    assert_eq!(
        fixture
            .rpc("ledger.command_validate", &command)
            .result
            .unwrap()["status"],
        "VALID"
    );

    fixture.advance_checkpoint();
    let before = fixture.event_bytes();
    let tables_before = fixture.table_contents();
    let validated = fixture.rpc("ledger.command_validate", &command);
    assert!(validated.error.is_none(), "{validated:?}");
    assert_eq!(validated.result.unwrap()["status"], "VALID");
    assert_eq!(fixture.table_contents(), tables_before);
    let response = fixture.rpc("ledger.command_submit", &command);
    assert!(response.error.is_none(), "{response:?}");
    let result = response.result.unwrap();
    assert_eq!(result["status"], "VERSION_CONFLICT");
    assert_eq!(result["state_transition"]["previous_state_version"], 2);
    assert_eq!(result["state_transition"]["new_state_version"], 2);
    assert_eq!(result["state_transition"]["emitted_event_refs"], json!([]));
    assert!(result["resulting_checkpoint_ref"].is_null());

    // Each call starts a new native process. Lookup remains valid after approval expires.
    for method in ["ledger.command_status", "ledger.command_reconcile"] {
        let found = fixture.rpc_at(method, &command, "2026-08-23T10:10:00-04:00");
        assert!(found.error.is_none(), "{found:?}");
        let found = found.result.unwrap();
        assert_eq!(found["status"], "FOUND");
        assert_eq!(found["result"]["status"], "VERSION_CONFLICT");
        assert_eq!(
            found["result"]["native_result_digest"],
            result["native_result_digest"]
        );
    }
    assert_eq!(fixture.event_bytes(), before);
    assert_eq!(command, original);
    let store = SqliteEventStore::open(&fixture.database).unwrap();
    store.verify_event_chain(&project_id()).unwrap();
    let durable = store
        .command_result(&CommandId::parse(&format!("command:{COMMAND_UUID}")).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(durable.status, CommandResultStatus::VersionConflict);
    assert_eq!(json!(durable.digest), result["native_result_digest"]);
}

#[test]
fn cli_sink_recovers_accepted_result_after_another_checkpoint_and_restart() {
    assert_result_survives_newer_checkpoint(false);
}

#[test]
fn cli_sink_recovers_rejected_result_after_another_checkpoint_and_restart() {
    assert_result_survives_newer_checkpoint(true);
}

fn assert_result_survives_newer_checkpoint(reject: bool) {
    let fixture = SinkFixture::new("TRUSTED_FOR_RECORDING");
    let command = fixture.command();
    if reject {
        fixture.advance_checkpoint();
    }
    let submitted = fixture
        .rpc("ledger.command_submit", &command)
        .result
        .unwrap();
    assert_eq!(
        submitted["status"],
        if reject {
            "VERSION_CONFLICT"
        } else {
            "ACCEPTED"
        }
    );
    assert_eq!(submitted["state_transition"]["new_state_version"], 2);
    let before = fixture
        .rpc("ledger.command_status", &command)
        .result
        .unwrap();
    assert_eq!(before["status"], "FOUND");
    assert_eq!(
        before["result"]["native_result_digest"],
        submitted["native_result_digest"]
    );

    fixture.advance_checkpoint();
    let events = fixture.event_bytes();
    for method in ["ledger.command_status", "ledger.command_reconcile"] {
        let response = fixture.rpc_at(method, &command, "2026-08-23T10:10:00-04:00");
        assert!(response.error.is_none(), "{response:?}");
        assert_eq!(response.result.unwrap(), before);
    }
    assert_eq!(fixture.event_bytes(), events);
    let store = SqliteEventStore::open(&fixture.database).unwrap();
    assert_eq!(store.current_version(&project_id()).unwrap(), 3);
    store.verify_event_chain(&project_id()).unwrap();
    let result = store
        .command_result(&CommandId::parse(&format!("command:{COMMAND_UUID}")).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(json!(result.digest), submitted["native_result_digest"]);
}

#[test]
fn cli_sink_matching_version_still_requires_exact_state_identity() {
    let fixture = SinkFixture::new("TRUSTED_FOR_RECORDING");
    let valid = fixture.command();
    let before = fixture.event_bytes();
    for field in ["material_state_version", "project_state_digest"] {
        let mut command = valid.clone();
        if field == "material_state_version" {
            command["expected_state"][field] = json!(99);
        } else {
            command["expected_state"][field] = json!(zero());
            command["preparation"]["expected_state_digest"] = json!(zero());
        }
        resign_command(&mut command);
        let response = fixture.rpc("ledger.command_submit", &command);
        let error = response
            .error
            .expect("inconsistent current identity must be rejected");
        assert_eq!(error.code, -32602);
        assert!(
            error.data.as_ref().unwrap()["record_sink_error"]
                .as_str()
                .unwrap()
                .contains("state identity mismatch"),
            "{error:?}"
        );
        assert_eq!(
            fixture
                .rpc("ledger.command_status", &command)
                .result
                .unwrap()["status"],
            "NOT_FOUND"
        );
    }
    assert_eq!(fixture.event_bytes(), before);
}

#[test]
fn cli_sink_stale_version_does_not_bypass_identity_policy_or_approval() {
    let fixture = SinkFixture::new("TRUSTED_FOR_RECORDING");
    let valid = fixture.command();
    fixture.advance_checkpoint();
    let before = fixture.event_bytes();
    for (pointer, replacement) in [
        ("/project_ref", json!("project:other")),
        ("/actor/id", json!("other-agent")),
        (
            "/idempotency_scope/actor_trust_domain",
            json!("other-domain"),
        ),
        ("/preparation/policy_digest", json!(zero())),
        ("/preparation/provider_executable_digest", json!(zero())),
        (
            "/preparation/approval_expires_at",
            json!("2026-08-23T10:01:30-04:00"),
        ),
        ("/privacy/raw_output_included", json!(true)),
        ("/native_command/expected_state_version", json!(99)),
    ] {
        let mut command = valid.clone();
        *command.pointer_mut(pointer).unwrap() = replacement;
        resign_command(&mut command);
        for method in ["ledger.command_validate", "ledger.command_submit"] {
            let response = fixture.rpc(method, &command);
            assert!(
                response.result.is_none(),
                "{method} {pointer}: {response:?}"
            );
            assert_eq!(response.error.unwrap().code, -32602, "{method} {pointer}");
        }
    }
    assert_eq!(fixture.event_bytes(), before);
    assert_eq!(
        fixture.rpc("ledger.command_status", &valid).result.unwrap()["status"],
        "NOT_FOUND"
    );
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
    fn table_contents(&self) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
        let connection = rusqlite::Connection::open(&self.database).unwrap();
        let tables: Vec<String> = connection
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        tables
            .into_iter()
            .map(|name| {
                let mut statement = connection
                    .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
                    .unwrap();
                let columns = statement.column_count();
                let mut rows = statement
                    .query_map([], |row| {
                        (0..columns)
                            .map(|index| row.get(index))
                            .collect::<Result<Vec<rusqlite::types::Value>, _>>()
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                rows.sort_by_cached_key(|row| format!("{row:?}"));
                (name, rows)
            })
            .collect()
    }

    fn rpc(&self, method: &str, command: &Value) -> JsonRpcResponse {
        self.rpc_at(method, command, "2026-08-23T10:02:00-04:00")
    }

    fn rpc_at(&self, method: &str, command: &Value, observed_at: &str) -> JsonRpcResponse {
        let input = encode_frame(
            &JsonRpcRequest {
                jsonrpc: "2.0".to_owned(),
                id: json!(1),
                method: method.to_owned(),
                params: json!({"command": command}),
            },
            FrameLimits::default(),
        )
        .unwrap();
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
            ])
            .arg(&self.database)
            .args([
                "--project-ref",
                PROJECT_ID,
                "--gskills-project-ref",
                "project:mempilot",
                "--policy",
            ])
            .arg(&self.policy)
            .args(["--observed-at", observed_at])
            .write_stdin(input)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut responses = decode_frames(&output.stdout);
        assert_eq!(responses.len(), 1);
        responses.remove(0)
    }

    fn advance_checkpoint(&self) {
        let version = SqliteEventStore::open(&self.database)
            .unwrap()
            .current_version(&project_id())
            .unwrap();
        let version_text = version.to_string();
        let key = format!("competing-checkpoint:{version}");
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args([
                "checkpoint",
                "create",
                "--reason",
                "Accepted competing checkpoint",
                "--expected-version",
                &version_text,
                "--idempotency-key",
                &key,
                "--actor-id",
                "agent:competitor",
                "--observed-at",
                "2026-08-23T10:01:30-04:00",
                "--database",
            ])
            .arg(&self.database)
            .args(["--project-ref", PROJECT_ID, "--json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["status"], "ACCEPTED");
        assert_eq!(result["previous_state_version"], version);
        assert_eq!(result["new_state_version"], version + 1);
    }

    fn event_bytes(&self) -> Vec<u8> {
        let store = SqliteEventStore::open(&self.database).unwrap();
        serde_json::to_vec(&store.load_stream(&project_id()).unwrap()).unwrap()
    }

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

fn resign_command(value: &mut Value) {
    let prepared = prepared_digest(value);
    value["preparation"]["prepared_command_digest"] = json!(prepared);
    value["preparation"]["preview_digest"] = json!(prepared);
    refresh_digest(value);
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
