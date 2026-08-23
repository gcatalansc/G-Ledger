use project_ledger_domain::{
    Command, CreateContinuityCheckpointPayload, LedgerTimestamp, RecordVerificationResultPayload,
    RecordWorkProgressPayload, RequestGateEvaluationPayload, RequestGateRevalidationPayload,
    ResourceRef,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    PreparedRecordCommand, RecordCommandKind, RecordSinkError, RecordSinkPolicy,
    RecordSinkStateIdentity,
    sink_model::{
        ExternalRecordCommand, external_command_id, parse_command_kind, prepared_core_digest,
        stable_id, verify_digest,
    },
    validate_gskills_contract,
};

pub(crate) fn prepare_record_command(
    value: &Value,
    policy: &RecordSinkPolicy,
    current: RecordSinkStateIdentity,
    observed_at: &LedgerTimestamp,
) -> Result<PreparedRecordCommand, RecordSinkError> {
    prepare_record_command_inner(value, policy, Some(current), Some(observed_at))
}

pub(crate) fn prepare_record_command_for_lookup(
    value: &Value,
    policy: &RecordSinkPolicy,
) -> Result<PreparedRecordCommand, RecordSinkError> {
    prepare_record_command_inner(value, policy, None, None)
}

pub(crate) fn prepare_record_command_for_submit(
    value: &Value,
    policy: &RecordSinkPolicy,
    observed_at: &LedgerTimestamp,
) -> Result<PreparedRecordCommand, RecordSinkError> {
    prepare_record_command_inner(value, policy, None, Some(observed_at))
}

fn prepare_record_command_inner(
    value: &Value,
    policy: &RecordSinkPolicy,
    current: Option<RecordSinkStateIdentity>,
    observed_at: Option<&LedgerTimestamp>,
) -> Result<PreparedRecordCommand, RecordSinkError> {
    validate_gskills_contract("external-record-command", value)?;
    let external_command_digest = verify_digest(value)?;
    let external: ExternalRecordCommand = serde_json::from_value(value.clone())?;
    let kind = parse_command_kind(&external.command_kind)?;

    validate_identity_scope(&external, policy, kind)?;
    validate_state(&external, current)?;
    validate_approval(value, &external, policy, observed_at)?;

    let command = map_payload(kind, &external, policy)?;
    let command_id = external_command_id(&external.external_record_command_id)?;
    let command_schema_ref = external
        .native_command
        .command_schema_ref
        .clone()
        .ok_or_else(|| RecordSinkError::SchemaReferenceMismatch {
            expected: kind.schema_ref().to_owned(),
            actual: "null".to_owned(),
        })?;

    Ok(PreparedRecordCommand {
        external_command: value.clone(),
        external_command_digest,
        command_id,
        command_schema_ref,
        project_ref: policy.native_project_ref.clone(),
        expected_state_version: external.native_command.expected_state_version,
        idempotency_key: stable_id(&external.idempotency_key, "idempotency_key")?,
        actor: policy.native_actor.clone(),
        command,
        prepared_at: external.preparation.prepared_at,
        provenance_ref: external.provenance_ref,
    })
}

fn validate_identity_scope(
    external: &ExternalRecordCommand,
    policy: &RecordSinkPolicy,
    kind: RecordCommandKind,
) -> Result<(), RecordSinkError> {
    if external.schema_version != "0.3.0"
        || external.sink_id != policy.sink_id
        || external.profile != policy.profile
        || external.profile != crate::GSKILLS_PROFILE
        || external.project_ref != policy.gskills_project_ref
        || external.external_project_ref != policy.external_project_ref
        || external.actor != policy.external_actor
        || external.idempotency_scope.sink_id != policy.sink_id
        || external.idempotency_scope.external_project_ref != policy.external_project_ref
        || external.idempotency_scope.actor_trust_domain != policy.actor_trust_domain
    {
        return Err(RecordSinkError::IdentityMismatch {
            message: "command identity differs from the explicit policy or idempotency scope",
        });
    }
    if external.session_ref.trim().is_empty() || external.preparation.prepared_by.trim().is_empty()
    {
        return Err(RecordSinkError::IdentityMismatch {
            message: "session_ref and prepared_by must be non-empty",
        });
    }
    if !policy.allowed_command_kinds.contains(&kind) {
        return Err(RecordSinkError::UnsupportedCapability {
            kind: external.command_kind.clone(),
        });
    }
    if external.native_command.command_type != external.command_kind
        || external.idempotency_scope.native_command_type != external.command_kind
    {
        return Err(RecordSinkError::IdentityMismatch {
            message: "command kind is not preserved across the native and idempotency scopes",
        });
    }
    let actual_schema = external
        .native_command
        .command_schema_ref
        .as_ref()
        .map_or("null", ResourceRef::as_str);
    if actual_schema != kind.schema_ref()
        || external.native_command.command_schema_ref_status != "EXACT"
    {
        return Err(RecordSinkError::SchemaReferenceMismatch {
            expected: kind.schema_ref().to_owned(),
            actual: actual_schema.to_owned(),
        });
    }
    if external.privacy.raw_output_included || !external.privacy.evidence_refs_only {
        return Err(RecordSinkError::PrivacyViolation {
            message: "raw output is forbidden and evidence_refs_only must be true",
        });
    }
    Ok(())
}

