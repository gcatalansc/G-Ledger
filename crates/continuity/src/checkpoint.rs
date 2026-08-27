use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    ArtifactReferenceAddedEvent, CheckpointCreatedEvent, CheckpointFreshnessStatus,
    CheckpointInvalidatedEvent, Command, CommandEnvelope, ContinuityCheckpoint, Event,
    EventCorrectionRecordedEvent, EventEnvelope, EventId, ExternalMappingConfirmedEvent,
    ExternalMappingRevokedEvent, GateCriticality, GateStatus, NoteRecordedEvent,
    RepositoryBindingStatus, RepositorySnapshot, SchemaVersion, SessionEndedEvent,
    SessionStartedEvent, StableId, UnknownRecordedEvent, UnknownResolvedEvent,
    WorkProgressRecordedEvent,
};
use project_ledger_event_store::{EventDraft, IntegrityError, build_event};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectionError, validate_state_integrity,
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Deterministic S6 command/freshness owner.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContinuityEngine;

/// Fail-closed continuity command error.
#[derive(Debug, Error)]
pub enum ContinuityError {
    #[error("invalid continuity command: {message}")]
    InvalidCommand {
        message: String,
        subject_refs: Vec<StableId>,
    },
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("closed contract encoding failed: {0}")]
    ContractEncoding(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("could not derive deterministic continuity identity")]
    Identity,
}

impl ContinuityError {
    pub fn subject_refs(&self) -> Vec<StableId> {
        match self {
            Self::InvalidCommand { subject_refs, .. } => subject_refs.clone(),
            Self::Projection(_)
            | Self::Integrity(_)
            | Self::ContractEncoding(_)
            | Self::Canonical(_)
            | Self::Identity => Vec::new(),
        }
    }
}

