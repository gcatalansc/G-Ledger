use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Actor, ArtifactReference, AuthorityDocument, Blocker, CheckpointFreshnessStatus,
    ClaimClassification, ClaimRecord, CommandId, ConflictRecord, ContinuityCheckpoint, Decision,
    EventId, EvidenceRecord, GateEvaluation, GateStatus, GateWaiver, IdentityMapping,
    LedgerTimestamp, LegacyImportReview, NextAction, PhaseStatus, Project, ProjectId,
    ProjectLifecycleStatus, RepositoryBinding, RepositorySnapshot, ResourceRef, Risk, Roadmap,
    SchemaVersion, Session, StableId, UnknownRecord, WorkRecord,
};

macro_rules! payload {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

payload!(ProjectCreatedEvent { project: Project });
payload!(ProjectMetadataUpdatedEvent { title: Option<String>, summary: Option<String> });
payload!(ProjectPausedEvent { reason: String });
payload!(ProjectResumedEvent { reason: String });
payload!(ProjectArchivedEvent { reason: String });
payload!(ProjectStatusDerivedEvent { previous_status: ProjectLifecycleStatus, new_status: ProjectLifecycleStatus, reason_codes: Vec<String> });
payload!(AuthorityDocumentRegisteredEvent {
    document: AuthorityDocument
});
payload!(AuthorityDocumentDigestRefreshedEvent {
    document_ref: StableId,
    observed_digest: Sha256Digest
});
payload!(AuthorityDocumentSupersededEvent {
    document_ref: StableId,
    replacement_ref: StableId
});
payload!(AuthorityDocumentMarkedUnavailableEvent {
    document_ref: StableId,
    reason: String
});
payload!(AuthorityDocumentDigestMismatchEvent {
    document_ref: StableId,
    expected_digest: Sha256Digest,
    observed_digest: Sha256Digest
});
payload!(RepositoryBoundEvent {
    binding: RepositoryBinding
});
payload!(RepositoryBindingRefreshedEvent {
    binding_ref: StableId,
    snapshot: RepositorySnapshot
});
payload!(RepositoryBindingRevokedEvent {
    binding_ref: StableId,
    reason: String
});
payload!(RoadmapDefinedEvent { roadmap: Roadmap });
payload!(RoadmapRevisedEvent { roadmap: Roadmap });
payload!(PhaseStartedEvent {
    phase_ref: StableId
});
payload!(PhaseBlockedEvent { phase_ref: StableId, blocker_refs: Vec<StableId> });
payload!(PhaseMarkedReadyToCloseEvent {
    phase_ref: StableId,
    pending_approval: bool
});
payload!(PhaseCompletedEvent { phase_ref: StableId, gate_evaluation_refs: Vec<StableId> });
payload!(PhaseReopenedEvent {
    phase_ref: StableId,
    resulting_status: PhaseStatus,
    reason: String
});
payload!(PhaseSkippedEvent {
    phase_ref: StableId,
    authority_ref: StableId,
    reason: String
});
payload!(GateDefinedEvent {
    gate: crate::GateDefinition
});
payload!(GateEvaluationRequestedEvent { gate_ref: StableId });
payload!(GateEvaluatedEvent {
    evaluation: GateEvaluation
});
payload!(GateVerifiedEvent {
    gate_ref: StableId,
    evaluation_ref: StableId
});
payload!(GateRejectedEvent {
    gate_ref: StableId,
    evaluation_ref: StableId
});
payload!(GateWaivedEvent { waiver: GateWaiver });
payload!(GateMarkedStaleEvent { gate_ref: StableId, reason_codes: Vec<String> });
payload!(GateRevalidationRequestedEvent { gate_ref: StableId, signal_ref: Option<ResourceRef> });
payload!(GateRevalidationRequiredEvent { gate_ref: StableId, prior_status: GateStatus, reason_codes: Vec<String> });
payload!(WorkProgressRecordedEvent {
    work_record: WorkRecord
});
payload!(NoteRecordedEvent { note_id: StableId, kind: String, text: String, scope_ref: Option<StableId> });
payload!(ArtifactReferenceAddedEvent {
    artifact: ArtifactReference
});
payload!(ClaimRecordedEvent { claim: ClaimRecord });
payload!(ClaimStatusChangedEvent { claim_ref: StableId, prior_status: ClaimClassification, new_status: ClaimClassification, evidence_refs: Vec<StableId> });
payload!(UnknownRecordedEvent {
    unknown: UnknownRecord
});
payload!(UnknownResolvedEvent {
    unknown_ref: StableId,
    resolution_ref: ResourceRef
});
payload!(VerificationRecordedEvent {
    evidence: EvidenceRecord
});
payload!(ExternalEvidenceLinkedEvent {
    evidence_ref: StableId,
    external_system: String,
    artifact_uri: ResourceRef,
    artifact_digest: Sha256Digest,
    schema_ref: ResourceRef
});
payload!(ExternalEvidenceUnlinkedEvent {
    evidence_ref: StableId,
    reason: String
});
payload!(EvidenceInvalidatedEvent {
    evidence_ref: StableId,
    reason: String
});
payload!(DecisionProposedEvent { decision: Decision });
payload!(DecisionAcceptedEvent {
    decision_ref: StableId,
    authority_ref: StableId
});
payload!(DecisionSupersededEvent {
    decision_ref: StableId,
    replacement_ref: StableId
});
payload!(DecisionRejectedEvent {
    decision_ref: StableId,
    reason: String
});
payload!(DecisionRevalidationRequiredEvent { decision_ref: StableId, reason_codes: Vec<String> });
payload!(BlockerOpenedEvent { blocker: Blocker });
payload!(BlockerUpdatedEvent { blocker: Blocker });
payload!(BlockerResolvedEvent { blocker_ref: StableId, resolution_summary: String, evidence_refs: Vec<StableId> });
payload!(RiskOpenedEvent { risk: Risk });
payload!(RiskUpdatedEvent { risk: Risk });
payload!(RiskMitigatedEvent { risk_ref: StableId, note: String, evidence_refs: Vec<StableId> });
payload!(RiskClosedEvent { risk_ref: StableId, note: String, evidence_refs: Vec<StableId> });
payload!(NextActionSetEvent { action: NextAction });
payload!(NextActionCompletedEvent { action_ref: StableId, evidence_refs: Vec<StableId> });
payload!(NextActionCancelledEvent {
    action_ref: StableId,
    reason: String
});
payload!(CheckpointCreatedEvent {
    checkpoint: ContinuityCheckpoint
});
payload!(CheckpointInvalidatedEvent { checkpoint_ref: StableId, freshness: CheckpointFreshnessStatus, reason_codes: Vec<String> });
payload!(SessionStartedEvent { session: Session });
payload!(SessionEndedEvent { session_ref: StableId, ending_checkpoint_ref: Option<StableId> });
payload!(LegacyImportReviewedEvent {
    review: LegacyImportReview
});
payload!(ConflictOpenedEvent {
    conflict: ConflictRecord
});
payload!(ConflictResolvedEvent {
    conflict_ref: StableId,
    resolution_ref: ResourceRef,
    reason: String
});
payload!(ExternalMappingConfirmedEvent {
    mapping: IdentityMapping
});
payload!(ExternalMappingRevokedEvent {
    mapping_ref: StableId,
    reason: String
});
payload!(EventCorrectionRecordedEvent { corrects_event_ref: EventId, reason: String, replacement_field_values: Vec<crate::CorrectionField> });

macro_rules! event_catalog {
    ($($variant:ident($payload:ty)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum EventType { $($variant),+ }

        impl EventType {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }

        #[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
        #[serde(tag = "event_type", content = "payload", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
        pub enum Event { $($variant($payload)),+ }

        impl Event {
            pub const fn event_type(&self) -> EventType {
                match self { $(Self::$variant(_) => EventType::$variant),+ }
            }
        }
    };
}

event_catalog! {
    ProjectCreated(ProjectCreatedEvent),
    ProjectMetadataUpdated(ProjectMetadataUpdatedEvent),
    ProjectPaused(ProjectPausedEvent),
    ProjectResumed(ProjectResumedEvent),
    ProjectArchived(ProjectArchivedEvent),
    ProjectStatusDerived(ProjectStatusDerivedEvent),
    AuthorityDocumentRegistered(AuthorityDocumentRegisteredEvent),
    AuthorityDocumentDigestRefreshed(AuthorityDocumentDigestRefreshedEvent),
    AuthorityDocumentSuperseded(AuthorityDocumentSupersededEvent),
    AuthorityDocumentMarkedUnavailable(AuthorityDocumentMarkedUnavailableEvent),
    AuthorityDocumentDigestMismatch(AuthorityDocumentDigestMismatchEvent),
    RepositoryBound(RepositoryBoundEvent),
    RepositoryBindingRefreshed(RepositoryBindingRefreshedEvent),
    RepositoryBindingRevoked(RepositoryBindingRevokedEvent),
    RoadmapDefined(RoadmapDefinedEvent),
    RoadmapRevised(RoadmapRevisedEvent),
    PhaseStarted(PhaseStartedEvent),
    PhaseBlocked(PhaseBlockedEvent),
    PhaseMarkedReadyToClose(PhaseMarkedReadyToCloseEvent),
    PhaseCompleted(PhaseCompletedEvent),
    PhaseReopened(PhaseReopenedEvent),
    PhaseSkipped(PhaseSkippedEvent),
    GateDefined(GateDefinedEvent),
    GateEvaluationRequested(GateEvaluationRequestedEvent),
    GateEvaluated(GateEvaluatedEvent),
    GateVerified(GateVerifiedEvent),
    GateRejected(GateRejectedEvent),
    GateWaived(GateWaivedEvent),
    GateMarkedStale(GateMarkedStaleEvent),
    GateRevalidationRequested(GateRevalidationRequestedEvent),
    GateRevalidationRequired(GateRevalidationRequiredEvent),
    WorkProgressRecorded(WorkProgressRecordedEvent),
    NoteRecorded(NoteRecordedEvent),
    ArtifactReferenceAdded(ArtifactReferenceAddedEvent),
    ClaimRecorded(ClaimRecordedEvent),
    ClaimStatusChanged(ClaimStatusChangedEvent),
    UnknownRecorded(UnknownRecordedEvent),
    UnknownResolved(UnknownResolvedEvent),
    VerificationRecorded(VerificationRecordedEvent),
    ExternalEvidenceLinked(ExternalEvidenceLinkedEvent),
    ExternalEvidenceUnlinked(ExternalEvidenceUnlinkedEvent),
    EvidenceInvalidated(EvidenceInvalidatedEvent),
    DecisionProposed(DecisionProposedEvent),
    DecisionAccepted(DecisionAcceptedEvent),
    DecisionSuperseded(DecisionSupersededEvent),
    DecisionRejected(DecisionRejectedEvent),
    DecisionRevalidationRequired(DecisionRevalidationRequiredEvent),
    BlockerOpened(BlockerOpenedEvent),
    BlockerUpdated(BlockerUpdatedEvent),
    BlockerResolved(BlockerResolvedEvent),
    RiskOpened(RiskOpenedEvent),
    RiskUpdated(RiskUpdatedEvent),
    RiskMitigated(RiskMitigatedEvent),
    RiskClosed(RiskClosedEvent),
    NextActionSet(NextActionSetEvent),
    NextActionCompleted(NextActionCompletedEvent),
    NextActionCancelled(NextActionCancelledEvent),
    CheckpointCreated(CheckpointCreatedEvent),
    CheckpointInvalidated(CheckpointInvalidatedEvent),
    SessionStarted(SessionStartedEvent),
    SessionEnded(SessionEndedEvent),
    LegacyImportReviewed(LegacyImportReviewedEvent),
    ConflictOpened(ConflictOpenedEvent),
    ConflictResolved(ConflictResolvedEvent),
    ExternalMappingConfirmed(ExternalMappingConfirmedEvent),
    ExternalMappingRevoked(ExternalMappingRevokedEvent),
    EventCorrectionRecorded(EventCorrectionRecordedEvent)
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventCausation {
    pub command_ref: CommandId,
    pub command_digest: Sha256Digest,
    pub correlation_ref: StableId,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub schema_version: SchemaVersion,
    pub event_id: EventId,
    pub project_ref: ProjectId,
    pub stream_id: ProjectId,
    pub stream_version: u64,
    #[serde(flatten)]
    pub event: Event,
    pub event_schema_ref: ResourceRef,
    pub actor: Actor,
    pub causation: EventCausation,
    pub occurred_at: LedgerTimestamp,
    pub recorded_at: LedgerTimestamp,
    pub previous_event_digest: Option<Sha256Digest>,
    pub payload_digest: Sha256Digest,
    pub event_digest: Sha256Digest,
}

impl EventEnvelope {
    pub const fn event_type(&self) -> EventType {
        self.event.event_type()
    }
}
