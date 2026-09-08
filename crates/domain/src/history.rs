//! Explicit, append-only development context. These contracts belong to v0.3.
use crate::{EventId, LedgerTimestamp, StableId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeUpdate {
    pub episode_ref: StableId,
    pub objective: Option<String>,
    pub summary: String,
    #[schemars(length(max = 200))]
    pub entries: Vec<EpisodeEntry>,
}

#[derive(Clone, Debug, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeEntry {
    pub entry_id: StableId,
    pub corrects_ref: Option<StableId>,
    #[serde(flatten)]
    pub fact: EpisodeFact,
}

// Serde's internally tagged enum buffers fields when flattened, which prevents
// the outer deny_unknown_fields visitor from consuming them. Remove only the
// common fields, then let the closed variant validate every remaining field.
impl<'de> Deserialize<'de> for EpisodeEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("entry must be an object"))?;
        let entry_id = serde_json::from_value(
            object
                .remove("entry_id")
                .ok_or_else(|| D::Error::missing_field("entry_id"))?,
        )
        .map_err(D::Error::custom)?;
        let corrects_ref = serde_json::from_value(
            object
                .remove("corrects_ref")
                .unwrap_or(serde_json::Value::Null),
        )
        .map_err(D::Error::custom)?;
        let fact = serde_json::from_value(value).map_err(D::Error::custom)?;
        Ok(Self {
            entry_id,
            corrects_ref,
            fact,
        })
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum EpisodeFact {
    Alternative {
        description: String,
        rationale: String,
    },
    Attempt {
        alternative_ref: StableId,
        description: String,
        outcome: EpisodeOutcome,
        evidence_refs: Vec<StableId>,
    },
    DecisionLink {
        decision_ref: StableId,
        alternative_ref: StableId,
        rationale: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpisodeOutcome {
    Succeeded,
    Failed,
    Inconclusive,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeEntryRecord {
    pub episode_ref: StableId,
    pub entry: EpisodeEntry,
    pub event_ref: EventId,
    pub stream_version: u64,
    pub actor_ref: StableId,
    pub occurred_at: LedgerTimestamp,
    pub recorded_at: LedgerTimestamp,
    pub superseded_by: Option<StableId>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeProjection {
    pub episode_ref: StableId,
    pub objective: String,
    pub latest_summary: String,
    pub update_event_refs: Vec<EventId>,
    pub entries: Vec<EpisodeEntryRecord>,
}
