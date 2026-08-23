//! Storage-independent event-store ports and canonical integrity rules.

mod integrity;
mod ports;

pub use integrity::{
    CommandDraft, CommandResultDraft, EventDraft, ExternalSubmissionDraft, IntegrityError,
    build_command_result, build_event, build_external_submission, prepare_command,
    validate_command_integrity, validate_command_result_integrity, validate_event_integrity,
    validate_external_submission_integrity,
};
pub use ports::{
    CommandCommit, CommitDisposition, EventStore, IdempotencyLookup, StoreError,
    SubmissionDisposition, SubmissionStore,
};
