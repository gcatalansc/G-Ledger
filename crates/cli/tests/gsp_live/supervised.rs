use super::{Fixture, cli, event_count, project_id, read_json, repository_root};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

fn policy(fixture: &Fixture, python: &Path, timeout_ms: u64) -> std::path::PathBuf {
    let mut environment = BTreeMap::new();
    // Explicit test policy, not ambient inheritance by the production supervisor.
    for name in ["PATH", "SystemRoot", "GSPGEMMA_UPSTREAM_ROOT"] {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name, value);
        }
    }
    let path = fixture.root.path().join("compile-policy.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({
            "allowed_programs":[python],
            "process":{"timeout_ms":timeout_ms,"environment":environment}
        }))
        .unwrap(),
    )
    .unwrap();
    path
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

fn compile(
    fixture: &Fixture,
    python: &Path,
    source: &Path,
    timeout_ms: u64,
) -> assert_cmd::Command {
    let mut cmd = cli();
    cmd.args(["integration", "gsp", "compile", "--python"])
        .arg(python)
        .arg("--source-root")
        .arg(source)
        .arg("--input")
        .arg(&fixture.corpus)
        .arg("--output")
        .arg(&fixture.validated)
        .arg("--cwd")
        .arg(fixture.root.path())
        .arg("--policy")
        .arg(policy(fixture, python, timeout_ms))
        .arg("--json");
    cmd
}

#[test]
fn supervised_entry_rejects_unauthorized_or_missing_runtime_without_writing() {
    let fixture = Fixture::new();
    export(&fixture);
    let python = std::env::current_exe().unwrap();
    let mut denied = compile(&fixture, &python, repository_root(), 60_000);
    fs::write(
        fixture.root.path().join("compile-policy.json"),
        b"{\"allowed_programs\":[]}",
    )
    .unwrap();
    denied.assert().code(9);
    compile(
        &fixture,
        &fixture.root.path().join("absent.exe"),
        repository_root(),
        60_000,
    )
    .assert()
    .code(12);
    assert!(!fixture.validated.exists());
    assert_eq!(event_count(&fixture.database), 7);
    fs::write(&fixture.validated, b"user-owned artifact").unwrap();
    compile(&fixture, &python, repository_root(), 60_000)
        .assert()
        .code(2);
    assert_eq!(
        fs::read(&fixture.validated).unwrap(),
        b"user-owned artifact"
    );
}

#[test]
fn gsp_result_reader_rejects_oversize_before_parsing_and_never_publishes() {
    let fixture = Fixture::new();
    export(&fixture);
    fs::write(&fixture.result, vec![b' '; 8_388_609]).unwrap();
    cli()
        .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
        .arg(&fixture.corpus)
        .arg("--result")
        .arg(&fixture.result)
        .arg("--output")
        .arg(&fixture.validated)
        .assert()
        .code(15);
    assert!(!fixture.validated.exists());
    assert_eq!(event_count(&fixture.database), 7);
}

#[test]
fn private_invalid_result_is_rejected_without_echoing_its_contents() {
    let fixture = Fixture::new();
    export(&fixture);
    let corpus = read_json(&fixture.corpus);
    let mut result = super::exact_result(&corpus);
    let canary = "FAKE_GSP_ERROR_CANARY";
    result["validation_report"]["status"] = json!(format!("token={canary}"));
    fs::write(&fixture.result, serde_json::to_vec(&result).unwrap()).unwrap();
    let assertion = cli()
        .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
        .arg(&fixture.corpus)
        .arg("--result")
        .arg(&fixture.result)
        .arg("--output")
        .arg(&fixture.validated)
        .arg("--json")
        .assert()
        .code(13);
    assert!(!String::from_utf8_lossy(&assertion.get_output().stdout).contains(canary));
    assert!(!String::from_utf8_lossy(&assertion.get_output().stderr).contains(canary));
    assert!(!fixture.validated.exists());
}

fn link(fixture: &Fixture, actor: &str, version: u64, key: &str) -> assert_cmd::Command {
    let mut cmd = cli();
    cmd.args(["integration", "gsp", "link-result", "--database"])
        .arg(&fixture.database)
        .args([
            "--project-ref",
            project_id().as_str(),
            "--receipt-uri",
            "artifact://gsp/supervised.json",
            "--expected-state-version",
            &version.to_string(),
            "--idempotency-key",
            key,
            "--actor-id",
            actor,
            "--observed-at",
            "2026-08-27T23:00:00Z",
            "--receipt",
        ])
        .arg(&fixture.validated)
        .arg("--json");
    cmd
}