impl ContinuityEngine {
    /// Handles S6 continuity facts against the exact verified stream.
    ///
    /// # Errors
    ///
    /// Rejects version/project mismatch, malformed facts, ambiguous repository
    /// selection and commands owned by another engine.
    #[allow(clippy::too_many_lines)] // Closed variants share the same checked append boundary.
    pub fn handle(
        command: &CommandEnvelope,
        stream: &[EventEnvelope],
    ) -> Result<Vec<EventDraft>, ContinuityError> {
        let mut projection = checked_projection(command, stream)?;
        let mut drafts = Vec::new();
        match &command.command {
            Command::RecordNote(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::NoteRecorded(NoteRecordedEvent {
                    note_id: stable_command_id("note", command)?,
                    kind: payload.kind.clone(),
                    text: payload.text.clone(),
                    scope_ref: payload.scope_ref.clone(),
                }),
            )?,
            Command::AddArtifactReference(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ArtifactReferenceAdded(ArtifactReferenceAddedEvent {
                    artifact: payload.artifact.clone(),
                }),
            )?,
            Command::StartSession(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::SessionStarted(SessionStartedEvent {
                    session: payload.session.clone(),
                }),
            )?,
            Command::EndSession(payload) => {
                // Create/validate both drafts before persistence. The checkpoint
                // precedes the end event so its reference is already resolvable.
                let checkpoint_ref = if payload.create_checkpoint {
                    let event_id = deterministic_event_id(command, drafts.len())?;
                    let checkpoint = build_checkpoint(command, &projection, event_id.clone())?;
                    let checkpoint_ref = checkpoint.checkpoint_id.clone();
                    append_with_id(
                        command,
                        &mut projection,
                        &mut drafts,
                        event_id,
                        Event::CheckpointCreated(CheckpointCreatedEvent { checkpoint }),
                    )?;
                    Some(checkpoint_ref)
                } else {
                    None
                };
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::SessionEnded(SessionEndedEvent {
                        session_ref: payload.session_ref.clone(),
                        ending_checkpoint_ref: checkpoint_ref,
                    }),
                )?;
            }
            Command::ConfirmExternalMapping(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ExternalMappingConfirmed(ExternalMappingConfirmedEvent {
                    mapping: payload.mapping.clone(),
                }),
            )?,
            Command::RevokeExternalMapping(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ExternalMappingRevoked(ExternalMappingRevokedEvent {
                    mapping_ref: payload.mapping_ref.clone(),
                    reason: payload.reason.clone(),
                }),
            )?,
            Command::RecordEventCorrection(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::EventCorrectionRecorded(EventCorrectionRecordedEvent {
                    corrects_event_ref: payload.corrects_event_ref.clone(),
                    reason: payload.reason.clone(),
                    replacement_field_values: payload.replacement_field_values.clone(),
                }),
            )?,
            Command::RecordWorkProgress(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::WorkProgressRecorded(WorkProgressRecordedEvent {
                    work_record: payload.work_record.clone(),
                }),
            )?,
            Command::RecordUnknown(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::UnknownRecorded(UnknownRecordedEvent {
                    unknown: payload.unknown.clone(),
                }),
            )?,
            Command::ResolveUnknown(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::UnknownResolved(UnknownResolvedEvent {
                    unknown_ref: payload.unknown_ref.clone(),
                    resolution_ref: payload.resolution_ref.clone(),
                }),
            )?,
            Command::CreateContinuityCheckpoint(payload) => {
                if payload.reason.trim().is_empty() {
                    return Err(invalid("checkpoint creation requires a reason"));
                }
                let event_id = deterministic_event_id(command, 0)?;
                let checkpoint = build_checkpoint(command, &projection, event_id.clone())?;
                append_with_id(
                    command,
                    &mut projection,
                    &mut drafts,
                    event_id,
                    Event::CheckpointCreated(CheckpointCreatedEvent { checkpoint }),
                )?;
            }
            _ => {
                return Err(invalid(
                    "command does not belong to the S6 continuity engine",
                ));
            }
        }
        Ok(drafts)
    }

    /// Appends at most one matrix-compatible checkpoint freshness event after
    /// previewing an already-owned command batch.
    ///
    /// # Errors
    ///
    /// Rejects an invalid base batch or a command/project/version mismatch.
    pub fn extend_checkpoint_freshness(
        command: &CommandEnvelope,
        stream: &[EventEnvelope],
        base_drafts: Vec<EventDraft>,
    ) -> Result<Vec<EventDraft>, ContinuityError> {
        if !allows_checkpoint_invalidation(&command.command) {
            return Ok(base_drafts);
        }
        let mut projection = checked_projection(command, stream)?;
        let Some(checkpoint_ref) = projection.state().checkpoint_ref.clone() else {
            return Ok(base_drafts);
        };
        if !matches!(
            projection.checkpoint_freshness(&checkpoint_ref),
            Some(
                CheckpointFreshnessStatus::Current
                    | CheckpointFreshnessStatus::CurrentForProjectOnly
            )
        ) {
            return Ok(base_drafts);
        }
        let mut drafts = Vec::with_capacity(base_drafts.len() + 1);
        for draft in base_drafts {
            preview_existing(command, &mut projection, &mut drafts, draft)?;
        }
        let freshness = invalidation_status(command, &drafts);
        let reason_code = serde_json::to_value(command.command_type())?
            .as_str()
            .ok_or(ContinuityError::Identity)?
            .to_owned();
        append(
            command,
            &mut projection,
            &mut drafts,
            Event::CheckpointInvalidated(CheckpointInvalidatedEvent {
                checkpoint_ref,
                freshness,
                reason_codes: vec![reason_code],
            }),
        )?;
        Ok(drafts)
    }
}

fn checked_projection(
    command: &CommandEnvelope,
    stream: &[EventEnvelope],
) -> Result<ProjectStateProjection, ContinuityError> {
    if stream.len() as u64 != command.expected_state_version {
        return Err(invalid(
            "command expected version does not match the supplied stream",
        ));
    }
    let projection = ProjectStateProjection::rebuild(stream)?;
    validate_state_integrity(projection.state())?;
    if projection.state().project.project_id != command.project_ref {
        return Err(invalid(
            "command project does not match the supplied stream",
        ));
    }
    Ok(projection)
}

