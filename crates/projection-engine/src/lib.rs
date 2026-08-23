//! Deterministic, storage-independent projections rebuilt from canonical events.

mod lifecycle;
mod project_state;

pub use lifecycle::ProjectStateProjection;
pub use project_state::{
    PROJECTOR_VERSION, ProjectStateProjector, ProjectionError, validate_state_integrity,
};
