use project_ledger_domain::{ProjectId, ProjectState};
use project_ledger_event_store::{
    EventStore, ProjectStateSnapshot, ProjectionSnapshotLoad, ProjectionSnapshotStore, StoreError,
};
use project_ledger_projection_engine::{
    PROJECTOR_VERSION, ProjectStateProjector, ProjectionError, validate_state_integrity,
};
use thiserror::Error;

/// Origin of the state returned by a query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionSource {
    Snapshot,
    Rebuilt,
}

/// One trusted project-state read and whether cache recovery was required.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectStateRead {
    pub state: ProjectState,
    pub source: ProjectionSource,
}

/// Failures while reading canonical events or rebuilding derived state.
#[derive(Debug, Error)]
pub enum ProjectStateQueryError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Projection(#[from] ProjectionError),
}

/// Side-effect-bounded query over canonical events and a discardable snapshot cache.
#[derive(Debug)]
pub struct ProjectStateQuery<'a, S> {
    store: &'a mut S,
}

impl<'a, S> ProjectStateQuery<'a, S>
where
    S: EventStore + ProjectionSnapshotStore,
{
    pub const fn new(store: &'a mut S) -> Self {
        Self { store }
    }

    /// Loads a matching snapshot or rebuilds it from the verified event stream.
    ///
    /// Snapshot replacement is the only permitted write; canonical event count
    /// and stream version are never changed by this service.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectStateQueryError`] when canonical storage is corrupt or
    /// the current projector cannot fold the stream.
    pub fn load(
        &mut self,
        project_ref: &ProjectId,
    ) -> Result<ProjectStateRead, ProjectStateQueryError> {
        self.store.verify_event_chain(project_ref)?;
        let events = self.store.load_stream(project_ref)?;
        let Some(head) = events.last() else {
            return Err(ProjectionError::EmptyStream.into());
        };

        if let ProjectionSnapshotLoad::Snapshot(snapshot) =
            self.store.load_project_state_snapshot(project_ref)?
            && snapshot_is_current(
                &snapshot,
                project_ref,
                head.stream_version,
                head.event_digest,
            )
        {
            return Ok(ProjectStateRead {
                state: snapshot.state,
                source: ProjectionSource::Snapshot,
            });
        }

        let state = ProjectStateProjector::rebuild(&events)?;
        self.store
            .store_project_state_snapshot(&ProjectStateSnapshot {
                state: state.clone(),
                source_head_event_digest: head.event_digest,
                projector_version: PROJECTOR_VERSION,
            })?;
        Ok(ProjectStateRead {
            state,
            source: ProjectionSource::Rebuilt,
        })
    }
}

fn snapshot_is_current(
    snapshot: &ProjectStateSnapshot,
    project_ref: &ProjectId,
    stream_version: u64,
    head_digest: project_ledger_canonical_json::Sha256Digest,
) -> bool {
    snapshot.projector_version == PROJECTOR_VERSION
        && snapshot.source_head_event_digest == head_digest
        && snapshot.state.project.project_id == *project_ref
        && snapshot.state.state_version == stream_version
        && validate_state_integrity(&snapshot.state).is_ok()
}
