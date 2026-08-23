use std::collections::BTreeSet;

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_digest, to_canonical_vec,
};
use project_ledger_domain::{
    FreshnessStatus, IntegrationManifest, IntegrationStorage, LedgerTimestamp, LossItem,
    LossReport, LossSeverity, ProjectionCapability, ProjectionEnvelope, ProjectionPayload,
    ProjectionRequest, ProtocolDescriptor, ProviderHealthReport, ProviderHealthStatus,
    RecordCapability, ResourceRef, SchemaVersion, SideEffectContract, SideEffectLevel, StableId,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

const PROTOCOL_NAME: &str = "project-ledger-integration";
const PROTOCOL_VERSION: &str = "0.2.0";
const MAX_PROJECTION_BYTES: u64 = 1_048_576;
const MAX_PROJECTION_ITEMS: u32 = 1_000;

/// Failures at the generic integration boundary.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("protocol serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid integration contract: {message}")]
    InvalidContract { message: &'static str },
    #[error("{contract} digest or bound fields do not match")]
    IntegrityMismatch { contract: &'static str },
    #[error("projection payload exceeds byte budget: limit {limit}, actual {actual}")]
    BudgetExceeded { limit: u64, actual: u64 },
    #[error("projection contains a private field at {path}")]
    PrivacyViolation { path: String },
    #[error("upstream lock field is invalid: {field}")]
    InvalidUpstreamLock { field: String },
    #[error("upstream lock claims certification while exact evidence is incomplete")]
    CompatibilityOverclaim,
}

/// Inputs whose canonical binding produces a projection envelope.
#[derive(Clone, Debug)]
pub struct ProjectionEnvelopeDraft<'a> {
    pub request: &'a ProjectionRequest,
    pub manifest: &'a IntegrationManifest,
    pub source_product: &'a str,
    pub source_version: &'a str,
    pub state_version: u64,
    pub state_digest: Sha256Digest,
    pub payload_schema_ref: ResourceRef,
    pub coverage: Vec<String>,
    pub omissions: Vec<String>,
    pub provenance_ref: ResourceRef,
    pub payload: ProjectionPayload,
}

/// Inputs whose canonical binding produces a loss report.
#[derive(Clone, Debug)]
pub struct LossReportDraft {
    pub loss_report_id: StableId,
    pub source_schema_ref: ResourceRef,
    pub target_schema_ref: ResourceRef,
    pub items: Vec<LossItem>,
    pub created_at: LedgerTimestamp,
}

/// Builds the storage-independent Project Ledger integration manifest.
///
/// # Errors
///
/// Returns [`ProtocolError`] when canonical digest construction or fixed
/// protocol validation fails.
pub fn build_default_manifest(
    upstream_locks: Vec<Sha256Digest>,
) -> Result<IntegrationManifest, ProtocolError> {
    let mut manifest = IntegrationManifest {
        schema_version: SchemaVersion::V0_2_0,
        integration_manifest_id: StableId::parse("integration-manifest:project-ledger").map_err(
            |_| ProtocolError::InvalidContract {
                message: "default manifest identifier is invalid",
            },
        )?,
        provider_id: "project-ledger".to_owned(),
        provider_version: PROTOCOL_VERSION.to_owned(),
        protocols: vec![ProtocolDescriptor {
            name: PROTOCOL_NAME.to_owned(),
            version: PROTOCOL_VERSION.to_owned(),
        }],
        projection_capabilities: vec![
            ProjectionCapability::ProjectContinuity,
            ProjectionCapability::LedgerProjectSummary,
        ],
        record_capabilities: vec![
            RecordCapability::RecordWorkProgress,
            RecordCapability::RecordVerificationResult,
            RecordCapability::RequestGateEvaluation,
            RecordCapability::RequestGateRevalidation,
            RecordCapability::CreateContinuityCheckpoint,
        ],
        side_effect_contract: SideEffectContract {
            projection: SideEffectLevel::None,
            health: SideEffectLevel::None,
            record_submit: SideEffectLevel::DeclaredOnly,
        },
        storage: IntegrationStorage {
            shared_database_required: false,
        },
        upstream_locks,
        digest: zero_digest(),
    };
    manifest.digest = digest_without(&manifest, "digest")?;
    validate_manifest_integrity(&manifest)?;
    Ok(manifest)
}

/// Verifies fixed side-effect/storage semantics and canonical manifest digest.
///
/// # Errors
///
/// Returns [`ProtocolError`] when any fixed manifest semantic or digest differs.
pub fn validate_manifest_integrity(manifest: &IntegrationManifest) -> Result<(), ProtocolError> {
    manifest
        .validate()
        .map_err(|_| ProtocolError::InvalidContract {
            message: "manifest side-effect or storage semantics are invalid",
        })?;
    if manifest.schema_version != SchemaVersion::V0_2_0
        || manifest.integration_manifest_id.as_str() != "integration-manifest:project-ledger"
        || manifest.provider_id != "project-ledger"
        || manifest.provider_version != PROTOCOL_VERSION
        || manifest.protocols.len() != 1
        || manifest.protocols[0].name != PROTOCOL_NAME
        || manifest.protocols[0].version != PROTOCOL_VERSION
        || manifest.projection_capabilities
            != [
                ProjectionCapability::ProjectContinuity,
                ProjectionCapability::LedgerProjectSummary,
            ]
        || manifest.record_capabilities
            != [
                RecordCapability::RecordWorkProgress,
                RecordCapability::RecordVerificationResult,
                RecordCapability::RequestGateEvaluation,
                RecordCapability::RequestGateRevalidation,
                RecordCapability::CreateContinuityCheckpoint,
            ]
    {
        return Err(ProtocolError::InvalidContract {
            message: "manifest does not match generic protocol 0.2.0",
        });
    }
    if digest_without(manifest, "digest")? != manifest.digest {
        return Err(ProtocolError::IntegrityMismatch {
            contract: "integration manifest",
        });
    }
    Ok(())
}

/// Builds a health report bound to one validated manifest.
///
/// # Errors
///
/// Returns [`ProtocolError`] when the manifest is invalid or digest construction fails.
pub fn build_health(
    manifest: &IntegrationManifest,
    checked_at: LedgerTimestamp,
    status: ProviderHealthStatus,
    explanation_codes: Vec<String>,
) -> Result<ProviderHealthReport, ProtocolError> {
    validate_manifest_integrity(manifest)?;
    let mut report = ProviderHealthReport {
        schema_version: SchemaVersion::V0_2_0,
        provider_id: manifest.provider_id.clone(),
        provider_version: manifest.provider_version.clone(),
        status,
        checked_at,
        supported_protocols: manifest.protocols.clone(),
        explanation_codes,
        digest: zero_digest(),
    };
    report.digest = digest_without(&report, "digest")?;
    Ok(report)
}

/// Validates a health report against the exact manifest it advertises.
///
/// # Errors
///
/// Returns [`ProtocolError`] when provider/protocol fields or digest are not
/// bound to the supplied manifest.
pub fn validate_health_integrity(
    report: &ProviderHealthReport,
    manifest: &IntegrationManifest,
) -> Result<(), ProtocolError> {
    validate_manifest_integrity(manifest)?;
    if report.schema_version != SchemaVersion::V0_2_0
        || report.provider_id != manifest.provider_id
        || report.provider_version != manifest.provider_version
        || report.supported_protocols != manifest.protocols
        || digest_without(report, "digest")? != report.digest
    {
        return Err(ProtocolError::IntegrityMismatch {
            contract: "provider health",
        });
    }
    Ok(())
}

/// Builds a bounded, privacy-checked projection envelope.
///
/// # Errors
///
/// Returns [`ProtocolError`] for negotiation, identity/state/schema, coverage,
/// privacy, canonicalization or budget violations.
pub fn build_projection_envelope(
    draft: ProjectionEnvelopeDraft<'_>,
) -> Result<ProjectionEnvelope, ProtocolError> {
    validate_manifest_integrity(draft.manifest)?;
    validate_request(draft.request, draft.manifest)?;
    validate_coverage(draft.request, &draft.coverage, &draft.omissions)?;
    validate_payload_binding(
        &draft.payload,
        &draft.payload_schema_ref,
        &draft.request.project_ref,
        draft.state_version,
        draft.state_digest,
        &draft.coverage,
    )?;
    validate_private_value("payload", &serde_json::to_value(&draft.payload)?)?;
    let payload_bytes = to_canonical_vec(&draft.payload)?;
    let actual = u64::try_from(payload_bytes.len()).unwrap_or(u64::MAX);
    if actual > draft.request.budget.max_bytes {
        return Err(ProtocolError::BudgetExceeded {
            limit: draft.request.budget.max_bytes,
            actual,
        });
    }
    let freshness = match draft.request.expected_source_version {
        Some(expected) if expected != draft.state_version => FreshnessStatus::Stale,
        _ => FreshnessStatus::Current,
    };
    Ok(ProjectionEnvelope {
        schema_version: SchemaVersion::V0_2_0,
        request_ref: draft.request.request_id.clone(),
        profile: format!("{PROTOCOL_NAME}/{PROTOCOL_VERSION}"),
        source_product: draft.source_product.to_owned(),
        source_version: draft.source_version.to_owned(),
        manifest_digest: draft.manifest.digest,
        project_ref: draft.request.project_ref.clone(),
        state_version: draft.state_version,
        state_digest: draft.state_digest,
        payload_schema_ref: draft.payload_schema_ref,
        payload_digest: sha256_digest(&draft.payload)?,
        freshness,
        coverage: draft.coverage,
        omissions: draft.omissions,
        privacy_profile: draft.request.privacy_profile.clone(),
        provenance_ref: draft.provenance_ref,
        payload: draft.payload,
    })
}

/// Revalidates all request, manifest, payload, freshness and budget bindings.
///
/// # Errors
///
/// Returns [`ProtocolError`] when the envelope no longer matches its request,
/// manifest, payload, privacy profile, freshness or budget.
pub fn validate_projection_envelope(
    envelope: &ProjectionEnvelope,
    request: &ProjectionRequest,
    manifest: &IntegrationManifest,
) -> Result<(), ProtocolError> {
    validate_manifest_integrity(manifest)?;
    validate_request(request, manifest)?;
    validate_coverage(request, &envelope.coverage, &envelope.omissions)?;
    validate_payload_binding(
        &envelope.payload,
        &envelope.payload_schema_ref,
        &request.project_ref,
        envelope.state_version,
        envelope.state_digest,
        &envelope.coverage,
    )?;
    validate_private_value("payload", &serde_json::to_value(&envelope.payload)?)?;
    let payload_bytes = to_canonical_vec(&envelope.payload)?;
    let actual = u64::try_from(payload_bytes.len()).unwrap_or(u64::MAX);
    let expected_freshness = match request.expected_source_version {
        Some(expected) if expected != envelope.state_version => FreshnessStatus::Stale,
        _ => FreshnessStatus::Current,
    };
    if actual > request.budget.max_bytes {
        return Err(ProtocolError::BudgetExceeded {
            limit: request.budget.max_bytes,
            actual,
        });
    }
    if envelope.schema_version != SchemaVersion::V0_2_0
        || envelope.request_ref != request.request_id
        || envelope.profile != format!("{PROTOCOL_NAME}/{PROTOCOL_VERSION}")
        || envelope.manifest_digest != manifest.digest
        || envelope.project_ref != request.project_ref
        || envelope.payload_digest != sha256_digest(&envelope.payload)?
        || envelope.freshness != expected_freshness
        || envelope.privacy_profile != request.privacy_profile
    {
        return Err(ProtocolError::IntegrityMismatch {
            contract: "projection envelope",
        });
    }
    Ok(())
}

/// Builds a loss report with a derived material/critical count and digest.
///
/// # Errors
///
/// Returns [`ProtocolError`] when canonical report digest construction fails.
pub fn build_loss_report(draft: LossReportDraft) -> Result<LossReport, ProtocolError> {
    let material_loss_count = u32::try_from(
        draft
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.severity,
                    LossSeverity::Material | LossSeverity::Critical
                )
            })
            .count(),
    )
    .unwrap_or(u32::MAX);
    let mut report = LossReport {
        schema_version: SchemaVersion::V0_2_0,
        loss_report_id: draft.loss_report_id,
        source_schema_ref: draft.source_schema_ref,
        target_schema_ref: draft.target_schema_ref,
        items: draft.items,
        material_loss_count,
        created_at: draft.created_at,
        digest: zero_digest(),
    };
    report.digest = digest_without(&report, "digest")?;
    Ok(report)
}

