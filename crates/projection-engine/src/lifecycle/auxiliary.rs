//! Non-authoritative record indexes. No new durable fields or schemas.
use std::collections::BTreeMap;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use project_ledger_domain::{
    ActorKind, ArtifactReference, Event, EventCorrectionRecordedEvent, EventEnvelope, EventId,
    IdentityMapping, IdentityMappingStatus, NoteRecordedEvent, ScalarValue, SchemaVersion, Session,
    StableId, TrustLevel,
};

use super::{
    ProjectStateProjection, ProjectionError, invariant, refresh_record_digest, validate_record,
};

#[derive(Clone, Debug, Default)]
pub(super) struct AuxiliaryRecords {
    notes: BTreeMap<StableId, NoteRecordedEvent>,
    artifacts: BTreeMap<StableId, ArtifactReference>,
    sessions: BTreeMap<StableId, Session>,
    mappings: BTreeMap<StableId, IdentityMapping>,
    pub(super) descriptive_events: BTreeMap<EventId, Event>,
}

impl ProjectStateProjection {
    /// Returns descriptive note views; original canonical event bytes stay unchanged.
    pub fn notes(&self) -> Vec<&NoteRecordedEvent> {
        self.auxiliary.notes.values().collect()
    }

    /// Returns registered artifact references, without reading their targets.
    pub fn artifacts(&self) -> Vec<&ArtifactReference> {
        self.auxiliary.artifacts.values().collect()
    }

    /// Returns the derived lifecycle of one recorded session.
    pub fn session(&self, id: &StableId) -> Option<&Session> {
        self.auxiliary.sessions.get(id)
    }

    /// Returns confirmed and revoked mappings without inferring identity from names.
    pub fn identity_mappings(&self) -> Vec<&IdentityMapping> {
        self.auxiliary.mappings.values().collect()
    }

    pub(super) fn reduce_auxiliary(
        &mut self,
        envelope: &EventEnvelope,
    ) -> Result<(), ProjectionError> {
        require_recording(envelope)?;
        if self.state.project.archived_at.is_some() {
            return Err(invariant("an archived project rejects auxiliary mutation"));
        }
        match &envelope.event {
            Event::NoteRecorded(note) => {
                if note.kind.trim().is_empty()
                    || note.text.trim().is_empty()
                    || self.auxiliary.notes.contains_key(&note.note_id)
                    || note
                        .scope_ref
                        .as_ref()
                        .is_some_and(|scope| !self.known_subject(scope))
                {
                    return Err(invariant(
                        "note requires text, kind, unique identity and a known scope",
                    ));
                }
                self.auxiliary
                    .notes
                    .insert(note.note_id.clone(), note.clone());
            }
            Event::ArtifactReferenceAdded(added) => {
                let artifact = &added.artifact;
                if artifact.schema_version != SchemaVersion::V0_2_0
                    || artifact.name.trim().is_empty()
                    || self
                        .auxiliary
                        .artifacts
                        .contains_key(&artifact.artifact_ref_id)
                {
                    return Err(invariant(
                        "artifact identity must be new and its description nonempty",
                    ));
                }
                // Artifact digest addresses external bytes, not this reference record.
                self.auxiliary
                    .artifacts
                    .insert(artifact.artifact_ref_id.clone(), artifact.clone());
            }
            Event::SessionStarted(started) => self.start_session(&started.session, envelope)?,
            Event::SessionEnded(ended) => {
                if ended
                    .ending_checkpoint_ref
                    .as_ref()
                    .is_some_and(|id| !self.checkpoints.contains_key(id))
                {
                    return Err(invariant(
                        "session ending checkpoint is not in this project",
                    ));
                }
                let session = self
                    .auxiliary
                    .sessions
                    .get_mut(&ended.session_ref)
                    .ok_or_else(|| invariant("session does not exist"))?;
                if session.ended_at.is_some() || session.actor_ref != envelope.actor.actor_id {
                    return Err(invariant("only the session actor may end an open session"));
                }
                let started = OffsetDateTime::parse(session.started_at.as_str(), &Rfc3339)
                    .map_err(|_| invariant("invalid session start time"))?;
                let ended_instant = OffsetDateTime::parse(envelope.occurred_at.as_str(), &Rfc3339)
                    .map_err(|_| invariant("invalid session end time"))?;
                if ended_instant < started {
                    return Err(invariant("session cannot end before it started"));
                }
                session.ended_at = Some(envelope.occurred_at.clone());
                session
                    .ending_checkpoint_ref
                    .clone_from(&ended.ending_checkpoint_ref);
            }
            Event::ExternalMappingConfirmed(confirmed) => {
                self.confirm_mapping(&confirmed.mapping, envelope)?;
            }
            Event::ExternalMappingRevoked(revoked) => {
                require_local_human(envelope)?;
                if revoked.reason.trim().is_empty() {
                    return Err(invariant("mapping revocation requires a reason"));
                }
                let mapping = self
                    .auxiliary
                    .mappings
                    .get_mut(&revoked.mapping_ref)
                    .ok_or_else(|| invariant("mapping does not exist"))?;
                if mapping.status != IdentityMappingStatus::Confirmed {
                    return Err(invariant("mapping is not confirmed"));
                }
                mapping.status = IdentityMappingStatus::Revoked;
                mapping.updated_at.clone_from(&envelope.occurred_at);
                refresh_record_digest(mapping, |value, digest| value.digest = digest)?;
            }
            Event::EventCorrectionRecorded(correction) => {
                require_local_human(envelope)?;
                self.correct_description(correction)?;
            }
            _ => return Err(ProjectionError::UnsupportedEvent),
        }
        Ok(())
    }

