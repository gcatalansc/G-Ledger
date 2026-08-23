use std::{collections::BTreeSet, env, path::PathBuf};

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    EvidenceKind, EvidenceMetrics, EvidenceStatus, LedgerTimestamp, ProjectId, RepositorySnapshot,
    ResourceRef, StableId,
};
use project_ledger_evidence::{
    EvidenceCommand, EvidenceParser, EvidenceRunContext, EvidenceRunPolicy, EvidenceRunner,
    EvidenceRunnerError, ParsedEvidence,
};

#[test]
fn ac_018_evidence_run_captures_exit_duration_digest_without_raw_output() {
    let working_directory = env::current_dir().unwrap();
    let request = EvidenceCommand {
        program: "rustc".to_owned(),
        args: vec!["--version".to_owned()],
        command_display: "rustc --version --token secret-value".to_owned(),
        working_directory,
        working_directory_ref: ResourceRef::parse("workspace://project/").unwrap(),
        tool: "rustc".to_owned(),
    };
    let policy = EvidenceRunPolicy {
        allowed_programs: BTreeSet::from(["rustc".to_owned()]),
        redactions: vec!["secret-value".to_owned()],
        max_summary_bytes: 256,
    };
    let context = EvidenceRunContext {
        evidence_id: id("evidence:rustc-version"),
        project_ref: project_id(),
        kind: EvidenceKind::BuildResult,
        repository_binding: Some(snapshot()),
        observed_at: timestamp(),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/rustc-version")
            .unwrap(),
        runner_version: "project-ledger/0.2.0".to_owned(),
    };

    let record = EvidenceRunner::run(&policy, &request, &context, &RustcParser).unwrap();

    assert_eq!(record.result.exit_code, Some(0));
    assert_eq!(record.result.metrics.passed, Some(1));
    assert!(record.result.summary.contains("rustc"));
    assert!(!record.result.summary.contains("secret-value"));
    assert_eq!(record.status, EvidenceStatus::VerifiedSource);
    assert!(!record.privacy.raw_output_stored);
    assert!(record.privacy.redaction_applied);
    assert_eq!(
        record.source.command_display.as_deref(),
        Some("rustc --version --token [REDACTED]")
    );
    assert_eq!(evidence_digest(&record), record.payload_digest);
}

#[test]
fn runner_rejects_non_allowlisted_program_before_process_creation() {
    let request = EvidenceCommand {
        program: "definitely-not-allowed".to_owned(),
        args: Vec::new(),
        command_display: "definitely-not-allowed".to_owned(),
        working_directory: PathBuf::from("."),
        working_directory_ref: ResourceRef::parse("workspace://project/").unwrap(),
        tool: "unknown".to_owned(),
    };
    let policy = EvidenceRunPolicy {
        allowed_programs: BTreeSet::from(["rustc".to_owned()]),
        redactions: Vec::new(),
        max_summary_bytes: 256,
    };
    let context = EvidenceRunContext {
        evidence_id: id("evidence:denied"),
        project_ref: project_id(),
        kind: EvidenceKind::BuildResult,
        repository_binding: Some(snapshot()),
        observed_at: timestamp(),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/denied").unwrap(),
        runner_version: "project-ledger/0.2.0".to_owned(),
    };

    assert!(matches!(
        EvidenceRunner::run(&policy, &request, &context, &RustcParser),
        Err(EvidenceRunnerError::PolicyDenied { .. })
    ));
}

struct RustcParser;

impl EvidenceParser for RustcParser {
    fn qualified(&self) -> bool {
        true
    }

    fn parse(&self, exit_code: Option<i32>, stdout: &[u8], _stderr: &[u8]) -> ParsedEvidence {
        ParsedEvidence {
            metrics: EvidenceMetrics {
                passed: (exit_code == Some(0)).then_some(1),
                failed: (exit_code != Some(0)).then_some(1),
                skipped: Some(0),
                errors: Some(0),
            },
            summary: format!(
                "rustc parser: {} secret-value",
                String::from_utf8_lossy(stdout).trim()
            ),
        }
    }
}

fn evidence_digest(record: &project_ledger_domain::EvidenceRecord) -> Sha256Digest {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove("payload_digest");
    sha256_digest(&value).unwrap()
}

fn snapshot() -> RepositorySnapshot {
    RepositorySnapshot {
        repository_ref: id("repository:main"),
        snapshot_ref: id("snapshot:current"),
        snapshot_digest: Sha256Digest::from([3_u8; 32]),
    }
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
