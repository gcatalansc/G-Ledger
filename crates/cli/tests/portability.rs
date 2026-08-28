use assert_cmd::prelude::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

struct Project {
    temp: TempDir,
    database: PathBuf,
    id: String,
}
impl Project {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let out = Command::cargo_bin("project-ledger")
            .unwrap()
            .args(["init", "--title", "Disposable recovery", "--data-root"])
            .arg(temp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let text = String::from_utf8(out.stdout).unwrap();
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("{key}=")))
                .unwrap()
                .to_owned()
        };
        Self {
            database: PathBuf::from(field("database")),
            id: field("project_ref"),
            temp,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        selected(&self.database, &self.id, args)
    }
    fn note(&self, key: &str, version: &str, text: &str) -> Output {
        self.run(&[
            "record",
            "note",
            "--kind",
            "LIMITATION",
            "--text",
            text,
            "--actor-id",
            "human:owner",
            "--as-human",
            "--expected-version",
            version,
            "--idempotency-key",
            key,
            "--observed-at",
            "2026-08-27T12:00:00Z",
        ])
    }
}
fn selected(db: &Path, id: &str, args: &[&str]) -> Output {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(args)
        .arg("--database")
        .arg(db)
        .args(["--project-ref", id, "--json"])
        .output()
        .unwrap()
}
fn call(args: &[&str]) -> Output {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .args(args)
        .arg("--json")
        .output()
        .unwrap()
}
fn value(output: &Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn online_backup_restores_wal_results_idempotency_and_uncertain_executions() {
    let p = Project::new();
    let writer = rusqlite::Connection::open(&p.database).unwrap();
    writer
        .execute_batch("PRAGMA wal_autocheckpoint=0; BEGIN; SELECT COUNT(*) FROM events;")
        .unwrap();
    let accepted = value(&p.note("backup:note", "1", "keep this WAL commit"), 0);
    assert!(
        fs::metadata(format!("{}-wal", p.database.display()))
            .unwrap()
            .len()
            > 0
    );
    let mut store =
        project_ledger_storage_sqlite::SqliteEventStore::open_existing(&p.database).unwrap();
    let id = project_ledger_domain::ProjectId::parse(&p.id).unwrap();
    let scope = project_ledger_canonical_json::sha256_bytes(b"scope");
    let intent = project_ledger_canonical_json::sha256_bytes(b"intent");
    store.reserve_execution(&id, scope, intent).unwrap();
    drop(store);
    let bundle = p.temp.path().join("backup con espacios");
    value(&p.run(&["backup", "--output", bundle.to_str().unwrap()]), 0);
    let restored = p.temp.path().join("restauración nueva.db");
    value(
        &call(&[
            "restore",
            "--file",
            bundle.to_str().unwrap(),
            "--database",
            restored.to_str().unwrap(),
        ]),
        0,
    );
    assert_eq!(
        value(&selected(&restored, &p.id, &["status"]), 0)["state_version"],
        2
    );
    let result = value(
        &selected(
            &restored,
            &p.id,
            &[
                "reconcile",
                "--command-type",
                "RECORD_NOTE",
                "--actor-id",
                "human:owner",
                "--as-human",
                "--idempotency-key",
                "backup:note",
            ],
        ),
        0,
    );
    assert_eq!(result["result"], accepted);
    let mut restored_store =
        project_ledger_storage_sqlite::SqliteEventStore::open_existing(&restored).unwrap();
    assert_eq!(
        restored_store
            .reserve_execution(&id, scope, intent)
            .unwrap(),
        project_ledger_storage_sqlite::ExecutionReservation::OutcomeUnknown
    );
    drop(restored_store);
    let before = fs::read(&restored).unwrap();
    value(
        &call(&[
            "restore",
            "--file",
            bundle.to_str().unwrap(),
            "--database",
            restored.to_str().unwrap(),
        ]),
        2,
    );
    assert_eq!(before, fs::read(&restored).unwrap());
    fs::write(bundle.join("ledger.db"), b"corrupted snapshot").unwrap();
    let absent = p.temp.path().join("never-created.db");
    value(
        &call(&[
            "restore",
            "--file",
            bundle.to_str().unwrap(),
            "--database",
            absent.to_str().unwrap(),
        ]),
        15,
    );
    assert!(!absent.exists());
}

#[test]
fn migration_is_explicit_and_retains_a_verified_prechange_snapshot() {
    let p = Project::new();
    let db = rusqlite::Connection::open(&p.database).unwrap();
    db.execute_batch(
        "DELETE FROM schema_migrations WHERE version=3; DROP TABLE execution_attempts;",
    )
    .unwrap();
    drop(db);
    let before = fs::read(&p.database).unwrap();
    value(&p.run(&["status"]), 11);
    assert_eq!(before, fs::read(&p.database).unwrap());
    let bundle = p.temp.path().join("before-migration");
    let occupied = p.temp.path().join("occupied-backup");
    fs::create_dir(&occupied).unwrap();
    value(
        &p.run(&["migrate", "--backup", occupied.to_str().unwrap()]),
        2,
    );
    assert_eq!(before, fs::read(&p.database).unwrap());
    value(
        &p.run(&["migrate", "--backup", bundle.to_str().unwrap()]),
        0,
    );
    value(&p.run(&["status"]), 0);
    let manifest: Value =
        serde_json::from_slice(&fs::read(bundle.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["migration_version"], 2);
    let old = p.temp.path().join("old-restored.db");
    value(
        &call(&[
            "restore",
            "--file",
            bundle.to_str().unwrap(),
            "--database",
            old.to_str().unwrap(),
        ]),
        0,
    );
    value(&selected(&old, &p.id, &["status"]), 11);
    let olddb = rusqlite::Connection::open(&old).unwrap();
    assert_eq!(
        olddb
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        olddb
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn portable_rejects_private_history_and_divergent_same_project_without_rewriting() {
    let p = Project::new();
    let bundle = p.temp.path().join("seed");
    value(&p.run(&["export", "--output", bundle.to_str().unwrap()]), 0);
    let other = p.temp.path().join("other.db");
    value(
        &call(&[
            "import",
            "--file",
            bundle.to_str().unwrap(),
            "--database",
            other.to_str().unwrap(),
        ]),
        0,
    );
    value(&p.note("branch:source", "1", "source branch"), 0);
    value(
        &selected(
            &other,
            &p.id,
            &[
                "record",
                "note",
                "--kind",
                "LIMITATION",
                "--text",
                "other branch",
                "--actor-id",
                "human:owner",
                "--as-human",
                "--expected-version",
                "1",
                "--idempotency-key",
                "branch:other",
                "--observed-at",
                "2026-08-27T12:00:00Z",
            ],
        ),
        0,
    );
    let advanced = p.temp.path().join("advanced");
    value(
        &p.run(&["export", "--output", advanced.to_str().unwrap()]),
        0,
    );
    let before = fs::read(&other).unwrap();
    value(
        &call(&[
            "import",
            "--file",
            advanced.to_str().unwrap(),
            "--database",
            other.to_str().unwrap(),
        ]),
        15,
    );
    assert_eq!(before, fs::read(&other).unwrap());
    value(&p.note("privacy:note", "2", "token=FAKE_PRIVATE_CANARY"), 0);
    let private = p.temp.path().join("rejected");
    let out = p.run(&["export", "--output", private.to_str().unwrap()]);
    value(&out, 13);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("FAKE_PRIVATE_CANARY"));
    assert!(!private.exists());
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 3);
}

#[test]
fn portable_roundtrip_fast_forward_divergence_and_fork_fail_closed() {
    let p = Project::new();
    let export = p.temp.path().join("portable one");
    value(
        &p.run(&[
            "export",
            "--format",
            "portable",
            "--output",
            export.to_str().unwrap(),
        ]),
        0,
    );
    let dest = p.temp.path().join("imported.db");
    let args = [
        "import",
        "--file",
        export.to_str().unwrap(),
        "--database",
        dest.to_str().unwrap(),
    ];
    value(&call(&args), 0);
    assert_eq!(value(&call(&args), 0)["disposition"], "IDENTICAL_CHAIN");
    let fork = p.temp.path().join("fork.db");
    value(
        &call(&[
            "import",
            "--file",
            export.to_str().unwrap(),
            "--database",
            fork.to_str().unwrap(),
            "--mode",
            "fork",
        ]),
        2,
    );
    assert!(!fork.exists());
    value(&p.note("portable:note", "1", "safe historical note"), 0);
    let export2 = p.temp.path().join("portable two");
    value(
        &p.run(&[
            "export",
            "--format",
            "portable",
            "--output",
            export2.to_str().unwrap(),
        ]),
        0,
    );
    value(
        &call(&[
            "import",
            "--file",
            export2.to_str().unwrap(),
            "--database",
            dest.to_str().unwrap(),
        ]),
        0,
    );
    assert_eq!(
        value(&selected(&dest, &p.id, &["status"]), 0)["state_version"],
        2
    );
    let before = fs::read(&dest).unwrap();
    value(&call(&args), 15);
    assert_eq!(before, fs::read(&dest).unwrap());
    fs::write(export2.join("events.jsonl"), b"tampered").unwrap();
    value(
        &call(&[
            "import",
            "--file",
            export2.to_str().unwrap(),
            "--database",
            dest.to_str().unwrap(),
        ]),
        15,
    );
    assert_eq!(before, fs::read(&dest).unwrap());
}

#[test]
fn rendered_markdown_detects_edits_and_requires_explicit_regeneration() {
    let p = Project::new();
    let output = p.temp.path().join("CONTINUIDAD.md");
    value(&p.run(&["render", "--output", output.to_str().unwrap()]), 0);
    let canonical = fs::read(&output).unwrap();
    value(
        &p.run(&["verify", "--artifact", output.to_str().unwrap()]),
        0,
    );
    fs::write(&output, "manually changed").unwrap();
    value(
        &p.run(&["verify", "--artifact", output.to_str().unwrap()]),
        15,
    );
    value(&p.run(&["render", "--output", output.to_str().unwrap()]), 2);
    value(
        &p.run(&[
            "render",
            "--output",
            output.to_str().unwrap(),
            "--regenerate",
        ]),
        0,
    );
    assert_eq!(canonical, fs::read(&output).unwrap());
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 1);
    let user_file = p.temp.path().join("user.md");
    fs::write(&user_file, "user owned").unwrap();
    value(
        &p.run(&[
            "render",
            "--output",
            user_file.to_str().unwrap(),
            "--regenerate",
        ]),
        15,
    );
    assert_eq!(fs::read_to_string(&user_file).unwrap(), "user owned");
}

#[test]
fn legacy_review_preserves_contradictions_as_assertions_without_promoting_lifecycle() {
    let p = Project::new();
    let markdown = p.temp.path().join("HISTORIA.md");
    fs::write(&markdown, "# Historia\n- Fase activa: phase-one\n- Fase activa: phase-two\n- Fase completada: phase-one\n- Gate verificado: gate:release\n").unwrap();
    let base = [
        "import-markdown",
        "--file",
        markdown.to_str().unwrap(),
        "--review-id",
        "review:history",
        "--observed-at",
        "2026-08-27T12:00:00Z",
    ];
    let mut preview_args = base.to_vec();
    preview_args.push("--preview");
    let preview = value(&p.run(&preview_args), 0);
    assert_eq!(preview["candidates"].as_array().unwrap().len(), 4);
    assert!(
        !preview["candidates"][0]["conflicts_with"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 1);
    let decisions = p.temp.path().join("decisions.json");
    let choices = preview["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            json!({
        "candidate_ref":c["candidate_id"],"action":"IMPORT_AS_ASSERTED","actor_ref":"human:owner",
        "reason":"Historical statement only, not a current verified fact"})
        })
        .collect::<Vec<_>>();
    let mut apply = base.to_vec();
    apply.extend([
        "--apply-reviewed",
        "--review",
        decisions.to_str().unwrap(),
        "--preview-digest",
        preview["digest"].as_str().unwrap(),
        "--actor-id",
        "human:owner",
        "--expected-version",
        "1",
        "--idempotency-key",
        "history:apply",
    ]);
    fs::write(&decisions, serde_json::to_vec(&choices).unwrap()).unwrap();
    value(&p.run(&apply), 9);
    apply.push("--as-human");
    fs::write(&decisions, serde_json::to_vec(&choices[..3]).unwrap()).unwrap();
    value(&p.run(&apply), 2);
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 1);
    fs::write(&decisions, serde_json::to_vec(&choices).unwrap()).unwrap();
    let applied = value(&p.run(&apply), 0);
    assert_eq!(applied["imported_assertions"], 4);
    let status = value(&p.run(&["status"]), 0);
    assert_eq!(status["state_version"], 5);
    assert_eq!(status["project"]["lifecycle_status"], "INITIALIZING");
    assert!(
        status["completed_phase_refs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    value(&p.run(&apply), 0);
    assert_eq!(value(&p.run(&["status"]), 0)["state_version"], 5);
    let db = rusqlite::Connection::open(&p.database).unwrap();
    let events: Vec<Vec<u8>> = db
        .prepare("SELECT event_json FROM events WHERE event_type='CLAIM_RECORDED'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for bytes in events {
        let event: Value = serde_json::from_slice(&bytes).unwrap();
        let claim = &event["payload"]["claim"];
        assert_eq!(claim["classification"], "ASSERTED");
        assert!(
            claim["statement"]
                .as_str()
                .unwrap()
                .contains("source_digest")
        );
        assert!(
            claim["statement"]
                .as_str()
                .unwrap()
                .contains("conflicts_with")
        );
        assert!(claim["evidence_refs"].as_array().unwrap().is_empty());
    }
    fs::write(&markdown, "# Changed\n- Fase activa: phase-three\n").unwrap();
    value(&p.run(&apply), 15);
}
