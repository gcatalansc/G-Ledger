use std::{fs, path::Path};

use jsonschema::Validator;
use project_ledger_canonical_json::{CanonicalJsonError, sha256_digest};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

pub const GSKILLS_PROFILE: &str = "gskills-context-provider/0.3.0";
pub const GSKILLS_SPEC_REVISION: &str = "PL-UPSTREAM-REFRESH-1";
pub const GSKILLS_SPEC_SHA256: &str =
    "957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e";
pub const GSKILLS_REGISTRY_SHA256: &str =
    "a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9";
pub const GSKILLS_FIXTURE_MANIFEST_SHA256: &str =
    "316c056480f3ab613f937426946da08276e9673ec6781d86f5a8b1e1e5e614ee";
pub const RECIPROCAL_LOCK_SHA256: &str =
    "49ff8bf60b33deb47cc688f288f7c2e1124d99fb223d0d8d4d2668ea4c60929d";
pub const LEDGER_SPEC_SHA256: &str =
    "31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VendoredReadContract {
    pub name: &'static str,
    pub schema_id: &'static str,
    pub schema_digest: &'static str,
    pub schema: &'static str,
    pub positive_fixture: &'static str,
    pub negative_fixture: &'static str,
}

macro_rules! contract {
    ($name:literal, $digest:literal) => {
        VendoredReadContract {
            name: $name,
            schema_id: concat!("urn:gskills:schema:", $name, ":0.3.0"),
            schema_digest: concat!("sha256:", $digest),
            schema: concat!($name, ".schema.json"),
            positive_fixture: concat!("fixtures/positive/", $name, ".json"),
            negative_fixture: concat!("fixtures/negative/", $name, ".additional-property.json"),
        }
    };
}

