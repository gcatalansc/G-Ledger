use std::{path::PathBuf, process::Command};

use assert_cmd::prelude::*;
use project_ledger_domain::ProjectId;
use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn ac_001_init_is_offline_and_creates_one_canonical_project_transaction() {
    let directory = tempdir().unwrap();
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .args([
            "init",
            "--title",
            "Ledger offline",
            "--summary",
            "Real SQLite bootstrap",
            "--timezone",
            "America/Santiago",
            "--data-root",
        ])
        .arg(directory.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let project_ref = value(&stdout, "project_ref");
    let database = PathBuf::from(value(&stdout, "database"));
    assert_eq!(value(&stdout, "state_version"), "1");
    assert!(database.starts_with(directory.path().join("project-ledger/projects")));
    assert!(database.ends_with("ledger.db"));
    assert!(database.is_file());

    let project_ref = ProjectId::parse(project_ref).unwrap();
    let store = SqliteEventStore::open(&database).unwrap();
    assert_eq!(store.current_version(&project_ref).unwrap(), 1);
    assert_eq!(store.event_count(&project_ref).unwrap(), 1);
    let report = store.recovery_report(&project_ref).unwrap();
    assert_eq!(report.command_count, 1);
    assert_eq!(report.command_result_count, 1);
    assert_eq!(report.unresolved_submission_count, 0);
    assert!(report.chain_verified);
    assert_eq!(report.quick_check, "ok");
}

fn value<'a>(output: &'a str, key: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key} in output: {output}"))
}
