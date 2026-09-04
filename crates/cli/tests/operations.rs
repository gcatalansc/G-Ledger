use assert_cmd::prelude::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

struct Project {
    temporary: TempDir,
    database: PathBuf,
    id: String,
}
impl Project {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["init", "--title", "Disposable CLI", "--data-root"])
            .arg(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("{key}=")))
                .unwrap()
                .to_owned()
        };
        Self {
            database: PathBuf::from(field("database")),
            id: field("project_ref"),
            temporary: temp,
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
    #[allow(clippy::needless_pass_by_value)] // Fixture ownership keeps scenario call sites readable.
    fn write(&self, operation: &[&str], payload: Value, version: u64, key: &str) -> Output {
        let file = self.temporary.path().join("payload.json");
        fs::write(&file, serde_json::to_vec(&payload).unwrap()).unwrap();
        let mut args = operation.to_vec();
        args.extend([
            "--file",
            file.to_str().unwrap(),
            "--as-human",
            "--actor-id",
            "human:owner",
            "--observed-at",
            "2026-08-27T12:00:00Z",
            "--expected-version",
        ]);
        let version = version.to_string();
        args.extend([&version, "--idempotency-key", key]);
        self.run(&args)
    }
}
fn value(output: &Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn missing_database_is_exit_four_and_never_created() {
    let p = Project::new();
    let missing = p.temporary.path().join("absent/ledger.db");
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resume", "--database"])
        .arg(&missing)
        .args(["--project-ref", &p.id, "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(!missing.parent().unwrap().exists());
}

#[test]
fn note_replay_reconcile_and_typed_conflicts_use_real_processes() {
    let p = Project::new();
    let payload = json!({"kind":"LIMITATION", "text":"A real append", "scope_ref":null});
    let first = value(
        &p.write(&["record", "note"], payload.clone(), 1, "test:note"),
        0,
    );
    assert_eq!(first["new_state_version"], 2);
    assert_eq!(
        value(&p.write(&["record", "note"], payload, 1, "test:note"), 0)["status"],
        "DUPLICATE_REPLAY"
    );
    let reconciled = value(
        &p.run(&[
            "reconcile",
            "--command-type",
            "RECORD_NOTE",
            "--actor-id",
            "human:owner",
            "--as-human",
            "--idempotency-key",
            "test:note",
        ]),
        0,
    );
    assert_eq!(reconciled["result"], first);
    value(
        &p.write(
            &["record", "note"],
            json!({"kind":"LIMITATION","text":"Changed", "scope_ref":null}),
            1,
            "test:note",
        ),
        10,
    );
    value(
        &p.write(
            &["record", "note"],
            json!({"kind":"LIMITATION","text":"Stale", "scope_ref":null}),
            1,
            "test:stale",
        ),
        5,
    );
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 2);
    assert_eq!(
        value(
            &p.run(&[
                "reconcile",
                "--command-type",
                "RECORD_NOTE",
                "--actor-id",
                "human:owner",
                "--as-human",
                "--idempotency-key",
                "test:absent"
            ]),
            12
        )["error"]["code"],
        "OUTCOME_UNKNOWN"
    );
}

#[test]
fn locator_never_grants_authority_and_explicit_selection_wins() {
    let p = Project::new();
    let locator = p.temporary.path().join("locator.json");
    fs::write(
        &locator,
        serde_json::to_vec(&json!({"database": p.database, "project_ref":p.id})).unwrap(),
    )
    .unwrap();
    let denied = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["pause", "--reason", "not authorized", "--locator"])
        .arg(&locator)
        .args([
            "--actor-id",
            "agent:codex",
            "--expected-version",
            "1",
            "--idempotency-key",
            "denied:01",
            "--observed-at",
            "2026-08-27T12:00:00Z",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(value(&denied, 9)["status"], "POLICY_DENIED");
    fs::write(&locator, "invalid locator").unwrap();
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["status", "--locator"])
        .arg(&locator)
        .arg("--database")
        .arg(&p.database)
        .args(["--project-ref", &p.id, "--json"])
        .output()
        .unwrap();
    assert_eq!(value(&output, 0)["state_version"], 1);
}

#[test]
fn read_queries_reject_migration_without_changing_database() {
    let p = Project::new();
    let db = rusqlite::Connection::open(&p.database).unwrap();
    db.execute("DELETE FROM schema_migrations WHERE version = 3", [])
        .unwrap();
    db.execute("DROP TABLE execution_attempts", []).unwrap();
    drop(db);
    let before = fs::read(&p.database).unwrap();
    let output = p.run(&["status"]);
    value(&output, 11);
    assert_eq!(fs::read(&p.database).unwrap(), before);
    let db = rusqlite::Connection::open(&p.database).unwrap();
    assert_eq!(
        db.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        2
    );
}

fn record(name: &str, changes: Value) -> Value {
    let mut record = project_ledger_schemas::fixture_catalog()
        .unwrap()
        .into_iter()
        .find(|f| f.schema_id.ends_with(&format!(":{name}:0.2.0")))
        .unwrap()
        .positive;
    let Value::Object(changes) = changes else {
        panic!("expected object overrides")
    };
    for (key, value) in changes {
        record[&key] = value;
    }
    if !matches!(name, "artifact-reference" | "authority-document")
        && record.get("digest").is_some()
    {
        record.as_object_mut().unwrap().remove("digest");
        record["digest"] = json!(project_ledger_canonical_json::sha256_digest(&record).unwrap());
    }
    record
}

#[test]
#[allow(clippy::too_many_lines)]
fn normative_families_persist_and_replay_through_the_binary() {
    let p = Project::new();
    let digest = "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";
    let steps: Vec<(&[&str], Value)> = vec![
        (
            &["authority", "add"],
            json!({"document":record("authority-document",json!({}))}),
        ),
        (
            &["authority", "refresh"],
            json!({"document_ref":"authority:spec","observation":{"outcome":"MATCH","observed_digest":digest}}),
        ),
        (
            &["roadmap", "import"],
            json!({"roadmap":record("roadmap",json!({}))}),
        ),
        (
            &["repository", "bind"],
            json!({"binding":record("repository-binding",json!({}))}),
        ),
        (
            &["repository", "refresh"],
            json!({"binding_ref":"binding:ledger","snapshot":{"repository_ref":"repo:ledger","snapshot_ref":"snapshot:01","snapshot_digest":digest}}),
        ),
        (
            &["gate", "define"],
            json!({"gate":record("gate-definition",json!({}))}),
        ),
        (&["phase", "start"], json!({"phase_ref":"phase-0"})),
        (
            &["record", "progress"],
            json!({"work_record":record("work-record",json!({"actor_ref":"human:owner","recorded_at":"2026-08-27T12:00:00Z","claim_refs":[]}))}),
        ),
        (
            &["artifact", "add"],
            json!({"artifact":record("artifact-reference",json!({}))}),
        ),
        (
            &["claim", "record"],
            json!({"claim":record("claim-record",json!({}))}),
        ),
        (
            &["unknown", "record"],
            json!({"unknown":record("unknown-record",json!({}))}),
        ),
        (
            &["unknown", "resolve"],
            json!({"unknown_ref":"unknown:01","resolution_ref":"workspace://reports/answer"}),
        ),
        (
            &["decision", "propose"],
            json!({"decision":record("decision",json!({"status":"PROPOSED","accepted_at":null,"authority_ref":null}))}),
        ),
        (
            &["decision", "accept"],
            json!({"decision_ref":"decision:rust","authority_ref":"authority:spec"}),
        ),
        (
            &["blocker", "open"],
            json!({"blocker":record("blocker",json!({"scope_kind":"PHASE","scope_ref":"phase-0"}))}),
        ),
        (
            &["blocker", "resolve"],
            json!({"blocker_ref":"blocker:01","evidence_refs":[],"resolution_summary":"Discardable dependency resolved"}),
        ),
        (
            &["risk", "open"],
            json!({"risk":record("risk",json!({"status":"OPEN"}))}),
        ),
        (
            &["risk", "mitigate"],
            json!({"risk_ref":"risk:01","note":"mitigated","evidence_refs":[]}),
        ),
        (
            &["next", "set"],
            json!({"action":record("next-action",json!({}))}),
        ),
        (
            &["gate", "request-revalidation"],
            json!({"gate_ref":"gate:contracts","signal_ref":null}),
        ),
        (
            &["session", "start"],
            json!({"session":record("session",json!({"actor_ref":"human:owner","started_at":"2026-08-27T12:00:00Z"}))}),
        ),
        (
            &["session", "end"],
            json!({"session_ref":"session:01","create_checkpoint":true}),
        ),
        (
            &["repository", "revoke"],
            json!({"binding_ref":"binding:ledger","reason":"Discardable binding"}),
        ),
        (&["pause"], json!({"reason":"Explicit pause"})),
        (&["resume-project"], json!({"reason":"Explicit resume"})),
        (
            &["archive"],
            json!({"reason":"Discardable scenario complete"}),
        ),
    ];
    let mut version = 1;
    for (index, (operation, payload)) in steps.into_iter().enumerate() {
        let key = format!("family:{index}");
        let first = value(&p.write(operation, payload.clone(), version, &key), 0);
        let replay = value(&p.write(operation, payload, version, &key), 0);
        assert_eq!(replay["status"], "DUPLICATE_REPLAY", "{operation:?}");
        version = first["new_state_version"].as_u64().unwrap();
        assert_eq!(value(&p.run(&["status"]), 0)["state_version"], version);
    }
    for operation in [
        &["doctor"][..],
        &["audit"],
        &["verify"],
        &["authority", "list"],
        &["roadmap", "show"],
        &["phase", "list"],
        &["gate", "list"],
        &["gate", "show", "gate:contracts"],
        &["record", "notes"],
        &["artifact", "list"],
    ] {
        value(&p.run(operation), 0);
    }
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], version);
}

