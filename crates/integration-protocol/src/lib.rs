//! Storage-independent protocol and conformance SDK for Project Ledger integrations.

mod conformance;
mod identity;
mod integrity;
mod upstream_lock;

pub use conformance::{
    ConformanceCheck, ConformanceError, ObservableState, ProviderFailure, ReadConformanceReport,
    ReadConformanceSuite, ReadOnlyProvider, malicious_read_requests, run_read_conformance,
};
pub use identity::{IdentityQuery, IdentityResolution, resolve_identity};
pub use integrity::{
    LossReportDraft, ProjectionEnvelopeDraft, ProtocolError, build_default_manifest, build_health,
    build_loss_report, build_projection_envelope, validate_health_integrity,
    validate_loss_report_integrity, validate_manifest_integrity, validate_projection_envelope,
};
pub use upstream_lock::{UpstreamLockInspection, UpstreamLockStatus, inspect_upstream_lock};
