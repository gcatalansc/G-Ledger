use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, Command, CommandEnvelope, CommandId, CommandResult, LedgerTimestamp, ProjectId,
    ResourceRef, StableId,
};
use project_ledger_event_store::{CommandDraft, prepare_command};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{ContractError, GSKILLS_PROFILE};

/// The only G-Skills record capabilities mapped by Project Ledger v0.2.0.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecordCommandKind {
    RecordWorkProgress,
    RecordVerificationResult,
    RequestGateEvaluation,
    RequestGateRevalidation,
    CreateContinuityCheckpoint,
}

impl RecordCommandKind {
    pub(crate) const fn schema_ref(self) -> &'static str {
        match self {
            Self::RecordWorkProgress => "urn:project-ledger:command:record-work-progress:0.2.0",
            Self::RecordVerificationResult => {
                "urn:project-ledger:command:record-verification-result:0.2.0"
            }
            Self::RequestGateEvaluation => {
                "urn:project-ledger:command:request-gate-evaluation:0.2.0"
            }
            Self::RequestGateRevalidation => {
                "urn:project-ledger:command:request-gate-revalidation:0.2.0"
            }
            Self::CreateContinuityCheckpoint => {
                "urn:project-ledger:command:create-continuity-checkpoint:0.2.0"
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalActorIdentity {
    #[serde(rename = "type")]
    pub actor_type: String,
    pub id: String,
    pub on_behalf_of: Option<String>,
}

/// Explicit operator-controlled write policy. Loading metadata alone never creates this grant.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordSinkPolicy {
    pub schema_version: String,
    pub policy_id: StableId,
    pub sink_id: String,
    pub profile: String,
    pub gskills_project_ref: String,
    pub external_project_ref: ResourceRef,
    pub native_project_ref: ProjectId,
    pub(crate) external_actor: ExternalActorIdentity,
    pub native_actor: Actor,
    pub actor_trust_domain: String,
    pub allowed_command_kinds: BTreeSet<RecordCommandKind>,
    pub approval_requirement: String,
    pub provider_manifest_digest: Sha256Digest,
    pub provider_executable_digest: Sha256Digest,
    pub compatibility_lock_digest: Sha256Digest,
    pub identity_map_digest: Sha256Digest,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

impl RecordSinkPolicy {
    /// Parses a closed policy and verifies its canonical self-digest.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] when syntax, identity, capability set or digest is invalid.
    pub fn from_value(value: &Value) -> Result<Self, RecordSinkError> {
        verify_digest(value).map_err(|_| RecordSinkError::PolicyDigestMismatch)?;
        let policy: Self = serde_json::from_value(value.clone())?;
        if policy.schema_version != "0.2.0"
            || policy.sink_id != "project-ledger"
            || policy.profile != GSKILLS_PROFILE
            || policy.gskills_project_ref.trim().is_empty()
            || policy.actor_trust_domain.trim().is_empty()
            || policy.approval_requirement != "USER_OR_POLICY"
            || policy.allowed_command_kinds.is_empty()
        {
            return Err(RecordSinkError::InvalidPolicy {
                message: "policy identity, approval or allowlist is invalid".to_owned(),
            });
        }
        Ok(policy)
    }
}

/// Current immutable identity tuple checked before mapping and again before submit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordSinkStateIdentity {
    pub state_version: u64,
    pub material_state_version: u64,
    pub state_digest: Sha256Digest,
}

/// Fully validated mapping that still has not invoked the mutable host operation.
#[derive(Clone, Debug)]
pub struct PreparedRecordCommand {
    pub external_command: Value,
    pub external_command_digest: Sha256Digest,
    pub command_id: CommandId,
    pub command_schema_ref: ResourceRef,
    pub project_ref: ProjectId,
    pub expected_state_version: u64,
    pub idempotency_key: StableId,
    pub actor: Actor,
    pub command: Command,
    pub prepared_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
}

impl PreparedRecordCommand {
    /// Materializes the native envelope only at the submit boundary.
    ///
    /// The stable preparation timestamp is used as the native wire timestamp so an exact
    /// replay produces the same native command digest; preparation itself remains read-only.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] if native canonical command construction fails.
    pub fn native_envelope(&self) -> Result<CommandEnvelope, RecordSinkError> {
        let envelope = prepare_command(CommandDraft {
            schema_version: project_ledger_domain::SchemaVersion::V0_2_0,
            command_id: self.command_id.clone(),
            project_ref: self.project_ref.clone(),
            expected_state_version: self.expected_state_version,
            idempotency_key: self.idempotency_key.clone(),
            actor: self.actor.clone(),
            command: self.command.clone(),
            submitted_at: self.prepared_at.clone(),
        })?;
        if envelope.command_schema_ref != self.command_schema_ref {
            return Err(RecordSinkError::SchemaReferenceMismatch {
                expected: envelope.command_schema_ref.to_string(),
                actual: self.command_schema_ref.to_string(),
            });
        }
        Ok(envelope)
    }
}

/// Storage-independent host boundary. Status and reconciliation have no execution primitive.
pub trait RecordSinkHost {
    /// Returns the deterministic host observation time used for approval expiry.
    fn observed_at(&self) -> LedgerTimestamp;

    /// Reads the current canonical state tuple for optimistic concurrency.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkHostError`] when canonical state cannot be read or trusted.
    fn state_identity(
        &self,
        project_ref: &ProjectId,
    ) -> Result<RecordSinkStateIdentity, RecordSinkHostError>;

    /// Executes one already validated native command through the canonical processor.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkHostError`] when command processing or persistence fails.
    fn execute(&self, command: CommandEnvelope) -> Result<CommandResult, RecordSinkHostError>;

    /// Looks up a durable native result without submitting or retrying the command.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkHostError`] when the durable lookup cannot be completed safely.
    fn command_result(
        &self,
        command_ref: &CommandId,
    ) -> Result<Option<CommandResult>, RecordSinkHostError>;

    /// Obtains a reference only after rebuilding and confirming the resulting projection.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkHostError`] when the requested state cannot be confirmed.
    fn confirmed_projection_ref(
        &self,
        project_ref: &ProjectId,
        state_version: u64,
    ) -> Result<ResourceRef, RecordSinkHostError>;
}

/// Opaque host error that does not leak storage implementation details into interop.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct RecordSinkHostError {
    message: String,
}

impl RecordSinkHostError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Fail-closed errors for write trust, semantic validation and native mapping.
#[derive(Debug, Error)]
pub enum RecordSinkError {
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    NativeIntegrity(#[from] project_ledger_event_store::IntegrityError),
    #[error(transparent)]
    Host(#[from] RecordSinkHostError),
    #[error("recording requires explicit TRUSTED_FOR_RECORDING policy")]
    WriteTrustRequired,
    #[error("invalid record sink policy: {message}")]
    InvalidPolicy { message: String },
    #[error("policy digest mismatch")]
    PolicyDigestMismatch,
    #[error("external command digest mismatch")]
    CommandDigestMismatch,
    #[error("prepared command digest or preview binding mismatch")]
    PreparedCommandDigestMismatch,
    #[error("unsupported record capability {kind}")]
    UnsupportedCapability { kind: String },
    #[error("schema reference mismatch: expected {expected}, got {actual}")]
    SchemaReferenceMismatch { expected: String, actual: String },
    #[error("identity or idempotency scope mismatch: {message}")]
    IdentityMismatch { message: &'static str },
    #[error("state identity mismatch: {message}")]
    StateMismatch { message: &'static str },
    #[error("privacy requirement violated: {message}")]
    PrivacyViolation { message: &'static str },
    #[error("approval binding mismatch: {field}")]
    ApprovalBindingMismatch { field: &'static str },
    #[error("invalid external record command: {message}")]
    InvalidCommand { message: String },
    #[error("native durable result does not belong to the mapped command")]
    DurableResultMismatch,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // Field name mirrors the exact upstream contract.
pub(crate) struct ExternalRecordCommand {
    pub schema_version: String,
    pub external_record_command_id: String,
    pub command_kind: String,
    pub sink_id: String,
    pub profile: String,
    pub project_ref: String,
    pub external_project_ref: ResourceRef,
    pub actor: ExternalActorIdentity,
    pub session_ref: String,
    pub native_command: ExternalNativeCommand,
    pub expected_state: ExternalExpectedState,
    pub payload: Value,
    pub idempotency_key: String,
    pub idempotency_scope: ExternalIdempotencyScope,
    pub preparation: ExternalPreparation,
    pub privacy: ExternalPrivacy,
    pub provenance_ref: ResourceRef,
    #[serde(rename = "digest")]
    pub _digest: Sha256Digest,
    #[serde(default, rename = "extensions")]
    pub _extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalNativeCommand {
    pub command_type: String,
    pub command_schema_ref: Option<ResourceRef>,
    pub command_schema_ref_status: String,
    pub expected_state_version: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalExpectedState {
    pub project_state_version: u64,
    pub material_state_version: u64,
    pub project_state_digest: Option<Sha256Digest>,
    pub checkpoint_ref: Option<ResourceRef>,
    pub checkpoint_digest: Option<Sha256Digest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalIdempotencyScope {
    pub sink_id: String,
    pub external_project_ref: ResourceRef,
    pub native_command_type: String,
    pub actor_trust_domain: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPreparation {
    pub prepared_by: String,
    pub prepared_at: LedgerTimestamp,
    pub prepared_command_digest: Sha256Digest,
    pub preview_digest: Sha256Digest,
    pub provider_manifest_digest: Sha256Digest,
    pub provider_executable_digest: Sha256Digest,
    pub compatibility_lock_digest: Sha256Digest,
    pub policy_digest: Sha256Digest,
    pub identity_map_digest: Sha256Digest,
    pub expected_state_digest: Sha256Digest,
    pub approval_requirement: String,
    pub approval_expires_at: LedgerTimestamp,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPrivacy {
    pub raw_output_included: bool,
    pub evidence_refs_only: bool,
}

pub(crate) fn verify_digest(value: &Value) -> Result<Sha256Digest, RecordSinkError> {
    let object = value
        .as_object()
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "digest-bearing record is not an object".to_owned(),
        })?;
    let actual = object
        .get("digest")
        .and_then(Value::as_str)
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "digest is missing or is not a string".to_owned(),
        })?;
    let actual = Sha256Digest::parse(actual)?;
    let mut unsigned = object.clone();
    unsigned.remove("digest");
    let expected = sha256_digest(&unsigned)?;
    if actual != expected {
        return Err(RecordSinkError::CommandDigestMismatch);
    }
    Ok(actual)
}

pub(crate) fn prepared_core_digest(value: &Value) -> Result<Sha256Digest, RecordSinkError> {
    let mut core = value.clone();
    let object = core
        .as_object_mut()
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "prepared command is not an object".to_owned(),
        })?;
    object.remove("digest");
    let preparation = object
        .get_mut("preparation")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "preparation is missing or is not an object".to_owned(),
        })?;
    for field in [
        "prepared_command_digest",
        "preview_digest",
        "provider_manifest_digest",
        "provider_executable_digest",
        "compatibility_lock_digest",
        "policy_digest",
        "identity_map_digest",
        "expected_state_digest",
    ] {
        preparation.remove(field);
    }
    Ok(sha256_digest(&core)?)
}

pub(crate) fn parse_command_kind(value: &str) -> Result<RecordCommandKind, RecordSinkError> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        RecordSinkError::UnsupportedCapability {
            kind: value.to_owned(),
        }
    })
}

pub(crate) fn external_command_id(value: &str) -> Result<CommandId, RecordSinkError> {
    let uuid =
        value
            .strip_prefix("external-command:")
            .ok_or_else(|| RecordSinkError::InvalidCommand {
                message: "external_record_command_id must use external-command:<UUIDv7>".to_owned(),
            })?;
    CommandId::parse(&format!("command:{uuid}")).map_err(|error| RecordSinkError::InvalidCommand {
        message: error.to_string(),
    })
}

pub(crate) fn stable_id(value: &str, field: &'static str) -> Result<StableId, RecordSinkError> {
    StableId::parse(value).map_err(|error| RecordSinkError::InvalidCommand {
        message: format!("{field}: {error}"),
    })
}
