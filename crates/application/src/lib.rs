//! Storage-independent application services for Project Ledger.

mod create_project;
mod processor;
mod project_state_query;
mod submission;

pub use create_project::CreateProjectHandler;
pub use processor::{
    Clock, CommandHandler, CommandHandlerError, CommandProcessor, FixedClock, ProcessorError,
};
pub use project_ledger_event_store::{
    CommandDraft, CommandResultDraft, EventDraft, ExternalSubmissionDraft, IntegrityError,
    build_command_result, build_event, build_external_submission, prepare_command,
    validate_command_integrity, validate_command_result_integrity, validate_event_integrity,
    validate_external_submission_integrity,
};
pub use project_state_query::{
    ProjectStateQuery, ProjectStateQueryError, ProjectStateRead, ProjectionSource,
};
pub use submission::{ReconciliationOutcome, SubmissionError, SubmissionService};
