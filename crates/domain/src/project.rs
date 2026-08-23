use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    EventId, LedgerTimestamp, ProjectId, ResourceRef, SchemaVersion, StableId, common::closed_enum,
};

closed_enum! { pub enum ProjectLifecycleStatus { Initializing, Active, Paused, Blocked, Completed, Archived } }
closed_enum! { pub enum AuthorityPolicy { SpecFirst } }
closed_enum! { pub enum StorageProfile { UserData, Portable } }
closed_enum! { pub enum AuthorityDocumentStatus { Current, Draft, Legacy, Deprecated, Superseded, Unavailable, DigestMismatch } }
closed_enum! { pub enum RoadmapExecutionModel { Sequential } }
closed_enum! { pub enum RoadmapCompletionPolicy { AllRequiredPhases } }
closed_enum! { pub enum PhaseKind { Discovery, Formalization, Implementation, Testing, Audit, Release, Maintenance, Other } }
closed_enum! { pub enum PhaseStatus { NotStarted, Active, Blocked, ReadyToClose, Completed, RevalidationRequired, Skipped } }
closed_enum! { pub enum GateStatus { Defined, Pending, Evaluating, Verified, Rejected, Waived, Stale, RevalidationRequired } }
closed_enum! { pub enum DecisionStatus { Proposed, Accepted, Superseded, Rejected, Deprecated, RevalidationRequired } }
closed_enum! { pub enum BlockerStatus { Open, Mitigated, Resolved, Accepted, Cancelled } }
closed_enum! { pub enum RiskStatus { Open, Monitored, Mitigated, Realized, Closed } }
closed_enum! { pub enum Severity { Low, Medium, High, Critical } }
closed_enum! { pub enum LikelihoodClass { Rare, Possible, Likely, Unknown } }
closed_enum! { pub enum ScopeKind { Project, Phase, Gate, Decision, Repository, External } }
closed_enum! { pub enum NextActionClassification { Authorized, Eligible, Recommended, Proposed, Blocked, Completed, Cancelled } }
closed_enum! { pub enum ArtifactKind { Spec, Audit, Build, Report, Package, Repository, Other } }
closed_enum! { pub enum Sensitivity { Public, Internal, Sensitive, Secret } }
closed_enum! { pub enum RepositoryIdentityMethod { ExplicitId, RemoteDigest, SignedBinding, UserConfirmed } }
closed_enum! { pub enum RepositoryBindingStatus { Current, Stale, Unavailable, Revoked, Conflicted } }
closed_enum! { pub enum ClaimClassification { Asserted, Observed, Verified, Rejected, Stale, Unknown } }
closed_enum! { pub enum UnknownCriticality { Low, Medium, High, Material } }
closed_enum! { pub enum UnknownStatus { Open, Resolved, AcceptedUnknown, Superseded } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub project_id: ProjectId,
    pub title: String,
    pub summary: String,
    pub lifecycle_status: ProjectLifecycleStatus,
    pub created_at: LedgerTimestamp,
    pub timezone: String,
    pub authority_policy: AuthorityPolicy,
    pub storage_profile: StorageProfile,
    pub repository_binding_refs: Vec<StableId>,
    pub paused_reason: Option<String>,
    pub archived_at: Option<LedgerTimestamp>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDocument {
    pub schema_version: SchemaVersion,
    pub document_id: StableId,
    pub uri: ResourceRef,
    pub digest: Sha256Digest,
    pub version: String,
    pub status: AuthorityDocumentStatus,
    pub authority_rank: u32,
    pub effective_from: LedgerTimestamp,
    pub supersedes_ref: Option<StableId>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Roadmap {
    pub schema_version: SchemaVersion,
    pub roadmap_id: StableId,
    pub title: String,
    pub execution_model: RoadmapExecutionModel,
    pub phase_order: Vec<StableId>,
    pub completion_policy: RoadmapCompletionPolicy,
    pub version: u64,
    pub source_document_ref: StableId,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub schema_version: SchemaVersion,
    pub phase_id: StableId,
    pub title: String,
    pub ordinal: u32,
    pub kind: PhaseKind,
    pub status: PhaseStatus,
    pub required_gate_refs: Vec<StableId>,
    pub optional_gate_refs: Vec<StableId>,
    pub started_at: Option<LedgerTimestamp>,
    pub completed_at: Option<LedgerTimestamp>,
    pub reopened_at: Option<LedgerTimestamp>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub schema_version: SchemaVersion,
    pub decision_id: StableId,
    pub title: String,
    pub domain: String,
    pub scope: String,
    pub context: String,
    pub decision: String,
    pub rationale: String,
    pub alternatives: Vec<String>,
    pub status: DecisionStatus,
    pub version: u64,
    pub authority_ref: Option<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub supersedes_ref: Option<StableId>,
    pub created_at: LedgerTimestamp,
    pub accepted_at: Option<LedgerTimestamp>,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRecord {
    pub schema_version: SchemaVersion,
    pub work_record_id: StableId,
    pub summary: String,
    pub phase_ref: Option<StableId>,
    pub task_ref: Option<StableId>,
    pub files_or_artifacts: Vec<ResourceRef>,
    pub commit_refs: Vec<String>,
    pub claim_refs: Vec<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub actor_ref: StableId,
    pub influence: Option<String>,
    pub occurred_at: LedgerTimestamp,
    pub recorded_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Blocker {
    pub schema_version: SchemaVersion,
    pub blocker_id: StableId,
    pub title: String,
    pub description: String,
    pub scope_kind: ScopeKind,
    pub scope_ref: StableId,
    pub severity: Severity,
    pub status: BlockerStatus,
    pub source_kind: String,
    pub source_ref: Option<ResourceRef>,
    pub owner_ref: Option<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub opened_at: LedgerTimestamp,
    pub updated_at: LedgerTimestamp,
    pub resolved_at: Option<LedgerTimestamp>,
    pub resolution_summary: Option<String>,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Risk {
    pub schema_version: SchemaVersion,
    pub risk_id: StableId,
    pub title: String,
    pub description: String,
    pub scope_kind: ScopeKind,
    pub scope_ref: StableId,
    pub severity: Severity,
    pub likelihood_class: LikelihoodClass,
    pub status: RiskStatus,
    pub owner_ref: Option<StableId>,
    pub trigger_conditions: Vec<String>,
    pub mitigation_actions: Vec<String>,
    pub evidence_refs: Vec<StableId>,
    pub opened_at: LedgerTimestamp,
    pub updated_at: LedgerTimestamp,
    pub closed_at: Option<LedgerTimestamp>,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NextAction {
    pub schema_version: SchemaVersion,
    pub next_action_id: StableId,
    pub title: String,
    pub description: String,
    pub phase_ref: Option<StableId>,
    pub gate_ref: Option<StableId>,
    pub classification: NextActionClassification,
    pub authority_ref: Option<StableId>,
    pub precondition_refs: Vec<StableId>,
    pub blocked_by_refs: Vec<StableId>,
    pub owner_ref: Option<StableId>,
    pub created_at: LedgerTimestamp,
    pub updated_at: LedgerTimestamp,
    pub completed_at: Option<LedgerTimestamp>,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    pub schema_version: SchemaVersion,
    pub artifact_ref_id: StableId,
    pub name: String,
    pub kind: ArtifactKind,
    pub uri: ResourceRef,
    pub digest: Sha256Digest,
    pub version: Option<String>,
    pub sensitivity: Sensitivity,
    pub source_ref: Option<ResourceRef>,
    pub created_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBinding {
    pub schema_version: SchemaVersion,
    pub repository_binding_id: StableId,
    pub repository_ref: StableId,
    pub workspace_root_ref: ResourceRef,
    pub identity_method: RepositoryIdentityMethod,
    pub remote_identity_digest: Option<Sha256Digest>,
    pub current_snapshot_ref: Option<StableId>,
    pub current_snapshot_digest: Option<Sha256Digest>,
    pub status: RepositoryBindingStatus,
    pub confirmed_by: Option<StableId>,
    pub created_at: LedgerTimestamp,
    pub updated_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRecord {
    pub schema_version: SchemaVersion,
    pub claim_id: StableId,
    pub statement: String,
    pub classification: ClaimClassification,
    pub subject_refs: Vec<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub authority_ref: Option<StableId>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownRecord {
    pub schema_version: SchemaVersion,
    pub unknown_id: StableId,
    pub subject_kind: String,
    pub subject_ref: StableId,
    pub field_path: String,
    pub statement: String,
    pub criticality: UnknownCriticality,
    pub status: UnknownStatus,
    pub blocks: Vec<StableId>,
    pub required_evidence: Vec<String>,
    pub evidence_refs: Vec<StableId>,
    pub opened_at: LedgerTimestamp,
    pub resolved_at: Option<LedgerTimestamp>,
    pub resolution_ref: Option<ResourceRef>,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCatalogEntry {
    pub schema_version: SchemaVersion,
    pub project_ref: ResourceRef,
    pub canonical_title: String,
    pub ledger_location_ref: ResourceRef,
    pub last_seen_state_version: u64,
    pub last_seen_state_digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectState {
    pub schema_version: SchemaVersion,
    pub project_state_id: StableId,
    pub project: Project,
    pub state_version: u64,
    pub material_state_version: u64,
    pub authority_documents: Vec<AuthorityDocument>,
    pub roadmap: Option<Roadmap>,
    pub active_phase_refs: Vec<StableId>,
    pub primary_active_phase_ref: Option<StableId>,
    pub completed_phase_refs: Vec<StableId>,
    pub pending_gate_refs: Vec<StableId>,
    pub verified_gate_refs: Vec<StableId>,
    pub revalidation_required_gate_refs: Vec<StableId>,
    pub current_decision_refs: Vec<StableId>,
    pub open_blocker_refs: Vec<StableId>,
    pub open_risk_refs: Vec<StableId>,
    pub ordered_next_action_refs: Vec<StableId>,
    pub preferred_next_action_ref: Option<StableId>,
    pub checkpoint_ref: Option<StableId>,
    pub last_material_event_ref: Option<EventId>,
    pub last_material_activity_at: Option<LedgerTimestamp>,
    pub open_conflict_refs: Vec<StableId>,
    pub updated_at: LedgerTimestamp,
    pub state_digest: Sha256Digest,
}
