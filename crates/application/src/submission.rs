use project_ledger_domain::{
    CommandId, CommandResult, CommandResultStatus, ExternalSubmissionRecord,
    ExternalSubmissionState, LedgerTimestamp, StableId,
};
use project_ledger_event_store::{
    EventStore, ExternalSubmissionDraft, IntegrityError, StoreError, SubmissionDisposition,
    SubmissionStore, build_external_submission,
};
use thiserror::Error;

/// Result of reconciling a previously unknown transport outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome {
    Accepted { result_ref: StableId },
    Rejected { result_ref: StableId },
    NotFound,
}

/// Failures at the local external-submission state boundary.
#[derive(Debug, Error)]
pub enum SubmissionError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("external submission {submission_ref} does not exist")]
    NotFound { submission_ref: StableId },
    #[error("illegal external submission transition from {from} to {to}")]
    IllegalTransition { from: String, to: String },
    #[error("external submission state conflict: expected {expected}, actual {actual}")]
    StateConflict { expected: String, actual: String },
}

/// Separates client-side transport uncertainty from durable server results.
#[derive(Debug)]
pub struct SubmissionService<'a, S> {
    store: &'a mut S,
}

impl<'a, S> SubmissionService<'a, S>
where
    S: EventStore + SubmissionStore,
{
    pub const fn new(store: &'a mut S) -> Self {
        Self { store }
    }

    /// Records a local submission snapshot and initial history entry.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] when integrity or persistence fails.
    pub fn record(
        &mut self,
        record: ExternalSubmissionRecord,
    ) -> Result<SubmissionDisposition, SubmissionError> {
        self.store.record_submission(record).map_err(Into::into)
    }

    /// Moves a submitted request to `OUTCOME_UNKNOWN` without inventing a result.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] for missing, illegal, stale, or corrupt state.
    pub fn mark_outcome_unknown(
        &mut self,
        submission_ref: &StableId,
        transport_error: &str,
        transitioned_at: &LedgerTimestamp,
    ) -> Result<ExternalSubmissionRecord, SubmissionError> {
        let current = self.required_submission(submission_ref)?;
        if current.state == ExternalSubmissionState::OutcomeUnknown {
            if current.last_transport_error.as_deref() == Some(transport_error) {
                self.store.transition_submission(
                    ExternalSubmissionState::Submitted,
                    current.clone(),
                    transitioned_at,
                )?;
                return Ok(current);
            }
            return Err(SubmissionError::StateConflict {
                expected: state_name(ExternalSubmissionState::Submitted),
                actual: state_name(current.state),
            });
        }
        if current.state != ExternalSubmissionState::Submitted {
            return Err(SubmissionError::IllegalTransition {
                from: state_name(current.state),
                to: state_name(ExternalSubmissionState::OutcomeUnknown),
            });
        }
        let replacement = rebuild_submission(
            current,
            ExternalSubmissionState::OutcomeUnknown,
            Some(transport_error.to_owned()),
            None,
            None,
        )?;
        self.transition(
            ExternalSubmissionState::Submitted,
            replacement,
            transitioned_at,
        )
    }

    /// Reconciles only against a durable server result; absence leaves unknown unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] for missing, illegal, stale, or corrupt state.
    pub fn reconcile(
        &mut self,
        submission_ref: &StableId,
        reconciled_at: &LedgerTimestamp,
    ) -> Result<ReconciliationOutcome, SubmissionError> {
        let current = self.required_submission(submission_ref)?;
        if current.state == ExternalSubmissionState::ReconciledAccepted {
            return terminal_outcome(&current, true);
        }
        if current.state == ExternalSubmissionState::ReconciledRejected {
            return terminal_outcome(&current, false);
        }
        if current.state != ExternalSubmissionState::OutcomeUnknown {
            return Err(SubmissionError::IllegalTransition {
                from: state_name(current.state),
                to: "RECONCILED_*".to_owned(),
            });
        }
        let Some(result) = self.store.command_result(&current.command_ref)? else {
            return Ok(ReconciliationOutcome::NotFound);
        };
        let accepted = matches!(
            result.status,
            CommandResultStatus::Accepted | CommandResultStatus::DuplicateReplay
        );
        let target = if accepted {
            ExternalSubmissionState::ReconciledAccepted
        } else {
            ExternalSubmissionState::ReconciledRejected
        };
        let result_ref = result.command_result_id;
        let replacement = rebuild_submission(
            current,
            target,
            None,
            Some(result_ref.clone()),
            Some(reconciled_at.clone()),
        )?;
        self.transition(
            ExternalSubmissionState::OutcomeUnknown,
            replacement,
            reconciled_at,
        )?;
        Ok(if accepted {
            ReconciliationOutcome::Accepted { result_ref }
        } else {
            ReconciliationOutcome::Rejected { result_ref }
        })
    }

    /// Returns the latest local submission snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] when storage or integrity verification fails.
    pub fn submission(
        &self,
        submission_ref: &StableId,
    ) -> Result<Option<ExternalSubmissionRecord>, SubmissionError> {
        self.store.submission(submission_ref).map_err(Into::into)
    }

    /// Returns the append-only snapshot history in transition order.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] when storage or integrity verification fails.
    pub fn submission_history(
        &self,
        submission_ref: &StableId,
    ) -> Result<Vec<ExternalSubmissionRecord>, SubmissionError> {
        self.store
            .submission_history(submission_ref)
            .map_err(Into::into)
    }

    /// Looks up the durable server result without interpreting transport state.
    ///
    /// # Errors
    ///
    /// Returns [`SubmissionError`] when storage cannot complete the lookup.
    pub fn command_result(
        &self,
        command_ref: &CommandId,
    ) -> Result<Option<CommandResult>, SubmissionError> {
        self.store.command_result(command_ref).map_err(Into::into)
    }

    fn required_submission(
        &self,
        submission_ref: &StableId,
    ) -> Result<ExternalSubmissionRecord, SubmissionError> {
        self.submission(submission_ref)?
            .ok_or_else(|| SubmissionError::NotFound {
                submission_ref: submission_ref.clone(),
            })
    }

    fn transition(
        &mut self,
        expected: ExternalSubmissionState,
        replacement: ExternalSubmissionRecord,
        transitioned_at: &LedgerTimestamp,
    ) -> Result<ExternalSubmissionRecord, SubmissionError> {
        match self
            .store
            .transition_submission(expected, replacement.clone(), transitioned_at)
        {
            Ok(_) => Ok(replacement),
            Err(StoreError::SubmissionStateConflict { expected, actual }) => {
                Err(SubmissionError::StateConflict { expected, actual })
            }
            Err(error) => Err(error.into()),
        }
    }
}

