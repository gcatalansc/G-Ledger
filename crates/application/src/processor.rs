use project_ledger_domain::{
    CommandEnvelope, CommandRejection, CommandResult, CommandResultStatus, ErrorCode,
    LedgerTimestamp, StableId,
};
use project_ledger_event_store::{
    CommandCommit, CommandResultDraft, CommitDisposition, EventDraft, EventStore,
    IdempotencyLookup, IntegrityError, StoreError, build_command_result, build_event,
    validate_command_integrity,
};
use project_ledger_schemas::command_event_matrix;
use thiserror::Error;

/// Deterministic time source for command-result construction.
pub trait Clock {
    fn now(&self) -> LedgerTimestamp;
}

/// A fixed clock useful for deterministic execution and tests.
#[derive(Clone, Debug)]
pub struct FixedClock {
    timestamp: LedgerTimestamp,
}

impl FixedClock {
    pub const fn new(timestamp: LedgerTimestamp) -> Self {
        Self { timestamp }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> LedgerTimestamp {
        self.timestamp.clone()
    }
}

/// A pure command handler: it proposes event drafts but cannot persist them.
pub trait CommandHandler {
    /// Proposes events from one immutable, already integrity-checked command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandHandlerError`] when domain validation rejects the command.
    fn handle(&self, command: &CommandEnvelope) -> Result<Vec<EventDraft>, CommandHandlerError>;
}

/// A typed domain-validation failure returned by a pure handler.
#[derive(Clone, Debug, Error)]
#[error("{message}")]
pub struct CommandHandlerError {
    pub code: ErrorCode,
    pub message: String,
    pub subject_refs: Vec<StableId>,
}

impl CommandHandlerError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            subject_refs: Vec::new(),
        }
    }
}

/// Failures that prevent the application boundary from returning a valid result contract.
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("closed contract enum encoding failed: {0}")]
    ContractEncoding(#[from] serde_json::Error),
    #[error("could not derive a stable result identity")]
    ResultIdentity,
}

/// Orchestrates the canonical command boundary over an abstract event store.
#[derive(Debug)]
pub struct CommandProcessor<'a, S, C> {
    store: &'a mut S,
    clock: &'a C,
}

