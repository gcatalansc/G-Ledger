//! Offline JSON Schema registry and deterministic fixture generation.

mod catalog;
mod fixtures;
mod generation;
mod matrix;
mod verification;

pub use catalog::{SchemaEntry, SchemaError, schema_catalog};
pub use fixtures::{FixtureEntry, fixture_catalog};
pub use generation::{GenerationError, check_generated, generated_files, write_generated};
pub use matrix::{
    CommandEventMatrix, CommandEventRow, DerivedEventOwner, EngineOwner, EventOutcome,
    command_event_matrix,
};
pub use verification::{ContractValidationReport, validate_contracts};
