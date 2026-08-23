use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::DomainError;

/// A validated RFC 3339 timestamp with an explicit UTC offset.
#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd)]
#[schemars(transparent)]
pub struct LedgerTimestamp(String);

impl LedgerTimestamp {
    /// Parses and normalizes an RFC 3339 timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidTimestamp`] when the input is not a valid
    /// RFC 3339 timestamp or cannot be rendered in its normalized form.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        let parsed =
            OffsetDateTime::parse(value, &Rfc3339).map_err(|_| DomainError::InvalidTimestamp)?;
        let normalized = parsed
            .format(&Rfc3339)
            .map_err(|_| DomainError::InvalidTimestamp)?;
        Ok(Self(normalized))
    }

    /// Returns the canonical RFC 3339 text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LedgerTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for LedgerTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LedgerTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
