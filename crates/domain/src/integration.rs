use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    CheckpointFreshnessStatus, DomainError, LedgerTimestamp, PhaseKind, PhaseStatus, ProjectId,
    ProjectLifecycleStatus, ProjectState, ResourceRef, ResumeProjection, SchemaVersion, StableId,
    common::closed_enum,
};

closed_enum! { pub enum IdentityMethod { ExplicitId, RemoteDigest, SignedBinding, UserConfirmed, ImportedMappingReviewed } }
closed_enum! { pub enum IdentityMappingStatus { Candidate, Confirmed, Revoked, Conflicted } }
closed_enum! { pub enum ProjectionCapability { ProjectContinuity, LedgerProjectSummary } }
closed_enum! { pub enum RecordCapability { RecordWorkProgress, RecordVerificationResult, RequestGateEvaluation, RequestGateRevalidation, CreateContinuityCheckpoint } }
closed_enum! { pub enum SideEffectLevel { None, DeclaredOnly } }
closed_enum! { pub enum ProviderHealthStatus { Healthy, Degraded, Unavailable, Incompatible } }
closed_enum! { pub enum FreshnessStatus { Current, Stale, Unavailable, Conflicted, Unknown } }
closed_enum! { pub enum LossSeverity { Informational, Material, Critical } }
closed_enum! { pub enum RadarCanonicalization { JcsRfc8785 } }
closed_enum! { pub enum SuggestedRadarStatus { Formalizing, SpecReady, InDevelopment, Testing, Auditing, Blocked, Paused, Done, Archived } }
closed_enum! { pub enum RadarProgressMethod { LinearRequiredPhases } }
closed_enum! { pub enum RadarProgressStatus { Complete, Partial, Unavailable } }
closed_enum! { pub enum RadarLinkStatus { Candidate, Confirmed, Revoked, Conflicted } }
closed_enum! { pub enum RadarFieldOwner { Ledger, Radar, SharedDisplay } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityMapping {
    pub schema_version: SchemaVersion,
    pub mapping_id: StableId,
    pub local_ref: ResourceRef,
    pub external_system: String,
    pub external_ref: ResourceRef,
    pub method: IdentityMethod,
    pub status: IdentityMappingStatus,
    pub confirmed_by: Option<StableId>,
    pub created_at: LedgerTimestamp,
    pub updated_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolDescriptor {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SideEffectContract {
    pub projection: SideEffectLevel,
    pub health: SideEffectLevel,
    pub record_submit: SideEffectLevel,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationStorage {
    pub shared_database_required: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationManifest {
    pub schema_version: SchemaVersion,
    pub integration_manifest_id: StableId,
    pub provider_id: String,
    pub provider_version: String,
    pub protocols: Vec<ProtocolDescriptor>,
    pub projection_capabilities: Vec<ProjectionCapability>,
    pub record_capabilities: Vec<RecordCapability>,
    pub side_effect_contract: SideEffectContract,
    pub storage: IntegrationStorage,
    pub upstream_locks: Vec<Sha256Digest>,
    pub digest: Sha256Digest,
}

impl IntegrationManifest {
    /// Validates the storage and side-effect boundaries of an S1 integration.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidContract`] when the provider requires shared
    /// storage or declares side effects outside the S1 contract.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.storage.shared_database_required
            || self.side_effect_contract.projection != SideEffectLevel::None
            || self.side_effect_contract.health != SideEffectLevel::None
            || self.side_effect_contract.record_submit != SideEffectLevel::DeclaredOnly
        {
            return Err(DomainError::InvalidContract {
                message: "integration manifest violates S1 side-effect or storage boundary",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionBudget {
    pub max_bytes: u64,
    pub max_items: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRequest {
    pub schema_version: SchemaVersion,
    pub request_id: StableId,
    pub protocol: ProtocolDescriptor,
    pub project_ref: ProjectId,
    pub repository_identity_digest: Option<Sha256Digest>,
    pub requested_fields: Vec<String>,
    pub budget: ProjectionBudget,
    pub privacy_profile: String,
    pub trace_ref: StableId,
    pub expected_source_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(
    tag = "projection_kind",
    content = "payload",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum ProjectionPayload {
    ProjectContinuity(ResumeProjection),
    ProjectState(ProjectState),
    LedgerProjectSummary(LedgerRadarProjectProjection),
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionEnvelope {
    pub schema_version: SchemaVersion,
    pub request_ref: StableId,
    pub profile: String,
    pub source_product: String,
    pub source_version: String,
    pub manifest_digest: Sha256Digest,
    pub project_ref: ProjectId,
    pub state_version: u64,
    pub state_digest: Sha256Digest,
    pub payload_schema_ref: ResourceRef,
    pub payload_digest: Sha256Digest,
    pub freshness: FreshnessStatus,
    pub coverage: Vec<String>,
    pub omissions: Vec<String>,
    pub privacy_profile: String,
    pub provenance_ref: ResourceRef,
    pub payload: ProjectionPayload,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderHealthReport {
    pub schema_version: SchemaVersion,
    pub provider_id: String,
    pub provider_version: String,
    pub status: ProviderHealthStatus,
    pub checked_at: LedgerTimestamp,
    pub supported_protocols: Vec<ProtocolDescriptor>,
    pub explanation_codes: Vec<String>,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LossItem {
    pub field_path: String,
    pub severity: LossSeverity,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LossReport {
    pub schema_version: SchemaVersion,
    pub loss_report_id: StableId,
    pub source_schema_ref: ResourceRef,
    pub target_schema_ref: ResourceRef,
    pub items: Vec<LossItem>,
    pub material_loss_count: u32,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarSource {
    pub system: String,
    pub product_version: String,
    pub project_ref: ResourceRef,
    pub state_version: u64,
    pub state_digest: Sha256Digest,
    pub checkpoint_ref: Option<ResourceRef>,
    pub checkpoint_digest: Option<Sha256Digest>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarIdentity {
    pub ledger_project_ref: ResourceRef,
    pub stable_project_id: ProjectId,
    pub repository_identity_digests: Vec<Sha256Digest>,
    pub source_aliases: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarProjectSummary {
    pub canonical_title: String,
    pub bounded_summary: String,
    pub lifecycle_status: ProjectLifecycleStatus,
    pub suggested_radar_status: SuggestedRadarStatus,
    pub status_mapping_rule: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarPhaseSummary {
    pub phase_ref: StableId,
    pub title: String,
    pub kind: PhaseKind,
    pub status: PhaseStatus,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarPercentage {
    pub available: bool,
    pub basis_points: Option<u16>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarProgress {
    pub method: RadarProgressMethod,
    pub completed_required_phases: u32,
    pub total_required_phases: u32,
    pub verified_required_gates: u32,
    pub total_required_gates: u32,
    pub percentage: RadarPercentage,
    pub status: RadarProgressStatus,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarCounts {
    pub open_count: u32,
    pub critical_count: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarGateCounts {
    pub pending_count: u32,
    pub revalidation_required_count: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarConflictCounts {
    pub critical_open_count: u32,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarContinuity {
    pub primary_phase: Option<RadarPhaseSummary>,
    pub active_phases: Vec<RadarPhaseSummary>,
    pub progress: RadarProgress,
    pub blockers: RadarCounts,
    pub risks: RadarCounts,
    pub gates: RadarGateCounts,
    pub conflicts: RadarConflictCounts,
    pub next_action_refs: Vec<StableId>,
    pub preferred_next_action_ref: Option<StableId>,
    pub last_material_activity_at: Option<LedgerTimestamp>,
    pub checkpoint_freshness: CheckpointFreshnessStatus,
    pub authority_health: FreshnessStatus,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarAttentionSignal {
    pub code: String,
    pub subject_ref: StableId,
    pub severity: crate::Severity,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarFreshness {
    pub status: FreshnessStatus,
    pub evaluated_at: LedgerTimestamp,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarPrivacy {
    pub event_log_included: bool,
    pub evidence_payloads_included: bool,
    pub repository_paths_included: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerRadarProjectProjection {
    pub schema_version: SchemaVersion,
    pub projection_id: StableId,
    pub projection_kind: String,
    pub source: RadarSource,
    pub identity: RadarIdentity,
    pub project: RadarProjectSummary,
    pub continuity: RadarContinuity,
    pub attention_signals: Vec<RadarAttentionSignal>,
    pub freshness: RadarFreshness,
    pub privacy: RadarPrivacy,
    pub created_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarSyncContract {
    pub schema_id: String,
    pub profile_schema_digest: Sha256Digest,
    pub source_spec_digest: Sha256Digest,
    pub canonicalization: RadarCanonicalization,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarSyncSemantics {
    pub omission_means_delete: bool,
    pub imports_are_idempotent: bool,
    pub name_only_mapping_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarSyncEnvelope {
    pub schema_version: SchemaVersion,
    pub sync_envelope_id: StableId,
    pub profile: String,
    pub source_instance_ref: ResourceRef,
    pub exported_at: LedgerTimestamp,
    pub projects: Vec<LedgerRadarProjectProjection>,
    pub tombstones: Vec<StableId>,
    pub contract: RadarSyncContract,
    pub semantics: RadarSyncSemantics,
    pub digest: Sha256Digest,
}

impl RadarSyncEnvelope {
    /// Validates the locked Radar artifact profile and its merge semantics.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidContract`] when the envelope diverges from
    /// profile 0.1.1, permits destructive omission, or contains tombstones.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != SchemaVersion::V0_1_1
            || self.profile != "project-ledger-radar-artifact/0.1.1"
            || self.semantics.omission_means_delete
            || !self.semantics.imports_are_idempotent
            || self.semantics.name_only_mapping_allowed
            || !self.tombstones.is_empty()
        {
            return Err(DomainError::InvalidContract {
                message: "Radar sync semantics violate profile 0.1.1",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarProjectLink {
    pub schema_version: SchemaVersion,
    pub link_id: StableId,
    pub ledger_project_ref: ResourceRef,
    pub radar_project_ref: ResourceRef,
    pub status: RadarLinkStatus,
    pub method: IdentityMethod,
    pub actor_ref: StableId,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarStatusMappingRule {
    pub ledger_status: ProjectLifecycleStatus,
    pub phase_kind: Option<PhaseKind>,
    pub radar_status: SuggestedRadarStatus,
    pub rule_code: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarStatusMappingPolicy {
    pub schema_version: SchemaVersion,
    pub policy_id: StableId,
    pub rules: Vec<RadarStatusMappingRule>,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarFieldOwnershipRule {
    pub field_path: String,
    pub owner: RadarFieldOwner,
    pub omission_clears: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarFieldOwnershipPolicy {
    pub schema_version: SchemaVersion,
    pub policy_id: StableId,
    pub rules: Vec<RadarFieldOwnershipRule>,
    pub digest: Sha256Digest,
}
