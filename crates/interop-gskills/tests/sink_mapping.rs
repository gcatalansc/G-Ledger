use std::cell::Cell;

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Command, CommandEnvelope, CommandId, CommandResult, CommandType, ProjectId, ResourceRef,
};
use project_ledger_interop_gskills::{
    GSKILLS_PROFILE, GSkillsRecordSink, RecordSinkError, RecordSinkHost, RecordSinkHostError,
    RecordSinkPolicy, RecordSinkStateIdentity,
};
use serde_json::{Value, json};

const PROJECT_ID: &str = "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const COMMAND_UUID: &str = "0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const STATE_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PROVIDER_MANIFEST_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PROVIDER_EXECUTABLE_DIGEST: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const COMPATIBILITY_LOCK_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const IDENTITY_MAP_DIGEST: &str =
    "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

#[test]
fn projection_trust_never_grants_recording_and_validate_has_no_write_side_effect() {
    let host = CountingHost::new();
    let projection_policy = policy("TRUSTED_FOR_PROJECTION");

    assert!(matches!(
        GSkillsRecordSink::new(&host, projection_policy),
        Err(RecordSinkError::WriteTrustRequired)
    ));
    assert_eq!(host.execute_calls.get(), 0);

    let sink = GSkillsRecordSink::new(&host, policy("TRUSTED_FOR_RECORDING")).unwrap();
    let prepared = sink
        .validate(&command(
            "RECORD_WORK_PROGRESS",
            "urn:project-ledger:command:record-work-progress:0.2.0",
            &work_progress_payload(),
        ))
        .unwrap();

    assert_eq!(
        prepared.command.command_type(),
        CommandType::RecordWorkProgress
    );
    assert_eq!(prepared.expected_state_version, 1);
    assert_eq!(
        prepared.command_id.as_str(),
        format!("command:{COMMAND_UUID}")
    );
    assert_eq!(host.execute_calls.get(), 0);
}

#[test]
fn exactly_five_capabilities_map_to_closed_native_payloads() {
    let host = CountingHost::new();
    let sink = GSkillsRecordSink::new(&host, policy("TRUSTED_FOR_RECORDING")).unwrap();
    let cases = [
        (
            "RECORD_WORK_PROGRESS",
            "urn:project-ledger:command:record-work-progress:0.2.0",
            work_progress_payload(),
            CommandType::RecordWorkProgress,
        ),
        (
            "RECORD_VERIFICATION_RESULT",
            "urn:project-ledger:command:record-verification-result:0.2.0",
            verification_payload(),
            CommandType::RecordVerificationResult,
        ),
        (
            "REQUEST_GATE_EVALUATION",
            "urn:project-ledger:command:request-gate-evaluation:0.2.0",
            json!({"gate_ref": "gate:sqlite-roundtrip"}),
            CommandType::RequestGateEvaluation,
        ),
        (
            "REQUEST_GATE_REVALIDATION",
            "urn:project-ledger:command:request-gate-revalidation:0.2.0",
            json!({"signal": gate_revalidation_signal()}),
            CommandType::RequestGateRevalidation,
        ),
        (
            "CREATE_CONTINUITY_CHECKPOINT",
            "urn:project-ledger:command:create-continuity-checkpoint:0.2.0",
            json!({"reason": "Close the exact S10 compatibility slice"}),
            CommandType::CreateContinuityCheckpoint,
        ),
    ];

    for (kind, schema_ref, payload, expected) in cases {
        let prepared = sink.validate(&command(kind, schema_ref, &payload)).unwrap();
        assert_eq!(prepared.command.command_type(), expected);
        assert_eq!(prepared.command_schema_ref.as_str(), schema_ref);
        assert_eq!(prepared.project_ref.as_str(), PROJECT_ID);
        let envelope = prepared.native_envelope().unwrap();
        assert_eq!(envelope.command_type(), expected);
        assert_eq!(envelope.command_schema_ref.as_str(), schema_ref);
    }
    assert_eq!(host.execute_calls.get(), 0);
}

