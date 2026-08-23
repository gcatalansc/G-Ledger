use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::DomainError;

macro_rules! uuid_v7_id {
    ($name:ident, $prefix:literal, $kind:literal) => {
        #[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd)]
        #[schemars(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new `UUIDv7` identifier with its canonical resource prefix.
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, Uuid::now_v7()))
            }

            /// Parses and validates a prefixed `UUIDv7` identifier.
            ///
            /// # Errors
            ///
            /// Returns [`DomainError::InvalidIdentifier`] when the prefix, UUID syntax,
            /// UUID version, or canonical textual representation is invalid.
            pub fn parse(value: &str) -> Result<Self, DomainError> {
                let raw = value
                    .strip_prefix($prefix)
                    .ok_or(DomainError::InvalidIdentifier { kind: $kind })?;
                let uuid = Uuid::parse_str(raw)
                    .map_err(|_| DomainError::InvalidIdentifier { kind: $kind })?;
                if uuid.get_version_num() != 7 || value != format!("{}{}", $prefix, uuid) {
                    return Err(DomainError::InvalidIdentifier { kind: $kind });
                }
                Ok(Self(value.to_owned()))
            }

            /// Returns the canonical prefixed representation.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Returns the validated `UUIDv7` component.
            ///
            /// # Panics
            ///
            /// This method cannot panic for values created through the public API;
            /// construction and deserialization both validate the UUID component.
            pub fn uuid(&self) -> Uuid {
                Uuid::parse_str(&self.0[$prefix.len()..])
                    .expect("validated identifier always contains UUIDv7")
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

uuid_v7_id!(EventId, "event:", "event");
uuid_v7_id!(CommandId, "command:", "command");
uuid_v7_id!(ProjectId, "project:", "project");

/// An absolute, minimized resource reference.
#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd)]
#[schemars(transparent)]
pub struct ResourceRef(String);

impl ResourceRef {
    /// Validates an absolute URI-like reference.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidReference`] when the reference lacks a valid
    /// scheme, contains whitespace or path traversal, or has an empty remainder.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        let valid = value.split_once(':').is_some_and(|(scheme, remainder)| {
            let mut characters = scheme.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && characters.all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
                })
                && !remainder.is_empty()
                && !value.chars().any(char::is_whitespace)
                && !value.contains("../")
        });
        if !valid {
            return Err(DomainError::InvalidReference);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical reference text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ResourceRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ResourceRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// A compact stable identifier for phases, gates, decisions, and related records.
#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd)]
#[schemars(transparent)]
pub struct StableId(String);

impl StableId {
    /// Validates a bounded identifier without whitespace or path traversal.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidIdentifier`] when the identifier is empty,
    /// too long, contains unsupported characters, or resembles path traversal.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        let valid = !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
            && !value.starts_with('.')
            && !value.contains("..");
        if !valid {
            return Err(DomainError::InvalidIdentifier { kind: "stable" });
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the validated identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for StableId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for StableId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