/// Validates the derived loss count and canonical report digest.
///
/// # Errors
///
/// Returns [`ProtocolError`] when the loss count or report digest was altered.
pub fn validate_loss_report_integrity(report: &LossReport) -> Result<(), ProtocolError> {
    let expected_count = u32::try_from(
        report
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.severity,
                    LossSeverity::Material | LossSeverity::Critical
                )
            })
            .count(),
    )
    .unwrap_or(u32::MAX);
    if report.schema_version != SchemaVersion::V0_2_0
        || report.material_loss_count != expected_count
        || digest_without(report, "digest")? != report.digest
    {
        return Err(ProtocolError::IntegrityMismatch {
            contract: "loss report",
        });
    }
    Ok(())
}

fn validate_request(
    request: &ProjectionRequest,
    manifest: &IntegrationManifest,
) -> Result<(), ProtocolError> {
    if request.schema_version != SchemaVersion::V0_2_0
        || request.protocol.name != PROTOCOL_NAME
        || request.protocol.version != PROTOCOL_VERSION
        || !manifest.protocols.contains(&request.protocol)
    {
        return Err(ProtocolError::InvalidContract {
            message: "projection protocol negotiation failed",
        });
    }
    if request.budget.max_bytes == 0
        || request.budget.max_bytes > MAX_PROJECTION_BYTES
        || request.budget.max_items == 0
        || request.budget.max_items > MAX_PROJECTION_ITEMS
        || request.requested_fields.is_empty()
        || request.requested_fields.len() > request.budget.max_items as usize
    {
        return Err(ProtocolError::InvalidContract {
            message: "projection request exceeds fixed bounds",
        });
    }
    if !matches!(
        request.privacy_profile.as_str(),
        "FULL_LOCAL" | "MINIMAL_LOCAL" | "SHAREABLE"
    ) {
        return Err(ProtocolError::InvalidContract {
            message: "projection privacy profile is unsupported",
        });
    }
    let requested = request.requested_fields.iter().collect::<BTreeSet<_>>();
    if requested.len() != request.requested_fields.len()
        || requested
            .iter()
            .any(|field| !matches!(field.as_str(), "continuity" | "project_summary"))
    {
        return Err(ProtocolError::InvalidContract {
            message: "projection requested fields are duplicate or unsupported",
        });
    }
    Ok(())
}

