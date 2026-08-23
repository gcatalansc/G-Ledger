use std::{collections::BTreeSet, io, path::PathBuf, process::Command, time::Instant};

use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    EvidenceKind, EvidenceMetrics, EvidencePrivacy, EvidenceRecord, EvidenceResult, EvidenceSource,
    EvidenceSourceType, EvidenceStatus, LedgerTimestamp, ProjectId, RepositorySnapshot,
    ResourceRef, SchemaVersion, StableId,
};
use thiserror::Error;

/// Exact executable and argv used for one evidence run. No shell string exists.
#[derive(Clone, Debug)]
pub struct EvidenceCommand {
    pub program: String,
    pub args: Vec<String>,
    pub command_display: String,
    pub working_directory: PathBuf,
    pub working_directory_ref: ResourceRef,
    pub tool: String,
}

/// Explicit local execution policy.
#[derive(Clone, Debug)]
pub struct EvidenceRunPolicy {
    pub allowed_programs: BTreeSet<String>,
    pub redactions: Vec<String>,
    pub max_summary_bytes: usize,
}

/// Durable identity and provenance supplied independently of process output.
#[derive(Clone, Debug)]
pub struct EvidenceRunContext {
    pub evidence_id: StableId,
    pub project_ref: ProjectId,
    pub kind: EvidenceKind,
    pub repository_binding: Option<RepositorySnapshot>,
    pub observed_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
    pub runner_version: String,
}

/// Bounded parser output; stdout/stderr remain ephemeral inputs.
#[derive(Clone, Debug)]
pub struct ParsedEvidence {
    pub metrics: EvidenceMetrics,
    pub summary: String,
}

/// Versioned toolchain parsers declare whether they are qualified for automatic
/// source verification.
pub trait EvidenceParser {
    fn qualified(&self) -> bool;
    fn parse(&self, exit_code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> ParsedEvidence;
}

/// Failures before a trustworthy evidence record can be produced.
#[derive(Debug, Error)]
pub enum EvidenceRunnerError {
    #[error("evidence execution denied by policy: {message}")]
    PolicyDenied { message: String },
    #[error("evidence process could not execute: {0}")]
    Process(#[from] io::Error),
    #[error("evidence record encoding failed: {0}")]
    Contract(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

/// Safe, synchronous local evidence runner.
#[derive(Clone, Copy, Debug, Default)]
pub struct EvidenceRunner;

impl EvidenceRunner {
    /// Runs an exact program+argv without a shell and returns a redacted record.
    ///
    /// # Errors
    ///
    /// Returns an error when policy rejects the request, the process cannot be
    /// created, or the closed record cannot be encoded and hashed.
    pub fn run<P: EvidenceParser>(
        policy: &EvidenceRunPolicy,
        request: &EvidenceCommand,
        context: &EvidenceRunContext,
        parser: &P,
    ) -> Result<EvidenceRecord, EvidenceRunnerError> {
        validate_policy(policy, request, context)?;
        let started = Instant::now();
        let output = Command::new(&request.program)
            .args(&request.args)
            .current_dir(&request.working_directory)
            .output()?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let exit_code = output.status.code();
        let parsed = parser.parse(exit_code, &output.stdout, &output.stderr);
        let summary = redact_and_bound(
            &parsed.summary,
            &policy.redactions,
            policy.max_summary_bytes,
        );
        let command_display = redact_and_bound(
            &request.command_display,
            &policy.redactions,
            policy.max_summary_bytes,
        );
        let status = if parser.qualified() && context.repository_binding.is_some() {
            EvidenceStatus::VerifiedSource
        } else {
            EvidenceStatus::Observed
        };
        let mut record = EvidenceRecord {
            schema_version: SchemaVersion::V0_2_0,
            evidence_id: context.evidence_id.clone(),
            project_ref: context.project_ref.clone(),
            kind: context.kind,
            status,
            source: EvidenceSource {
                source_type: EvidenceSourceType::CommandExecution,
                tool: request.tool.clone(),
                command_display: Some(command_display),
                working_directory_ref: Some(request.working_directory_ref.clone()),
                runner_version: context.runner_version.clone(),
            },
            result: EvidenceResult {
                exit_code,
                duration_ms,
                metrics: parsed.metrics,
                summary,
            },
            repository_binding: context.repository_binding.clone(),
            artifacts: Vec::new(),
            privacy: EvidencePrivacy {
                raw_output_stored: false,
                redaction_applied: true,
            },
            observed_at: context.observed_at.clone(),
            payload_digest: Sha256Digest::from([0_u8; 32]),
            provenance_ref: context.provenance_ref.clone(),
        };
        record.payload_digest = evidence_digest(&record)?;
        Ok(record)
    }
}

fn validate_policy(
    policy: &EvidenceRunPolicy,
    request: &EvidenceCommand,
    context: &EvidenceRunContext,
) -> Result<(), EvidenceRunnerError> {
    if !policy.allowed_programs.contains(&request.program) {
        return Err(EvidenceRunnerError::PolicyDenied {
            message: format!("program '{}' is not allowlisted", request.program),
        });
    }
    if request.program.trim().is_empty()
        || request.tool.trim().is_empty()
        || request.command_display.trim().is_empty()
        || context.runner_version.trim().is_empty()
        || policy.max_summary_bytes == 0
    {
        return Err(EvidenceRunnerError::PolicyDenied {
            message: "program, tool, display, runner version and summary bound are required"
                .to_owned(),
        });
    }
    Ok(())
}

fn redact_and_bound(value: &str, redactions: &[String], max_bytes: usize) -> String {
    let mut redacted = value.to_owned();
    for secret in redactions.iter().filter(|secret| !secret.is_empty()) {
        redacted = redacted.replace(secret, "[REDACTED]");
    }
    if redacted.len() <= max_bytes {
        return redacted;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !redacted.is_char_boundary(boundary) {
        boundary -= 1;
    }
    redacted.truncate(boundary);
    redacted
}

fn evidence_digest(record: &EvidenceRecord) -> Result<Sha256Digest, EvidenceRunnerError> {
    let mut value = serde_json::to_value(record)?;
    value
        .as_object_mut()
        .ok_or_else(|| EvidenceRunnerError::PolicyDenied {
            message: "evidence record must encode as an object".to_owned(),
        })?
        .remove("payload_digest");
    Ok(sha256_digest(&value)?)
}
