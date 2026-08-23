use std::path::{Path, PathBuf};

use project_ledger_canonical_json::{Sha256Digest, sha256_bytes, sha256_digest};
use project_ledger_domain::{LedgerTimestamp, ProjectId};
use project_ledger_interop_gsp::{GspCorpus, GspCorpusEntry, GspCorpusManifest};
use serde_json::{Value, json};

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn corpus() -> GspCorpus {
    let evidence_bundle = fixture_payload("evidence_bundle__valid_min.yaml");
    let query = evidence_bundle["normalized_query"].as_str().unwrap();
    let content = evidence_bundle["evidence_items"][0].clone();
    let entry = GspCorpusEntry {
        relative_path: "authority/authority_spec.json".to_owned(),
        source_ref: "authority:spec".to_owned(),
        source_kind: "AUTHORITY_DOCUMENT".to_owned(),
        title: "Authority".to_owned(),
        status: "CURRENT".to_owned(),
        content: content.clone(),
        content_digest: sha256_digest(&content).unwrap(),
    };
    let manifest = GspCorpusManifest {
        schema_version: "1.0.0".to_owned(),
        artifact_kind: "GSP_MINIMIZED_CORPUS_MANIFEST".to_owned(),
        profile: "GSP_MINIMIZED_CORPUS_V1".to_owned(),
        project_ref: project_id(),
        project_state_version: 8,
        material_state_version: 7,
        project_state_digest: Sha256Digest::from([7; 32]),
        query_digest: sha256_bytes(query.as_bytes()),
        observed_at: LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
        directories: [
            "authority",
            "decisions",
            "gates",
            "checkpoints",
            "selected-material-events",
            "sidecars",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect(),
        files: vec![entry],
        full_timeline_included: false,
        raw_evidence_output_included: false,
        workspace_paths_included: false,
        omitted_classes: vec!["FULL_EVENT_TIMELINE".to_owned()],
        corpus_digest: Sha256Digest::from([0; 32]),
    };
    let mut corpus = GspCorpus {
        manifest,
        evidence_bundle,
    };
    corpus.manifest.corpus_digest = sha256_digest(&corpus).unwrap();
    corpus
}

pub fn exact_result(corpus: &GspCorpus) -> Value {
    let request_id = corpus.evidence_bundle["request_id"].as_str().unwrap();
    let evidence_id = corpus.evidence_bundle["evidence_items"][0]["evidence_id"]
        .as_str()
        .unwrap();
    let mut candidate = fixture_payload("candidate_bundle__valid_min.yaml");
    let mut compilation = fixture_payload("compilation_bundle__valid_min.yaml");
    let mut report = fixture_payload("validation_report__valid_min.yaml");
    let mut uncertainty = fixture_payload("uncertainty_map__valid_min.yaml");
    let mut projection = fixture_payload("task_projection_bundle__valid_min.yaml");
    for value in [
        &mut candidate,
        &mut compilation,
        &mut report,
        &mut uncertainty,
        &mut projection,
    ] {
        replace_string(value, "request_id", request_id);
        replace_exact_string(value, "ev-001", evidence_id);
    }
    json!({
        "evidence_bundle": corpus.evidence_bundle,
        "candidate_bundle": candidate,
        "compilation_bundle": compilation,
        "validation_report": report,
        "uncertainty_map": uncertainty,
        "task_projection_bundle": projection,
        "validation": {
            "status": "pass",
            "authority": "gsp-spec@gsp-exact-2026.04.12",
            "contracts_checked": [
                "evidence.bundle",
                "candidate.bundle",
                "compilation.bundle",
                "validation.report",
                "uncertainty.map",
                "task.projection.bundle"
            ]
        }
    })
}

#[allow(dead_code)] // Shared integration support; only bridge.rs needs review-signal mutation.
pub fn with_review_signals(mut result: Value) -> Value {
    result["validation_report"]["status"] = json!("partial");
    result["validation_report"]["contradiction_clusters"] =
        json!([{"cluster_id":"conflict-1","reason":"sources disagree"}]);
    result
}

fn fixture_payload(file_name: &str) -> Value {
    let path = repository_root()
        .join("integrations/gsp/contracts/gsp-exact-2026.04.12/fixtures/contracts")
        .join(file_name);
    let document: Value = serde_yaml::from_slice(&std::fs::read(path).unwrap()).unwrap();
    document["payload"].clone()
}

fn replace_string(value: &mut Value, field: &str, replacement: &str) {
    match value {
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| replace_string(value, field, replacement)),
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == field {
                    *value = json!(replacement);
                } else {
                    replace_string(value, field, replacement);
                }
            }
        }
        _ => {}
    }
}

fn replace_exact_string(value: &mut Value, expected: &str, replacement: &str) {
    match value {
        Value::String(text) if text == expected => replacement.clone_into(text),
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| replace_exact_string(value, expected, replacement)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|value| replace_exact_string(value, expected, replacement)),
        _ => {}
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198cf00-0000-7000-8000-000000000021").unwrap()
}
