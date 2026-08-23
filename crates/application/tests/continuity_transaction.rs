use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, CheckpointFreshnessStatus, Command, CommandEnvelope,
    CommandId, CommandResultStatus, CreateContinuityCheckpointPayload, CreateProjectPayload, Event,
    LedgerTimestamp, Project, ProjectId, ProjectLifecycleStatus, RecordUnknownPayload,
    RecordWorkProgressPayload, ResourceRef, SchemaVersion, StableId, StorageProfile, TrustLevel,
    UnknownCriticality, UnknownRecord, UnknownStatus, WorkRecord,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde::Serialize;
use tempfile::tempdir;

#[test]
fn s6_continuity_commands_commit_checkpoint_and_freshness_atomically_in_sqlite() {
    let directory = tempdir().unwrap();
    let mut store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let clock = FixedClock::new(at());

    let created = execute(
        &mut store,
        &clock,
        command(
            1,
            0,
            Command::CreateProject(CreateProjectPayload { project: project() }),
        ),
    );
    assert_eq!(created.status, CommandResultStatus::Accepted);

    let progress = execute(
        &mut store,
        &clock,
        command(
            2,
            1,
            Command::RecordWorkProgress(RecordWorkProgressPayload {
                work_record: work_record(),
            }),
        ),
    );
    assert_eq!(progress.status, CommandResultStatus::Accepted);
    assert_eq!(progress.new_state_version, 2);

    let checkpoint = execute(
        &mut store,
        &clock,
        command(
            3,
            2,
            Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
                reason: "handoff".to_owned(),
            }),
        ),
    );
    assert_eq!(checkpoint.status, CommandResultStatus::Accepted);
    assert_eq!(checkpoint.new_state_version, 3);
    assert_eq!(checkpoint.emitted_event_refs.len(), 1);

    let unknown = execute(
        &mut store,
        &clock,
        command(
            4,
            3,
            Command::RecordUnknown(RecordUnknownPayload { unknown: unknown() }),
        ),
    );
    assert_eq!(unknown.status, CommandResultStatus::Accepted);
    assert_eq!(unknown.previous_state_version, 3);
    assert_eq!(unknown.new_state_version, 5);
    assert_eq!(unknown.emitted_event_refs.len(), 2);

    let stream = store.load_stream(&project_id()).unwrap();
    assert_eq!(stream.len(), 5);
    assert!(matches!(stream[3].event, Event::UnknownRecorded(_)));
    assert!(matches!(stream[4].event, Event::CheckpointInvalidated(_)));
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    assert_eq!(projection.state().state_version, 5);
    assert_eq!(projection.state().material_state_version, 4);
    assert_eq!(
        projection.latest_work_record().unwrap().work_record_id,
        id("work:s6")
    );
    let checkpoint_ref = projection.state().checkpoint_ref.as_ref().unwrap();
    let checkpoint_record = projection.checkpoint(checkpoint_ref).unwrap();
    assert_eq!(checkpoint_record.captured_state_version, 2);
    assert_eq!(checkpoint_record.checkpoint_event_version, 3);
    assert_eq!(
        projection.checkpoint_freshness(checkpoint_ref),
        Some(CheckpointFreshnessStatus::Invalidated)
    );
}

fn execute(
    store: &mut SqliteEventStore,
    clock: &FixedClock,
    command: CommandEnvelope,
) -> project_ledger_domain::CommandResult {
    CommandProcessor::new(store, clock)
        .execute(command, &CoreCommandHandler)
        .unwrap()
}

fn command(number: u8, expected: u64, payload: Command) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(&format!(
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
        ))
        .unwrap(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(&format!("application:continuity:{number}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: at(),
    })
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Continuity transaction".to_owned(),
        summary: "S6 real SQLite integration".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn work_record() -> WorkRecord {
    with_digest(
        WorkRecord {
            schema_version: SchemaVersion::V0_2_0,
            work_record_id: id("work:s6"),
            summary: "checkpoint and resume implemented".to_owned(),
            phase_ref: None,
            task_ref: None,
            files_or_artifacts: vec![ResourceRef::parse("workspace://crates/continuity").unwrap()],
            commit_refs: Vec::new(),
            claim_refs: Vec::new(),
            evidence_refs: Vec::new(),
            actor_ref: id("actor:test"),
            influence: None,
            occurred_at: at(),
            recorded_at: at(),
            provenance_ref: ResourceRef::parse("project-ledger://provenance/work/s6").unwrap(),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn unknown() -> UnknownRecord {
    with_digest(
        UnknownRecord {
            schema_version: SchemaVersion::V0_2_0,
            unknown_id: id("unknown:s6"),
            subject_kind: "AUTHORITY".to_owned(),
            subject_ref: id("authority:pending"),
            field_path: "digest".to_owned(),
            statement: "authority digest requires confirmation".to_owned(),
            criticality: UnknownCriticality::Material,
            status: UnknownStatus::Open,
            blocks: Vec::new(),
            required_evidence: vec!["authority digest".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: at(),
            resolved_at: None,
            resolution_ref: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/unknown/s6").unwrap(),
            digest: zero(),
        },
        |record, digest| record.digest = digest,
    )
}

fn with_digest<T: Clone + Serialize>(mut value: T, set_digest: impl Fn(&mut T, Sha256Digest)) -> T {
    set_digest(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    json.as_object_mut().unwrap().remove("digest");
    let digest = sha256_digest(&json).unwrap();
    set_digest(&mut value, digest);
    value
}

fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T16:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
