use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, CheckpointCreatedEvent, CheckpointFreshnessStatus,
    CheckpointInvalidatedEvent, Command, CommandId, ContinuityCheckpoint, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    RecordWorkProgressPayload, ResourceRef, SchemaVersion, StableId, StorageProfile, TrustLevel,
    UnknownCriticality, UnknownRecord, UnknownRecordedEvent, UnknownResolvedEvent, UnknownStatus,
    WorkProgressRecordedEvent, WorkRecord,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::{ProjectStateProjection, ProjectionError};
use serde::Serialize;

#[test]
fn ac_027_checkpoint_captures_n_and_registers_n_plus_one_without_material_advance() {
    let mut stream = Stream::genesis();
    let before = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let checkpoint_event_ref = event_id(20);
    let first_checkpoint = checkpoint(
        &before,
        id("checkpoint:first"),
        checkpoint_event_ref.clone(),
    );
    stream.push_with_id(
        checkpoint_event_ref,
        Event::CheckpointCreated(CheckpointCreatedEvent {
            checkpoint: first_checkpoint.clone(),
        }),
    );

    let projected = ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(projected.state().state_version, 2);
    assert_eq!(projected.state().material_state_version, 1);
    assert_eq!(
        projected.state().checkpoint_ref,
        Some(first_checkpoint.checkpoint_id.clone())
    );
    assert_eq!(
        projected.checkpoint(&first_checkpoint.checkpoint_id),
        Some(&first_checkpoint)
    );
    assert_eq!(
        projected.checkpoint_freshness(&first_checkpoint.checkpoint_id),
        Some(CheckpointFreshnessStatus::CurrentForProjectOnly)
    );
    assert_eq!(projected.checkpoint_freshness_records().len(), 1);

    stream.push(Event::CheckpointInvalidated(CheckpointInvalidatedEvent {
        checkpoint_ref: first_checkpoint.checkpoint_id.clone(),
        freshness: CheckpointFreshnessStatus::Invalidated,
        reason_codes: vec!["MATERIAL_CHANGE".to_owned()],
    }));
    let invalidated = ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(invalidated.state().state_version, 3);
    assert_eq!(invalidated.state().material_state_version, 1);
    assert_eq!(
        invalidated.checkpoint(&first_checkpoint.checkpoint_id),
        Some(&first_checkpoint)
    );
    assert_eq!(
        invalidated.checkpoint_freshness(&first_checkpoint.checkpoint_id),
        Some(CheckpointFreshnessStatus::Invalidated)
    );

    let second_event_ref = event_id(30);
    let second = checkpoint(
        &invalidated,
        id("checkpoint:second"),
        second_event_ref.clone(),
    );
    stream.push_with_id(
        second_event_ref,
        Event::CheckpointCreated(CheckpointCreatedEvent {
            checkpoint: second.clone(),
        }),
    );
    let rebuilt = ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(rebuilt.state().state_version, 4);
    assert_eq!(rebuilt.state().material_state_version, 1);
    assert_eq!(
        rebuilt.state().checkpoint_ref,
        Some(second.checkpoint_id.clone())
    );
    assert_eq!(
        rebuilt.checkpoint(&first_checkpoint.checkpoint_id),
        Some(&first_checkpoint)
    );
    assert_eq!(rebuilt.checkpoint(&second.checkpoint_id), Some(&second));
    assert_eq!(
        rebuilt.checkpoint_freshness(&first_checkpoint.checkpoint_id),
        Some(CheckpointFreshnessStatus::SupersededByNewerCheckpoint)
    );
    assert_eq!(
        rebuilt.checkpoint_freshness(&second.checkpoint_id),
        Some(CheckpointFreshnessStatus::CurrentForProjectOnly)
    );
    assert_eq!(rebuilt.checkpoint_freshness_records().len(), 4);
}

