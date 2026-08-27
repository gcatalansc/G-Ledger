use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::{Value, json};

use super::{Fixture, cli, event_count, project_id, projection, read_json, repository_root};

struct Runtime {
    python: PathBuf,
    source: PathBuf,
}

impl Runtime {
    fn explicit() -> Self {
        Self {
            python: PathBuf::from(
                std::env::var_os("PROJECT_LEDGER_GSP_PYTHON")
                    .expect("live qualification requires an explicit Python interpreter"),
            ),
            source: PathBuf::from(
                std::env::var_os("PROJECT_LEDGER_GSP_SOURCE_ROOT")
                    .expect("live qualification requires the exact GSP source checkout"),
            ),
        }
    }

    fn command(&self, fixture: &Fixture) -> Command {
        assert!(self.python.is_absolute() && self.python.is_file());
        assert!(self.source.is_absolute() && self.source.is_dir());
        let mut command = Command::new(&self.python);
        command
            .args(["-I", "-B"])
            .arg(repository_root().join("integrations/gsp/sidecar/compile_bundle.py"))
            .arg("--source-root")
            .arg(&self.source)
            .arg("--input")
            .arg(&fixture.corpus)
            .arg("--output")
            .arg(&fixture.result)
            .current_dir(fixture.root.path())
            .stdin(Stdio::null());
        command
    }
}

struct OwnedChild(Child);

impl OwnedChild {
    fn wait_bounded(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "owned sidecar exceeded 30 seconds"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn export(fixture: &Fixture) {
    cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&fixture.corpus)
        .assert()
        .success();
}

fn validate(fixture: &Fixture) {
    cli()
        .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
        .arg(&fixture.corpus)
        .arg("--result")
        .arg(&fixture.result)
        .arg("--output")
        .arg(&fixture.validated)
        .assert()
        .success();
}

#[test]
#[ignore = "requires the exact external GSP checkout and its frozen Python environment"]
fn actual_compiler_emits_a_result_accepted_by_ledger_without_mutation() {
    let runtime = Runtime::explicit();
    let fixture = Fixture::new();
    export(&fixture);
    let mut child = OwnedChild(runtime.command(&fixture).spawn().unwrap());
    assert!(
        child.wait_bounded().success(),
        "actual compiler must succeed"
    );
    validate(&fixture);
    assert_eq!(event_count(&fixture.database), 7);
}

#[test]
#[ignore = "requires the exact external GSP checkout and its frozen Python environment"]
fn actual_compiler_rejects_wrong_source_bad_input_and_output_overwrite() {
    let runtime = Runtime::explicit();
    let fixture = Fixture::new();
    export(&fixture);
    let mut wrong_source = runtime.command(&fixture);
    wrong_source.arg("--source-root").arg(repository_root());
    rejected(wrong_source, "SOURCE_COMMIT_MISMATCH");
    assert!(!fixture.result.exists());
    let corpus = fs::read(&fixture.corpus).unwrap();
    let duplicate = [b"{\"evidence_bundle\":{},".as_slice(), &corpus[1..]].concat();
    for (invalid, code) in [
        (b"{invalid".to_vec(), "COMPILATION_FAILED"),
        (vec![b' '; 1_048_577], "INPUT_LIMIT"),
        (duplicate, "DUPLICATE_JSON_KEY"),
    ] {
        fs::write(&fixture.corpus, invalid).unwrap();
        rejected(runtime.command(&fixture), code);
        assert!(!fixture.result.exists());
    }
    fs::write(&fixture.corpus, corpus).unwrap();
    fs::write(&fixture.result, b"existing-user-artifact").unwrap();
    rejected(runtime.command(&fixture), "OUTPUT_EXISTS");
    assert_eq!(
        fs::read(&fixture.result).unwrap(),
        b"existing-user-artifact"
    );
    assert_eq!(event_count(&fixture.database), 7);
}

fn rejected(mut command: Command, code: &str) {
    command.stderr(Stdio::piped());
    let mut child = OwnedChild(command.spawn().unwrap());
    assert_eq!(child.wait_bounded().code(), Some(2));
    let mut error = String::new();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .take(1_024)
        .read_to_string(&mut error)
        .unwrap();
    assert_eq!(error.trim(), format!("GSP_SIDECAR_REJECTED:{code}"));
}

fn fingerprint(database: &Path) -> Value {
    let store = SqliteEventStore::open(database).unwrap();
    assert_eq!(store.quick_check().unwrap(), "ok");
    assert_eq!(store.journal_mode().unwrap().to_lowercase(), "wal");
    store.verify_event_chain(&project_id()).unwrap();
    let stream = store.load_stream(&project_id()).unwrap();
    json!({
        "event_digests": stream.iter().map(|event| event.event_digest).collect::<Vec<_>>(),
        "state": projection(database).state(),
    })
}

fn await_compile_marker(child: &mut OwnedChild, marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !marker.exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "sidecar exited before actual compile entry"
        );
        assert!(
            Instant::now() < deadline,
            "actual compile entry was not observed"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(marker).unwrap(), "GSP_COMPILE_ENTERED\n");
}