#[test]
fn legacy_or_lifecycle_capabilities_and_schema_analogy_are_denied() {
    let host = CountingHost::new();
    let sink = GSkillsRecordSink::new(&host, policy("TRUSTED_FOR_RECORDING")).unwrap();

    for forbidden in [
        "RECORD_DECISION_REFERENCE",
        "RECORD_BLOCKER_REFERENCE",
        "REQUEST_PHASE_COMPLETION",
    ] {
        let rejected = command(
            forbidden,
            "urn:project-ledger:command:record-work-progress:0.2.0",
            &work_progress_payload(),
        );
        assert!(matches!(
            sink.validate(&rejected),
            Err(RecordSinkError::UnsupportedCapability { .. })
        ));
    }

    let wrong_schema = command(
        "RECORD_WORK_PROGRESS",
        "urn:project-ledger:command:record-verification-result:0.2.0",
        &work_progress_payload(),
    );
    assert!(matches!(
        sink.validate(&wrong_schema),
        Err(RecordSinkError::SchemaReferenceMismatch { .. })
    ));
    assert_eq!(host.execute_calls.get(), 0);
}

#[test]
fn identity_state_scope_privacy_and_approval_binding_fail_closed() {
    let host = CountingHost::new();
    let sink = GSkillsRecordSink::new(&host, policy("TRUSTED_FOR_RECORDING")).unwrap();
    let valid = command(
        "RECORD_WORK_PROGRESS",
        "urn:project-ledger:command:record-work-progress:0.2.0",
        &work_progress_payload(),
    );

    let mut stale = valid.clone();
    stale["expected_state"]["project_state_version"] = json!(0);
    refresh_prepared_binding_and_digest(&mut stale);
    assert!(matches!(
        sink.validate(&stale),
        Err(RecordSinkError::StateMismatch { .. })
    ));

    let mut wrong_scope = valid.clone();
    wrong_scope["idempotency_scope"]["actor_trust_domain"] = json!("projection-only");
    refresh_prepared_binding_and_digest(&mut wrong_scope);
    assert!(matches!(
        sink.validate(&wrong_scope),
        Err(RecordSinkError::IdentityMismatch { .. })
    ));

    let mut raw_output = valid.clone();
    raw_output["privacy"]["raw_output_included"] = json!(true);
    refresh_prepared_binding_and_digest(&mut raw_output);
    assert!(matches!(
        sink.validate(&raw_output),
        Err(RecordSinkError::PrivacyViolation { .. })
    ));

    let mut approval_drift = valid;
    approval_drift["payload"]["work_record"]["summary"] = json!("Changed after approval");
    refresh_outer_digest(&mut approval_drift);
    assert!(matches!(
        sink.validate(&approval_drift),
        Err(RecordSinkError::PreparedCommandDigestMismatch)
    ));

    let mut expired = command(
        "RECORD_WORK_PROGRESS",
        "urn:project-ledger:command:record-work-progress:0.2.0",
        &work_progress_payload(),
    );
    expired["preparation"]["approval_expires_at"] = json!("2026-08-23T10:01:30-04:00");
    refresh_prepared_binding_and_digest(&mut expired);
    assert!(matches!(
        sink.validate(&expired),
        Err(RecordSinkError::ApprovalBindingMismatch {
            field: "approval_expires_at"
        })
    ));
    assert_eq!(host.execute_calls.get(), 0);
}

struct CountingHost {
    execute_calls: Cell<u64>,
}

impl CountingHost {
    fn new() -> Self {
        Self {
            execute_calls: Cell::new(0),
        }
    }
}

impl RecordSinkHost for CountingHost {
    fn observed_at(&self) -> project_ledger_domain::LedgerTimestamp {
        project_ledger_domain::LedgerTimestamp::parse("2026-08-23T10:02:00-04:00").unwrap()
    }

    fn state_identity(
        &self,
        project_ref: &ProjectId,
    ) -> Result<RecordSinkStateIdentity, RecordSinkHostError> {
        assert_eq!(project_ref.as_str(), PROJECT_ID);
        Ok(RecordSinkStateIdentity {
            state_version: 1,
            material_state_version: 1,
            state_digest: Sha256Digest::parse(STATE_DIGEST).unwrap(),
        })
    }

    fn execute(&self, _command: CommandEnvelope) -> Result<CommandResult, RecordSinkHostError> {
        self.execute_calls.set(self.execute_calls.get() + 1);
        Err(RecordSinkHostError::new(
            "execute is forbidden during validation",
        ))
    }