#[test]
fn risk_close_is_public_idempotent_and_removes_a_mitigated_risk_from_resume() {
    let p = Project::new();
    let opened = value(
        &p.write(
            &["risk", "open"],
            json!({"risk":record("risk",json!({"status":"OPEN"}))}),
            1,
            "risk:open",
        ),
        0,
    );
    assert_eq!(opened["new_state_version"], 2);
    let mitigated = value(
        &p.write(
            &["risk", "mitigate"],
            json!({"risk_ref":"risk:01","note":"control applied","evidence_refs":[]}),
            2,
            "risk:mitigate",
        ),
        0,
    );
    assert_eq!(mitigated["new_state_version"], 3);
    assert_eq!(
        value(&p.run(&["status"]), 0)["open_risk_refs"],
        json!(["risk:01"])
    );

    let close = [
        "risk",
        "close",
        "risk:01",
        "--note",
        "risk retired after verified mitigation",
        "--expected-version",
        "3",
        "--idempotency-key",
        "risk:close",
        "--actor-id",
        "human:owner",
        "--as-human",
        "--observed-at",
        "2026-09-03T23:55:00-04:00",
    ];
    let closed = value(&p.run(&close), 0);
    assert_eq!(closed["new_state_version"], 4);
    assert_eq!(value(&p.run(&close), 0)["status"], "DUPLICATE_REPLAY");
    assert_eq!(value(&p.run(&["status"]), 0)["open_risk_refs"], json!([]));

    let resume = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resume", "--database"])
        .arg(&p.database)
        .args(["--project-ref", &p.id, "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(value(&resume, 0)["open_risk_refs"], json!([]));
    assert_eq!(value(&p.run(&["audit"]), 0)["findings"], json!([]));
}

#[test]
fn ergonomic_note_and_progress_preserve_payload_and_actor() {
    let p = Project::new();
    let common = [
        "--actor-id",
        "agent:codex",
        "--observed-at",
        "2026-08-27T12:00:00Z",
        "--expected-version",
        "1",
        "--idempotency-key",
        "ergonomic:note",
    ];
    let mut args = vec![
        "record",
        "note",
        "--kind",
        "LIMITATION",
        "--text",
        "No simulated completion",
    ];
    args.extend(common);
    value(&p.run(&args), 0);
    assert_eq!(
        value(&p.run(&["record", "notes"]), 0)[0]["text"],
        "No simulated completion"
    );
    let output = p.run(&[
        "record",
        "progress",
        "--summary",
        "CLI operational",
        "--files",
        "workspace://src/main.rs",
        "--actor-id",
        "agent:codex",
        "--observed-at",
        "2026-08-27T12:00:00Z",
        "--expected-version",
        "2",
        "--idempotency-key",
        "ergonomic:work",
    ]);
    value(&output, 0);
    let resumed = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resume", "--database"])
        .arg(&p.database)
        .args(["--project-ref", &p.id, "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(
        value(&resumed, 0)["last_material_progress_summary"],
        "CLI operational"
    );
}

fn configure(p: &Project) -> (u64, Value) {
    let first = value(
        &p.write(
            &["authority", "add"],
            json!({"document":record("authority-document",json!({}))}),
            1,
            "setup:authority",
        ),
        0,
    );
    let second = value(
        &p.write(
            &["roadmap", "import"],
            json!({"roadmap":record("roadmap",json!({}))}),
            first["new_state_version"].as_u64().unwrap(),
            "setup:roadmap",
        ),
        0,
    );
    let gate = record("gate-definition", json!({}));
    let third = value(
        &p.write(
            &["gate", "define"],
            json!({"gate":gate}),
            second["new_state_version"].as_u64().unwrap(),
            "setup:gate",
        ),
        0,
    );
    let fourth = value(
        &p.write(
            &["phase", "start"],
            json!({"phase_ref":"phase-0"}),
            third["new_state_version"].as_u64().unwrap(),
            "setup:phase",
        ),
        0,
    );
    (fourth["new_state_version"].as_u64().unwrap(), gate)
}

#[test]
fn evidence_import_cannot_self_verify_and_gate_rejections_have_exit_six() {
    let p = Project::new();
    let (mut version, gate) = configure(&p);
    let mut evidence = record("evidence-record", json!({"project_ref":p.id}));
    evidence.as_object_mut().unwrap().remove("payload_digest");
    evidence["payload_digest"] =
        json!(project_ledger_canonical_json::sha256_digest(&evidence).unwrap());
    let result = value(
        &p.write(
            &["evidence", "import"],
            evidence,
            version,
            "evidence:import",
        ),
        0,
    );
    version = result["new_state_version"].as_u64().unwrap();
    assert_eq!(
        value(&p.run(&["evidence", "show", "evidence:01"]), 0)["status"],
        "OBSERVED"
    );
    let evaluated = value(
        &p.write(
            &["gate", "evaluate"],
            json!({"gate_ref":"gate:contracts"}),
            version,
            "gate:evaluate",
        ),
        6,
    );
    assert_eq!(evaluated["status"], "ACCEPTED");
    version = evaluated["new_state_version"].as_u64().unwrap();
    value(
        &p.write(
            &["phase", "request-close"],
            json!({"phase_ref":"phase-0"}),
            version,
            "phase:incomplete",
        ),
        6,
    );
    let waiver = record(
        "gate-waiver",
        json!({"gate_definition_digest":gate["digest"]}),
    );
    let waiver_result = value(
        &p.write(
            &["gate", "waive"],
            json!({"waiver":waiver}),
            version,
            "gate:waive",
        ),
        0,
    );
    let closed = value(
        &p.write(
            &["phase", "request-close"],
            json!({"phase_ref":"phase-0"}),
            waiver_result["new_state_version"].as_u64().unwrap(),
            "phase:close",
        ),
        0,
    );
    assert_eq!(
        value(&p.run(&["phase", "list"]), 0)[0]["status"],
        "COMPLETED"
    );
    value(
        &p.write(
            &["phase", "reopen"],
            json!({"phase_ref":"phase-0","reason":"new work","authority_ref":"authority:spec"}),
            closed["new_state_version"].as_u64().unwrap(),
            "phase:reopen",
        ),
        0,
    );
    assert_eq!(value(&p.run(&["phase", "list"]), 0)[0]["status"], "ACTIVE");
}

#[test]
fn everyday_flags_build_valid_records_without_payload_files() {
    let p = Project::new();
    let mut version = 1;
    for (i, operation) in [
        vec![
            "authority",
            "add",
            "--uri",
            "workspace://spec.md",
            "--digest",
            "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862",
            "--record-id",
            "authority:spec",
        ],
        vec![
            "artifact",
            "add",
            "--uri",
            "workspace://report.json",
            "--digest",
            "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862",
        ],
        vec![
            "claim",
            "record",
            "--statement",
            "A claim, not verified",
            "--classification",
            "ASSERTED",
        ],
        vec![
            "unknown",
            "record",
            "--subject",
            "authority:spec",
            "--statement",
            "A question",
            "--criticality",
            "MATERIAL",
        ],
        vec!["blocker", "open", "--title", "An unresolved prerequisite"],
        vec![
            "risk",
            "open",
            "--title",
            "A potential limitation",
            "--severity",
            "MEDIUM",
        ],
        vec![
            "next",
            "set",
            "--title",
            "Review prerequisite",
            "--classification",
            "PROPOSED",
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let key = format!("flags:{i}");
        let version_arg = version.to_string();
        let mut args = operation;
        args.extend([
            "--as-human",
            "--actor-id",
            "human:owner",
            "--observed-at",
            "2026-08-27T12:00:00Z",
            "--expected-version",
            &version_arg,
            "--idempotency-key",
            &key,
        ]);
        version = value(&p.run(&args), 0)["new_state_version"]
            .as_u64()
            .unwrap();
    }
    assert!(version > 7);
}

#[test]
#[allow(clippy::too_many_lines)] // Ordered process scenario with explicit independent expectations.
fn positional_actions_link_unlink_and_yaml_use_the_normative_cli() {
    let p = Project::new();
    let (mut version, gate) = configure(&p);
    let mut ordinal = 0;
    let mut send = |args: Vec<&str>, expected| {
        ordinal += 1;
        let key = format!("positional:{ordinal}");
        let v = version.to_string();
        let args: Vec<_> = args
            .into_iter()
            .chain([
                "--as-human",
                "--actor-id",
                "human:owner",
                "--observed-at",
                "2026-08-27T12:00:00Z",
                "--expected-version",
                &v,
                "--idempotency-key",
                &key,
            ])
            .collect();
        let result = value(&p.run(&args), expected);
        if let Some(v) = result["new_state_version"].as_u64() {
            version = v;
        }
        result
    };
    send(vec!["gate", "evaluate", "gate:contracts"], 6);
    send(
        vec![
            "blocker",
            "open",
            "--title",
            "Dependency",
            "--record-id",
            "blocker:flags",
        ],
        0,
    );
    send(
        vec![
            "blocker",
            "resolve",
            "blocker:flags",
            "--summary",
            "Resolved",
        ],
        0,
    );
    send(
        vec![
            "risk",
            "open",
            "--title",
            "Risk",
            "--record-id",
            "risk:flags",
        ],
        0,
    );
    send(
        vec!["risk", "mitigate", "risk:flags", "--note", "Bounded"],
        0,
    );
    send(
        vec![
            "unknown",
            "record",
            "--subject",
            "phase-0",
            "--statement",
            "Unknown",
            "--record-id",
            "unknown:flags",
        ],
        0,
    );
    send(
        vec![
            "unknown",
            "resolve",
            "unknown:flags",
            "--resolution",
            "urn:test:resolution",
        ],
        0,
    );
    send(
        vec![
            "claim",
            "record",
            "--statement",
            "Assertion",
            "--subject",
            "phase-0",
            "--record-id",
            "claim:flags",
        ],
        0,
    );
    send(
        vec!["claim", "status", "claim:flags", "--set", "REJECTED"],
        0,
    );
    let link = send(
        vec![
            "evidence",
            "link",
            "--system",
            "gsp",
            "--artifact-ref",
            "urn:test:external",
            "--digest",
            "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862",
            "--schema-ref",
            "urn:test:external:1",
        ],
        0,
    );
    let external = value(&p.run(&["evidence", "external-list"]), 0);
    assert_eq!(external.as_array().unwrap().len(), 1);
    let evidence_ref = external[0]["evidence_ref"].as_str().unwrap();
    send(
        vec![
            "evidence",
            "unlink",
            evidence_ref,
            "--reason",
            "Explicit unlink",
        ],
        0,
    );
    assert!(!link["emitted_event_refs"].as_array().unwrap().is_empty());
    send(
        vec![
            "gate",
            "waive",
            "gate:contracts",
            "--reason",
            "Reviewed",
            "--approved-by",
            "human:owner",
            "--authority",
            "authority:spec",
        ],
        0,
    );
    send(vec!["phase", "request-close", "phase-0"], 0);
    send(
        vec![
            "phase",
            "reopen",
            "phase-0",
            "--reason",
            "Follow-up",
            "--authority",
            "authority:spec",
        ],
        0,
    );
    let yaml = p.temporary.path().join("decision.yaml");
    fs::write(&yaml,serde_yaml::to_string(&record("decision",json!({"decision_id":"decision:yaml","status":"PROPOSED","accepted_at":null,"authority_ref":null}))).unwrap()).unwrap();
    send(
        vec!["decision", "propose", "--file", yaml.to_str().unwrap()],
        0,
    );
    send(
        vec![
            "decision",
            "accept",
            "decision:yaml",
            "--authority",
            "authority:spec",
        ],
        0,
    );
    let replacement = record(
        "decision",
        json!({"decision_id":"decision:replacement","status":"PROPOSED","accepted_at":null,"authority_ref":null,"supersedes_ref":"decision:yaml","version":2}),
    );
    fs::write(&yaml, serde_yaml::to_string(&replacement).unwrap()).unwrap();
    send(
        vec!["decision", "propose", "--file", yaml.to_str().unwrap()],
        0,
    );
    send(
        vec![
            "decision",
            "accept",
            "decision:replacement",
            "--authority",
            "authority:spec",
        ],
        0,
    );
    let accepted = value(&p.run(&["decision", "show", "decision:replacement"]), 0);
    fs::write(&yaml, serde_yaml::to_string(&accepted).unwrap()).unwrap();
    send(
        vec![
            "decision",
            "supersede",
            "decision:yaml",
            "--with",
            yaml.to_str().unwrap(),
        ],
        0,
    );
    assert_eq!(
        value(&p.run(&["decision", "show", "decision:yaml"]), 0)["status"],
        "SUPERSEDED"
    );
    assert_eq!(gate["gate_id"], "gate:contracts");
}

#[test]
fn repository_binding_and_authority_refresh_never_execute_locator_content() {
    let p = Project::new();
    let root = p.temporary.path().join("existing repo ñ");
    fs::create_dir(&root).unwrap();
    let spec = root.join("spec.md");
    fs::write(&spec, "Canonical disposable spec").unwrap();
    let digest =
        project_ledger_canonical_json::sha256_bytes(b"Canonical disposable spec").to_string();
    let mut version = 1;
    for (i, mut args) in [
        vec![
            "authority",
            "add",
            "--record-id",
            "authority:local",
            "--uri",
            spec.to_str().unwrap(),
            "--digest",
            &digest,
        ],
        vec!["authority", "refresh", "authority:local"],
        vec![
            "repository",
            "bind",
            "--record-id",
            "binding:local",
            "--root",
            root.to_str().unwrap(),
            "--remote",
            "https://example.invalid/declared.git",
        ],
        vec![
            "repository",
            "refresh",
            "binding:local",
            "--snapshot-ref",
            "snapshot:declared",
            "--snapshot-digest",
            &digest,
        ],
        vec![
            "repository",
            "revoke",
            "binding:local",
            "--reason",
            "Explicitly revoked",
        ],
        vec![
            "authority",
            "add",
            "--record-id",
            "authority:replacement",
            "--uri",
            spec.to_str().unwrap(),
            "--digest",
            &digest,
            "--supersedes",
            "authority:local",
        ],
        vec![
            "authority",
            "supersede",
            "authority:local",
            "--with",
            spec.to_str().unwrap(),
            "--digest",
            &digest,
            "--record-id",
            "authority:replacement",
        ],
    ]
    .into_iter()
    .enumerate()
    {
        let key = format!("local:{i}");
        let v = version.to_string();
        args.extend([
            "--as-human",
            "--actor-id",
            "human:owner",
            "--observed-at",
            "2026-08-27T12:00:00Z",
            "--expected-version",
            &v,
            "--idempotency-key",
            &key,
        ]);
        version = value(&p.run(&args), 0)["new_state_version"]
            .as_u64()
            .unwrap();
    }
    assert_eq!(fs::read(&spec).unwrap(), b"Canonical disposable spec");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn inline_signed_records_normalize_rfc3339_before_digesting() {
    let p = Project::new();
    let root = p.temporary.path().join("canonical timestamp repo");
    fs::create_dir(&root).unwrap();

    let output = p.run(&[
        "repository",
        "bind",
        "--record-id",
        "binding:canonical-timestamp",
        "--root",
        root.to_str().unwrap(),
        "--remote",
        "https://example.invalid/declared.git",
        "--as-human",
        "--actor-id",
        "human:owner",
        "--observed-at",
        "2026-09-02T19:38:54.0820837+00:00",
        "--expected-version",
        "1",
        "--idempotency-key",
        "test:canonical-timestamp",
    ]);

    let result = value(&output, 0);
    assert_eq!(result["status"], "ACCEPTED");
    assert_eq!(result["new_state_version"], 2);
}

#[test]
fn idempotency_identity_is_scoped_to_actor_and_trust_domain() {
    let p = Project::new();
    for (i, actor) in ["agent:first", "agent:second"].into_iter().enumerate() {
        value(
            &p.run(&[
                "record",
                "note",
                "--kind",
                "NOTE",
                "--text",
                "Independent actor scope",
                "--actor-id",
                actor,
                "--observed-at",
                "2026-08-27T12:00:00Z",
                "--expected-version",
                &(i + 1).to_string(),
                "--idempotency-key",
                "same:key",
            ]),
            0,
        );
    }
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 3);
}

#[test]
fn catalog_is_rebuildable_and_listing_never_writes() {
    let p = Project::new();
    let catalog = p.temporary.path().join("project-ledger/catalog.json");
    let run = |operation| {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["catalog", operation, "--data-root"])
            .arg(p.temporary.path())
            .arg("--json")
            .output()
            .unwrap()
    };
    let listing = value(&run("list"), 0);
    assert_eq!(listing.as_array().unwrap().len(), 1);
    assert_eq!(listing[0]["canonical_title"], "Disposable CLI");
    assert!(!catalog.exists());
    assert_eq!(value(&run("rebuild"), 0), listing);
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&catalog).unwrap()).unwrap(),
        listing
    );
}

