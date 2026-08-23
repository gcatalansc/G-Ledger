use std::time::{Duration, Instant};

use project_ledger_canonical_json::{Sha256Digest, to_canonical_vec};
use project_ledger_domain::{
    FreshnessStatus, IntegrationManifest, ProjectionEnvelope, ProjectionRequest,
    ProviderHealthReport,
};
use serde::Serialize;
use thiserror::Error;

use crate::{validate_health_integrity, validate_manifest_integrity, validate_projection_envelope};

/// Canonical state visible to a conformance harness before and after provider calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ObservableState {
    pub state_version: u64,
    pub event_count: u64,
    pub state_digest: Sha256Digest,
}

/// A typed provider failure that remains separate from domain command results.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{code}: {message}")]
pub struct ProviderFailure {
    pub code: String,
    pub message: String,
}

impl ProviderFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Storage-independent read surface exercised by the generic SDK.
pub trait ReadOnlyProvider {
    /// Captures the canonical tuple whose mutation the harness can observe.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderFailure`] when no trustworthy snapshot is available.
    fn observable_state(&self) -> Result<ObservableState, ProviderFailure>;
    /// Returns the provider manifest without side effects.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderFailure`] when the manifest cannot be served.
    fn manifest(&self) -> Result<IntegrationManifest, ProviderFailure>;
    /// Returns provider health without side effects.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderFailure`] when health cannot be evaluated.
    fn health(&self) -> Result<ProviderHealthReport, ProviderFailure>;
    /// Returns one bounded projection for an exact request.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderFailure`] when negotiation, identity, policy or runtime fails.
    fn project(&self, request: &ProjectionRequest) -> Result<ProjectionEnvelope, ProviderFailure>;
}

/// Complete input corpus for one read-only conformance run.
#[derive(Clone, Debug)]
pub struct ReadConformanceSuite {
    pub current_request: ProjectionRequest,
    pub stale_request: ProjectionRequest,
    pub malicious_requests: Vec<ProjectionRequest>,
    pub projection_repetitions: u32,
    pub max_call_duration: Duration,
}

/// One stable check entry emitted by a successful harness run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceCheck {
    pub code: String,
    pub passed: bool,
}

/// Evidence summary for a successful read-only conformance run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadConformanceReport {
    pub checks: Vec<ConformanceCheck>,
    pub projection_calls: u32,
    pub rejected_malicious: u32,
    pub observable_mutations: u32,
}

