use std::{fs, path::Path};

use assert_cmd::Command;
use project_ledger_application::{CommandProcessor, CoreCommandHandler, FixedClock};
use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::*;
use project_ledger_event_store::{CommandDraft, EventStore, prepare_command};
use project_ledger_storage_sqlite::SqliteEventStore;
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcRequest, JsonRpcResponse, decode_frame, encode_frame,
};
use serde_json::{Value, json};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn contract(path: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("integrations/gskills-confirmation/v0.1")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn confirmation_contracts_reject_undeclared_fields_partial_proofs_and_invalid_native_ids() {
    for row in contract("fixtures/index.json").as_array().unwrap() {
        let schema = contract(&format!("{}.schema.json", row["schema"].as_str().unwrap()));
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();
        let value = contract(row["path"].as_str().unwrap());
        assert_eq!(
            validator.is_valid(&value),
            row["valid"].as_bool().unwrap(),
            "{}",
            row["path"]
        );
        if row["valid"] == true {
            let mut unsigned = value.clone();
            unsigned.as_object_mut().unwrap().remove("digest");
            assert_eq!(json!(sha256_digest(&unsigned).unwrap()), value["digest"]);
        }
    }
}

#[test]
fn confirmation_profile_exposes_only_read_methods_and_binds_manifest_to_policy() {
    let fixture = tempfile::tempdir().unwrap();
    let db = fixture.path().join("ledger.db");
    drop(SqliteEventStore::open(&db).unwrap());
    let policy =
        root().join("integrations/gskills-confirmation/v0.1/fixtures/positive/policy.json");
    for (method, want) in [
        ("ledger.manifest", None),
        ("ledger.command_submit", Some(-32601)),
        ("ledger.command_status", Some(-32601)),
    ] {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: json!("test"),
            method: method.into(),
            params: json!({}),
        };
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args([
                "serve",
                "confirmation",
                "--stdio",
                "--profile",
                "gskills-record-confirmation/0.1.0",
                "--database",
                db.to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--project-ref",
                "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
                "--gskills-project-ref",
                "project:mempilot",
                "--observed-at",
                "2026-09-08T04:00:00Z",
            ])
            .write_stdin(encode_frame(&request, FrameLimits::default()).unwrap())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reply: JsonRpcResponse = decode_frame(&output.stdout, FrameLimits::default()).unwrap();
        if let Some(code) = want {
            assert_eq!(reply.error.unwrap().code, code);
        } else {
            assert_eq!(
                reply.result.unwrap(),
                contract("fixtures/positive/manifest.json")
            );
        }
    }
}

struct Fixture {
    dir: tempfile::TempDir,
    request: Value,
    result: CommandResult,
}

fn sign(value: &mut Value) {
    value.as_object_mut().unwrap().remove("digest");
    value["digest"] = json!(sha256_digest(value).unwrap());
}