impl<'a, S, C> CommandProcessor<'a, S, C>
where
    S: EventStore,
    C: Clock,
{
    pub const fn new(store: &'a mut S, clock: &'a C) -> Self {
        Self { store, clock }
    }

    /// Validates, handles, chains, and atomically persists one command.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessorError`] only when storage or result construction cannot
    /// produce a trustworthy contract. Expected conflicts are returned as
    /// [`CommandResult`] values.
    #[allow(clippy::too_many_lines)] // Keeping the normative transaction flow linear aids audit review.
    pub fn execute<H>(
        &mut self,
        command: CommandEnvelope,
        handler: &H,
    ) -> Result<CommandResult, ProcessorError>
    where
        H: CommandHandler,
    {
        if let Err(error) = validate_command_integrity(&command) {
            return self.rejected(
                &command,
                CommandResultStatus::IncompatibleContract,
                0,
                ErrorCode::IntegrityViolation,
                error.to_string(),
                Vec::new(),
            );
        }

        let trust_domain = contract_name(&command.actor.trust_level)?;
        match self.store.lookup_idempotency(
            &command.project_ref,
            command.command_type(),
            &command.actor.actor_id,
            &trust_domain,
            &command.idempotency_key,
            command.command_digest,
        )? {
            IdempotencyLookup::Vacant => {}
            IdempotencyLookup::Replay(previous) => return self.replay(previous),
            IdempotencyLookup::Conflict { .. } => {
                let current = self.store.current_version(&command.project_ref)?;
                return self.rejected(
                    &command,
                    CommandResultStatus::Rejected,
                    current,
                    ErrorCode::IdempotencyConflict,
                    "the scoped idempotency key belongs to a different command digest",
                    Vec::new(),
                );
            }
        }

        self.store.verify_event_chain(&command.project_ref)?;
        let stream = self.store.load_stream(&command.project_ref)?;
        let current = stream.len() as u64;
        if command.expected_state_version != current {
            return self.persist_rejected(
                &command,
                CommandResultStatus::VersionConflict,
                current,
                ErrorCode::VersionConflict,
                format!(
                    "expected state version {}, actual {current}",
                    command.expected_state_version
                ),
                Vec::new(),
            );
        }

        let drafts = match handler.handle(&command) {
            Ok(drafts) => drafts,
            Err(error) => {
                return self.persist_rejected(
                    &command,
                    CommandResultStatus::ValidationFailed,
                    current,
                    error.code,
                    error.message,
                    error.subject_refs,
                );
            }
        };
        if !matrix_accepts(&command, &drafts) {
            return self.persist_rejected(
                &command,
                CommandResultStatus::IncompatibleContract,
                current,
                ErrorCode::CommandEventContractViolation,
                "handler output violates the closed command-event matrix",
                Vec::new(),
            );
        }

        let mut prior_digest = stream.last().map(|event| event.event_digest);
        let mut events = Vec::with_capacity(drafts.len());
        for (index, draft) in drafts.into_iter().enumerate() {
            let event = build_event(&command, current + index as u64 + 1, prior_digest, draft)?;
            prior_digest = Some(event.event_digest);
            events.push(event);
        }
        let result = build_result(
            &command,
            CommandResultStatus::Accepted,
            current,
            current + events.len() as u64,
            events.iter().map(|event| event.event_id.clone()).collect(),
            None,
            false,
            self.clock.now(),
        )?;
        let command_for_outcome = command.clone();
        match self.store.commit_command(CommandCommit {
            command,
            events,
            result,
        }) {
            Ok(CommitDisposition::Committed(result)) => Ok(result),
            Ok(CommitDisposition::Replayed(previous)) => self.replay(previous),
            Err(StoreError::IdempotencyConflict { .. }) => {
                let current = self
                    .store
                    .current_version(&command_for_outcome.project_ref)?;
                self.rejected(
                    &command_for_outcome,
                    CommandResultStatus::Rejected,
                    current,
                    ErrorCode::IdempotencyConflict,
                    "the scoped idempotency key changed during commit",
                    Vec::new(),
                )
            }
            Err(StoreError::StreamVersionConflict { actual, .. }) => self.rejected(
                &command_for_outcome,
                CommandResultStatus::VersionConflict,
                actual,
                ErrorCode::VersionConflict,
                "the stream version changed during commit",
                Vec::new(),
            ),
            Err(error) => Err(error.into()),
        }
    }

    fn rejected(
        &self,
        command: &CommandEnvelope,
        status: CommandResultStatus,
        current: u64,
        code: ErrorCode,
        message: impl Into<String>,
        subject_refs: Vec<StableId>,
    ) -> Result<CommandResult, ProcessorError> {
        build_result(
            command,
            status,
            current,
            current,
            Vec::new(),
            Some(CommandRejection {
                code,
                message: message.into(),
                subject_refs,
            }),
            false,
            self.clock.now(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn persist_rejected(
        &mut self,
        command: &CommandEnvelope,
        status: CommandResultStatus,
        current: u64,
        code: ErrorCode,
        message: impl Into<String>,
        subject_refs: Vec<StableId>,
    ) -> Result<CommandResult, ProcessorError> {
        let result = self.rejected(command, status, current, code, message, subject_refs)?;
        match self.store.commit_command(CommandCommit {
            command: command.clone(),
            events: Vec::new(),
            result: result.clone(),
        }) {
            Ok(CommitDisposition::Committed(result)) => Ok(result),
            Ok(CommitDisposition::Replayed(previous)) => self.replay(previous),
            Err(StoreError::StreamVersionConflict { actual, .. }) => self.rejected(
                command,
                CommandResultStatus::VersionConflict,
                actual,
                ErrorCode::VersionConflict,
                "the stream version changed while recording the command result",
                Vec::new(),
            ),
            Err(StoreError::IdempotencyConflict { .. }) => self.rejected(
                command,
                CommandResultStatus::Rejected,
                current,
                ErrorCode::IdempotencyConflict,
                "the scoped idempotency key changed while recording the command result",
                Vec::new(),
            ),
            Err(error) => Err(error.into()),
        }
    }

    fn replay(&self, previous: CommandResult) -> Result<CommandResult, ProcessorError> {
        let replay_status = if previous.status == CommandResultStatus::Accepted {
            CommandResultStatus::DuplicateReplay
        } else {
            previous.status
        };
        build_command_result(CommandResultDraft {
            command_result_id: previous.command_result_id,
            command_ref: previous.command_ref,
            status: replay_status,
            previous_state_version: previous.previous_state_version,
            new_state_version: previous.new_state_version,
            emitted_event_refs: previous.emitted_event_refs,
            resulting_checkpoint_ref: previous.resulting_checkpoint_ref,
            rejection: previous.rejection,
            replayed: true,
            processed_at: self.clock.now(),
        })
        .map_err(ProcessorError::from)
    }
}

fn matrix_accepts(command: &CommandEnvelope, drafts: &[EventDraft]) -> bool {
    let matrix = command_event_matrix();
    let Some(row) = matrix.row(command.command_type()) else {
        return false;
    };
    let emitted: Vec<_> = drafts
        .iter()
        .map(|draft| draft.event.event_type())
        .collect();
    row.accepted_outcomes.iter().any(|outcome| {
        emitted.iter().all(|event| {
            outcome.required_events.contains(event) || row.possible_events.contains(event)
        }) && emitted
            .iter()
            .filter(|event| !row.possible_events.contains(event))
            .copied()
            .eq(outcome.required_events.iter().copied())
    })
}

#[allow(clippy::too_many_arguments)]
fn build_result(
    command: &CommandEnvelope,
    status: CommandResultStatus,
    previous_state_version: u64,
    new_state_version: u64,
    emitted_event_refs: Vec<project_ledger_domain::EventId>,
    rejection: Option<CommandRejection>,
    replayed: bool,
    processed_at: LedgerTimestamp,
) -> Result<CommandResult, ProcessorError> {
    let command_result_id = StableId::parse(&format!("result:{}", command.command_id.uuid()))
        .map_err(|_| ProcessorError::ResultIdentity)?;
    build_command_result(CommandResultDraft {
        command_result_id,
        command_ref: command.command_id.clone(),
        status,
        previous_state_version,
        new_state_version,
        emitted_event_refs,
        resulting_checkpoint_ref: None,
        rejection,
        replayed,
        processed_at,
    })
    .map_err(ProcessorError::from)
}

fn contract_name<T: serde::Serialize>(value: &T) -> Result<String, ProcessorError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or(ProcessorError::ResultIdentity)
}
