use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn binary() -> PathBuf {
    std::env::var_os("PROJECT_LEDGER_PACKAGE_BINARY").map_or_else(
        || assert_cmd::cargo::cargo_bin("project-ledger"),
        PathBuf::from,
    )
}

fn run(cwd: &Path, args: &[&str], exit: i32) -> Value {
    let mut command = Command::new(binary());
    command
        .current_dir(cwd)
        .env_clear()
        .arg("--json")
        .args(args);
    for key in ["SystemRoot", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "args={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
#[allow(clippy::too_many_lines)]
fn portable_artifact_operates_recovers_and_rejects_private_exports_without_toolchains() {
    let temp = tempfile::Builder::new()
        .prefix("Ledger paquete recuperación á ")
        .tempdir()
        .unwrap();
    let cwd = temp.path();
    let initialized = run(
        cwd,
        &[
            "init",
            "--title",
            "Discardable portable package",
            "--data-root",
            cwd.to_str().unwrap(),
        ],
        0,
    );
    let output = initialized["output"].as_str().unwrap();
    let field = |key| {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap()
            .to_owned()
    };
    let db = field("database");
    let id = field("project_ref");
    let selected = |args: &[&str], exit| {
        let mut args = args.to_vec();
        args.extend(["--database", &db, "--project-ref", &id]);
        run(cwd, &args, exit)
    };
    let at = "2026-08-27T23:00:00Z";
    let note = [
        "record",
        "note",
        "--kind",
        "LIMITATION",
        "--text",
        "Package smoke only",
        "--actor-id",
        "agent:package",
        "--observed-at",
        at,
        "--expected-version",
        "1",
        "--idempotency-key",
        "package:note",
    ];
    selected(&note, 0);
    assert_eq!(selected(&note, 0)["status"], "DUPLICATE_REPLAY");
    let process_canary = "FAKE_PACKAGE_PROCESS_CANARY";
    let policy = cwd.join("explicit-evidence-policy.json");
    fs::write(
        &policy,
        serde_json::to_vec(&json!({"allowed_programs":[binary()]})).unwrap(),
    )
    .unwrap();
    let failure = run(
        cwd,
        &[
            "evidence",
            "run",
            "--database",
            &db,
            "--project-ref",
            &id,
            "--kind",
            "TEST_RESULT",
            "--actor-id",
            "agent:package",
            "--observed-at",
            at,
            "--expected-version",
            "2",
            "--idempotency-key",
            "package:failed-evidence",
            "--policy",
            policy.to_str().unwrap(),
            "--cwd",
            cwd.to_str().unwrap(),
            "--",
            binary().to_str().unwrap(),
            "--token=FAKE_PACKAGE_PROCESS_CANARY",
        ],
        3,
    );
    assert!(!failure.to_string().contains(process_canary));
    assert!(!String::from_utf8_lossy(&fs::read(&db).unwrap()).contains(process_canary));
    let checkpoint = [
        "checkpoint",
        "create",
        "--reason",
        "Package recovery",
        "--actor-id",
        "agent:package",
        "--observed-at",
        at,
        "--expected-version",
        "3",
        "--idempotency-key",
        "package:checkpoint",
    ];
    selected(&checkpoint, 0);
    let reconciled = selected(
        &[
            "reconcile",
            "--command-type",
            "CREATE_CONTINUITY_CHECKPOINT",
            "--actor-id",
            "agent:package",
            "--trust-domain",
            "LOCAL_USER",
            "--idempotency-key",
            "package:checkpoint",
        ],
        0,
    );
    let version = reconciled["result"]["new_state_version"].clone();
    let resume = selected(&["resume", "--format", "json"], 0);
    assert_eq!(resume["state_version"], version);
    assert!(resume["checkpoint_ref"].is_string());
    let md = cwd.join("continuidad á.md");
    selected(&["render", "--output", md.to_str().unwrap()], 0);
    selected(&["verify", "--artifact", md.to_str().unwrap()], 0);
    assert!(!fs::read_to_string(&md).unwrap().contains(process_canary));
    let portable = cwd.join("historia portable");
    selected(&["export", "--output", portable.to_str().unwrap()], 0);
    let imported = cwd.join("importado á.db");
    run(
        cwd,
        &[
            "import",
            "--file",
            portable.to_str().unwrap(),
            "--database",
            imported.to_str().unwrap(),
        ],
        0,
    );
    let imported_state = run(
        cwd,
        &[
            "resume",
            "--database",
            imported.to_str().unwrap(),
            "--project-ref",
            &id,
        ],
        0,
    );
    assert_eq!(imported_state, resume);
    let backup = cwd.join("respaldo privado");
    selected(&["backup", "--output", backup.to_str().unwrap()], 0);
    let restored = cwd.join("restaurado á.db");
    let restore = [
        "restore",
        "--file",
        backup.to_str().unwrap(),
        "--database",
        restored.to_str().unwrap(),
    ];
    run(cwd, &restore, 0);
    let before = fs::read(&restored).unwrap();
    run(cwd, &restore, 2);
    assert_eq!(before, fs::read(&restored).unwrap());
    let mut replay = checkpoint.to_vec();
    replay.extend([
        "--database",
        restored.to_str().unwrap(),
        "--project-ref",
        &id,
    ]);
    assert_eq!(run(cwd, &replay, 0)["status"], "DUPLICATE_REPLAY");
    let audit = run(
        cwd,
        &[
            "audit",
            "--database",
            restored.to_str().unwrap(),
            "--project-ref",
            &id,
        ],
        0,
    );
    assert_eq!(audit["findings"], json!([]));
    assert_eq!(audit["state_version"], version);
    let missing = cwd.join("missing.db");
    run(
        cwd,
        &[
            "resume",
            "--database",
            missing.to_str().unwrap(),
            "--project-ref",
            &id,
        ],
        4,
    );
    assert!(!missing.exists());
    let fork = cwd.join("fork.db");
    run(
        cwd,
        &[
            "import",
            "--mode",
            "fork",
            "--file",
            portable.to_str().unwrap(),
            "--database",
            fork.to_str().unwrap(),
        ],
        2,
    );
    assert!(!fork.exists());
    // Fixture secret enters only through a reviewed local record; shareable channels must refuse it.
    selected(
        &[
            "record",
            "note",
            "--kind",
            "LIMITATION",
            "--text",
            "token=FAKE_PACKAGE_EXPORT_CANARY",
            "--actor-id",
            "agent:package",
            "--observed-at",
            at,
            "--expected-version",
            &version.to_string(),
            "--idempotency-key",
            "package:private",
        ],
        0,
    );
    let private_export = cwd.join("must not publish");
    let rejected = selected(
        &["export", "--output", private_export.to_str().unwrap()],
        13,
    );
    assert!(!rejected.to_string().contains("FAKE_PACKAGE_EXPORT_CANARY"));
    assert!(!private_export.exists());
    let shareable = cwd.join("shareable.md");
    let rendered = selected(
        &[
            "render",
            "--privacy",
            "shareable",
            "--output",
            shareable.to_str().unwrap(),
        ],
        0,
    );
    assert!(!rendered.to_string().contains("FAKE_PACKAGE_EXPORT_CANARY"));
    assert!(
        !fs::read_to_string(shareable)
            .unwrap()
            .contains("FAKE_PACKAGE_EXPORT_CANARY")
    );
    let existing = cwd.join("existing work á");
    fs::create_dir(&existing).unwrap();
    let work = existing.join("work.md");
    fs::write(&work, b"preserved user work").unwrap();
    let bind_version = selected(&["status"], 0)["state_version"].to_string();
    selected(
        &[
            "repository",
            "bind",
            "--record-id",
            "binding:package",
            "--root",
            existing.to_str().unwrap(),
            "--actor-id",
            "human:owner",
            "--as-human",
            "--observed-at",
            at,
            "--expected-version",
            &bind_version,
            "--idempotency-key",
            "package:bind",
        ],
        0,
    );
    assert_eq!(fs::read(&work).unwrap(), b"preserved user work");
    assert_eq!(fs::read_dir(existing).unwrap().count(), 1);
}
