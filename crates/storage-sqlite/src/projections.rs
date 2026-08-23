use project_ledger_canonical_json::{Sha256Digest, to_canonical_vec};
use project_ledger_domain::{ProjectId, ProjectState};
use project_ledger_event_store::{
    ProjectStateSnapshot, ProjectionSnapshotLoad, ProjectionSnapshotStore, StoreError,
};
use rusqlite::{OptionalExtension, params};

use crate::SqliteEventStore;

impl ProjectionSnapshotStore for SqliteEventStore {
    fn load_project_state_snapshot(
        &self,
        project_ref: &ProjectId,
    ) -> Result<ProjectionSnapshotLoad, StoreError> {
        let stored = self
            .connection
            .query_row(
                "SELECT state_version, state_digest, source_head_event_digest,
                        projector_version, state_json
                 FROM project_state_snapshots WHERE project_ref = ?1",
                [project_ref.as_str()],
                |row| {
                    Ok(StoredSnapshot {
                        state_version: row.get(0)?,
                        state_digest: row.get(1)?,
                        source_head_event_digest: row.get(2)?,
                        projector_version: row.get(3)?,
                        state_json: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(persistence)?;
        let Some(stored) = stored else {
            return Ok(ProjectionSnapshotLoad::Missing);
        };
        Ok(decode_snapshot(project_ref, &stored))
    }

    fn store_project_state_snapshot(
        &mut self,
        snapshot: &ProjectStateSnapshot,
    ) -> Result<(), StoreError> {
        if snapshot.projector_version == 0 || snapshot.state.state_version == 0 {
            return Err(StoreError::Persistence {
                message: "projection snapshot versions must be positive".to_owned(),
            });
        }
        let state_version =
            i64::try_from(snapshot.state.state_version).map_err(|_| StoreError::Persistence {
                message: "projection state version exceeds SQLite integer range".to_owned(),
            })?;
        self.connection
            .execute(
                "INSERT INTO project_state_snapshots(
                    project_ref, state_version, state_digest, source_head_event_digest,
                    projector_version, state_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(project_ref) DO UPDATE SET
                    state_version = excluded.state_version,
                    state_digest = excluded.state_digest,
                    source_head_event_digest = excluded.source_head_event_digest,
                    projector_version = excluded.projector_version,
                    state_json = excluded.state_json",
                params![
                    snapshot.state.project.project_id.as_str(),
                    state_version,
                    snapshot.state.state_digest.to_string(),
                    snapshot.source_head_event_digest.to_string(),
                    snapshot.projector_version,
                    to_canonical_vec(&snapshot.state).map_err(|error| StoreError::Persistence {
                        message: error.to_string(),
                    })?,
                ],
            )
            .map_err(persistence)?;
        Ok(())
    }
}

#[derive(Debug)]
struct StoredSnapshot {
    state_version: i64,
    state_digest: String,
    source_head_event_digest: String,
    projector_version: i64,
    state_json: Vec<u8>,
}

fn decode_snapshot(project_ref: &ProjectId, stored: &StoredSnapshot) -> ProjectionSnapshotLoad {
    let Ok(state_version) = u64::try_from(stored.state_version) else {
        return ProjectionSnapshotLoad::Corrupt;
    };
    let Ok(projector_version) = u32::try_from(stored.projector_version) else {
        return ProjectionSnapshotLoad::Corrupt;
    };
    let Ok(source_head_event_digest) = Sha256Digest::parse(&stored.source_head_event_digest) else {
        return ProjectionSnapshotLoad::Corrupt;
    };
    let Ok(state) = serde_json::from_slice::<ProjectState>(&stored.state_json) else {
        return ProjectionSnapshotLoad::Corrupt;
    };
    if state.project.project_id != *project_ref
        || state.state_version != state_version
        || state.state_digest.to_string() != stored.state_digest
        || projector_version == 0
    {
        return ProjectionSnapshotLoad::Corrupt;
    }
    ProjectionSnapshotLoad::Snapshot(Box::new(ProjectStateSnapshot {
        state,
        source_head_event_digest,
        projector_version,
    }))
}

#[allow(clippy::needless_pass_by_value)] // `Result::map_err` transfers ownership here.
fn persistence(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
