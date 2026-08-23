use std::{collections::BTreeMap, time::Instant};

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_digest, to_canonical_vec,
};
use project_ledger_domain::{LedgerTimestamp, Phase, ResourceRef, StableId};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use thiserror::Error;

use crate::{
    ContractError, GSKILLS_FIXTURE_MANIFEST_SHA256, GSKILLS_PROFILE, GSKILLS_REGISTRY_SHA256,
    GSKILLS_SPEC_REVISION, GSKILLS_SPEC_SHA256, LEDGER_SPEC_SHA256, RECIPROCAL_LOCK_SHA256,
    mapping::{
        assign_digest, continuity_projection, current_repository, map_checkpoint_freshness,
        phase_metadata_is_complete,
    },
    validate_read_contract,
};

const MAX_ITEMS: u64 = 256;
const MAX_TOKENS: u64 = 4096;
const MAX_OUTPUT_BYTES: u64 = 1_048_576;
const MAX_TIMEOUT_MS: u64 = 5000;
const SUPPORTED_FIELDS: &[&str] = &[
    "project_state_version",
    "material_state_version",
    "project_state_digest",
    "authority_documents",
    "active_phases",
    "primary_active_phase_ref",
    "verified_completed_phases",
    "current_decisions",
    "open_blockers",
    "open_risks",
    "pending_gates",
    "revalidation_required_gates",
    "stale_gates",
    "next_actions",
    "checkpoint",
    "repository_binding",
    "recent_material_events",
    "last_material_activity_at",
    "unknowns",
    "conflicts",
];

/// Immutable composition metadata that is not retained by compact `ProjectState`.
#[derive(Clone, Debug)]
pub struct ProviderContext {
    pub gskills_project_ref: String,
    pub external_project_ref: ResourceRef,
    pub phase_catalog: BTreeMap<StableId, Phase>,
    pub process_instance_id: StableId,
    pub observed_at: LedgerTimestamp,
}

/// State tuple used to prove the exact provider's reads do not mutate Ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GSkillsObservableState {
    pub state_version: u64,
    pub material_state_version: u64,
    pub event_count: u64,
    pub state_digest: Sha256Digest,
}

/// Exact three-record result returned by the S9 profile boundary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GSkillsProjectionResult {
    pub payload: Value,
    pub freshness: Value,
    pub envelope: Value,
}

