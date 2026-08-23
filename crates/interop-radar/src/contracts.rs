use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use jsonschema::Validator;
use project_ledger_canonical_json::sha256_bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const RADAR_PROFILE: &str = "project-ledger-radar-artifact/0.1.1";
pub const RADAR_SOURCE_SPEC_SHA256: &str =
    "277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b";
pub const RADAR_SCHEMA_REGISTRY_SHA256: &str =
    "b80e2659dad658da72d86d9ff74186e9f3f86077ef114845b192a1391404dc99";
pub const RADAR_FIXTURE_MANIFEST_SHA256: &str =
    "d0aae58b0136b4fa80362b948022353508cc317eb5563dadf91374cf75ffa077";

const PROFILE_ROOT: &str = "integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1";
const EXPECTED_CONTRACTS: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RadarContractVerificationReport {
    pub schema_count: usize,
    pub valid_fixture_count: usize,
    pub invalid_fixture_count: usize,
    pub schema_registry_sha256: String,
    pub fixture_manifest_sha256: String,
}

#[derive(Debug, Error)]
pub enum RadarContractError {
    #[error("cannot read Project Radar profile artifact {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Project Radar profile JSON is invalid at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "Project Radar profile manifest {kind} digest mismatch: expected {expected}, actual {actual}"
    )]
    ManifestDigestMismatch {
        kind: &'static str,
        expected: String,
        actual: String,
    },
    #[error(
        "Project Radar artifact {path} raw digest mismatch: expected {expected}, actual {actual}"
    )]
    RawDigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("Project Radar profile shape differs at {field}")]
    ProfileMismatch { field: &'static str },
    #[error("Project Radar contract count mismatch: expected {expected}, actual {actual}")]
    CountMismatch { expected: usize, actual: usize },
    #[error("duplicate Project Radar contract {contract}")]
    DuplicateContract { contract: String },
    #[error("fixture entry is missing for Project Radar contract {contract}")]
    MissingFixture { contract: String },
    #[error("unknown Project Radar contract {contract}")]
    UnknownContract { contract: String },
    #[error("Project Radar schema identity differs for {contract}")]
    SchemaIdentityMismatch { contract: String },
    #[error("Project Radar schema {contract} cannot be compiled: {message}")]
    InvalidSchema { contract: String, message: String },
    #[error("value does not satisfy Project Radar contract {contract}: {message}")]
    InvalidValue { contract: String, message: String },
    #[error("Project Radar fixture {fixture} has the wrong outcome for {contract}")]
    FixtureOutcome {
        contract: String,
        fixture: &'static str,
    },
}

#[derive(Clone, Debug)]
pub struct RadarContractDescriptor {
    name: String,
    schema_id: String,
}

impl RadarContractDescriptor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }
}

