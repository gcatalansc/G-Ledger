use project_ledger_canonical_json::to_canonical_vec;
use project_ledger_domain::{
    ExternalSubmissionRecord, ExternalSubmissionState, LedgerTimestamp, StableId,
};
use project_ledger_event_store::{
    StoreError, SubmissionDisposition, SubmissionStore, validate_external_submission_integrity,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::SqliteEventStore;

impl SubmissionStore for SqliteEventStore {
    fn submission(
        &self,
        submission_ref: &StableId,
    ) -> Result<Option<ExternalSubmissionRecord>, StoreError> {
        let bytes = self
            .connection
            .query_row(
                "SELECT record_json FROM external_submissions WHERE submission_id = ?1",
                [submission_ref.as_str()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(persistence)?;
        decode_optional(bytes)
    }

    fn submission_history(
        &self,
        submission_ref: &StableId,
    ) -> Result<Vec<ExternalSubmissionRecord>, StoreError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT record_json FROM external_submission_transitions
                 WHERE submission_ref = ?1 ORDER BY ordinal",
            )
            .map_err(persistence)?;
        let rows = statement
            .query_map([submission_ref.as_str()], |row| row.get::<_, Vec<u8>>(0))
            .map_err(persistence)?;
        let mut history = Vec::new();
        for row in rows {
            let bytes = row.map_err(persistence)?;
            let record: ExternalSubmissionRecord = serde_json::from_slice(&bytes)?;
            validate_external_submission_integrity(&record)?;
            history.push(record);
        }
        Ok(history)
    }

    fn record_submission(
        &mut self,
        record: ExternalSubmissionRecord,
    ) -> Result<SubmissionDisposition, StoreError> {
        validate_external_submission_integrity(&record)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(persistence)?;
        let existing = transaction
            .query_row(
                "SELECT record_digest FROM external_submissions WHERE submission_id = ?1",
                [record.submission_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(persistence)?;
        if let Some(existing) = existing {
            if existing == record.digest.to_string() {
                return Ok(SubmissionDisposition::Replayed);
            }
            return Err(StoreError::InvalidCommit {
                message: "submission identity already exists with another digest".to_owned(),
            });
        }
        let bytes = canonical_bytes(&record)?;
        let transitioned_at = latest_record_time(&record);
        transaction
            .execute(
                "INSERT INTO external_submissions(
                    submission_id, command_ref, idempotency_key, state, record_digest,
                    record_json, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.submission_id.as_str(),
                    record.command_ref.as_str(),
                    record.idempotency_key.as_str(),
                    contract_name(&record.state)?,
                    record.digest.to_string(),
                    bytes,
                    transitioned_at.as_str(),
                ],
            )
            .map_err(persistence)?;
        transaction
            .execute(
                "INSERT INTO external_submission_transitions(
                    submission_ref, ordinal, prior_state, new_state, record_digest,
                    record_json, transitioned_at
                 ) VALUES (?1, 1, NULL, ?2, ?3, ?4, ?5)",
                params![
                    record.submission_id.as_str(),
                    contract_name(&record.state)?,
                    record.digest.to_string(),
                    canonical_bytes(&record)?,
                    transitioned_at.as_str(),
                ],
            )
            .map_err(persistence)?;
        transaction.commit().map_err(persistence)?;
        Ok(SubmissionDisposition::Recorded)
    }

    fn transition_submission(
        &mut self,
        expected_state: ExternalSubmissionState,
        replacement: ExternalSubmissionRecord,
        transitioned_at: &LedgerTimestamp,
    ) -> Result<SubmissionDisposition, StoreError> {
        validate_external_submission_integrity(&replacement)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(persistence)?;
        let current_bytes = transaction
            .query_row(
                "SELECT record_json FROM external_submissions WHERE submission_id = ?1",
                [replacement.submission_id.as_str()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(persistence)?
            .ok_or_else(|| StoreError::InvalidCommit {
                message: "external submission does not exist".to_owned(),
            })?;
        let current: ExternalSubmissionRecord = serde_json::from_slice(&current_bytes)?;
        validate_external_submission_integrity(&current)?;
        if current.digest == replacement.digest {
            return Ok(SubmissionDisposition::Replayed);
        }
        if current.state != expected_state {
            return Err(StoreError::SubmissionStateConflict {
                expected: contract_name(&expected_state)?,
                actual: contract_name(&current.state)?,
            });
        }
        validate_submission_identity(&current, &replacement)?;
        let ordinal: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) + 1
                 FROM external_submission_transitions WHERE submission_ref = ?1",
                [replacement.submission_id.as_str()],
                |row| row.get(0),
            )
            .map_err(persistence)?;
        let replacement_bytes = canonical_bytes(&replacement)?;
        transaction
            .execute(
                "UPDATE external_submissions
                 SET state = ?2, record_digest = ?3, record_json = ?4, updated_at = ?5
                 WHERE submission_id = ?1",
                params![
                    replacement.submission_id.as_str(),
                    contract_name(&replacement.state)?,
                    replacement.digest.to_string(),
                    replacement_bytes,
                    transitioned_at.as_str(),
                ],
            )
            .map_err(persistence)?;
        transaction
            .execute(
                "INSERT INTO external_submission_transitions(
                    submission_ref, ordinal, prior_state, new_state, record_digest,
                    record_json, transitioned_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    replacement.submission_id.as_str(),
                    ordinal,
                    contract_name(&current.state)?,
                    contract_name(&replacement.state)?,
                    replacement.digest.to_string(),
                    canonical_bytes(&replacement)?,
                    transitioned_at.as_str(),
                ],
            )
            .map_err(persistence)?;
        transaction.commit().map_err(persistence)?;
        Ok(SubmissionDisposition::Recorded)
    }

    fn unresolved_submission_count(&self) -> Result<u64, StoreError> {
        let count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM external_submissions WHERE state = 'OUTCOME_UNKNOWN'",
                [],
                |row| row.get(0),
            )
            .map_err(persistence)?;
        u64::try_from(count).map_err(|_| StoreError::Persistence {
            message: "negative unresolved submission count".to_owned(),
        })
    }
}

