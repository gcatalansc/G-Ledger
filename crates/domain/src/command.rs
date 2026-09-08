use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    Actor, ArtifactReference, AuthorityDocument, Blocker, ClaimClassification, ClaimRecord,
    CommandId, Decision, ErrorCode, EvidenceRecord, GateDefinition, GateWaiver, IdentityMapping,
    LedgerTimestamp, NextAction, Project, ProjectId, RepositoryBinding, RepositorySnapshot,
    ResourceRef, Risk, Roadmap, ScalarValue, SchemaVersion, Session, StableId, UnknownRecord,
    WorkRecord, common::closed_enum,
};

macro_rules! payload {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

payload!(CreateProjectPayload { project: Project });
payload!(UpdateProjectMetadataPayload { title: Option<String>, summary: Option<String> });
payload!(PauseProjectPayload { reason: String });
payload!(ResumeProjectPayload { reason: String });
payload!(ArchiveProjectPayload { reason: String });
payload!(RegisterAuthorityDocumentPayload {
    document: AuthorityDocument
});

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum AuthorityDigestObservation {
    Match { observed_digest: Sha256Digest },
    Unavailable { reason: String },
    Mismatch { observed_digest: Sha256Digest },
}

payload!(RefreshAuthorityDocumentDigestPayload {
    document_ref: StableId,
    observation: AuthorityDigestObservation
});
payload!(SupersedeAuthorityDocumentPayload {
    document_ref: StableId,
    replacement: AuthorityDocument
});
payload!(BindRepositoryPayload {
    binding: RepositoryBinding
});
payload!(RefreshRepositoryBindingPayload {
    binding_ref: StableId,
    snapshot: RepositorySnapshot
});
payload!(RevokeRepositoryBindingPayload {
    binding_ref: StableId,
    reason: String
});
payload!(DefineRoadmapPayload { roadmap: Roadmap });
payload!(ReviseRoadmapPayload { roadmap: Roadmap });
payload!(StartPhasePayload {
    phase_ref: StableId
});
payload!(RequestPhaseCompletionPayload {
    phase_ref: StableId
});
payload!(ReopenPhasePayload { phase_ref: StableId, reason: String, authority_ref: Option<StableId> });
payload!(SkipPhasePayload {
    phase_ref: StableId,
    reason: String,
    authority_ref: StableId
});
payload!(DefineGatePayload {
    gate: GateDefinition
});
payload!(RequestGateEvaluationPayload { gate_ref: StableId });
payload!(RequestGateRevalidationPayload { gate_ref: StableId, signal_ref: Option<ResourceRef> });
payload!(WaiveGatePayload { waiver: GateWaiver });
payload!(RecordWorkProgressPayload {
    work_record: WorkRecord
});
payload!(RecordNotePayload { kind: String, text: String, scope_ref: Option<StableId> });
payload!(AddArtifactReferencePayload {
    artifact: ArtifactReference
});
payload!(RecordClaimPayload { claim: ClaimRecord });
payload!(UpdateClaimStatusPayload { claim_ref: StableId, classification: ClaimClassification, evidence_refs: Vec<StableId> });
payload!(RecordUnknownPayload {
    unknown: UnknownRecord
});
payload!(ResolveUnknownPayload {
    unknown_ref: StableId,
    resolution_ref: ResourceRef
});
payload!(RecordVerificationResultPayload {
    evidence: EvidenceRecord
});
payload!(LinkExternalEvidencePayload {
    external_system: String,
    artifact_uri: ResourceRef,
    artifact_digest: Sha256Digest,
    schema_ref: ResourceRef
});
payload!(UnlinkExternalEvidencePayload {
    evidence_ref: StableId,
    reason: String
});
payload!(InvalidateEvidencePayload {
    evidence_ref: StableId,
    reason: String
});
payload!(ProposeDecisionPayload { decision: Decision });
payload!(AcceptDecisionPayload {
    decision_ref: StableId,
    authority_ref: StableId
});
payload!(SupersedeDecisionPayload {
    decision_ref: StableId,
    replacement: Decision
});
payload!(RejectDecisionPayload {
    decision_ref: StableId,
    reason: String
});
payload!(OpenBlockerPayload { blocker: Blocker });
payload!(UpdateBlockerPayload { blocker: Blocker });
payload!(ResolveBlockerPayload { blocker_ref: StableId, evidence_refs: Vec<StableId>, resolution_summary: String });
payload!(OpenRiskPayload { risk: Risk });
payload!(UpdateRiskPayload { risk: Risk });
payload!(MitigateRiskPayload { risk_ref: StableId, note: String, evidence_refs: Vec<StableId> });
payload!(CloseRiskPayload { risk_ref: StableId, note: String, evidence_refs: Vec<StableId> });
payload!(SetNextActionPayload { action: NextAction });
payload!(CompleteNextActionPayload { action_ref: StableId, evidence_refs: Vec<StableId> });
payload!(CancelNextActionPayload {
    action_ref: StableId,
    reason: String
});
payload!(CreateContinuityCheckpointPayload { reason: String });
payload!(StartSessionPayload { session: Session });
payload!(EndSessionPayload {
    session_ref: StableId,
    create_checkpoint: bool
});
payload!(ConfirmExternalMappingPayload {
    mapping: IdentityMapping
});
payload!(RevokeExternalMappingPayload {
    mapping_ref: StableId,
    reason: String
});
payload!(ResolveConflictPayload {
    conflict_ref: StableId,
    resolution_ref: ResourceRef,
    reason: String
});

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionField {
    pub field_path: String,
    pub replacement: ScalarValue,
}

payload!(RecordEventCorrectionPayload { corrects_event_ref: crate::EventId, reason: String, replacement_field_values: Vec<CorrectionField> });

macro_rules! command_catalog {
    ($($variant:ident($payload:ty)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum CommandType { $($variant),+, #[schemars(skip)] RecordEpisodeUpdate }

        impl CommandType {
            /// The frozen v0.2 catalog. Extension commands are versioned separately.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }

        #[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
        #[serde(tag = "command_type", content = "payload", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
        pub enum Command { $($variant($payload)),+, #[schemars(skip)] RecordEpisodeUpdate(crate::EpisodeUpdate) }

        impl Command {
            pub const fn command_type(&self) -> CommandType {
                match self { $(Self::$variant(_) => CommandType::$variant),+, Self::RecordEpisodeUpdate(_) => CommandType::RecordEpisodeUpdate }
            }
        }
    };
}

command_catalog! {
    CreateProject(CreateProjectPayload),
    UpdateProjectMetadata(UpdateProjectMetadataPayload),
    PauseProject(PauseProjectPayload),
    ResumeProject(ResumeProjectPayload),
    ArchiveProject(ArchiveProjectPayload),
    RegisterAuthorityDocument(RegisterAuthorityDocumentPayload),
    RefreshAuthorityDocumentDigest(RefreshAuthorityDocumentDigestPayload),
    SupersedeAuthorityDocument(SupersedeAuthorityDocumentPayload),
    BindRepository(BindRepositoryPayload),
    RefreshRepositoryBinding(RefreshRepositoryBindingPayload),
    RevokeRepositoryBinding(RevokeRepositoryBindingPayload),
    DefineRoadmap(DefineRoadmapPayload),
    ReviseRoadmap(ReviseRoadmapPayload),
    StartPhase(StartPhasePayload),
    RequestPhaseCompletion(RequestPhaseCompletionPayload),
    ReopenPhase(ReopenPhasePayload),
    SkipPhase(SkipPhasePayload),
    DefineGate(DefineGatePayload),
    RequestGateEvaluation(RequestGateEvaluationPayload),
    RequestGateRevalidation(RequestGateRevalidationPayload),
    WaiveGate(WaiveGatePayload),
    RecordWorkProgress(RecordWorkProgressPayload),
    RecordNote(RecordNotePayload),
    AddArtifactReference(AddArtifactReferencePayload),
    RecordClaim(RecordClaimPayload),
    UpdateClaimStatus(UpdateClaimStatusPayload),
    RecordUnknown(RecordUnknownPayload),
    ResolveUnknown(ResolveUnknownPayload),
    RecordVerificationResult(RecordVerificationResultPayload),
    LinkExternalEvidence(LinkExternalEvidencePayload),
    UnlinkExternalEvidence(UnlinkExternalEvidencePayload),
    InvalidateEvidence(InvalidateEvidencePayload),
    ProposeDecision(ProposeDecisionPayload),
    AcceptDecision(AcceptDecisionPayload),
    SupersedeDecision(SupersedeDecisionPayload),
    RejectDecision(RejectDecisionPayload),
    OpenBlocker(OpenBlockerPayload),
    UpdateBlocker(UpdateBlockerPayload),
    ResolveBlocker(ResolveBlockerPayload),
    OpenRisk(OpenRiskPayload),
    UpdateRisk(UpdateRiskPayload),
    MitigateRisk(MitigateRiskPayload),
    CloseRisk(CloseRiskPayload),
    SetNextAction(SetNextActionPayload),
    CompleteNextAction(CompleteNextActionPayload),
    CancelNextAction(CancelNextActionPayload),
    CreateContinuityCheckpoint(CreateContinuityCheckpointPayload),
    StartSession(StartSessionPayload),
    EndSession(EndSessionPayload),
    ConfirmExternalMapping(ConfirmExternalMappingPayload),
    RevokeExternalMapping(RevokeExternalMappingPayload),
    ResolveConflict(ResolveConflictPayload),
    RecordEventCorrection(RecordEventCorrectionPayload)
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub schema_version: SchemaVersion,
    pub command_id: CommandId,
    pub command_schema_ref: ResourceRef,
    pub project_ref: ProjectId,
    pub expected_state_version: u64,
    pub idempotency_key: StableId,
    pub actor: Actor,
    #[serde(flatten)]
    pub command: Command,
    pub submitted_at: LedgerTimestamp,
    pub payload_digest: Sha256Digest,
    pub command_digest: Sha256Digest,
}

impl CommandEnvelope {
    pub const fn command_type(&self) -> CommandType {
        self.command.command_type()
    }
}

closed_enum! { pub enum CommandResultStatus { Accepted, Rejected, VersionConflict, ValidationFailed, PolicyDenied, DuplicateReplay, IncompatibleContract } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRejection {
    pub code: ErrorCode,
    pub message: String,
    pub subject_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdempotencyResult {
    pub replayed: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandResult {
    pub schema_version: SchemaVersion,
    pub command_result_id: StableId,
    pub command_ref: CommandId,
    pub status: CommandResultStatus,
    pub previous_state_version: u64,
    pub new_state_version: u64,
    pub emitted_event_refs: Vec<crate::EventId>,
    pub resulting_checkpoint_ref: Option<StableId>,
    pub rejection: Option<CommandRejection>,
    pub idempotency: IdempotencyResult,
    pub processed_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

closed_enum! { pub enum ExternalSubmissionState { Prepared, ApprovalRequired, Approved, Submitted, OutcomeUnknown, ReconciledAccepted, ReconciledRejected, Cancelled } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalSubmissionRecord {
    pub schema_version: SchemaVersion,
    pub submission_id: StableId,
    pub command_ref: CommandId,
    pub idempotency_key: StableId,
    pub state: ExternalSubmissionState,
    pub attempt_count: u32,
    pub last_transport_error: Option<String>,
    pub known_result_ref: Option<StableId>,
    pub prepared_at: LedgerTimestamp,
    pub submitted_at: Option<LedgerTimestamp>,
    pub reconciled_at: Option<LedgerTimestamp>,
    pub digest: Sha256Digest,
}
