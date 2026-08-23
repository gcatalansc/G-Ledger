use std::time::{Duration, Instant};

use project_ledger_canonical_json::{sha256_digest, to_canonical_vec};
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

use crate::{
    GSkillsObservableState, GSkillsProjectionResult, GSkillsProvider, ProviderError,
    validate_read_contract,
};

/// Exact-profile provider surface exercised by the S9 conformance harness.
pub trait GSkillsReadProvider {
    fn observable_state(&self) -> GSkillsObservableState;
    /// Returns the exact provider manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] when the manifest cannot be produced.
    fn manifest(&self) -> Result<Value, ProviderError>;
    /// Returns the exact provider health record.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] when health cannot be produced.
    fn health(&self) -> Result<Value, ProviderError>;
    /// Returns the exact three-record projection result.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError`] when request negotiation or mapping fails.
    fn project(&self, request: &Value) -> Result<GSkillsProjectionResult, ProviderError>;
}

impl GSkillsReadProvider for GSkillsProvider {
    fn observable_state(&self) -> GSkillsObservableState {
        GSkillsProvider::observable_state(self)
    }

    fn manifest(&self) -> Result<Value, ProviderError> {
        GSkillsProvider::manifest(self)
    }

    fn health(&self) -> Result<Value, ProviderError> {
        GSkillsProvider::health(self)
    }

    fn project(&self, request: &Value) -> Result<GSkillsProjectionResult, ProviderError> {
        GSkillsProvider::project(self, request)
    }
}

/// Complete corpus for one exact G-Skills S9 read-conformance run.
#[derive(Clone, Debug)]
pub struct GSkillsReadConformanceSuite {
    pub current_request: Value,
    pub stale_request: Value,
    pub malicious_requests: Vec<Value>,
    pub projection_repetitions: u32,
    pub max_call_duration: Duration,
}

/// Stable evidence row for one exact-profile conformance property.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GSkillsConformanceCheck {
    pub code: String,
    pub passed: bool,
}

/// Successful exact-profile conformance summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GSkillsReadConformanceReport {
    pub checks: Vec<GSkillsConformanceCheck>,
    pub projection_calls: u32,
    pub rejected_malicious: u32,
    pub observable_mutations: u32,
    pub material_losses: u32,
}

