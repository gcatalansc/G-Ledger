use std::{collections::BTreeSet, fmt::Write as _, fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{PrivacyScanner, validate_surface_relative_path};

/// Aggregated, non-certifying result of the local release evidence gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the explicit release assertions returned to CLI consumers.
pub struct ReleaseVerification {
    pub acceptance_criteria_count: usize,
    pub benchmark_scenario_count: usize,
    pub missing_evidence_count: usize,
    pub blocked_count: usize,
    pub waived_count: usize,
    pub chaos_waived_count: usize,
    pub general_availability_blocker_count: usize,
    pub gsp_runtime_certification_blocker_count: usize,
    pub migration_source: String,
    pub migration_target: String,
    pub migration_source_preserved: bool,
    pub migration_reversible: bool,
    pub migration_in_place: bool,
    pub declared_privacy_surface_count: usize,
    pub secret_persistence_count: usize,
    pub compatibility_profile_count: usize,
    pub certified_compatibility_count: usize,
    pub overall_certified_compatibility: bool,
}

/// Fail-closed release manifest and evidence errors.
#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("release artifact could not be read at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("release JSON is invalid at {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("release path is unsafe: {path}")]
    UnsafePath { path: String },
    #[error("release artifact is a symbolic link: {path}")]
    SymbolicLink { path: String },
    #[error("release artifact digest mismatch at {path}")]
    ArtifactDigestMismatch { path: String },
    #[error("release evidence is invalid: {message}")]
    Invalid { message: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseManifest {
    profile: String,
    product_version: String,
    release_status: String,
    acceptance_report: String,
    migration_manifest: String,
    privacy_corpus_manifest: String,
    compatibility_report: String,
    artifact_inventory: Vec<ArtifactEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactEntry {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceReport {
    profile: String,
    criterion_groups: Vec<CriterionGroup>,
    scenario_groups: Vec<ScenarioGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CriterionGroup {
    ids: Vec<String>,
    status: AcceptanceStatus,
    evidence: Vec<EvidenceReference>,
    waiver: Option<Waiver>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum AcceptanceStatus {
    Pass,
    Waived,
    Blocked,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioGroup {
    ids: Vec<String>,
    evidence: Vec<EvidenceReference>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceReference {
    path: String,
    kind: String,
    locator: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Waiver {
    #[serde(rename = "waiver_id")]
    id: String,
    scope: String,
    reason: String,
    owner: String,
    review_by: String,
    non_critical: bool,
    blocks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChaosReport {
    profile: String,
    status: String,
    covered: Vec<CoveredChaosScenario>,
    waived: Vec<ChaosWaiver>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoveredChaosScenario {
    scenario: String,
    evidence: String,
    locator: Option<String>,
    observed_fault: Option<String>,
    counterpart_commit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChaosWaiver {
    scenario: String,
    reason: String,
    non_critical: bool,
    blocks: Vec<String>,
    review_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)] // Migration safety properties are independent contract assertions.
struct MigrationManifest {
    migration_id: String,
    source_contract_version: String,
    target_contract_version: String,
    strategy: String,
    in_place: bool,
    source_preserved: bool,
    reversible: bool,
    rollback: String,
    silent_reinterpretation: bool,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivacyCorpus {
    profile: String,
    scanner_profile: String,
    surfaces: Vec<PrivacySurface>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivacySurface {
    channel: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityReport {
    profile: String,
    overall_certified_compatibility: bool,
    profiles: Vec<CompatibilityProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityProfile {
    profile: String,
    local_status: String,
    certified_compatibility: bool,
    pending_evidence: Vec<String>,
}

/// Verifies all hash-locked release evidence rooted at `root` without network access.
///
/// # Errors
///
/// Returns a fail-closed error for unreadable, unsafe, drifted, incomplete, sensitive or
/// overclaiming release evidence.
pub fn verify_release(root: &Path) -> Result<ReleaseVerification, ReleaseError> {
    let manifest: ReleaseManifest = read_json(root, "release/release-manifest.json")?;
    if manifest.profile != "project-ledger-release/0.2.0"
        || manifest.product_version != "0.2.0"
        || manifest.release_status != "ENGINEERING_RELEASE_CANDIDATE"
    {
        return invalid("release identity or status is not the locked v0.2.0 candidate");
    }

    let mut inventory_paths = BTreeSet::new();
    for artifact in &manifest.artifact_inventory {
        if !inventory_paths.insert(artifact.path.as_str()) {
            return invalid("release artifact inventory contains a duplicate path");
        }
        verify_raw_artifact(root, &artifact.path, &artifact.sha256)?;
    }
    if !inventory_paths.contains("release/chaos-report.json") {
        return invalid("chaos report must be hash-locked in the release inventory");
    }

    let chaos: ChaosReport = read_json(root, "release/chaos-report.json")?;
    let (
        chaos_waived_count,
        general_availability_blocker_count,
        gsp_runtime_certification_blocker_count,
    ) = verify_chaos(root, &chaos)?;

    let acceptance: AcceptanceReport = read_json(root, &manifest.acceptance_report)?;
    if acceptance.profile != "project-ledger-acceptance-report/0.2.0" {
        return invalid("acceptance report profile mismatch");
    }
    let (acceptance_ids, waived_count, blocked_count, missing_evidence_count) =
        verify_acceptance(root, &acceptance)?;
    let scenario_ids = verify_scenarios(root, &acceptance.scenario_groups)?;

    let migration: MigrationManifest = read_json(root, &manifest.migration_manifest)?;
    verify_migration(root, &migration)?;

    let corpus: PrivacyCorpus = read_json(root, &manifest.privacy_corpus_manifest)?;
    let secret_persistence_count = verify_privacy_corpus(root, &corpus)?;

    let compatibility: CompatibilityReport = read_json(root, &manifest.compatibility_report)?;
    let certified_compatibility_count = verify_compatibility(&compatibility)?;
    verify_benchmark(root)?;
    verify_package(root)?;
    verify_sbom(root)?;
    verify_checksums(root)?;

    Ok(ReleaseVerification {
        acceptance_criteria_count: acceptance_ids.len(),
        benchmark_scenario_count: scenario_ids.len(),
        missing_evidence_count,
        blocked_count,
        waived_count,
        chaos_waived_count,
        general_availability_blocker_count,
        gsp_runtime_certification_blocker_count,
        migration_source: migration.source_contract_version,
        migration_target: migration.target_contract_version,
        migration_source_preserved: migration.source_preserved,
        migration_reversible: migration.reversible,
        migration_in_place: migration.in_place,
        declared_privacy_surface_count: corpus.surfaces.len(),
        secret_persistence_count,
        compatibility_profile_count: compatibility.profiles.len(),
        certified_compatibility_count,
        overall_certified_compatibility: compatibility.overall_certified_compatibility,
    })
}

fn verify_chaos(root: &Path, report: &ChaosReport) -> Result<(usize, usize, usize), ReleaseError> {
    if report.profile != "project-ledger-chaos-report/0.2.0"
        || (report.waived.is_empty() && report.status != "PASS")
        || (!report.waived.is_empty() && report.status != "PASS_WITH_DECLARED_SCOPE")
    {
        return invalid("chaos report identity or status is inconsistent with its waivers");
    }

    let expected = [
        "adapter-version-change",
        "authority-change",
        "concurrent-writers",
        "corrupted-projection-rebuild",
        "crash-append-rollback",
        "duplicate-and-stale-artifact",
        "live-sidecar-process-crash",
        "malformed-event-and-corrupt-stream",
        "markdown-edit",
        "physical-disk-full",
        "timeout-after-accept",
        "truncated-radar-file",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();

    for covered in &report.covered {
        if !observed.insert(covered.scenario.as_str()) {
            return invalid("chaos scenario inventory contains a duplicate");
        }
        checked_relative_path(&covered.evidence)?;
        let metadata = fs::symlink_metadata(root.join(&covered.evidence)).map_err(|_| {
            ReleaseError::Invalid {
                message: format!(
                    "covered chaos evidence does not exist: {}",
                    covered.evidence
                ),
            }
        })?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return invalid("covered chaos evidence must be a regular non-symlink file");
        }
        if covered.scenario == "physical-disk-full"
            && (covered.observed_fault.as_deref() != Some("ENOSPC(28)")
                || covered.locator.as_deref().is_none_or(|locator| {
                    !locator.starts_with("https://github.com/")
                        || !locator.contains("/actions/runs/")
                }))
        {
            return invalid("physical disk-full coverage requires hosted ENOSPC(28) evidence");
        }
        if covered.scenario == "live-sidecar-process-crash"
            && (covered.observed_fault.as_deref() != Some("PROCESS_TERMINATED_DURING_COMPILE")
                || covered.counterpart_commit.as_deref()
                    != Some("7a43559c02417641691dbb5f2a6e79c13d74d1a1")
                || covered
                    .locator
                    .as_deref()
                    .is_none_or(|locator| !hosted_runtime_locator(locator)))
        {
            return invalid(
                "live sidecar coverage requires hosted process-termination evidence and the exact counterpart commit",
            );
        }
    }

    let mut general_availability_blocker_count = 0;
    let mut gsp_runtime_certification_blocker_count = 0;
    for waiver in &report.waived {
        if !observed.insert(waiver.scenario.as_str()) {
            return invalid("chaos scenario inventory contains a duplicate");
        }
        if !waiver.non_critical
            || waiver.reason.trim().is_empty()
            || waiver.review_by.trim().is_empty()
            || waiver.blocks.is_empty()
        {
            return invalid("chaos waiver must be explicit, non-critical, reviewed and blocking");
        }
        let expected_block = if waiver.scenario == "live-sidecar-process-crash" {
            "GSP_RUNTIME_CERTIFICATION"
        } else {
            "GENERAL_AVAILABILITY"
        };
        if waiver.blocks.len() != 1 || waiver.blocks[0] != expected_block {
            return invalid("chaos waiver blocking scope is invalid for its scenario");
        }
        match expected_block {
            "GENERAL_AVAILABILITY" => general_availability_blocker_count += 1,
            "GSP_RUNTIME_CERTIFICATION" => gsp_runtime_certification_blocker_count += 1,
            _ => unreachable!("expected chaos blocking scopes are exhaustive"),
        }
    }

    if observed != expected {
        return invalid("chaos scenario inventory is incomplete or contains an unknown scenario");
    }
    Ok((
        report.waived.len(),
        general_availability_blocker_count,
        gsp_runtime_certification_blocker_count,
    ))
}

fn hosted_runtime_locator(locator: &str) -> bool {
    let Some((url, metadata)) = locator.split_once("#head_sha=") else {
        return false;
    };
    let sha = metadata.split('&').next().unwrap_or_default();
    let Some(path) = url.strip_prefix("https://github.com/gcatalansc/G-Ledger/actions/runs/")
    else {
        return false;
    };
    let Some((run, job)) = path.split_once("/job/") else {
        return false;
    };
    sha.len() == 40
        && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !run.is_empty()
        && !job.is_empty()
        && run.bytes().all(|byte| byte.is_ascii_digit())
        && job.bytes().all(|byte| byte.is_ascii_digit())
}

fn verify_benchmark(root: &Path) -> Result<(), ReleaseError> {
    let report: serde_json::Value =
        read_json(root, "benchmark/project-ledger-v0.2.0-windows-x86_64.json")?;
    let metrics = report["metrics"]
        .as_array()
        .ok_or_else(|| ReleaseError::Invalid {
            message: "benchmark metrics are missing".to_owned(),
        })?;
    let waivers = report["waivers"]
        .as_array()
        .ok_or_else(|| ReleaseError::Invalid {
            message: "benchmark waivers are missing".to_owned(),
        })?;
    let required_metrics = [
        "append_command",
        "cold_cli_startup",
        "gate_evaluation_simple",
        "idle_cpu",
        "memory_one_shot_p95",
        "network_calls_default",
        "projection_request_warm",
        "render_10k_events",
        "status_resume_warm",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let metric_names = metrics
        .iter()
        .filter_map(|metric| metric["name"].as_str())
        .collect::<BTreeSet<_>>();
    let memory = metrics
        .iter()
        .find(|metric| metric["name"] == "memory_one_shot_p95");
    if report["profile"] != "project-ledger-performance-benchmark/0.2.0"
        || report["measurement_status"] != "COMPLETE"
        || metrics.len() != 9
        || metric_names != required_metrics
        || metrics.iter().any(|metric| {
            metric["passed"] != true
                || metric["samples"] != 20
                || metric["method"].as_str().is_none_or(str::is_empty)
        })
        || !waivers.is_empty()
        || memory.is_none_or(|metric| {
            metric["unit"] != "MiB"
                || metric["budget"] != 100.0
                || metric["p95"]
                    .as_f64()
                    .is_none_or(|p95| p95 <= 0.0 || p95 >= 100.0)
        })
    {
        return invalid("benchmark is incomplete, over budget or has invalid memory evidence");
    }
    Ok(())
}

fn verify_package(root: &Path) -> Result<(), ReleaseError> {
    let package: serde_json::Value = read_json(root, "release/package-manifest.json")?;
    let windows = &package["windows_distribution"];
    let required_contents = [
        "project-ledger.exe",
        "curated documentation",
        "operational acceptance evidence",
        "SBOM.spdx.json",
        "CHECKSUMS.sha256",
    ];
    let complete_contents = windows["contents"].as_array().is_some_and(|contents| {
        required_contents
            .iter()
            .all(|required| contents.iter().any(|entry| entry == required))
    });
    if package["profile"] != "project-ledger-package/0.2.0"
        || package["default_profile"]["binary"] != "project-ledger"
        || package["default_profile"]["single_binary"] != true
        || package["default_profile"]["daemon"] != false
        || package["default_profile"]["network_default"] != false
        || package["targets"]
            .as_array()
            .is_none_or(|targets| targets.len() != 4)
    {
        return invalid("default package is not the locked offline single-binary profile");
    }
    if windows["documentation_entrypoint"] != "START_HERE.md"
        || windows["operational_acceptance"] != "OPERATIONALLY_ACCEPTED"
        || windows["release_status"] != "ENGINEERING_RELEASE_CANDIDATE"
        || windows["certified_compatibility"] != false
        || !complete_contents
    {
        return invalid("official Windows delivery identity or contents are incomplete");
    }
    Ok(())
}

fn verify_sbom(root: &Path) -> Result<(), ReleaseError> {
    let sbom: serde_json::Value = read_json(root, "release/sbom.spdx.json")?;
    if sbom["spdxVersion"] != "SPDX-2.3"
        || sbom["dataLicense"] != "CC0-1.0"
        || sbom["packages"].as_array().is_none_or(Vec::is_empty)
    {
        return invalid("SPDX SBOM identity or package inventory is incomplete");
    }
    Ok(())
}

fn verify_checksums(root: &Path) -> Result<(), ReleaseError> {
    let bytes = read_bytes(root, "release/checksums.sha256")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| ReleaseError::Invalid {
        message: "release checksums are not UTF-8".to_owned(),
    })?;
    let mut paths = BTreeSet::new();
    for line in text.lines() {
        let Some((expected, path)) = line.split_once("  ") else {
            return invalid("release checksum line is malformed");
        };
        if expected.len() != 64 || !paths.insert(path.to_owned()) {
            return invalid("release checksum digest or path is invalid");
        }
        verify_raw_artifact(root, path, expected)?;
    }
    if paths.len() != 13
        || !paths.contains("specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md")
        || !paths.contains("release/sbom.spdx.json")
        || !paths.contains("release/release-manifest.json")
    {
        return invalid("release checksum inventory is incomplete");
    }
    Ok(())
}

fn verify_acceptance(
    root: &Path,
    report: &AcceptanceReport,
) -> Result<(BTreeSet<String>, usize, usize, usize), ReleaseError> {
    let mut ids = BTreeSet::new();
    let mut waived = 0;
    let mut blocked = 0;
    let mut missing = 0;
    for group in &report.criterion_groups {
        if group.ids.is_empty() || group.evidence.is_empty() {
            return invalid("acceptance groups require IDs and evidence");
        }
        for id in &group.ids {
            if !ids.insert(id.clone()) {
                return invalid("acceptance report contains a duplicate AC ID");
            }
        }
        missing += verify_evidence(root, &group.evidence)?;
        match group.status {
            AcceptanceStatus::Pass if group.waiver.is_some() => {
                return invalid("PASS acceptance group cannot carry a waiver");
            }
            AcceptanceStatus::Pass => {}
            AcceptanceStatus::Blocked => {
                blocked += group.ids.len();
                if group.waiver.is_some() {
                    return invalid("BLOCKED acceptance group cannot be disguised as a waiver");
                }
            }
            AcceptanceStatus::Waived => {
                waived += group.ids.len();
                verify_waiver(group.waiver.as_ref())?;
            }
        }
    }
    if ids != expected_ids("AC", 90, 3) {
        return invalid("acceptance report must contain exactly AC-001 through AC-090");
    }
    Ok((ids, waived, blocked, missing))
}

fn verify_scenarios(
    root: &Path,
    groups: &[ScenarioGroup],
) -> Result<BTreeSet<String>, ReleaseError> {
    let mut ids = BTreeSet::new();
    for group in groups {
        if group.ids.is_empty() || group.evidence.is_empty() {
            return invalid("scenario groups require IDs and evidence");
        }
        for id in &group.ids {
            if !ids.insert(id.clone()) {
                return invalid("acceptance report contains a duplicate PL ID");
            }
        }
        if verify_evidence(root, &group.evidence)? != 0 {
            return invalid("benchmark scenarios contain missing evidence");
        }
    }
    if ids != expected_ids("PL", 32, 2) {
        return invalid("acceptance report must contain exactly PL-01 through PL-32");
    }
    Ok(ids)
}

fn verify_evidence(root: &Path, evidence: &[EvidenceReference]) -> Result<usize, ReleaseError> {
    let mut missing = 0;
    for reference in evidence {
        checked_relative_path(&reference.path)?;
        if reference.kind.trim().is_empty() || reference.locator.trim().is_empty() {
            return invalid("evidence kind and locator must be non-empty");
        }
        if !root.join(&reference.path).is_file() {
            missing += 1;
        }
    }
    Ok(missing)
}

fn verify_waiver(waiver: Option<&Waiver>) -> Result<(), ReleaseError> {
    let Some(waiver) = waiver else {
        return invalid("WAIVED acceptance group requires waiver metadata");
    };
    if !waiver.non_critical
        || waiver.id.trim().is_empty()
        || waiver.scope.trim().is_empty()
        || waiver.reason.trim().is_empty()
        || waiver.owner.trim().is_empty()
        || waiver.review_by.trim().is_empty()
        || waiver.blocks.is_empty()
    {
        return invalid("waiver must be explicit, non-critical, owned, reviewed and blocking");
    }
    Ok(())
}

fn verify_migration(root: &Path, migration: &MigrationManifest) -> Result<(), ReleaseError> {
    if migration.migration_id != "project-ledger-contracts-v0.1.0-to-v0.2.0"
        || migration.source_contract_version != "0.1.0"
        || migration.target_contract_version != "0.2.0"
        || migration.strategy != "REVIEWED_IMPORT_INTO_NEW_STORE"
        || migration.in_place
        || !migration.source_preserved
        || !migration.reversible
        || migration.silent_reinterpretation
        || migration.rollback.trim().is_empty()
        || migration.evidence.is_empty()
    {
        return invalid("v0.1 to v0.2 migration must be explicit, reviewed and reversible");
    }
    for path in &migration.evidence {
        checked_relative_path(path)?;
        if !root.join(path).is_file() {
            return invalid("migration evidence path does not exist");
        }
    }
    Ok(())
}

fn verify_privacy_corpus(root: &Path, corpus: &PrivacyCorpus) -> Result<usize, ReleaseError> {
    if corpus.profile != "project-ledger-privacy-corpus/0.2.0"
        || corpus.scanner_profile != "project-ledger-supported-secret-patterns/0.1.0"
    {
        return invalid("privacy corpus profile mismatch");
    }
    let expected_channels = [
        "CRASH_ARTIFACTS",
        "DB_WAL_SHM",
        "EXPORTS",
        "INTEGRATION_PAYLOADS",
        "LOGS",
        "MARKDOWN",
        "RADAR_ARTIFACTS",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    let channels = corpus
        .surfaces
        .iter()
        .map(|surface| surface.channel.clone())
        .collect::<BTreeSet<_>>();
    if channels != expected_channels || channels.len() != corpus.surfaces.len() {
        return invalid("privacy corpus must contain each required surface exactly once");
    }

    let scanner = PrivacyScanner;
    let mut count = 0;
    for surface in &corpus.surfaces {
        verify_raw_artifact(root, &surface.path, &surface.sha256)?;
        let bytes = read_bytes(root, &surface.path)?;
        count += scanner.scan_bytes(&surface.channel, &bytes).len();
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|source| ReleaseError::Json {
                path: surface.path.clone(),
                source,
            })?;
        count += scanner.scan_json(&surface.channel, &value).len();
    }
    if count != 0 {
        return invalid("secret persistence count is non-zero on the declared corpus");
    }
    Ok(count)
}

fn verify_compatibility(report: &CompatibilityReport) -> Result<usize, ReleaseError> {
    if report.profile != "project-ledger-compatibility-report/0.2.0"
        || report.overall_certified_compatibility
        || report.profiles.len() != 4
    {
        return invalid("compatibility report identity/count or global certification is invalid");
    }
    let mut names = BTreeSet::new();
    let certified = report
        .profiles
        .iter()
        .filter(|profile| profile.certified_compatibility)
        .count();
    for profile in &report.profiles {
        if !names.insert(profile.profile.as_str())
            || profile.local_status != "COMPLETE"
            || profile.pending_evidence.is_empty()
        {
            return invalid("compatibility profiles must be unique, locally complete and pending");
        }
    }
    if certified != 0 {
        return invalid("compatibility certification overclaim detected");
    }
    Ok(certified)
}

fn verify_raw_artifact(root: &Path, path: &str, expected: &str) -> Result<(), ReleaseError> {
    checked_relative_path(path)?;
    let full_path = root.join(path);
    let metadata = fs::symlink_metadata(&full_path).map_err(|source| ReleaseError::Io {
        path: path.to_owned(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(ReleaseError::SymbolicLink {
            path: path.to_owned(),
        });
    }
    let bytes = read_bytes(root, path)?;
    let actual = hex_digest(&bytes);
    if actual != expected {
        return Err(ReleaseError::ArtifactDigestMismatch {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(root: &Path, path: &str) -> Result<T, ReleaseError> {
    let bytes = read_bytes(root, path)?;
    serde_json::from_slice(&bytes).map_err(|source| ReleaseError::Json {
        path: path.to_owned(),
        source,
    })
}

fn read_bytes(root: &Path, path: &str) -> Result<Vec<u8>, ReleaseError> {
    checked_relative_path(path)?;
    fs::read(root.join(path)).map_err(|source| ReleaseError::Io {
        path: path.to_owned(),
        source,
    })
}

fn checked_relative_path(path: &str) -> Result<(), ReleaseError> {
    validate_surface_relative_path(Path::new(path)).map_err(|_| ReleaseError::UnsafePath {
        path: path.to_owned(),
    })
}

fn expected_ids(prefix: &str, count: usize, width: usize) -> BTreeSet<String> {
    (1..=count)
        .map(|index| format!("{prefix}-{index:0width$}"))
        .collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        },
    )
}

fn invalid<T>(message: &str) -> Result<T, ReleaseError> {
    Err(ReleaseError::Invalid {
        message: message.to_owned(),
    })
}
