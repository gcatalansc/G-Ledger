use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{LedgerTimestamp, LinkExternalEvidencePayload, ProjectId, ResourceRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const RECEIPT_SCHEMA_REF: &str = "urn:project-ledger:schema:gsp-compilation-receipt:1.0.0";

/// The seven-field public result emitted by GSP-Gemma Compiler v0.2.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCompilationResult {
    pub evidence_bundle: Value,
    pub candidate_bundle: Value,
    pub compilation_bundle: Value,
    pub validation_report: Value,
    pub uncertainty_map: Value,
    pub task_projection_bundle: Value,
    pub validation: GspRuntimeValidation,
}

/// Compiler-side confirmation that all exact runtime contracts were checked.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspRuntimeValidation {
    pub status: String,
    pub authority: String,
    pub contracts_checked: Vec<String>,
}

/// Digest of one unmodified exact GSP artifact.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspArtifactReceipt {
    pub artifact_name: String,
    pub contract_name: String,
    pub artifact_digest: Sha256Digest,
}

/// Visible semantic signals that must survive the adapter boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspConflictVisibility {
    pub contradictions_present: bool,
    pub uncertainty_present: bool,
    pub requires_human_review: bool,
}

/// Digest-bound local receipt; it references exact outputs without reducing them.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCompilationReceipt {
    pub schema_version: String,
    pub artifact_kind: String,
    pub profile: String,
    pub project_ref: ProjectId,
    pub project_state_version: u64,
    pub material_state_version: u64,
    pub project_state_digest: Sha256Digest,
    pub query_digest: Sha256Digest,
    pub corpus_digest: Sha256Digest,
    pub source_refs: Vec<String>,
    pub request_id: String,
    pub artifacts: Vec<GspArtifactReceipt>,
    pub runtime_validation_status: String,
    pub runtime_authority_digest: Sha256Digest,
    pub semantic_validation_status: String,
    pub conflict_visibility: GspConflictVisibility,
    pub authority_elevation: bool,
    pub created_at: LedgerTimestamp,
    pub receipt_digest: Sha256Digest,
}

impl GspCompilationReceipt {
    /// Creates the only native Ledger mutation payload exposed by this bridge.
    ///
    /// # Panics
    ///
    /// Cannot panic in a valid build because the fixed schema reference is a
    /// compile-time constant covered by conformance tests.
    pub fn link_payload(&self, artifact_uri: ResourceRef) -> LinkExternalEvidencePayload {
        LinkExternalEvidencePayload {
            external_system: "GSP-Gemma Compiler".to_owned(),
            artifact_uri,
            artifact_digest: self.receipt_digest,
            schema_ref: ResourceRef::parse(RECEIPT_SCHEMA_REF)
                .expect("the fixed receipt schema reference is valid"),
        }
    }

    /// Verifies the embedded canonical receipt digest.
    ///
    /// # Errors
    ///
    /// Returns a canonical JSON error if the receipt cannot be serialized.
    pub fn verify_digest(&self) -> Result<bool, project_ledger_canonical_json::CanonicalJsonError> {
        let mut value = serde_json::to_value(self)?;
        let Some(object) = value.as_object_mut() else {
            return Ok(false);
        };
        object.remove("receipt_digest");
        Ok(sha256_digest(&value)? == self.receipt_digest)
    }

    pub(crate) fn seal(&mut self) -> Result<(), project_ledger_canonical_json::CanonicalJsonError> {
        self.receipt_digest = Sha256Digest::from([0; 32]);
        let mut value = serde_json::to_value(&*self)?;
        value
            .as_object_mut()
            .expect("receipt serializes as an object")
            .remove("receipt_digest");
        self.receipt_digest = sha256_digest(&value)?;
        Ok(())
    }
}

/// Local conformance summary with explicit authority and degradation counts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspBridgeReport {
    pub schema_version: String,
    pub artifact_kind: String,
    pub exact_contract_count: usize,
    pub semantic_validation_status: String,
    pub conflict_visibility: GspConflictVisibility,
    pub semantic_degradation_count: usize,
    pub authority_elevation_count: usize,
    pub certified_compatibility: bool,
}
