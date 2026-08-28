use assert_cmd::prelude::*;
use project_ledger_agent_skill::{
    EvidenceAvailability, RuntimeConditions, SkillIntent, SkillPlanStatus, SubmissionState,
    ToolAvailability, plan_skill_actions,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const PACKAGE: &str = "integrations/agent-skills/project-ledger-continuity";
const AT: &str = "2026-08-27T23:00:00Z";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn commands() -> Value {
    let path = root()
        .join(PACKAGE)
        .join("examples/operational-session.json");
    serde_json::from_slice(&fs::read(path).expect("skill must provide executable argv examples"))
        .unwrap()
}

fn run(example: &Value, name: &str, values: &BTreeMap<String, String>) -> std::process::Output {
    let args: Vec<String> = example["commands"][name]
        .as_array()
        .unwrap_or_else(|| panic!("missing executable command {name}"))
        .iter()
        .map(|arg| {
            let arg = arg.as_str().unwrap();
            if arg.starts_with("${") {
                values
                    .get(arg)
                    .unwrap_or_else(|| panic!("missing placeholder {arg}"))
                    .clone()
            } else {
                arg.to_owned()
            }
        })
        .collect();
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(args)
        .output()
        .unwrap()
}

fn json_output(output: &std::process::Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{} / {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn state(values: &BTreeMap<String, String>) -> Value {
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args([
            "status",
            "--database",
            &values["${DATABASE_PATH}"],
            "--project-ref",
            &values["${PROJECT_REF}"],
            "--json",
        ])
        .output()
        .unwrap();
    json_output(&output, 0)
}

fn refresh(values: &mut BTreeMap<String, String>) -> u64 {
    let version = state(values)["state_version"].as_u64().unwrap();
    values.insert("${STATE_VERSION}".into(), version.to_string());
    version
}

fn fixture(name: &str) -> Value {
    let mut value = project_ledger_schemas::fixture_catalog()
        .unwrap()
        .into_iter()
        .find(|f| f.schema_id.ends_with(&format!(":{name}:0.2.0")))
        .unwrap()
        .positive;
    if name != "authority-document" && value.get("digest").is_some() {
        value.as_object_mut().unwrap().remove("digest");
        value["digest"] = json!(project_ledger_canonical_json::sha256_digest(&value).unwrap());
    }
    value
}

// Setup is an explicitly human-owned disposable fixture, not an agent escalation.
fn setup(path: &Path, values: &mut BTreeMap<String, String>) {
    for (index, (args, payload)) in [
        (
            ["authority", "add"],
            json!({"document": fixture("authority-document")}),
        ),
        (
            ["roadmap", "import"],
            json!({"roadmap": fixture("roadmap")}),
        ),
        (
            ["gate", "define"],
            json!({"gate": fixture("gate-definition")}),
        ),
        (["phase", "start"], json!({"phase_ref":"phase-0"})),
    ]
    .into_iter()
    .enumerate()
    {
        let payload_path = path.join("human-setup.json");
        fs::write(&payload_path, serde_json::to_vec(&payload).unwrap()).unwrap();
        let version = refresh(values);
        let output = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(args)
            .arg("--file")
            .arg(payload_path)
            .args([
                "--database",
                &values["${DATABASE_PATH}"],
                "--project-ref",
                &values["${PROJECT_REF}"],
                "--as-human",
                "--actor-id",
                "human:fixture-owner",
                "--observed-at",
                AT,
                "--expected-version",
                &version.to_string(),
                "--idempotency-key",
                &format!("setup:{index}"),
                "--json",
            ])
            .output()
            .unwrap();
        json_output(&output, 0);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // The complete procedure is kept in observable execution order.
fn published_codex_argv_runs_the_full_governed_flow_without_direct_storage_access() {
    let example = commands();
    let temp = tempfile::tempdir().unwrap();
    let initialized = Command::cargo_bin("project-ledger")
        .unwrap()
        .args([
            "init",
            "--title",
            "Discardable Codex procedure",
            "--data-root",
        ])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(initialized.status.success());
    let text = String::from_utf8(initialized.stdout).unwrap();
    let field = |name| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .unwrap()
            .to_owned()
    };
    let tool = assert_cmd::cargo::cargo_bin("project-ledger");
    let policy = temp.path().join("explicit-process-policy.json");
    fs::write(
        &policy,
        serde_json::to_vec(&json!({"allowed_programs":[tool],"redactions":[]})).unwrap(),
    )
    .unwrap();
    let mut values = BTreeMap::from([
        ("${DATABASE_PATH}".into(), field("database")),
        ("${PROJECT_REF}".into(), field("project_ref")),
        ("${ACTOR_ID}".into(), "agent:codex".into()),
        ("${OBSERVED_AT}".into(), AT.into()),
        ("${STATE_VERSION}".into(), "1".into()),
        ("${IDEMPOTENCY_KEY}".into(), "codex:progress".into()),
        ("${GATE_REF}".into(), "gate:contracts".into()),
        ("${POLICY_PATH}".into(), policy.to_str().unwrap().into()),
        (
            "${WORKING_DIRECTORY}".into(),
            temp.path().to_str().unwrap().into(),
        ),
        ("${AUTHORIZED_TOOL}".into(), tool.to_str().unwrap().into()),
        (
            "${SUMMARY}".into(),
            "Bounded procedure exercised; no completion claim".into(),
        ),
    ]);
    setup(temp.path(), &mut values);
    let version = refresh(&mut values);
    assert!(run(&example, "probe_version", &values).status.success());
    for name in [
        "probe_progress",
        "probe_evidence",
        "probe_evaluate",
        "probe_checkpoint",
        "probe_reconcile",
    ] {
        assert!(run(&example, name, &values).status.success());
    }
    assert_eq!(
        json_output(&run(&example, "resume", &values), 0)["state_version"],
        version
    );
    json_output(&run(&example, "progress", &values), 0);
    let progress_version = refresh(&mut values);
    assert!(progress_version > version);
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:denied".into());
    json_output(&run(&example, "human_only_negative", &values), 9);
    assert_eq!(refresh(&mut values), progress_version);
    values.insert("${STATE_VERSION}".into(), version.to_string());
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:stale".into());
    json_output(&run(&example, "progress", &values), 5);
    assert_eq!(refresh(&mut values), progress_version);
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:evidence".into());
    json_output(&run(&example, "evidence", &values), 0);
    refresh(&mut values);
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:evaluate".into());
    let evaluated = json_output(&run(&example, "evaluate", &values), 6);
    assert_eq!(evaluated["status"], "ACCEPTED"); // Accepted evaluation is NOT a passing gate.
    let before_checkpoint = refresh(&mut values);
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:checkpoint".into());
    // Deliberately discard the client response; resolve uncertainty by read-only reconciliation.
    assert!(run(&example, "checkpoint", &values).status.success());
    let after_checkpoint = refresh(&mut values);
    assert_eq!(after_checkpoint, before_checkpoint + 1);
    let reconciled = json_output(&run(&example, "reconcile_checkpoint", &values), 0);
    assert_eq!(reconciled["result"]["new_state_version"], after_checkpoint);
    assert_eq!(refresh(&mut values), after_checkpoint);
    let resumed = json_output(&run(&example, "resume", &values), 0);
    assert_eq!(resumed["state_version"], after_checkpoint);
    assert!(resumed["checkpoint_ref"].is_string());
    assert_eq!(
        resumed["authority_document_refs"],
        json!(["authority:spec"])
    );
    assert_eq!(resumed["pending_gate_refs"], json!(["gate:contracts"]));
    assert_eq!(
        resumed["last_material_progress_summary"],
        values["${SUMMARY}"]
    );
    assert_eq!(state(&values)["verified_gate_refs"], json!([]));
    assert!(
        state(&values)["completed_phase_refs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:unresolved".into());
    json_output(&run(&example, "reconcile_checkpoint", &values), 12);
    assert_eq!(refresh(&mut values), after_checkpoint);
    values.insert("${IDEMPOTENCY_KEY}".into(), "codex:checkpoint".into());
    values.insert("${STATE_VERSION}".into(), before_checkpoint.to_string());
    assert_eq!(
        json_output(&run(&example, "checkpoint", &values), 0)["status"],
        "DUPLICATE_REPLAY"
    );
    assert_eq!(refresh(&mut values), after_checkpoint);
}

// This probe is the test interpreter of the documented procedure, not a claim of universal model obedience.
fn supported_version(tool: &Path, args: &[&str]) -> bool {
    Command::new(tool).args(args).output().is_ok_and(|out| {
        out.status.success()
            && String::from_utf8_lossy(&out.stdout).lines().any(|line| {
                line.strip_prefix("project-ledger ").is_some_and(|version| {
                    version
                        .strip_prefix("0.2.")
                        .is_some_and(|patch| patch.parse::<u64>().is_ok())
                })
            })
    })
}

#[test]
fn missing_and_outdated_processes_degrade_before_any_ledger_operation() {
    let temp = tempfile::tempdir().unwrap();
    assert!(!supported_version(
        &temp.path().join("missing-ledger"),
        &["--version"]
    ));
    assert!(supported_version(
        &assert_cmd::cargo::cargo_bin("project-ledger"),
        &["--version"]
    ));
    // An owned process emitting an old version, NOT an actual historical Ledger release.
    let old = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "old_version_process_fixture",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(old.status.success());
    assert!(
        String::from_utf8_lossy(&old.stdout)
            .lines()
            .any(|line| line == "project-ledger 0.1.0")
    );
    assert!(!supported_version(
        &std::env::current_exe().unwrap(),
        &[
            "--ignored",
            "--exact",
            "old_version_process_fixture",
            "--nocapture"
        ]
    ));
    let plan = plan_skill_actions(
        SkillIntent::StartOrResume,
        RuntimeConditions {
            tool: ToolAvailability::Missing,
            evidence: EvidenceAvailability::Unavailable,
            submission: SubmissionState::Clean,
        },
    );
    assert_eq!(plan.status, SkillPlanStatus::DegradedMissingTool);
    assert!(plan.actions.is_empty());
    // A supported version still must reject a capability that this executable does not expose.
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["absent-capability", "--help"])
        .assert()
        .code(2);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
#[ignore = "owned process fixture for an unsupported version response, not a released binary"]
fn old_version_process_fixture() {
    println!("project-ledger 0.1.0");
}
