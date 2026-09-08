//! Read-only reconstruction over a verified canonical stream.
use crate::{ProjectStateProjection, ProjectionError};
use project_ledger_domain::{EpisodeFact, Event, EventEnvelope, StableId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn invalid(message: impl Into<String>) -> ProjectionError {
    ProjectionError::LifecycleInvariant {
        message: message.into(),
    }
}

/// Exact historical cut; no state cache or canonical writes.
#[derive(Debug)]
pub struct HistoryView<'a> {
    events: &'a [EventEnvelope],
    projection: ProjectStateProjection,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TraceCursor {
    project_ref: String,
    decision_ref: String,
    version: u64,
    head_digest: String,
    offset: usize,
}

impl<'a> HistoryView<'a> {
    /// Rebuild at an atomic command boundary, never at an intermediate event.
    /// # Errors
    /// Rejects absent versions, incomplete commands and invalid streams.
    pub fn at(events: &'a [EventEnvelope], version: Option<u64>) -> Result<Self, ProjectionError> {
        let n = version.map_or(events.len(), |v| usize::try_from(v).unwrap_or(usize::MAX));
        if n == 0 || n > events.len() {
            return Err(invalid("history version is outside the recorded stream"));
        }
        if n < events.len()
            && events[n - 1].causation.command_ref == events[n].causation.command_ref
        {
            let command = &events[n].causation.command_ref;
            let before = events[..n]
                .iter()
                .rposition(|e| &e.causation.command_ref != command)
                .map_or(0, |i| i + 1);
            let after = n + events[n..]
                .iter()
                .take_while(|e| &e.causation.command_ref == command)
                .count();
            return Err(invalid(format!(
                "incomplete command cut; valid boundaries are {before} and {after}"
            )));
        }
        let selected = &events[..n];
        Ok(Self {
            events: selected,
            projection: ProjectStateProjection::rebuild(selected)?,
        })
    }

    fn response(&self, kind: &str, body: Value) -> Value {
        let mut value = json!({"schema_version":"0.3.0","kind":kind,
            "project_ref":self.projection.state().project.project_id,
            "state_version":self.projection.state().state_version,
            "state_digest":self.projection.state().state_digest,
            "head_digest":self.projection.head_digest(),"read_only":true});
        if let (Some(target), Value::Object(fields)) = (value.as_object_mut(), body) {
            target.extend(fields);
        }
        value
    }

    /// State plus explicit episode context at this cut.
    pub fn state(&self) -> Value {
        self.response(
            "STATE",
            json!({"state":self.projection.state(), "episodes":self.projection.episodes()}),
        )
    }

    /// Retrieve full data without implicit file or network access.
    /// # Errors
    /// Rejects unknown references and invalid IDs.
    pub fn get(&self, reference: &str) -> Result<Value, ProjectionError> {
        if let Some(event) = self
            .events
            .iter()
            .find(|e| e.event_id.as_str() == reference)
        {
            return Ok(self.response("RECORD", json!({"record":event})));
        }
        let id = StableId::parse(reference).map_err(|error| invalid(error.to_string()))?;
        for episode in self.projection.episodes() {
            if episode.episode_ref == id {
                return Ok(self.response("RECORD", json!({"record":episode})));
            }
            if let Some(entry) = episode
                .entries
                .iter()
                .find(|entry| entry.entry.entry_id == id)
            {
                return Ok(self.response("RECORD", json!({"record":entry})));
            }
        }
        let record = if let Some(record) = self.projection.decision(&id) {
            json!(record)
        } else if let Some(record) = self.projection.evidence(&id) {
            json!(record)
        } else if let Some(record) = self.projection.claim(&id) {
            json!(record)
        } else if let Some(record) = self.projection.checkpoint(&id) {
            json!(record)
        } else if let Some(record) = self.projection.gate(&id) {
            json!({"definition":record,"status":self.projection.gate_status(&id)})
        } else if let Some(record) = self.projection.work_record(&id) {
            json!(record)
        } else if let Some(record) = self.projection.authority_document(&id) {
            json!(record)
        } else if let Some(record) = self.projection.repository_binding(&id) {
            json!(record)
        } else if let Some(record) = self.projection.unknown(&id) {
            json!(record)
        } else if let Some(record) = self.projection.blocker(&id) {
            json!(record)
        } else if let Some(record) = self.projection.risk(&id) {
            json!(record)
        } else if let Some(record) = self.projection.next_action(&id) {
            json!(record)
        } else if let Some(record) = self.projection.conflict(&id) {
            json!(record)
        } else if let Some(record) = self.projection.gate_evaluation(&id) {
            json!(record)
        } else {
            return Err(invalid(format!(
                "reference not recorded at this cut: {reference}"
            )));
        };
        let source_event_refs: Vec<_> = self
            .events
            .iter()
            .filter(|event| event_mentions(event, &id))
            .map(|e| &e.event_id)
            .collect();
        Ok(self.response(
            "RECORD",
            json!({"record":record,"source_event_refs":source_event_refs}),
        ))
    }

    /// Differences have explicit before/after values and canonical triggers.
    /// # Errors
    /// Rejects reverse or cross-project comparisons.
    pub fn diff(&self, later: &Self) -> Result<Value, ProjectionError> {
        if self.events.len() > later.events.len()
            || self.projection.state().project.project_id
                != later.projection.state().project.project_id
            || later.events[self.events.len() - 1].event_digest != self.projection.head_digest()
        {
            return Err(invalid(
                "history diff requires ordered cuts of the same project",
            ));
        }
        let before = json!({"state":self.projection.state(),"episodes":self.projection.episodes()});
        let after =
            json!({"state":later.projection.state(),"episodes":later.projection.episodes()});
        let mut changes = Vec::new();
        differences("", &before, &after, &mut changes);
        Ok(later.response("DIFF", json!({"from_version":self.events.len(),"from_head_digest":self.projection.head_digest(),
            "changes":changes,"events":&later.events[self.events.len()..]})))
    }

    /// Trace one decision family, preserving recorded gaps and evidence status.
    /// # Errors
    /// Rejects unknown decisions or a cursor bound to another query/cut.
    pub fn trace(
        events: &'a [EventEnvelope],
        reference: &str,
        version: Option<u64>,
        cursor: Option<&str>,
    ) -> Result<Value, ProjectionError> {
        let parsed: Option<TraceCursor> = cursor
            .map(serde_json::from_str)
            .transpose()
            .map_err(|e| invalid(format!("invalid history cursor: {e}")))?;
        if let Some(c) = &parsed
            && (c.decision_ref != reference || version.is_some_and(|v| v != c.version))
        {
            return Err(invalid("cursor does not match trace query"));
        }
        let view = Self::at(events, parsed.as_ref().map(|c| c.version).or(version))?;
        let id = StableId::parse(reference).map_err(|e| invalid(e.to_string()))?;
        if view.projection.decision(&id).is_none() {
            return Err(invalid("decision not recorded at this cut"));
        }
        let family = decision_family(view.events, &id);
        let decisions: Vec<_> = family
            .iter()
            .filter_map(|id| view.projection.decision(id))
            .collect();
        let episodes: Vec<_> = view.projection.episodes().into_iter().filter(|episode| episode.entries.iter().any(|entry|
            matches!(&entry.entry.fact, EpisodeFact::DecisionLink { decision_ref, .. } if family.contains(decision_ref)))).collect();
        let mut evidence_refs: BTreeSet<_> = decisions
            .iter()
            .flat_map(|d| d.evidence_refs.iter().cloned())
            .collect();
        for episode in &episodes {
            for record in &episode.entries {
                if let EpisodeFact::Attempt {
                    evidence_refs: refs,
                    ..
                } = &record.entry.fact
                {
                    evidence_refs.extend(refs.iter().cloned());
                }
            }
        }
        let evidence: Vec<_> = evidence_refs
            .iter()
            .map(|id| json!({"evidence_ref":id,"record":view.projection.evidence(id)}))
            .collect();
        let mut timeline = Vec::new();
        for event in view.events {
            if family
                .iter()
                .chain(evidence_refs.iter())
                .any(|id| event_mentions(event, id))
            {
                timeline.push(json!({"stream_version":event.stream_version,"entry_ref":event.event_id,"event":event}));
            }
        }
        for episode in &episodes {
            for entry in &episode.entries {
                timeline.push(json!({"stream_version":entry.stream_version,"entry_ref":entry.entry.entry_id,"record":entry}));
            }
        }
        timeline.sort_by(|a, b| {
            (a["stream_version"].as_u64(), a["entry_ref"].as_str())
                .cmp(&(b["stream_version"].as_u64(), b["entry_ref"].as_str()))
        });
        let offset = parsed.as_ref().map_or(0, |c| c.offset);
        if let Some(c) = &parsed
            && (c.project_ref != view.projection.state().project.project_id.to_string()
                || c.head_digest != view.projection.head_digest().to_string()
                || offset > timeline.len()
                || !offset.is_multiple_of(50))
        {
            return Err(invalid(
                "cursor does not match project, digest or page boundary",
            ));
        }
        let end = offset.saturating_add(50).min(timeline.len());
        let next_cursor = if end < timeline.len() {
            Some(
                serde_json::to_string(&TraceCursor {
                    project_ref: view.projection.state().project.project_id.to_string(),
                    decision_ref: reference.to_owned(),
                    version: view.projection.state().state_version,
                    head_digest: view.projection.head_digest().to_string(),
                    offset: end,
                })
                .map_err(|e| invalid(e.to_string()))?,
            )
        } else {
            None
        };
        let missing: Vec<_> = decisions.iter().filter(|decision| !episodes.iter().any(|episode| episode.entries.iter().any(|record|
            matches!(&record.entry.fact, EpisodeFact::DecisionLink { decision_ref, .. } if *decision_ref == decision.decision_id))))
            .map(|d| json!({"subject_ref":d.decision_id,"reason":"NO_STRUCTURED_EPISODE_RECORDED","legacy_alternatives":d.alternatives})).collect();
        Ok(view.response("TRACE", json!({"decision_ref":id,"decisions":decisions,"evidence":evidence,
            "episode_refs":episodes.iter().map(|e| &e.episode_ref).collect::<Vec<_>>(), "missing":missing,
            "entries":&timeline[offset..end],"total_entries":timeline.len(),"next_cursor":next_cursor})))
    }
}

fn decision_family(events: &[EventEnvelope], id: &StableId) -> BTreeSet<StableId> {
    let mut family = BTreeSet::from([id.clone()]);
    loop {
        let before = family.len();
        for event in events {
            let edge = match &event.event {
                Event::DecisionSuperseded(e) => Some((&e.decision_ref, &e.replacement_ref)),
                Event::DecisionProposed(e) => e
                    .decision
                    .supersedes_ref
                    .as_ref()
                    .map(|previous| (previous, &e.decision.decision_id)),
                _ => None,
            };
            if let Some((a, b)) = edge
                && (family.contains(a) || family.contains(b))
            {
                family.insert(a.clone());
                family.insert(b.clone());
            }
        }
        if family.len() == before {
            return family;
        }
    }
}

fn event_mentions(event: &EventEnvelope, id: &StableId) -> bool {
    match &event.event {
        Event::DecisionProposed(e) => e.decision.decision_id == *id,
        Event::DecisionAccepted(e) => e.decision_ref == *id,
        Event::DecisionSuperseded(e) => e.decision_ref == *id || e.replacement_ref == *id,
        Event::DecisionRejected(e) => e.decision_ref == *id,
        Event::DecisionRevalidationRequired(e) => e.decision_ref == *id,
        Event::VerificationRecorded(e) => e.evidence.evidence_id == *id,
        Event::EvidenceInvalidated(e) => e.evidence_ref == *id,
        Event::ExternalEvidenceLinked(e) => e.evidence_ref == *id,
        Event::ExternalEvidenceUnlinked(e) => e.evidence_ref == *id,
        Event::WorkProgressRecorded(e) => e.work_record.work_record_id == *id,
        Event::ClaimRecorded(e) => e.claim.claim_id == *id,
        Event::ClaimStatusChanged(e) => e.claim_ref == *id,
        Event::CheckpointCreated(e) => e.checkpoint.checkpoint_id == *id,
        Event::CheckpointInvalidated(e) => e.checkpoint_ref == *id,
        _ => false,
    }
}

fn differences(path: &str, before: &Value, after: &Value, changes: &mut Vec<Value>) {
    if before == after {
        return;
    }
    if let (Some(a), Some(b)) = (before.as_object(), after.as_object()) {
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for key in keys {
            differences(
                &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                &before[key],
                &after[key],
                changes,
            );
        }
    } else {
        changes.push(json!({"path":path,"before":before,"after":after}));
    }
}