/// Failures that prevent a provider from claiming generic read conformance.
#[derive(Debug, Error)]
pub enum ConformanceError {
    #[error("invalid conformance suite: {message}")]
    InvalidSuite { message: &'static str },
    #[error("provider call {operation} failed with {code}: {message}")]
    Provider {
        operation: &'static str,
        code: String,
        message: String,
    },
    #[error("provider call {operation} exceeded timeout {limit_ms}ms with {actual_ms}ms")]
    Timeout {
        operation: &'static str,
        limit_ms: u128,
        actual_ms: u128,
    },
    #[error("provider call {operation} mutated observable state")]
    ObservableMutation { operation: &'static str },
    #[error("provider response {operation} is invalid: {message}")]
    InvalidResponse {
        operation: &'static str,
        message: String,
    },
    #[error("provider response changed for identical request and observable state")]
    NondeterministicProjection,
    #[error("malicious request {index} was accepted")]
    MaliciousAccepted { index: usize },
}

/// Returns the generic hostile request corpus required by the S8 gate.
pub fn malicious_read_requests(base: &ProjectionRequest) -> Vec<ProjectionRequest> {
    let mut unsupported_protocol = base.clone();
    "9.9.9".clone_into(&mut unsupported_protocol.protocol.version);

    let mut zero_budget = base.clone();
    zero_budget.budget.max_bytes = 0;

    let mut oversized = base.clone();
    oversized.budget.max_bytes = 1_048_577;

    let mut duplicate_fields = base.clone();
    duplicate_fields.requested_fields = vec!["continuity".to_owned(), "continuity".to_owned()];

    let mut authority_escalation = base.clone();
    authority_escalation.requested_fields = vec!["authority_override".to_owned()];

    let mut raw_private = base.clone();
    "RAW_PRIVATE".clone_into(&mut raw_private.privacy_profile);

    let mut ambiguous_identity = base.clone();
    ambiguous_identity.repository_identity_digest = Some(Sha256Digest::from([0_u8; 32]));

    vec![
        unsupported_protocol,
        zero_budget,
        oversized,
        duplicate_fields,
        authority_escalation,
        raw_private,
        ambiguous_identity,
    ]
}

/// Executes deterministic read conformance against observable provider state.
///
/// # Errors
///
/// Returns [`ConformanceError`] on an invalid suite, provider failure/timeout,
/// observable mutation, malformed/nondeterministic output or accepted hostile request.
pub fn run_read_conformance<P: ReadOnlyProvider>(
    provider: &P,
    suite: &ReadConformanceSuite,
) -> Result<ReadConformanceReport, ConformanceError> {
    if suite.projection_repetitions == 0 || suite.max_call_duration.is_zero() {
        return Err(ConformanceError::InvalidSuite {
            message: "repetitions and timeout must be non-zero",
        });
    }
    let baseline = provider
        .observable_state()
        .map_err(|failure| provider_error("observable.baseline", failure))?;

    let (manifest, elapsed) = timed(|| provider.manifest());
    ensure_elapsed("manifest", elapsed, suite.max_call_duration)?;
    let manifest = manifest.map_err(|failure| provider_error("manifest", failure))?;
    validate_manifest_integrity(&manifest).map_err(|error| invalid("manifest", error))?;
    ensure_unchanged(provider, baseline, "manifest")?;

    let (health, elapsed) = timed(|| provider.health());
    ensure_elapsed("health", elapsed, suite.max_call_duration)?;
    let health = health.map_err(|failure| provider_error("health", failure))?;
    validate_health_integrity(&health, &manifest).map_err(|error| invalid("health", error))?;
    ensure_unchanged(provider, baseline, "health")?;

    let (reference, mut projection_calls) =
        run_current_projections(provider, suite, &manifest, baseline)?;

    let stale = call_projection(
        provider,
        &suite.stale_request,
        &manifest,
        baseline,
        suite.max_call_duration,
        "projection.stale",
    )?;
    projection_calls = projection_calls.saturating_add(1);
    if stale.freshness != FreshnessStatus::Stale {
        return Err(ConformanceError::InvalidResponse {
            operation: "projection.stale",
            message: "mismatched expected version was not labeled STALE".to_owned(),
        });
    }

    let rejected_malicious = reject_malicious(provider, suite, baseline)?;

    let final_envelope = call_projection(
        provider,
        &suite.current_request,
        &manifest,
        baseline,
        suite.max_call_duration,
        "projection.after_failure",
    )?;
    projection_calls = projection_calls.saturating_add(1);
    let final_bytes = to_canonical_vec(&final_envelope)
        .map_err(|error| invalid("projection.after_failure", error))?;
    if reference != final_bytes {
        return Err(ConformanceError::NondeterministicProjection);
    }

    Ok(ReadConformanceReport {
        checks: [
            "NEGOTIATION",
            "MANIFEST_HEALTH_INTEGRITY",
            "OBSERVABLE_SIDE_EFFECTS_ZERO",
            "BOUNDS_AND_TIMEOUT",
            "IDENTITY_AND_FRESHNESS",
            "NO_AUTHORITY_ESCALATION",
            "REDACTION_AND_DETERMINISM",
            "FAILURE_ISOLATION",
        ]
        .into_iter()
        .map(|code| ConformanceCheck {
            code: code.to_owned(),
            passed: true,
        })
        .collect(),
        projection_calls,
        rejected_malicious,
        observable_mutations: 0,
    })
}

fn run_current_projections<P: ReadOnlyProvider>(
    provider: &P,
    suite: &ReadConformanceSuite,
    manifest: &IntegrationManifest,
    baseline: ObservableState,
) -> Result<(Vec<u8>, u32), ConformanceError> {
    let mut reference = None;
    for _ in 0..suite.projection_repetitions {
        let envelope = call_projection(
            provider,
            &suite.current_request,
            manifest,
            baseline,
            suite.max_call_duration,
            "projection.current",
        )?;
        if envelope.freshness != FreshnessStatus::Current {
            return Err(ConformanceError::InvalidResponse {
                operation: "projection.current",
                message: "current request was not labeled CURRENT".to_owned(),
            });
        }
        let bytes =
            to_canonical_vec(&envelope).map_err(|error| invalid("projection.current", error))?;
        if reference
            .as_ref()
            .is_some_and(|expected| expected != &bytes)
        {
            return Err(ConformanceError::NondeterministicProjection);
        }
        reference.get_or_insert(bytes);
    }
    Ok((
        reference.ok_or(ConformanceError::InvalidSuite {
            message: "projection repetitions must be non-zero",
        })?,
        suite.projection_repetitions,
    ))
}

fn reject_malicious<P: ReadOnlyProvider>(
    provider: &P,
    suite: &ReadConformanceSuite,
    baseline: ObservableState,
) -> Result<u32, ConformanceError> {
    let mut rejected = 0_u32;
    for (index, request) in suite.malicious_requests.iter().enumerate() {
        let (result, elapsed) = timed(|| provider.project(request));
        ensure_elapsed("projection.malicious", elapsed, suite.max_call_duration)?;
        if result.is_ok() {
            return Err(ConformanceError::MaliciousAccepted { index });
        }
        rejected = rejected.saturating_add(1);
        ensure_unchanged(provider, baseline, "projection.malicious")?;
    }
    Ok(rejected)
}

fn call_projection<P: ReadOnlyProvider>(
    provider: &P,
    request: &ProjectionRequest,
    manifest: &IntegrationManifest,
    baseline: ObservableState,
    timeout: Duration,
    operation: &'static str,
) -> Result<ProjectionEnvelope, ConformanceError> {
    let (envelope, elapsed) = timed(|| provider.project(request));
    ensure_elapsed(operation, elapsed, timeout)?;
    let envelope = envelope.map_err(|failure| provider_error(operation, failure))?;
    validate_projection_envelope(&envelope, request, manifest)
        .map_err(|error| invalid(operation, error))?;
    if envelope.state_version != baseline.state_version
        || envelope.state_digest != baseline.state_digest
    {
        return Err(ConformanceError::InvalidResponse {
            operation,
            message: "projection state is not bound to the observable provider state".to_owned(),
        });
    }
    ensure_unchanged(provider, baseline, operation)?;
    Ok(envelope)
}

fn ensure_unchanged<P: ReadOnlyProvider>(
    provider: &P,
    baseline: ObservableState,
    operation: &'static str,
) -> Result<(), ConformanceError> {
    let current = provider
        .observable_state()
        .map_err(|failure| provider_error(operation, failure))?;
    if current != baseline {
        return Err(ConformanceError::ObservableMutation { operation });
    }
    Ok(())
}

fn timed<T>(operation: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let result = operation();
    (result, started.elapsed())
}

fn ensure_elapsed(
    operation: &'static str,
    actual: Duration,
    limit: Duration,
) -> Result<(), ConformanceError> {
    if actual > limit {
        return Err(ConformanceError::Timeout {
            operation,
            limit_ms: limit.as_millis(),
            actual_ms: actual.as_millis(),
        });
    }
    Ok(())
}

fn provider_error(operation: &'static str, failure: ProviderFailure) -> ConformanceError {
    ConformanceError::Provider {
        operation,
        code: failure.code,
        message: failure.message,
    }
}

fn invalid(operation: &'static str, error: impl std::fmt::Display) -> ConformanceError {
    ConformanceError::InvalidResponse {
        operation,
        message: error.to_string(),
    }
}
