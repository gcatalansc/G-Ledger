use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable machine-readable error codes used by S1 contracts.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidIdentifier,
    InvalidTimestamp,
    InvalidReference,
    InvalidDigest,
    InvalidContract,
    SchemaValidationFailed,
    ChecksumMismatch,
    CommandEventContractViolation,
}

/// Validation failures for storage-independent domain primitives.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid {kind} identifier")]
    InvalidIdentifier { kind: &'static str },
    #[error("timestamp must be valid RFC 3339 with an explicit offset")]
    InvalidTimestamp,
    #[error("reference must be an absolute URI without whitespace")]
    InvalidReference,
    #[error("invalid contract: {message}")]
    InvalidContract { message: &'static str },
}

impl DomainError {
    /// Returns the stable error code for the failure.
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidIdentifier { .. } => ErrorCode::InvalidIdentifier,
            Self::InvalidTimestamp => ErrorCode::InvalidTimestamp,
            Self::InvalidReference => ErrorCode::InvalidReference,
            Self::InvalidContract { .. } => ErrorCode::InvalidContract,
        }
    }
}
