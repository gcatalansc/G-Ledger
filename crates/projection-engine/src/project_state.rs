use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{Event, EventEnvelope, ProjectState, SchemaVersion, StableId};
use project_ledger_event_store::validate_event_integrity;
use serde::Serialize;
use thiserror::Error;

/// Version of the deterministic `ProjectState` fold implemented by this crate.
pub const PROJECTOR_VERSION: u32 = 1;

/// Failures that prevent a stream or event from becoming trusted derived state.
#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("a project-state rebuild requires a non-empty event stream")]
    EmptyStream,
    #[error("stream genesis must be PROJECT_CREATED at version 1 without a prior digest")]
    InvalidGenesis,
    #[error("event integrity validation failed: {0}")]
    EventIntegrity(String),
    #[error("project-state identifier derivation failed")]
    InvalidStateIdentifier,
    #[error("only Project Ledger contract version 0.2.0 can be projected")]
    UnsupportedSchemaVersion,
    #[error("project-state identity does not match its project")]
    StateIdentityMismatch,
    #[error("project-state versions are semantically invalid")]
    InvalidStateVersion,
    #[error("event belongs to a different project or stream")]
    ProjectMismatch,
    #[error("event sequence is invalid: {message}")]
    InvalidSequence { message: String },
    #[error("PROJECT_CREATED may appear only as stream genesis")]
    DuplicateGenesis,
    #[error("project-state serialization failed: {0}")]
    StateSerialization(String),
    #[error("canonical project-state hashing failed: {0}")]
    Canonical(#[from] CanonicalJsonError),
    #[error("project-state digest mismatch")]
    StateDigestMismatch,
    #[error("event type is not supported by projector version {PROJECTOR_VERSION}")]
    UnsupportedEvent,
}

/// Deterministic fold from canonical events to the current project state.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectStateProjector;

impl ProjectStateProjector {
    /// Rebuilds `ProjectState` from event-stream genesis.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError`] for empty, invalid, inconsistent or unsupported input.
    pub fn rebuild(events: &[EventEnvelope]) -> Result<ProjectState, ProjectionError> {
        let Some(genesis) = events.first() else {
            return Err(ProjectionError::EmptyStream);
        };
        validate_event_integrity(genesis)
            .map_err(|error| ProjectionError::EventIntegrity(error.to_string()))?;
        if genesis.schema_version != SchemaVersion::V0_2_0 {
            return Err(ProjectionError::UnsupportedSchemaVersion);
        }
        if genesis.stream_version != 1
            || genesis.previous_event_digest.is_some()
            || genesis.project_ref != genesis.stream_id
        {
            return Err(ProjectionError::InvalidGenesis);
        }
        let Event::ProjectCreated(created) = &genesis.event else {
            return Err(ProjectionError::InvalidGenesis);
        };
        if created.project.project_id != genesis.project_ref {
            return Err(ProjectionError::InvalidGenesis);
        }
        let state_id = state_id(&genesis.project_ref)?;
        let mut state = ProjectState {
            schema_version: SchemaVersion::V0_2_0,
            project_state_id: state_id,
            project: created.project.clone(),
            state_version: 1,
            material_state_version: 1,
            authority_documents: Vec::new(),
            roadmap: None,
            active_phase_refs: Vec::new(),
            primary_active_phase_ref: None,
            completed_phase_refs: Vec::new(),
            pending_gate_refs: Vec::new(),
            verified_gate_refs: Vec::new(),
            revalidation_required_gate_refs: Vec::new(),
            current_decision_refs: Vec::new(),
            open_blocker_refs: Vec::new(),
            open_risk_refs: Vec::new(),
            ordered_next_action_refs: Vec::new(),
            preferred_next_action_ref: None,
            checkpoint_ref: None,
            last_material_event_ref: Some(genesis.event_id.clone()),
            last_material_activity_at: Some(genesis.occurred_at.clone()),
            open_conflict_refs: Vec::new(),
            updated_at: genesis.recorded_at.clone(),
            state_digest: Sha256Digest::from([0_u8; 32]),
        };
        state.state_digest = state_digest(&state)?;

        let mut head_digest = genesis.event_digest;
        for event in &events[1..] {
            state = Self::apply(&state, head_digest, event)?;
            head_digest = event.event_digest;
        }
        Ok(state)
    }

