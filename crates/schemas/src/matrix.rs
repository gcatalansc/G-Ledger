use project_ledger_domain::{CommandType as C, EventType as E, SchemaVersion};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EngineOwner {
    CommandHandler,
    AuthorityEngine,
    RepositoryIdentityEngine,
    DependencyEngine,
    FreshnessEngine,
    GateEngine,
    PhaseEngine,
    ProjectStatusEngine,
    ClaimEngine,
    UnknownEngine,
    EvidenceEngine,
    DecisionEngine,
    CheckpointEngine,
    ConflictEngine,
    IdentityEngine,
    ProjectionEngine,
    LegacyImportWorkflow,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventOutcome {
    pub required_events: Vec<E>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEventRow {
    pub command_type: C,
    pub accepted_outcomes: Vec<EventOutcome>,
    pub possible_events: Vec<E>,
    pub forbidden_events: Vec<E>,
    pub engine_owners: Vec<EngineOwner>,
    pub atomicity_group: String,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedEventOwner {
    pub event_type: E,
    pub owner: EngineOwner,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEventMatrix {
    pub schema_version: SchemaVersion,
    pub rows: Vec<CommandEventRow>,
    pub derived_only_events: Vec<DerivedEventOwner>,
}

impl CommandEventMatrix {
    pub fn row(&self, command_type: C) -> Option<&CommandEventRow> {
        self.rows
            .iter()
            .find(|row| row.command_type == command_type)
    }
}

/// Builds the exhaustive, auditable command-to-event contract table.
#[allow(clippy::too_many_lines)] // A single declarative table makes omissions and drift reviewable.
pub fn command_event_matrix() -> CommandEventMatrix {
    let derived_only_events = derived_only_events();
    let rows = vec![
        row(
            C::CreateProject,
            &[&[E::ProjectCreated]],
            &[E::ProjectStatusDerived],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::UpdateProjectMetadata,
            &[&[E::ProjectMetadataUpdated]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::PauseProject,
            &[&[E::ProjectPaused]],
            &[E::ProjectStatusDerived],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::ResumeProject,
            &[&[E::ProjectResumed]],
            &[E::ProjectStatusDerived],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::ArchiveProject,
            &[&[E::ProjectArchived]],
            &[E::ProjectStatusDerived],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::RegisterAuthorityDocument,
            &[&[E::AuthorityDocumentRegistered]],
            &[
                E::DecisionRevalidationRequired,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::RefreshAuthorityDocumentDigest,
            &[
                &[E::AuthorityDocumentDigestRefreshed],
                &[E::AuthorityDocumentMarkedUnavailable],
                &[E::AuthorityDocumentDigestMismatch],
            ],
            &[
                E::ConflictOpened,
                E::DecisionRevalidationRequired,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::AuthorityEngine,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::SupersedeAuthorityDocument,
            &[&[E::AuthorityDocumentSuperseded]],
            &[
                E::DecisionRevalidationRequired,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::BindRepository,
            &[&[E::RepositoryBound]],
            &[E::ConflictOpened, E::CheckpointInvalidated],
            &[
                EngineOwner::RepositoryIdentityEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::RefreshRepositoryBinding,
            &[&[E::RepositoryBindingRefreshed]],
            &[
                E::ConflictOpened,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::RepositoryIdentityEngine,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::RevokeRepositoryBinding,
            &[&[E::RepositoryBindingRevoked]],
            &[
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::RepositoryIdentityEngine,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::DefineRoadmap,
            &[&[E::RoadmapDefined]],
            &[E::ProjectStatusDerived],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::ReviseRoadmap,
            &[&[E::RoadmapRevised]],
            &[
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::StartPhase,
            &[&[E::PhaseStarted]],
            &[E::ProjectStatusDerived],
            &[EngineOwner::PhaseEngine, EngineOwner::ProjectStatusEngine],
        ),
        row(
            C::RequestPhaseCompletion,
            &[&[E::PhaseMarkedReadyToClose], &[E::PhaseCompleted]],
            &[
                E::GateEvaluationRequested,
                E::GateEvaluated,
                E::GateVerified,
                E::GateRejected,
                E::PhaseBlocked,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::ReopenPhase,
            &[&[E::PhaseReopened]],
            &[E::CheckpointInvalidated, E::ProjectStatusDerived],
            &[
                EngineOwner::PhaseEngine,
                EngineOwner::FreshnessEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::SkipPhase,
            &[&[E::PhaseSkipped]],
            &[E::ProjectStatusDerived],
            &[EngineOwner::PhaseEngine, EngineOwner::ProjectStatusEngine],
        ),
        row(
            C::DefineGate,
            &[&[E::GateDefined]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::GateEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::RequestGateEvaluation,
            &[&[E::GateEvaluationRequested, E::GateEvaluated]],
            &[
                E::GateVerified,
                E::GateRejected,
                E::GateMarkedStale,
                E::PhaseCompleted,
                E::PhaseBlocked,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::RequestGateRevalidation,
            &[&[E::GateRevalidationRequested, E::GateEvaluated]],
            &[
                E::GateVerified,
                E::GateRejected,
                E::GateRevalidationRequired,
                E::PhaseReopened,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::WaiveGate,
            &[&[E::GateWaived]],
            &[
                E::GateEvaluationRequested,
                E::GateEvaluated,
                E::GateVerified,
                E::PhaseCompleted,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::RecordWorkProgress,
            &[&[E::WorkProgressRecorded]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::RecordNote,
            &[&[E::NoteRecorded]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::AddArtifactReference,
            &[&[E::ArtifactReferenceAdded]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::RecordClaim,
            &[&[E::ClaimRecorded]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::ClaimEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::UpdateClaimStatus,
            &[&[E::ClaimStatusChanged]],
            &[
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::ClaimEngine,
                EngineOwner::GateEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::RecordUnknown,
            &[&[E::UnknownRecorded]],
            &[
                E::PhaseBlocked,
                E::ProjectStatusDerived,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::UnknownEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::ResolveUnknown,
            &[&[E::UnknownResolved]],
            &[E::ProjectStatusDerived, E::CheckpointInvalidated],
            &[
                EngineOwner::UnknownEngine,
                EngineOwner::ProjectStatusEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::RecordVerificationResult,
            &[&[E::VerificationRecorded]],
            &[
                E::GateEvaluationRequested,
                E::GateEvaluated,
                E::GateVerified,
                E::GateRejected,
                E::GateMarkedStale,
                E::PhaseCompleted,
                E::PhaseBlocked,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::EvidenceEngine,
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::LinkExternalEvidence,
            &[&[E::ExternalEvidenceLinked]],
            &[
                E::GateEvaluationRequested,
                E::GateEvaluated,
                E::GateVerified,
                E::GateRejected,
            ],
            &[EngineOwner::EvidenceEngine, EngineOwner::GateEngine],
        ),
        row(
            C::UnlinkExternalEvidence,
            &[&[E::ExternalEvidenceUnlinked]],
            &[
                E::EvidenceInvalidated,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::EvidenceEngine,
                EngineOwner::GateEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::InvalidateEvidence,
            &[&[E::EvidenceInvalidated]],
            &[
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::PhaseReopened,
                E::CheckpointInvalidated,
                E::ProjectStatusDerived,
            ],
            &[
                EngineOwner::EvidenceEngine,
                EngineOwner::GateEngine,
                EngineOwner::PhaseEngine,
                EngineOwner::FreshnessEngine,
                EngineOwner::ProjectStatusEngine,
            ],
        ),
        row(
            C::ProposeDecision,
            &[&[E::DecisionProposed]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::AcceptDecision,
            &[&[E::DecisionAccepted]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::DecisionEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::SupersedeDecision,
            &[&[E::DecisionSuperseded]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::DecisionEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::RejectDecision,
            &[&[E::DecisionRejected]],
            &[],
            &[EngineOwner::DecisionEngine],
        ),
        row(
            C::OpenBlocker,
            &[&[E::BlockerOpened]],
            &[
                E::PhaseBlocked,
                E::ProjectStatusDerived,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::UpdateBlocker,
            &[&[E::BlockerUpdated]],
            &[
                E::PhaseBlocked,
                E::ProjectStatusDerived,
                E::CheckpointInvalidated,
            ],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::PhaseEngine,
                EngineOwner::ProjectStatusEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::ResolveBlocker,
            &[&[E::BlockerResolved]],
            &[E::ProjectStatusDerived, E::CheckpointInvalidated],
            &[
                EngineOwner::CommandHandler,
                EngineOwner::ProjectStatusEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
        row(
            C::OpenRisk,
            &[&[E::RiskOpened]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::UpdateRisk,
            &[&[E::RiskUpdated]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::MitigateRisk,
            &[&[E::RiskMitigated]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::CloseRisk,
            &[&[E::RiskClosed]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::SetNextAction,
            &[&[E::NextActionSet]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::CompleteNextAction,
            &[&[E::NextActionCompleted]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::CancelNextAction,
            &[&[E::NextActionCancelled]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
        ),
        row(
            C::CreateContinuityCheckpoint,
            &[&[E::CheckpointCreated]],
            &[],
            &[EngineOwner::CheckpointEngine],
        ),
        row(
            C::StartSession,
            &[&[E::SessionStarted]],
            &[],
            &[EngineOwner::CommandHandler],
        ),
        row(
            C::EndSession,
            &[&[E::SessionEnded]],
            &[E::CheckpointCreated],
            &[EngineOwner::CommandHandler, EngineOwner::CheckpointEngine],
        ),
        row(
            C::ConfirmExternalMapping,
            &[&[E::ExternalMappingConfirmed]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::IdentityEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::RevokeExternalMapping,
            &[&[E::ExternalMappingRevoked]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::IdentityEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::ResolveConflict,
            &[&[E::ConflictResolved]],
            &[E::CheckpointInvalidated],
            &[EngineOwner::ConflictEngine, EngineOwner::FreshnessEngine],
        ),
        row(
            C::RecordEventCorrection,
            &[&[E::EventCorrectionRecorded]],
            &[
                E::ProjectStatusDerived,
                E::PhaseBlocked,
                E::PhaseCompleted,
                E::GateEvaluated,
                E::GateVerified,
                E::GateRejected,
                E::GateMarkedStale,
                E::GateRevalidationRequired,
                E::DecisionRevalidationRequired,
                E::CheckpointInvalidated,
                E::ConflictOpened,
            ],
            &[
                EngineOwner::ProjectionEngine,
                EngineOwner::DependencyEngine,
                EngineOwner::FreshnessEngine,
            ],
        ),
    ];
    CommandEventMatrix {
        schema_version: SchemaVersion::V0_2_0,
        rows,
        derived_only_events,
    }
}

/// Additive v0.3 catalog; the v0.2 matrix remains byte-identical.
pub fn command_event_matrix_v0_3() -> CommandEventMatrix {
    let mut matrix = command_event_matrix();
    matrix.schema_version = SchemaVersion::V0_3_0;
    matrix.rows.push(row(
        C::RecordEpisodeUpdate,
        &[&[E::EpisodeUpdateRecorded, E::WorkProgressRecorded]],
        &[E::CheckpointInvalidated],
        &[EngineOwner::CommandHandler, EngineOwner::FreshnessEngine],
    ));
    matrix
}

fn row(
    command_type: C,
    accepted_outcomes: &[&[E]],
    possible_events: &[E],
    engine_owners: &[EngineOwner],
) -> CommandEventRow {
    let required_or_possible: Vec<E> = accepted_outcomes
        .iter()
        .flat_map(|events| events.iter().copied())
        .chain(possible_events.iter().copied())
        .collect();
    let forbidden_events = derived_only_events()
        .into_iter()
        .map(|entry| entry.event_type)
        .filter(|event| !required_or_possible.contains(event))
        .collect();
    CommandEventRow {
        command_type,
        accepted_outcomes: accepted_outcomes
            .iter()
            .map(|events| EventOutcome {
                required_events: events.to_vec(),
            })
            .collect(),
        possible_events: possible_events.to_vec(),
        forbidden_events,
        engine_owners: engine_owners.to_vec(),
        atomicity_group: format!("command:{command_type:?}"),
    }
}

fn derived_only_events() -> Vec<DerivedEventOwner> {
    vec![
        DerivedEventOwner {
            event_type: E::ProjectStatusDerived,
            owner: EngineOwner::ProjectStatusEngine,
        },
        DerivedEventOwner {
            event_type: E::PhaseBlocked,
            owner: EngineOwner::PhaseEngine,
        },
        DerivedEventOwner {
            event_type: E::PhaseCompleted,
            owner: EngineOwner::PhaseEngine,
        },
        DerivedEventOwner {
            event_type: E::GateEvaluated,
            owner: EngineOwner::GateEngine,
        },
        DerivedEventOwner {
            event_type: E::GateVerified,
            owner: EngineOwner::GateEngine,
        },
        DerivedEventOwner {
            event_type: E::GateRejected,
            owner: EngineOwner::GateEngine,
        },
        DerivedEventOwner {
            event_type: E::GateMarkedStale,
            owner: EngineOwner::FreshnessEngine,
        },
        DerivedEventOwner {
            event_type: E::GateRevalidationRequired,
            owner: EngineOwner::FreshnessEngine,
        },
        DerivedEventOwner {
            event_type: E::DecisionRevalidationRequired,
            owner: EngineOwner::FreshnessEngine,
        },
        DerivedEventOwner {
            event_type: E::CheckpointInvalidated,
            owner: EngineOwner::FreshnessEngine,
        },
        DerivedEventOwner {
            event_type: E::ConflictOpened,
            owner: EngineOwner::ConflictEngine,
        },
        DerivedEventOwner {
            event_type: E::LegacyImportReviewed,
            owner: EngineOwner::LegacyImportWorkflow,
        },
    ]
}
