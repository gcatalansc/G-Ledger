use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_bytes, sha256_digest, to_canonical_vec,
};
use project_ledger_domain::{
    EventEnvelope, LedgerTimestamp, PortableExport, PortableImportPolicy, ResumeProjection,
    SchemaVersion, StableId,
};
use project_ledger_event_store::{IntegrityError, validate_event_integrity};
use project_ledger_projection_engine::{ProjectStateProjection, ProjectionError};
use serde_json::Value;
use thiserror::Error;

const CHECKSUMS: &str = "checksums.sha256";
const EVENTS: &str = "events.jsonl";
const MANIFEST: &str = "manifest.json";
const SCHEMAS_LOCK: &str = "schemas-lock.json";

/// Complete inputs for the fixed §31.1 portable directory tree.
#[derive(Clone, Debug)]
pub struct PortableExportRequest {
    pub export_id: StableId,
    pub created_at: LedgerTimestamp,
    pub import_policy: PortableImportPolicy,
    pub stream: Vec<EventEnvelope>,
    pub schemas_lock: Value,
    pub upstream_locks: BTreeMap<String, Value>,
    pub resume: ResumeProjection,
    pub evidence_references: Value,
    pub radar_projection: Value,
    pub generated_markdown: Vec<u8>,
}

/// One deterministic in-memory representation of a portable directory bundle.
#[derive(Clone, Debug)]
pub struct PortableBundle {
    pub(crate) manifest: PortableExport,
    pub(crate) events: Vec<EventEnvelope>,
    pub(crate) entries: BTreeMap<String, Vec<u8>>,
}

