//! Deterministic, storage-independent projections rebuilt from canonical events.

mod episodes;
mod history;
mod lifecycle;
pub use history::HistoryView;
mod project_state;

pub use lifecycle::ProjectStateProjection;
pub use project_state::{
    PROJECTOR_VERSION, ProjectStateProjector, ProjectionError, validate_state_integrity,
};