fn build_checkpoint(
    command: &CommandEnvelope,
    projection: &ProjectStateProjection,
    checkpoint_event_ref: EventId,
) -> Result<ContinuityCheckpoint, ContinuityError> {
    let state = projection.state();
    let current_bindings = projection
        .repository_bindings()
        .into_iter()
        .filter(|binding| binding.status == RepositoryBindingStatus::Current)
        .collect::<Vec<_>>();
    if current_bindings.len() > 1 {
        return Err(invalid(
            "checkpoint cannot choose implicitly between current repositories",
        ));
    }
    let repository_binding = current_bindings.first().and_then(|binding| {
        match (
            &binding.current_snapshot_ref,
            binding.current_snapshot_digest,
        ) {
            (Some(snapshot_ref), Some(snapshot_digest)) => Some(RepositorySnapshot {
                repository_ref: binding.repository_ref.clone(),
                snapshot_ref: snapshot_ref.clone(),
                snapshot_digest,
            }),
            _ => None,
        }
    });
    let verified_completed_phase_refs = state
        .completed_phase_refs
        .iter()
        .filter(|phase_ref| {
            projection
                .gates_for_phase(phase_ref)
                .into_iter()
                .all(|gate| {
                    gate.criticality != GateCriticality::Required
                        || projection.gate_status(&gate.gate_id) == Some(GateStatus::Verified)
                })
        })
        .cloned()
        .collect();
    let mut checkpoint = ContinuityCheckpoint {
        schema_version: SchemaVersion::V0_2_0,
        checkpoint_id: stable_command_id("checkpoint", command)?,
        project_ref: state.project.project_id.clone(),
        captured_state_version: state.state_version,
        captured_material_state_version: state.material_state_version,
        captured_state_digest: state.state_digest,
        checkpoint_event_version: state.state_version + 1,
        checkpoint_event_ref,
        authority_document_refs: state
            .authority_documents
            .iter()
            .map(|document| document.document_id.clone())
            .collect(),
        active_phase_refs: state.active_phase_refs.clone(),
        primary_active_phase_ref: state.primary_active_phase_ref.clone(),
        verified_completed_phase_refs,
        pending_gate_refs: state.pending_gate_refs.clone(),
        current_decision_refs: state.current_decision_refs.clone(),
        open_blocker_refs: state.open_blocker_refs.clone(),
        open_risk_refs: state.open_risk_refs.clone(),
        next_action_refs: state.ordered_next_action_refs.clone(),
        last_material_event_refs: state.last_material_event_ref.iter().cloned().collect(),
        repository_binding,
        unknown_refs: projection.open_unknown_refs(),
        conflict_refs: state.open_conflict_refs.clone(),
        created_by: command.actor.actor_id.clone(),
        created_at: command.submitted_at.clone(),
        checkpoint_digest: Sha256Digest::from([0_u8; 32]),
    };
    checkpoint.checkpoint_digest = checkpoint_digest(&checkpoint)?;
    Ok(checkpoint)
}

const fn allows_checkpoint_invalidation(command: &Command) -> bool {
    matches!(
        command,
        Command::RegisterAuthorityDocument(_)
            | Command::RefreshAuthorityDocumentDigest(_)
            | Command::SupersedeAuthorityDocument(_)
            | Command::BindRepository(_)
            | Command::RefreshRepositoryBinding(_)
            | Command::RevokeRepositoryBinding(_)
            | Command::ReviseRoadmap(_)
            | Command::ReopenPhase(_)
            | Command::DefineGate(_)
            | Command::RecordClaim(_)
            | Command::UpdateClaimStatus(_)
            | Command::RecordUnknown(_)
            | Command::ResolveUnknown(_)
            | Command::UnlinkExternalEvidence(_)
            | Command::InvalidateEvidence(_)
            | Command::AcceptDecision(_)
            | Command::SupersedeDecision(_)
            | Command::OpenBlocker(_)
            | Command::UpdateBlocker(_)
            | Command::ResolveBlocker(_)
            | Command::OpenRisk(_)
            | Command::UpdateRisk(_)
            | Command::MitigateRisk(_)
            | Command::CloseRisk(_)
            | Command::SetNextAction(_)
            | Command::CompleteNextAction(_)
            | Command::CancelNextAction(_)
            | Command::ConfirmExternalMapping(_)
            | Command::RevokeExternalMapping(_)
            | Command::ResolveConflict(_)
            | Command::RecordEventCorrection(_)
    )
}

