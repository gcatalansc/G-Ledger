use std::{fs, path::PathBuf};

use project_ledger_interop_radar::{
    RADAR_PROFILE, RADAR_SOURCE_SPEC_SHA256, RadarContractCatalog, RadarContractError,
    verify_profile_contracts,
};
use serde_json::Value;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn ac_065_exact_profile_lock_and_all_fixtures_replay_offline() {
    let root = repository_root();
    let lock: Value = serde_json::from_slice(
        &fs::read(root.join("integrations/project-radar/upstream-lock.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(RADAR_PROFILE, "project-ledger-radar-artifact/0.1.1");
    assert_eq!(
        RADAR_SOURCE_SPEC_SHA256,
        "277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b"
    );
    assert_eq!(lock["profile"], RADAR_PROFILE);
    assert_eq!(lock["source_spec_sha256"], RADAR_SOURCE_SPEC_SHA256);

    let catalog = RadarContractCatalog::load(&root).unwrap();
    assert_eq!(catalog.contract_count(), 5);
    for contract in catalog.contracts() {
        let positive = catalog.read_positive_fixture(contract.name()).unwrap();
        catalog.validate(contract.name(), &positive).unwrap();
        let negative = catalog.read_negative_fixture(contract.name()).unwrap();
        assert!(catalog.validate(contract.name(), &negative).is_err());
    }

    let report = verify_profile_contracts(&root).unwrap();
    assert_eq!(report.schema_count, 5);
    assert_eq!(report.valid_fixture_count, 5);
    assert_eq!(report.invalid_fixture_count, 5);
    assert_eq!(
        lock["schema_registry_sha256"],
        report.schema_registry_sha256
    );
    assert_eq!(
        lock["fixture_manifest_sha256"],
        report.fixture_manifest_sha256
    );
    assert_eq!(lock["counterpart_extension_status"], "PENDING");
    assert_eq!(lock["contract_status"], "EXACT_LOCAL_PROFILE_PASS");
    assert_eq!(lock["reference_importer_status"], "PURE_FIXTURE_PASS");
    assert_eq!(lock["sqlite_cli_status"], "READ_ONLY_PASS");
    assert_eq!(lock["certified_compatibility"], false);
    for (path, field) in [
        (
            "integrations/project-radar/reports/radar-field-mapping.json",
            "field_mapping_report_sha256",
        ),
        (
            "integrations/project-radar/reports/radar-loss-report.json",
            "loss_report_sha256",
        ),
        (
            "integrations/project-radar/reports/radar-conformance.json",
            "conformance_report_sha256",
        ),
    ] {
        assert_eq!(
            project_ledger_canonical_json::sha256_bytes(&fs::read(root.join(path)).unwrap())
                .to_string(),
            format!("sha256:{}", lock[field].as_str().unwrap())
        );
    }
}

#[test]
fn raw_schema_drift_is_rejected_before_fixture_replay() {
    let source = repository_root();
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    copy_profile_surface(&source, target);
    let schema = target.join("schemas/v0.2/radar/ledger-radar-project-projection.schema.json");
    let mut bytes = fs::read(&schema).unwrap();
    bytes.push(b' ');
    fs::write(schema, bytes).unwrap();

    assert!(matches!(
        RadarContractCatalog::load(target),
        Err(RadarContractError::RawDigestMismatch { .. })
    ));
}

fn copy_profile_surface(source: &std::path::Path, target: &std::path::Path) {
    for relative in [
        "integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1/schema-registry.json",
        "integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1/fixture-manifest.json",
        "schemas/v0.2/radar/ledger-radar-project-projection.schema.json",
        "schemas/v0.2/radar/radar-field-ownership-policy.schema.json",
        "schemas/v0.2/radar/radar-project-link.schema.json",
        "schemas/v0.2/radar/radar-status-mapping-policy.schema.json",
        "schemas/v0.2/radar/radar-sync-envelope.schema.json",
        "schemas/v0.2/fixtures/positive/radar-ledger-radar-project-projection.json",
        "schemas/v0.2/fixtures/positive/radar-radar-field-ownership-policy.json",
        "schemas/v0.2/fixtures/positive/radar-radar-project-link.json",
        "schemas/v0.2/fixtures/positive/radar-radar-status-mapping-policy.json",
        "schemas/v0.2/fixtures/positive/radar-radar-sync-envelope.json",
        "schemas/v0.2/fixtures/negative/radar-ledger-radar-project-projection.json",
        "schemas/v0.2/fixtures/negative/radar-radar-field-ownership-policy.json",
        "schemas/v0.2/fixtures/negative/radar-radar-project-link.json",
        "schemas/v0.2/fixtures/negative/radar-radar-status-mapping-policy.json",
        "schemas/v0.2/fixtures/negative/radar-radar-sync-envelope.json",
    ] {
        let destination = target.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source.join(relative), destination).unwrap();
    }
}
