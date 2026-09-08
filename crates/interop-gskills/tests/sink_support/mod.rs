#![allow(dead_code)] // Shared integration-test support is compiled once per test crate.

use std::cell::{Cell, RefCell};

use project_ledger_application::{CommandProcessor, CoreCommandHandler, FixedClock};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult,
    CommandResultStatus, CreateProjectPayload, LedgerTimestamp, Project, ProjectId,
    ProjectLifecycleStatus, ResourceRef, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventStore, prepare_command};
use project_ledger_interop_gskills::{
    GSKILLS_PROFILE, RecordSinkHost, RecordSinkHostError, RecordSinkPolicy, RecordSinkStateIdentity,
};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::{Value, json};
use tempfile::TempDir;

pub const PROJECT_ID: &str = "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
pub const COMMAND_UUID: &str = "0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const PROVIDER_MANIFEST_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PROVIDER_EXECUTABLE_DIGEST: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const COMPATIBILITY_LOCK_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const IDENTITY_MAP_DIGEST: &str =
    "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

pub struct ProcessingHost {
    _directory: TempDir,
    store: RefCell<SqliteEventStore>,
    execute_calls: Cell<u64>,
    lose_next_response: Cell<bool>,
    state_override: RefCell<Option<RecordSinkStateIdentity>>,
}

impl ProcessingHost {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("ledger.db");
        let mut store = SqliteEventStore::open(&database_path).unwrap();
        let created = execute_native(
            &mut store,
            prepare_command(CommandDraft {
                schema_version: SchemaVersion::V0_2_0,
                command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df50")
                    .unwrap(),
                project_ref: project_id(),
                expected_state_version: 0,
                idempotency_key: id("test:create-project"),
                actor: human_actor(),
                command: Command::CreateProject(CreateProjectPayload { project: project() }),
                submitted_at: timestamp("2026-08-23T10:00:00-04:00"),
            })
            .unwrap(),
        );
        assert_eq!(created.status, CommandResultStatus::Accepted);
        Self {
            _directory: directory,
            store: RefCell::new(store),
            execute_calls: Cell::new(0),
            lose_next_response: Cell::new(false),
            state_override: RefCell::new(None),
        }
    }

    pub fn execute_calls(&self) -> u64 {
        self.execute_calls.get()
    }

    pub fn event_count(&self) -> u64 {
        self.store.borrow().event_count(&project_id()).unwrap()
    }

    pub fn current_state(&self) -> RecordSinkStateIdentity {
        projection_state(&self.store.borrow())
    }

    pub fn lose_next_response(&self) {
        self.lose_next_response.set(true);
    }

    pub fn override_reported_state(&self, state: RecordSinkStateIdentity) {
        self.state_override.replace(Some(state));
    }

    pub fn advance_out_of_band(&self) {
        let state = self.current_state();
        let payload =
            serde_json::from_value(work_progress_command(state)["payload"].clone()).unwrap();
        let command = prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df51").unwrap(),
            project_ref: project_id(),
            expected_state_version: state.state_version,
            idempotency_key: id("test:competing-progress"),
            actor: integration_actor(),
            command: Command::RecordWorkProgress(payload),
            submitted_at: timestamp("2026-08-23T10:01:00-04:00"),
        })
        .unwrap();
        let result = execute_native(&mut self.store.borrow_mut(), command);
        assert_eq!(
            result.status,
            CommandResultStatus::Accepted,
            "out-of-band fixture rejection: {:?}",
            result.rejection
        );
    }

    pub fn clear_reported_state_override(&self) {
        self.state_override.replace(None);
    }
}

impl RecordSinkHost for ProcessingHost {
    fn observed_at(&self) -> LedgerTimestamp {
        timestamp("2026-08-23T10:02:00-04:00")
    }

    fn state_identity(
        &self,
        project_ref: &ProjectId,
    ) -> Result<RecordSinkStateIdentity, RecordSinkHostError> {
        if project_ref != &project_id() {
            return Err(RecordSinkHostError::new("unexpected project"));
        }
        if let Some(state) = *self.state_override.borrow() {
            return Ok(state);
        }
        Ok(projection_state(&self.store.borrow()))
    }

    fn execute(&self, command: CommandEnvelope) -> Result<CommandResult, RecordSinkHostError> {
        self.execute_calls.set(self.execute_calls.get() + 1);
        let result = execute_native(&mut self.store.borrow_mut(), command);
        if self.lose_next_response.replace(false) {
            return Err(RecordSinkHostError::new("TIMEOUT_AFTER_SEND"));
        }
        Ok(result)
    }

    fn command_result(
        &self,
        command_ref: &CommandId,
    ) -> Result<Option<(CommandEnvelope, CommandResult)>, RecordSinkHostError> {
        let store = self.store.borrow();
        let Some(result) = store
            .command_result(command_ref)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))?
        else {
            return Ok(None);
        };
        let command = store
            .recorded_command(command_ref)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))?
            .ok_or_else(|| RecordSinkHostError::new("durable result has no recorded command"))?;
        Ok(Some((command, result)))
    }

    fn confirmed_projection_ref(
        &self,
        project_ref: &ProjectId,
        state_version: u64,
    ) -> Result<ResourceRef, RecordSinkHostError> {
        if project_ref != &project_id() {
            return Err(RecordSinkHostError::new("unexpected project"));
        }
        let state = projection_state(&self.store.borrow());
        if state.state_version != state_version {
            return Err(RecordSinkHostError::new(format!(
                "projection version mismatch: expected {state_version}, actual {}",
                state.state_version
            )));
        }
        ResourceRef::parse(&format!("continuity:ledger-{state_version}"))
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }
}

