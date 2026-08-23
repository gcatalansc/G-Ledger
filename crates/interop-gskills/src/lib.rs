#![forbid(unsafe_code)]

//! Exact read-only G-Skills v0.3.0 adapter boundary.

mod conformance;
mod contracts;
mod mapping;
mod provider;
mod sink;
mod sink_conformance;
mod sink_mapping;
mod sink_model;
mod sink_stdio;
mod stdio;

pub use conformance::{
    ConformanceError, GSkillsConformanceCheck, GSkillsReadConformanceReport,
    GSkillsReadConformanceSuite, GSkillsReadProvider, malicious_gskills_requests,
    run_gskills_read_conformance,
};

pub use contracts::{
    ContractError, ContractVerificationReport, GSKILLS_FIXTURE_MANIFEST_SHA256, GSKILLS_PROFILE,
    GSKILLS_REGISTRY_SHA256, GSKILLS_SPEC_REVISION, GSKILLS_SPEC_SHA256, LEDGER_SPEC_SHA256,
    RECIPROCAL_LOCK_SHA256, VENDORED_GSKILLS_CONTRACTS, VENDORED_READ_CONTRACTS,
    VendoredReadContract, validate_gskills_contract, validate_read_contract,
    verify_vendored_contracts,
};
pub use provider::{
    GSkillsObservableState, GSkillsProjectionResult, GSkillsProvider, ProviderContext,
    ProviderError,
};
pub use sink::GSkillsRecordSink;
pub use sink_conformance::{
    GSkillsRecordSinkConformanceCheck, GSkillsRecordSinkConformanceReport,
    GSkillsRecordSinkConformanceSuite, RecordSinkConformanceError,
    run_gskills_record_sink_conformance,
};
pub use sink_model::{
    PreparedRecordCommand, RecordCommandKind, RecordSinkError, RecordSinkHost, RecordSinkHostError,
    RecordSinkPolicy, RecordSinkStateIdentity,
};
pub use sink_stdio::GSkillsRecordSinkJsonRpcHandler;
pub use stdio::GSkillsJsonRpcHandler;