#[test]
fn gsp_link_idempotency_is_actor_scoped_and_stale_rejections_stay_typed() {
    let fixture = Fixture::new();
    export(&fixture);
    let corpus = read_json(&fixture.corpus);
    let result = project_ledger_interop_gsp::GspBridge::load(repository_root())
        .unwrap()
        .validate_result(&corpus, super::exact_result(&corpus))
        .unwrap();
    fs::write(&fixture.validated, serde_json::to_vec(&result).unwrap()).unwrap();
    link(&fixture, "human:first", 7, "gsp:same-scope-key")
        .assert()
        .success();
    link(&fixture, "human:second", 7, "gsp:same-scope-key")
        .assert()
        .code(5);
    link(&fixture, "human:first", 7, "gsp:same-scope-key")
        .assert()
        .success()
        .stdout(predicates::str::contains("DUPLICATE_REPLAY"));
    assert_eq!(event_count(&fixture.database), 8);
}

fn runtime() -> (std::path::PathBuf, std::path::PathBuf) {
    (
        std::env::var_os("PROJECT_LEDGER_GSP_PYTHON")
            .expect("explicit pinned Python")
            .into(),
        std::env::var_os("PROJECT_LEDGER_GSP_SOURCE_ROOT")
            .expect("explicit pinned GSP source")
            .into(),
    )
}

fn semantic_fixture(text: &str) -> Fixture {
    use project_ledger_event_store::EventStore;
    let fixture = Fixture::new();
    cli()
        .args(["record", "progress", "--summary", text, "--database"])
        .arg(&fixture.database)
        .args([
            "--project-ref",
            project_id().as_str(),
            "--expected-version",
            "7",
            "--idempotency-key",
            "gsp:semantic-input",
            "--actor-id",
            "agent:fixture",
            "--observed-at",
            "2026-08-27T23:00:00Z",
        ])
        .assert()
        .success();
    let store =
        project_ledger_storage_sqlite::SqliteEventStore::open_read_only(&fixture.database).unwrap();
    let stream = store.load_stream(&project_id()).unwrap();
    let event = stream
        .iter()
        .rev()
        .find(|event| {
            matches!(
                event.event,
                project_ledger_domain::Event::WorkProgressRecorded(_)
            )
        })
        .unwrap();
    let mut request: super::GspCorpusRequest = read_json(&fixture.request);
    request.material_event_refs = vec![event.event_id.clone()];
    fs::write(&fixture.request, serde_json::to_vec(&request).unwrap()).unwrap();
    fixture
}

#[test]
fn private_selected_progress_is_rejected_before_gsp_export() {
    let canary = "FAKE_GSP_PRIVATE_CANARY";
    let fixture = semantic_fixture(&format!("token={canary}"));
    let before = event_count(&fixture.database);
    let assertion = cli()
        .args(["integration", "gsp", "export-corpus", "--database"])
        .arg(&fixture.database)
        .args(["--project-ref", project_id().as_str(), "--request"])
        .arg(&fixture.request)
        .arg("--output")
        .arg(&fixture.corpus)
        .arg("--json")
        .assert()
        .code(13);
    assert!(!String::from_utf8_lossy(&assertion.get_output().stdout).contains(canary));
    assert!(!String::from_utf8_lossy(&assertion.get_output().stderr).contains(canary));
    assert!(!fixture.corpus.exists());
    assert_eq!(event_count(&fixture.database), before);
}