    /// Applies the exact next event to integrity-valid projected state.
    ///
    /// The caller supplies the verified canonical stream head because
    /// `ProjectState` intentionally does not duplicate that event-store field.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionError`] for a corrupt state/event, wrong project or
    /// head, noncontiguous version, duplicate genesis, or unsupported reducer.
    pub fn apply(
        state: &ProjectState,
        expected_previous_event_digest: Sha256Digest,
        event: &EventEnvelope,
    ) -> Result<ProjectState, ProjectionError> {
        validate_state_integrity(state)?;
        validate_event_integrity(event)
            .map_err(|error| ProjectionError::EventIntegrity(error.to_string()))?;
        if event.schema_version != SchemaVersion::V0_2_0 {
            return Err(ProjectionError::UnsupportedSchemaVersion);
        }
        if event.project_ref != state.project.project_id
            || event.stream_id != state.project.project_id
            || state.project_state_id != state_id(&state.project.project_id)?
        {
            return Err(ProjectionError::ProjectMismatch);
        }
        let expected_version =
            state
                .state_version
                .checked_add(1)
                .ok_or_else(|| ProjectionError::InvalidSequence {
                    message: "state version cannot advance beyond u64::MAX".to_owned(),
                })?;
        if event.stream_version != expected_version
            || event.previous_event_digest != Some(expected_previous_event_digest)
        {
            return Err(ProjectionError::InvalidSequence {
                message: format!(
                    "expected version {expected_version} chained to {expected_previous_event_digest}, found version {} chained to {:?}",
                    event.stream_version, event.previous_event_digest
                ),
            });
        }

        let mut next = state.clone();
        match &event.event {
            Event::ProjectCreated(_) => return Err(ProjectionError::DuplicateGenesis),
            Event::ProjectMetadataUpdated(metadata) => {
                if let Some(title) = &metadata.title {
                    next.project.title.clone_from(title);
                }
                if let Some(summary) = &metadata.summary {
                    next.project.summary.clone_from(summary);
                }
            }
            _ => return Err(ProjectionError::UnsupportedEvent),
        }
        next.state_version = event.stream_version;
        next.material_state_version = event.stream_version;
        next.last_material_event_ref = Some(event.event_id.clone());
        next.last_material_activity_at = Some(event.occurred_at.clone());
        next.updated_at.clone_from(&event.recorded_at);
        next.state_digest = state_digest(&next)?;
        Ok(next)
    }
}

fn state_id(project_ref: &project_ledger_domain::ProjectId) -> Result<StableId, ProjectionError> {
    StableId::parse(&format!("state:{}", project_ref.uuid()))
        .map_err(|_| ProjectionError::InvalidStateIdentifier)
}

/// Verifies that a projected state still matches its canonical digest.
///
/// # Errors
///
/// Returns [`ProjectionError::StateDigestMismatch`] when any covered field changed.
pub fn validate_state_integrity(state: &ProjectState) -> Result<(), ProjectionError> {
    if state.schema_version != SchemaVersion::V0_2_0 {
        return Err(ProjectionError::UnsupportedSchemaVersion);
    }
    if state.project_state_id != state_id(&state.project.project_id)? {
        return Err(ProjectionError::StateIdentityMismatch);
    }
    if state.state_version == 0
        || state.material_state_version == 0
        || state.material_state_version > state.state_version
    {
        return Err(ProjectionError::InvalidStateVersion);
    }
    if state_digest(state)? != state.state_digest {
        return Err(ProjectionError::StateDigestMismatch);
    }
    Ok(())
}

fn state_digest<T: Serialize>(state: &T) -> Result<Sha256Digest, ProjectionError> {
    let mut value = serde_json::to_value(state)
        .map_err(|error| ProjectionError::StateSerialization(error.to_string()))?;
    value
        .as_object_mut()
        .ok_or_else(|| {
            ProjectionError::StateSerialization(
                "project state must serialize as a JSON object".to_owned(),
            )
        })?
        .remove("state_digest");
    Ok(sha256_digest(&value)?)
}