impl Fixture {
    fn new(reject: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqliteEventStore::open(&dir.path().join("ledger.db")).unwrap();
        let timestamp = LedgerTimestamp::parse("2026-09-08T04:00:00Z").unwrap();
        let project: Project = serde_json::from_value(json!({
            "project_id":"project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d","title":"Confirmation fixture","summary":"Test owned",
            "lifecycle_status":"INITIALIZING","created_at":timestamp,"timezone":"UTC","authority_policy":"SPEC_FIRST",
            "storage_profile":"USER_DATA","repository_binding_refs":[],"paused_reason":null,"archived_at":null
        })).unwrap();
        let command = prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: CommandId::new(),
            project_ref: project.project_id.clone(),
            expected_state_version: 0,
            idempotency_key: StableId::parse("fixture:create").unwrap(),
            actor: Actor {
                actor_id: StableId::parse("human:fixture").unwrap(),
                actor_kind: ActorKind::Human,
                trust_level: TrustLevel::LocalUser,
                display_name: None,
            },
            command: project_ledger_domain::Command::CreateProject(CreateProjectPayload {
                project,
            }),
            submitted_at: timestamp.clone(),
        })
        .unwrap();
        CommandProcessor::new(&mut store, &FixedClock::new(timestamp))
            .execute(command, &CoreCommandHandler)
            .unwrap();
        let mut request = contract("fixtures/positive/request.json");
        let result = checkpoint(
            &mut store,
            u64::from(!reject),
            request["command_ref"].as_str().unwrap(),
            "gskills:confirmation-fixture",
        );
        assert_eq!(
            result.status,
            if reject {
                CommandResultStatus::VersionConflict
            } else {
                CommandResultStatus::Accepted
            }
        );
        request["expected_native_result_digest"] = json!(result.digest);
        request["expected_result_version"] = json!(result.new_state_version);
        sign(&mut request);
        let current = store
            .current_version(&ProjectId::parse(request["project_ref"].as_str().unwrap()).unwrap())
            .unwrap();
        checkpoint(
            &mut store,
            current,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df65",
            "fixture:later",
        );
        fs::write(
            dir.path().join("policy.json"),
            serde_json::to_vec(&contract("fixtures/positive/policy.json")).unwrap(),
        )
        .unwrap();
        Self {
            dir,
            request,
            result,
        }
    }
    fn rpc(&self, request: &Value) -> JsonRpcResponse {
        let frame = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: json!("confirmation"),
            method: "ledger.command_confirmation".into(),
            params: json!({"request":request}),
        };
        let output = self.process(encode_frame(&frame, FrameLimits::default()).unwrap());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        decode_frame(&output.stdout, FrameLimits::default()).unwrap()
    }
    fn process(&self, input: Vec<u8>) -> std::process::Output {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args([
                "serve",
                "confirmation",
                "--stdio",
                "--profile",
                "gskills-record-confirmation/0.1.0",
                "--database",
                self.dir.path().join("ledger.db").to_str().unwrap(),
                "--policy",
                self.dir.path().join("policy.json").to_str().unwrap(),
                "--project-ref",
                "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
                "--gskills-project-ref",
                "project:mempilot",
                "--observed-at",
                "2026-09-08T04:00:00Z",
            ])
            .write_stdin(input)
            .output()
            .unwrap()
    }
    fn replace_result(&mut self, mut result: Value) {
        sign(&mut result);
        rusqlite::Connection::open(self.dir.path().join("ledger.db")).unwrap().execute(
            "UPDATE command_results SET result_json=?1,result_digest=?2,status=?3 WHERE command_ref=?4",
            rusqlite::params![serde_json::to_vec(&result).unwrap(),result["digest"].as_str().unwrap(),result["status"].as_str().unwrap(),self.request["command_ref"].as_str().unwrap()]
        ).unwrap();
        self.request["expected_native_result_digest"] = result["digest"].clone();
        self.request["expected_result_version"] = result["new_state_version"].clone();
        sign(&mut self.request);
    }
    fn events(&self) -> Value {
        json!(
            SqliteEventStore::open_read_only(&self.dir.path().join("ledger.db"))
                .unwrap()
                .load_stream(
                    &ProjectId::parse(self.request["project_ref"].as_str().unwrap()).unwrap()
                )
                .unwrap()
        )
    }
}

fn checkpoint(
    store: &mut SqliteEventStore,
    version: u64,
    command_ref: &str,
    key: &str,
) -> CommandResult {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(command_ref).unwrap(),
        project_ref: ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        expected_state_version: version,
        idempotency_key: StableId::parse(key).unwrap(),
        actor: Actor {
            actor_id: StableId::parse("agent:gskills").unwrap(),
            actor_kind: ActorKind::Agent,
            trust_level: TrustLevel::TrustedForRecording,
            display_name: None,
        },
        command: project_ledger_domain::Command::CreateContinuityCheckpoint(
            CreateContinuityCheckpointPayload {
                reason: "Test checkpoint".into(),
            },
        ),
        submitted_at: LedgerTimestamp::parse("2026-09-08T04:00:00Z").unwrap(),
    })
    .unwrap();
    CommandProcessor::new(store, &FixedClock::new(command.submitted_at.clone()))
        .execute(command, &CoreCommandHandler)
        .unwrap()
}

#[test]
fn confirms_durable_acceptance_and_rejection_after_advance_without_writes() {
    for reject in [false, true] {
        let fixture = Fixture::new(reject);
        let before = fixture.events();
        let proof = fixture.rpc(&fixture.request).result.unwrap();
        assert_eq!(proof["status"], "VERIFIED");
        assert_eq!(
            proof["native_result"]["digest"],
            json!(fixture.result.digest)
        );
        assert_eq!(
            proof["result_cut"]["state_version"],
            fixture.result.new_state_version
        );
        assert_eq!(
            proof["current_cut"]["state_version"],
            fixture.result.new_state_version + 1
        );
        assert_eq!(
            proof["native_result"]["emitted_event_refs"],
            json!(fixture.result.emitted_event_refs)
        );
        assert_eq!(
            proof["effect_verification"]["event_count"],
            u64::from(!reject)
        );
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .build(&contract("response.schema.json"))
            .unwrap();
        assert!(validator.is_valid(&proof), "{proof}");
        let mut unsigned = proof.clone();
        unsigned.as_object_mut().unwrap().remove("digest");
        assert_eq!(proof["digest"], json!(sha256_digest(&unsigned).unwrap()));
        assert_eq!(fixture.events(), before);
        let durable = SqliteEventStore::open_read_only(&fixture.dir.path().join("ledger.db"))
            .unwrap()
            .command_result(&fixture.result.command_ref)
            .unwrap()
            .unwrap();
        assert_eq!(durable, fixture.result);
    }
}