fn validate_coverage(
    request: &ProjectionRequest,
    coverage: &[String],
    omissions: &[String],
) -> Result<(), ProtocolError> {
    if coverage.len() + omissions.len() > request.budget.max_items as usize {
        return Err(ProtocolError::InvalidContract {
            message: "projection coverage exceeds item budget",
        });
    }
    let requested = request
        .requested_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let covered = coverage
        .iter()
        .chain(omissions)
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if requested != covered || covered.len() != coverage.len() + omissions.len() {
        return Err(ProtocolError::InvalidContract {
            message: "projection coverage and omissions must partition requested fields",
        });
    }
    Ok(())
}

fn validate_payload_binding(
    payload: &ProjectionPayload,
    schema_ref: &ResourceRef,
    project_ref: &project_ledger_domain::ProjectId,
    state_version: u64,
    state_digest: Sha256Digest,
    coverage: &[String],
) -> Result<(), ProtocolError> {
    let matches = match payload {
        ProjectionPayload::ProjectContinuity(value) => {
            schema_ref.as_str() == "urn:project-ledger:schema:resume-projection:0.2.0"
                && value.project_ref == *project_ref
                && value.state_version == state_version
                && value.state_digest == state_digest
                && coverage.iter().any(|field| field == "continuity")
        }
        ProjectionPayload::ProjectState(value) => {
            schema_ref.as_str() == "urn:project-ledger:schema:project-state:0.2.0"
                && value.project.project_id == *project_ref
                && value.state_version == state_version
                && value.state_digest == state_digest
        }
        ProjectionPayload::LedgerProjectSummary(value) => {
            schema_ref.as_str() == "urn:project-ledger:schema:ledger-radar-project-projection:0.1.1"
                && value.identity.stable_project_id == *project_ref
                && value.source.state_version == state_version
                && value.source.state_digest == state_digest
                && coverage.iter().any(|field| field == "project_summary")
        }
    };
    if !matches {
        return Err(ProtocolError::InvalidContract {
            message: "projection payload identity, state or schema binding is invalid",
        });
    }
    Ok(())
}

