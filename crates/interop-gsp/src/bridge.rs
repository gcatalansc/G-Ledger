use std::path::Path;

use project_ledger_canonical_json::{CanonicalJsonError, sha256_bytes, sha256_digest};
use serde_json::Value;
use thiserror::Error;

use crate::{
    GSP_CONTRACT_TAG, GspArtifactReceipt, GspBridgeReport, GspCompilationReceipt,
    GspCompilationResult, GspConflictVisibility, GspContractCatalog, GspContractError, GspCorpus,
    GspCorpusError,
};

const RUNTIME_CONTRACTS: [(&str, &str); 6] = [
    ("evidence_bundle", "evidence.bundle"),
    ("candidate_bundle", "candidate.bundle"),
    ("compilation_bundle", "compilation.bundle"),
    ("validation_report", "validation.report"),
    ("uncertainty_map", "uncertainty.map"),
    ("task_projection_bundle", "task.projection.bundle"),
];

/// Optional out-of-process compiler port. It has no Ledger storage capability.
pub trait GspCompilerSidecar: std::fmt::Debug {
    /// Compiles one already validated exact `EvidenceBundle`.
    ///
    /// # Errors
    ///
    /// Returns a transport/runtime failure without any Ledger side effect.
    fn compile(&self, query: &str, evidence_bundle: &Value) -> Result<Value, GspSidecarError>;
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum GspSidecarError {
    #[error("GSP sidecar is unavailable: {message}")]
    Unavailable { message: String },
    #[error("GSP sidecar returned an invalid transport response: {message}")]
    InvalidResponse { message: String },
}

#[derive(Debug, Error)]
pub enum GspBridgeError {
    #[error(transparent)]
    Contract(#[from] GspContractError),
    #[error(transparent)]
    Corpus(#[from] GspCorpusError),
    #[error(transparent)]
    Sidecar(#[from] GspSidecarError),
    #[error("GSP CompilationResult shape is invalid: {0}")]
    MalformedResult(serde_json::Error),
    #[error("GSP EvidenceBundle does not expose a valid request_id")]
    MissingRequestId,
    #[error("GSP EvidenceBundle does not expose a non-empty normalized_query")]
    MissingQuery,
    #[error("GSP result did not preserve the exact input EvidenceBundle")]
    EvidenceBundleMismatch,
    #[error("GSP request_id mismatch in {artifact}: expected {expected}, actual {actual}")]
    RequestIdMismatch {
        artifact: String,
        expected: String,
        actual: String,
    },
    #[error("GSP runtime validation overclaim: {message}")]
    InvalidRuntimeValidation { message: String },
    #[error("GSP result semantic status is absent")]
    MissingSemanticStatus,
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

/// Validated exact result, immutable receipt and non-authoritative report.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspBridgeOutput {
    pub result: GspCompilationResult,
    pub receipt: GspCompilationReceipt,
    pub report: GspBridgeReport,
}

/// Read-only exact-contract adapter. It owns no database or Python runtime.
#[derive(Clone, Debug)]
pub struct GspBridge {
    contracts: GspContractCatalog,
}

impl GspBridge {
    /// Loads the byte-locked vendored exact GSP contract catalog.
    ///
    /// # Errors
    ///
    /// Returns a contract error when any registry/schema byte or shape is invalid.
    pub fn load(repository_root: &Path) -> Result<Self, GspBridgeError> {
        Ok(Self {
            contracts: GspContractCatalog::load_vendored(repository_root)?,
        })
    }

    /// Calls an optional sidecar only after exact input validation.
    ///
    /// # Errors
    ///
    /// Returns input, sidecar, result or cross-artifact validation errors. This
    /// method has no Ledger storage handle and therefore cannot append events.
    pub fn compile(
        &self,
        sidecar: &dyn GspCompilerSidecar,
        corpus: &GspCorpus,
    ) -> Result<GspBridgeOutput, GspBridgeError> {
        corpus.verify_integrity()?;
        self.contracts
            .validate("evidence.bundle", &corpus.evidence_bundle)?;
        let query = corpus
            .evidence_bundle
            .get("normalized_query")
            .and_then(Value::as_str)
            .filter(|query| !query.trim().is_empty())
            .ok_or(GspBridgeError::MissingQuery)?;
        let raw = sidecar.compile(query, &corpus.evidence_bundle)?;
        self.validate_result(corpus, raw)
    }

    /// Validates a stored `CompilationResult` against all six exact contracts.
    ///
    /// # Errors
    ///
    /// Rejects reduced/extra result fields, schema drift, identity mismatch,
    /// changed evidence input or runtime validation overclaim.
    pub fn validate_result(
        &self,
        corpus: &GspCorpus,
        raw: Value,
    ) -> Result<GspBridgeOutput, GspBridgeError> {
        corpus.verify_integrity()?;
        self.contracts
            .validate("evidence.bundle", &corpus.evidence_bundle)?;
        let result: GspCompilationResult =
            serde_json::from_value(raw).map_err(GspBridgeError::MalformedResult)?;
        if result.evidence_bundle != corpus.evidence_bundle {
            return Err(GspBridgeError::EvidenceBundleMismatch);
        }
        let artifacts = artifact_values(&result);
        for ((_, contract), value) in RUNTIME_CONTRACTS.iter().zip(&artifacts) {
            self.contracts.validate(contract, value)?;
        }
        validate_runtime_confirmation(&result)?;
        let expected_request_id = request_id(&result.evidence_bundle)?;
        for ((artifact_name, _), value) in RUNTIME_CONTRACTS.iter().zip(&artifacts) {
            let actual = request_id(value)?;
            if actual != expected_request_id {
                return Err(GspBridgeError::RequestIdMismatch {
                    artifact: (*artifact_name).to_owned(),
                    expected: expected_request_id.to_owned(),
                    actual: actual.to_owned(),
                });
            }
        }
        if let Some(nested) = result
            .compilation_bundle
            .pointer("/uncertainty_map/request_id")
        {
            let actual = nested.as_str().ok_or(GspBridgeError::MissingRequestId)?;
            if actual != expected_request_id {
                return Err(GspBridgeError::RequestIdMismatch {
                    artifact: "compilation_bundle.uncertainty_map".to_owned(),
                    expected: expected_request_id.to_owned(),
                    actual: actual.to_owned(),
                });
            }
        }

        let semantic_status = result
            .validation_report
            .get("status")
            .and_then(Value::as_str)
            .ok_or(GspBridgeError::MissingSemanticStatus)?;
        let conflict_visibility = conflict_visibility(&result, semantic_status);
        let artifact_receipts = RUNTIME_CONTRACTS
            .iter()
            .zip(&artifacts)
            .map(|((artifact_name, contract_name), value)| {
                Ok(GspArtifactReceipt {
                    artifact_name: (*artifact_name).to_owned(),
                    contract_name: (*contract_name).to_owned(),
                    artifact_digest: sha256_digest(value)?,
                })
            })
            .collect::<Result<Vec<_>, CanonicalJsonError>>()?;
        let mut source_refs = corpus
            .manifest
            .files
            .iter()
            .map(|entry| entry.source_ref.clone())
            .collect::<Vec<_>>();
        source_refs.sort();
        source_refs.dedup();
        let mut receipt = GspCompilationReceipt {
            schema_version: "1.0.0".to_owned(),
            artifact_kind: "GSP_COMPILATION_RECEIPT".to_owned(),
            profile: GSP_CONTRACT_TAG.to_owned(),
            project_ref: corpus.manifest.project_ref.clone(),
            project_state_version: corpus.manifest.project_state_version,
            material_state_version: corpus.manifest.material_state_version,
            project_state_digest: corpus.manifest.project_state_digest,
            query_digest: corpus.manifest.query_digest,
            corpus_digest: corpus.manifest.corpus_digest,
            source_refs,
            request_id: expected_request_id.to_owned(),
            artifacts: artifact_receipts,
            runtime_validation_status: result.validation.status.clone(),
            runtime_authority_digest: sha256_bytes(result.validation.authority.as_bytes()),
            semantic_validation_status: semantic_status.to_owned(),
            conflict_visibility: conflict_visibility.clone(),
            authority_elevation: false,
            created_at: corpus.manifest.observed_at.clone(),
            receipt_digest: project_ledger_canonical_json::Sha256Digest::from([0; 32]),
        };
        receipt.seal()?;
        let report = GspBridgeReport {
            schema_version: "1.0.0".to_owned(),
            artifact_kind: "GSP_BRIDGE_REPORT".to_owned(),
            exact_contract_count: RUNTIME_CONTRACTS.len(),
            semantic_validation_status: semantic_status.to_owned(),
            conflict_visibility,
            semantic_degradation_count: 0,
            authority_elevation_count: 0,
            certified_compatibility: false,
        };
        Ok(GspBridgeOutput {
            result,
            receipt,
            report,
        })
    }
}

fn artifact_values(result: &GspCompilationResult) -> [&Value; 6] {
    [
        &result.evidence_bundle,
        &result.candidate_bundle,
        &result.compilation_bundle,
        &result.validation_report,
        &result.uncertainty_map,
        &result.task_projection_bundle,
    ]
}

fn validate_runtime_confirmation(result: &GspCompilationResult) -> Result<(), GspBridgeError> {
    let expected = RUNTIME_CONTRACTS
        .iter()
        .map(|(_, contract)| (*contract).to_owned())
        .collect::<Vec<_>>();
    if result.validation.status != "pass" {
        return Err(GspBridgeError::InvalidRuntimeValidation {
            message: "status must be pass after six exact contract checks".to_owned(),
        });
    }
    if result.validation.authority.trim().is_empty() {
        return Err(GspBridgeError::InvalidRuntimeValidation {
            message: "authority must be non-empty".to_owned(),
        });
    }
    if result.validation.contracts_checked != expected {
        return Err(GspBridgeError::InvalidRuntimeValidation {
            message: "contracts_checked must list the exact six runtime contracts in API order"
                .to_owned(),
        });
    }
    Ok(())
}

fn request_id(value: &Value) -> Result<&str, GspBridgeError> {
    value
        .get("request_id")
        .and_then(Value::as_str)
        .ok_or(GspBridgeError::MissingRequestId)
}

fn conflict_visibility(
    result: &GspCompilationResult,
    semantic_status: &str,
) -> GspConflictVisibility {
    let contradictions_present = result
        .evidence_bundle
        .get("conflicts_present")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || non_empty_array(&result.validation_report, "/contradiction_clusters")
        || non_empty_array(
            &result.compilation_bundle,
            "/validation/contradiction_clusters",
        );
    let uncertainty_present = non_empty_array(&result.uncertainty_map, "/items")
        || non_empty_array(&result.task_projection_bundle, "/uncertainty_notes")
        || non_empty_array(&result.compilation_bundle, "/uncertainty_map/items");
    GspConflictVisibility {
        contradictions_present,
        uncertainty_present,
        requires_human_review: semantic_status != "pass"
            || contradictions_present
            || uncertainty_present,
    }
}

fn non_empty_array(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}