fn rebuild_submission(
    current: ExternalSubmissionRecord,
    state: ExternalSubmissionState,
    last_transport_error: Option<String>,
    known_result_ref: Option<StableId>,
    reconciled_at: Option<LedgerTimestamp>,
) -> Result<ExternalSubmissionRecord, IntegrityError> {
    build_external_submission(ExternalSubmissionDraft {
        submission_id: current.submission_id,
        command_ref: current.command_ref,
        idempotency_key: current.idempotency_key,
        state,
        attempt_count: current.attempt_count,
        last_transport_error,
        known_result_ref,
        prepared_at: current.prepared_at,
        submitted_at: current.submitted_at,
        reconciled_at,
    })
}

fn terminal_outcome(
    current: &ExternalSubmissionRecord,
    accepted: bool,
) -> Result<ReconciliationOutcome, SubmissionError> {
    let result_ref =
        current
            .known_result_ref
            .clone()
            .ok_or_else(|| SubmissionError::StateConflict {
                expected: "known result reference".to_owned(),
                actual: "none".to_owned(),
            })?;
    Ok(if accepted {
        ReconciliationOutcome::Accepted { result_ref }
    } else {
        ReconciliationOutcome::Rejected { result_ref }
    })
}

fn state_name(state: ExternalSubmissionState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{state:?}"))
}
