//! Bounded native command/result/scope/event read in one `SQLite` snapshot.
use project_ledger_domain::{CommandEnvelope, CommandId, CommandResult, EventEnvelope, ProjectId};
use project_ledger_event_store::{EventStore, StoreError, validate_command_integrity};
use rusqlite::OptionalExtension;
use thiserror::Error;

use crate::SqliteEventStore;

/// Native records from one consistent transaction; no externally supplied payloads.
#[derive(Debug)]
pub struct CommandSnapshot {
    pub command: Option<CommandEnvelope>,
    pub result: Option<CommandResult>,
    pub events: Vec<EventEnvelope>,
}

/// A resource limit is distinct from corruption or unavailable storage.
#[derive(Debug, Error)]
pub enum ConfirmationSnapshotError {
    #[error("confirmation snapshot exceeds read budget")]
    ReadLimitExceeded,
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl SqliteEventStore {
    /// Loads a bounded, verified command, result, idempotency scope and stream.
    ///
    /// # Errors
    /// Fails on any contradictory indexed metadata, chain corruption or read limit.
    pub fn load_command_snapshot(
        &self,
        project: &ProjectId,
        command: &CommandId,
    ) -> Result<CommandSnapshot, ConfirmationSnapshotError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(persistence)?;
        let snapshot = self.command_snapshot_in_transaction(project, command)?;
        transaction.commit().map_err(persistence)?;
        Ok(snapshot)
    }

    fn command_snapshot_in_transaction(
        &self,
        project: &ProjectId,
        command_id: &CommandId,
    ) -> Result<CommandSnapshot, ConfirmationSnapshotError> {
        let (count, bytes): (i64, i64) = self.connection.query_row(
            "SELECT (SELECT COUNT(*) FROM events WHERE project_ref=?1),
             (SELECT COALESCE(SUM(length(event_json)),0) FROM events WHERE project_ref=?1)
             + (SELECT COALESCE(SUM(length(command_json)),0) FROM commands WHERE command_id=?2)
             + (SELECT COALESCE(SUM(length(result_json)),0) FROM command_results WHERE command_ref=?2)",
            rusqlite::params![project.as_str(), command_id.as_str()],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(persistence)?;
        if count > 100_000 || bytes > 64 * 1024 * 1024 {
            return Err(ConfirmationSnapshotError::ReadLimitExceeded);
        }
        let stored = self
            .connection
            .query_row(
                "SELECT command_json,command_id,project_ref,command_type,actor_id,trust_domain,
             idempotency_key,CAST(expected_state_version AS TEXT),command_digest,submitted_at
             FROM commands WHERE command_id=?1",
                [command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        (1..=9)
                            .map(|i| row.get::<_, String>(i))
                            .collect::<rusqlite::Result<Vec<_>>>()?,
                    ))
                },
            )
            .optional()
            .map_err(persistence)?;
        let Some((bytes, index)) = stored else {
            return Ok(CommandSnapshot {
                command: None,
                result: None,
                events: vec![],
            });
        };
        let command: CommandEnvelope = serde_json::from_slice(&bytes).map_err(StoreError::from)?;
        validate_command_integrity(&command).map_err(StoreError::from)?;
        let kind = contract_name(&command.command_type())?;
        let trust = contract_name(&command.actor.trust_level)?;
        if command.command_id != *command_id
            || index
                != [
                    command.command_id.to_string(),
                    command.project_ref.to_string(),
                    kind.clone(),
                    command.actor.actor_id.to_string(),
                    trust.clone(),
                    command.idempotency_key.to_string(),
                    command.expected_state_version.to_string(),
                    command.command_digest.to_string(),
                    command.submitted_at.to_string(),
                ]
        {
            return Err(corrupt("command indices disagree with canonical command"));
        }
        let result = self.command_result(command_id)?;
        if let Some(result) = &result {
            let result_index=self.connection.query_row(
                "SELECT command_result_id,status,processed_at FROM command_results WHERE command_ref=?1",[command_id.as_str()],
                |row|(0..3).map(|i|row.get::<_,String>(i)).collect::<rusqlite::Result<Vec<_>>>(),
            ).map_err(persistence)?;
            if result.command_ref != *command_id
                || result_index
                    != [
                        result.command_result_id.to_string(),
                        contract_name(&result.status)?,
                        result.processed_at.to_string(),
                    ]
            {
                return Err(corrupt("result indices disagree with canonical result"));
            }
            let scope=self.connection.query_row(
                "SELECT project_ref,command_type,actor_id,trust_domain,idempotency_key,command_digest,command_ref,command_result_ref
                 FROM idempotency_keys WHERE command_ref=?1",[command_id.as_str()],
                |row|(0..8).map(|i|row.get::<_,String>(i)).collect::<rusqlite::Result<Vec<_>>>(),
            ).optional().map_err(persistence)?;
            let expected = vec![
                command.project_ref.to_string(),
                kind,
                command.actor.actor_id.to_string(),
                trust,
                command.idempotency_key.to_string(),
                command.command_digest.to_string(),
                command_id.to_string(),
                result.command_result_id.to_string(),
            ];
            if scope.as_ref() != Some(&expected) {
                return Err(corrupt("idempotency scope disagrees with command/result"));
            }
        }
        self.verify_event_chain(project)?;
        let events = self.load_stream(project)?;
        Ok(CommandSnapshot {
            command: Some(command),
            result,
            events,
        })
    }
}

#[allow(clippy::needless_pass_by_value)] // Direct map_err adapter consumes the error.
fn persistence(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
fn corrupt(message: &str) -> ConfirmationSnapshotError {
    StoreError::CorruptStream {
        version: None,
        message: message.to_owned(),
    }
    .into()
}
fn contract_name(value: &impl serde::Serialize) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::InvalidCommit {
            message: "invalid native enum".into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{create_commit, metadata_commit, project_id};

    #[test]
    fn pinned_wal_snapshot_keeps_command_result_and_head_together_during_another_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.db");
        let mut writer = SqliteEventStore::open(&path).unwrap();
        let initial = create_commit();
        let command = initial.command.command_id.clone();
        let head = initial.events[0].event_digest;
        let durable = initial.result.clone();
        writer.commit_command(initial).unwrap();
        let reader = SqliteEventStore::open_read_only(&path).unwrap();
        let transaction = reader.connection.unchecked_transaction().unwrap();
        assert_eq!(reader.current_version(&project_id()).unwrap(), 1);
        writer.commit_command(metadata_commit(head, 16)).unwrap();
        let pinned = reader
            .command_snapshot_in_transaction(&project_id(), &command)
            .unwrap();
        assert_eq!(pinned.events.len(), 1);
        assert_eq!(pinned.result.as_ref(), Some(&durable));
        assert_eq!(pinned.events[0].event_digest, head);
        transaction.commit().unwrap();
        let fresh = reader
            .load_command_snapshot(&project_id(), &command)
            .unwrap();
        assert_eq!(fresh.events.len(), 2);
        assert_eq!(fresh.result, Some(durable));
        assert_eq!(writer.current_version(&project_id()).unwrap(), 2);
    }

    #[test]
    fn snapshot_rejects_excessive_json_before_decoding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("budget.db");
        let mut store = SqliteEventStore::open(&path).unwrap();
        let commit = create_commit();
        let command = commit.command.command_id.clone();
        store.commit_command(commit).unwrap();
        store
            .connection
            .execute("UPDATE commands SET command_json=zeroblob(67108865)", [])
            .unwrap();
        assert!(matches!(
            store.load_command_snapshot(&project_id(), &command),
            Err(ConfirmationSnapshotError::ReadLimitExceeded)
        ));
    }
}
