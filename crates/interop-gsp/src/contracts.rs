use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use jsonschema::Registry;
use project_ledger_canonical_json::sha256_bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const GSP_IMPLEMENTATION_VERSION: &str = "0.2.0";
pub const GSP_IMPLEMENTATION_COMMIT: &str = "7a43559c02417641691dbb5f2a6e79c13d74d1a1";
pub const GSP_IMPLEMENTATION_SPEC_SHA256: &str =
    "c838bdd8eee3fb6648db5e7dd757d08d8ced81d35be8b9bf8001e321f49ffea1";
pub const GSP_CONTRACT_TAG: &str = "gsp-exact-2026.04.12";
pub const GSP_CONTRACT_COMMIT: &str = "004b14d7b64837025671970eac2469f092c49475";
pub const GSP_ARCHIVE_SHA256: &str =
    "59395d1ecd0c573451f2fcdaa6cf04ce70fb1f4fe129ba1a3d2ec2b754ab074b";
pub const GSP_FIXTURE_MANIFEST_SHA256: &str =
    "c6ca0f6df33bda69e7e73eb201058884a3111bce506c9328aca5c421836e2e8c";
pub const GSP_SCHEMA_REGISTRY_SHA256: &str =
    "523faabcc2e120a847849b582c9dbb97d758a6c34f78b96e153b7b67a622d7b3";

const EXPECTED_REGISTRY_CONTRACTS: usize = 28;
const EXPECTED_SCHEMA_ARTIFACTS: usize = 31;
const EXPECTED_VALID_FIXTURES: usize = 56;
const EXPECTED_INVALID_FIXTURES: usize = 84;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GspContractVerificationReport {
    pub registry_contract_count: usize,
    pub schema_artifact_count: usize,
    pub valid_fixture_count: usize,
    pub invalid_fixture_count: usize,
}

#[derive(Debug, Error)]
pub enum GspContractError {
    #[error("cannot read GSP contract artifact {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("GSP JSON artifact is invalid at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("GSP YAML fixture is invalid at {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("GSP HASH_MANIFEST.csv digest mismatch: expected {expected}, actual {actual}")]
    ManifestDigestMismatch { expected: String, actual: String },
    #[error("GSP schema registry digest mismatch: expected {expected}, actual {actual}")]
    RegistryDigestMismatch { expected: String, actual: String },
    #[error("malformed GSP hash manifest row: {row}")]
    ManifestMalformed { row: String },
    #[error("GSP artifact {path} differs from upstream: expected {expected}, actual {actual}")]
    RawDigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("unexpected or unmanifested GSP artifact: {path}")]
    UnexpectedArtifact { path: String },
    #[error("GSP exact surface count mismatch for {kind}: expected {expected}, actual {actual}")]
    CountMismatch {
        kind: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("GSP schema registry is invalid: {0}")]
    InvalidRegistry(String),
    #[error("unknown GSP contract: {contract}")]
    UnknownContract { contract: String },
    #[error("GSP schema {contract} cannot be compiled: {message}")]
    InvalidSchema { contract: String, message: String },
    #[error("value does not satisfy exact GSP contract {contract}: {message}")]
    InvalidValue { contract: String, message: String },
    #[error("GSP fixture {fixture} has the wrong validation outcome for {contract}")]
    FixtureOutcome { fixture: String, contract: String },
}

