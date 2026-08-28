use crate::SqliteEventStore;
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::ProjectId;
use project_ledger_event_store::StoreError;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

/// A reservation never means that a command was accepted or a process succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionReservation {
    Reserved,
    OutcomeUnknown,
    Conflict,
}

fn digest(project: &str, scope: &str, request: &str) -> Result<String, StoreError> {
    sha256_digest(&(project, scope, request))
        .map(|digest| digest.to_string())
        .map_err(|error| StoreError::Persistence {
            message: error.to_string(),
        })
}
#[allow(clippy::needless_pass_by_value)] // Result::map_err transfers the SQLite error.
fn storage(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
fn corrupt() -> StoreError {
    StoreError::CorruptStream {
        version: None,
        message: "execution reservation integrity mismatch".into(),
    }
}

impl SqliteEventStore {
    /// Atomically reserves a full-scope execution key before any external effect.
    /// An existing reservation without a reconciled canonical result must not rerun.
    /// # Errors
    /// Rejects corrupt metadata, unknown projects or storage failures.
    pub fn reserve_execution(
        &mut self,
        project: &ProjectId,
        scope: Sha256Digest,
        request: Sha256Digest,
    ) -> Result<ExecutionReservation, StoreError> {
        let scope = scope.to_string();
        let request = request.to_string();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let existing:Option<(String,String,String)>=transaction.query_row(
            "SELECT project_ref,request_digest,record_digest FROM execution_attempts WHERE scope_digest=?1",[&scope],
            |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(storage)?;
        if let Some((stored_project, stored_request, stored_digest)) = existing {
            if stored_digest != digest(&stored_project, &scope, &stored_request)? {
                return Err(corrupt());
            }
            return Ok(
                if stored_project == project.as_str() && stored_request == request {
                    ExecutionReservation::OutcomeUnknown
                } else {
                    ExecutionReservation::Conflict
                },
            );
        }
        transaction.execute("INSERT INTO execution_attempts(scope_digest,project_ref,request_digest,record_digest) VALUES (?1,?2,?3,?4)",params![scope,project.as_str(),request,digest(project.as_str(),&scope,&request)?]).map_err(storage)?;
        transaction.commit().map_err(storage)?;
        Ok(ExecutionReservation::Reserved)
    }

    pub(crate) fn verify_execution_attempts(&self, project: &ProjectId) -> Result<u64, StoreError> {
        let mut statement=self.connection.prepare("SELECT scope_digest,request_digest,record_digest FROM execution_attempts WHERE project_ref=?1").map_err(storage)?;
        let rows = statement
            .query_map([project.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?;
        let mut count = 0;
        for row in rows {
            let (scope, request, record) = row.map_err(storage)?;
            if record != digest(project.as_str(), &scope, &request)? {
                return Err(corrupt());
            }
            count += 1;
        }
        Ok(count)
    }
}