    fn known_subject(&self, id: &StableId) -> bool {
        id.as_str() == self.state.project.project_id.to_string()
            || self.phase_statuses.contains_key(id)
            || self.gates.contains_key(id)
            || self.authority_document(id).is_some()
            || self.decisions.contains_key(id)
            || self.blockers.contains_key(id)
            || self.risks.contains_key(id)
            || self.next_actions.contains_key(id)
            || self.claims.contains_key(id)
            || self.evidence.contains_key(id)
            || self.unknowns.contains_key(id)
            || self.repository_bindings.contains_key(id)
            || self.work_records.contains_key(id)
    }

    fn start_session(
        &mut self,
        session: &Session,
        envelope: &EventEnvelope,
    ) -> Result<(), ProjectionError> {
        if session.schema_version != SchemaVersion::V0_2_0
            || session.host.trim().is_empty()
            || session.actor_ref != envelope.actor.actor_id
            || session.ended_at.is_some()
            || session.ending_checkpoint_ref.is_some()
            || session.started_at != envelope.occurred_at
            || self.auxiliary.sessions.contains_key(&session.session_id)
            || session
                .starting_checkpoint_ref
                .as_ref()
                .is_some_and(|id| !self.checkpoints.contains_key(id))
        {
            return Err(invariant(
                "session initial state, actor or checkpoint is invalid",
            ));
        }
        self.auxiliary
            .sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    fn confirm_mapping(
        &mut self,
        mapping: &IdentityMapping,
        envelope: &EventEnvelope,
    ) -> Result<(), ProjectionError> {
        require_local_human(envelope)?;
        validate_record(mapping, mapping.digest)?;
        let project = &self.state.project.project_id;
        let local_is_known = mapping.local_ref.as_str()
            == format!("project-ledger://project/{}", project.uuid())
            || mapping.local_ref.as_str()
                == format!("project-ledger://projects/{}", project.uuid())
            || mapping.local_ref.as_str() == project.to_string()
            || StableId::parse(mapping.local_ref.as_str()).is_ok_and(|id| self.known_subject(&id));
        if mapping.schema_version != SchemaVersion::V0_2_0
            || mapping.external_system.trim().is_empty()
            || mapping.status != IdentityMappingStatus::Confirmed
            || mapping.confirmed_by.as_ref() != Some(&envelope.actor.actor_id)
            || !local_is_known
            || self.auxiliary.mappings.contains_key(&mapping.mapping_id)
            || self.auxiliary.mappings.values().any(|prior| {
                prior.status == IdentityMappingStatus::Confirmed
                    && prior.external_system == mapping.external_system
                    && (prior.external_ref == mapping.external_ref
                        || prior.local_ref == mapping.local_ref)
            })
        {
            return Err(invariant(
                "mapping requires a unique, explicit, human-confirmed local identity",
            ));
        }
        self.auxiliary
            .mappings
            .insert(mapping.mapping_id.clone(), mapping.clone());
        Ok(())
    }

    fn correct_description(
        &mut self,
        correction: &EventCorrectionRecordedEvent,
    ) -> Result<(), ProjectionError> {
        if correction.reason.trim().is_empty() || correction.replacement_field_values.len() != 1 {
            return Err(invariant(
                "correction requires a reason and one allowlisted descriptive scalar",
            ));
        }
        let original = self
            .auxiliary
            .descriptive_events
            .get(&correction.corrects_event_ref)
            .ok_or_else(|| {
                invariant("correction target is absent or not descriptively correctable")
            })?;
        let field = &correction.replacement_field_values[0];
        let ScalarValue::String(text) = &field.replacement else {
            return Err(invariant("descriptive correction requires a string"));
        };
        if text.trim().is_empty() {
            return Err(invariant("description cannot be empty"));
        }
        // Closed allowlist: note text and work summary only. No identity, trust,
        // status, evidence predicate, timestamp, reference or digest patching.
        match original {
            Event::NoteRecorded(note) if field.field_path == "text" => {
                let note = self
                    .auxiliary
                    .notes
                    .get_mut(&note.note_id)
                    .ok_or_else(|| invariant("corrected note is absent"))?;
                note.text.clone_from(text);
            }
            Event::WorkProgressRecorded(work) if field.field_path == "work_record.summary" => {
                let record = self
                    .work_records
                    .get_mut(&work.work_record.work_record_id)
                    .ok_or_else(|| invariant("corrected work record is absent"))?;
                record.summary.clone_from(text);
                refresh_record_digest(record, |value, digest| value.digest = digest)?;
            }
            _ => {
                return Err(invariant(
                    "correction field is not allowlisted for this event",
                ));
            }
        }
        Ok(())
    }
}

fn require_recording(event: &EventEnvelope) -> Result<(), ProjectionError> {
    if !matches!(
        event.actor.trust_level,
        TrustLevel::LocalUser | TrustLevel::TrustedForRecording
    ) {
        return Err(invariant(
            "auxiliary facts require explicit recording trust",
        ));
    }
    Ok(())
}

fn require_local_human(event: &EventEnvelope) -> Result<(), ProjectionError> {
    if event.actor.actor_kind != ActorKind::Human
        || event.actor.trust_level != TrustLevel::LocalUser
    {
        return Err(invariant(
            "identity confirmation and corrections require a local human",
        ));
    }
    Ok(())
}
