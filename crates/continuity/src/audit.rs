use project_ledger_canonical_json::sha256_bytes;
use project_ledger_domain::EventEnvelope;
use project_ledger_domain::{
    CheckpointFreshnessStatus, ConflictStatus, GeneratedArtifactKind, GeneratedArtifactRecord,
    SchemaVersion, Severity, StableId,
};
use project_ledger_projection_engine::ProjectStateProjection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditCode {
    StreamOrProjectionInvalid,
    GeneratedArtifactSourceDiverged,
    GeneratedArtifactModified,
    GeneratedArtifactContractInvalid,
    CheckpointNotCurrent,
    CriticalUnknownOpen,
    CriticalConflictOpen,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditFinding {
    pub severity: AuditSeverity,
    pub code: AuditCode,
    pub subject_refs: Vec<StableId>,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditReport {
    pub findings: Vec<AuditFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub valid: bool,
    pub findings: Vec<AuditFinding>,
}

/// Verifies source-chain integrity and the bytes of a generated artifact.
pub fn verify_stream_and_artifact(
    stream: &[EventEnvelope],
    record: &GeneratedArtifactRecord,
    artifact_bytes: &[u8],
) -> VerificationReport {
    let projection = match ProjectStateProjection::rebuild(stream) {
        Ok(projection) => projection,
        Err(error) => {
            return report(vec![AuditFinding {
                severity: AuditSeverity::Critical,
                code: AuditCode::StreamOrProjectionInvalid,
                subject_refs: vec![record.generated_artifact_id.clone()],
                message: format!("canonical stream or projection is invalid: {error}"),
            }]);
        }
    };
    let mut findings = Vec::new();
    if record.schema_version != SchemaVersion::V0_2_0
        || record.kind != GeneratedArtifactKind::Markdown
        || record.renderer_version.trim().is_empty()
        || !matches!(
            record.privacy_profile.as_str(),
            "FULL_LOCAL" | "MINIMAL_LOCAL" | "SHAREABLE"
        )
    {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: AuditCode::GeneratedArtifactContractInvalid,
            subject_refs: vec![record.generated_artifact_id.clone()],
            message: "generated artifact record does not identify a valid Markdown renderer"
                .to_owned(),
        });
    }
    if record.source_state_version != projection.state().state_version
        || record.source_state_digest != projection.state().state_digest
    {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: AuditCode::GeneratedArtifactSourceDiverged,
            subject_refs: vec![record.generated_artifact_id.clone()],
            message: "generated artifact source state differs from the canonical head".to_owned(),
        });
    }
    if record.artifact_digest != sha256_bytes(artifact_bytes) {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: AuditCode::GeneratedArtifactModified,
            subject_refs: vec![record.generated_artifact_id.clone()],
            message:
                "generated artifact bytes differ from the recorded digest; regenerate the view"
                    .to_owned(),
        });
    }
    report(findings)
}

/// Produces deterministic continuity findings from the current projection.
pub fn audit_projection(projection: &ProjectStateProjection) -> AuditReport {
    let state = projection.state();
    let mut findings = Vec::new();
    if let Some(checkpoint_ref) = &state.checkpoint_ref {
        let freshness = projection
            .checkpoint_freshness(checkpoint_ref)
            .unwrap_or(CheckpointFreshnessStatus::Unknown);
        if !matches!(
            freshness,
            CheckpointFreshnessStatus::Current | CheckpointFreshnessStatus::CurrentForProjectOnly
        ) {
            findings.push(AuditFinding {
                severity: AuditSeverity::High,
                code: AuditCode::CheckpointNotCurrent,
                subject_refs: vec![checkpoint_ref.clone()],
                message: format!("continuity checkpoint is not current: {freshness:?}"),
            });
        }
    }
    for unknown_ref in projection.critical_unknown_refs() {
        findings.push(AuditFinding {
            severity: AuditSeverity::High,
            code: AuditCode::CriticalUnknownOpen,
            subject_refs: vec![unknown_ref],
            message: "material unknown remains open".to_owned(),
        });
    }
    for conflict_ref in &state.open_conflict_refs {
        if projection.conflict(conflict_ref).is_some_and(|conflict| {
            conflict.status == ConflictStatus::Open && conflict.severity == Severity::Critical
        }) {
            findings.push(AuditFinding {
                severity: AuditSeverity::Critical,
                code: AuditCode::CriticalConflictOpen,
                subject_refs: vec![conflict_ref.clone()],
                message: "critical conflict remains open".to_owned(),
            });
        }
    }
    AuditReport { findings }
}

fn report(findings: Vec<AuditFinding>) -> VerificationReport {
    VerificationReport {
        valid: findings.is_empty(),
        findings,
    }
}