/// Failures that prevent the exact S9 provider gate from passing.
#[derive(Debug, Error)]
pub enum ConformanceError {
    #[error("invalid exact conformance suite: {message}")]
    InvalidSuite { message: &'static str },
    #[error("exact provider call {operation} failed: {message}")]
    Provider {
        operation: &'static str,
        message: String,
    },
    #[error("exact provider call {operation} exceeded {limit_ms}ms ({actual_ms}ms)")]
    Timeout {
        operation: &'static str,
        limit_ms: u128,
        actual_ms: u128,
    },
    #[error("exact provider call {operation} mutated observable Ledger state")]
    ObservableMutation { operation: &'static str },
    #[error("exact provider response {operation} is invalid: {message}")]
    InvalidResponse {
        operation: &'static str,
        message: String,
    },
    #[error("exact projection changed for an identical request and state")]
    NondeterministicProjection,
    #[error("malicious exact request {index} was accepted")]
    MaliciousAccepted { index: usize },
}

/// Creates the exact-profile hostile request corpus from a valid full request.
pub fn malicious_gskills_requests(base: &Value) -> Vec<Value> {
    let mut wrong_provider = base.clone();
    wrong_provider["provider_id"] = json!("attacker-provider");
    resign(&mut wrong_provider);

    let mut zero_budget = base.clone();
    zero_budget["budget"]["max_items"] = json!(0);
    resign(&mut zero_budget);

    let mut oversized = base.clone();
    oversized["budget"]["max_output_bytes"] = json!(1_048_577);
    resign(&mut oversized);

    let mut duplicate_fields = base.clone();
    duplicate_fields["requested_fields"][1] = json!("project_state_version");
    resign(&mut duplicate_fields);

    let mut write_escalation = base.clone();
    write_escalation["write_capability"] = json!("RECORD_SUBMIT");
    resign(&mut write_escalation);

    let mut raw_events = base.clone();
    raw_events["privacy"]["raw_events"] = json!(true);
    resign(&mut raw_events);

    let mut wrong_identity = base.clone();
    wrong_identity["identity"]["project_ref"] = json!("project:other");
    resign(&mut wrong_identity);

    let mut bad_digest = base.clone();
    bad_digest["purpose"] = json!("tampered without digest refresh");

    let mut unknown_outer = base.clone();
    unknown_outer["unexpected_mutable_operation"] = json!(true);
    resign(&mut unknown_outer);

    vec![
        wrong_provider,
        zero_budget,
        oversized,
        duplicate_fields,
        write_escalation,
        raw_events,
        wrong_identity,
        bad_digest,
        unknown_outer,
    ]
}

/// Executes deterministic, bounded and side-effect-free exact S9 conformance.
///
/// # Errors
///
/// Returns [`ConformanceError`] for malformed/non-deterministic output, accepted
/// hostile input, mutation, timeout or a provider failure on the normative path.
pub fn run_gskills_read_conformance<P: GSkillsReadProvider>(
    provider: &P,
    suite: &GSkillsReadConformanceSuite,
) -> Result<GSkillsReadConformanceReport, ConformanceError> {
    if suite.projection_repetitions == 0 || suite.max_call_duration.is_zero() {
        return Err(ConformanceError::InvalidSuite {
            message: "repetitions and timeout must be non-zero",
        });
    }
    let baseline = provider.observable_state();
    let manifest = timed_provider("manifest", suite.max_call_duration, || provider.manifest())?;
    validate_read_contract("provider-manifest", &manifest)
        .map_err(|error| invalid("manifest", error))?;
    ensure_unchanged(provider, baseline, "manifest")?;

    let health = timed_provider("health", suite.max_call_duration, || provider.health())?;
    validate_read_contract("provider-health-report", &health)
        .map_err(|error| invalid("health", error))?;
    if health["provider_manifest_digest"] != manifest["digest"] {
        return Err(invalid("health", "health is not bound to the manifest"));
    }
    ensure_unchanged(provider, baseline, "health")?;

    let (reference, mut projection_calls) = run_current(provider, suite, baseline, &manifest)?;
    let stale = call_projection(
        provider,
        &suite.stale_request,
        baseline,
        suite.max_call_duration,
        "projection.stale",
    )?;
    projection_calls = projection_calls.saturating_add(1);
    if stale.freshness["status"] != "SUPERSEDED_BY_NEWER_CHECKPOINT" {
        return Err(invalid(
            "projection.stale",
            "state mismatch was not labeled as superseded",
        ));
    }

    let rejected_malicious = reject_malicious(provider, suite, baseline)?;
    let recovered = call_projection(
        provider,
        &suite.current_request,
        baseline,
        suite.max_call_duration,
        "projection.after_failure",
    )?;
    projection_calls = projection_calls.saturating_add(1);
    if canonical_result(&recovered)? != reference {
        return Err(ConformanceError::NondeterministicProjection);
    }

    Ok(GSkillsReadConformanceReport {
        checks: [
            "EXACT_CONTRACTS",
            "MANIFEST_HEALTH_BINDING",
            "OBSERVABLE_SIDE_EFFECTS_ZERO",
            "BOUNDS_AND_TIMEOUT",
            "IDENTITY_AND_FRESHNESS",
            "NO_AUTHORITY_ESCALATION",
            "ZERO_MATERIAL_LOSS",
            "DETERMINISM_AND_FAILURE_ISOLATION",
        ]
        .into_iter()
        .map(|code| GSkillsConformanceCheck {
            code: code.to_owned(),
            passed: true,
        })
        .collect(),
        projection_calls,
        rejected_malicious,
        observable_mutations: 0,
        material_losses: 0,
    })
}

fn run_current<P: GSkillsReadProvider>(
    provider: &P,
    suite: &GSkillsReadConformanceSuite,
    baseline: GSkillsObservableState,
    manifest: &Value,
) -> Result<(Vec<u8>, u32), ConformanceError> {
    let mut reference = None;
    for _ in 0..suite.projection_repetitions {
        let result = call_projection(
            provider,
            &suite.current_request,
            baseline,
            suite.max_call_duration,
            "projection.current",
        )?;
        if result.freshness["status"] != "CURRENT_FOR_PROJECT_ONLY"
            || result.envelope["provider"]["native_manifest_digest"] != manifest["digest"]
        {
            return Err(invalid(
                "projection.current",
                "freshness or manifest binding differs",
            ));
        }
        let bytes = canonical_result(&result)?;
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

fn call_projection<P: GSkillsReadProvider>(
    provider: &P,
    request: &Value,
    baseline: GSkillsObservableState,
    timeout: Duration,
    operation: &'static str,
) -> Result<GSkillsProjectionResult, ConformanceError> {
    let result = timed_provider(operation, timeout, || provider.project(request))?;
    validate_result(&result, request, baseline, operation)?;
    ensure_unchanged(provider, baseline, operation)?;
    Ok(result)
}

fn validate_result(
    result: &GSkillsProjectionResult,
    request: &Value,
    baseline: GSkillsObservableState,
    operation: &'static str,
) -> Result<(), ConformanceError> {
    for (contract, value) in [
        ("project-continuity-projection", &result.payload),
        ("provider-freshness-record", &result.freshness),
        ("context-projection-envelope", &result.envelope),
    ] {
        validate_read_contract(contract, value).map_err(|error| invalid(operation, error))?;
    }
    let bound = result.payload["project"]["state_version"] == baseline.state_version
        && result.payload["project"]["material_state_version"] == baseline.material_state_version
        && result.payload["project"]["state_digest"] == baseline.state_digest.to_string()
        && result.envelope["request_digest"] == request["digest"]
        && result.envelope["payload"]["digest"] == result.payload["digest"]
        && result.envelope["freshness_record_ref"]
            == result.freshness["provider_freshness_record_id"]
        && result.envelope["effective_authority_domains"] == json!([])
        && result.envelope["coverage"]["omitted_fields"] == json!([])
        && result.envelope["loss_report_ref"].is_null();
    if !bound {
        return Err(invalid(
            operation,
            "projection/envelope/freshness bindings or zero-loss authority boundary differ",
        ));
    }
    Ok(())
}

fn reject_malicious<P: GSkillsReadProvider>(
    provider: &P,
    suite: &GSkillsReadConformanceSuite,
    baseline: GSkillsObservableState,
) -> Result<u32, ConformanceError> {
    let mut rejected = 0_u32;
    for (index, request) in suite.malicious_requests.iter().enumerate() {
        let started = Instant::now();
        if provider.project(request).is_ok() {
            return Err(ConformanceError::MaliciousAccepted { index });
        }
        ensure_elapsed(
            "projection.malicious",
            started.elapsed(),
            suite.max_call_duration,
        )?;
        ensure_unchanged(provider, baseline, "projection.malicious")?;
        rejected = rejected.saturating_add(1);
    }
    Ok(rejected)
}

fn ensure_unchanged<P: GSkillsReadProvider>(
    provider: &P,
    baseline: GSkillsObservableState,
    operation: &'static str,
) -> Result<(), ConformanceError> {
    if provider.observable_state() != baseline {
        return Err(ConformanceError::ObservableMutation { operation });
    }
    Ok(())
}

fn timed_provider<T>(
    operation: &'static str,
    limit: Duration,
    call: impl FnOnce() -> Result<T, ProviderError>,
) -> Result<T, ConformanceError> {
    let started = Instant::now();
    let result = call().map_err(|error| ConformanceError::Provider {
        operation,
        message: error.to_string(),
    })?;
    ensure_elapsed(operation, started.elapsed(), limit)?;
    Ok(result)
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

fn canonical_result(result: &GSkillsProjectionResult) -> Result<Vec<u8>, ConformanceError> {
    to_canonical_vec(result).map_err(|error| invalid("projection.canonical", error))
}

fn resign(value: &mut Value) {
    let mut unsigned = value.clone();
    unsigned
        .as_object_mut()
        .expect("test corpus base is an exact request object")
        .remove("digest");
    value["digest"] = json!(
        sha256_digest(&unsigned)
            .expect("exact request corpus is canonicalizable")
            .to_string()
    );
}

fn invalid(operation: &'static str, message: impl std::fmt::Display) -> ConformanceError {
    ConformanceError::InvalidResponse {
        operation,
        message: message.to_string(),
    }
}
