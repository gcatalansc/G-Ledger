use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

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
        .stdout(predicate::str::contains("chaos_waived_count\":2"))
        .stdout(predicate::str::contains(
            "general_availability_blocker_count\":1",
        ))
        .stdout(predicate::str::contains(
            "gsp_runtime_certification_blocker_count\":1",
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
