//! Offline JSON Schema registry and deterministic fixture generation.

mod catalog;
mod fixtures;
mod generation;
mod history;
mod matrix;
mod verification;

pub use catalog::{SchemaEntry, SchemaError, schema_catalog};
pub use fixtures::{FixtureEntry, fixture_catalog};
pub use generation::{
    GenerationError, check_generated, generated_files, history_generated_files, write_generated,
    write_history_generated,
};
pub use history::schema_catalog_v0_3;
pub use matrix::{
    CommandEventMatrix, CommandEventRow, DerivedEventOwner, EngineOwner, EventOutcome,
    command_event_matrix, command_event_matrix_v0_3,
};
pub use verification::{ContractValidationReport, validate_contracts};
