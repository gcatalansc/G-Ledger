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
fn exact_candidate_qualifier_pins_every_primary_checkout_and_artifact_identity() {
    const CANDIDATE_SHA: &str = "bd564cdba07b616337681c15801d22478824108b";
    const CANDIDATE_EXPRESSION: &str = "${{ env.PROJECT_LEDGER_CANDIDATE_SHA }}";

    let text = std::fs::read_to_string(workspace_root().join(".github/workflows/ci.yml"))
        .expect("exact-candidate qualification workflow must be readable");
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(&text).expect("exact-candidate qualification workflow must parse");

    assert_eq!(
        workflow["env"]["PROJECT_LEDGER_CANDIDATE_SHA"].as_str(),
        Some(CANDIDATE_SHA),
        "the auxiliary workflow must select the authorized candidate"
    );

    let jobs = workflow["jobs"].as_mapping().expect("jobs must be a map");
    for (job_name, job) in jobs {
        let steps = job["steps"]
            .as_sequence()
            .expect("steps must be a sequence");
        let checkout_index = steps
            .iter()
            .position(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|uses| uses.starts_with("actions/checkout@"))
                    && step["with"].get("repository").is_none()
            })
            .unwrap_or_else(|| panic!("job {job_name:?} must checkout Project Ledger"));
        let checkout = &steps[checkout_index];
        assert_eq!(
            checkout["with"]["ref"].as_str(),
            Some(CANDIDATE_EXPRESSION),
            "job {job_name:?} must checkout the authorized SHA, not the PR merge ref"
        );
        assert_eq!(
            checkout["with"]["persist-credentials"].as_bool(),
            Some(false),
            "the read-only qualifier must not retain Git credentials"
        );
        assert_eq!(
            steps
                .get(checkout_index + 1)
                .and_then(|step| step["name"].as_str()),
            Some("Verify exact candidate checkout"),
            "every job must fail before testing if checkout identity drifted"
        );
    }

    let upload = jobs
        .values()
        .flat_map(|job| job["steps"].as_sequence().into_iter().flatten())
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
        })
        .expect("Windows delivery upload step must exist");
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("project-ledger-windows-${{ env.PROJECT_LEDGER_CANDIDATE_SHA }}"),
        "artifact identity must use the candidate SHA rather than github.sha"
    );
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
