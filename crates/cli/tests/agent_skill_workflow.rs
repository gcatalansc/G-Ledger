use std::{
    path::{Path, PathBuf},
    process::Command,
};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use project_ledger_domain::ProjectId;
use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn ac_061_cli_verifies_the_exact_agent_skill_package() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "agent-skill", "verify-contracts"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "agent skill MATCH schemas=2 triggers=7 policies=5 profile=project-ledger-agent-skill/0.2.0",
        ));
}

#[test]
fn ac_063_ac_064_checkpoint_resume_handoff_is_real_read_only_and_idempotent() {
    let temporary = tempdir().unwrap();
    let initialized = Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["init", "--title", "Agent handoff", "--data-root"])
        .arg(temporary.path())
        .output()
        .unwrap();
    assert!(initialized.status.success());
    let initialized = String::from_utf8(initialized.stdout).unwrap();
    let project_ref = value(&initialized, "project_ref").to_owned();
    let database = PathBuf::from(value(&initialized, "database"));
    assert_eq!(current_version(&database, &project_ref), 1);

    let first_resume = resume(&database, &project_ref, "json");
    assert!(first_resume.status.success());
    let first: Value = serde_json::from_slice(&first_resume.stdout).unwrap();
    assert_eq!(first["state_version"], 1);
    assert_eq!(first["checkpoint_freshness"], "UNKNOWN");
    assert_eq!(first["checkpoint_ref"], Value::Null);
    assert_eq!(current_version(&database, &project_ref), 1);

    let checkpoint_output = checkpoint(
        &database,
        &project_ref,
        "1",
        "handoff:session-01",
        "2026-08-23T20:00:00-04:00",
    );
    assert!(
        checkpoint_output.status.success(),
        "{}",
        String::from_utf8_lossy(&checkpoint_output.stderr)
    );
    let result: Value = serde_json::from_slice(&checkpoint_output.stdout).unwrap();
    assert_eq!(result["status"], "ACCEPTED");
    assert_eq!(result["previous_state_version"], 1);
    assert_eq!(result["new_state_version"], 2);
    assert_eq!(current_version(&database, &project_ref), 2);

    let replay = checkpoint(
        &database,
        &project_ref,
        "1",
        "handoff:session-01",
        "2026-08-23T20:00:00-04:00",
    );
    assert!(replay.status.success());
    let replay: Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(replay["status"], "DUPLICATE_REPLAY");
    assert_eq!(current_version(&database, &project_ref), 2);

    let final_resume = resume(&database, &project_ref, "json");
    assert!(final_resume.status.success());
    let final_projection: Value = serde_json::from_slice(&final_resume.stdout).unwrap();
    assert_eq!(final_projection["state_version"], 2);
    assert_eq!(
        final_projection["checkpoint_freshness"],
        "CURRENT_FOR_PROJECT_ONLY"
    );
    assert!(final_projection["checkpoint_ref"].as_str().is_some());
    assert_eq!(current_version(&database, &project_ref), 2);

    let conflict = checkpoint(
        &database,
        &project_ref,
        "1",
        "handoff:stale-session",
        "2026-08-23T20:01:00-04:00",
    );
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("VERSION_CONFLICT"));
    assert_eq!(current_version(&database, &project_ref), 2);

    let codex_resume = resume(&database, &project_ref, "codex");
    assert!(codex_resume.status.success());
    let codex_resume = String::from_utf8(codex_resume.stdout).unwrap();
    assert!(codex_resume.starts_with("# Project Ledger Resume\n"));
    assert!(codex_resume.contains("- Freshness: `CURRENT_FOR_PROJECT_ONLY`"));
    assert_eq!(current_version(&database, &project_ref), 2);
}

fn resume(database: &Path, project_ref: &str, format: &str) -> std::process::Output {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["resume", "--database"])
        .arg(database)
        .args(["--project-ref", project_ref, "--format", format])
        .output()
        .unwrap()
}

fn checkpoint(
    database: &Path,
    project_ref: &str,
    expected_version: &str,
    idempotency_key: &str,
    observed_at: &str,
) -> std::process::Output {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(["checkpoint", "create", "--database"])
        .arg(database)
        .args([
            "--project-ref",
            project_ref,
            "--reason",
            "session handoff",
            "--expected-version",
            expected_version,
            "--idempotency-key",
            idempotency_key,
            "--actor-id",
            "agent:codex",
            "--observed-at",
            observed_at,
        ])
        .output()
        .unwrap()
}

fn current_version(database: &Path, project_ref: &str) -> u64 {
    let project_ref = ProjectId::parse(project_ref).unwrap();
    let store = SqliteEventStore::open(database).unwrap();
    store.current_version(&project_ref).unwrap()
}

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

fn value<'a>(output: &'a str, key: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key} in output: {output}"))
}