#[test]
fn confirmation_rejects_identity_scope_result_and_cut_contradictions() {
    let fixture = Fixture::new(false);
    for (pointer, value, code) in [
        (
            "/project_ref",
            json!("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e"),
            "IDENTITY_MISMATCH",
        ),
        (
            "/idempotency_scope/actor_ref",
            json!("agent:other"),
            "SCOPE_MISMATCH",
        ),
        (
            "/idempotency_scope/key",
            json!("key:other"),
            "SCOPE_MISMATCH",
        ),
        (
            "/expected_native_result_digest",
            json!(format!("sha256:{}", "0".repeat(64))),
            "RESULT_MISMATCH",
        ),
        ("/expected_result_version", json!(1), "RESULT_MISMATCH"),
        (
            "/command_ref",
            json!("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df99"),
            "COMMAND_NOT_FOUND",
        ),
    ] {
        let mut request = fixture.request.clone();
        *request.pointer_mut(pointer).unwrap() = value;
        sign(&mut request);
        let proof = fixture.rpc(&request).result.unwrap();
        assert_eq!(proof["error"]["code"], code, "{pointer}: {proof}");
        assert!(proof.get("native_result").is_none());
    }
    let mut request = fixture.request.clone();
    request["digest"] = json!(format!("sha256:{}", "0".repeat(64)));
    assert_eq!(fixture.rpc(&request).error.unwrap().code, -32602);
    let mut request = fixture.request.clone();
    request["requested_at"] = json!("2026-09-08T03:59:00Z");
    sign(&mut request);
    assert_eq!(fixture.rpc(&request).error.unwrap().code, -32602);
}

#[test]
fn confirmation_rejects_corrupt_indices_and_forged_event_interval() {
    for sql in [
        "UPDATE commands SET actor_id='actor:forged' WHERE command_id LIKE '%df64'",
        "UPDATE idempotency_keys SET command_digest='sha256:0000000000000000000000000000000000000000000000000000000000000000' WHERE command_ref LIKE '%df64'",
        "UPDATE event_streams SET head_event_digest='sha256:0000000000000000000000000000000000000000000000000000000000000000'",
    ] {
        let fixture = Fixture::new(false);
        rusqlite::Connection::open(fixture.dir.path().join("ledger.db"))
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let proof = fixture.rpc(&fixture.request).result.unwrap();
        assert_eq!(proof["error"]["code"], "INTEGRITY_VIOLATION");
    }
    let mut fixture = Fixture::new(false);
    let mut result = json!(fixture.result);
    result["emitted_event_refs"] = json!(["event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df99"]);
    sign(&mut result);
    rusqlite::Connection::open(fixture.dir.path().join("ledger.db"))
        .unwrap()
        .execute(
            "UPDATE command_results SET result_json=?1,result_digest=?2 WHERE command_ref=?3",
            rusqlite::params![
                serde_json::to_vec(&result).unwrap(),
                result["digest"].as_str().unwrap(),
                fixture.request["command_ref"].as_str().unwrap()
            ],
        )
        .unwrap();
    fixture.request["expected_native_result_digest"] = result["digest"].clone();
    sign(&mut fixture.request);
    assert_eq!(
        fixture.rpc(&fixture.request).result.unwrap()["error"]["code"],
        "EVENT_MISMATCH"
    );
}