fn validate_private_value(path: &str, value: &Value) -> Result<(), ProtocolError> {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let next = format!("{path}.{key}");
                let normalized = key.to_ascii_lowercase();
                if matches!(
                    normalized.as_str(),
                    "raw_output"
                        | "transcript"
                        | "reasoning"
                        | "secret"
                        | "password"
                        | "token"
                        | "api_key"
                        | "private_id"
                        | "encrypted_blob"
                ) {
                    return Err(ProtocolError::PrivacyViolation { path: next });
                }
                validate_private_value(&next, nested)?;
            }
        }
        Value::Array(values) => {
            for (index, nested) in values.iter().enumerate() {
                validate_private_value(&format!("{path}[{index}]"), nested)?;
            }
        }
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            let contains_secret = [
                "api_key=",
                "apikey=",
                "access_token=",
                "token=",
                "secret=",
                "password=",
                "bearer ",
            ]
            .iter()
            .any(|marker| lower.contains(marker));
            if contains_secret && !lower.contains("[redacted]") {
                return Err(ProtocolError::PrivacyViolation {
                    path: path.to_owned(),
                });
            }
        }
        _ => {}
    }
    Ok(())
}

fn digest_without<T: Serialize>(value: &T, field: &str) -> Result<Sha256Digest, ProtocolError> {
    let mut value = serde_json::to_value(value)?;
    value
        .as_object_mut()
        .ok_or(ProtocolError::InvalidContract {
            message: "digest-bearing contract must be a JSON object",
        })?
        .remove(field);
    Ok(sha256_digest(&value)?)
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from([0_u8; 32])
}
