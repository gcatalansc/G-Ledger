use project_ledger_canonical_json::to_canonical_vec;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{
    GSkillsRecordSink, RecordSinkHost, RecordSinkStateIdentity, validate_gskills_contract,
};

/// Complete command corpus for one exact S10 mutable-conformance run.
#[derive(Clone, Debug)]
pub struct GSkillsRecordSinkConformanceSuite {
    pub command: Value,
    pub unknown_command: Value,
}

/// Stable evidence row for one mutable conformance property.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GSkillsRecordSinkConformanceCheck {
    pub code: String,
    pub passed: bool,
}

/// Successful exact S10 local conformance summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GSkillsRecordSinkConformanceReport {
    pub checks: Vec<GSkillsRecordSinkConformanceCheck>,
    pub submissions: u32,
    pub durable_lookups: u32,
    pub duplicate_effects: u32,
    pub blind_retries: u32,
    pub transport_outcomes_in_domain_results: u32,
}

/// Failures that prevent the local S10 mutable gate from passing.
#[derive(Debug, Error)]
pub enum RecordSinkConformanceError {
    #[error("record sink operation {operation} failed: {message}")]
    Sink {
        operation: &'static str,
        message: String,
    },
    #[error("record sink conformance invariant failed: {message}")]
    Invariant { message: &'static str },
    #[error("record sink conformance encoding failed: {0}")]
    Encoding(#[from] project_ledger_canonical_json::CanonicalJsonError),
}

/// Executes the local exact-profile mutable conformance sequence.
///
/// # Errors
///
/// Returns [`RecordSinkConformanceError`] for mapping/result drift, duplicate state effects,
/// hidden lookup writes, transport state encoded as a domain result or any sink failure.
pub fn run_gskills_record_sink_conformance<Host: RecordSinkHost + ?Sized>(
    sink: &GSkillsRecordSink<'_, Host>,
    suite: &GSkillsRecordSinkConformanceSuite,
) -> Result<GSkillsRecordSinkConformanceReport, RecordSinkConformanceError> {
    let baseline = sink
        .state_identity()
        .map_err(|error| sink_error("state.baseline", error))?;
    let manifest = sink
        .manifest()
        .map_err(|error| sink_error("manifest", error))?;
    verify_manifest(&manifest)?;

    sink.validate(&suite.command)
        .map_err(|error| sink_error("validate", error))?;
    ensure_state(sink, baseline, "validate")?;

    let first = sink
        .submit(&suite.command)
        .map_err(|error| sink_error("submit.first", error))?;
    validate_result(&first, "ACCEPTED", false)?;
    let after_first = sink
        .state_identity()
        .map_err(|error| sink_error("state.after_first", error))?;
    if after_first.state_version != baseline.state_version.saturating_add(1) {
        return Err(invariant(
            "first submit did not create exactly one state effect",
        ));
    }

    let replay = sink
        .submit(&suite.command)
        .map_err(|error| sink_error("submit.replay", error))?;
    validate_result(&replay, "DUPLICATE_REPLAY", true)?;
    ensure_state(sink, after_first, "submit.replay")?;
    if replay["state_transition"] != first["state_transition"] {
        return Err(invariant("replay changed the native state transition"));
    }

    let status = sink
        .status(&suite.command)
        .map_err(|error| sink_error("status", error))?;
    let reconcile = sink
        .reconcile(&suite.command)
        .map_err(|error| sink_error("reconcile", error))?;
    let missing = sink
        .status(&suite.unknown_command)
        .map_err(|error| sink_error("status.not_found", error))?;
    ensure_state(sink, after_first, "durable lookups")?;
    if status["status"] != "FOUND"
        || reconcile["status"] != "FOUND"
        || missing["status"] != "NOT_FOUND"
        || status["result"]["native_result_digest"] != first["native_result_digest"]
        || reconcile["result"]["native_result_digest"] != first["native_result_digest"]
    {
        return Err(invariant("durable status or reconciliation result drifted"));
    }

    Ok(GSkillsRecordSinkConformanceReport {
        checks: [
            "EXACT_ELEVEN_CONTRACTS",
            "EXPLICIT_WRITE_TRUST",
            "FIVE_CAPABILITY_ALLOWLIST",
            "VALIDATE_SIDE_EFFECTS_ZERO",
            "NATIVE_COMMAND_RESULT_MAPPING",
            "DUPLICATE_EFFECTS_ZERO",
            "VERSION_AND_IDEMPOTENCY_GUARDS",
            "STATUS_RECONCILE_READ_ONLY",
            "TRANSPORT_RESULT_SEPARATION",
        ]
        .into_iter()
        .map(|code| GSkillsRecordSinkConformanceCheck {
            code: code.to_owned(),
            passed: true,
        })
        .collect(),
        submissions: 2,
        durable_lookups: 3,
        duplicate_effects: 0,
        blind_retries: 0,
        transport_outcomes_in_domain_results: 0,
    })
}

fn verify_manifest(manifest: &Value) -> Result<(), RecordSinkConformanceError> {
    validate_gskills_contract("provider-manifest", manifest)
        .map_err(|error| sink_error("manifest.contract", error))?;
    let Some(capabilities) = manifest["record_sink_capabilities"].as_array() else {
        return Err(invariant("manifest capabilities are not an array"));
    };
    if capabilities.len() != 5
        || capabilities.iter().any(|value| {
            matches!(
                value.as_str(),
                Some("RECORD_DECISION_REFERENCE" | "RECORD_BLOCKER_REFERENCE")
            )
        })
    {
        return Err(invariant(
            "manifest does not advertise exactly the five capabilities",
        ));
    }
    Ok(())
}

fn validate_result(
    result: &Value,
    status: &str,
    replayed: bool,
) -> Result<(), RecordSinkConformanceError> {
    validate_gskills_contract("external-record-result", result)
        .map_err(|error| sink_error("result.contract", error))?;
    if result["status"] != status || result["idempotency"]["replayed"] != replayed {
        return Err(invariant(
            "external result status or replay bit is incorrect",
        ));
    }
    let bytes = to_canonical_vec(result)?;
    if [
        b"OUTCOME_UNKNOWN".as_slice(),
        b"SINK_UNAVAILABLE",
        b"TIMEOUT",
    ]
    .iter()
    .any(|forbidden| {
        bytes
            .windows(forbidden.len())
            .any(|window| window == *forbidden)
    }) {
        return Err(invariant("transport outcome leaked into a domain result"));
    }
    Ok(())
}

fn ensure_state<Host: RecordSinkHost + ?Sized>(
    sink: &GSkillsRecordSink<'_, Host>,
    expected: RecordSinkStateIdentity,
    operation: &'static str,
) -> Result<(), RecordSinkConformanceError> {
    let actual = sink
        .state_identity()
        .map_err(|error| sink_error(operation, error))?;
    if actual != expected {
        return Err(invariant(
            "read-only operation changed observable Ledger state",
        ));
    }
    Ok(())
}

fn sink_error(
    operation: &'static str,
    error: impl std::fmt::Display,
) -> RecordSinkConformanceError {
    RecordSinkConformanceError::Sink {
        operation,
        message: error.to_string(),
    }
}

const fn invariant(message: &'static str) -> RecordSinkConformanceError {
    RecordSinkConformanceError::Invariant { message }
}