#[test]
fn resume_can_use_explicit_locator_and_json_is_clean_for_legacy_commands() {
    let p = Project::new();
    let locator = p.temporary.path().join("project.json");
    fs::write(
        &locator,
        serde_json::to_vec(&json!({"database":p.database,"project_ref":p.id})).unwrap(),
    )
    .unwrap();
    let resumed = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resume", "--locator"])
        .arg(&locator)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(value(&resumed, 0)["state_version"], 1);
    let contracts = Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["contracts", "verify", "--json"])
        .output()
        .unwrap();
    value(&contracts, 0);
}

#[test]
fn existing_integration_routes_return_typed_missing_database_errors() {
    let p = Project::new();
    let missing = p.temporary.path().join("missing.db");
    let input = p.temporary.path().join("input.json");
    for operation in [
        vec![
            "integration",
            "gsp",
            "export-corpus",
            "--request",
            input.to_str().unwrap(),
            "--output",
            input.to_str().unwrap(),
        ],
        vec![
            "integration",
            "gsp",
            "link-result",
            "--receipt",
            input.to_str().unwrap(),
            "--receipt-uri",
            "urn:test:receipt",
            "--expected-state-version",
            "1",
            "--idempotency-key",
            "test:missing",
            "--actor-id",
            "human:owner",
            "--observed-at",
            "2026-08-27T12:00:00Z",
        ],
    ] {
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(operation)
            .arg("--database")
            .arg(&missing)
            .args(["--project-ref", &p.id, "--json"])
            .output()
            .unwrap();
        value(&output, 4);
    }
    assert!(!missing.exists());
    assert!(!input.exists());
}

