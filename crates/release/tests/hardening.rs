use std::path::Path;

use project_ledger_domain::TrustLevel;
use project_ledger_release::{
    ExactProcessPlan, PrivacyFindingKind, PrivacyScanner, ReleasePolicy,
    validate_surface_relative_path,
};
use serde_json::json;

#[test]
fn ac_081_untrusted_repository_has_no_adapter_write_authority_network_or_hook_capability() {
    let capabilities = ReleasePolicy::capabilities(TrustLevel::Untrusted);

    assert!(!capabilities.adapter_read);
    assert!(!capabilities.adapter_write);
    assert!(!capabilities.lifecycle_authority);
    assert!(!capabilities.network);
    assert!(!capabilities.credentials);
    assert!(!capabilities.executable_hooks);
}

#[test]
fn ac_082_process_plan_preserves_literal_argv_and_never_builds_a_shell_command() {
    let plan = ExactProcessPlan::new(
        "project-ledger",
        ["checkpoint", "--reason", "$(touch owned) & calc.exe"],
        Path::new("workspace"),
    )
    .unwrap();

    assert_eq!(plan.executable(), "project-ledger");
    assert_eq!(
        plan.arguments(),
        ["checkpoint", "--reason", "$(touch owned) & calc.exe"]
    );
    assert_eq!(plan.cwd(), Path::new("workspace"));
    assert!(!plan.uses_shell());
}

#[test]
fn ac_083_ac_085_scanner_rejects_secrets_transcripts_and_private_reasoning() {
    let scanner = PrivacyScanner;
    let findings = scanner.scan_json(
        "integration-payload",
        &json!({
            "token": "secret-value",
            "transcript": "private conversation",
            "reasoning_private": "hidden chain",
            "nested": {"password": "hunter2"}
        }),
    );

    assert_eq!(findings.len(), 4);
    assert!(
        findings
            .iter()
            .any(|finding| finding.kind == PrivacyFindingKind::Secret)
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.kind == PrivacyFindingKind::Transcript)
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.kind == PrivacyFindingKind::PrivateReasoning)
    );
}

#[test]
fn ac_083_redacted_values_and_disabled_private_channels_are_clean() {
    let scanner = PrivacyScanner;
    let findings = scanner.scan_json(
        "redacted-export",
        &json!({
            "token": "[REDACTED]",
            "transcript": false,
            "reasoning_private": null,
            "raw_terminal_output": false,
            "summary": "credential removed [REDACTED]"
        }),
    );

    assert!(findings.is_empty());
}

#[test]
fn ac_083_declared_surfaces_reject_absolute_parent_and_dot_paths() {
    for invalid in [
        "../secret.json",
        "nested/../../secret.json",
        "./surface.json",
        "",
    ] {
        assert!(validate_surface_relative_path(Path::new(invalid)).is_err());
    }
    assert!(validate_surface_relative_path(Path::new("exports/minimal.json")).is_ok());
}

#[test]
fn ac_083_byte_scanner_reports_supported_secret_patterns_without_echoing_values() {
    let findings = PrivacyScanner.scan_bytes(
        "crash-artifact",
        b"Authorization: Bearer abcdef123456\npostgres://user:pass@localhost/db",
    );

    assert_eq!(findings.len(), 2);
    let rendered = format!("{findings:?}");
    assert!(!rendered.contains("abcdef123456"));
    assert!(!rendered.contains("user:pass"));
}
