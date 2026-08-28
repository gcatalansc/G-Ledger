use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_continuity::{
    ConservativeTokenCounter, ContinuityEngine, ResumeBudget, ResumeBuilder, render_agent_markdown,
    render_compact,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, CheckpointFreshnessStatus, Command, CommandEnvelope,
    CommandId, CreateContinuityCheckpointPayload, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    RecordUnknownPayload, ResourceRef, SchemaVersion, StableId, StorageProfile, TrustLevel,
    UnknownCriticality, UnknownRecord, UnknownStatus,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;

#[test]
fn ac_024_resume_fits_budget_and_ac_025_stale_checkpoint_is_labeled() {
    let mut stream = genesis();
    for number in 10_u8..22 {
        let record = command(
            number,
            stream.len() as u64,
            Command::RecordUnknown(RecordUnknownPayload {
                unknown: unknown(number),
            }),
        );
        let drafts = ContinuityEngine::handle(&record, &stream).unwrap();
        append(&mut stream, &record, drafts);
    }
    let create = command(
        30,
        stream.len() as u64,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "context compaction".to_owned(),
        }),
    );
    let drafts = ContinuityEngine::handle(&create, &stream).unwrap();
    append(&mut stream, &create, drafts);

    let invalidate = command(
        31,
        stream.len() as u64,
        Command::RecordUnknown(RecordUnknownPayload {
            unknown: unknown(31),
        }),
    );
    let base = ContinuityEngine::handle(&invalidate, &stream).unwrap();
    let drafts = ContinuityEngine::extend_checkpoint_freshness(&invalidate, &stream, base).unwrap();
    append(&mut stream, &invalidate, drafts);

    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    let first = ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap();
    let second = ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.checkpoint_freshness,
        CheckpointFreshnessStatus::Invalidated
    );
    assert!(!first.critical_unknown_refs.is_empty());
    assert_eq!(first.critical_unknown_refs[0], id("unknown:10"));
    assert!(first.checkpoint_ref.is_some());
    let rendered = render_compact(&first);
    assert_eq!(
        first.rendered_token_count,
        ConservativeTokenCounter::count(&rendered)
    );
    assert!(first.rendered_token_count <= 220);
    assert!(rendered.contains("freshness=INVALIDATED"));
    assert!(rendered.contains("unknowns=unknown:10"));
    assert_eq!(resume_digest(&first), first.digest);

    let agent = ResumeBuilder::build(&projection, ResumeBudget::AgentMarkdown).unwrap();
    let agent_markdown = render_agent_markdown(&agent);
    assert!(agent_markdown.starts_with("# Project Ledger Resume\n"));
    assert!(agent_markdown.contains("- Freshness: `INVALIDATED`"));
    assert!(agent_markdown.contains("unknown:10"));
    assert_eq!(
        agent.rendered_token_count,
        ConservativeTokenCounter::count(&agent_markdown)
    );
    assert!(agent.rendered_token_count <= 450);
    assert_eq!(resume_digest(&agent), agent.digest);
    assert!(agent_markdown.contains("Bounded view; use resume --format json"));
    let structured = ResumeBuilder::build(&projection, ResumeBudget::Structured).unwrap();
    assert_eq!(structured.critical_unknown_refs.len(), 13);
    assert_eq!(resume_digest(&structured), structured.digest);
}

fn unknown(number: u8) -> UnknownRecord {
    with_digest(
        UnknownRecord {
            schema_version: SchemaVersion::V0_2_0,
            unknown_id: id(&format!("unknown:{number}")),
            subject_kind: "CHECKPOINT".to_owned(),
            subject_ref: id("checkpoint:future"),
            field_path: format!("contract.field.{number}"),
            statement: format!("material continuity uncertainty number {number}"),
            criticality: UnknownCriticality::Material,
            status: UnknownStatus::Open,
            blocks: Vec::new(),
            required_evidence: vec!["explicit evidence".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: timestamp(u64::from(number)),
            resolved_at: None,
            resolution_ref: None,
            provenance_ref: ResourceRef::parse(&format!(
                "project-ledger://provenance/unknown/{number}"
            ))
            .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    )
}

fn genesis() -> Vec<project_ledger_domain::EventEnvelope> {
    let command = command(
        1,
        0,
        Command::CreateProject(CreateProjectPayload { project: project() }),
    );
    vec![
        build_event(
            &command,
            1,
            None,
            EventDraft {
                event_id: event_id(1),
                event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
                correlation_ref: id("correlation:resume"),
                occurred_at: timestamp(1),
                recorded_at: timestamp(1),
            },
        )
        .unwrap(),
    ]
}

fn append(
    stream: &mut Vec<project_ledger_domain::EventEnvelope>,
    command: &CommandEnvelope,
    drafts: Vec<EventDraft>,
) {
    let mut previous = stream.last().map(|event| event.event_digest);
    for draft in drafts {
        let envelope = build_event(command, stream.len() as u64 + 1, previous, draft).unwrap();
        previous = Some(envelope.event_digest);
        stream.push(envelope);
    }
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
        idempotency_key: id(&format!("idempotency:resume:{number}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: timestamp(u64::from(number)),
    })
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "A deliberately long continuity project title".to_owned(),
        summary: "S6 resume budget".to_owned(),
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

fn resume_digest(resume: &project_ledger_domain::ResumeProjection) -> Sha256Digest {
    let mut value = serde_json::to_value(resume).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    sha256_digest(&value).unwrap()
}

fn with_digest<T: Clone + Serialize>(mut value: T, set_digest: impl Fn(&mut T, Sha256Digest)) -> T {
    set_digest(&mut value, zero());
    let mut json = serde_json::to_value(&value).unwrap();
    json.as_object_mut().unwrap().remove("digest");
    let digest = sha256_digest(&json).unwrap();
    set_digest(&mut value, digest);
    value
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T15:{:02}:00-04:00", minute % 60)).unwrap()
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