pub const VENDORED_READ_CONTRACTS: &[VendoredReadContract] = &[
    contract!(
        "provider-manifest",
        "981fc45ee4742f5140a1703aadc1f2dc9c164e59eb5f000924449abfdc03afd7"
    ),
    contract!(
        "provider-health-report",
        "8ff88bbdb9945a756c1eb99b1a59bba1d2b7a1505a2ca2aec0f2e1f164d719d5"
    ),
    contract!(
        "context-projection-request",
        "722f26ebce88d0757258869ea44e01809d9fd73b8dab55d95892424a0486af23"
    ),
    contract!(
        "context-projection-envelope",
        "b72000ccd0738d2ff945762b86371cf6302918134ad92527cab4926578e2b122"
    ),
    contract!(
        "provider-freshness-record",
        "8737ebf687c8a798c24c8c3e3d0021ecd3d2d1a5980e3ee9bd69fcce11ec8929"
    ),
    contract!(
        "project-continuity-projection",
        "99c2651ac831c41b3918639ad53117016e83e8baec06c80699901fde43fe6ee1"
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ContractVerificationReport {
    pub schema_count: usize,
    pub positive_fixture_count: usize,
    pub negative_fixture_count: usize,
}

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("cannot read vendored G-Skills contract {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("vendored G-Skills JSON is invalid at {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("vendored G-Skills contract {contract} differs from its upstream lock: {reason}")]
    LockMismatch {
        contract: &'static str,
        reason: &'static str,
    },
    #[error("cannot compile vendored G-Skills schema {contract}: {message}")]
    InvalidSchema {
        contract: &'static str,
        message: String,
    },
    #[error("vendored G-Skills fixture has the wrong outcome for {contract}: {fixture}")]
    FixtureOutcome {
        contract: &'static str,
        fixture: &'static str,
    },
    #[error("unknown vendored G-Skills read contract: {contract}")]
    UnknownContract { contract: String },
    #[error("G-Skills value does not satisfy {contract}: {message}")]
    InvalidValue {
        contract: &'static str,
        message: String,
    },
}

/// Validates a JSON value against one exact embedded S9 read contract.
///
/// # Errors
///
/// Returns [`ContractError`] when the contract name is unknown, the embedded
/// schema cannot compile, or the value violates that schema.
pub fn validate_read_contract(contract: &str, value: &Value) -> Result<(), ContractError> {
    let (name, source) =
        embedded_schema(contract).ok_or_else(|| ContractError::UnknownContract {
            contract: contract.to_owned(),
        })?;
    let schema: Value = serde_json::from_str(source).map_err(|source| ContractError::Json {
        path: format!("embedded:{name}"),
        source,
    })?;
    let validator = Validator::new(&schema).map_err(|error| ContractError::InvalidSchema {
        contract: name,
        message: error.to_string(),
    })?;
    if let Err(error) = validator.validate(value) {
        return Err(ContractError::InvalidValue {
            contract: name,
            message: error.to_string(),
        });
    }
    Ok(())
}

/// Verifies the exact schemas and paired fixtures vendored for the S9 read profile.
///
/// # Errors
///
/// Returns [`ContractError`] when a file is absent/malformed, a schema digest or
/// identifier differs, or a positive/negative fixture has the wrong outcome.
pub fn verify_vendored_contracts(
    repository_root: &Path,
) -> Result<ContractVerificationReport, ContractError> {
    let root = repository_root.join("integrations/gskills/contracts/v0.3");
    for contract in VENDORED_READ_CONTRACTS {
        let schema = read_json(&root.join(contract.schema))?;
        if schema.get("$id").and_then(Value::as_str) != Some(contract.schema_id) {
            return Err(ContractError::LockMismatch {
                contract: contract.name,
                reason: "schema identifier",
            });
        }
        if sha256_digest(&schema)?.to_string() != contract.schema_digest {
            return Err(ContractError::LockMismatch {
                contract: contract.name,
                reason: "canonical schema digest",
            });
        }
        let validator = Validator::new(&schema).map_err(|error| ContractError::InvalidSchema {
            contract: contract.name,
            message: error.to_string(),
        })?;
        let positive = read_json(&root.join(contract.positive_fixture))?;
        if !validator.is_valid(&positive) {
            return Err(ContractError::FixtureOutcome {
                contract: contract.name,
                fixture: "positive",
            });
        }
        let negative = read_json(&root.join(contract.negative_fixture))?;
        if validator.is_valid(&negative) {
            return Err(ContractError::FixtureOutcome {
                contract: contract.name,
                fixture: "negative",
            });
        }
    }
    Ok(ContractVerificationReport {
        schema_count: VENDORED_READ_CONTRACTS.len(),
        positive_fixture_count: VENDORED_READ_CONTRACTS.len(),
        negative_fixture_count: VENDORED_READ_CONTRACTS.len(),
    })
}

fn read_json(path: &Path) -> Result<Value, ContractError> {
    let bytes = fs::read(path).map_err(|source| ContractError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| ContractError::Json {
        path: path.display().to_string(),
        source,
    })
}

fn embedded_schema(contract: &str) -> Option<(&'static str, &'static str)> {
    match contract {
        "provider-manifest" => Some((
            "provider-manifest",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/provider-manifest.schema.json"
            ),
        )),
        "provider-health-report" => Some((
            "provider-health-report",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/provider-health-report.schema.json"
            ),
        )),
        "context-projection-request" => Some((
            "context-projection-request",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/context-projection-request.schema.json"
            ),
        )),
        "context-projection-envelope" => Some((
            "context-projection-envelope",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/context-projection-envelope.schema.json"
            ),
        )),
        "provider-freshness-record" => Some((
            "provider-freshness-record",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/provider-freshness-record.schema.json"
            ),
        )),
        "project-continuity-projection" => Some((
            "project-continuity-projection",
            include_str!(
                "../../../integrations/gskills/contracts/v0.3/project-continuity-projection.schema.json"
            ),
        )),
        _ => None,
    }
}