pub fn policy() -> RecordSinkPolicy {
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
            "trust_level": "TRUSTED_FOR_RECORDING",
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

pub fn work_progress_command(state: RecordSinkStateIdentity) -> Value {
    command_with_identity(
        state,
        COMMAND_UUID,
        "work:gskills-s10",
        "First exact effect",
    )
}

pub fn unknown_command(state: RecordSinkStateIdentity) -> Value {
    command_with_identity(
        state,
        "0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e",
        "work:gskills-unknown",
        "Not submitted",
    )
}

pub fn change_work_summary_and_resign(value: &mut Value, summary: &str) {
    value["payload"]["work_record"]["summary"] = json!(summary);
    refresh_outer_digest(&mut value["payload"]["work_record"]);
    refresh_prepared_binding_and_digest(value);
}

fn command_with_identity(
    state: RecordSinkStateIdentity,
    command_uuid: &str,
    work_record_id: &str,
    summary: &str,
) -> Value {
    let policy = policy();
    let mut work_record = json!({
        "schema_version": "0.2.0",
        "work_record_id": work_record_id,
        "summary": summary,
        "phase_ref": null,
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
    let mut value = json!({
        "schema_version": "0.3.0",
        "external_record_command_id": format!("external-command:{command_uuid}"),
        "command_kind": "RECORD_WORK_PROGRESS",
        "sink_id": "project-ledger",
        "profile": GSKILLS_PROFILE,
        "project_ref": "project:mempilot",
        "external_project_ref": "project-ledger://projects/mempilot",
        "actor": {"type":"agent_harness","id":"codex-session-44","on_behalf_of":"human:local-user"},
        "session_ref": "session:44",
        "native_command": {
            "command_type": "RECORD_WORK_PROGRESS",
            "command_schema_ref": "urn:project-ledger:command:record-work-progress:0.2.0",
            "command_schema_ref_status": "EXACT",
            "expected_state_version": state.state_version
        },
        "expected_state": {
            "project_state_version": state.state_version,
            "material_state_version": state.material_state_version,
            "project_state_digest": state.state_digest
        },
        "payload": {"work_record": work_record},
        "idempotency_key": "gskills:outcome:982:ledger-record",
        "idempotency_scope": {
            "sink_id":"project-ledger",
            "external_project_ref":"project-ledger://projects/mempilot",
            "native_command_type":"RECORD_WORK_PROGRESS",
            "actor_trust_domain":"agent_harness:on_behalf_of_local_user"
        },
        "preparation": {
            "prepared_by":"gskills",
            "prepared_at":"2026-08-23T10:01:00-04:00",
            "prepared_command_digest":zero_digest(),
            "preview_digest":zero_digest(),
            "provider_manifest_digest":PROVIDER_MANIFEST_DIGEST,
            "provider_executable_digest":PROVIDER_EXECUTABLE_DIGEST,
            "compatibility_lock_digest":COMPATIBILITY_LOCK_DIGEST,
            "policy_digest":policy.digest,
            "identity_map_digest":IDENTITY_MAP_DIGEST,
            "expected_state_digest":state.state_digest,
            "approval_requirement":"USER_OR_POLICY",
            "approval_expires_at":"2026-08-23T10:06:00-04:00"
        },
        "privacy":{"raw_output_included":false,"evidence_refs_only":true},
        "provenance_ref":"gskills://provenance/external-command-0198",
        "digest":zero_digest()
    });
    refresh_prepared_binding_and_digest(&mut value);
    value
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

fn projection_state(store: &SqliteEventStore) -> RecordSinkStateIdentity {
    let stream = store.load_stream(&project_id()).unwrap();
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    RecordSinkStateIdentity {
        state_version: projection.state().state_version,
        material_state_version: projection.state().material_state_version,
        state_digest: projection.state().state_digest,
    }
}

fn execute_native(store: &mut SqliteEventStore, command: CommandEnvelope) -> CommandResult {
    CommandProcessor::new(
        store,
        &FixedClock::new(timestamp("2026-08-23T10:02:00-04:00")),
    )
    .execute(command, &CoreCommandHandler)
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "RecordSink test".to_owned(),
        summary: "Real native processing host".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp("2026-08-23T10:00:00-04:00"),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn human_actor() -> Actor {
    Actor {
        actor_id: id("human:test"),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn integration_actor() -> Actor {
    Actor {
        actor_id: id("integration:gskills"),
        actor_kind: ActorKind::Integration,
        trust_level: TrustLevel::TrustedForRecording,
        display_name: Some("G-Skills RecordSink".to_owned()),
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse(PROJECT_ID).unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn timestamp(value: &str) -> LedgerTimestamp {
    LedgerTimestamp::parse(value).unwrap()
}

fn zero_digest() -> String {
    Sha256Digest::from([0; 32]).to_string()
}