fn validate_state(
    external: &ExternalRecordCommand,
    current: Option<RecordSinkStateIdentity>,
) -> Result<(), RecordSinkError> {
    if external.native_command.expected_state_version
        != external.expected_state.project_state_version
    {
        return Err(RecordSinkError::StateMismatch {
            message: "project_state_version is inconsistent",
        });
    }
    if external
        .expected_state
        .project_state_digest
        .is_some_and(|digest| digest != external.preparation.expected_state_digest)
    {
        return Err(RecordSinkError::StateMismatch {
            message: "project_state_digest differs from its approval binding",
        });
    }
    if external.expected_state.checkpoint_ref.is_some()
        != external.expected_state.checkpoint_digest.is_some()
    {
        return Err(RecordSinkError::StateMismatch {
            message: "checkpoint_ref and checkpoint_digest must be supplied together",
        });
    }
    if let Some(current) = current {
        if external.expected_state.project_state_version != current.state_version {
            return Err(RecordSinkError::StateMismatch {
                message: "project_state_version is stale",
            });
        }
        if external.expected_state.material_state_version != current.material_state_version {
            return Err(RecordSinkError::StateMismatch {
                message: "material_state_version is stale",
            });
        }
        if external.preparation.expected_state_digest != current.state_digest {
            return Err(RecordSinkError::StateMismatch {
                message: "project_state_digest is stale",
            });
        }
    }
    Ok(())
}

fn validate_approval(
    value: &Value,
    external: &ExternalRecordCommand,
    policy: &RecordSinkPolicy,
    observed_at: Option<&LedgerTimestamp>,
) -> Result<(), RecordSinkError> {
    let binding = &external.preparation;
    for (field, matches) in [
        (
            "provider_manifest_digest",
            binding.provider_manifest_digest == policy.provider_manifest_digest,
        ),
        (
            "provider_executable_digest",
            binding.provider_executable_digest == policy.provider_executable_digest,
        ),
        (
            "compatibility_lock_digest",
            binding.compatibility_lock_digest == policy.compatibility_lock_digest,
        ),
        ("policy_digest", binding.policy_digest == policy.digest),
        (
            "identity_map_digest",
            binding.identity_map_digest == policy.identity_map_digest,
        ),
        (
            "approval_requirement",
            binding.approval_requirement == policy.approval_requirement,
        ),
    ] {
        if !matches {
            return Err(RecordSinkError::ApprovalBindingMismatch { field });
        }
    }
    if binding.approval_expires_at < binding.prepared_at
        || observed_at.is_some_and(|observed| {
            observed < &binding.prepared_at || observed > &binding.approval_expires_at
        })
    {
        return Err(RecordSinkError::ApprovalBindingMismatch {
            field: "approval_expires_at",
        });
    }
    let prepared = prepared_core_digest(value)?;
    if binding.prepared_command_digest != prepared || binding.preview_digest != prepared {
        return Err(RecordSinkError::PreparedCommandDigestMismatch);
    }
    Ok(())
}