#[test]
fn checkpoint_rejects_forged_capture_digest_event_ref_and_snapshot_vectors() {
    let cases = ["digest", "event_ref", "version", "vectors"];
    for (index, mutation) in cases.into_iter().enumerate() {
        let mut stream = Stream::genesis();
        let before = ProjectStateProjection::rebuild(&stream.events).unwrap();
        let actual_event_ref = event_id(u8::try_from(40 + index).unwrap());
        let mut forged = checkpoint(
            &before,
            id(&format!("checkpoint:forged-{mutation}")),
            actual_event_ref.clone(),
        );
        match mutation {
            "digest" => forged.captured_state_digest = Sha256Digest::from([9_u8; 32]),
            "event_ref" => forged.checkpoint_event_ref = event_id(99),
            "version" => forged.checkpoint_event_version += 1,
            "vectors" => forged.pending_gate_refs.push(id("gate:invented")),
            _ => unreachable!(),
        }
        forged.checkpoint_digest = zero();
        forged = with_digest(forged, |value, digest| value.checkpoint_digest = digest);
        stream.push_with_id(
            actual_event_ref,
            Event::CheckpointCreated(CheckpointCreatedEvent { checkpoint: forged }),
        );
        assert!(matches!(
            ProjectStateProjection::rebuild(&stream.events),
            Err(ProjectionError::LifecycleInvariant { .. })
        ));
    }
}

