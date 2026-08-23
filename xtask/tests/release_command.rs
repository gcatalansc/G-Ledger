use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn ac_089_benchmark_report_is_complete_measured_and_within_declared_budgets() {
    let mut command = Command::cargo_bin("xtask").unwrap();
    command
        .current_dir(workspace_root())
        .args(["benchmark", "check", "--root"])
        .arg(workspace_root())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "8 measured metrics, 1 explicit waiver",
        ));
}

#[test]
fn release_artifacts_are_deterministic_and_the_aggregate_gate_passes() {
    let mut generate = Command::cargo_bin("xtask").unwrap();
    generate
        .current_dir(workspace_root())
        .args(["release", "generate", "--check", "--root"])
        .arg(workspace_root())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "release artifacts are reproducible",
        ));

    let mut check = Command::cargo_bin("xtask").unwrap();
    check
        .current_dir(workspace_root())
        .args(["release", "check", "--root"])
        .arg(workspace_root())
        .assert()
        .success()
        .stdout(predicate::str::contains("90 AC, 32 PL, 0 secrets"));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
