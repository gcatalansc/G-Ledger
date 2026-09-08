use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{CommandResult, TrustLevel};
use project_ledger_event_store::{validate_command_integrity, validate_command_result_integrity};
use serde_json::{Value, json};

use crate::{
    GSKILLS_FIXTURE_MANIFEST_SHA256, GSKILLS_PROFILE, GSKILLS_REGISTRY_SHA256,
    GSKILLS_SPEC_REVISION, GSKILLS_SPEC_SHA256, LEDGER_SPEC_SHA256, PreparedRecordCommand,
    RECIPROCAL_LOCK_SHA256, RecordSinkError, RecordSinkHost, RecordSinkPolicy,
    sink_mapping::{
        prepare_record_command, prepare_record_command_for_lookup,
        prepare_record_command_for_submit,
    },
    validate_gskills_contract,
};

/// Exact, storage-independent G-Skills `RecordSink` semantic boundary.
#[derive(Debug)]
pub struct GSkillsRecordSink<'host, Host: RecordSinkHost + ?Sized> {
    host: &'host Host,
    policy: RecordSinkPolicy,
}

impl<'host, Host: RecordSinkHost + ?Sized> GSkillsRecordSink<'host, Host> {
    /// Builds a sink only from an explicit policy with independent recording trust.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError::WriteTrustRequired`] unless the configured native actor is
    /// exactly `TRUSTED_FOR_RECORDING`.
    pub fn new(host: &'host Host, policy: RecordSinkPolicy) -> Result<Self, RecordSinkError> {
        if policy.native_actor.trust_level != TrustLevel::TrustedForRecording {
            return Err(RecordSinkError::WriteTrustRequired);
        }
        Ok(Self { host, policy })
    }

    /// Validates and maps an external command without invoking the mutable host operation.
    /// Admissibility does not guarantee acceptance: an obsolete expected version is preserved
    /// so submit can return the native, durable conflict. All approval bindings still apply.
    ///
    /// # Errors
    ///
    /// Returns a fail-closed semantic, policy, state, privacy, approval or mapping error.
    pub fn validate(&self, command: &Value) -> Result<PreparedRecordCommand, RecordSinkError> {
        let observed_at = self.host.observed_at();
        let prepared = prepare_record_command_for_submit(command, &self.policy, &observed_at)?;
        if self.bound_result(&prepared)?.is_some() {
            return Ok(prepared);
        }
        let current = self.host.state_identity(&self.policy.native_project_ref)?;
        if current.state_version == prepared.expected_state_version {
            prepare_record_command(command, &self.policy, current, &observed_at)
        } else {
            // The native processor owns VERSION_CONFLICT. Never rewrite approved state.
            Ok(prepared)
        }
    }

    /// Reads the canonical state tuple without mutating the project.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] when the host cannot read or trust canonical state.
    pub fn state_identity(&self) -> Result<crate::RecordSinkStateIdentity, RecordSinkError> {
        Ok(self.host.state_identity(&self.policy.native_project_ref)?)
    }

    /// Emits the exact mutable profile manifest without deriving trust from the manifest itself.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] when canonical digesting or exact schema validation fails.
    pub fn manifest(&self) -> Result<Value, RecordSinkError> {
        let mut manifest = json!({
            "schema_version": "0.3.0",
            "provider_manifest_id": "provider-manifest:project-ledger-record-sink-v0.2.0",
            "provider_id": "project-ledger",
            "provider_version": "0.2.0",
            "protocol": {
                "name": "gskills-context-provider",
                "version": "0.3.0",
                "compatible_range": ">=0.3.0 <0.4.0"
            },
            "implementation": {
                "language": "rust",
                "crate": "project-ledger-interop-gskills",
                "shared_database_required": false
            },
            "transports": ["gskills-stdio-jsonrpc-content-length/1.0.0"],
            "operations": {
                "manifest": "ledger.manifest",
                "command_validate": "ledger.command_validate",
                "command_submit": "ledger.command_submit",
                "command_status": "ledger.command_status",
                "command_reconcile": "ledger.command_reconcile"
            },
            "projection_capabilities": [],
            "record_sink_capabilities": [
                "RECORD_WORK_PROGRESS",
                "RECORD_VERIFICATION_RESULT",
                "REQUEST_GATE_EVALUATION",
                "REQUEST_GATE_REVALIDATION",
                "CREATE_CONTINUITY_CHECKPOINT"
            ],
            "provided_domains": [],
            "claimed_authority_domains": [],
            "side_effect_contract": {
                "reads_mutate_project_state": false,
                "validate_mutates_project_state": false,
                "status_or_reconcile_submit": false,
                "writes_supported": true,
                "writes_require_explicit_policy": true
            },
            "freshness_model": {
                "expected_state_revalidated_before_first_submit": true,
                "status_and_reconcile_use_durable_result": true
            },
            "privacy": {
                "raw_output": false,
                "evidence_refs_only": true
            },
            "schema_refs": {
                "command": "urn:gskills:schema:external-record-command:0.3.0",
                "result": "urn:gskills:schema:external-record-result:0.3.0",
                "submission": "urn:gskills:schema:external-submission-record:0.3.0",
                "gate_revalidation_signal": "urn:gskills:schema:gate-revalidation-signal:0.3.0"
            },
            "contract_identity": {
                "profile": GSKILLS_PROFILE,
                "registry_file_sha256": GSKILLS_REGISTRY_SHA256,
                "fixture_manifest_file_sha256": GSKILLS_FIXTURE_MANIFEST_SHA256,
                "policy_digest": self.policy.digest
            },
            "upstream": {
                "revision": GSKILLS_SPEC_REVISION,
                "spec_sha256": GSKILLS_SPEC_SHA256,
                "reciprocal_lock_sha256": RECIPROCAL_LOCK_SHA256,
                "ledger_spec_sha256": LEDGER_SPEC_SHA256
            },
            "created_at": self.policy.created_at,
            "digest": Sha256Digest::from([0; 32])
        });
        assign_digest(&mut manifest)?;
        validate_gskills_contract("provider-manifest", &manifest)?;
        Ok(manifest)
    }

