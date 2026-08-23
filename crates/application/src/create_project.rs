use project_ledger_domain::{
    Command, CommandEnvelope, ErrorCode, Event, EventId, ProjectCreatedEvent, StableId,
};
use project_ledger_event_store::EventDraft;

use crate::{CommandHandler, CommandHandlerError};

/// Minimal S2 handler for the one command needed by offline initialization.
#[derive(Clone, Copy, Debug, Default)]
pub struct CreateProjectHandler;

impl CommandHandler for CreateProjectHandler {
    fn handle(
        &self,
        command: &CommandEnvelope,
        verified_stream: &[project_ledger_domain::EventEnvelope],
    ) -> Result<Vec<EventDraft>, CommandHandlerError> {
        let Command::CreateProject(payload) = &command.command else {
            return Err(CommandHandlerError::new(
                ErrorCode::InvalidContract,
                "CREATE_PROJECT handler received another command type",
            ));
        };
        if payload.project.project_id != command.project_ref {
            return Err(CommandHandlerError::new(
                ErrorCode::InvalidContract,
                "project payload identity differs from the command project reference",
            ));
        }
        if !verified_stream.is_empty() {
            return Err(CommandHandlerError::new(
                ErrorCode::InvalidContract,
                "CREATE_PROJECT requires an empty verified stream",
            ));
        }
        let suffix = command.command_id.uuid();
        let event_id = EventId::parse(&format!("event:{suffix}")).map_err(|_| {
            CommandHandlerError::new(
                ErrorCode::InvalidIdentifier,
                "could not derive event identity",
            )
        })?;
        let correlation_ref = StableId::parse(&format!("correlation:{suffix}")).map_err(|_| {
            CommandHandlerError::new(
                ErrorCode::InvalidIdentifier,
                "could not derive correlation identity",
            )
        })?;
        Ok(vec![EventDraft {
            event_id,
            event: Event::ProjectCreated(ProjectCreatedEvent {
                project: payload.project.clone(),
            }),
            correlation_ref,
            occurred_at: command.submitted_at.clone(),
            recorded_at: command.submitted_at.clone(),
        }])
    }
}
