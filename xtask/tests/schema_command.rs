use std::process::Command;

use assert_cmd::prelude::*;

#[test]
fn schema_generate_and_check_operate_on_an_explicit_repository_root() {
    let temporary = tempfile::tempdir().unwrap();

    Command::cargo_bin("xtask")
        .unwrap()
        .args(["schema", "generate", "--root"])
        .arg(temporary.path())
        .assert()
        .success();
    assert!(
        temporary
            .path()
            .join("schemas/v0.2/schema-registry.json")
            .is_file()
    );

    Command::cargo_bin("xtask")
        .unwrap()
        .args(["schema", "check", "--root"])
        .arg(temporary.path())
        .assert()
        .success();
}
