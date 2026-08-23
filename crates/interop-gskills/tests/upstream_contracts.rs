use std::{fs, path::PathBuf};

use jsonschema::Validator;
use project_ledger_canonical_json::sha256_bytes;
use project_ledger_interop_gskills::{
    GSKILLS_FIXTURE_MANIFEST_SHA256, GSKILLS_PROFILE, GSKILLS_REGISTRY_SHA256,
    GSKILLS_SPEC_REVISION, GSKILLS_SPEC_SHA256, LEDGER_SPEC_SHA256, RECIPROCAL_LOCK_SHA256,
    VENDORED_GSKILLS_CONTRACTS, VENDORED_READ_CONTRACTS, verify_vendored_contracts,
};
use serde_json::Value;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn ac_042_exact_v0_3_0_profile_validates_against_upstream_lock() {
    let root = repository_root();
    let lock: Value = serde_json::from_slice(
        &fs::read(root.join("integrations/gskills/upstream-lock.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(lock["profile"], GSKILLS_PROFILE);
    assert_eq!(lock["document_revision"], GSKILLS_SPEC_REVISION);
    assert_eq!(lock["source_spec_sha256"], GSKILLS_SPEC_SHA256);
    assert_eq!(lock["source_provider_lock_sha256"], RECIPROCAL_LOCK_SHA256);
    assert_eq!(lock["schema_registry_sha256"], GSKILLS_REGISTRY_SHA256);
    assert_eq!(
        lock["fixture_manifest_sha256"],
        GSKILLS_FIXTURE_MANIFEST_SHA256
    );
    assert_eq!(lock["current_ledger_spec_sha256"], LEDGER_SPEC_SHA256);
    assert_eq!(lock["reciprocal_lock_status"], "CURRENT");
    assert_eq!(
        GSKILLS_REGISTRY_SHA256,
        "a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9"
    );
    assert_eq!(
        VENDORED_READ_CONTRACTS
            .iter()
            .find(|contract| contract.name == "project-continuity-projection")
            .unwrap()
            .schema_digest,
        "sha256:99c2651ac831c41b3918639ad53117016e83e8baec06c80699901fde43fe6ee1"
    );
    assert_eq!(
        lock["compatibility_range"],
        serde_json::json!({
            "protocol": "gskills-context-provider",
            "minimum": "0.3.0",
            "maximum": "0.4.0"
        })
    );
    assert_eq!(lock["certified_compatibility"], false);
    assert_eq!(lock["provider_field_mapping_status"], "EXACT");
    assert_eq!(lock["provider_loss_report_status"], "ZERO_MATERIAL_LOSS");
    assert_eq!(lock["provider_conformance_status"], "PASS");
    for (path, digest_field) in [
        (
            "integrations/gskills/reports/provider-field-mapping.json",
            "provider_field_mapping_report_sha256",
        ),
        (
            "integrations/gskills/reports/provider-loss-report.json",
            "provider_loss_report_sha256",
        ),
        (
            "integrations/gskills/reports/provider-conformance.json",
            "provider_conformance_report_sha256",
        ),
    ] {
        assert_eq!(
            sha256_bytes(&fs::read(root.join(path)).unwrap()).to_string(),
            format!("sha256:{}", lock[digest_field].as_str().unwrap())
        );
    }

    let expected_s10 = [
        (
            "external-record-command",
            "sha256:8b6319d8c1b9bb9b99ec3a7450b3ec6456f308ea66ce70e00821d95a82a57dd3",
        ),
        (
            "external-record-result",
            "sha256:54daeed672b1b6abeec19746ceb0ac4639c9ee092ca2ca43d3992a2b0dfd3da3",
        ),
        (
            "external-submission-record",
            "sha256:48f8a00cbd0e93ef7ed01f1326e7a3cfd2e7a578319aacbacdeba99716f3a584",
        ),
        (
            "gate-revalidation-signal",
            "sha256:c8d80e7d14f1008ace8f82152a9484aa707c1b6fa41a474e5a7fed6985a4695c",
        ),
        (
            "cross-system-reference",
            "sha256:6054bd16e82a7471dc899a36e7fe2a451cf31980d752007500d2e768f7705667",
        ),
    ];
    for (name, digest) in expected_s10 {
        assert_eq!(
            VENDORED_GSKILLS_CONTRACTS
                .iter()
                .find(|contract| contract.name == name)
                .unwrap()
                .schema_digest,
            digest
        );
    }

    let report = verify_vendored_contracts(&root).unwrap();
    assert_eq!(report.schema_count, 11);
    assert_eq!(report.positive_fixture_count, 11);
    assert_eq!(report.negative_fixture_count, 11);
    assert_eq!(report.schema_count, VENDORED_GSKILLS_CONTRACTS.len());
}

#[test]
fn exact_upstream_positive_and_negative_fixtures_are_executable() {
    let root = repository_root().join("integrations/gskills/contracts/v0.3");

    for contract in VENDORED_GSKILLS_CONTRACTS {
        let schema: Value =
            serde_json::from_slice(&fs::read(root.join(contract.schema)).unwrap()).unwrap();
        assert_eq!(schema["$id"], contract.schema_id);
        assert_eq!(schema["additionalProperties"], false);
        let validator = Validator::new(&schema).unwrap();
        let positive: Value =
            serde_json::from_slice(&fs::read(root.join(contract.positive_fixture)).unwrap())
                .unwrap();
        let negative: Value =
            serde_json::from_slice(&fs::read(root.join(contract.negative_fixture)).unwrap())
                .unwrap();

        assert!(validator.is_valid(&positive), "{} positive", contract.name);
        assert!(!validator.is_valid(&negative), "{} negative", contract.name);
    }
}

#[test]
fn pl_10_corrected_upstream_schema_accepts_normative_phase_and_action_fields() {
    let root = repository_root().join("integrations/gskills/contracts/v0.3");
    let schema: Value = serde_json::from_slice(
        &fs::read(root.join("project-continuity-projection.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = Validator::new(&schema).unwrap();
    let mut normative: Value = serde_json::from_slice(
        &fs::read(root.join("fixtures/positive/project-continuity-projection.json")).unwrap(),
    )
    .unwrap();
    normative["active_phases"] = serde_json::json!([{
        "ref": "phase:implementation",
        "title": "Implementation",
        "kind": "IMPLEMENTATION",
        "status": "ACTIVE",
        "started_at": "2026-08-23T00:00:00Z"
    }]);
    normative["next_actions"] = serde_json::json!({
        "ordered": [{
            "ref": "action:implement-provider",
            "title": "Implement provider",
            "classification": "AUTHORIZED",
            "status": "AUTHORIZED",
            "authority_ref": "roadmap:main",
            "gskills_derivation": null
        }],
        "preferred_ref": "action:implement-provider"
    });

    let diagnostics = validator
        .iter_errors(&normative)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        validator.is_valid(&normative),
        "corrected normative projection was rejected:\n{diagnostics}"
    );

    let mut undeclared = normative.clone();
    undeclared["active_phases"][0]["unexpected_provider_field"] = serde_json::json!(true);
    assert!(!validator.is_valid(&undeclared));

    let mut rewritten_taxonomy = normative.clone();
    rewritten_taxonomy["next_actions"]["ordered"][0]["classification"] =
        serde_json::json!("RANKED_BY_GSKILLS");
    assert!(!validator.is_valid(&rewritten_taxonomy));

    let mut unauthorized = normative;
    unauthorized["next_actions"]["ordered"][0]["classification"] = serde_json::json!("AUTHORIZED");
    unauthorized["next_actions"]["ordered"][0]["authority_ref"] = Value::Null;
    assert!(!validator.is_valid(&unauthorized));
}
