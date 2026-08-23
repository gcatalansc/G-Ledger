use std::collections::BTreeMap;

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, CheckpointCreatedEvent, Command, CommandId, ContinuityCheckpoint,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Phase, PhaseKind, PhaseStartedEvent,
    PhaseStatus, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectStatusDerivedEvent, ResourceRef, Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent,
    RoadmapExecutionModel, SchemaVersion, StableId, StorageProfile, TrustLevel,
    WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_interop_gskills::{GSkillsProvider, ProviderContext};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;
use serde_json::{Value, json};

const REQUESTED_FIELDS: &[&str] = &[
    "project_state_version",
    "material_state_version",
    "project_state_digest",
    "authority_documents",
    "active_phases",
    "primary_active_phase_ref",
    "verified_completed_phases",
    "current_decisions",
    "open_blockers",
    "open_risks",
    "pending_gates",
    "revalidation_required_gates",
    "stale_gates",
    "next_actions",
    "checkpoint",
    "repository_binding",
    "recent_material_events",
    "last_material_activity_at",
    "unknowns",
    "conflicts",
];

pub fn provider() -> GSkillsProvider {
    let mut stream = Stream::genesis();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: authority(),
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: roadmap(),
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:implementation"),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Initializing,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_STARTED".to_owned()],
    }));
    stream.push(Event::WorkProgressRecorded(WorkProgressRecordedEvent {
        work_record: work(),
    }));
    let before = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let checkpoint_event_ref = event_id();
    let checkpoint = checkpoint(&before, checkpoint_event_ref.clone());
    stream.push_with_id(
        checkpoint_event_ref,
        Event::CheckpointCreated(CheckpointCreatedEvent { checkpoint }),
    );
    let projection = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let phase = Phase {
        schema_version: SchemaVersion::V0_2_0,
        phase_id: id("phase:implementation"),
        title: "Implementation".to_owned(),
        ordinal: 1,
        kind: PhaseKind::Implementation,
        status: PhaseStatus::Active,
        required_gate_refs: Vec::new(),
        optional_gate_refs: Vec::new(),
        started_at: Some(timestamp(4)),
        completed_at: None,
        reopened_at: None,
    };
    GSkillsProvider::new(
        projection,
        ProviderContext {
            gskills_project_ref: "project:mempilot".to_owned(),
            external_project_ref: ResourceRef::parse("project-ledger://projects/mempilot").unwrap(),
            phase_catalog: BTreeMap::from([(phase.phase_id.clone(), phase)]),
            process_instance_id: id("provider-process:conformance"),
            observed_at: timestamp(7),
        },
    )
    .unwrap()
}

pub fn current_request(provider: &GSkillsProvider) -> Value {
    let observable = provider.observable_state();
    let mut request = json!({
        "schema_version": "0.3.0",
        "context_projection_request_id": "projection-request:conformance",
        "request_id": "req:conformance",
        "traceparent": "00-00000000000000000000000000000002-0000000000000002-01",
        "provider_id": "project-ledger",
        "projection_kind": "PROJECT_CONTINUITY",
        "stage": "TASK_CONTEXT",
        "purpose": "Run exact read conformance",
        "identity": {
            "project_ref": "project:mempilot",
            "session_ref": "session:conformance",
            "task_ref": "task:provider-conformance"
        },
        "structural_scope": {"path_refs": [], "symbol_refs": [], "impact_ref": null},
        "requested_fields": REQUESTED_FIELDS,
        "budget": {"max_items": 24, "max_tokens": 350, "max_output_bytes": 65536, "timeout_ms": 1000},
        "privacy": {
            "allowed_fields": ["ids", "titles", "statuses", "digests", "bounded_summaries"],
            "source_bodies": false,
            "raw_events": false
        },
        "consistency": {
            "expected_project_state_version": observable.state_version,
            "expected_material_state_version": observable.material_state_version,
            "expected_project_state_digest": observable.state_digest.to_string(),
            "require_freshness": true,
            "allow_partial": true
        },
        "write_capability": "NONE",
        "requested_at": "2026-08-23T09:07:00-04:00",
        "digest": zero().to_string()
    });
    refresh_digest(&mut request);
    request
}

