use assert_cmd::prelude::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

struct Project {
    temp: TempDir,
    database: PathBuf,
    id: String,
}
impl Project {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["init", "--title", "History scenario", "--data-root"])
            .arg(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .unwrap()
                .to_owned()
        };
        Self {
            database: PathBuf::from(field("database")),
            id: field("project_ref"),
            temp,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args(args)
            .arg("--database")
            .arg(&self.database)
            .args(["--project-ref", &self.id, "--json"])
            .output()
            .unwrap()
    }
    fn write(&self, payload: &Value, version: u64, key: &str) -> Output {
        let path = self.temp.path().join("update.json");
        fs::write(&path, serde_json::to_vec(payload).unwrap()).unwrap();
        self.run(&[
            "record",
            "episode",
            "--file",
            path.to_str().unwrap(),
            "--actor-id",
            "agent:history",
            "--expected-version",
            &version.to_string(),
            "--idempotency-key",
            key,
            "--observed-at",
            "2026-09-07T12:00:00Z",
        ])
    }
    fn human(&self, operation: &[&str], payload: &Value, version: &mut u64, key: &str) {
        let path = self.temp.path().join("human.json");
        fs::write(&path, serde_json::to_vec(payload).unwrap()).unwrap();
        let version_text = version.to_string();
        let mut args = operation.to_vec();
        args.extend([
            "--file",
            path.to_str().unwrap(),
            "--actor-id",
            "human:owner",
            "--as-human",
            "--expected-version",
            &version_text,
            "--idempotency-key",
            key,
            "--observed-at",
            "2026-09-07T12:00:00Z",
        ]);
        *version = value(self.run(&args), 0)["new_state_version"]
            .as_u64()
            .unwrap();
    }
}
fn value(output: Output, expected: i32) -> Value {
    let Output {
        status,
        stdout,
        stderr,
    } = output;
    assert_eq!(
        status.code(),
        Some(expected),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    let value: Value = serde_json::from_slice(&stdout).unwrap();
    if value["schema_version"] == "0.3.0" && value.get("kind").is_some() {
        let schema = project_ledger_schemas::schema_catalog_v0_3()
            .unwrap()
            .into_iter()
            .find(|s| s.relative_path == "history-response.schema.json")
            .unwrap();
        jsonschema::validator_for(&schema.schema)
            .unwrap()
            .validate(&value)
            .unwrap();
    }
    value
}
fn initial() -> Value {
    json!({"episode_ref":"episode:history", "objective":"Choose a historical read strategy", "summary":"Current snapshots cannot answer earlier cuts", "entries":[
        {"entry_id":"alternative:snapshot", "corrects_ref":null, "kind":"ALTERNATIVE", "description":"Read current snapshot", "rationale":"Reuse existing cache"},
        {"entry_id":"alternative:replay", "corrects_ref":null, "kind":"ALTERNATIVE", "description":"Replay a complete prefix", "rationale":"Preserve historical scope"},
        {"entry_id":"attempt:one", "corrects_ref":null, "kind":"ATTEMPT", "alternative_ref":"alternative:snapshot", "description":"Current snapshot contains later changes", "outcome":"FAILED", "evidence_refs":[]}
    ]})
}
#[test]
fn episode_record_is_atomic_replayable_and_historical() {
    let p = Project::new();
    let payload = initial();
    let written = value(p.write(&payload, 1, "history:first"), 0);
    assert_eq!(written["new_state_version"], 3);
    assert_eq!(written["emitted_event_refs"].as_array().unwrap().len(), 2);
    assert_eq!(
        value(p.write(&payload, 1, "history:first"), 0)["status"],
        "DUPLICATE_REPLAY"
    );
    let old = value(p.run(&["history", "at", "--version", "1"]), 0);
    assert_eq!(old["episodes"], json!([]));
    let current = value(p.run(&["history", "at", "--version", "3"]), 0);
    assert_eq!(
        current["episodes"][0]["entries"].as_array().unwrap().len(),
        3
    );
    let detail = value(p.run(&["history", "get", "attempt:one"]), 0);
    assert_eq!(detail["record"]["entry"]["outcome"], "FAILED");
    let delta = value(p.run(&["history", "diff", "--from", "1", "--to", "3"]), 0);
    assert_eq!(delta["events"].as_array().unwrap().len(), 2);
    let invalid = p.run(&["history", "at", "--version", "2"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("1 and 3"));
    assert_eq!(value(p.run(&["status"]), 0)["state_version"], 3);
    validate_stored_schemas(&p);
    let checkpoint = value(
        p.run(&[
            "checkpoint",
            "create",
            "--reason",
            "Before next material update",
            "--expected-version",
            "3",
            "--idempotency-key",
            "checkpoint",
            "--actor-id",
            "agent:history",
            "--observed-at",
            "2026-09-07T12:00:00Z",
        ]),
        0,
    );
    let checkpoint_version = checkpoint["new_state_version"].as_u64().unwrap();
    let update = value(
        p.write(
            &json!({"episode_ref":"episode:history","summary":"New material context","entries":[]}),
            checkpoint_version,
            "after-checkpoint",
        ),
        0,
    );
    let diff = value(
        p.run(&[
            "history",
            "diff",
            "--from",
            &checkpoint_version.to_string(),
            "--to",
            &update["new_state_version"].to_string(),
        ]),
        0,
    );
    assert!(
        diff["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event_type"] == "CHECKPOINT_INVALIDATED")
    );
}

fn validate_stored_schemas(project: &Project) {
    let db = rusqlite::Connection::open(&project.database).unwrap();
    let catalog = project_ledger_schemas::schema_catalog_v0_3().unwrap();
    for (table, column, schema_name) in [
        ("commands", "command_json", "command-envelope"),
        ("events", "event_json", "event-envelope"),
    ] {
        let schema = catalog
            .iter()
            .find(|s| s.relative_path == format!("{schema_name}.schema.json"))
            .unwrap();
        let validator = jsonschema::validator_for(&schema.schema).unwrap();
        let mut statement = db
            .prepare(&format!("SELECT {column} FROM {table}"))
            .unwrap();
        for row in statement
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .unwrap()
        {
            let mut value: Value = serde_json::from_slice(&row.unwrap()).unwrap();
            if table == "commands" && value["command_type"] != "RECORD_EPISODE_UPDATE" {
                continue;
            }
            validator.validate(&value).unwrap();
            value["unexpected"] = json!(true);
            assert!(!validator.is_valid(&value));
        }
    }
}

#[test]
fn corrections_never_rewrite_the_past_and_invalid_relations_are_atomic() {
    let p = Project::new();
    value(p.write(&initial(), 1, "first"), 0);
    let old = value(
        p.run(&["history", "get", "attempt:one", "--version", "3"]),
        0,
    );
    let corrected = json!({"episode_ref":"episode:history","summary":"The attempt was inconclusive, not a product failure","entries":[
        {"entry_id":"attempt:corrected","kind":"ATTEMPT","corrects_ref":"attempt:one","alternative_ref":"alternative:snapshot",
         "description":"The experiment read the wrong file","outcome":"INCONCLUSIVE","evidence_refs":[]}]});
    value(p.write(&corrected, 3, "correction"), 0);
    assert_eq!(
        value(
            p.run(&["history", "get", "attempt:one", "--version", "3"]),
            0
        ),
        old
    );
    assert_eq!(
        value(p.run(&["history", "get", "attempt:one"]), 0)["record"]["superseded_by"],
        "attempt:corrected"
    );
    assert!(
        !p.run(&["history", "get", "attempt:corrected", "--version", "3"])
            .status
            .success()
    );
    let mut bad = corrected.clone();
    bad["entries"][0]["entry_id"] = json!("attempt:third");
    assert!(!p.write(&bad, 5, "double-correction").status.success());
    bad["entries"][0]["corrects_ref"] = Value::Null;
    bad["entries"][0]["alternative_ref"] = json!("alternative:missing");
    assert!(!p.write(&bad, 5, "bad-reference").status.success());
    bad["entries"][0]["alternative_ref"] = json!("alternative:snapshot");
    bad["entries"][0]["unrecognized"] = json!(true);
    assert!(!p.write(&bad, 5, "unknown-field").status.success());
    assert_eq!(value(p.run(&["status"]), 0)["state_version"], 5);
}

#[test]
fn concurrent_writers_conflicts_and_permissions_use_the_canonical_pipeline() {
    let p = Project::new();
    let file = p.temp.path().join("concurrent.json");
    fs::write(&file, serde_json::to_vec(&initial()).unwrap()).unwrap();
    let spawn = |key| {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["record", "episode", "--file"])
            .arg(&file)
            .arg("--database")
            .arg(&p.database)
            .args([
                "--project-ref",
                &p.id,
                "--actor-id",
                "agent:history",
                "--expected-version",
                "1",
                "--idempotency-key",
                key,
                "--observed-at",
                "2026-09-07T12:00:00Z",
                "--json",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let a = spawn("concurrent:a");
    let b = spawn("concurrent:b");
    let mut codes = [
        a.wait_with_output().unwrap().status.code().unwrap(),
        b.wait_with_output().unwrap().status.code().unwrap(),
    ];
    codes.sort_unstable();
    assert_eq!(codes, [0, 5]);
    assert_eq!(value(p.run(&["status"]), 0)["state_version"], 3);
    let denied = p.run(&[
        "decision",
        "accept",
        "decision:absent",
        "--authority",
        "authority:spec",
        "--actor-id",
        "agent:history",
        "--expected-version",
        "3",
        "--idempotency-key",
        "denied",
        "--observed-at",
        "2026-09-07T12:00:00Z",
    ]);
    assert_eq!(value(denied, 9)["status"], "POLICY_DENIED");
    let replay = value(p.write(&initial(), 3, "reused-entry-id"), 3);
    assert_eq!(replay["status"], "VALIDATION_FAILED");
    assert_eq!(value(p.run(&["verify"]), 0)["integrity"], "ok");
}

fn fixture(name: &str, changes: Value) -> Value {
    let mut record = project_ledger_schemas::fixture_catalog()
        .unwrap()
        .into_iter()
        .find(|f| f.schema_id.ends_with(&format!(":{name}:0.2.0")))
        .unwrap()
        .positive;
    let Value::Object(changes) = changes else {
        panic!("fixture overrides must be an object");
    };
    for (key, value) in changes {
        record[&key] = value;
    }
    if name == "decision" {
        record.as_object_mut().unwrap().remove("digest");
        record["digest"] = json!(project_ledger_canonical_json::sha256_digest(&record).unwrap());
    }
    record
}

#[test]
#[allow(clippy::too_many_lines)] // One chronological CLI scenario keeps the causal assertions together.
fn trace_reconstructs_supersession_attempts_and_evidence_invalidation() {
    let p = Project::new();
    let mut v = 1;
    p.human(
        &["authority", "add"],
        &json!({"document":fixture("authority-document",json!({}))}),
        &mut v,
        "authority",
    );
    let mut evidence = fixture("evidence-record", json!({"project_ref":p.id}));
    evidence.as_object_mut().unwrap().remove("payload_digest");
    evidence["payload_digest"] =
        json!(project_ledger_canonical_json::sha256_digest(&evidence).unwrap());
    p.human(&["evidence", "import"], &evidence, &mut v, "evidence");
    let decision = fixture(
        "decision",
        json!({"status":"PROPOSED","accepted_at":null,"authority_ref":null,"evidence_refs":["evidence:01"]}),
    );
    p.human(
        &["decision", "propose"],
        &json!({"decision":decision}),
        &mut v,
        "first-decision",
    );
    p.human(
        &["decision", "accept"],
        &json!({"decision_ref":"decision:rust","authority_ref":"authority:spec"}),
        &mut v,
        "accept-first",
    );
    let before_episode = v.to_string();
    let legacy = value(
        p.run(&[
            "decision",
            "trace",
            "decision:rust",
            "--version",
            &before_episode,
        ]),
        0,
    );
    assert_eq!(
        legacy["missing"][0]["reason"],
        "NO_STRUCTURED_EPISODE_RECORDED"
    );
    assert_eq!(legacy["missing"][0]["legacy_alternatives"], json!(["Go"]));
    let mut episode = initial();
    episode["entries"][2]["evidence_refs"] = json!(["evidence:01"]);
    episode["entries"].as_array_mut().unwrap().push(json!({"entry_id":"link:first", "kind":"DECISION_LINK", "decision_ref":"decision:rust", "alternative_ref":"alternative:snapshot", "rationale":"The team initially chose the simpler read path"}));
    v = value(p.write(&episode, v, "episode"), 0)["new_state_version"]
        .as_u64()
        .unwrap();
    let old_cut = v.to_string();
    let old_trace = value(
        p.run(&["decision", "trace", "decision:rust", "--version", &old_cut]),
        0,
    );
    assert_eq!(old_trace["evidence"][0]["record"]["status"], "OBSERVED");
    let replacement = fixture(
        "decision",
        json!({"decision_id":"decision:replay","version":2,"status":"PROPOSED","accepted_at":null,"authority_ref":null,"supersedes_ref":"decision:rust"}),
    );
    p.human(
        &["decision", "propose"],
        &json!({"decision":replacement}),
        &mut v,
        "replacement",
    );
    p.human(
        &["decision", "accept"],
        &json!({"decision_ref":"decision:replay","authority_ref":"authority:spec"}),
        &mut v,
        "accept-replacement",
    );
    let accepted = value(p.run(&["decision", "show", "decision:replay"]), 0);
    p.human(
        &["decision", "supersede"],
        &json!({"decision_ref":"decision:rust","replacement":accepted}),
        &mut v,
        "supersede",
    );
    let update = json!({"episode_ref":"episode:history","summary":"Replaced the read strategy after the failed attempt","entries":[
        {"entry_id":"attempt:success","kind":"ATTEMPT","alternative_ref":"alternative:replay","description":"A complete prefix reconstructed the earlier state","outcome":"SUCCEEDED","evidence_refs":[]},
        {"entry_id":"link:replacement","kind":"DECISION_LINK","decision_ref":"decision:replay","alternative_ref":"alternative:replay","rationale":"The prefix preserves the historical boundary"}]});
    v = value(p.write(&update, v, "replacement-context"), 0)["new_state_version"]
        .as_u64()
        .unwrap();
    p.human(
        &["evidence", "invalidate"],
        &json!({"evidence_ref":"evidence:01","reason":"The original input was superseded"}),
        &mut v,
        "invalidate",
    );
    let trace = value(p.run(&["decision", "trace", "decision:replay"]), 0);
    assert_eq!(trace["decisions"].as_array().unwrap().len(), 2);
    assert!(
        trace["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["decision_id"] == "decision:rust" && d["status"] == "SUPERSEDED")
    );
    assert_eq!(trace["evidence"][0]["record"]["status"], "INVALIDATED");
    assert_eq!(trace["missing"], json!([]));
    assert!(
        trace["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["record"]["entry"]["outcome"] == "FAILED")
    );
    assert_eq!(
        value(
            p.run(&["decision", "trace", "decision:rust", "--version", &old_cut]),
            0
        ),
        old_trace
    );
}

#[test]
fn trace_cursor_stays_on_its_original_cut_as_new_updates_arrive() {
    let p = Project::new();
    let mut v = 1;
    let decision = fixture(
        "decision",
        json!({"status":"PROPOSED","accepted_at":null,"authority_ref":null}),
    );
    p.human(
        &["decision", "propose"],
        &json!({"decision":decision}),
        &mut v,
        "decision",
    );
    let mut update = initial();
    update["entries"].as_array_mut().unwrap().push(json!({"entry_id":"link:page","kind":"DECISION_LINK","decision_ref":"decision:rust","alternative_ref":"alternative:replay","rationale":"Explicit proposal context"}));
    for i in 0..55 {
        update["entries"].as_array_mut().unwrap().push(json!({"entry_id":format!("attempt:page-{i:03}"),"kind":"ATTEMPT","alternative_ref":"alternative:replay","description":format!("Recorded trial {i}"),"outcome":"INCONCLUSIVE","evidence_refs":[]}));
    }
    v = value(p.write(&update, v, "pages"), 0)["new_state_version"]
        .as_u64()
        .unwrap();
    let first = value(p.run(&["decision", "trace", "decision:rust"]), 0);
    assert_eq!(first["entries"].as_array().unwrap().len(), 50);
    let cursor = first["next_cursor"].as_str().unwrap();
    value(p.write(&json!({"episode_ref":"episode:history","summary":"A later material update","entries":[]}), v, "later"), 0);
    let second = value(
        p.run(&["decision", "trace", "decision:rust", "--cursor", cursor]),
        0,
    );
    assert_eq!(second["state_version"], first["state_version"]);
    assert_eq!(second["head_digest"], first["head_digest"]);
    assert!(second["next_cursor"].is_null());
    assert_eq!(
        50 + second["entries"].as_array().unwrap().len(),
        usize::try_from(first["total_entries"].as_u64().unwrap()).unwrap()
    );
    assert!(
        !p.run(&["decision", "trace", "decision:other", "--cursor", cursor])
            .status
            .success()
    );
    let mut invalid: Value = serde_json::from_str(cursor).unwrap();
    invalid["head_digest"] = json!("sha256:wrong");
    assert!(
        !p.run(&[
            "decision",
            "trace",
            "decision:rust",
            "--cursor",
            &invalid.to_string()
        ])
        .status
        .success()
    );
}

#[test]
fn legacy_and_mixed_portable_bundles_preserve_every_event_and_digest() {
    let p = Project::new();
    for (index, expected_schema) in [(0, "0.2.0"), (1, "0.3.0")] {
        if index == 1 {
            value(p.write(&initial(), 1, "portable"), 0);
        }
        let bundle = p.temp.path().join(format!("portable-{index}"));
        let manifest = value(p.run(&["export", "--output", bundle.to_str().unwrap()]), 0);
        assert_eq!(manifest["schema_version"], expected_schema);
        let imported = p.temp.path().join(format!("import-{index}.db"));
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["import", "--file"])
            .arg(&bundle)
            .arg("--database")
            .arg(&imported)
            .arg("--json")
            .output()
            .unwrap();
        value(output, 0);
        let original = value(p.run(&["history", "at"]), 0);
        let restored = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["history", "at", "--database"])
            .arg(&imported)
            .args(["--project-ref", &p.id, "--json"])
            .output()
            .unwrap();
        assert_eq!(value(restored, 0), original);
        let db = rusqlite::Connection::open(&p.database).unwrap();
        let copy = rusqlite::Connection::open(&imported).unwrap();
        let read = |db: &rusqlite::Connection| {
            db.prepare("SELECT event_json FROM events ORDER BY stream_version")
                .unwrap()
                .query_map([], |row| row.get::<_, Vec<u8>>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(read(&db), read(&copy));
    }
}