fn link_and_replay(fixture: &Fixture) {
    let validated: Value = read_json(&fixture.validated);
    assert_eq!(
        validated["receipt"]["artifacts"].as_array().unwrap().len(),
        6
    );
    assert_eq!(validated["report"]["authority_elevation_count"], 0);
    assert_eq!(validated["report"]["certified_compatibility"], false);
    fs::write(
        &fixture.receipt,
        serde_json::to_vec(&validated["receipt"]).unwrap(),
    )
    .unwrap();
    for expected_status in ["ACCEPTED", "DUPLICATE_REPLAY"] {
        cli()
            .args([
                "integration",
                "gsp",
                "link-result",
                "--project-ref",
                project_id().as_str(),
                "--receipt-uri",
                "artifact://gsp/live-qualification.json",
                "--expected-state-version",
                "7",
                "--idempotency-key",
                "gsp:live:qualification",
                "--actor-id",
                "actor:gsp-live",
                "--observed-at",
                "2026-08-27T12:00:00-04:00",
                "--database",
            ])
            .arg(&fixture.database)
            .arg("--receipt")
            .arg(&fixture.receipt)
            .assert()
            .success()
            .stdout(predicates::str::contains(expected_status));
        assert_eq!(event_count(&fixture.database), 8);
    }
    let state = projection(&fixture.database);
    assert_eq!(
        state.state().project.lifecycle_status,
        super::ProjectLifecycleStatus::Initializing
    );
    assert!(state.state().active_phase_refs.is_empty());
    assert!(state.state().verified_gate_refs.is_empty());
}

#[test]
#[ignore = "terminates only its own real GSP child; requires explicit frozen external runtime"]
fn real_process_crash_preserves_ledger_then_restart_links_once() {
    let runtime = Runtime::explicit();
    let fixture = Fixture::new();
    export(&fixture);
    let before = fingerprint(&fixture.database);
    let marker = fixture.root.path().join("compile-entered.marker");
    let mut crash = Command::new(&runtime.python);
    crash
        .args(["-I", "-B"])
        .arg(repository_root().join("crates/cli/tests/gsp_live/crash_probe.py"))
        .arg(&marker)
        .args(runtime.command(&fixture).get_args().skip(2))
        .stdin(Stdio::null())
        .current_dir(fixture.root.path());
    let mut child = OwnedChild(crash.spawn().unwrap());
    await_compile_marker(&mut child, &marker);
    assert!(child.0.try_wait().unwrap().is_none());
    println!(
        "gsp-live: entered actual compile_evidence in owned child pid={}",
        child.0.id()
    );
    child.0.kill().unwrap();
    assert!(!child.wait_bounded().success());
    assert!(!fixture.result.exists());
    assert_eq!(fingerprint(&fixture.database), before);
    println!(
        "gsp-live: PROCESS_TERMINATED_DURING_COMPILE; reaped; no result; event delta=0; chain/projection preserved"
    );
    let mut restarted = OwnedChild(runtime.command(&fixture).spawn().unwrap());
    assert!(restarted.wait_bounded().success());
    validate(&fixture);
    assert_eq!(fingerprint(&fixture.database), before);
    println!("gsp-live: fresh process restart; six exact artifacts validated; event delta=0");
    link_and_replay(&fixture);
    println!(
        "gsp-live: explicit link delta=1; replay delta=0; authority elevation=0; certified=false"
    );
}
