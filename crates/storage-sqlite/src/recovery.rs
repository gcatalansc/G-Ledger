use project_ledger_domain::ProjectId;
use project_ledger_event_store::{
    EventStore, StoreError, SubmissionStore, validate_command_result_integrity,
};

use crate::SqliteEventStore;

/// Read-only recovery evidence for one project database.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    pub quick_check: String,
    pub journal_mode: String,
    pub migration_version: u32,
    pub chain_verified: bool,
    pub current_version: u64,
    pub event_count: u64,
    pub command_count: u64,
    pub command_result_count: u64,
    pub unresolved_submission_count: u64,
}

impl SqliteEventStore {
    /// Verifies `SQLite` structure, migration level, WAL policy, event chain, and
    /// unresolved local submissions without changing canonical state.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when any recovery check is inconsistent or corrupt.
    pub fn recovery_report(&self, project_ref: &ProjectId) -> Result<RecoveryReport, StoreError> {
        let quick_check: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(persistence)?;
        if quick_check != "ok" {
            return Err(StoreError::CorruptStream {
                version: None,
                message: format!("SQLite quick_check returned {quick_check}"),
            });
        }
        let journal_mode: String = self
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(persistence)?;
        if journal_mode != "wal" {
            return Err(StoreError::Persistence {
                message: format!("recovery requires WAL journal mode, found {journal_mode}"),
            });
        }
        let migration_version: u32 = self
            .connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(persistence)?;
        self.verify_event_chain(project_ref)?;
        let current_version = self.current_version(project_ref)?;
        let event_count = self.event_count(project_ref)?;
        if event_count != current_version {
            return Err(StoreError::CorruptStream {
                version: Some(current_version),
                message: format!("event count {event_count} differs from stream version"),
            });
        }
        let command_count = project_count(
            &self.connection,
            "SELECT COUNT(*) FROM commands WHERE project_ref = ?1",
            project_ref,
        )?;
        let command_result_count = project_count(
            &self.connection,
            "SELECT COUNT(*) FROM command_results r
             JOIN commands c ON c.command_id = r.command_ref
             WHERE c.project_ref = ?1",
            project_ref,
        )?;
        verify_command_results(&self.connection, project_ref)?;
        let unresolved_submission_count = self.unresolved_submission_count()?;
        Ok(RecoveryReport {
            quick_check,
            journal_mode,
            migration_version,
            chain_verified: true,
            current_version,
            event_count,
            command_count,
            command_result_count,
            unresolved_submission_count,
        })
    }
}

fn verify_command_results(
    connection: &rusqlite::Connection,
    project_ref: &ProjectId,
) -> Result<(), StoreError> {
    let mut statement = connection
        .prepare(
            "SELECT r.result_digest, r.result_json
             FROM command_results r
             JOIN commands c ON c.command_id = r.command_ref
             WHERE c.project_ref = ?1",
        )
        .map_err(persistence)?;
    let rows = statement
        .query_map([project_ref.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(persistence)?;
    for row in rows {
        let (stored_digest, bytes) = row.map_err(persistence)?;
        let result: project_ledger_domain::CommandResult = serde_json::from_slice(&bytes)?;
        validate_command_result_integrity(&result).map_err(|error| StoreError::CorruptStream {
            version: None,
            message: format!("command-result integrity failed during recovery: {error}"),
        })?;
        if stored_digest != result.digest.to_string() {
            return Err(StoreError::CorruptStream {
                version: None,
                message: "indexed command-result digest disagrees during recovery".to_owned(),
            });
        }
    }
    Ok(())
}

fn project_count(
    connection: &rusqlite::Connection,
    sql: &str,
    project_ref: &ProjectId,
) -> Result<u64, StoreError> {
    let count: i64 = connection
        .query_row(sql, [project_ref.as_str()], |row| row.get(0))
        .map_err(persistence)?;
    u64::try_from(count).map_err(|_| StoreError::Persistence {
        message: "negative recovery row count".to_owned(),
    })
}

#[allow(clippy::needless_pass_by_value)] // `Result::map_err` transfers ownership here.
fn persistence(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
