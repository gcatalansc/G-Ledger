//! Deterministic, storage-independent projections rebuilt from canonical events.

mod project_state;

pub use project_state::{
    PROJECTOR_VERSION, ProjectStateProjector, ProjectionError, validate_state_integrity,
};
