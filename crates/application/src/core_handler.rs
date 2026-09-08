use project_ledger_continuity::{ContinuityEngine, ContinuityError};
use project_ledger_domain::{Command, CommandEnvelope, ErrorCode, EventEnvelope, TrustLevel};
use project_ledger_event_store::EventDraft;
use project_ledger_gate_engine::GateEngine;
use project_ledger_lifecycle::LifecycleEngine;

use crate::{CommandHandler, CommandHandlerError, CreateProjectHandler};

/// Routes closed core commands to their bounded deterministic owner.
#[derive(Clone, Copy, Debug, Default)]
pub struct CoreCommandHandler;

impl CommandHandler for CoreCommandHandler {
    fn handle(
        &self,
        command: &CommandEnvelope,
        verified_stream: &[EventEnvelope],
    ) -> Result<Vec<EventDraft>, CommandHandlerError> {
        if !matches!(
            command.actor.trust_level,
            TrustLevel::LocalUser | TrustLevel::TrustedForRecording
        ) {
            return Err(CommandHandlerError::new(
                ErrorCode::InvalidContract,
                "canonical commands require explicit recording trust",
            ));
        }
        let base = match &command.command {
            Command::CreateProject(_) => CreateProjectHandler.handle(command, verified_stream),
            Command::RegisterAuthorityDocument(_)
            | Command::RefreshAuthorityDocumentDigest(_)
            | Command::SupersedeAuthorityDocument(_) => {
                let base = LifecycleEngine::handle(command, verified_stream).map_err(|error| {
                    CommandHandlerError {
                        code: ErrorCode::InvalidContract,
                        message: error.to_string(),
                        subject_refs: error.subject_refs(),
                    }
                })?;
                GateEngine::extend_authority_freshness(command, verified_stream, base).map_err(
                    |error| CommandHandlerError {
                        code: ErrorCode::InvalidContract,
                        message: error.to_string(),
                        subject_refs: error.subject_refs(),
                    },
                )
            }
            Command::RecordEpisodeUpdate(_)
            | Command::RecordWorkProgress(_)
            | Command::RecordNote(_)
            | Command::AddArtifactReference(_)
            | Command::StartSession(_)
            | Command::EndSession(_)
            | Command::ConfirmExternalMapping(_)
            | Command::RevokeExternalMapping(_)
            | Command::RecordEventCorrection(_)
            | Command::RecordUnknown(_)
            | Command::ResolveUnknown(_)
            | Command::CreateContinuityCheckpoint(_) => {
                ContinuityEngine::handle(command, verified_stream)
                    .map_err(|error| continuity_error(&error))
            }
            Command::DefineGate(_)
            | Command::BindRepository(_)
            | Command::RefreshRepositoryBinding(_)
            | Command::RevokeRepositoryBinding(_)
            | Command::RequestGateEvaluation(_)
            | Command::RequestGateRevalidation(_)
            | Command::WaiveGate(_)
            | Command::RequestPhaseCompletion(_)
            | Command::RecordVerificationResult(_)
            | Command::RecordClaim(_)
            | Command::UpdateClaimStatus(_)
            | Command::LinkExternalEvidence(_)
            | Command::UnlinkExternalEvidence(_)
            | Command::InvalidateEvidence(_) => GateEngine::handle(command, verified_stream)
                .map_err(|error| CommandHandlerError {
                    code: ErrorCode::InvalidContract,
                    message: error.to_string(),
                    subject_refs: error.subject_refs(),
                }),
            _ => LifecycleEngine::handle(command, verified_stream).map_err(|error| {
                CommandHandlerError {
                    code: ErrorCode::InvalidContract,
                    message: error.to_string(),
                    subject_refs: error.subject_refs(),
                }
            }),
        }?;
        ContinuityEngine::extend_checkpoint_freshness(command, verified_stream, base)
            .map_err(|error| continuity_error(&error))
    }
}

fn continuity_error(error: &ContinuityError) -> CommandHandlerError {
    CommandHandlerError {
        code: ErrorCode::InvalidContract,
        message: error.to_string(),
        subject_refs: error.subject_refs(),
    }
}
