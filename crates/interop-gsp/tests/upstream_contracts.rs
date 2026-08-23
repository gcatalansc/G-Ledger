use std::{fs, path::Path};

use project_ledger_interop_gsp::{
    GSP_CONTRACT_COMMIT, GSP_CONTRACT_TAG, GSP_FIXTURE_MANIFEST_SHA256, GSP_IMPLEMENTATION_COMMIT,
    GSP_IMPLEMENTATION_SPEC_SHA256, GSP_IMPLEMENTATION_VERSION, GSP_SCHEMA_REGISTRY_SHA256,
    GspContractError, verify_vendored_contracts,
};

fn repository_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn exact_upstream_surface_and_all_fixture_outcomes_are_locked() {
    assert_eq!(GSP_IMPLEMENTATION_VERSION, "0.2.0");
    assert_eq!(
        GSP_IMPLEMENTATION_COMMIT,
        "7a43559c02417641691dbb5f2a6e79c13d74d1a1"
    );
    assert_eq!(
        GSP_IMPLEMENTATION_SPEC_SHA256,
        "c838bdd8eee3fb6648db5e7dd757d08d8ced81d35be8b9bf8001e321f49ffea1"
    );
    assert_eq!(GSP_CONTRACT_TAG, "gsp-exact-2026.04.12");
    assert_eq!(
        GSP_CONTRACT_COMMIT,
        "004b14d7b64837025671970eac2469f092c49475"
    );
    assert_eq!(
        GSP_SCHEMA_REGISTRY_SHA256,
        "523faabcc2e120a847849b582c9dbb97d758a6c34f78b96e153b7b67a622d7b3"
    );
    assert_eq!(
        GSP_FIXTURE_MANIFEST_SHA256,
        "c6ca0f6df33bda69e7e73eb201058884a3111bce506c9328aca5c421836e2e8c"
    );

    let report = verify_vendored_contracts(&repository_root()).expect("exact GSP contracts");
    assert_eq!(report.registry_contract_count, 28);
    assert_eq!(report.schema_artifact_count, 31);
    assert_eq!(report.valid_fixture_count, 56);
    assert_eq!(report.invalid_fixture_count, 84);
}

#[test]
fn one_changed_upstream_byte_is_rejected_before_fixture_replay() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let source = repository_root().join("integrations/gsp/contracts/gsp-exact-2026.04.12");
    let target = temporary
        .path()
        .join("integrations/gsp/contracts/gsp-exact-2026.04.12");
    copy_tree(&source, &target);
    let schema = target.join("schemas/evidence.bundle.schema.json");
    let mut bytes = fs::read(&schema).expect("evidence schema");
    bytes.push(b' ');
    fs::write(&schema, bytes).expect("mutated evidence schema");

    let error = verify_vendored_contracts(temporary.path()).expect_err("digest drift must fail");
    assert!(matches!(
        error,
        GspContractError::RawDigestMismatch { ref path, .. }
            if path == "core/gsp/schemas/evidence.bundle.schema.json"
    ));
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("target directory");
    for entry in fs::read_dir(source).expect("source directory") {
        let entry = entry.expect("source entry");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).expect("copy fixture");
        }
    }
}