#[derive(Clone, Debug)]
pub struct GspContractCatalog {
    schemas_by_name: BTreeMap<String, Value>,
    schemas_by_file: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct SchemaRegistryDocument {
    schemas: Vec<SchemaRegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct SchemaRegistryEntry {
    schema_name: String,
    schema_id: String,
    path: String,
}

impl GspContractCatalog {
    /// Loads the complete vendored GSP schema registry after validating its exact shape.
    ///
    /// # Errors
    ///
    /// Returns [`GspContractError`] when registry bytes, entries, paths, IDs or schemas drift.
    pub fn load_vendored(repository_root: &Path) -> Result<Self, GspContractError> {
        let root = vendored_root(repository_root).join("schemas");
        let registry_path = root.join("schema-registry.json");
        let registry_bytes = read(&registry_path)?;
        let actual_registry_digest = sha256_hex(&registry_bytes);
        if actual_registry_digest != GSP_SCHEMA_REGISTRY_SHA256 {
            return Err(GspContractError::RegistryDigestMismatch {
                expected: GSP_SCHEMA_REGISTRY_SHA256.to_owned(),
                actual: actual_registry_digest,
            });
        }
        let registry_value: Value = parse_json(&registry_path, &registry_bytes)?;
        let registry_schema_path = root.join("schema-registry.schema.json");
        let registry_schema = read_json(&registry_schema_path)?;
        let registry_validator = jsonschema::options()
            .should_validate_formats(true)
            .build(&registry_schema)
            .map_err(|error| GspContractError::InvalidRegistry(error.to_string()))?;
        if let Err(error) = registry_validator.validate(&registry_value) {
            return Err(GspContractError::InvalidRegistry(error.to_string()));
        }
        let registry: SchemaRegistryDocument =
            serde_json::from_value(registry_value).map_err(|source| GspContractError::Json {
                path: registry_path.clone(),
                source,
            })?;
        require_count(
            "registry contracts",
            EXPECTED_REGISTRY_CONTRACTS,
            registry.schemas.len(),
        )?;

        let mut schemas_by_file = BTreeMap::new();
        for entry in fs::read_dir(&root).map_err(|source| io(&root, source))? {
            let entry = entry.map_err(|source| io(&root, source))?;
            let path = entry.path();
            if path.extension().and_then(std::ffi::OsStr::to_str) == Some("json") {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                schemas_by_file.insert(file_name, read_json(&path)?);
            }
        }
        require_count(
            "JSON schema artifacts",
            EXPECTED_SCHEMA_ARTIFACTS,
            schemas_by_file.len(),
        )?;

        let mut schemas_by_name = BTreeMap::new();
        let mut seen_paths = BTreeSet::new();
        for entry in registry.schemas {
            let Some(file_name) = entry.path.strip_prefix("schemas/") else {
                return Err(GspContractError::InvalidRegistry(format!(
                    "schema path is outside schemas/: {}",
                    entry.path
                )));
            };
            if !seen_paths.insert(file_name.to_owned()) {
                return Err(GspContractError::InvalidRegistry(format!(
                    "duplicate schema path: {}",
                    entry.path
                )));
            }
            let schema = schemas_by_file.get(file_name).ok_or_else(|| {
                GspContractError::InvalidRegistry(format!("missing schema path: {}", entry.path))
            })?;
            if schema.get("$id").and_then(Value::as_str) != Some(entry.schema_id.as_str()) {
                return Err(GspContractError::InvalidRegistry(format!(
                    "schema ID mismatch for {}",
                    entry.schema_name
                )));
            }
            if schemas_by_name
                .insert(entry.schema_name.clone(), schema.clone())
                .is_some()
            {
                return Err(GspContractError::InvalidRegistry(format!(
                    "duplicate schema name: {}",
                    entry.schema_name
                )));
            }
        }

        let catalog = Self {
            schemas_by_name,
            schemas_by_file,
        };
        catalog.compile_all()?;
        Ok(catalog)
    }

    /// Validates a value against one complete upstream GSP contract.
    ///
    /// # Errors
    ///
    /// Returns [`GspContractError`] for an unknown contract, unresolved schema or invalid value.
    pub fn validate(&self, contract: &str, value: &Value) -> Result<(), GspContractError> {
        let schema = self.schemas_by_name.get(contract).ok_or_else(|| {
            GspContractError::UnknownContract {
                contract: contract.to_owned(),
            }
        })?;
        let registry = self.build_registry()?;
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .with_registry(&registry)
            .build(schema)
            .map_err(|error| GspContractError::InvalidSchema {
                contract: contract.to_owned(),
                message: error.to_string(),
            })?;
        if let Err(error) = validator.validate(value) {
            return Err(GspContractError::InvalidValue {
                contract: contract.to_owned(),
                message: error.to_string(),
            });
        }
        Ok(())
    }

    fn compile_all(&self) -> Result<(), GspContractError> {
        for contract in self.schemas_by_name.keys() {
            let schema = &self.schemas_by_name[contract];
            let registry = self.build_registry()?;
            jsonschema::options()
                .should_validate_formats(true)
                .with_registry(&registry)
                .build(schema)
                .map_err(|error| GspContractError::InvalidSchema {
                    contract: contract.clone(),
                    message: error.to_string(),
                })?;
        }
        Ok(())
    }

    fn build_registry(&self) -> Result<Registry<'_>, GspContractError> {
        let mut registry = Registry::new();
        for (file_name, schema) in &self.schemas_by_file {
            registry = registry
                .add(format!("schema://contracts/{file_name}"), schema)
                .map_err(|error| GspContractError::InvalidSchema {
                    contract: file_name.clone(),
                    message: error.to_string(),
                })?;
            if let Some(schema_id) = schema.get("$id").and_then(Value::as_str)
                && schema_id != format!("schema://contracts/{file_name}")
            {
                registry = registry.add(schema_id, schema).map_err(|error| {
                    GspContractError::InvalidSchema {
                        contract: file_name.clone(),
                        message: error.to_string(),
                    }
                })?;
            }
        }
        registry
            .prepare()
            .map_err(|error| GspContractError::InvalidRegistry(error.to_string()))
    }
}

/// Verifies raw upstream bytes, registry semantics and every contractual fixture outcome.
///
/// # Errors
///
/// Returns [`GspContractError`] on any digest, registry, schema or fixture drift.
pub fn verify_vendored_contracts(
    repository_root: &Path,
) -> Result<GspContractVerificationReport, GspContractError> {
    let root = vendored_root(repository_root);
    let manifest_path = root.join("HASH_MANIFEST.csv");
    let manifest_bytes = read(&manifest_path)?;
    let actual_manifest_digest = sha256_hex(&manifest_bytes);
    if actual_manifest_digest != GSP_FIXTURE_MANIFEST_SHA256 {
        return Err(GspContractError::ManifestDigestMismatch {
            expected: GSP_FIXTURE_MANIFEST_SHA256.to_owned(),
            actual: actual_manifest_digest,
        });
    }
    let manifest = parse_manifest(&manifest_bytes)?;
    verify_tree_bytes(&root.join("schemas"), "core/gsp/schemas", &manifest)?;
    verify_tree_bytes(
        &root.join("fixtures/contracts"),
        "core/gsp/fixtures/contracts",
        &manifest,
    )?;
    let catalog = GspContractCatalog::load_vendored(repository_root)?;
    let fixtures_root = root.join("fixtures/contracts");
    let mut valid_fixture_count = 0_usize;
    let mut invalid_fixture_count = 0_usize;
    for entry in fs::read_dir(&fixtures_root).map_err(|source| io(&fixtures_root, source))? {
        let entry = entry.map_err(|source| io(&fixtures_root, source))?;
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("yaml") {
            return Err(GspContractError::UnexpectedArtifact {
                path: path.display().to_string(),
            });
        }
        let bytes = read(&path)?;
        let fixture: Value =
            serde_yaml::from_slice(&bytes).map_err(|source| GspContractError::Yaml {
                path: path.clone(),
                source,
            })?;
        let contracts = fixture
            .pointer("/meta/expected_contracts")
            .and_then(Value::as_array)
            .ok_or_else(|| GspContractError::FixtureOutcome {
                fixture: entry.file_name().to_string_lossy().into_owned(),
                contract: "missing meta.expected_contracts".to_owned(),
            })?;
        if contracts.len() != 1 {
            return Err(GspContractError::FixtureOutcome {
                fixture: entry.file_name().to_string_lossy().into_owned(),
                contract: "expected exactly one contract".to_owned(),
            });
        }
        let contract = contracts[0]
            .as_str()
            .ok_or_else(|| GspContractError::FixtureOutcome {
                fixture: entry.file_name().to_string_lossy().into_owned(),
                contract: "contract name is not text".to_owned(),
            })?;
        let payload = fixture
            .get("payload")
            .ok_or_else(|| GspContractError::FixtureOutcome {
                fixture: entry.file_name().to_string_lossy().into_owned(),
                contract: contract.to_owned(),
            })?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let expected_valid = if file_name.contains("__valid_") {
            valid_fixture_count += 1;
            true
        } else if file_name.contains("__invalid_") {
            invalid_fixture_count += 1;
            false
        } else {
            return Err(GspContractError::FixtureOutcome {
                fixture: file_name,
                contract: contract.to_owned(),
            });
        };
        if catalog.validate(contract, payload).is_ok() != expected_valid {
            return Err(GspContractError::FixtureOutcome {
                fixture: file_name,
                contract: contract.to_owned(),
            });
        }
    }
    require_count(
        "valid fixtures",
        EXPECTED_VALID_FIXTURES,
        valid_fixture_count,
    )?;
    require_count(
        "invalid fixtures",
        EXPECTED_INVALID_FIXTURES,
        invalid_fixture_count,
    )?;

    Ok(GspContractVerificationReport {
        registry_contract_count: catalog.schemas_by_name.len(),
        schema_artifact_count: catalog.schemas_by_file.len(),
        valid_fixture_count,
        invalid_fixture_count,
    })
}

fn vendored_root(repository_root: &Path) -> PathBuf {
    repository_root.join("integrations/gsp/contracts/gsp-exact-2026.04.12")
}

fn parse_manifest(bytes: &[u8]) -> Result<BTreeMap<String, String>, GspContractError> {
    let text = std::str::from_utf8(bytes).map_err(|_| GspContractError::ManifestMalformed {
        row: "manifest is not UTF-8".to_owned(),
    })?;
    let mut lines = text.lines();
    if lines.next() != Some("path,sha256_in_slice,sha256_in_upstream,exact_match") {
        return Err(GspContractError::ManifestMalformed {
            row: "header".to_owned(),
        });
    }
    let mut manifest = BTreeMap::new();
    for line in lines {
        let columns = line.split(',').collect::<Vec<_>>();
        if columns.len() != 4
            || columns[0].is_empty()
            || columns[1].len() != 64
            || columns[1] != columns[2]
            || columns[3] != "YES"
            || manifest
                .insert(columns[0].to_owned(), columns[2].to_owned())
                .is_some()
        {
            return Err(GspContractError::ManifestMalformed {
                row: line.to_owned(),
            });
        }
    }
    Ok(manifest)
}

fn verify_tree_bytes(
    local_root: &Path,
    upstream_prefix: &str,
    manifest: &BTreeMap<String, String>,
) -> Result<(), GspContractError> {
    for entry in fs::read_dir(local_root).map_err(|source| io(local_root, source))? {
        let entry = entry.map_err(|source| io(local_root, source))?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|source| io(&path, source))?
            .is_dir()
        {
            return Err(GspContractError::UnexpectedArtifact {
                path: path.display().to_string(),
            });
        }
        let upstream_path = format!("{upstream_prefix}/{}", entry.file_name().to_string_lossy());
        verify_one_raw(&path, &upstream_path, manifest)?;
    }
    Ok(())
}

