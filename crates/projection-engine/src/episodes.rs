use crate::ProjectionError;
use project_ledger_domain::{
    EpisodeEntryRecord, EpisodeFact, EpisodeProjection, EpisodeUpdate, EventEnvelope, StableId,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct EpisodeIndex {
    episodes: BTreeMap<StableId, EpisodeProjection>,
}

fn invalid(message: &str) -> ProjectionError {
    ProjectionError::LifecycleInvariant {
        message: message.to_owned(),
    }
}

impl EpisodeIndex {
    pub(crate) fn all(&self) -> Vec<EpisodeProjection> {
        self.episodes.values().cloned().collect()
    }
    pub(crate) fn record(&self, id: &StableId) -> Option<&EpisodeEntryRecord> {
        self.episodes
            .values()
            .flat_map(|episode| &episode.entries)
            .find(|record| record.entry.entry_id == *id)
    }
    fn validate(
        &self,
        update: &EpisodeUpdate,
        decision_exists: impl Fn(&StableId) -> bool,
        evidence_exists: impl Fn(&StableId) -> bool,
    ) -> Result<String, ProjectionError> {
        if !update.episode_ref.as_str().starts_with("episode:") || update.summary.trim().is_empty()
        {
            return Err(invalid(
                "episode ID must use episode: and summary must be nonempty",
            ));
        }
        if update.entries.len() > 200 {
            return Err(invalid("an episode update accepts at most 200 entries"));
        }
        let objective = self.objective(update)?;
        let mut additions = BTreeMap::new();
        for entry in &update.entries {
            if self.record(&entry.entry_id).is_some()
                || additions.insert(entry.entry_id.clone(), entry).is_some()
            {
                return Err(invalid("episode entry ID already exists"));
            }
        }
        let mut corrected = std::collections::BTreeSet::new();
        for entry in &update.entries {
            let prefix = match &entry.fact {
                EpisodeFact::Alternative {
                    description,
                    rationale,
                } => {
                    if description.trim().is_empty() || rationale.trim().is_empty() {
                        return Err(invalid("alternative requires description and rationale"));
                    }
                    "alternative:"
                }
                EpisodeFact::Attempt {
                    description,
                    evidence_refs,
                    ..
                } => {
                    if description.trim().is_empty()
                        || evidence_refs.iter().any(|id| !evidence_exists(id))
                    {
                        return Err(invalid(
                            "attempt requires description and existing project evidence",
                        ));
                    }
                    "attempt:"
                }
                EpisodeFact::DecisionLink {
                    decision_ref,
                    rationale,
                    ..
                } => {
                    if rationale.trim().is_empty() || !decision_exists(decision_ref) {
                        return Err(invalid(
                            "link requires rationale and existing project decision",
                        ));
                    }
                    "link:"
                }
            };
            if !entry.entry_id.as_str().starts_with(prefix) {
                return Err(invalid("entry ID prefix does not match its kind"));
            }
            if let Some(previous) = &entry.corrects_ref {
                let old = self
                    .record(previous)
                    .ok_or_else(|| invalid("correction target does not exist"))?;
                if old.episode_ref != update.episode_ref
                    || old.superseded_by.is_some()
                    || std::mem::discriminant(&old.entry.fact)
                        != std::mem::discriminant(&entry.fact)
                    || !corrected.insert(previous.clone())
                {
                    return Err(invalid(
                        "correction must replace one active entry of the same episode and kind",
                    ));
                }
            }
            let alternative = match &entry.fact {
                EpisodeFact::Alternative { .. } => None,
                EpisodeFact::Attempt {
                    alternative_ref, ..
                }
                | EpisodeFact::DecisionLink {
                    alternative_ref, ..
                } => Some(alternative_ref),
            };
            if let Some(id) = alternative {
                let valid = additions
                    .get(id)
                    .is_some_and(|entry| matches!(entry.fact, EpisodeFact::Alternative { .. }))
                    || self.record(id).is_some_and(|record| {
                        record.episode_ref == update.episode_ref
                            && matches!(record.entry.fact, EpisodeFact::Alternative { .. })
                    });
                if !valid {
                    return Err(invalid(
                        "alternative reference must identify an alternative in this episode",
                    ));
                }
            }
        }
        Ok(objective)
    }

    fn objective(&self, update: &EpisodeUpdate) -> Result<String, ProjectionError> {
        let objective = self
            .episodes
            .get(&update.episode_ref)
            .map(|episode| episode.objective.as_str())
            .or(update.objective.as_deref())
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| invalid("new episode requires an objective"))?
            .to_owned();
        if update
            .objective
            .as_ref()
            .is_some_and(|text| *text != objective)
        {
            return Err(invalid("the episode objective is immutable"));
        }
        Ok(objective)
    }

    pub(crate) fn apply(
        &mut self,
        update: &EpisodeUpdate,
        event: &EventEnvelope,
        decision_exists: impl Fn(&StableId) -> bool,
        evidence_exists: impl Fn(&StableId) -> bool,
    ) -> Result<(), ProjectionError> {
        let objective = self.validate(update, decision_exists, evidence_exists)?;
        let episode = self
            .episodes
            .entry(update.episode_ref.clone())
            .or_insert_with(|| EpisodeProjection {
                episode_ref: update.episode_ref.clone(),
                objective,
                latest_summary: String::new(),
                update_event_refs: Vec::new(),
                entries: Vec::new(),
            });
        episode.latest_summary.clone_from(&update.summary);
        episode.update_event_refs.push(event.event_id.clone());
        for entry in &update.entries {
            if let Some(old) = &entry.corrects_ref
                && let Some(record) = episode
                    .entries
                    .iter_mut()
                    .find(|record| record.entry.entry_id == *old)
            {
                record.superseded_by = Some(entry.entry_id.clone());
            }
            episode.entries.push(EpisodeEntryRecord {
                episode_ref: update.episode_ref.clone(),
                entry: entry.clone(),
                event_ref: event.event_id.clone(),
                stream_version: event.stream_version,
                actor_ref: event.actor.actor_id.clone(),
                occurred_at: event.occurred_at.clone(),
                recorded_at: event.recorded_at.clone(),
                superseded_by: None,
            });
        }
        episode.entries.sort_by(|a, b| {
            (a.stream_version, &a.entry.entry_id).cmp(&(b.stream_version, &b.entry.entry_id))
        });
        Ok(())
    }
}