fn map_payload(
    kind: RecordCommandKind,
    external: &ExternalRecordCommand,
    policy: &RecordSinkPolicy,
) -> Result<Command, RecordSinkError> {
    match kind {
        RecordCommandKind::RecordWorkProgress => {
            let raw = external.payload.get("work_record").ok_or_else(|| {
                RecordSinkError::InvalidCommand {
                    message: "work_record is required".to_owned(),
                }
            })?;
            verify_digest(raw)?;
            let payload: RecordWorkProgressPayload =
                serde_json::from_value(external.payload.clone())?;
            if payload.work_record.actor_ref != policy.native_actor.actor_id {
                return Err(RecordSinkError::IdentityMismatch {
                    message: "work_record.actor_ref differs from the configured native actor",
                });
            }
            Ok(Command::RecordWorkProgress(payload))
        }
        RecordCommandKind::RecordVerificationResult => {
            let payload: RecordVerificationResultPayload =
                serde_json::from_value(external.payload.clone())?;
            if payload.evidence.project_ref != policy.native_project_ref {
                return Err(RecordSinkError::IdentityMismatch {
                    message: "evidence.project_ref differs from the native project",
                });
            }
            if payload.evidence.privacy.raw_output_stored {
                return Err(RecordSinkError::PrivacyViolation {
                    message: "native evidence cannot store raw external output",
                });
            }
            Ok(Command::RecordVerificationResult(payload))
        }
        RecordCommandKind::RequestGateEvaluation => {
            let payload: RequestGateEvaluationPayload =
                serde_json::from_value(external.payload.clone())?;
            Ok(Command::RequestGateEvaluation(payload))
        }
        RecordCommandKind::RequestGateRevalidation => {
            let payload: RevalidationPayload = serde_json::from_value(external.payload.clone())?;
            validate_gskills_contract("gate-revalidation-signal", &payload.signal)?;
            let signal_digest = verify_digest(&payload.signal)?;
            let signal: GateRevalidationSignal = serde_json::from_value(payload.signal)?;
            if signal.schema_version != "0.3.0"
                || signal.project_ref != policy.gskills_project_ref
                || signal.recommended_command_kind != "REQUEST_GATE_REVALIDATION"
                || signal.reason_codes.is_empty()
                || signal
                    .previous_verification_refs
                    .iter()
                    .any(String::is_empty)
                || signal.previous_repository_snapshot_ref.trim().is_empty()
                || signal.current_repository_snapshot_ref.trim().is_empty()
                || signal.structural_impact_ref.trim().is_empty()
                || signal.matched_dependency_refs.is_empty()
                || signal.confidence_status.trim().is_empty()
                || signal.severity.trim().is_empty()
                || signal.provenance_ref.as_str().is_empty()
                || signal.digest != signal_digest
            {
                return Err(RecordSinkError::InvalidCommand {
                    message: "gate revalidation signal identity or recommendation is invalid"
                        .to_owned(),
                });
            }
            let gate_ref = stable_id(&signal.gate_ref, "signal.gate_ref")?;
            let signal_ref =
                ResourceRef::parse(&signal.gate_revalidation_signal_id).map_err(|error| {
                    RecordSinkError::InvalidCommand {
                        message: format!("signal.gate_revalidation_signal_id: {error}"),
                    }
                })?;
            Ok(Command::RequestGateRevalidation(
                RequestGateRevalidationPayload {
                    gate_ref,
                    signal_ref: Some(signal_ref),
                },
            ))
        }
        RecordCommandKind::CreateContinuityCheckpoint => {
            let payload: CreateContinuityCheckpointPayload =
                serde_json::from_value(external.payload.clone())?;
            if payload.reason.trim().is_empty() {
                return Err(RecordSinkError::InvalidCommand {
                    message: "checkpoint reason must be non-empty".to_owned(),
                });
            }
            Ok(Command::CreateContinuityCheckpoint(payload))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RevalidationPayload {
    signal: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // Field name mirrors the exact upstream contract.
struct GateRevalidationSignal {
    schema_version: String,
    gate_revalidation_signal_id: String,
    project_ref: String,
    gate_ref: String,
    reason_codes: Vec<String>,
    previous_verification_refs: Vec<String>,
    previous_repository_snapshot_ref: String,
    current_repository_snapshot_ref: String,
    structural_impact_ref: String,
    matched_dependency_refs: Vec<String>,
    confidence_status: String,
    recommended_command_kind: String,
    severity: String,
    provenance_ref: ResourceRef,
    digest: project_ledger_canonical_json::Sha256Digest,
    #[serde(default, rename = "extensions")]
    _extensions: std::collections::BTreeMap<String, Value>,
}
