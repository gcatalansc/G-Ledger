//! Project Radar local artifact profile interoperability.

mod contracts;
mod export;
mod import;

pub use contracts::{
    RADAR_FIXTURE_MANIFEST_SHA256, RADAR_PROFILE, RADAR_SCHEMA_REGISTRY_SHA256,
    RADAR_SOURCE_SPEC_SHA256, RadarContractCatalog, RadarContractDescriptor, RadarContractError,
    RadarContractVerificationReport, verify_profile_contracts,
};
pub use export::{
    RadarEnvelopeRequest, RadarExportContext, RadarExportError, RadarStatusSuggestion,
    build_project_projection, build_sync_envelope, suggested_radar_status,
};
pub use import::{
    RadarImportConflict, RadarImportError, RadarImportEvaluation, RadarImportSnapshot,
    RadarMergeDecision, RadarOwnedFields, evaluate_import, validate_sync_envelope,
};
