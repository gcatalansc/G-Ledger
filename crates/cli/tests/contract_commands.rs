use std::{path::Path, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

#[test]
fn specs_verify_reports_both_normative_digests() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["specs", "verify"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v0.1.0 MATCH 772ea517"))
        .stdout(predicate::str::contains("v0.2.0 MATCH 31cb7cd4"));
}

#[test]
fn contracts_verify_checks_generated_artifacts_and_catalogs() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["contracts", "verify"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "contracts MATCH schemas=46 fixtures=92 matrix_rows=53",
        ));
}

#[test]
fn unknown_command_is_rejected_by_the_cli_parser() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .arg("mutate-directly")
        .assert()
        .failure();
}
