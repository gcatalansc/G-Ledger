use std::path::{Component, Path};

use serde_json::Value;
use thiserror::Error;

/// Supported privacy violation classes. Finding values are deliberately omitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivacyFindingKind {
    Secret,
    Transcript,
    PrivateReasoning,
    RawTerminalOutput,
}

/// One location-only scanner finding that is safe to render in logs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivacyFinding {
    pub surface: String,
    pub location: String,
    pub kind: PrivacyFindingKind,
}

/// Invalid declared surface paths.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SurfacePathError {
    #[error("surface path must be a non-empty normalized relative path")]
    Unsafe,
}

/// Validates a manifest-controlled path without resolving or following it.
///
/// # Errors
///
/// Returns an error for empty, absolute, current-directory or parent-traversal paths.
pub fn validate_surface_relative_path(path: &Path) -> Result<(), SurfacePathError> {
    let mut normal_components = 0_u32;
    for component in path.components() {
        match component {
            Component::Normal(_) => normal_components += 1,
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(SurfacePathError::Unsafe);
            }
        }
    }
    if normal_components == 0 {
        return Err(SurfacePathError::Unsafe);
    }
    Ok(())
}

/// Conservative scanner for the release-declared surfaces and supported secret patterns.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrivacyScanner;

impl PrivacyScanner {
    /// Scans structured JSON without returning sensitive values.
    #[must_use]
    pub fn scan_json(&self, surface: &str, value: &Value) -> Vec<PrivacyFinding> {
        let mut findings = Vec::new();
        scan_value(surface, "$", value, &mut findings);
        findings
    }

    /// Scans raw bytes for the explicitly supported high-signal patterns.
    #[must_use]
    pub fn scan_bytes(&self, surface: &str, bytes: &[u8]) -> Vec<PrivacyFinding> {
        let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
        let patterns = [
            ("authorization: bearer ", "authorization"),
            ("-----begin private key-----", "private-key"),
            ("-----begin rsa private key-----", "private-key"),
            ("password=", "password"),
            ("token=", "token"),
            ("api_key=", "api-key"),
        ];
        let mut findings = patterns
            .into_iter()
            .filter(|(pattern, _)| text.contains(pattern))
            .map(|(_, location)| PrivacyFinding {
                surface: surface.to_owned(),
                location: location.to_owned(),
                kind: PrivacyFindingKind::Secret,
            })
            .collect::<Vec<_>>();
        if contains_uri_credentials(&text) {
            findings.push(PrivacyFinding {
                surface: surface.to_owned(),
                location: "uri-userinfo".to_owned(),
                kind: PrivacyFindingKind::Secret,
            });
        }
        findings
    }
}

fn scan_value(surface: &str, path: &str, value: &Value, findings: &mut Vec<PrivacyFinding>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if is_present_sensitive_value(child) {
                    let normalized = key.to_ascii_lowercase();
                    let kind = match normalized.as_str() {
                        "transcript" | "transcripts" => Some(PrivacyFindingKind::Transcript),
                        "reasoning" | "reasoning_private" | "private_reasoning" => {
                            Some(PrivacyFindingKind::PrivateReasoning)
                        }
                        "raw_terminal_output" | "raw_output" => {
                            Some(PrivacyFindingKind::RawTerminalOutput)
                        }
                        "secret" | "password" | "token" | "api_key" | "api-key" => {
                            Some(PrivacyFindingKind::Secret)
                        }
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        findings.push(PrivacyFinding {
                            surface: surface.to_owned(),
                            location: child_path.clone(),
                            kind,
                        });
                    }
                }
                scan_value(surface, &child_path, child, findings);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_value(surface, &format!("{path}[{index}]"), child, findings);
            }
        }
        Value::String(text) if !is_redacted(text) => {
            findings.extend(PrivacyScanner.scan_bytes(surface, text.as_bytes()));
        }
        _ => {}
    }
}

fn is_present_sensitive_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::String(value) => !value.is_empty() && !is_redacted(value),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
        Value::Number(_) => true,
    }
}

fn is_redacted(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_uppercase().as_str(),
        "[REDACTED]" | "REDACTED"
    )
}

fn contains_uri_credentials(text: &str) -> bool {
    let Some(scheme_end) = text.find("://") else {
        return false;
    };
    let authority = &text[(scheme_end + 3)..];
    let authority = authority.split(['/', '?', '#']).next().unwrap_or_default();
    let Some(userinfo) = authority.rsplit_once('@').map(|(userinfo, _)| userinfo) else {
        return false;
    };
    userinfo.contains(':') && !userinfo.contains("[redacted]")
}
