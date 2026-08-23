use std::{collections::BTreeMap, fs, io, path::Path};

use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::SchemaVersion;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::{SchemaError, command_event_matrix, fixture_catalog, schema_catalog};

#[derive(Debug, Error)]
pub enum GenerationError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("JSON generation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("canonical digest generation failed: {0}")]
    Canonical(#[from] project_ledger_canonical_json::CanonicalJsonError),
    #[error("I/O failed for {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("generated artifact drift: {0}")]
    Drift(String),
}

#[derive(Serialize)]
struct RegistryDocument {
    schema_version: SchemaVersion,
    entries: Vec<RegistryEntry>,
}

#[derive(Serialize)]
struct RegistryEntry {
    schema_id: String,
    relative_path: String,
    canonical_digest: String,
}

#[derive(Serialize)]
struct FixtureManifest {
    schema_version: SchemaVersion,
    entries: Vec<FixtureManifestEntry>,
}

#[derive(Serialize)]
struct FixtureManifestEntry {
    schema_id: String,
    positive_path: String,
    positive_digest: String,
    negative_path: String,
    negative_digest: String,
}

#[derive(Serialize)]
struct ArraySemanticsDocument {
    schema_version: SchemaVersion,
    entries: Vec<ArraySemanticsEntry>,
}

#[derive(Serialize)]
struct ArraySemanticsEntry {
    schema_id: String,
    schema_pointer: String,
    class: &'static str,
    canonical_key: Option<&'static str>,
}

/// Produces every deterministic schema, fixture, registry, and matrix artifact.
///
/// # Errors
///
/// Returns [`GenerationError`] when contract generation, canonical hashing, or
/// JSON serialization fails.
pub fn generated_files() -> Result<BTreeMap<String, Vec<u8>>, GenerationError> {
    let schemas = schema_catalog()?;
    let fixtures = fixture_catalog()?;
    let mut files = BTreeMap::new();
    let mut registry_entries = Vec::with_capacity(schemas.len());
    let mut fixture_entries = Vec::with_capacity(fixtures.len());
    let mut array_entries = Vec::new();

    for schema in &schemas {
        registry_entries.push(RegistryEntry {
            schema_id: schema.schema_id.clone(),
            relative_path: schema.relative_path.clone(),
            canonical_digest: sha256_digest(&schema.schema)?.to_string(),
        });
        collect_array_semantics(&schema.schema_id, &schema.schema, "#", &mut array_entries);
        files.insert(schema.relative_path.clone(), pretty_bytes(&schema.schema)?);
    }

    for fixture in fixtures {
        let slug = fixture_slug(&fixture.schema_id);
        let positive_path = format!("fixtures/positive/{slug}.json");
        let negative_path = format!("fixtures/negative/{slug}.json");
        fixture_entries.push(FixtureManifestEntry {
            schema_id: fixture.schema_id,
            positive_path: positive_path.clone(),
            positive_digest: sha256_digest(&fixture.positive)?.to_string(),
            negative_path: negative_path.clone(),
            negative_digest: sha256_digest(&fixture.negative)?.to_string(),
        });
        files.insert(positive_path, pretty_bytes(&fixture.positive)?);
        files.insert(negative_path, pretty_bytes(&fixture.negative)?);
    }

    registry_entries.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
    fixture_entries.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
    array_entries.sort_by(|left, right| {
        (&left.schema_id, &left.schema_pointer).cmp(&(&right.schema_id, &right.schema_pointer))
    });

    files.insert(
        "schema-registry.json".to_owned(),
        pretty_bytes(&RegistryDocument {
            schema_version: SchemaVersion::V0_2_0,
            entries: registry_entries,
        })?,
    );
    files.insert(
        "fixtures/manifest.json".to_owned(),
        pretty_bytes(&FixtureManifest {
            schema_version: SchemaVersion::V0_2_0,
            entries: fixture_entries,
        })?,
    );
    files.insert(
        "array-semantics.json".to_owned(),
        pretty_bytes(&ArraySemanticsDocument {
            schema_version: SchemaVersion::V0_2_0,
            entries: array_entries,
        })?,
    );
    files.insert(
        "command-event-matrix.json".to_owned(),
        pretty_bytes(&command_event_matrix())?,
    );
    Ok(files)
}

/// Writes all generated contract artifacts below `schemas/v0.2`.
///
/// # Errors
///
/// Returns [`GenerationError`] when generation fails or a directory or file
/// cannot be written.
pub fn write_generated(repository_root: &Path) -> Result<(), GenerationError> {
    let output_root = repository_root.join("schemas").join("v0.2");
    for (relative_path, bytes) in generated_files()? {
        let path = output_root.join(&relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| GenerationError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        fs::write(&path, bytes).map_err(|source| GenerationError::Io {
            path: path.display().to_string(),
            source,
        })?;
    }
    Ok(())
}

/// Verifies that committed artifacts exactly match a fresh deterministic build.
///
/// # Errors
///
/// Returns [`GenerationError::Drift`] on byte-level divergence, or another
/// [`GenerationError`] when generation or file reads fail.
pub fn check_generated(repository_root: &Path) -> Result<(), GenerationError> {
    let output_root = repository_root.join("schemas").join("v0.2");
    for (relative_path, expected) in generated_files()? {
        let path = output_root.join(&relative_path);
        let actual = fs::read(&path).map_err(|source| GenerationError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if actual != expected {
            return Err(GenerationError::Drift(relative_path));
        }
    }
    Ok(())
}

fn pretty_bytes<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize,
{
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn fixture_slug(schema_id: &str) -> String {
    schema_id
        .trim_start_matches("urn:project-ledger:schema:")
        .trim_end_matches(":0.2.0")
        .trim_end_matches(":0.1.1")
        .replace(':', "-")
}

fn collect_array_semantics(
    schema_id: &str,
    value: &Value,
    pointer: &str,
    entries: &mut Vec<ArraySemanticsEntry>,
) {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("array") {
                let field = pointer.rsplit('/').next().unwrap_or_default();
                let is_sequence = matches!(
                    field,
                    "phase_order"
                        | "ordered_next_action_refs"
                        | "next_action_refs"
                        | "last_material_event_refs"
                        | "required_events"
                        | "emitted_event_refs"
                );
                entries.push(ArraySemanticsEntry {
                    schema_id: schema_id.to_owned(),
                    schema_pointer: pointer.to_owned(),
                    class: if is_sequence {
                        "SEQUENCE"
                    } else {
                        "STABLE_SET"
                    },
                    canonical_key: if is_sequence {
                        None
                    } else {
                        Some("CANONICAL_ELEMENT_DIGEST")
                    },
                });
            }
            for (key, child) in object {
                collect_array_semantics(
                    schema_id,
                    child,
                    &format!("{pointer}/{}", escape_pointer(key)),
                    entries,
                );
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                collect_array_semantics(schema_id, child, &format!("{pointer}/{index}"), entries);
            }
        }
        _ => {}
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