/// Fail-closed errors for exact G-Skills request negotiation and mapping.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("active phase metadata is missing for {phase_ref}")]
    PhaseMetadataMissing { phase_ref: String },
    #[error("active phase metadata conflicts with projected status for {phase_ref}")]
    PhaseMetadataMismatch { phase_ref: String },
    #[error("repository identity is ambiguous: {count} CURRENT bindings")]
    AmbiguousRepositoryBinding { count: usize },
    #[error("missing indexed {kind} record {reference}")]
    MissingIndexedRecord {
        kind: &'static str,
        reference: String,
    },
    #[error("canonical source field is unavailable: {field}")]
    MissingSourceField { field: &'static str },
    #[error("invalid request contract: {message}")]
    InvalidRequest { message: String },
    #[error("request digest does not cover the exact request")]
    RequestDigestMismatch,
    #[error("identity mismatch: {message}")]
    IdentityMismatch { message: &'static str },
    #[error("privacy request is forbidden: {message}")]
    PrivacyViolation { message: &'static str },
    #[error("projection exceeds {dimension} budget: {actual} > {limit}")]
    BudgetExceeded {
        dimension: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("internal exact-contract invariant failed: {message}")]
    InternalContract { message: &'static str },
}

/// Storage-independent exact G-Skills v0.3.0 read-only provider.
#[derive(Clone, Debug)]
pub struct GSkillsProvider {
    projection: ProjectStateProjection,
    context: ProviderContext,
}

impl GSkillsProvider {
    /// Builds a provider only when compact state and injected phase/repository metadata agree.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] for missing/mismatched active phase metadata or
    /// more than one current repository binding.
    pub fn new(
        projection: ProjectStateProjection,
        context: ProviderContext,
    ) -> Result<Self, ProviderError> {
        if context.gskills_project_ref.trim().is_empty() {
            return Err(ProviderError::IdentityMismatch {
                message: "empty G-Skills project reference",
            });
        }
        current_repository(&projection)?;
        for phase_ref in &projection.state().active_phase_refs {
            let phase = context.phase_catalog.get(phase_ref).ok_or_else(|| {
                ProviderError::PhaseMetadataMissing {
                    phase_ref: phase_ref.to_string(),
                }
            })?;
            if !phase_metadata_is_complete(phase)
                || projection.phase_status(phase_ref) != Some(phase.status)
            {
                return Err(ProviderError::PhaseMetadataMismatch {
                    phase_ref: phase_ref.to_string(),
                });
            }
        }
        Ok(Self {
            projection,
            context,
        })
    }

    /// Returns the exact observable state tuple without modifying the projection.
    pub fn observable_state(&self) -> GSkillsObservableState {
        let state = self.projection.state();
        GSkillsObservableState {
            state_version: state.state_version,
            material_state_version: state.material_state_version,
            event_count: state.state_version,
            state_digest: state.state_digest,
        }
    }

    /// Emits a deterministic exact-profile manifest with no write capability.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] if canonical digesting or schema validation fails.
    pub fn manifest(&self) -> Result<Value, ProviderError> {
        let mut manifest = json!({
            "schema_version": "0.3.0",
            "provider_manifest_id": "provider-manifest:project-ledger-v0.2.0",
            "provider_id": "project-ledger",
            "provider_version": "0.2.0",
            "protocol": {
                "name": "gskills-context-provider",
                "version": "0.3.0",
                "compatible_range": ">=0.3.0 <0.4.0"
            },
            "implementation": {
                "language": "rust",
                "crate": "project-ledger-interop-gskills",
                "process_instance_id": self.context.process_instance_id,
                "shared_database_required": false
            },
            "transports": ["gskills-stdio-jsonrpc-content-length/1.0.0"],
            "operations": {
                "manifest": "ledger.manifest",
                "health": "ledger.health",
                "project_continuity": "ledger.project_continuity"
            },
            "projection_capabilities": ["PROJECT_CONTINUITY"],
            "record_sink_capabilities": [],
            "provided_domains": ["project.lifecycle", "project.gates", "project.decisions"],
            "claimed_authority_domains": ["project.lifecycle", "project.gates", "project.decisions"],
            "side_effect_contract": {
                "reads_mutate_project_state": false,
                "reads_mutate_event_count": false,
                "writes_supported": false
            },
            "freshness_model": {
                "project_and_repository_separate": true,
                "checkpoint_n_plus_one_preserved": true
            },
            "privacy": {
                "source_bodies": false,
                "raw_events": false,
                "repository_paths": false
            },
            "schema_refs": {
                "request": "urn:gskills:schema:context-projection-request:0.3.0",
                "envelope": "urn:gskills:schema:context-projection-envelope:0.3.0",
                "freshness": "urn:gskills:schema:provider-freshness-record:0.3.0",
                "projection": "urn:gskills:schema:project-continuity-projection:0.3.0"
            },
            "contract_identity": {
                "profile": GSKILLS_PROFILE,
                "registry_file_sha256": GSKILLS_REGISTRY_SHA256,
                "fixture_manifest_file_sha256": GSKILLS_FIXTURE_MANIFEST_SHA256
            },
            "upstream": {
                "revision": GSKILLS_SPEC_REVISION,
                "spec_sha256": GSKILLS_SPEC_SHA256,
                "reciprocal_lock_sha256": RECIPROCAL_LOCK_SHA256,
                "ledger_spec_sha256": LEDGER_SPEC_SHA256
            },
            "created_at": self.context.observed_at,
            "digest": Sha256Digest::from([0; 32]),
        });
        assign_digest(&mut manifest)?;
        validate_read_contract("provider-manifest", &manifest)?;
        Ok(manifest)
    }

    /// Emits deterministic provider health bound to the exact manifest digest.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] if manifest creation or schema validation fails.
    pub fn health(&self) -> Result<Value, ProviderError> {
        let manifest = self.manifest()?;
        let mut health = json!({
            "schema_version": "0.3.0",
            "provider_health_report_id": format!("provider-health:ledger-{}", self.projection.state().state_version),
            "provider_ref": "project-ledger",
            "provider_manifest_digest": manifest["digest"],
            "runtime_status": "AVAILABLE",
            "checks": {
                "projection_loaded": "PASS",
                "phase_metadata": "PASS",
                "repository_identity": "PASS",
                "schema_fixture_lock": "PASS"
            },
            "checked_at": self.context.observed_at,
            "valid_until": self.context.observed_at,
            "latency_ms": 0,
            "quarantine_reason": null,
            "warnings": [],
            "digest": Sha256Digest::from([0; 32]),
        });
        assign_digest(&mut health)?;
        validate_read_contract("provider-health-report", &health)?;
        Ok(health)
    }

    /// Maps one exact request to projection, freshness and envelope records.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] for invalid digest/schema, identity ambiguity,
    /// forbidden privacy/write access, unavailable canonical fields or bounds.
    pub fn project(&self, request: &Value) -> Result<GSkillsProjectionResult, ProviderError> {
        let started = Instant::now();
        let parsed = parse_request(request)?;
        self.validate_identity(&parsed.identity)?;
        let payload = continuity_projection(&self.projection, &self.context)?;
        enforce_payload_bounds(&payload, &parsed.budget)?;
        let freshness = self.freshness(&payload, &parsed)?;
        let envelope = self.envelope(request, &payload, &freshness, &parsed)?;
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if elapsed_ms > parsed.budget.timeout_ms {
            return Err(ProviderError::BudgetExceeded {
                dimension: "timeout_ms",
                actual: elapsed_ms,
                limit: parsed.budget.timeout_ms,
            });
        }
        Ok(GSkillsProjectionResult {
            payload,
            freshness,
            envelope,
        })
    }

    fn validate_identity(&self, identity: &RequestIdentity) -> Result<(), ProviderError> {
        if identity.project_ref != self.context.gskills_project_ref {
            return Err(ProviderError::IdentityMismatch {
                message: "project_ref does not match the confirmed provider context",
            });
        }
        let repository = current_repository(&self.projection)?;
        match (&identity.repository_ref, repository) {
            (Some(expected), Some(binding)) if expected == binding.repository_ref.as_str() => {}
            (None, None) => {}
            (Some(_), Some(_)) => {
                return Err(ProviderError::IdentityMismatch {
                    message: "repository_ref does not match the current binding",
                });
            }
            _ => {
                return Err(ProviderError::IdentityMismatch {
                    message: "repository identity presence differs from the current binding",
                });
            }
        }
        Ok(())
    }

    fn freshness(&self, payload: &Value, request: &ParsedRequest) -> Result<Value, ProviderError> {
        let state = self.projection.state();
        let repository = current_repository(&self.projection)?;
        let status = if request.consistency.expected_project_state_version != state.state_version
            || request.consistency.expected_material_state_version != state.material_state_version
        {
            "SUPERSEDED_BY_NEWER_CHECKPOINT"
        } else if request.consistency.expected_project_state_digest
            != state.state_digest.to_string()
        {
            "CONFLICTED"
        } else if repository.is_some_and(|binding| {
            request.identity.repository_snapshot_ref.as_deref()
                != binding.current_snapshot_ref.as_ref().map(StableId::as_str)
        }) {
            "STALE_FOR_REPOSITORY_STATE"
        } else if let Some(checkpoint_ref) = &state.checkpoint_ref {
            map_checkpoint_freshness(
                self.projection
                    .checkpoint_freshness(checkpoint_ref)
                    .unwrap_or(project_ledger_domain::CheckpointFreshnessStatus::Unknown),
            )
        } else if repository.is_none() {
            "CURRENT_FOR_PROJECT_ONLY"
        } else {
            "CURRENT"
        };
        let checkpoint = state
            .checkpoint_ref
            .as_ref()
            .and_then(|checkpoint_ref| self.projection.checkpoint(checkpoint_ref));
        let mut freshness = json!({
            "schema_version": "0.3.0",
            "provider_freshness_record_id": format!("provider-freshness:ledger-{}", state.state_version),
            "provider_ref": "project-ledger",
            "projection_ref": payload["project_continuity_projection_id"],
            "status": status,
            "basis": {
                "project_state_version": state.state_version,
                "material_state_version": state.material_state_version,
                "project_state_digest": state.state_digest,
                "checkpoint": checkpoint.map(|value| json!({
                    "ref": value.checkpoint_id,
                    "digest": value.checkpoint_digest,
                    "captured_state_version": value.captured_state_version,
                    "captured_material_state_version": value.captured_material_state_version,
                    "captured_state_digest": value.captured_state_digest,
                    "checkpoint_event_version": value.checkpoint_event_version,
                    "checkpoint_event_ref": value.checkpoint_event_ref,
                    "upstream_freshness": self.projection.checkpoint_freshness(&value.checkpoint_id)
                        .map_or("COMPATIBILITY_UNKNOWN", map_checkpoint_freshness)
                })),
                "authority_document_digests": state.authority_documents.iter().map(|value| value.digest).collect::<Vec<_>>(),
                "repository_binding": repository.map(|binding| json!({
                    "binding_ref": binding.repository_binding_id,
                    "status": binding.status,
                    "repository_ref": binding.repository_ref,
                    "repository_snapshot_ref": binding.current_snapshot_ref,
                    "repository_snapshot_digest": binding.current_snapshot_digest,
                }))
            },
            "compatibility_snapshot": {
                "provider_contract": "SPEC_LEVEL_MATCH",
                "upstream_spec_lock": "PASS",
                "schema_fixture_lock": "PASS",
                "project_mapping": "PASS",
                "repository_binding": if repository.is_some() { "PASS" } else { "NOT_APPLICABLE" },
                "eligibility_record_ref": format!("provider-eligibility:ledger-{}", state.state_version)
            },
            "expires_at": null,
            "evaluated_at": self.context.observed_at,
            "evaluator": {"id": "project-ledger-gskills-freshness", "version": "0.2.0"},
            "reason_codes": freshness_reason_codes(status),
            "digest": Sha256Digest::from([0; 32]),
        });
        assign_digest(&mut freshness)?;
        validate_read_contract("provider-freshness-record", &freshness)?;
        Ok(freshness)
    }

    fn envelope(
        &self,
        request: &Value,
        payload: &Value,
        freshness: &Value,
        parsed: &ParsedRequest,
    ) -> Result<Value, ProviderError> {
        let manifest = self.manifest()?;
        let returned_fields = parsed.requested_fields.len();
        let mut envelope = json!({
            "schema_version": "0.3.0",
            "context_projection_envelope_id": format!("projection-envelope:ledger-{}", self.projection.state().state_version),
            "request_ref": parsed.context_projection_request_id,
            "request_digest": request["digest"],
            "provider": {
                "id": "project-ledger",
                "version": "0.2.0",
                "native_manifest_digest": manifest["digest"],
                "normalized_manifest_digest": manifest["digest"],
                "compatibility_lock_ref": "provider-lock:project-ledger-v0.2-gskills-v0.3",
                "process_instance_id": self.context.process_instance_id,
            },
            "projection_kind": "PROJECT_CONTINUITY",
            "delivery_status": "DELIVERED",
            "payload_validation_status": "PASS",
            "payload": {
                "schema_ref": "urn:gskills:schema:project-continuity-projection:0.3.0",
                "ref": payload["project_continuity_projection_id"],
                "digest": payload["digest"]
            },
            "freshness_record_ref": freshness["provider_freshness_record_id"],
            "eligibility_record_ref": format!("provider-eligibility:ledger-{}", self.projection.state().state_version),
            "claimed_authority_domains": ["project.lifecycle", "project.gates", "project.decisions"],
            "effective_authority_domains": [],
            "advisory_domains": ["project.lifecycle", "project.gates", "project.decisions"],
            "coverage": {
                "requested_fields": returned_fields,
                "returned_fields": returned_fields,
                "omitted_fields": [],
                "partial_reasons": []
            },
            "loss_report_ref": null,
            "warnings": [],
            "provenance_ref": format!("project-ledger://provenance/gskills/envelope/{}", self.projection.state().state_version),
            "created_at": self.context.observed_at,
            "digest": Sha256Digest::from([0; 32]),
        });
        assign_digest(&mut envelope)?;
        validate_read_contract("context-projection-envelope", &envelope)?;
        Ok(envelope)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // Names mirror the exact external identity contract.
struct RequestIdentity {
    project_ref: String,
    repository_ref: Option<String>,
    repository_snapshot_ref: Option<String>,
    session_ref: Option<String>,
    task_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestBudget {
    max_items: u64,
    max_tokens: u64,
    max_output_bytes: u64,
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestPrivacy {
    allowed_fields: Vec<String>,
    source_bodies: bool,
    raw_events: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestConsistency {
    expected_project_state_version: u64,
    expected_material_state_version: u64,
    expected_project_state_digest: String,
    require_freshness: bool,
    allow_partial: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuralScope {
    path_refs: Vec<String>,
    symbol_refs: Vec<String>,
    impact_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    schema_version: String,
    context_projection_request_id: String,
    request_id: String,
    traceparent: String,
    provider_id: String,
    projection_kind: String,
    stage: String,
    purpose: String,
    identity: RequestIdentity,
    structural_scope: StructuralScope,
    requested_fields: Vec<String>,
    budget: RequestBudget,
    privacy: RequestPrivacy,
    consistency: RequestConsistency,
    write_capability: String,
    requested_at: String,
    digest: String,
    #[serde(default)]
    extensions: Option<Map<String, Value>>,
}

struct ParsedRequest {
    context_projection_request_id: String,
    identity: RequestIdentity,
    requested_fields: Vec<String>,
    budget: RequestBudget,
    consistency: RequestConsistency,
}

fn parse_request(value: &Value) -> Result<ParsedRequest, ProviderError> {
    validate_read_contract("context-projection-request", value)?;
    let wire: WireRequest =
        serde_json::from_value(value.clone()).map_err(|error| ProviderError::InvalidRequest {
            message: error.to_string(),
        })?;
    validate_request_digest(value, &wire.digest)?;
    if wire.schema_version != "0.3.0"
        || wire.provider_id != "project-ledger"
        || wire.projection_kind != "PROJECT_CONTINUITY"
        || wire.write_capability != "NONE"
        || wire.context_projection_request_id.trim().is_empty()
        || wire.request_id.trim().is_empty()
        || wire.traceparent.trim().is_empty()
        || wire.stage.trim().is_empty()
        || wire.purpose.trim().is_empty()
        || wire.requested_at.trim().is_empty()
    {
        return Err(ProviderError::InvalidRequest {
            message: "profile, operation, identity or required text differs from v0.3.0".to_owned(),
        });
    }
    validate_fields(&wire.requested_fields)?;
    validate_budget(&wire.budget)?;
    validate_privacy(&wire.privacy)?;
    if !wire.consistency.require_freshness || !wire.consistency.allow_partial {
        return Err(ProviderError::InvalidRequest {
            message: "S9 requires freshness evaluation and explicit partial handling".to_owned(),
        });
    }
    let _ = (
        &wire.identity.session_ref,
        &wire.identity.task_ref,
        &wire.structural_scope.path_refs,
        &wire.structural_scope.symbol_refs,
        &wire.structural_scope.impact_ref,
        &wire.extensions,
    );
    Ok(ParsedRequest {
        context_projection_request_id: wire.context_projection_request_id,
        identity: wire.identity,
        requested_fields: wire.requested_fields,
        budget: wire.budget,
        consistency: wire.consistency,
    })
}

fn validate_request_digest(value: &Value, claimed: &str) -> Result<(), ProviderError> {
    let mut unsigned = value.clone();
    unsigned
        .as_object_mut()
        .ok_or(ProviderError::InvalidRequest {
            message: "request must be an object".to_owned(),
        })?
        .remove("digest");
    if sha256_digest(&unsigned)?.to_string() != claimed {
        return Err(ProviderError::RequestDigestMismatch);
    }
    Ok(())
}

fn validate_fields(fields: &[String]) -> Result<(), ProviderError> {
    if fields.len() != SUPPORTED_FIELDS.len()
        || fields
            .iter()
            .enumerate()
            .any(|(index, field)| field != SUPPORTED_FIELDS[index])
    {
        return Err(ProviderError::InvalidRequest {
            message: "S9 full profile requires the exact ordered 20-field projection".to_owned(),
        });
    }
    Ok(())
}

fn validate_budget(budget: &RequestBudget) -> Result<(), ProviderError> {
    for (dimension, value, maximum) in [
        ("items", budget.max_items, MAX_ITEMS),
        ("tokens", budget.max_tokens, MAX_TOKENS),
        ("output_bytes", budget.max_output_bytes, MAX_OUTPUT_BYTES),
        ("timeout_ms", budget.timeout_ms, MAX_TIMEOUT_MS),
    ] {
        if value == 0 || value > maximum {
            return Err(ProviderError::BudgetExceeded {
                dimension,
                actual: value,
                limit: maximum,
            });
        }
    }
    Ok(())
}

fn validate_privacy(privacy: &RequestPrivacy) -> Result<(), ProviderError> {
    const ALLOWED: &[&str] = &["ids", "titles", "statuses", "digests", "bounded_summaries"];
    if privacy.source_bodies
        || privacy.raw_events
        || privacy.allowed_fields.len() != ALLOWED.len()
        || privacy
            .allowed_fields
            .iter()
            .enumerate()
            .any(|(index, field)| field != ALLOWED[index])
    {
        return Err(ProviderError::PrivacyViolation {
            message: "only the exact minimized field set without source bodies/raw events is allowed",
        });
    }
    Ok(())
}

fn enforce_payload_bounds(payload: &Value, budget: &RequestBudget) -> Result<(), ProviderError> {
    let items = projection_item_count(payload);
    if items > budget.max_items {
        return Err(ProviderError::BudgetExceeded {
            dimension: "items",
            actual: items,
            limit: budget.max_items,
        });
    }
    let tokens = value_token_count(payload);
    if tokens > budget.max_tokens {
        return Err(ProviderError::BudgetExceeded {
            dimension: "tokens",
            actual: tokens,
            limit: budget.max_tokens,
        });
    }
    let bytes = u64::try_from(to_canonical_vec(payload)?.len()).unwrap_or(u64::MAX);
    if bytes > budget.max_output_bytes {
        return Err(ProviderError::BudgetExceeded {
            dimension: "output_bytes",
            actual: bytes,
            limit: budget.max_output_bytes,
        });
    }
    Ok(())
}

fn projection_item_count(payload: &Value) -> u64 {
    let array_len = |value: &Value| value.as_array().map_or(0, |items| items.len() as u64);
    [
        &payload["authority_documents"],
        &payload["active_phases"],
        &payload["verified_completed_phases"],
        &payload["current_decisions"],
        &payload["open_blockers"],
        &payload["open_risks"],
        &payload["gates"]["pending"],
        &payload["gates"]["revalidation_required"],
        &payload["gates"]["stale"],
        &payload["gates"]["verified"],
        &payload["next_actions"]["ordered"],
        &payload["recent_material_events"],
        &payload["unknowns"],
        &payload["conflict_refs"],
    ]
    .into_iter()
    .map(array_len)
    .sum()
}

fn value_token_count(value: &Value) -> u64 {
    match value {
        Value::String(text) => text.split_whitespace().count().max(1) as u64,
        Value::Array(values) => values.iter().map(value_token_count).sum(),
        Value::Object(values) => values.values().map(value_token_count).sum(),
        Value::Null => 0,
        Value::Bool(_) | Value::Number(_) => 1,
    }
}

fn freshness_reason_codes(status: &str) -> Vec<&'static str> {
    match status {
        "CURRENT" => Vec::new(),
        "CURRENT_FOR_PROJECT_ONLY" => vec!["REPOSITORY_BINDING_UNAVAILABLE"],
        "STALE_FOR_REPOSITORY_STATE" => vec!["REPOSITORY_SNAPSHOT_MISMATCH"],
        "STALE_FOR_AUTHORITY_DOCUMENTS" => vec!["AUTHORITY_FRESHNESS_STALE"],
        "SUPERSEDED_BY_NEWER_CHECKPOINT" => vec!["EXPECTED_STATE_VERSION_MISMATCH"],
        "CONFLICTED" => vec!["EXPECTED_STATE_DIGEST_MISMATCH"],
        "INVALIDATED" => vec!["CHECKPOINT_INVALIDATED"],
        _ => vec!["FRESHNESS_UNKNOWN"],
    }
}
