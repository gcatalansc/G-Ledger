use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{LedgerTimestamp, ResourceRef, StableId};

macro_rules! closed_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name { $($variant),+ }
    };
}

pub(crate) use closed_enum;

/// Supported durable contract versions in the first contract slice.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SchemaVersion {
    // The default derived schemas are the frozen v0.2 catalog. The extension
    // has an explicit, separate catalog in project-ledger-schemas.
    #[schemars(skip)]
    #[serde(rename = "0.3.0")]
    V0_3_0,
    #[serde(rename = "0.2.0")]
    V0_2_0,
    #[serde(rename = "0.1.1")]
    V0_1_1,
}

closed_enum! {
    /// Actor categories carried in commands, events, and audit records.
    pub enum ActorKind { Human, Agent, System, Integration }
}

closed_enum! {
    /// Explicit trust level; read and write grants remain distinct.
    pub enum TrustLevel {
        Untrusted,
        LocalUser,
        TrustedProject,
        TrustedForProjection,
        TrustedForRecording,
        Revoked
    }
}

/// Minimal actor identity with no implicit authority.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub actor_id: StableId,
    pub actor_kind: ActorKind,
    pub trust_level: TrustLevel,
    pub display_name: Option<String>,
}

/// A versioned engine or parser that produced a derived result.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub engine: String,
    pub version: String,
}

/// A reproducible repository snapshot reference.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositorySnapshot {
    pub repository_ref: StableId,
    pub snapshot_ref: StableId,
    pub snapshot_digest: project_ledger_canonical_json::Sha256Digest,
}

/// Common provenance anchor for imported or derived data.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub provenance_ref: ResourceRef,
    pub source_system: String,
    pub source_version: Option<String>,
    pub observed_at: LedgerTimestamp,
}