/// Fail-closed export, verification and filesystem failures.
#[derive(Debug, Error)]
pub enum PortableBundleError {
    #[error("portable export requires a non-empty v0.2.0 event stream")]
    EmptyStream,
    #[error("portable export requires explicit migration before reading schema {schema_version:?}")]
    MigrationRequired { schema_version: SchemaVersion },
    #[error("portable event stream is invalid: {message}")]
    InvalidStream { message: String },
    #[error("portable bundle integrity failed: {message}")]
    Integrity { message: String },
    #[error("portable export rejected unsafe or private material at {field}")]
    PrivateMaterial { field: String },
    #[error("upstream lock name is not one safe JSON filename: {name}")]
    InvalidLockName { name: String },
    #[error("portable destination already exists: {path}")]
    DestinationExists { path: PathBuf },
    #[error("local stream version {local_version} is newer than bundle version {bundle_version}")]
    DestinationNewer {
        local_version: u64,
        bundle_version: u64,
    },
    #[error("same-project event streams diverge at version {version}")]
    StreamDivergence { version: u64 },
    #[error("explicit fork target must be a new project identity")]
    InvalidForkTarget,
    #[error("portable import policy does not authorize this chain transition")]
    ImportPolicyViolation,
    #[error("portable directory contains an unsupported entry: {path}")]
    UnsupportedEntry { path: String },
    #[error("portable filesystem operation failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error(transparent)]
    EventIntegrity(#[from] IntegrityError),
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Store(#[from] project_ledger_event_store::StoreError),
}

impl PortableBundle {
    /// Builds all portable bytes from one already event-sourced project.
    ///
    /// # Errors
    ///
    /// Returns [`PortableBundleError`] before producing a bundle when the stream,
    /// privacy boundary, lock names or canonical artifacts are invalid.
    pub fn build(request: PortableExportRequest) -> Result<Self, PortableBundleError> {
        let projection = validate_stream(&request.stream)?;
        validate_private_value("schemas-lock.json", &request.schemas_lock)?;
        validate_private_value("evidence/references.json", &request.evidence_references)?;
        validate_private_value(
            "integrations/radar-project-projection.json",
            &request.radar_projection,
        )?;
        validate_private_bytes(
            "generated/BITACORA_PROYECTO.md",
            &request.generated_markdown,
        )?;

        let mut entries = BTreeMap::new();
        insert_json(&mut entries, "project.json", &projection.state().project)?;
        insert_json(&mut entries, "projections/state.json", projection.state())?;
        insert_json(&mut entries, "projections/resume.json", &request.resume)?;
        insert_value(
            &mut entries,
            "evidence/references.json",
            &request.evidence_references,
        )?;
        insert_value(
            &mut entries,
            "integrations/radar-project-projection.json",
            &request.radar_projection,
        )?;
        insert_value(&mut entries, SCHEMAS_LOCK, &request.schemas_lock)?;

        for (name, lock) in &request.upstream_locks {
            validate_lock_name(name)?;
            validate_private_value(&format!("upstream-locks/{name}"), lock)?;
            insert_value(&mut entries, &format!("upstream-locks/{name}"), lock)?;
        }
        entries.insert(EVENTS.to_owned(), encode_events(&request.stream)?);
        entries.insert(
            "generated/BITACORA_PROYECTO.md".to_owned(),
            request.generated_markdown,
        );

        let checksums = encode_checksums(&entries);
        let checksums_digest = sha256_bytes(&checksums);
        let schemas_lock_digest = sha256_bytes(required_entry(&entries, SCHEMAS_LOCK)?);
        entries.insert(CHECKSUMS.to_owned(), checksums);

        let first = request
            .stream
            .first()
            .ok_or(PortableBundleError::EmptyStream)?;
        let last = request
            .stream
            .last()
            .ok_or(PortableBundleError::EmptyStream)?;
        let mut manifest = PortableExport {
            schema_version: SchemaVersion::V0_2_0,
            export_id: request.export_id,
            project_ref: projection.state().project.project_id.clone(),
            project_state_version: projection.state().state_version,
            project_state_digest: projection.state().state_digest,
            first_event_ref: first.event_id.clone(),
            last_event_ref: last.event_id.clone(),
            event_count: request.stream.len() as u64,
            chain_digest: last.event_digest,
            import_policy: request.import_policy,
            schemas_lock_digest,
            checksums_digest,
            created_at: request.created_at,
            digest: zero_digest(),
        };
        manifest.digest = record_digest(&manifest)?;
        entries.insert(MANIFEST.to_owned(), to_canonical_vec(&manifest)?);

        let bundle = Self {
            manifest,
            events: request.stream,
            entries,
        };
        bundle.verify()?;
        Ok(bundle)
    }

    /// Returns the durable portable manifest.
    pub const fn manifest(&self) -> &PortableExport {
        &self.manifest
    }

    /// Returns the verified exported event sequence.
    pub fn events(&self) -> &[EventEnvelope] {
        &self.events
    }

    /// Returns every portable file in stable path order.
    pub const fn entries(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.entries
    }

    /// Returns one portable file by its normalized relative path.
    pub fn entry(&self, path: &str) -> Option<&[u8]> {
        self.entries.get(path).map(Vec::as_slice)
    }

    /// Revalidates all self-contained export invariants without I/O or mutation.
    ///
    /// # Errors
    ///
    /// Returns [`PortableBundleError::Integrity`] when any byte or binding has
    /// diverged from the manifest, checksums, stream or rebuilt state.
    pub fn verify(&self) -> Result<(), PortableBundleError> {
        if self.manifest.schema_version != SchemaVersion::V0_2_0 {
            return Err(PortableBundleError::MigrationRequired {
                schema_version: self.manifest.schema_version,
            });
        }
        if record_digest(&self.manifest)? != self.manifest.digest {
            return integrity("manifest digest mismatch");
        }
        if self.entries.get(MANIFEST) != Some(&to_canonical_vec(&self.manifest)?) {
            return integrity("manifest bytes mismatch");
        }
        let checksums = required_entry(&self.entries, CHECKSUMS)?;
        if sha256_bytes(checksums) != self.manifest.checksums_digest {
            return integrity("checksums digest mismatch");
        }
        if checksums != encode_checksums_without_control_files(&self.entries).as_slice() {
            return integrity("checksums do not match portable payloads");
        }
        let schemas_lock = required_entry(&self.entries, SCHEMAS_LOCK)?;
        if sha256_bytes(schemas_lock) != self.manifest.schemas_lock_digest {
            return integrity("schemas lock digest mismatch");
        }
        validate_schema_lock(schemas_lock)?;
        let projection = validate_stream(&self.events)?;
        let first = self
            .events
            .first()
            .ok_or(PortableBundleError::EmptyStream)?;
        let last = self.events.last().ok_or(PortableBundleError::EmptyStream)?;
        if self.manifest.project_ref != projection.state().project.project_id
            || self.manifest.project_state_version != projection.state().state_version
            || self.manifest.project_state_digest != projection.state().state_digest
            || self.manifest.first_event_ref != first.event_id
            || self.manifest.last_event_ref != last.event_id
            || self.manifest.event_count != self.events.len() as u64
            || self.manifest.chain_digest != last.event_digest
        {
            return integrity("manifest does not bind the rebuilt stream and state");
        }
        if required_entry(&self.entries, EVENTS)? != encode_events(&self.events)?.as_slice()
            || required_entry(&self.entries, "project.json")?
                != to_canonical_vec(&projection.state().project)?.as_slice()
            || required_entry(&self.entries, "projections/state.json")?
                != to_canonical_vec(projection.state())?.as_slice()
        {
            return integrity("portable content diverges from the verified event stream");
        }
        for (path, bytes) in self
            .entries
            .iter()
            .filter(|(path, _)| path.to_ascii_lowercase().ends_with(".json"))
        {
            let value: Value = serde_json::from_slice(bytes)?;
            if to_canonical_vec(&value)?.as_slice() != bytes {
                return integrity(&format!("portable JSON is not canonical: {path}"));
            }
        }
        Ok(())
    }

    /// Materializes the fixed directory tree without overwriting an existing path.
    ///
    /// # Errors
    ///
    /// Returns an error when the destination exists or any directory/file cannot
    /// be created. The function never follows caller-controlled bundle paths.
    pub fn write_directory(&self, destination: &Path) -> Result<(), PortableBundleError> {
        self.verify()?;
        if destination.exists() {
            return Err(PortableBundleError::DestinationExists {
                path: destination.to_path_buf(),
            });
        }
        fs::create_dir(destination).map_err(|source| io(destination, source))?;
        let upstream_locks = destination.join("upstream-locks");
        fs::create_dir(&upstream_locks).map_err(|source| io(&upstream_locks, source))?;
        for (relative, bytes) in &self.entries {
            let path = destination.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| io(parent, source))?;
            }
            fs::write(&path, bytes).map_err(|source| io(&path, source))?;
        }
        Ok(())
    }
}

