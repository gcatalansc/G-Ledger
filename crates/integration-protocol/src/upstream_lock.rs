use project_ledger_canonical_json::Sha256Digest;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::ProtocolError;

/// Whether all exact evidence required by REQ-INT-001 is present.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpstreamLockStatus {
    Complete,
    Pending,
}

/// Conservative inspection result for heterogeneous upstream-lock files.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UpstreamLockInspection {
    pub integration: String,
    pub status: UpstreamLockStatus,
    pub missing_evidence: Vec<String>,
    pub certified_compatibility: bool,
}

/// Inspects a lock without reinterpreting absent evidence or certification.
///
/// # Errors
///
/// Returns [`ProtocolError`] when required metadata is malformed, a supplied
/// digest/range is invalid, or incomplete evidence claims certification.
pub fn inspect_upstream_lock(value: &Value) -> Result<UpstreamLockInspection, ProtocolError> {
    let object = value.as_object().ok_or_else(|| invalid("root"))?;
    if object.get("schema_version").and_then(Value::as_str) != Some("1.0.0") {
        return Err(invalid("schema_version"));
    }
    let integration = required_text(object, "integration")?.to_owned();
    let certified_compatibility = object
        .get("certified_compatibility")
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid("certified_compatibility"))?;

    let mut missing = Vec::new();
    if object
        .get("product_version")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        missing.push("product_version".to_owned());
    }
    inspect_digest(object, "source_spec_sha256", &mut missing)?;
    inspect_digest(object, "schema_registry_sha256", &mut missing)?;
    inspect_digest(object, "fixture_manifest_sha256", &mut missing)?;
    inspect_compatibility_range(object, &mut missing)?;
    if object
        .get("reciprocal_lock_status")
        .and_then(Value::as_str)
        .is_some_and(|status| status != "CURRENT")
    {
        missing.push("reciprocal_lock".to_owned());
    }
    missing.sort();
    missing.dedup();
    if certified_compatibility && !missing.is_empty() {
        return Err(ProtocolError::CompatibilityOverclaim);
    }
    Ok(UpstreamLockInspection {
        integration,
        status: if missing.is_empty() {
            UpstreamLockStatus::Complete
        } else {
            UpstreamLockStatus::Pending
        },
        missing_evidence: missing,
        certified_compatibility,
    })
}

fn inspect_digest(
    object: &Map<String, Value>,
    field: &str,
    missing: &mut Vec<String>,
) -> Result<(), ProtocolError> {
    let Some(value) = object.get(field).and_then(Value::as_str) else {
        missing.push(field.to_owned());
        return Ok(());
    };
    if value.is_empty() {
        missing.push(field.to_owned());
        return Ok(());
    }
    let canonical = if value.starts_with("sha256:") {
        value.to_owned()
    } else {
        format!("sha256:{value}")
    };
    Sha256Digest::parse(&canonical).map_err(|_| invalid(field))?;
    Ok(())
}

fn inspect_compatibility_range(
    object: &Map<String, Value>,
    missing: &mut Vec<String>,
) -> Result<(), ProtocolError> {
    let Some(range) = object.get("compatibility_range") else {
        missing.push("compatibility_range".to_owned());
        return Ok(());
    };
    let range = range
        .as_object()
        .ok_or_else(|| invalid("compatibility_range"))?;
    for field in ["protocol", "minimum", "maximum"] {
        if range
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(invalid(&format!("compatibility_range.{field}")));
        }
    }
    Ok(())
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, ProtocolError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(field))
}

fn invalid(field: &str) -> ProtocolError {
    ProtocolError::InvalidUpstreamLock {
        field: field.to_owned(),
    }
}