fn verify_one_raw(
    path: &Path,
    upstream_path: &str,
    manifest: &BTreeMap<String, String>,
) -> Result<(), GspContractError> {
    let expected =
        manifest
            .get(upstream_path)
            .ok_or_else(|| GspContractError::UnexpectedArtifact {
                path: upstream_path.to_owned(),
            })?;
    let actual = sha256_hex(&read(path)?);
    if &actual != expected {
        return Err(GspContractError::RawDigestMismatch {
            path: upstream_path.to_owned(),
            expected: expected.clone(),
            actual,
        });
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, GspContractError> {
    let bytes = read(path)?;
    parse_json(path, &bytes)
}

fn parse_json(path: &Path, bytes: &[u8]) -> Result<Value, GspContractError> {
    serde_json::from_slice(bytes).map_err(|source| GspContractError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read(path: &Path) -> Result<Vec<u8>, GspContractError> {
    fs::read(path).map_err(|source| io(path, source))
}

fn io(path: &Path, source: std::io::Error) -> GspContractError {
    GspContractError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    sha256_bytes(bytes).to_string()[7..].to_owned()
}

fn require_count(
    kind: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), GspContractError> {
    if actual != expected {
        return Err(GspContractError::CountMismatch {
            kind,
            expected,
            actual,
        });
    }
    Ok(())
}
