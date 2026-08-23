//! Deterministic JSON and SHA-256 primitives for Project Ledger contracts.

use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors produced while canonicalizing JSON or parsing a digest.
#[derive(Debug, Error)]
pub enum CanonicalJsonError {
    /// The input could not be serialized as RFC 8785 JSON.
    #[error("canonical JSON serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The digest text was not `sha256:` followed by 64 lowercase hexadecimal characters.
    #[error("invalid canonical SHA-256 digest")]
    InvalidDigest,
}

/// A canonical lowercase SHA-256 digest with the `sha256:` prefix.
#[derive(Clone, Copy, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd)]
#[schemars(with = "String")]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    /// Parses the canonical textual representation.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalJsonError::InvalidDigest`] when the value is not
    /// `sha256:` followed by exactly 64 lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, CanonicalJsonError> {
        value.parse()
    }

    /// Returns the raw 32-byte digest.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for Sha256Digest {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl FromStr for Sha256Digest {
    type Err = CanonicalJsonError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let hex = value
            .strip_prefix("sha256:")
            .ok_or(CanonicalJsonError::InvalidDigest)?;
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CanonicalJsonError::InvalidDigest);
        }

        let mut bytes = [0_u8; 32];
        for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_value(chunk[0])? << 4) | hex_value(chunk[1])?;
        }
        Ok(Self(bytes))
    }
}

fn hex_value(value: u8) -> Result<u8, CanonicalJsonError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(CanonicalJsonError::InvalidDigest),
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("sha256:")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Sha256Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Serializes a value according to RFC 8785 JSON Canonicalization Scheme.
///
/// # Errors
///
/// Returns [`CanonicalJsonError::Serialization`] when `value` cannot be
/// represented as canonical JSON.
pub fn to_canonical_vec<T>(value: &T) -> Result<Vec<u8>, CanonicalJsonError>
where
    T: Serialize,
{
    serde_json_canonicalizer::to_vec(value).map_err(CanonicalJsonError::from)
}

/// Computes SHA-256 over the RFC 8785 canonical bytes of a value.
///
/// # Errors
///
/// Returns [`CanonicalJsonError::Serialization`] when `value` cannot be
/// represented as canonical JSON.
pub fn sha256_digest<T>(value: &T) -> Result<Sha256Digest, CanonicalJsonError>
where
    T: Serialize,
{
    let canonical = to_canonical_vec(value)?;
    let bytes: [u8; 32] = Sha256::digest(canonical).into();
    Ok(Sha256Digest::from(bytes))
}

/// Computes SHA-256 over an exact byte sequence without JSON transformation.
pub fn sha256_bytes(bytes: &[u8]) -> Sha256Digest {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    Sha256Digest::from(digest)
}