pub(crate) fn validate_stream(
    events: &[EventEnvelope],
) -> Result<ProjectStateProjection, PortableBundleError> {
    let Some(first) = events.first() else {
        return Err(PortableBundleError::EmptyStream);
    };
    let project_ref = &first.project_ref;
    let mut previous = None;
    for (index, event) in events.iter().enumerate() {
        if event.schema_version != SchemaVersion::V0_2_0 {
            return Err(PortableBundleError::MigrationRequired {
                schema_version: event.schema_version,
            });
        }
        validate_event_integrity(event)?;
        if event.project_ref != *project_ref
            || event.stream_id != *project_ref
            || event.stream_version != index as u64 + 1
            || event.previous_event_digest != previous
        {
            return Err(PortableBundleError::InvalidStream {
                message: format!(
                    "identity, version or previous digest failed at event {}",
                    index + 1
                ),
            });
        }
        previous = Some(event.event_digest);
    }
    ProjectStateProjection::rebuild(events).map_err(PortableBundleError::from)
}

fn encode_events(events: &[EventEnvelope]) -> Result<Vec<u8>, PortableBundleError> {
    let mut bytes = Vec::new();
    for event in events {
        bytes.extend(to_canonical_vec(event)?);
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn insert_json<T: serde::Serialize>(
    entries: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &T,
) -> Result<(), PortableBundleError> {
    entries.insert(path.to_owned(), to_canonical_vec(value)?);
    Ok(())
}

fn insert_value(
    entries: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &Value,
) -> Result<(), PortableBundleError> {
    insert_json(entries, path, value)
}

fn encode_checksums(entries: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut encoded = String::new();
    for (path, bytes) in entries {
        let digest = sha256_bytes(bytes).to_string();
        encoded.push_str(
            digest
                .strip_prefix("sha256:")
                .expect("canonical digest prefix"),
        );
        encoded.push_str("  ");
        encoded.push_str(path);
        encoded.push('\n');
    }
    encoded.into_bytes()
}

fn encode_checksums_without_control_files(entries: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let filtered = entries
        .iter()
        .filter(|(path, _)| path.as_str() != MANIFEST && path.as_str() != CHECKSUMS)
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    encode_checksums(&filtered)
}

fn record_digest<T: serde::Serialize>(record: &T) -> Result<Sha256Digest, PortableBundleError> {
    let mut value = serde_json::to_value(record)?;
    value
        .as_object_mut()
        .ok_or_else(|| PortableBundleError::Integrity {
            message: "digest-bearing contract is not a JSON object".to_owned(),
        })?
        .remove("digest");
    Ok(sha256_digest(&value)?)
}

fn required_entry<'a>(
    entries: &'a BTreeMap<String, Vec<u8>>,
    path: &str,
) -> Result<&'a [u8], PortableBundleError> {
    entries
        .get(path)
        .map(Vec::as_slice)
        .ok_or_else(|| PortableBundleError::Integrity {
            message: format!("required entry is missing: {path}"),
        })
}