#[test]
fn evidence_run_requires_explicit_policy_and_replay_does_not_execute_again() {
    let p = Project::new();
    let (version, _) = configure(&p);
    let program = assert_cmd::cargo::cargo_bin("project-ledger");
    let policy = p.temporary.path().join("execution-policy.json");
    fs::write(
        &policy,
        serde_json::to_vec(
            &json!({"allowed_programs":[program],"redactions":[],"max_summary_bytes":4096}),
        )
        .unwrap(),
    )
    .unwrap();
    let run = |key: &str, v: u64| {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["evidence", "run", "--database"])
            .arg(&p.database)
            .args([
                "--project-ref",
                &p.id,
                "--expected-version",
                &v.to_string(),
                "--idempotency-key",
                key,
                "--actor-id",
                "agent:codex",
                "--observed-at",
                "2026-08-27T12:00:00Z",
                "--kind",
                "TEST_RESULT",
                "--gate",
                "gate:contracts",
                "--policy",
            ])
            .arg(&policy)
            .arg("--cwd")
            .arg(p.temporary.path())
            .args(["--json", "--"])
            .arg(&program)
            .arg("--version")
            .output()
            .unwrap()
    };
    let first = value(&run("execution:one", version), 0);
    assert_eq!(first["status"], "ACCEPTED");
    assert_eq!(
        value(&run("execution:one", version), 0)["status"],
        "DUPLICATE_REPLAY"
    );
    let after = first["new_state_version"].as_u64().unwrap();
    fs::write(
        &policy,
        serde_json::to_vec(
            &json!({"allowed_programs":[],"redactions":[],"max_summary_bytes":4096}),
        )
        .unwrap(),
    )
    .unwrap();
    value(&run("execution:denied", after), 9);
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], after);
}

