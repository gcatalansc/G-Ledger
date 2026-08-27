use std::{
    fs,
    path::{Path, PathBuf},
};

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
            "9 measured metrics, 0 explicit waivers",
        ));
}

#[test]
fn ac_089_previous_peak_rss_waiver_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let benchmark_directory = directory.path().join("benchmark");
    fs::create_dir_all(&benchmark_directory).unwrap();
    fs::copy(
        workspace_root().join("xtask/tests/fixtures/benchmark-with-peak-rss-waiver.json"),
        benchmark_directory.join("project-ledger-v0.2.0-windows-x86_64.json"),
    )
    .unwrap();

    let mut command = Command::cargo_bin("xtask").unwrap();
    command
        .current_dir(workspace_root())
        .args(["benchmark", "check", "--root"])
        .arg(directory.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "benchmark report is incomplete, over budget or has an invalid waiver",
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
