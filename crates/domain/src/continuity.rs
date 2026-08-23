use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    DomainError, EventId, LedgerTimestamp, ProjectId, RepositorySnapshot, ResourceRef,
    SchemaVersion, StableId, common::closed_enum,
};

closed_enum! {
    pub enum ConflictKind {
        AuthorityDocumentDivergence,
        ActivePhaseDivergence,
        GateStateDivergence,
        DecisionDivergence,
        CheckpointStale,
        RepositoryBindingStale,
        LegacyImportContradiction,
        ExternalVersionConflict,
        SourceDivergence,
        StreamDivergence
    }
}
closed_enum! { pub enum ConflictStatus { Open, Resolved, Accepted, Superseded } }
closed_enum! { pub enum CheckpointFreshnessStatus { Current, CurrentForProjectOnly, StaleForRepository, StaleForAuthority, SupersededByNewerCheckpoint, Conflicted, Invalidated, Unknown } }
closed_enum! { pub enum LegacyImportMode { Preview, ReviewedImport, ArchiveOnly } }
closed_enum! { pub enum LegacyCandidateAction { Review, ImportAsAsserted, Ignore, ArchiveOnly } }
closed_enum! { pub enum GeneratedArtifactKind { Markdown, PortableBundle, RadarExport, Report } }
closed_enum! { pub enum PortableImportPolicy { IdenticalChain, FastForward, NewProject, ForkExplicit } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictRecord {
    pub schema_version: SchemaVersion,
    pub conflict_id: StableId,
    pub kind: ConflictKind,
    pub severity: crate::Severity,
    pub domain: String,
    pub subject_refs: Vec<StableId>,
    pub source_refs: Vec<ResourceRef>,
    pub status: ConflictStatus,
    pub required_action: String,
    pub opened_at: LedgerTimestamp,
    pub resolved_at: Option<LedgerTimestamp>,
    pub resolution_ref: Option<ResourceRef>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub schema_version: SchemaVersion,
    pub session_id: StableId,
    pub actor_ref: StableId,
    pub host: String,
    pub started_at: LedgerTimestamp,
    pub ended_at: Option<LedgerTimestamp>,
    pub starting_checkpoint_ref: Option<StableId>,
    pub ending_checkpoint_ref: Option<StableId>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuityCheckpoint {
    pub schema_version: SchemaVersion,
    pub checkpoint_id: StableId,
    pub project_ref: ProjectId,
    pub captured_state_version: u64,
    pub captured_material_state_version: u64,
    pub captured_state_digest: Sha256Digest,
    pub checkpoint_event_version: u64,
    pub checkpoint_event_ref: EventId,
    pub authority_document_refs: Vec<StableId>,
    pub active_phase_refs: Vec<StableId>,
    pub primary_active_phase_ref: Option<StableId>,
    pub verified_completed_phase_refs: Vec<StableId>,
    pub pending_gate_refs: Vec<StableId>,
    pub current_decision_refs: Vec<StableId>,
    pub open_blocker_refs: Vec<StableId>,
    pub open_risk_refs: Vec<StableId>,
    pub next_action_refs: Vec<StableId>,
    pub last_material_event_refs: Vec<EventId>,
    pub repository_binding: Option<RepositorySnapshot>,
    pub unknown_refs: Vec<StableId>,
    pub conflict_refs: Vec<StableId>,
    pub created_by: StableId,
    pub created_at: LedgerTimestamp,
    pub checkpoint_digest: Sha256Digest,
}

impl ContinuityCheckpoint {
    /// Enforces the non-circular capture-N/register-N+1 checkpoint invariant.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidContract`] when the checkpoint event is not
    /// exactly the next version or the material version exceeds the captured one.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.checkpoint_event_version != self.captured_state_version.saturating_add(1) {
            return Err(DomainError::InvalidContract {
                message: "checkpoint_event_version must equal captured_state_version + 1",
            });
        }
        if self.captured_material_state_version > self.captured_state_version {
            return Err(DomainError::InvalidContract {
                message: "material state version cannot exceed captured state version",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointFreshnessRecord {
    pub schema_version: SchemaVersion,
    pub freshness_record_id: StableId,
    pub checkpoint_ref: StableId,
    pub status: CheckpointFreshnessStatus,
    pub evaluated_at: LedgerTimestamp,
    pub reason_codes: Vec<String>,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeProjection {
    pub schema_version: SchemaVersion,
    pub project_ref: ProjectId,
    pub state_version: u64,
    pub state_digest: Sha256Digest,
    pub checkpoint_freshness: CheckpointFreshnessStatus,
    pub authority_document_refs: Vec<StableId>,
    pub active_phase_refs: Vec<StableId>,
    pub completed_phase_refs: Vec<StableId>,
    pub current_decision_refs: Vec<StableId>,
    pub open_blocker_refs: Vec<StableId>,
    pub open_risk_refs: Vec<StableId>,
    pub pending_gate_refs: Vec<StableId>,
    pub last_material_progress_summary: Option<String>,
    pub preferred_next_action_ref: Option<StableId>,
    pub critical_unknown_refs: Vec<StableId>,
    pub conflict_refs: Vec<StableId>,
    pub checkpoint_ref: Option<StableId>,
    pub rendered_token_count: u32,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LineRange(pub [u32; 2]);

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyCandidateSource {
    pub path: String,
    pub section: String,
    pub line_range: LineRange,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyImportCandidate {
    pub schema_version: SchemaVersion,
    pub candidate_id: StableId,
    pub kind: String,
    pub value: String,
    pub source: LegacyCandidateSource,
    pub conflicts_with: Vec<StableId>,
    pub recommended_action: LegacyCandidateAction,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyImportDecision {
    pub candidate_ref: StableId,
    pub action: LegacyCandidateAction,
    pub actor_ref: StableId,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyImportReview {
    pub schema_version: SchemaVersion,
    pub review_id: StableId,
    pub source_uri: ResourceRef,
    pub source_digest: Sha256Digest,
    pub mode: LegacyImportMode,
    pub candidates: Vec<LegacyImportCandidate>,
    pub decisions: Vec<LegacyImportDecision>,
    pub reviewed_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedArtifactRecord {
    pub schema_version: SchemaVersion,
    pub generated_artifact_id: StableId,
    pub kind: GeneratedArtifactKind,
    pub uri: ResourceRef,
    pub source_state_version: u64,
    pub source_state_digest: Sha256Digest,
    pub renderer_version: String,
    pub privacy_profile: String,
    pub artifact_digest: Sha256Digest,
    pub created_at: LedgerTimestamp,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableExport {
    pub schema_version: SchemaVersion,
    pub export_id: StableId,
    pub project_ref: ProjectId,
    pub project_state_version: u64,
    pub project_state_digest: Sha256Digest,
    pub first_event_ref: EventId,
    pub last_event_ref: EventId,
    pub event_count: u64,
    pub chain_digest: Sha256Digest,
    pub import_policy: PortableImportPolicy,
    pub schemas_lock_digest: Sha256Digest,
    pub checksums_digest: Sha256Digest,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}