#[test]
fn audit_rejects_tampered_command_results_without_writing() {
    let p = Project::new();
    let db = rusqlite::Connection::open(&p.database).unwrap();
    db.execute("UPDATE command_results SET result_digest='sha256:0000000000000000000000000000000000000000000000000000000000000000'",[]).unwrap();
    drop(db);
    let before = fs::read(&p.database).unwrap();
    value(&p.run(&["audit"]), 15);
    assert_eq!(fs::read(&p.database).unwrap(), before);
}

#[test]
fn checkpoint_identity_is_actor_scoped_and_reconcilable() {
    let p = Project::new();
    for (index, actor) in ["agent:first", "agent:second"].into_iter().enumerate() {
        let args = [
            "checkpoint",
            "create",
            "--reason",
            "Explicit checkpoint",
            "--expected-version",
            &(index + 1).to_string(),
            "--idempotency-key",
            "checkpoint:same-key",
            "--actor-id",
            actor,
            "--observed-at",
            "2026-08-27T12:00:00Z",
        ];
        let first = value(&p.run(&args), 0);
        assert_eq!(value(&p.run(&args), 0)["status"], "DUPLICATE_REPLAY");
        let reconciled = value(
            &p.run(&[
                "reconcile",
                "--command-type",
                "CREATE_CONTINUITY_CHECKPOINT",
                "--actor-id",
                actor,
                "--trust-domain",
                "LOCAL_USER",
                "--idempotency-key",
                "checkpoint:same-key",
            ]),
            0,
        );
        assert_eq!(reconciled["result"], first);
    }
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 3);
}

