use std::{fmt::Write as _, fs, path::Path, process::Command};

use project_ledger_release::verify_release;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::benchmark;

const PACKAGE_PATH: &str = "release/package-manifest.json";
const SBOM_PATH: &str = "release/sbom.spdx.json";
const CHECKSUMS_PATH: &str = "release/checksums.sha256";

pub fn generate(root: &Path, check_only: bool) -> Result<String, String> {
    let package = package_manifest()?;
    let sbom = sbom(root)?;
    let expected = [(PACKAGE_PATH, package), (SBOM_PATH, sbom)];
    if check_only {
        for (path, bytes) in &expected {
            let actual = fs::read(root.join(path)).map_err(|error| error.to_string())?;
            if actual != *bytes {
                return Err(format!("generated release artifact drift: {path}"));
            }
        }
        let expected_manifest = locked_manifest(root)?;
        let actual_manifest = fs::read(root.join("release/release-manifest.json"))
            .map_err(|error| error.to_string())?;
        if actual_manifest != expected_manifest {
            return Err("generated release manifest digest drift".to_owned());
        }
        let expected_checksums = checksums(root)?;
        let actual_checksums =
            fs::read(root.join(CHECKSUMS_PATH)).map_err(|error| error.to_string())?;
        if actual_checksums != expected_checksums {
            return Err("generated release checksum drift".to_owned());
        }
        return Ok("release artifacts are reproducible".to_owned());
    }
    for (path, bytes) in expected {
        fs::write(root.join(path), bytes).map_err(|error| error.to_string())?;
    }
    fs::write(
        root.join("release/release-manifest.json"),
        locked_manifest(root)?,
    )
    .map_err(|error| error.to_string())?;
    fs::write(root.join(CHECKSUMS_PATH), checksums(root)?).map_err(|error| error.to_string())?;
    Ok("release artifacts generated".to_owned())
}

fn locked_manifest(root: &Path) -> Result<Vec<u8>, String> {
    let path = root.join("release/release-manifest.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let inventory = manifest["artifact_inventory"]
        .as_array_mut()
        .ok_or_else(|| "release artifact inventory missing".to_owned())?;
    for entry in inventory {
        let artifact_path = entry["path"]
            .as_str()
            .ok_or_else(|| "release artifact path missing".to_owned())?;
        let bytes = fs::read(root.join(artifact_path)).map_err(|error| error.to_string())?;
        entry["sha256"] = Value::String(digest(&bytes));
    }
    pretty_json(&manifest)
}

pub fn check(root: &Path) -> Result<String, String> {
    generate(root, true)?;
    let benchmark = benchmark::check(root)?;
    let verified = verify_release(root).map_err(|error| error.to_string())?;
    if verified.blocked_count != 0
        || verified.missing_evidence_count != 0
        || verified.secret_persistence_count != 0
        || verified.certified_compatibility_count != 0
    {
        return Err(
            "aggregate release evidence contains a blocker, gap, secret or overclaim".to_owned(),
        );
    }
    Ok(format!(
        "release verified: {} AC, {} PL, {} secrets; {benchmark}",
        verified.acceptance_criteria_count,
        verified.benchmark_scenario_count,
        verified.secret_persistence_count
    ))
}

fn package_manifest() -> Result<Vec<u8>, String> {
    pretty_json(&json!({
        "profile": "project-ledger-package/0.2.0",
        "default_profile": {
            "binary": "project-ledger",
            "cargo_package": "project-ledger-cli",
            "single_binary": true,
            "daemon": false,
            "network_default": false
        },
        "targets": ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-apple-darwin", "x86_64-apple-darwin"],
        "build": "cargo build --locked --release -p project-ledger-cli",
        "verification": "cargo xtask release check",
        "windows_distribution": {
            "format": "ZIP",
            "build_and_verify": "pwsh -File scripts/build-windows-package.ps1 -Output <new.zip> -RequireClean",
            "contents": [
                "START_HERE.md",
                "project-ledger.exe",
                "curated documentation",
                "operational acceptance evidence",
                "SBOM.spdx.json",
                "CHECKSUMS.sha256",
                "package.json",
                "verified runtime resources",
                "license, security, changelog and release notes"
            ],
            "resources": "binary-locked resources/; explicit --resources override; never cwd discovery",
            "crt": "static",
            "core_requires_toolchain": false,
            "core_requires_python": false,
            "documentation_entrypoint": "START_HERE.md",
            "operational_acceptance": "OPERATIONALLY_ACCEPTED",
            "release_status": "ENGINEERING_RELEASE_CANDIDATE",
            "certified_compatibility": false,
            "optional_gsp": "explicit pinned external environment, no automatic downloads"
        }
    }))
}

fn sbom(root: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--locked", "--offline", "--format-version", "1"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let mut packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata packages missing".to_owned())?
        .iter()
        .map(|package| {
            let name = package["name"].as_str().unwrap_or("unknown");
            let version = package["version"].as_str().unwrap_or("unknown");
            let license = package["license"].as_str().unwrap_or("NOASSERTION");
            let source = package["source"].as_str().unwrap_or("NOASSERTION");
            json!({
                "SPDXID": format!("SPDXRef-Package-{}-{}", sanitize(name), sanitize(version)),
                "name": name,
                "versionInfo": version,
                "downloadLocation": source,
                "filesAnalyzed": false,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": license,
                "copyrightText": "NOASSERTION",
                "externalRefs": [{
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": format!("pkg:cargo/{name}@{version}")
                }]
            })
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left["SPDXID"].as_str().cmp(&right["SPDXID"].as_str()));
    let lock_digest =
        digest(&fs::read(root.join("Cargo.lock")).map_err(|error| error.to_string())?);
    pretty_json(&json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "project-ledger-v0.2.0",
        "documentNamespace": format!("https://project-ledger.invalid/spdx/0.2.0/{lock_digest}"),
        "creationInfo": {"created":"2026-08-23T00:00:00Z","creators":["Tool: project-ledger-xtask-0.2.0"]},
        "packages": packages
    }))
}

