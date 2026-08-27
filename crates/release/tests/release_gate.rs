use std::{fs, path::Path};

use project_ledger_release::{ReleaseError, verify_release};
use tempfile::tempdir;

#[test]
fn ac_088_release_gate_covers_exactly_ac_001_to_090_and_pl_01_to_32() {
    let report = verify_release(workspace_root()).unwrap();

    assert_eq!(report.acceptance_criteria_count, 90);
    assert_eq!(report.benchmark_scenario_count, 32);
    assert_eq!(report.missing_evidence_count, 0);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(report.waived_count, 1);
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
