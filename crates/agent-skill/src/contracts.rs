use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use jsonschema::Validator;
use project_ledger_canonical_json::{sha256_bytes, sha256_digest};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

pub const AGENT_SKILL_PROFILE: &str = "project-ledger-agent-skill/0.2.0";
const PACKAGE_ROOT: &str = "integrations/agent-skills/project-ledger-continuity";
const LOCK_PATH: &str = "integrations/agent-skills/profile-lock.json";
const SOURCE_SPEC_SHA256: &str = "31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillContractReport {
    pub profile: String,
    pub skill_version: String,
    pub required_tool: String,
    pub minimum_tool_version: String,
    pub maximum_tool_version_exclusive: String,
    pub supported_platforms: Vec<String>,
    pub schema_count: usize,
    pub trigger_case_count: usize,
    pub policy_case_count: usize,
    pub state_authority: bool,
    pub core_dependency: bool,
}

#[derive(Debug, Error)]
pub enum AgentSkillContractError {
    #[error("cannot read Agent Skill artifact {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Agent Skill JSON is invalid at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "Agent Skill artifact {path} raw digest mismatch: expected {expected}, actual {actual}"
    )]
    RawDigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("Agent Skill self-digest mismatch at {path}: expected {expected}, actual {actual}")]
    SelfDigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("Agent Skill schema {name} cannot compile: {message}")]
    InvalidSchema { name: String, message: String },
    #[error("Agent Skill value {name} does not satisfy its schema: {message}")]
    InvalidValue { name: String, message: String },
    #[error("Agent Skill package contract differs at {field}")]
    ContractMismatch { field: &'static str },
    #[error("Agent Skill package contains duplicate artifact {path}")]
    DuplicateArtifact { path: String },
    #[error("Agent Skill executable content contains concrete current state at {path}")]
    CurrentStateEmbedded { path: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileLock {
    schema_version: String,
    integration: String,
    product_version: String,
    source_spec_sha256: String,
    profile: String,
    schema_registry_sha256: String,
    fixture_manifest_sha256: String,
    compatibility_range: CompatibilityRange,
    contract_status: String,
    content_status: String,
    degradation_status: String,
    policy_status: String,
    handoff_e2e_status: String,
    state_authority: bool,
    core_dependency: bool,
    certified_compatibility: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompatibilityRange {
    protocol: String,
    minimum: String,
    maximum: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaRegistry {
    schema_version: String,
    profile: String,
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
    trigger_cases: CaseArtifact,
    policy_cases: CaseArtifact,
    instructions: Artifact,
    manifest: Artifact,
    workflow: Artifact,
    examples: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseArtifact {
    path: String,
    raw_sha256: String,
    count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    raw_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillManifest {
    schema_version: String,
    name: String,
    version: String,
    profile: String,
    description: String,
    required_tools: Vec<RequiredTool>,
    supported_platforms: Vec<String>,
    compatibility_range: CompatibilityRange,
    state_authority: bool,
    core_dependency: bool,
    workflow_ref: String,
    trigger_cases_ref: String,
    policy_cases_ref: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiredTool {
    name: String,
    minimum_version: String,
    maximum_version_exclusive: String,
}

/// Verifies the exact offline skill package, its manifests and every locked byte.
///
/// # Errors
///
/// Returns [`AgentSkillContractError`] when raw bytes, schemas, self-digests,
/// profile metadata, fixture counts or state-authority boundaries differ.
#[allow(clippy::too_many_lines)] // The raw-before-semantic audit sequence is intentionally linear.
pub fn verify_agent_skill_package(
    repository_root: &Path,
) -> Result<AgentSkillContractReport, AgentSkillContractError> {
    let lock_path = repository_root.join(LOCK_PATH);
    let lock: ProfileLock = read_json(&lock_path)?;
    verify_lock_header(&lock)?;

    let package_root = repository_root.join(PACKAGE_ROOT);
    let registry_path = package_root.join("schemas/schema-registry.json");
    verify_raw_path(
        &registry_path,
        "schemas/schema-registry.json",
        &lock.schema_registry_sha256,
    )?;
    let registry: SchemaRegistry = read_json(&registry_path)?;
    let fixtures_path = package_root.join("tests/fixture-manifest.json");
    verify_raw_path(
        &fixtures_path,
        "tests/fixture-manifest.json",
        &lock.fixture_manifest_sha256,
    )?;
    let fixtures: FixtureManifest = read_json(&fixtures_path)?;
    verify_profile_headers(&registry, &fixtures)?;

    let mut seen = BTreeSet::new();
    let mut schemas = Vec::with_capacity(registry.entries.len());
    for entry in &registry.entries {
        if !seen.insert(entry.path.clone()) {
            return Err(AgentSkillContractError::DuplicateArtifact {
                path: entry.path.clone(),
            });
        }
        let path = package_root.join(&entry.path);
        verify_raw_path(&path, &entry.path, &entry.raw_sha256)?;
        let schema: Value = read_json(&path)?;
        if schema.get("$id").and_then(Value::as_str) != Some(entry.schema_id.as_str()) {
            return Err(mismatch("schema identity"));
        }
        let validator =
            Validator::new(&schema).map_err(|error| AgentSkillContractError::InvalidSchema {
                name: entry.name.clone(),
                message: error.to_string(),
            })?;
        schemas.push((entry.name.as_str(), validator));
    }
    if schemas.len() != 2 {
        return Err(mismatch("schema count"));
    }

    let artifacts = [&fixtures.trigger_cases, &fixtures.policy_cases];
    for artifact in artifacts {
        if !seen.insert(artifact.path.clone()) {
            return Err(AgentSkillContractError::DuplicateArtifact {
                path: artifact.path.clone(),
            });
        }
        verify_raw_path(
            &package_root.join(&artifact.path),
            &artifact.path,
            &artifact.raw_sha256,
        )?;
    }
    for artifact in [
        &fixtures.instructions,
        &fixtures.manifest,
        &fixtures.workflow,
    ]
    .into_iter()
    .chain(fixtures.examples.iter())
    {
        if !seen.insert(artifact.path.clone()) {
            return Err(AgentSkillContractError::DuplicateArtifact {
                path: artifact.path.clone(),
            });
        }
        let path = package_root.join(&artifact.path);
        verify_raw_path(&path, &artifact.path, &artifact.raw_sha256)?;
        reject_concrete_state(&path, &artifact.path)?;
    }

    let manifest_path = package_root.join(&fixtures.manifest.path);
    let manifest_value: Value = read_json(&manifest_path)?;
    validate(&schemas, "manifest", &manifest_value)?;
    verify_self_digest(&fixtures.manifest.path, &manifest_value)?;
    let manifest: SkillManifest =
        serde_json::from_value(manifest_value).map_err(|source| AgentSkillContractError::Json {
            path: manifest_path,
            source,
        })?;
    verify_manifest(&manifest)?;

    let workflow_path = package_root.join(&fixtures.workflow.path);
    let workflow: Value = read_json(&workflow_path)?;
    validate(&schemas, "workflow", &workflow)?;
    verify_self_digest(&fixtures.workflow.path, &workflow)?;

    let trigger_values: Vec<Value> = read_json(&package_root.join(&fixtures.trigger_cases.path))?;
    let policy_values: Vec<Value> = read_json(&package_root.join(&fixtures.policy_cases.path))?;
    if trigger_values.len() != fixtures.trigger_cases.count
        || policy_values.len() != fixtures.policy_cases.count
    {
        return Err(mismatch("fixture counts"));
    }

    let tool = manifest
        .required_tools
        .first()
        .ok_or_else(|| mismatch("required tool"))?;
    Ok(AgentSkillContractReport {
        profile: manifest.profile,
        skill_version: manifest.version,
        required_tool: tool.name.clone(),
        minimum_tool_version: tool.minimum_version.clone(),
        maximum_tool_version_exclusive: tool.maximum_version_exclusive.clone(),
        supported_platforms: manifest.supported_platforms,
        schema_count: schemas.len(),
        trigger_case_count: trigger_values.len(),
        policy_case_count: policy_values.len(),
        state_authority: manifest.state_authority,
        core_dependency: manifest.core_dependency,
    })
}

fn verify_lock_header(lock: &ProfileLock) -> Result<(), AgentSkillContractError> {
    if lock.schema_version != "1.0.0"
        || lock.integration != "agent-skills-codex"
        || lock.product_version != "0.2.0"
        || lock.source_spec_sha256 != SOURCE_SPEC_SHA256
        || lock.profile != AGENT_SKILL_PROFILE
        || lock.compatibility_range.protocol != "project-ledger-agent-skill"
        || lock.compatibility_range.minimum != "0.2.0"
        || lock.compatibility_range.maximum != "0.2.x"
        || lock.contract_status != "PASS_LOCAL"
        || lock.content_status != "PASS_NO_CURRENT_STATE"
        || lock.degradation_status != "PASS_FAIL_CLOSED"
        || lock.policy_status != "PASS_NO_AUTHORITY_BYPASS"
        || lock.handoff_e2e_status != "PASS_LOCAL_SQLITE"
        || lock.state_authority
        || lock.core_dependency
        || lock.certified_compatibility
    {
        return Err(mismatch("profile lock"));
    }
    Ok(())
}

fn verify_profile_headers(
    registry: &SchemaRegistry,
    fixtures: &FixtureManifest,
) -> Result<(), AgentSkillContractError> {
    if registry.schema_version != "1.0.0"
        || fixtures.schema_version != "1.0.0"
        || registry.profile != AGENT_SKILL_PROFILE
        || fixtures.profile != AGENT_SKILL_PROFILE
    {
        return Err(mismatch("registry/fixture profile"));
    }
    Ok(())
}

fn verify_manifest(manifest: &SkillManifest) -> Result<(), AgentSkillContractError> {
    let expected_platforms = ["linux", "macos", "windows"];
    let Some(tool) = manifest.required_tools.as_slice().first() else {
        return Err(mismatch("required tool"));
    };
    if manifest.schema_version != "1.0.0"
        || manifest.name != "project-ledger-continuity"
        || manifest.version != "0.2.0"
        || manifest.profile != AGENT_SKILL_PROFILE
        || manifest.description.trim().is_empty()
        || manifest.required_tools.len() != 1
        || tool.name != "project-ledger"
        || tool.minimum_version != "0.2.0"
        || tool.maximum_version_exclusive != "0.3.0"
        || manifest
            .supported_platforms
            .iter()
            .map(String::as_str)
            .ne(expected_platforms)
        || manifest.compatibility_range.protocol != "project-ledger"
        || manifest.compatibility_range.minimum != "0.2.0"
        || manifest.compatibility_range.maximum != "0.2.x"
        || manifest.state_authority
        || manifest.core_dependency
        || manifest.workflow_ref != "workflow.json"
        || manifest.trigger_cases_ref != "tests/trigger-cases.json"
        || manifest.policy_cases_ref != "tests/policy-cases.json"
        || manifest.digest.is_empty()
    {
        return Err(mismatch("manifest"));
    }
    Ok(())
}

fn validate(
    schemas: &[(&str, Validator)],
    name: &str,
    value: &Value,
) -> Result<(), AgentSkillContractError> {
    let validator = schemas
        .iter()
        .find_map(|(candidate, validator)| (*candidate == name).then_some(validator))
        .ok_or_else(|| mismatch("schema association"))?;
    validator
        .validate(value)
        .map_err(|error| AgentSkillContractError::InvalidValue {
            name: name.to_owned(),
            message: error.to_string(),
        })
}

fn verify_self_digest(path: &str, value: &Value) -> Result<(), AgentSkillContractError> {
    let expected = value
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| mismatch("self digest"))?
        .to_owned();
    let mut unsigned = value.clone();
    unsigned
        .as_object_mut()
        .ok_or_else(|| mismatch("self digest object"))?
        .remove("digest");
    let actual = sha256_digest(&unsigned)
        .map_err(|_| mismatch("canonical self digest"))?
        .to_string();
    if expected != actual {
        return Err(AgentSkillContractError::SelfDigestMismatch {
            path: path.to_owned(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn reject_concrete_state(path: &Path, relative: &str) -> Result<(), AgentSkillContractError> {
    let bytes = read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    crate::workflow::inspect_instruction_content(relative, &text).map_err(|_| {
        AgentSkillContractError::CurrentStateEmbedded {
            path: relative.to_owned(),
        }
    })
}

fn verify_raw_path(
    path: &Path,
    relative: &str,
    expected: &str,
) -> Result<(), AgentSkillContractError> {
    let actual = sha256_bytes(&read(path)?).to_string();
    let normalized = expected.strip_prefix("sha256:").unwrap_or(expected);
    let actual_hex = actual.strip_prefix("sha256:").unwrap_or(&actual);
    if normalized != actual_hex {
        return Err(AgentSkillContractError::RawDigestMismatch {
            path: relative.to_owned(),
            expected: normalized.to_owned(),
            actual: actual_hex.to_owned(),
        });
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AgentSkillContractError> {
    let bytes = read(path)?;
    serde_json::from_slice(&bytes).map_err(|source| AgentSkillContractError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read(path: &Path) -> Result<Vec<u8>, AgentSkillContractError> {
    fs::read(path).map_err(|source| AgentSkillContractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

const fn mismatch(field: &'static str) -> AgentSkillContractError {
    AgentSkillContractError::ContractMismatch { field }
}
