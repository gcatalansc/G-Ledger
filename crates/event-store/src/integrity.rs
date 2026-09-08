//! Canonical integrity material shared by application and storage adapters.

use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, Command, CommandEnvelope, CommandId, CommandRejection, CommandResult,
    CommandResultStatus, Event, EventCausation, EventEnvelope, EventId, ExternalSubmissionRecord,
    ExternalSubmissionState, LedgerTimestamp, ProjectId, ResourceRef, SchemaVersion, StableId,
};
use serde::Serialize;
use thiserror::Error;

/// Failures while constructing or validating canonical command and event envelopes.
#[derive(Debug, Error)]
pub enum IntegrityError {
    /// JSON serialization failed before canonical hashing.
    #[error("contract serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    /// RFC 8785 serialization or hashing failed.
    #[error("canonical hashing failed: {0}")]
    Canonical(#[from] CanonicalJsonError),
    /// A generated schema reference was not a valid resource reference.
    #[error("generated contract schema reference is invalid")]
    InvalidSchemaReference,
    /// A discriminated command or event did not contain its payload member.
    #[error("discriminated contract payload is missing")]
    MissingPayload,
    /// The payload no longer matches its recorded digest.
    #[error("payload digest mismatch")]
    PayloadDigestMismatch,
    /// Envelope metadata or payload digest no longer matches the envelope digest.
    #[error("envelope digest mismatch")]
    EnvelopeDigestMismatch,
    /// The schema reference does not match the closed discriminator.
    #[error("contract schema reference mismatch")]
    SchemaReferenceMismatch,
}

/// Inputs required to construct a canonical command envelope.
#[derive(Clone, Debug)]
pub struct CommandDraft {
    pub schema_version: SchemaVersion,
    pub command_id: CommandId,
    pub project_ref: ProjectId,
    pub expected_state_version: u64,
    pub idempotency_key: StableId,
    pub actor: Actor,
    pub command: Command,
    pub submitted_at: LedgerTimestamp,
}

/// Inputs required to construct the next event in a stream.
#[derive(Clone, Debug)]
pub struct EventDraft {
    pub event_id: EventId,
    pub event: Event,
    pub correlation_ref: StableId,
    pub occurred_at: LedgerTimestamp,
    pub recorded_at: LedgerTimestamp,
}

/// Inputs required to construct a canonical server-side command result.
#[derive(Clone, Debug)]
pub struct CommandResultDraft {
    pub command_result_id: StableId,
    pub command_ref: CommandId,
    pub status: CommandResultStatus,
    pub previous_state_version: u64,
    pub new_state_version: u64,
    pub emitted_event_refs: Vec<EventId>,
    pub resulting_checkpoint_ref: Option<StableId>,
    pub rejection: Option<CommandRejection>,
    pub replayed: bool,
    pub processed_at: LedgerTimestamp,
}

/// Inputs required to construct a canonical local external-submission record.
#[derive(Clone, Debug)]
pub struct ExternalSubmissionDraft {
    pub submission_id: StableId,
    pub command_ref: CommandId,
    pub idempotency_key: StableId,
    pub state: ExternalSubmissionState,
    pub attempt_count: u32,
    pub last_transport_error: Option<String>,
    pub known_result_ref: Option<StableId>,
    pub prepared_at: LedgerTimestamp,
    pub submitted_at: Option<LedgerTimestamp>,
    pub reconciled_at: Option<LedgerTimestamp>,
}

/// Constructs a command envelope and both of its canonical digests.
///
/// # Errors
///
/// Returns [`IntegrityError`] when serialization, hashing, or generated schema
/// reference validation fails.
pub fn prepare_command(draft: CommandDraft) -> Result<CommandEnvelope, IntegrityError> {
    let payload_digest = payload_digest(&draft.command)?;
    let command_schema_ref = contract_schema_ref("command", &draft.command.command_type())?;
    let mut envelope = CommandEnvelope {
        schema_version: draft.schema_version,
        command_id: draft.command_id,
        command_schema_ref,
        project_ref: draft.project_ref,
        expected_state_version: draft.expected_state_version,
        idempotency_key: draft.idempotency_key,
        actor: draft.actor,
        command: draft.command,
        submitted_at: draft.submitted_at,
        payload_digest,
        command_digest: Sha256Digest::from([0_u8; 32]),
    };
    envelope.command_digest = digest_without(&envelope, "command_digest")?;
    Ok(envelope)
}

/// Validates a command's schema reference, payload digest, and envelope digest.
///
/// # Errors
///
/// Returns [`IntegrityError`] on any serialization failure or mismatch.
pub fn validate_command_integrity(command: &CommandEnvelope) -> Result<(), IntegrityError> {
    let expected_version = if matches!(command.command, Command::RecordEpisodeUpdate(_)) {
        SchemaVersion::V0_3_0
    } else {
        SchemaVersion::V0_2_0
    };
    if command.schema_version != expected_version {
        return Err(IntegrityError::SchemaReferenceMismatch);
    }
    let expected_payload = payload_digest(&command.command)?;
    if expected_payload != command.payload_digest {
        return Err(IntegrityError::PayloadDigestMismatch);
    }
    let expected_schema = contract_schema_ref("command", &command.command_type())?;
    if expected_schema != command.command_schema_ref {
        return Err(IntegrityError::SchemaReferenceMismatch);
    }
    if digest_without(command, "command_digest")? != command.command_digest {
        return Err(IntegrityError::EnvelopeDigestMismatch);
    }
    Ok(())
}

/// Builds the next immutable event and its mandatory chain/digest fields.
///
/// # Errors
///
/// Returns [`IntegrityError`] when serialization, hashing, or generated schema
/// reference validation fails.
pub fn build_event(
    command: &CommandEnvelope,
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    draft: EventDraft,
) -> Result<EventEnvelope, IntegrityError> {
    let payload_digest = payload_digest(&draft.event)?;
    let event_schema_ref = contract_schema_ref("event", &draft.event.event_type())?;
    let mut envelope = EventEnvelope {
        schema_version: if command.schema_version == SchemaVersion::V0_3_0
            && matches!(command.command, Command::RecordEpisodeUpdate(_))
            && !matches!(draft.event, Event::EpisodeUpdateRecorded(_))
        {
            SchemaVersion::V0_2_0
        } else {
            command.schema_version
        },
        event_id: draft.event_id,
        project_ref: command.project_ref.clone(),
        stream_id: command.project_ref.clone(),
        stream_version,
        event: draft.event,
        event_schema_ref,
        actor: command.actor.clone(),
        causation: EventCausation {
            command_ref: command.command_id.clone(),
            command_digest: command.command_digest,
            correlation_ref: draft.correlation_ref,
        },
        occurred_at: draft.occurred_at,
        recorded_at: draft.recorded_at,
        previous_event_digest,
        payload_digest,
        event_digest: Sha256Digest::from([0_u8; 32]),
    };
    envelope.event_digest = digest_without(&envelope, "event_digest")?;
    Ok(envelope)
}

/// Validates an event's schema reference, payload digest, and envelope digest.
///
/// # Errors
///
/// Returns [`IntegrityError`] on any serialization failure or mismatch.
pub fn validate_event_integrity(event: &EventEnvelope) -> Result<(), IntegrityError> {
    let expected_version = if matches!(event.event, Event::EpisodeUpdateRecorded(_)) {
        SchemaVersion::V0_3_0
    } else {
        SchemaVersion::V0_2_0
    };
    if event.schema_version != expected_version {
        return Err(IntegrityError::SchemaReferenceMismatch);
    }
    let expected_payload = payload_digest(&event.event)?;
    if expected_payload != event.payload_digest {
        return Err(IntegrityError::PayloadDigestMismatch);
    }
    let expected_schema = contract_schema_ref("event", &event.event_type())?;
    if expected_schema != event.event_schema_ref {
        return Err(IntegrityError::SchemaReferenceMismatch);
    }
    if digest_without(event, "event_digest")? != event.event_digest {
        return Err(IntegrityError::EnvelopeDigestMismatch);
    }
    Ok(())
}

/// Constructs a server result and its canonical digest.
///
/// # Errors
///
/// Returns [`IntegrityError`] when canonical serialization fails.
pub fn build_command_result(draft: CommandResultDraft) -> Result<CommandResult, IntegrityError> {
    let mut result = CommandResult {
        schema_version: SchemaVersion::V0_2_0,
        command_result_id: draft.command_result_id,
        command_ref: draft.command_ref,
        status: draft.status,
        previous_state_version: draft.previous_state_version,
        new_state_version: draft.new_state_version,
        emitted_event_refs: draft.emitted_event_refs,
        resulting_checkpoint_ref: draft.resulting_checkpoint_ref,
        rejection: draft.rejection,
        idempotency: project_ledger_domain::IdempotencyResult {
            replayed: draft.replayed,
        },
        processed_at: draft.processed_at,
        digest: Sha256Digest::from([0_u8; 32]),
    };
    result.digest = digest_without(&result, "digest")?;
    Ok(result)
}

/// Validates that a command result still matches its canonical digest.
///
/// # Errors
///
/// Returns [`IntegrityError::EnvelopeDigestMismatch`] when the result changed.
pub fn validate_command_result_integrity(result: &CommandResult) -> Result<(), IntegrityError> {
    if digest_without(result, "digest")? != result.digest {
        return Err(IntegrityError::EnvelopeDigestMismatch);
    }
    Ok(())
}

/// Constructs a local external-submission record and canonical digest.
///
/// # Errors
///
/// Returns [`IntegrityError`] when canonical serialization fails.
pub fn build_external_submission(
    draft: ExternalSubmissionDraft,
) -> Result<ExternalSubmissionRecord, IntegrityError> {
    let mut record = ExternalSubmissionRecord {
        schema_version: SchemaVersion::V0_2_0,
        submission_id: draft.submission_id,
        command_ref: draft.command_ref,
        idempotency_key: draft.idempotency_key,
        state: draft.state,
        attempt_count: draft.attempt_count,
        last_transport_error: draft.last_transport_error,
        known_result_ref: draft.known_result_ref,
        prepared_at: draft.prepared_at,
        submitted_at: draft.submitted_at,
        reconciled_at: draft.reconciled_at,
        digest: Sha256Digest::from([0_u8; 32]),
    };
    record.digest = digest_without(&record, "digest")?;
    Ok(record)
}

/// Validates that a local external-submission record still matches its digest.
///
/// # Errors
///
/// Returns [`IntegrityError::EnvelopeDigestMismatch`] when the record changed.
pub fn validate_external_submission_integrity(
    record: &ExternalSubmissionRecord,
) -> Result<(), IntegrityError> {
    if digest_without(record, "digest")? != record.digest {
        return Err(IntegrityError::EnvelopeDigestMismatch);
    }
    Ok(())
}

fn payload_digest<T: Serialize>(value: &T) -> Result<Sha256Digest, IntegrityError> {
    let value = serde_json::to_value(value)?;
    let payload = value.get("payload").ok_or(IntegrityError::MissingPayload)?;
    Ok(sha256_digest(payload)?)
}

fn digest_without<T: Serialize>(value: &T, field: &str) -> Result<Sha256Digest, IntegrityError> {
    let mut value = serde_json::to_value(value)?;
    value
        .as_object_mut()
        .ok_or(IntegrityError::MissingPayload)?
        .remove(field);
    Ok(sha256_digest(&value)?)
}

fn contract_schema_ref<T: Serialize>(
    kind: &str,
    discriminator: &T,
) -> Result<ResourceRef, IntegrityError> {
    let encoded = serde_json::to_value(discriminator)?;
    let name = encoded
        .as_str()
        .ok_or(IntegrityError::MissingPayload)?
        .to_ascii_lowercase()
        .replace('_', "-");
    let version = if matches!(
        name.as_str(),
        "record-episode-update" | "episode-update-recorded"
    ) {
        "0.3.0"
    } else {
        "0.2.0"
    };
    ResourceRef::parse(&format!("urn:project-ledger:{kind}:{name}:{version}"))
        .map_err(|_| IntegrityError::InvalidSchemaReference)
}