#[allow(dead_code)] // Shared integration-test support; stdio does not require the stale variant.
pub fn stale_request(provider: &GSkillsProvider) -> Value {
    let mut request = current_request(provider);
    request["context_projection_request_id"] = json!("projection-request:stale");
    request["request_id"] = json!("req:stale");
    request["consistency"]["expected_project_state_version"] =
        json!(provider.observable_state().state_version - 1);
    refresh_digest(&mut request);
    request
}

pub fn refresh_digest(value: &mut Value) {
    value["digest"] = json!(zero().to_string());
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("digest");
    value["digest"] = json!(sha256_digest(&unsigned).unwrap().to_string());
}

struct Stream {
    events: Vec<project_ledger_domain::EventEnvelope>,
}

impl Stream {
    fn genesis() -> Self {
        Self {
            events: vec![event(
                1,
                None,
                EventId::new(),
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        self.push_with_id(EventId::new(), payload);
    }

    fn push_with_id(&mut self, event_id: EventId, payload: Event) {
        self.events.push(event(
            self.events.len() as u64 + 1,
            self.events.last().map(|value| value.event_digest),
            event_id,
            payload,
        ));
    }
}

fn event(
    stream_version: u64,
    previous_event_digest: Option<Sha256Digest>,
    event_id: EventId,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("gskills:conformance:{stream_version}")),
        actor: actor(),
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(stream_version),
    })
    .unwrap();
    build_event(
        &command,
        stream_version,
        previous_event_digest,
        EventDraft {
            event_id,
            event: payload,
            correlation_ref: id("correlation:gskills-conformance"),
            occurred_at: timestamp(stream_version),
            recorded_at: timestamp(stream_version),
        },
    )
    .unwrap()
}

fn checkpoint(
    projection: &ProjectStateProjection,
    checkpoint_event_ref: EventId,
) -> ContinuityCheckpoint {
    let state = projection.state();
    with_digest(
        ContinuityCheckpoint {
            schema_version: SchemaVersion::V0_2_0,
            checkpoint_id: id("checkpoint:conformance"),
            project_ref: state.project.project_id.clone(),
            captured_state_version: state.state_version,
            captured_material_state_version: state.material_state_version,
            captured_state_digest: state.state_digest,
            checkpoint_event_version: state.state_version + 1,
            checkpoint_event_ref,
            authority_document_refs: vec![id("authority:spec")],
            active_phase_refs: vec![id("phase:implementation")],
            primary_active_phase_ref: Some(id("phase:implementation")),
            verified_completed_phase_refs: Vec::new(),
            pending_gate_refs: Vec::new(),
            current_decision_refs: Vec::new(),
            open_blocker_refs: Vec::new(),
            open_risk_refs: Vec::new(),
            next_action_refs: Vec::new(),
            last_material_event_refs: state.last_material_event_ref.iter().cloned().collect(),
            repository_binding: None,
            unknown_refs: Vec::new(),
            conflict_refs: Vec::new(),
            created_by: id("actor:test"),
            created_at: timestamp(7),
            checkpoint_digest: zero(),
        },
        |value, digest| value.checkpoint_digest = digest,
    )
}

fn work() -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:conformance"),
            summary: "Exact provider ready for conformance".to_owned(),
            phase_ref: Some(id("phase:implementation")),
            task_ref: Some(id("task:provider-conformance")),
            files_or_artifacts: Vec::new(),
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(6),
            recorded_at: timestamp(6),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/conformance")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:spec"),
        uri: ResourceRef::parse("workspace://PROJECT_LEDGER_SPEC_v0.2.0.md").unwrap(),
        digest: Sha256Digest::from([1; 32]),
        version: "0.2.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: timestamp(2),
        supersedes_ref: None,
    }
}

fn roadmap() -> Roadmap {
    Roadmap {
        schema_version: SchemaVersion::V0_2_0,
        roadmap_id: id("roadmap:main"),
        title: "Main roadmap".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:spec"),
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "MemPilot".to_owned(),
        summary: "Conformance fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(1),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn with_digest<T: Clone + Serialize>(mut value: T, set: impl Fn(&mut T, Sha256Digest)) -> T {
    set(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("digest");
    object.remove("checkpoint_digest");
    set(&mut value, sha256_digest(&json).unwrap());
    value
}

fn actor() -> Actor {
    Actor {
        actor_id: id("actor:test"),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn event_id() -> EventId {
    EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7d777").unwrap()
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T09:{minute:02}:00-04:00")).unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}