    fn command_result(
        &self,
        _command_ref: &CommandId,
    ) -> Result<Option<CommandResult>, RecordSinkHostError> {
        Ok(None)
    }

    fn confirmed_projection_ref(
        &self,
        _project_ref: &ProjectId,
        _state_version: u64,
    ) -> Result<ResourceRef, RecordSinkHostError> {
        ResourceRef::parse("project-ledger://continuity/ledger-1")
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }
}

fn policy(write_trust: &str) -> RecordSinkPolicy {
    let mut value = json!({
        "schema_version": "0.2.0",
        "policy_id": "policy:gskills-record-sink",
        "sink_id": "project-ledger",
        "profile": GSKILLS_PROFILE,
        "gskills_project_ref": "project:mempilot",
        "external_project_ref": "project-ledger://projects/mempilot",
        "native_project_ref": PROJECT_ID,
        "external_actor": {
            "type": "agent_harness",
            "id": "codex-session-44",
            "on_behalf_of": "human:local-user"
        },
        "native_actor": {
            "actor_id": "integration:gskills",
            "actor_kind": "INTEGRATION",
            "trust_level": write_trust,
            "display_name": "G-Skills RecordSink"
        },
        "actor_trust_domain": "agent_harness:on_behalf_of_local_user",
        "allowed_command_kinds": [
            "RECORD_WORK_PROGRESS",
            "RECORD_VERIFICATION_RESULT",
            "REQUEST_GATE_EVALUATION",
            "REQUEST_GATE_REVALIDATION",
            "CREATE_CONTINUITY_CHECKPOINT"
        ],
        "approval_requirement": "USER_OR_POLICY",
        "provider_manifest_digest": PROVIDER_MANIFEST_DIGEST,
        "provider_executable_digest": PROVIDER_EXECUTABLE_DIGEST,
        "compatibility_lock_digest": COMPATIBILITY_LOCK_DIGEST,
        "identity_map_digest": IDENTITY_MAP_DIGEST,
        "created_at": "2026-08-23T10:00:00-04:00",
        "digest": zero_digest()
    });
    refresh_outer_digest(&mut value);
    RecordSinkPolicy::from_value(&value).unwrap()
}

fn command(kind: &str, schema_ref: &str, payload: &Value) -> Value {
    let policy = policy("TRUSTED_FOR_RECORDING");
    let mut value = json!({
        "schema_version": "0.3.0",
        "external_record_command_id": format!("external-command:{COMMAND_UUID}"),
        "command_kind": kind,
        "sink_id": "project-ledger",
        "profile": GSKILLS_PROFILE,
        "project_ref": "project:mempilot",
        "external_project_ref": "project-ledger://projects/mempilot",
        "actor": {
            "type": "agent_harness",
            "id": "codex-session-44",
            "on_behalf_of": "human:local-user"
        },
        "session_ref": "session:44",
        "native_command": {
            "command_type": kind,
            "command_schema_ref": schema_ref,
            "command_schema_ref_status": "EXACT",
            "expected_state_version": 1
        },
        "expected_state": {
            "project_state_version": 1,
            "material_state_version": 1,
            "project_state_digest": STATE_DIGEST
        },
        "payload": payload,
        "idempotency_key": "gskills:outcome:982:ledger-record",
        "idempotency_scope": {
            "sink_id": "project-ledger",
            "external_project_ref": "project-ledger://projects/mempilot",
            "native_command_type": kind,
            "actor_trust_domain": "agent_harness:on_behalf_of_local_user"
        },
        "preparation": {
            "prepared_by": "gskills",
            "prepared_at": "2026-08-23T10:01:00-04:00",
            "prepared_command_digest": zero_digest(),
            "preview_digest": zero_digest(),
            "provider_manifest_digest": PROVIDER_MANIFEST_DIGEST,
            "provider_executable_digest": PROVIDER_EXECUTABLE_DIGEST,
            "compatibility_lock_digest": COMPATIBILITY_LOCK_DIGEST,
            "policy_digest": policy.digest.to_string(),
            "identity_map_digest": IDENTITY_MAP_DIGEST,
            "expected_state_digest": STATE_DIGEST,
            "approval_requirement": "USER_OR_POLICY",
            "approval_expires_at": "2026-08-23T10:06:00-04:00"
        },
        "privacy": {
            "raw_output_included": false,
            "evidence_refs_only": true
        },
        "provenance_ref": "gskills://provenance/external-command-0198",
        "digest": zero_digest()
    });
    refresh_prepared_binding_and_digest(&mut value);
    value
}