#[test]
#[ignore = "requires exact frozen external GSP runtime; actual results then labeled adversarial alterations"]
fn actual_supervised_compiler_preserves_normal_conflict_and_uncertainty_results() {
    let (python, source) = runtime();
    for text in [
        "Context. Ledger must preserve receipts. End.",
        "Context. Ledger must preserve receipts. Ledger must not preserve receipts. End.",
    ] {
        let fixture = semantic_fixture(text);
        export(&fixture);
        let before = event_count(&fixture.database);
        let corpus_bytes = fs::read(&fixture.corpus).unwrap();
        compile(&fixture, &python, &source, 60_000)
            .assert()
            .success();
        assert_eq!(fs::read(&fixture.corpus).unwrap(), corpus_bytes);
        assert_eq!(event_count(&fixture.database), before);
        let output: Value = read_json(&fixture.validated);
        assert_eq!(output["report"]["exact_contract_count"], 6);
        assert_eq!(output["report"]["authority_elevation_count"], 0);
        assert_eq!(output["report"]["certified_compatibility"], false);
        if text.contains("must not") {
            assert!(
                !output["result"]["uncertainty_map"]["items"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                output["report"]["conflict_visibility"]["requires_human_review"],
                true
            );
        }
        let mut wrong_ref = output["result"].clone();
        wrong_ref["candidate_bundle"]["request_id"] = json!("00000000-0000-4000-8000-000000000000");
        let mut missing = output["result"].clone();
        missing.as_object_mut().unwrap().remove("uncertainty_map");
        for (index, bytes) in [
            b"{\"truncated\":".to_vec(),
            serde_json::to_vec(&wrong_ref).unwrap(),
            serde_json::to_vec(&missing).unwrap(),
        ]
        .into_iter()
        .enumerate()
        {
            fs::write(&fixture.result, bytes).unwrap();
            let rejected = fixture.root.path().join(format!("rejected-{index}.json"));
            cli()
                .args(["integration", "gsp", "validate-result", "--evidence-bundle"])
                .arg(&fixture.corpus)
                .arg("--result")
                .arg(&fixture.result)
                .arg("--output")
                .arg(&rejected)
                .assert()
                .failure();
            assert!(!rejected.exists());
            assert_eq!(event_count(&fixture.database), before);
        }
        println!(
            "gsp-supervised: actual compiler; conflict={}; exact artifacts=6; adversarial variants rejected=3; event delta=0",
            text.contains("must not")
        );
        for expected in ["ACCEPTED", "DUPLICATE_REPLAY"] {
            link(
                &fixture,
                "human:runtime-reviewer",
                before,
                "gsp:runtime-link",
            )
            .assert()
            .success()
            .stdout(predicates::str::contains(expected));
            assert_eq!(event_count(&fixture.database), before + 1);
        }
        let state = super::projection(&fixture.database);
        assert!(state.state().verified_gate_refs.is_empty());
        assert!(state.state().completed_phase_refs.is_empty());
    }
}

#[test]
#[ignore = "requires exact frozen external GSP runtime; bounded process startup timeout and cancellation"]
fn actual_supervised_runtime_drift_timeout_and_cancellation_publish_nothing() {
    let (python, source) = runtime();
    let fixture = Fixture::new();
    export(&fixture);
    compile(&fixture, &python, repository_root(), 60_000)
        .assert()
        .failure();
    assert!(!fixture.validated.exists());
    compile(&fixture, &python, &source, 1)
        .assert()
        .code(12)
        .stdout(predicates::str::contains("EXECUTION_TIMEOUT"));
    assert!(!fixture.validated.exists());
    let cancel = fixture.root.path().join("cancel.marker");
    fs::write(&cancel, b"cancel").unwrap();
    compile(&fixture, &python, &source, 60_000)
        .arg("--cancel-file")
        .arg(cancel)
        .assert()
        .code(12)
        .stdout(predicates::str::contains("EXECUTION_CANCELLED"));
    assert!(!fixture.validated.exists());
    assert_eq!(event_count(&fixture.database), 7);
    println!(
        "gsp-supervised: runtime drift, startup timeout and prearmed cancellation; no published result; event delta=0"
    );
}

#[test]
#[ignore = "real pinned compiler entry with test-only synchronization; shared supervisor timeout/cancellation"]
fn actual_compile_entry_timeout_and_cancellation_are_reaped() {
    use project_ledger_evidence::supervisor::{
        CancelToken, ProcessPolicy, ProcessRequest, SupervisionError, supervise,
    };
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let (python, source) = runtime();
    let fixture = Fixture::new();
    export(&fixture);
    for fault in ["timeout", "cancel"] {
        let marker = fixture.root.path().join(format!("{fault}-compile.marker"));
        let policy_value: Value = read_json(&policy(&fixture, &python, 15_000));
        let process: ProcessPolicy =
            serde_json::from_value(policy_value["process"].clone()).unwrap();
        let request = ProcessRequest {
            program: python.clone(),
            working_directory: fixture.root.path().to_owned(),
            args: vec![
                "-I".into(),
                "-B".into(),
                repository_root()
                    .join("crates/cli/tests/gsp_live/crash_probe.py")
                    .to_str()
                    .unwrap()
                    .into(),
                marker.to_str().unwrap().into(),
                repository_root()
                    .join("integrations/gsp/sidecar/compile_bundle.py")
                    .to_str()
                    .unwrap()
                    .into(),
                "--source-root".into(),
                source.to_str().unwrap().into(),
                "--input".into(),
                fixture.corpus.to_str().unwrap().into(),
                "--output".into(),
                fixture.result.to_str().unwrap().into(),
            ],
        };
        let cancel = CancelToken::default();
        let child_cancel = cancel.clone();
        let worker = thread::spawn(move || supervise(&process, &request, &child_cancel));
        let deadline = Instant::now() + Duration::from_secs(12);
        while !marker.exists() && !worker.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let entered = marker.exists();
        if fault == "cancel" || !entered {
            cancel.cancel();
        }
        let outcome = worker.join().unwrap();
        assert!(
            entered,
            "actual compile entry must precede the injected fault: {outcome:?}"
        );
        assert_eq!(
            fs::read_to_string(&marker).unwrap(),
            "GSP_COMPILE_ENTERED\n"
        );
        assert_eq!(
            outcome.unwrap_err(),
            if fault == "cancel" {
                SupervisionError::Cancelled
            } else {
                SupervisionError::Timeout
            }
        );
        assert!(!fixture.result.exists());
        assert_eq!(event_count(&fixture.database), 7);
        println!(
            "gsp-supervised: actual compiler entry observed; {fault}; family reaped; no result; event delta=0"
        );
    }
}
