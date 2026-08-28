use assert_cmd::prelude::*;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn source() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn resource_catalog_exactly_matches_the_reviewed_distribution_allowlist() {
    let list = fs::read_to_string(source().join("crates/cli/resources.txt")).expect(
        "distribution must have a reviewed explicit file allowlist, not directory discovery",
    );
    let expected: Vec<_> = list
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resources", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let catalog: Value = serde_json::from_slice(&output.stdout).unwrap();
    let actual: Vec<_> = catalog["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        actual, expected,
        "unreviewed local files must not enter a release"
    );
}

#[test]
fn resource_queries_do_not_take_authority_from_the_working_directory() {
    let cwd = tempfile::tempdir().unwrap();
    fs::create_dir(cwd.path().join("schemas")).unwrap();
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(cwd.path())
        .args(["contracts", "verify", "--json"])
        .assert()
        .success();
}

#[test]
fn installed_resources_are_locked_and_explicit_selection_never_falls_back() {
    let list = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resources", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let catalog: Value = serde_json::from_slice(&list.stdout).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("Ledger instalación á ")
        .tempdir()
        .unwrap();
    let install = temp.path().join("install");
    let resources = install.join("resources");
    fs::create_dir_all(&resources).unwrap();
    for file in catalog["files"].as_array().unwrap() {
        let relative = file["path"].as_str().unwrap();
        let destination = resources.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source().join(relative), destination).unwrap();
    }
    let binary = install.join(format!("project-ledger{}", std::env::consts::EXE_SUFFIX));
    fs::copy(assert_cmd::cargo::cargo_bin("project-ledger"), &binary).unwrap();
    for args in [
        ["resources", "verify"],
        ["specs", "verify"],
        ["contracts", "verify"],
        ["release", "verify"],
    ] {
        Command::new(&binary)
            .current_dir(temp.path())
            .args(args)
            .assert()
            .success();
    }
    // Even a valid installation cannot mask an invalid explicit selection.
    Command::new(&binary)
        .args([
            "--resources",
            temp.path().to_str().unwrap(),
            "specs",
            "verify",
            "--json",
        ])
        .assert()
        .code(15);
    let altered = resources.join("specs/PROJECT_LEDGER_SPEC_v0.2.0.md");
    fs::write(&altered, b"modified installation").unwrap();
    Command::new(&binary)
        .current_dir(source())
        .args(["contracts", "verify", "--json"])
        .assert()
        .code(15);
    // An explicitly supplied, matching resource directory overrides a damaged installation.
    Command::new(&binary)
        .args([
            "--resources",
            source().to_str().unwrap(),
            "contracts",
            "verify",
        ])
        .assert()
        .success();
    // Core state operations do not require optional resource consumers to be available.
    Command::new(&binary)
        .current_dir(temp.path())
        .args(["init", "--title", "Independent core", "--data-root"])
        .arg(temp.path().join("data"))
        .assert()
        .success();
}