#[test]
fn supervised_execution_is_bounded_private_and_uncertain_attempts_never_retry() {
    let p = Project::new();
    let (version, _) = configure(&p);
    let exe = std::env::current_exe().unwrap();
    let policy = p.temporary.path().join("supervisor-policy.json");
    let marker = p.temporary.path().join("attempts");
    fs::write(&policy,serde_json::to_vec(&json!({"allowed_programs":[exe],"process":{"timeout_ms":700,"environment":{"LEDGER_EXECUTION_MODE":"hang","LEDGER_EXECUTION_MARKER":marker}}})).unwrap()).unwrap();
    let execute = |key: &str| {
        Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["evidence", "run", "--database"])
            .arg(&p.database)
            .args([
                "--project-ref",
                &p.id,
                "--expected-version",
                &version.to_string(),
                "--idempotency-key",
                key,
                "--actor-id",
                "agent:codex",
                "--observed-at",
                "2026-08-27T12:00:00Z",
                "--policy",
            ])
            .arg(&policy)
            .arg("--cwd")
            .arg(p.temporary.path())
            .args(["--json", "--"])
            .arg(&exe)
            .args(["--ignored", "--exact", "execution_fixture", "--nocapture"])
            .output()
            .unwrap()
    };
    assert_eq!(
        value(&execute("run:uncertain"), 12)["error"]["code"],
        "EXECUTION_TIMEOUT"
    );
    let before = fs::read(&marker).unwrap();
    assert_eq!(
        value(&execute("run:uncertain"), 12)["error"]["code"],
        "OUTCOME_UNKNOWN"
    );
    assert_eq!(before, fs::read(&marker).unwrap());
    let canary = "CLI_CANARY_582_NEVER_PERSIST";
    fs::write(&policy,serde_json::to_vec(&json!({"allowed_programs":[exe],"process":{"environment":{"LEDGER_EXECUTION_MODE":"fail","LEDGER_EXECUTION_MARKER":marker,"LEDGER_PRIVATE_CANARY":canary}}})).unwrap()).unwrap();
    value(&execute("run:uncertain"), 10);
    let result = value(&execute("run:known-failure"), 3);
    assert_eq!(result["status"], "ACCEPTED");
    assert_eq!(
        value(&execute("run:known-failure"), 3)["status"],
        "DUPLICATE_REPLAY"
    );
    let after = result["new_state_version"].as_u64().unwrap();
    let evaluated = value(
        &p.write(
            &["gate", "evaluate"],
            json!({"gate_ref":"gate:contracts"}),
            after,
            "gate:failed-process",
        ),
        6,
    );
    assert_eq!(evaluated["status"], "ACCEPTED");
    let status = value(&p.run(&["status"]), 0);
    assert_eq!(status["verified_gate_refs"], json!([]));
    for entry in fs::read_dir(p.database.parent().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            assert!(!String::from_utf8_lossy(&fs::read(path).unwrap()).contains(canary));
        }
    }
}