    /// Submits one explicit command and maps only the returned native domain result.
    ///
    /// Existing command identity is passed once through the native idempotency boundary so
    /// Project Ledger can return its canonical `DUPLICATE_REPLAY`; no hidden retry occurs.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] for semantic/policy/state failure, host failure or invalid
    /// native/external result mapping. A host transport error is not converted into a result.
    pub fn submit(&self, command: &Value) -> Result<Value, RecordSinkError> {
        let prepared = self.validate(command)?;
        let result = self.host.execute(prepared.native_envelope()?)?;
        self.map_result(&prepared, &result)
    }

    /// Looks up the durable result without submitting or retrying the command.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] for invalid command identity, lookup failure or invalid result.
    pub fn status(&self, command: &Value) -> Result<Value, RecordSinkError> {
        self.lookup(command)
    }

    /// Reconciles uncertain client transport state through the same read-only durable lookup.
    ///
    /// # Errors
    ///
    /// Returns [`RecordSinkError`] for invalid command identity, lookup failure or invalid result.
    pub fn reconcile(&self, command: &Value) -> Result<Value, RecordSinkError> {
        self.lookup(command)
    }

    fn lookup(&self, command: &Value) -> Result<Value, RecordSinkError> {
        let prepared = prepare_record_command_for_lookup(command, &self.policy)?;
        let Some(result) = self.bound_result(&prepared)? else {
            return Ok(json!({
                "status": "NOT_FOUND",
                "command_ref": prepared.external_command["external_record_command_id"],
                "idempotency_key": prepared.idempotency_key
            }));
        };
        Ok(json!({
            "status": "FOUND",
            "result": self.map_result(&prepared, &result)?
        }))
    }

    fn bound_result(
        &self,
        prepared: &PreparedRecordCommand,
    ) -> Result<Option<CommandResult>, RecordSinkError> {
        let Some((command, result)) = self.host.command_result(&prepared.command_id)? else {
            return Ok(None);
        };
        validate_command_integrity(&command)?;
        if command != prepared.native_envelope()? {
            return Err(RecordSinkError::IdentityMismatch {
                message: "command id belongs to a different native command, payload or idempotency scope",
            });
        }
        validate_command_result_integrity(&result)?;
        if result.command_ref != command.command_id {
            return Err(RecordSinkError::DurableResultMismatch);
        }
        Ok(Some(result))
    }

    fn map_result(
        &self,
        prepared: &PreparedRecordCommand,
        result: &CommandResult,
    ) -> Result<Value, RecordSinkError> {
        validate_command_result_integrity(result)?;
        if result.command_ref != prepared.command_id {
            return Err(RecordSinkError::DurableResultMismatch);
        }
        let projection_ref = self
            .host
            .confirmed_projection_ref(&prepared.project_ref, result.new_state_version)?;
        let command_uuid = prepared.command_id.uuid();
        let mut external = json!({
            "schema_version": "0.3.0",
            "external_record_result_id": format!("external-result:{command_uuid}"),
            "command_ref": prepared.external_command["external_record_command_id"],
            "command_digest": prepared.external_command_digest,
            "native_result_ref": format!("project-ledger://command-results/{command_uuid}"),
            "native_result_digest": result.digest,
            "sink": {
                "id": "project-ledger",
                "version": "0.2.0",
                "native_manifest_digest": self.policy.provider_manifest_digest,
                "compatibility_lock_ref": "provider-lock:project-ledger-v0.2-gskills-v0.3"
            },
            "status": enum_value(result.status)?,
            "idempotency": {
                "key": prepared.idempotency_key,
                "replayed": result.idempotency.replayed
            },
            "state_transition": {
                "previous_state_version": result.previous_state_version,
                "new_state_version": result.new_state_version,
                "emitted_event_refs": result.emitted_event_refs
            },
            "resulting_checkpoint_ref": result.resulting_checkpoint_ref,
            "post_submit_projection_refs": [],
            "resulting_projection_ref": projection_ref,
            "rejection": result.rejection,
            "processed_at": result.processed_at,
            "provenance_ref": format!("gskills://provenance/external-result/{command_uuid}"),
            "digest": Sha256Digest::from([0; 32])
        });
        assign_digest(&mut external)?;
        validate_gskills_contract("external-record-result", &external)?;
        Ok(external)
    }
}

fn assign_digest(value: &mut Value) -> Result<(), RecordSinkError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "digest-bearing result is not an object".to_owned(),
        })?;
    object.remove("digest");
    let digest = sha256_digest(&*object)?;
    object.insert("digest".to_owned(), Value::String(digest.to_string()));
    Ok(())
}

fn enum_value<T: serde::Serialize>(value: T) -> Result<String, RecordSinkError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| RecordSinkError::InvalidCommand {
            message: "native result status did not serialize as a closed enum".to_owned(),
        })
}
