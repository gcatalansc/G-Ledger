use std::{fmt::Write as _, fs, path::Path};

use project_ledger_release::{ReleaseError, verify_release};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn ac_086_ac_088_release_gate_covers_all_ids_with_zero_waivers() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.acceptance_criteria_count, 90);
    assert_eq!(report.benchmark_scenario_count, 32);
    assert_eq!(report.missing_evidence_count, 0);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(report.waived_count, 0);
}

#[test]
fn phase_11_chaos_gate_reports_each_blocking_scope_without_conflation() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.chaos_waived_count, 2);
    assert_eq!(report.general_availability_blocker_count, 1);
    assert_eq!(report.gsp_runtime_certification_blocker_count, 1);
}

#[test]
fn phase_11_chaos_gate_rejects_an_unknown_hash_locked_scenario() {
    let directory = chaos_fixture();
    let path = directory.path().join("release/chaos-report.json");
    let mut report: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    report["covered"][0]["scenario"] = serde_json::json!("unknown-chaos-scenario");
    write_pretty_json(&path, &report);
    lock_chaos_digest(directory.path());

    let result = verify_release(directory.path());
    assert!(
        matches!(
            &result,
            Err(ReleaseError::Invalid { message })
                if message.contains("chaos scenario inventory")
        ),
        "unexpected chaos result: {result:?}"
    );
}

#[test]
fn phase_11_chaos_gate_rejects_missing_covered_evidence() {
    let directory = chaos_fixture();
    let path = directory.path().join("release/chaos-report.json");
    let mut report: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    report["covered"][0]["evidence"] =
        serde_json::json!("crates/storage-sqlite/tests/missing-chaos-evidence.rs");
    write_pretty_json(&path, &report);
    lock_chaos_digest(directory.path());

    let result = verify_release(directory.path());
    assert!(
        matches!(
            &result,
            Err(ReleaseError::Invalid { message })
                if message.contains("covered chaos evidence")
        ),
        "unexpected chaos result: {result:?}"
    );
}

#[test]
fn phase_11_chaos_report_cannot_be_removed_from_the_hash_locked_inventory() {
    let directory = chaos_fixture();
    let path = directory.path().join("release/release-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["artifact_inventory"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["path"] != "release/chaos-report.json");
    write_pretty_json(&path, &manifest);

    let result = verify_release(directory.path());
    assert!(
        matches!(&result, Err(ReleaseError::Invalid { message }) if message.contains("chaos report must be hash-locked")),
        "unexpected unlocked chaos result: {result:?}"
    );
}

#[test]
fn req_rel_002_migration_is_explicit_versioned_non_destructive_and_reversible() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.migration_source, "0.1.0");
    assert_eq!(report.migration_target, "0.2.0");
    assert!(report.migration_source_preserved);
    assert!(report.migration_reversible);
    assert!(!report.migration_in_place);
}

#[test]
fn ac_083_release_gate_scans_every_declared_surface_and_reports_zero_secrets() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.declared_privacy_surface_count, 7);
    assert_eq!(report.secret_persistence_count, 0);
}

#[test]
fn ac_090_compatibility_report_never_overclaims_pending_certification() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.compatibility_profile_count, 4);
    assert_eq!(report.certified_compatibility_count, 0);
    assert!(!report.overall_certified_compatibility);
}

#[test]
fn release_gate_rejects_one_byte_drift_before_accepting_report_semantics() {
    let directory = tempdir().unwrap();
    copy_tree(
        &workspace_root().join("release"),
        &directory.path().join("release"),
    );
    let surface = directory
        .path()
        .join("release/privacy/surfaces/structured-log.json");
    fs::write(&surface, b"{}\n ").unwrap();

    assert!(matches!(
        verify_release(directory.path()),
        Err(ReleaseError::ArtifactDigestMismatch { .. })
    ));
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn chaos_fixture() -> tempfile::TempDir {
    let directory = tempdir().unwrap();
    copy_tree(
        &workspace_root().join("release"),
        &directory.path().join("release"),
    );
    copy_file_into_fixture(
        directory.path(),
        "benchmark/project-ledger-v0.2.0-windows-x86_64.json",
    );
    copy_file_into_fixture(directory.path(), "docs/THREAT_MODEL.md");
    let chaos: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace_root().join("release/chaos-report.json")).unwrap(),
    )
    .unwrap();
    for covered in chaos["covered"].as_array().unwrap() {
        copy_file_into_fixture(directory.path(), covered["evidence"].as_str().unwrap());
    }
    directory
}

fn copy_file_into_fixture(root: &Path, relative: &str) {
    let destination = root.join(relative);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(workspace_root().join(relative), destination).unwrap();
}

fn lock_chaos_digest(root: &Path) {
    let chaos = fs::read(root.join("release/chaos-report.json")).unwrap();
    let digest =
        Sha256::digest(chaos)
            .iter()
            .fold(String::with_capacity(64), |mut output, byte| {
                write!(output, "{byte:02x}").unwrap();
                output
            });
    let path = root.join("release/release-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let entry = manifest["artifact_inventory"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["path"] == "release/chaos-report.json")
        .unwrap();
    entry["sha256"] = serde_json::Value::String(digest);
    write_pretty_json(&path, &manifest);
}

fn write_pretty_json(path: &Path, value: &serde_json::Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}