fn validate_submission_identity(
    current: &ExternalSubmissionRecord,
    replacement: &ExternalSubmissionRecord,
) -> Result<(), StoreError> {
    if current.schema_version != replacement.schema_version
        || current.submission_id != replacement.submission_id
        || current.command_ref != replacement.command_ref
        || current.idempotency_key != replacement.idempotency_key
        || current.prepared_at != replacement.prepared_at
        || replacement.attempt_count < current.attempt_count
    {
        return Err(StoreError::InvalidCommit {
            message: "submission transition changed immutable identity or regressed attempts"
                .to_owned(),
        });
    }
    Ok(())
}

fn decode_optional(bytes: Option<Vec<u8>>) -> Result<Option<ExternalSubmissionRecord>, StoreError> {
    bytes
        .map(|bytes| {
            let record: ExternalSubmissionRecord = serde_json::from_slice(&bytes)?;
            validate_external_submission_integrity(&record)?;
            Ok(record)
        })
        .transpose()
}

fn latest_record_time(record: &ExternalSubmissionRecord) -> &LedgerTimestamp {
    record
        .reconciled_at
        .as_ref()
        .or(record.submitted_at.as_ref())
        .unwrap_or(&record.prepared_at)
}

fn canonical_bytes(record: &ExternalSubmissionRecord) -> Result<Vec<u8>, StoreError> {
    to_canonical_vec(record).map_err(|error| StoreError::Persistence {
        message: error.to_string(),
    })
}

fn contract_name<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Persistence {
            message: "closed contract enum did not serialize as text".to_owned(),
        })
}

#[allow(clippy::needless_pass_by_value)] // `Result::map_err` transfers ownership here.
fn persistence(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
