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

    assert_eq!(report.chaos_waived_count, 0);
    assert_eq!(report.general_availability_blocker_count, 0);
    assert_eq!(report.gsp_runtime_certification_blocker_count, 0);
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
fn live_sidecar_coverage_without_observed_runtime_evidence_is_rejected() {
    let directory = chaos_fixture();
    assert_eq!(
        verify_release(directory.path())
            .unwrap()
            .missing_evidence_count,
        0
    );
    let path = directory.path().join("release/chaos-report.json");
    let mut report: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    report["waived"] = serde_json::json!([]);
    report["status"] = serde_json::json!("PASS");
    report["covered"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["scenario"] != "live-sidecar-process-crash");
    report["covered"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "scenario": "live-sidecar-process-crash",
            "evidence": "crates/storage-sqlite/tests/recovery.rs"
        }));
    write_pretty_json(&path, &report);
    lock_chaos_digest(directory.path());

    let result = verify_release(directory.path());
    assert!(
        matches!(&result, Err(ReleaseError::Invalid { message }) if message.contains("live sidecar coverage")),
        "unqualified live coverage was not rejected: {result:?}"
    );
}

#[test]
fn live_sidecar_coverage_rejects_wrong_counterpart_fault_and_unbound_hosted_sha() {
    for (field, invalid_value) in [
        ("counterpart_commit", "unlocked-counterpart"),
        ("observed_fault", "fixture-error-return"),
        (
            "locator",
            "https://github.com/gcatalansc/G-Ledger/actions/runs/123/job/456#head_sha=unverified",
        ),
    ] {
        let directory = chaos_fixture();
        let path = directory.path().join("release/chaos-report.json");
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        report["waived"] = serde_json::json!([]);
        report["status"] = serde_json::json!("PASS");
        let covered = report["covered"].as_array_mut().unwrap();
        covered.retain(|entry| entry["scenario"] != "live-sidecar-process-crash");
        let mut probe = serde_json::json!({
            "scenario": "live-sidecar-process-crash",
            "evidence": "crates/storage-sqlite/tests/recovery.rs",
            "observed_fault": "PROCESS_TERMINATED_DURING_COMPILE",
            "counterpart_commit": "7a43559c02417641691dbb5f2a6e79c13d74d1a1",
            "locator": "https://github.com/gcatalansc/G-Ledger/actions/runs/123/job/456#head_sha=1111111111111111111111111111111111111111"
        });
        probe[field] = serde_json::json!(invalid_value);
        covered.push(probe);
        write_pretty_json(&path, &report);
        lock_chaos_digest(directory.path());
        let result = verify_release(directory.path());
        assert!(
            matches!(&result, Err(ReleaseError::Invalid { message }) if message.contains("live sidecar coverage")),
            "invalid {field} was accepted: {result:?}"
        );
    }
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
fn official_windows_delivery_cannot_omit_its_human_entrypoint_or_bounded_status() {
    for field in [
        "documentation_entrypoint",
        "operational_acceptance",
        "release_status",
        "certified_compatibility",
        "contents",
    ] {
        let directory = chaos_fixture();
        let path = directory.path().join("release/package-manifest.json");
        let mut package: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        package["windows_distribution"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        write_pretty_json(&path, &package);
        lock_artifact_digest(directory.path(), "release/package-manifest.json");

        let result = verify_release(directory.path());
        assert!(
            matches!(&result, Err(ReleaseError::Invalid { message }) if message.contains("official Windows delivery")),
            "missing {field} was accepted: {result:?}"
        );
    }
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
    let acceptance: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace_root().join("release/acceptance-report.json")).unwrap(),
    )
    .unwrap();
    for kind in ["criterion_groups", "scenario_groups"] {
        for group in acceptance[kind].as_array().unwrap() {
            for evidence in group["evidence"].as_array().unwrap() {
                copy_file_into_fixture(directory.path(), evidence["path"].as_str().unwrap());
            }
        }
    }
    let migration: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace_root().join("release/migrations/v0.1-to-v0.2.json")).unwrap(),
    )
    .unwrap();
    for path in migration["evidence"].as_array().unwrap() {
        copy_file_into_fixture(directory.path(), path.as_str().unwrap());
    }
    for line in fs::read_to_string(workspace_root().join("release/checksums.sha256"))
        .unwrap()
        .lines()
    {
        copy_file_into_fixture(directory.path(), line.split_once("  ").unwrap().1);
    }
    directory
}

fn copy_file_into_fixture(root: &Path, relative: &str) {
    let destination = root.join(relative);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(workspace_root().join(relative), destination).unwrap();
}

fn lock_chaos_digest(root: &Path) {
    lock_artifact_digest(root, "release/chaos-report.json");
}

fn lock_artifact_digest(root: &Path, relative_path: &str) {
    let artifact = fs::read(root.join(relative_path)).unwrap();
    let digest =
        Sha256::digest(artifact)
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
        .find(|entry| entry["path"] == relative_path)
        .unwrap();
    entry["sha256"] = serde_json::Value::String(digest);
    write_pretty_json(&path, &manifest);
    let checksum_path = root.join("release/checksums.sha256");
    let checksums = fs::read_to_string(&checksum_path).unwrap();
    let mut relocked = String::new();
    for line in checksums.lines() {
        let (_, relative) = line.split_once("  ").unwrap();
        let digest = Sha256::digest(fs::read(root.join(relative)).unwrap());
        for byte in digest {
            write!(relocked, "{byte:02x}").unwrap();
        }
        writeln!(relocked, "  {relative}").unwrap();
    }
    fs::write(checksum_path, relocked).unwrap();
}

fn write_pretty_json(path: &Path, value: &serde_json::Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}
