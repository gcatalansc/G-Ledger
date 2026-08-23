use std::{fs, path::PathBuf};

use jsonschema::Validator;
use project_ledger_interop_gskills::{
    GSKILLS_FIXTURE_MANIFEST_SHA256, GSKILLS_PROFILE, GSKILLS_REGISTRY_SHA256,
    GSKILLS_SPEC_REVISION, GSKILLS_SPEC_SHA256, LEDGER_SPEC_SHA256, RECIPROCAL_LOCK_SHA256,
    VENDORED_READ_CONTRACTS, verify_vendored_contracts,
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
        lock["compatibility_range"],
        serde_json::json!({
            "protocol": "gskills-context-provider",
            "minimum": "0.3.0",
            "maximum": "0.4.0"
        })
    );
    assert_eq!(lock["certified_compatibility"], false);

    let report = verify_vendored_contracts(&root).unwrap();
    assert_eq!(report.schema_count, 6);
    assert_eq!(report.positive_fixture_count, 6);
    assert_eq!(report.negative_fixture_count, 6);
    assert_eq!(report.schema_count, VENDORED_READ_CONTRACTS.len());
}

#[test]
fn exact_upstream_positive_and_negative_fixtures_are_executable() {
    let root = repository_root().join("integrations/gskills/contracts/v0.3");

    for contract in VENDORED_READ_CONTRACTS {
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
fn pl_10_upstream_schema_currently_rejects_normative_phase_and_action_fields() {
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

    assert!(!validator.is_valid(&normative));
    let diagnostics = validator
        .iter_errors(&normative)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(diagnostics.contains("/active_phases/0"));
    assert!(diagnostics.contains("/next_actions/ordered/0"));
    assert!(diagnostics.contains("Additional properties"));
}
