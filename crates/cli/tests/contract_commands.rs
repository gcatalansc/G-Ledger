use std::{path::Path, process::Command};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use project_ledger_canonical_json::to_canonical_vec;
use project_ledger_integration_protocol::build_default_manifest;
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcErrorObject, JsonRpcHandler, JsonRpcRequest, JsonRpcResponse, decode_frame,
    dispatch_frame, encode_frame,
};
use serde_json::{Value, json};

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

#[derive(Debug)]
struct ManifestHandler;

impl JsonRpcHandler for ManifestHandler {
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject> {
        if method != "ledger.manifest" || params != &json!({}) {
            return Err(JsonRpcErrorObject {
                code: -32601,
                message: "method not found".to_owned(),
                data: None,
            });
        }
        Ok(serde_json::to_value(build_default_manifest(Vec::new()).unwrap()).unwrap())
    }
}

#[test]
fn ac_003_cli_json_and_stdio_manifest_are_semantically_equivalent() {
    let manifest = build_default_manifest(Vec::new()).unwrap();
    let mut expected_stdout = to_canonical_vec(&manifest).unwrap();
    expected_stdout.push(b'\n');
    let cli = Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "manifest"])
        .output()
        .unwrap();
    assert!(cli.status.success());
    assert_eq!(cli.stdout, expected_stdout);

    let limits = FrameLimits::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!("manifest:01"),
        method: "ledger.manifest".to_owned(),
        params: json!({}),
    };
    let response = dispatch_frame(
        &encode_frame(&request, limits).unwrap(),
        limits,
        &ManifestHandler,
    )
    .unwrap();
    let response = decode_frame::<JsonRpcResponse>(&response, limits).unwrap();
    assert_eq!(
        response.result,
        Some(serde_json::to_value(manifest).unwrap())
    );
}

#[test]
fn integration_compatibility_reports_all_local_profiles_complete_but_uncertified() {
    let output = Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "compatibility"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], "1.0.0");
    assert_eq!(value["certified_compatibility"], false);
    let integrations = value["integrations"].as_array().unwrap();
    assert_eq!(integrations.len(), 4);
    assert_eq!(integrations[0]["integration"], "gskills");
    assert_eq!(integrations[0]["status"], "COMPLETE");
    assert_eq!(integrations[1]["integration"], "gsp");
    assert_eq!(integrations[1]["status"], "COMPLETE");
    assert_eq!(integrations[2]["integration"], "project-radar");
    assert_eq!(integrations[2]["status"], "COMPLETE");
    assert_eq!(integrations[3]["integration"], "agent-skills-codex");
    assert_eq!(integrations[3]["status"], "COMPLETE");
    assert!(
        integrations
            .iter()
            .all(|item| item["certified_compatibility"] == false)
    );
}