fn checksums(root: &Path) -> Result<Vec<u8>, String> {
    let paths = [
        "Cargo.lock",
        "benchmark/project-ledger-v0.2.0-windows-x86_64.json",
        "docs/THREAT_MODEL.md",
        "release/acceptance-report.json",
        "release/chaos-report.json",
        "release/compatibility-report.json",
        "release/migrations/v0.1-to-v0.2.json",
        "release/package-manifest.json",
        "release/privacy/corpus-manifest.json",
        "release/release-manifest.json",
        "release/sbom.spdx.json",
        "schemas/v0.2/schema-registry.json",
        "specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md",
    ];
    let mut output = String::new();
    for path in paths {
        let bytes = fs::read(root.join(path)).map_err(|error| error.to_string())?;
        output.push_str(&digest(&bytes));
        output.push_str("  ");
        output.push_str(path);
        output.push('\n');
    }
    Ok(output.into_bytes())
}

fn digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        },
    )
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn pretty_json(value: &Value) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manifest_declares_the_official_bounded_delivery() {
        let bytes = package_manifest().expect("package manifest");
        let manifest: Value = serde_json::from_slice(&bytes).expect("valid JSON");
        let windows = &manifest["windows_distribution"];

        assert_eq!(windows["documentation_entrypoint"], "START_HERE.md");
        assert_eq!(windows["operational_acceptance"], "OPERATIONALLY_ACCEPTED");
        assert_eq!(windows["release_status"], "ENGINEERING_RELEASE_CANDIDATE");
        assert_eq!(windows["certified_compatibility"], false);
        let contents = windows["contents"].as_array().expect("delivery contents");
        for required in [
            "project-ledger.exe",
            "curated documentation",
            "operational acceptance evidence",
            "SBOM.spdx.json",
            "CHECKSUMS.sha256",
        ] {
            assert!(
                contents.iter().any(|entry| entry == required),
                "missing {required}"
            );
        }
    }
}