fn work_progress_payload() -> Value {
    let mut work_record = json!({
        "schema_version": "0.2.0",
        "work_record_id": "work:gskills-s10",
        "summary": "Implement exact G-Skills RecordSink mapping",
        "phase_ref": "phase:7",
        "task_ref": "task:s10",
        "files_or_artifacts": ["gskills://outcomes/982"],
        "commit_refs": [],
        "claim_refs": [],
        "evidence_refs": [],
        "actor_ref": "integration:gskills",
        "influence": "External bounded progress record",
        "occurred_at": "2026-08-23T09:59:00-04:00",
        "recorded_at": "2026-08-23T10:01:00-04:00",
        "provenance_ref": "gskills://provenance/outcome-982",
        "digest": zero_digest()
    });
    refresh_outer_digest(&mut work_record);
    json!({"work_record": work_record})
}

fn verification_payload() -> Value {
    json!({
        "evidence": {
            "schema_version": "0.2.0",
            "evidence_id": "evidence:gskills-s10",
            "project_ref": PROJECT_ID,
            "kind": "TEST_RESULT",
            "status": "OBSERVED",
            "source": {
                "source_type": "EXTERNAL_SYSTEM",
                "tool": "gskills",
                "command_display": null,
                "working_directory_ref": null,
                "runner_version": "0.3.0"
            },
            "result": {
                "exit_code": 0,
                "duration_ms": 100,
                "metrics": {"passed": 24, "failed": 0, "skipped": 0, "errors": 0},
                "summary": "G-Skills verification result"
            },
            "repository_binding": null,
            "artifacts": [],
            "privacy": {"raw_output_stored": false, "redaction_applied": false},
            "observed_at": "2026-08-23T10:00:00-04:00",
            "payload_digest": "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "provenance_ref": "gskills://provenance/verification-982"
        }
    })
}

fn gate_revalidation_signal() -> Value {
    let mut signal = json!({
        "schema_version": "0.3.0",
        "gate_revalidation_signal_id": "gate-revalidation:0198",
        "project_ref": "project:mempilot",
        "gate_ref": "gate:sqlite-roundtrip",
        "reason_codes": ["REPOSITORY_DEPENDENCY_CHANGED"],
        "previous_verification_refs": ["project-ledger://gates/sqlite-roundtrip/evaluations/18"],
        "previous_repository_snapshot_ref": "snapshot:0197",
        "current_repository_snapshot_ref": "snapshot:0198",
        "structural_impact_ref": "impact:0198",
        "matched_dependency_refs": ["symbol:storage.SqliteRepository"],
        "confidence_status": "STRUCTURAL_SUPPORTED",
        "recommended_command_kind": "REQUEST_GATE_REVALIDATION",
        "severity": "HIGH",
        "provenance_ref": "gskills://provenance/gate-revalidation-0198",
        "digest": zero_digest()
    });
    refresh_outer_digest(&mut signal);
    signal
}

fn refresh_prepared_binding_and_digest(value: &mut Value) {
    let core_digest = prepared_core_digest(value);
    value["preparation"]["prepared_command_digest"] = json!(core_digest.to_string());
    value["preparation"]["preview_digest"] = json!(core_digest.to_string());
    refresh_outer_digest(value);
}

fn prepared_core_digest(value: &Value) -> Sha256Digest {
    let mut core = value.clone();
    let object = core.as_object_mut().unwrap();
    object.remove("digest");
    let preparation = object["preparation"].as_object_mut().unwrap();
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
    sha256_digest(&core).unwrap()
}

fn refresh_outer_digest(value: &mut Value) {
    let object = value.as_object_mut().unwrap();
    object.remove("digest");
    let digest = sha256_digest(&*object).unwrap();
    object.insert("digest".to_owned(), json!(digest.to_string()));
}

fn zero_digest() -> String {
    Sha256Digest::from([0; 32]).to_string()
}

#[allow(dead_code)]
fn _assert_native_command_is_closed(command: &Command) -> CommandType {
    command.command_type()
}