#[derive(Clone, Debug)]
pub struct RadarContractCatalog {
    repository_root: PathBuf,
    contracts: Vec<RadarContractDescriptor>,
    schemas: BTreeMap<String, Value>,
    fixtures: BTreeMap<String, FixtureEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaRegistry {
    schema_version: String,
    profile: String,
    source_spec_sha256: String,
    entries: Vec<SchemaEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaEntry {
    name: String,
    schema_id: String,
    path: String,
    raw_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureManifest {
    schema_version: String,
    profile: String,
    entries: Vec<FixtureEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureEntry {
    contract: String,
    positive_path: String,
    positive_raw_sha256: String,
    negative_path: String,
    negative_raw_sha256: String,
}

impl RadarContractCatalog {
    /// Loads and verifies the exact local Project Radar artifact profile.
    ///
    /// # Errors
    ///
    /// Returns [`RadarContractError`] when either manifest, any raw artifact,
    /// schema identity, fixture association, or schema compilation differs.
    pub fn load(repository_root: &Path) -> Result<Self, RadarContractError> {
        let profile_root = repository_root.join(PROFILE_ROOT);
        let registry_path = profile_root.join("schema-registry.json");
        let registry_bytes = read(&registry_path)?;
        verify_manifest_digest(
            "schema registry",
            &registry_bytes,
            RADAR_SCHEMA_REGISTRY_SHA256,
        )?;
        let registry: SchemaRegistry = parse_json(&registry_path, &registry_bytes)?;
        verify_registry_header(&registry)?;
        require_count(registry.entries.len())?;

        let fixture_path = profile_root.join("fixture-manifest.json");
        let fixture_bytes = read(&fixture_path)?;
        verify_manifest_digest(
            "fixture manifest",
            &fixture_bytes,
            RADAR_FIXTURE_MANIFEST_SHA256,
        )?;
        let fixture_manifest: FixtureManifest = parse_json(&fixture_path, &fixture_bytes)?;
        if fixture_manifest.schema_version != "0.1.1" {
            return Err(RadarContractError::ProfileMismatch {
                field: "fixture schema_version",
            });
        }
        if fixture_manifest.profile != RADAR_PROFILE {
            return Err(RadarContractError::ProfileMismatch {
                field: "fixture profile",
            });
        }
        require_count(fixture_manifest.entries.len())?;

        let mut fixtures = BTreeMap::new();
        for fixture in fixture_manifest.entries {
            verify_raw(
                repository_root,
                &fixture.positive_path,
                &fixture.positive_raw_sha256,
            )?;
            verify_raw(
                repository_root,
                &fixture.negative_path,
                &fixture.negative_raw_sha256,
            )?;
            parse_json::<Value>(
                &repository_root.join(&fixture.positive_path),
                &read(&repository_root.join(&fixture.positive_path))?,
            )?;
            parse_json::<Value>(
                &repository_root.join(&fixture.negative_path),
                &read(&repository_root.join(&fixture.negative_path))?,
            )?;
            let contract = fixture.contract.clone();
            if fixtures.insert(contract.clone(), fixture).is_some() {
                return Err(RadarContractError::DuplicateContract { contract });
            }
        }

        let mut seen = BTreeSet::new();
        let mut contracts = Vec::with_capacity(EXPECTED_CONTRACTS);
        let mut schemas = BTreeMap::new();
        for entry in registry.entries {
            if !seen.insert(entry.name.clone()) {
                return Err(RadarContractError::DuplicateContract {
                    contract: entry.name,
                });
            }
            if !fixtures.contains_key(&entry.name) {
                return Err(RadarContractError::MissingFixture {
                    contract: entry.name,
                });
            }
            verify_raw(repository_root, &entry.path, &entry.raw_sha256)?;
            let schema_path = repository_root.join(&entry.path);
            let schema: Value = parse_json(&schema_path, &read(&schema_path)?)?;
            if schema.get("$id").and_then(Value::as_str) != Some(entry.schema_id.as_str()) {
                return Err(RadarContractError::SchemaIdentityMismatch {
                    contract: entry.name,
                });
            }
            Validator::new(&schema).map_err(|error| RadarContractError::InvalidSchema {
                contract: entry.name.clone(),
                message: error.to_string(),
            })?;
            contracts.push(RadarContractDescriptor {
                name: entry.name.clone(),
                schema_id: entry.schema_id,
            });
            schemas.insert(entry.name, schema);
        }
        contracts.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(Self {
            repository_root: repository_root.to_path_buf(),
            contracts,
            schemas,
            fixtures,
        })
    }

    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }

    pub fn contracts(&self) -> &[RadarContractDescriptor] {
        &self.contracts
    }

    /// Validates a JSON value against a named profile contract.
    ///
    /// # Errors
    ///
    /// Returns [`RadarContractError::UnknownContract`] or the first schema error.
    pub fn validate(&self, contract: &str, value: &Value) -> Result<(), RadarContractError> {
        let schema =
            self.schemas
                .get(contract)
                .ok_or_else(|| RadarContractError::UnknownContract {
                    contract: contract.to_owned(),
                })?;
        let validator =
            Validator::new(schema).map_err(|error| RadarContractError::InvalidSchema {
                contract: contract.to_owned(),
                message: error.to_string(),
            })?;
        validator
            .validate(value)
            .map_err(|error| RadarContractError::InvalidValue {
                contract: contract.to_owned(),
                message: error.to_string(),
            })
    }

    /// Reads the raw-digest-verified positive fixture for a contract.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract is unknown or the JSON cannot be read.
    pub fn read_positive_fixture(&self, contract: &str) -> Result<Value, RadarContractError> {
        let entry = self.fixture(contract)?;
        read_json(&self.repository_root.join(&entry.positive_path))
    }

    /// Reads the raw-digest-verified negative fixture for a contract.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract is unknown or the JSON cannot be read.
    pub fn read_negative_fixture(&self, contract: &str) -> Result<Value, RadarContractError> {
        let entry = self.fixture(contract)?;
        read_json(&self.repository_root.join(&entry.negative_path))
    }

    fn fixture(&self, contract: &str) -> Result<&FixtureEntry, RadarContractError> {
        self.fixtures
            .get(contract)
            .ok_or_else(|| RadarContractError::UnknownContract {
                contract: contract.to_owned(),
            })
    }
}

/// Verifies all locked schemas and paired fixtures offline.
///
/// # Errors
///
/// Returns [`RadarContractError`] on any raw drift, malformed schema, or wrong fixture outcome.
pub fn verify_profile_contracts(
    repository_root: &Path,
) -> Result<RadarContractVerificationReport, RadarContractError> {
    let catalog = RadarContractCatalog::load(repository_root)?;
    for contract in catalog.contracts() {
        let positive = catalog.read_positive_fixture(contract.name())?;
        catalog.validate(contract.name(), &positive).map_err(|_| {
            RadarContractError::FixtureOutcome {
                contract: contract.name().to_owned(),
                fixture: "positive",
            }
        })?;
        let negative = catalog.read_negative_fixture(contract.name())?;
        if catalog.validate(contract.name(), &negative).is_ok() {
            return Err(RadarContractError::FixtureOutcome {
                contract: contract.name().to_owned(),
                fixture: "negative",
            });
        }
    }
    Ok(RadarContractVerificationReport {
        schema_count: catalog.contract_count(),
        valid_fixture_count: catalog.contract_count(),
        invalid_fixture_count: catalog.contract_count(),
        schema_registry_sha256: RADAR_SCHEMA_REGISTRY_SHA256.to_owned(),
        fixture_manifest_sha256: RADAR_FIXTURE_MANIFEST_SHA256.to_owned(),
    })
}

fn verify_registry_header(registry: &SchemaRegistry) -> Result<(), RadarContractError> {
    if registry.schema_version != "0.1.1" {
        return Err(RadarContractError::ProfileMismatch {
            field: "schema_version",
        });
    }
    if registry.profile != RADAR_PROFILE {
        return Err(RadarContractError::ProfileMismatch { field: "profile" });
    }
    if registry.source_spec_sha256 != RADAR_SOURCE_SPEC_SHA256 {
        return Err(RadarContractError::ProfileMismatch {
            field: "source_spec_sha256",
        });
    }
    Ok(())
}

fn require_count(actual: usize) -> Result<(), RadarContractError> {
    if actual != EXPECTED_CONTRACTS {
        return Err(RadarContractError::CountMismatch {
            expected: EXPECTED_CONTRACTS,
            actual,
        });
    }
    Ok(())
}

fn verify_manifest_digest(
    kind: &'static str,
    bytes: &[u8],
    expected: &str,
) -> Result<(), RadarContractError> {
    let actual = raw_hex(bytes);
    if actual != expected {
        return Err(RadarContractError::ManifestDigestMismatch {
            kind,
            expected: expected.to_owned(),
            actual,
        });
    }
    Ok(())
}

fn verify_raw(root: &Path, relative: &str, expected: &str) -> Result<(), RadarContractError> {
    let bytes = read(&root.join(relative))?;
    let actual = sha256_bytes(&bytes).to_string();
    if actual != expected {
        return Err(RadarContractError::RawDigestMismatch {
            path: relative.to_owned(),
            expected: expected.to_owned(),
            actual,
        });
    }
    Ok(())
}

fn raw_hex(bytes: &[u8]) -> String {
    sha256_bytes(bytes)
        .to_string()
        .trim_start_matches("sha256:")
        .to_owned()
}

fn read(path: &Path) -> Result<Vec<u8>, RadarContractError> {
    fs::read(path).map_err(|source| RadarContractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_json(path: &Path) -> Result<Value, RadarContractError> {
    parse_json(path, &read(path)?)
}

fn parse_json<T>(path: &Path, bytes: &[u8]) -> Result<T, RadarContractError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_slice(bytes).map_err(|source| RadarContractError::Json {
        path: path.to_path_buf(),
        source,
    })
}