fn validate_lock_name(name: &str) -> Result<(), PortableBundleError> {
    if name.is_empty()
        || !name.to_ascii_lowercase().ends_with(".json")
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
    {
        return Err(PortableBundleError::InvalidLockName {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn validate_schema_lock(bytes: &[u8]) -> Result<(), PortableBundleError> {
    let value: Value = serde_json::from_slice(bytes)?;
    match value.get("schema_version").and_then(Value::as_str) {
        Some("0.2.0") => Ok(()),
        Some("0.1.1") => Err(PortableBundleError::MigrationRequired {
            schema_version: SchemaVersion::V0_1_1,
        }),
        _ => integrity("schemas lock does not declare a supported schema_version"),
    }
}

fn validate_private_value(path: &str, value: &Value) -> Result<(), PortableBundleError> {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase().replace('-', "_");
                if matches!(
                    normalized.as_str(),
                    "secret"
                        | "password"
                        | "token"
                        | "api_key"
                        | "raw_output"
                        | "transcript"
                        | "reasoning"
                        | "private_id"
                        | "encrypted_blob"
                ) {
                    return Err(PortableBundleError::PrivateMaterial {
                        field: format!("{path}/{key}"),
                    });
                }
                validate_private_value(&format!("{path}/{key}"), nested)?;
            }
        }
        Value::Array(values) => {
            for (index, nested) in values.iter().enumerate() {
                validate_private_value(&format!("{path}/{index}"), nested)?;
            }
        }
        Value::String(text) if looks_like_absolute_path(text) => {
            return Err(PortableBundleError::PrivateMaterial {
                field: path.to_owned(),
            });
        }
        _ => {}
    }
    Ok(())
}

fn validate_private_bytes(path: &str, bytes: &[u8]) -> Result<(), PortableBundleError> {
    let text = std::str::from_utf8(bytes).map_err(|_| PortableBundleError::PrivateMaterial {
        field: path.to_owned(),
    })?;
    if text.contains('\r') {
        return Err(PortableBundleError::Integrity {
            message: format!("portable text must use LF only: {path}"),
        });
    }
    let lowered = text.to_ascii_lowercase();
    if ["password=", "token=", "secret=", "api_key=", "raw output"]
        .iter()
        .any(|marker| lowered.contains(marker))
        || text.lines().any(looks_like_absolute_path)
    {
        return Err(PortableBundleError::PrivateMaterial {
            field: path.to_owned(),
        });
    }
    Ok(())
}

fn looks_like_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('/')
        || value.starts_with("\\\\")
        || value.to_ascii_lowercase().starts_with("file://")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

fn integrity<T>(message: &str) -> Result<T, PortableBundleError> {
    Err(PortableBundleError::Integrity {
        message: message.to_owned(),
    })
}

fn io(path: &Path, source: std::io::Error) -> PortableBundleError {
    PortableBundleError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from([0_u8; 32])
}