#[test]
fn confirmation_rejects_partial_nonexistent_cuts_and_oversized_event_proof() {
    let mut fixture = Fixture::new(false);
    let mut store = SqliteEventStore::open_existing(&fixture.dir.path().join("ledger.db")).unwrap();
    let command=prepare_command(CommandDraft{schema_version:SchemaVersion::V0_3_0,command_id:CommandId::new(),
        project_ref:ProjectId::parse(fixture.request["project_ref"].as_str().unwrap()).unwrap(),expected_state_version:3,
        idempotency_key:StableId::parse("test:episode").unwrap(),actor:Actor{actor_id:StableId::parse("agent:gskills").unwrap(),actor_kind:ActorKind::Agent,trust_level:TrustLevel::TrustedForRecording,display_name:None},
        command:project_ledger_domain::Command::RecordEpisodeUpdate(serde_json::from_value(json!({"episode_ref":"episode:cut","objective":"Atomic cut","summary":"Test owned","entries":[]})).unwrap()),submitted_at:LedgerTimestamp::parse("2026-09-08T04:00:00Z").unwrap()}).unwrap();
    let result = CommandProcessor::new(&mut store, &FixedClock::new(command.submitted_at.clone()))
        .execute(command, &CoreCommandHandler)
        .unwrap();
    // Episode, progress and stale-checkpoint invalidation are one three-event command.
    assert_eq!(result.new_state_version, 6);
    assert_eq!(
        fixture.rpc(&fixture.request).result.unwrap()["status"],
        "VERIFIED"
    );
    for version in [4, 5, 7] {
        let mut forged = json!(fixture.result);
        forged["new_state_version"] = json!(version);
        forged["previous_state_version"] = json!(version - 1);
        fixture.replace_result(forged);
        assert_eq!(
            fixture.rpc(&fixture.request).result.unwrap()["error"]["code"],
            "INVALID_CUT"
        );
    }
    let mut oversized = json!(fixture.result);
    oversized["emitted_event_refs"] =
        json!(vec![fixture.result.emitted_event_refs[0].clone(); 257]);
    fixture.replace_result(oversized);
    let proof = fixture.rpc(&fixture.request).result.unwrap();
    assert_eq!(proof["status"], "UNAVAILABLE");
    assert_eq!(proof["error"]["code"], "READ_LIMIT_EXCEEDED");
    assert!(proof.get("native_result").is_none());
}

#[test]
fn replay_observation_does_not_replace_the_persisted_result_proof() {
    let fixture = Fixture::new(false);
    let mut store = SqliteEventStore::open_existing(&fixture.dir.path().join("ledger.db")).unwrap();
    let command = store
        .recorded_command(&fixture.result.command_ref)
        .unwrap()
        .unwrap();
    let replay = CommandProcessor::new(
        &mut store,
        &FixedClock::new(LedgerTimestamp::parse("2026-09-08T04:00:01Z").unwrap()),
    )
    .execute(command, &CoreCommandHandler)
    .unwrap();
    assert_eq!(replay.status, CommandResultStatus::DuplicateReplay);
    assert_ne!(replay.digest, fixture.result.digest);
    let mut request = fixture.request.clone();
    request["expected_native_result_digest"] = json!(replay.digest);
    sign(&mut request);
    assert_eq!(
        fixture.rpc(&request).result.unwrap()["error"]["code"],
        "RESULT_MISMATCH"
    );
    assert_eq!(
        fixture.rpc(&fixture.request).result.unwrap()["status"],
        "VERIFIED"
    );
    assert_eq!(
        store
            .command_result(&fixture.result.command_ref)
            .unwrap()
            .unwrap(),
        fixture.result
    );
}

#[test]
fn framing_and_policy_limits_fail_before_proof_and_missing_store_is_not_created() {
    let fixture = Fixture::new(false);
    for input in [
        b"Content-Length: 32769\r\n\r\n".to_vec(),
        vec![b'X'; 1025],
        b"Content-Length: 10\r\n\r\n{}".to_vec(),
    ] {
        let output = fixture.process(input);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    for (key, value) in [
        ("read_authorized", json!(false)),
        (
            "native_project_ref",
            json!("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e"),
        ),
        ("digest", json!(format!("sha256:{}", "0".repeat(64)))),
    ] {
        let mut policy = contract("fixtures/positive/policy.json");
        policy[key] = value;
        if key != "digest" {
            sign(&mut policy);
        }
        fs::write(
            fixture.dir.path().join("policy.json"),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        assert!(!fixture.process(vec![]).status.success());
    }
    let empty = tempfile::tempdir().unwrap();
    fs::write(
        empty.path().join("policy.json"),
        serde_json::to_vec(&contract("fixtures/positive/policy.json")).unwrap(),
    )
    .unwrap();
    let absent = Fixture {
        dir: empty,
        request: fixture.request.clone(),
        result: fixture.result.clone(),
    };
    let proof = absent.rpc(&absent.request).result.unwrap();
    assert_eq!(proof["error"]["code"], "STORAGE_UNAVAILABLE");
    assert!(!absent.dir.path().join("ledger.db").exists());
}
