use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn published_workflow_parses_and_run_steps_are_executable_strings() {
    let text = std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml")).unwrap();
    let workflow: serde_yaml::Value = serde_yaml::from_str(&text).expect("published CI must parse");
    let jobs = workflow["jobs"].as_mapping().unwrap();
    assert!(!jobs.is_empty());
    for job in jobs.values() {
        for step in job["steps"].as_sequence().unwrap() {
            if let Some(run) = step.get("run") {
                assert!(
                    !run.as_str()
                        .expect("run must be a command string")
                        .trim()
                        .is_empty()
                );
            }
        }
    }
}

#[test]
fn ac_083_ac_086_ac_088_ac_090_cli_verifies_zero_waivers_without_certification_overclaim() {
    let mut command = Command::cargo_bin("project-ledger").unwrap();
    command
        .current_dir(workspace_root())
        .args(["release", "verify", "--root"])
        .arg(workspace_root())
        .assert()
        .success()
        .stdout(predicate::str::contains("acceptance_criteria_count\":90"))
        .stdout(predicate::str::contains("benchmark_scenario_count\":32"))
        .stdout(predicate::str::contains("secret_persistence_count\":0"))
        .stdout(predicate::str::contains("waived_count\":0"))
        .stdout(predicate::str::contains("chaos_waived_count\":0"))
        .stdout(predicate::str::contains(
            "general_availability_blocker_count\":0",
        ))
        .stdout(predicate::str::contains(
            "gsp_runtime_certification_blocker_count\":0",
        ))
        .stdout(predicate::str::contains(
            "overall_certified_compatibility\":false",
        ));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn distribution_identity_does_not_reuse_baseline_acceptance() {
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["release", "verify", "--root"])
        .arg(workspace_root())
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["distribution"]["operational_acceptance"],
        "NOT_GRANTED"
    );
    assert_eq!(report["distribution"]["product_version"], "0.3.0-rc.1");
    assert_eq!(report["distribution"]["certified_compatibility"], false);
    assert_eq!(report["verified_evidence_subject"], "v0.2.0-baseline");
    Command::cargo_bin("project-ledger")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout("project-ledger 0.3.0-rc.1\n");
}
