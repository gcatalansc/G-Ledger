//! Safe canonical JSON exchange for local out-of-process integrations.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_bytes, to_canonical_vec,
};
use serde::{Serialize, de::DeserializeOwned};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Byte bound applied before creating or decoding an exchange artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactLimits {
    pub max_bytes: u64,
}

impl Default for ArtifactLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024,
        }
    }
}

/// Exact receipt for one published canonical artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactReceipt {
    pub file_name: String,
    pub digest: Sha256Digest,
    pub byte_len: u64,
}

/// Failures at the local artifact-exchange boundary.
#[derive(Debug, Error)]
pub enum ArtifactExchangeError {
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("unsafe artifact filename: {name}")]
    UnsafeFileName { name: String },
    #[error("artifact root is not a trusted regular directory: {path}")]
    UnsafeRoot { path: PathBuf },
    #[error("artifact is not a regular non-symlink file: {path}")]
    UnsafeArtifact { path: PathBuf },
    #[error("artifact already exists and will not be overwritten: {path}")]
    AlreadyExists { path: PathBuf },
    #[error("artifact exceeds byte bound: limit {limit}, actual {actual}")]
    TooLarge { limit: u64, actual: u64 },
    #[error("artifact digest does not match: {path}")]
    DigestMismatch { path: PathBuf },
    #[error("artifact JSON is not RFC 8785 canonical: {path}")]
    NonCanonicalJson { path: PathBuf },
    #[error("artifact JSON cannot be decoded: {0}")]
    Json(#[from] serde_json::Error),
    #[error("artifact I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Atomically publishes canonical JSON without replacing an existing file.
///
/// # Errors
///
/// Returns [`ArtifactExchangeError`] for unsafe roots/names, serialization or
/// I/O failures, exceeded bounds, or an existing destination.
pub fn write_canonical_json<T: Serialize>(
    root: &Path,
    file_name: &str,
    value: &T,
    limits: ArtifactLimits,
) -> Result<ArtifactReceipt, ArtifactExchangeError> {
    validate_file_name(file_name)?;
    validate_root(root)?;
    let bytes = to_canonical_vec(value)?;
    let byte_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if byte_len > limits.max_bytes {
        return Err(ArtifactExchangeError::TooLarge {
            limit: limits.max_bytes,
            actual: byte_len,
        });
    }
    let destination = root.join(file_name);
    let mut temporary = NamedTempFile::new_in(root).map_err(|source| io(root, source))?;
    temporary
        .write_all(&bytes)
        .map_err(|source| io(temporary.path(), source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| io(temporary.path(), source))?;
    if let Err(error) = temporary.persist_noclobber(&destination) {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            return Err(ArtifactExchangeError::AlreadyExists { path: destination });
        }
        return Err(io(&destination, error.error));
    }
    Ok(ArtifactReceipt {
        file_name: file_name.to_owned(),
        digest: sha256_bytes(&bytes),
        byte_len,
    })
}

/// Reads one canonical, digest-bound JSON artifact from an explicit root.
///
/// # Errors
///
/// Returns [`ArtifactExchangeError`] for unsafe paths/file types, exceeded
/// bounds, noncanonical or malformed JSON, I/O failures or digest mismatch.
pub fn read_canonical_json<T: DeserializeOwned>(
    root: &Path,
    file_name: &str,
    expected_digest: Sha256Digest,
    limits: ArtifactLimits,
) -> Result<T, ArtifactExchangeError> {
    validate_file_name(file_name)?;
    validate_root(root)?;
    let path = root.join(file_name);
    let metadata = fs::symlink_metadata(&path).map_err(|source| io(&path, source))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(ArtifactExchangeError::UnsafeArtifact { path });
    }
    if metadata.len() > limits.max_bytes {
        return Err(ArtifactExchangeError::TooLarge {
            limit: limits.max_bytes,
            actual: metadata.len(),
        });
    }
    let bytes = fs::read(&path).map_err(|source| io(&path, source))?;
    let actual_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual_len > limits.max_bytes {
        return Err(ArtifactExchangeError::TooLarge {
            limit: limits.max_bytes,
            actual: actual_len,
        });
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if to_canonical_vec(&value)? != bytes {
        return Err(ArtifactExchangeError::NonCanonicalJson { path });
    }
    if sha256_bytes(&bytes) != expected_digest {
        return Err(ArtifactExchangeError::DigestMismatch { path });
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn validate_root(root: &Path) -> Result<(), ArtifactExchangeError> {
    let metadata = fs::symlink_metadata(root).map_err(|source| io(root, source))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(ArtifactExchangeError::UnsafeRoot {
            path: root.to_path_buf(),
        });
    }
    Ok(())
}

fn validate_file_name(file_name: &str) -> Result<(), ArtifactExchangeError> {
    let valid = !file_name.is_empty()
        && file_name.len() <= 128
        && Path::new(file_name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        && file_name != ".json"
        && !file_name.starts_with('.')
        && file_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !valid {
        return Err(ArtifactExchangeError::UnsafeFileName {
            name: file_name.to_owned(),
        });
    }
    Ok(())
}

fn io(path: &Path, source: std::io::Error) -> ArtifactExchangeError {
    ArtifactExchangeError::Io {
        path: path.to_path_buf(),
        source,
    }
}