#[test]
#[ignore = "bounded real child fixture, invoked by evidence run with explicit policy"]
fn execution_fixture() {
    use std::io::Write;
    let mut marker = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(std::env::var("LEDGER_EXECUTION_MARKER").unwrap())
        .unwrap();
    writeln!(marker, "started").unwrap();
    drop(marker);
    if std::env::var("LEDGER_EXECUTION_MODE").unwrap() == "hang" {
        std::thread::sleep(std::time::Duration::from_secs(8));
    } else {
        println!("token={}", std::env::var("LEDGER_PRIVATE_CANARY").unwrap());
        std::process::exit(17);
    }
}

#[test]
fn cancellation_file_stops_the_real_cli_child_without_appending_events() {
    let p = Project::new();
    let exe = std::env::current_exe().unwrap();
    let policy = p.temporary.path().join("cancel-policy.json");
    let marker = p.temporary.path().join("cancel-started");
    let cancel = p.temporary.path().join("cancel-request");
    fs::write(&policy,serde_json::to_vec(&json!({"allowed_programs":[exe],"process":{"environment":{"LEDGER_EXECUTION_MODE":"hang","LEDGER_EXECUTION_MARKER":marker}}})).unwrap()).unwrap();
    let child = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["evidence", "run", "--database"])
        .arg(&p.database)
        .args([
            "--project-ref",
            &p.id,
            "--expected-version",
            "1",
            "--idempotency-key",
            "cancel:one",
            "--actor-id",
            "agent:codex",
            "--observed-at",
            "2026-08-27T12:00:00Z",
            "--policy",
        ])
        .arg(&policy)
        .arg("--cwd")
        .arg(p.temporary.path())
        .arg("--cancel-file")
        .arg(&cancel)
        .args(["--json", "--"])
        .arg(&exe)
        .args(["--ignored", "--exact", "execution_fixture", "--nocapture"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !marker.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    fs::write(&cancel, b"cancel").unwrap();
    let result = value(&child.wait_with_output().unwrap(), 12);
    assert_eq!(result["error"]["code"], "EXECUTION_CANCELLED");
    assert!(marker.exists(), "the child must really have started");
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 1);
}

#[test]
fn evidence_import_privacy_rejection_has_exit_thirteen_and_no_secret_echo() {
    let p = Project::new();
    let canary = "IMPORT_CANARY_NOT_ALLOWED_6371";
    let mut evidence = record("evidence-record", json!({"project_ref":p.id}));
    evidence["result"]["summary"] = json!(format!("token={canary}"));
    evidence.as_object_mut().unwrap().remove("payload_digest");
    evidence["payload_digest"] =
        json!(project_ledger_canonical_json::sha256_digest(&evidence).unwrap());
    let output = p.write(&["evidence", "import"], evidence, 1, "import:private");
    assert!(!String::from_utf8_lossy(&output.stdout).contains(canary));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(canary));
    assert_eq!(value(&output, 13)["error"]["code"], "PRIVACY_REJECTED");
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 1);
}
