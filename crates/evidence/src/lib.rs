//! Safe local evidence execution and trust-preserving import.

mod import;
mod runner;
pub mod supervisor;

pub use import::{EvidenceImportContext, EvidenceImporter, EvidenceImporterError};
pub use runner::{
    EvidenceCommand, EvidenceParser, EvidenceRunContext, EvidenceRunPolicy, EvidenceRunner,
    EvidenceRunnerError, ParsedEvidence,
};
