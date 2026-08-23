use project_ledger_domain::{CommandEnvelope, ErrorCode, EventEnvelope};
use project_ledger_event_store::EventDraft;
use project_ledger_lifecycle::LifecycleEngine;

use crate::{CommandHandler, CommandHandlerError};

impl CommandHandler for LifecycleEngine {
    fn handle(
        &self,
        command: &CommandEnvelope,
        verified_stream: &[EventEnvelope],
    ) -> Result<Vec<EventDraft>, CommandHandlerError> {
        LifecycleEngine::handle(command, verified_stream).map_err(|error| CommandHandlerError {
            code: ErrorCode::InvalidContract,
            message: error.to_string(),
            subject_refs: error.subject_refs(),
        })
    }
}
