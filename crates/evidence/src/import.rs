use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_bytes, sha256_digest,
};
use project_ledger_domain::{
    EvidenceArtifactRef, EvidenceRecord, EvidenceSource, EvidenceSourceType, EvidenceStatus,
    ProjectId, ResourceRef, SchemaVersion,
};
use thiserror::Error;

/// Trusted context supplied by the local import boundary.
#[derive(Clone, Debug)]
pub struct EvidenceImportContext {
    pub expected_project_ref: ProjectId,
    pub artifact_uri: ResourceRef,
    pub provenance_ref: ResourceRef,
    pub importer_version: String,
}

/// Failures that prevent an imported artifact from becoming even asserted/observed evidence.
#[derive(Debug, Error)]
pub enum EvidenceImporterError {
    #[error("imported evidence does not satisfy the closed contract: {0}")]
    Contract(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("imported evidence payload digest does not match its contents")]
    DigestMismatch,
    #[error("imported evidence belongs to a different project")]
    ProjectMismatch,
    #[error("only evidence contract version 0.2.0 can be imported")]
    UnsupportedSchemaVersion,
    #[error("importer version cannot be empty")]
    InvalidImporterVersion,
}

/// Trust-preserving evidence importer.
#[derive(Clone, Copy, Debug, Default)]
pub struct EvidenceImporter;

impl EvidenceImporter {
    /// Imports closed JSON while downgrading unsigned local content to OBSERVED.
    ///
    /// # Errors
    ///
    /// Rejects unknown fields, invalid JSON, digest mismatch, schema mismatch or
    /// cross-project evidence before producing a new canonical record.
    pub fn import_unsigned_json(
        bytes: &[u8],
        context: &EvidenceImportContext,
    ) -> Result<EvidenceRecord, EvidenceImporterError> {
        if context.importer_version.trim().is_empty() {
            return Err(EvidenceImporterError::InvalidImporterVersion);
        }
        let mut record: EvidenceRecord = serde_json::from_slice(bytes)?;
        if record.schema_version != SchemaVersion::V0_2_0 {
            return Err(EvidenceImporterError::UnsupportedSchemaVersion);
        }
        if record.project_ref != context.expected_project_ref {
            return Err(EvidenceImporterError::ProjectMismatch);
        }
        if evidence_digest(&record)? != record.payload_digest {
            return Err(EvidenceImporterError::DigestMismatch);
        }

        record.status = EvidenceStatus::Observed;
        record.source = EvidenceSource {
            source_type: EvidenceSourceType::ImportedArtifact,
            tool: "project-ledger-evidence-import".to_owned(),
            command_display: None,
            working_directory_ref: None,
            runner_version: context.importer_version.clone(),
        };
        record.artifacts = vec![EvidenceArtifactRef {
            uri: context.artifact_uri.clone(),
            digest: sha256_bytes(bytes),
            persisted: false,
        }];
        record.privacy.raw_output_stored = false;
        record.privacy.redaction_applied = true;
        record.provenance_ref = context.provenance_ref.clone();
        record.payload_digest = Sha256Digest::from([0_u8; 32]);
        record.payload_digest = evidence_digest(&record)?;
        Ok(record)
    }
}

fn evidence_digest(record: &EvidenceRecord) -> Result<Sha256Digest, EvidenceImporterError> {
    let mut value = serde_json::to_value(record)?;
    value
        .as_object_mut()
        .ok_or_else(|| {
            EvidenceImporterError::Contract(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "evidence record must encode as an object",
            )))
        })?
        .remove("payload_digest");
    Ok(sha256_digest(&value)?)
}