fn invalidation_status(
    command: &CommandEnvelope,
    drafts: &[EventDraft],
) -> CheckpointFreshnessStatus {
    if drafts
        .iter()
        .any(|draft| matches!(draft.event, Event::ConflictOpened(_)))
    {
        return CheckpointFreshnessStatus::Conflicted;
    }
    match command.command {
        Command::RegisterAuthorityDocument(_)
        | Command::RefreshAuthorityDocumentDigest(_)
        | Command::SupersedeAuthorityDocument(_) => CheckpointFreshnessStatus::StaleForAuthority,
        Command::BindRepository(_)
        | Command::RefreshRepositoryBinding(_)
        | Command::RevokeRepositoryBinding(_) => CheckpointFreshnessStatus::StaleForRepository,
        _ => CheckpointFreshnessStatus::Invalidated,
    }
}

fn append(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    event: Event,
) -> Result<(), ContinuityError> {
    let event_id = deterministic_event_id(command, drafts.len())?;
    append_with_id(command, projection, drafts, event_id, event)
}

fn append_with_id(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    event_id: EventId,
    event: Event,
) -> Result<(), ContinuityError> {
    let draft = EventDraft {
        event_id,
        event,
        correlation_ref: stable_command_id("correlation", command)?,
        occurred_at: command.submitted_at.clone(),
        recorded_at: command.submitted_at.clone(),
    };
    let head = projection.head_digest();
    let envelope = build_event(
        command,
        projection.state().state_version + 1,
        Some(head),
        draft.clone(),
    )?;
    *projection = projection.clone().apply(head, &envelope)?;
    drafts.push(draft);
    Ok(())
}

fn preview_existing(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    draft: EventDraft,
) -> Result<(), ContinuityError> {
    let head = projection.head_digest();
    let envelope = build_event(
        command,
        projection.state().state_version + 1,
        Some(head),
        draft.clone(),
    )?;
    *projection = projection.clone().apply(head, &envelope)?;
    drafts.push(draft);
    Ok(())
}

fn deterministic_event_id(
    command: &CommandEnvelope,
    ordinal: usize,
) -> Result<EventId, ContinuityError> {
    let command_uuid = command.command_id.uuid();
    let mut hasher = Sha256::new();
    hasher.update(b"project-ledger:event-id:v1");
    hasher.update(command_uuid.as_bytes());
    hasher.update(
        u64::try_from(ordinal)
            .map_err(|_| ContinuityError::Identity)?
            .to_be_bytes(),
    );
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes[..6].copy_from_slice(&command_uuid.as_bytes()[..6]);
    bytes[6..].copy_from_slice(&digest[..10]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    EventId::parse(&format!("event:{}", Uuid::from_bytes(bytes)))
        .map_err(|_| ContinuityError::Identity)
}

fn stable_command_id(prefix: &str, command: &CommandEnvelope) -> Result<StableId, ContinuityError> {
    StableId::parse(&format!("{prefix}:{}", command.command_id.uuid()))
        .map_err(|_| ContinuityError::Identity)
}

fn checkpoint_digest(checkpoint: &ContinuityCheckpoint) -> Result<Sha256Digest, ContinuityError> {
    let mut value = serde_json::to_value(checkpoint)?;
    value
        .as_object_mut()
        .ok_or(ContinuityError::Identity)?
        .remove("checkpoint_digest");
    Ok(sha256_digest(&value)?)
}

fn invalid(message: &str) -> ContinuityError {
    ContinuityError::InvalidCommand {
        message: message.to_owned(),
        subject_refs: Vec::new(),
    }
}