#[test]
fn work_progress_and_material_unknowns_rebuild_then_resolve_without_history_loss() {
    let mut stream = Stream::genesis();
    let work = with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:s6-red"),
            summary: "checkpoint projection tests written".to_owned(),
            phase_ref: None,
            task_ref: None,
            files_or_artifacts: vec![ResourceRef::parse("workspace://project/tests").unwrap()],
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(2),
            recorded_at: timestamp(2),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/s6-red").unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    );
    stream.push(Event::WorkProgressRecorded(WorkProgressRecordedEvent {
        work_record: work.clone(),
    }));
    let unknown = with_digest(
        UnknownRecord {
            schema_version: SchemaVersion::V0_2_0,
            unknown_id: id("unknown:critical-contract"),
            subject_kind: "CHECKPOINT".to_owned(),
            subject_ref: id("checkpoint:future"),
            field_path: "repository_binding".to_owned(),
            statement: "repository snapshot is not confirmed".to_owned(),
            criticality: UnknownCriticality::Material,
            status: UnknownStatus::Open,
            blocks: vec![id("phase:continuity")],
            required_evidence: vec!["repository snapshot".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: timestamp(3),
            resolved_at: None,
            resolution_ref: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/unknown/critical")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    );
    stream.push(Event::UnknownRecorded(UnknownRecordedEvent {
        unknown: unknown.clone(),
    }));

    let open = ProjectStateProjection::rebuild(&stream.events).unwrap();
    assert_eq!(open.latest_work_record(), Some(&work));
    assert_eq!(open.unknown(&unknown.unknown_id), Some(&unknown));
    assert_eq!(
        open.critical_unknown_refs(),
        vec![unknown.unknown_id.clone()]
    );
    assert_eq!(open.state().material_state_version, 3);

    let resolution = ResourceRef::parse("project-ledger://evidence/repository-confirmed").unwrap();
    stream.push(Event::UnknownResolved(UnknownResolvedEvent {
        unknown_ref: unknown.unknown_id.clone(),
        resolution_ref: resolution.clone(),
    }));
    let resolved = ProjectStateProjection::rebuild(&stream.events).unwrap();
    let resolved_unknown = resolved.unknown(&unknown.unknown_id).unwrap();
    assert_eq!(resolved_unknown.status, UnknownStatus::Resolved);
    assert_eq!(resolved_unknown.resolution_ref, Some(resolution));
    assert_eq!(resolved_unknown.resolved_at, Some(timestamp(4)));
    assert!(resolved.critical_unknown_refs().is_empty());
    assert_eq!(resolved.latest_work_record(), Some(&work));
    assert_eq!(resolved.state().material_state_version, 4);
}

fn checkpoint(
    projection: &ProjectStateProjection,
    checkpoint_id: StableId,
    checkpoint_event_ref: EventId,
) -> ContinuityCheckpoint {
    let state = projection.state();
    with_digest(
        ContinuityCheckpoint {
            schema_version: SchemaVersion::V0_2_0,
            checkpoint_id,
            project_ref: state.project.project_id.clone(),
            captured_state_version: state.state_version,
            captured_material_state_version: state.material_state_version,
            captured_state_digest: state.state_digest,
            checkpoint_event_version: state.state_version + 1,
            checkpoint_event_ref,
            authority_document_refs: Vec::new(),
            active_phase_refs: state.active_phase_refs.clone(),
            primary_active_phase_ref: state.primary_active_phase_ref.clone(),
            verified_completed_phase_refs: Vec::new(),
            pending_gate_refs: state.pending_gate_refs.clone(),
            current_decision_refs: state.current_decision_refs.clone(),
            open_blocker_refs: state.open_blocker_refs.clone(),
            open_risk_refs: state.open_risk_refs.clone(),
            next_action_refs: state.ordered_next_action_refs.clone(),
            last_material_event_refs: state.last_material_event_ref.iter().cloned().collect(),
            repository_binding: None,
            unknown_refs: Vec::new(),
            conflict_refs: state.open_conflict_refs.clone(),
            created_by: id("actor:test"),
            created_at: timestamp(state.state_version + 1),
            checkpoint_digest: zero(),
        },
        |value, digest| value.checkpoint_digest = digest,
    )
}

struct Stream {
    events: Vec<project_ledger_domain::EventEnvelope>,
}

impl Stream {
    fn genesis() -> Self {
        Self {
            events: vec![event(
                1,
                event_id(1),
                None,
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        self.push_with_id(
            event_id(u8::try_from(self.events.len() + 1).unwrap()),
            payload,
        );
    }

    fn push_with_id(&mut self, event_id: EventId, payload: Event) {
        let envelope = event(
            self.events.len() as u64 + 1,
            event_id,
            self.events.last().map(|event| event.event_digest),
            payload,
        );
        self.events.push(envelope);
    }
}

fn event(
    stream_version: u64,
    event_id: EventId,
    previous_event_digest: Option<Sha256Digest>,
    payload: Event,
) -> project_ledger_domain::EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(&format!(
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7{stream_version:04x}"
        ))
        .unwrap(),
        project_ref: project_id(),
        expected_state_version: stream_version.saturating_sub(1),
        idempotency_key: id(&format!("idempotency:checkpoint:{stream_version}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::RecordWorkProgress(RecordWorkProgressPayload {
            work_record: placeholder_work(),
        }),
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
            correlation_ref: id("correlation:checkpoints"),
            occurred_at: timestamp(stream_version),
            recorded_at: timestamp(stream_version),
        },
    )
    .unwrap()
}

fn placeholder_work() -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:placeholder"),
            summary: "placeholder command envelope".to_owned(),
            phase_ref: None,
            task_ref: None,
            files_or_artifacts: Vec::new(),
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: timestamp(1),
            recorded_at: timestamp(1),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/placeholder")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Continuity projection".to_owned(),
        summary: "S6".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(1),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn with_digest<T: Clone + Serialize>(mut value: T, set_digest: impl Fn(&mut T, Sha256Digest)) -> T {
    set_digest(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("digest");
    object.remove("checkpoint_digest");
    let digest = sha256_digest(&json).unwrap();
    set_digest(&mut value, digest);
    value
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T13:{:02}:00-04:00", minute % 60)).unwrap()
}

fn event_id(number: u8) -> EventId {
    EventId::parse(&format!(
        "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
    ))
    .unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0_u8; 32])
}
