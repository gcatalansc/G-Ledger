//! Exact-contract GSP bridge for Project Ledger.

mod bridge;
mod contracts;
mod corpus;
mod result;

pub use bridge::{GspBridge, GspBridgeError, GspBridgeOutput, GspCompilerSidecar, GspSidecarError};

pub use contracts::{
    GSP_ARCHIVE_SHA256, GSP_CONTRACT_COMMIT, GSP_CONTRACT_TAG, GSP_FIXTURE_MANIFEST_SHA256,
    GSP_IMPLEMENTATION_COMMIT, GSP_IMPLEMENTATION_SPEC_SHA256, GSP_IMPLEMENTATION_VERSION,
    GSP_SCHEMA_REGISTRY_SHA256, GspContractCatalog, GspContractError,
    GspContractVerificationReport, verify_vendored_contracts,
};
pub use corpus::{
    GspCorpus, GspCorpusBudget, GspCorpusEntry, GspCorpusError, GspCorpusManifest,
    GspCorpusRequest, build_minimized_corpus,
};
pub use result::{
    GspArtifactReceipt, GspBridgeReport, GspCompilationReceipt, GspCompilationResult,
    GspConflictVisibility, GspRuntimeValidation,
};
