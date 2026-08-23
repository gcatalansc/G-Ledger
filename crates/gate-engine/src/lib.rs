//! Deterministic declarative Gate and Phase engines.

mod engine;
mod predicate;

pub use engine::{GateEngine, GateEngineError};
pub use predicate::evaluate_predicate;
