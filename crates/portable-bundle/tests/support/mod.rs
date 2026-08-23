use std::collections::BTreeMap;

use project_ledger_continuity::{ResumeBudget, ResumeBuilder};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, PortableImportPolicy, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel, UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_portable_bundle::{PortableBundle, PortableExportRequest};
use project_ledger_projection_engine::ProjectStateProjection;
use serde_json::json;

pub fn bundle(
    stream: Vec<project_ledger_domain::EventEnvelope>,
    import_policy: PortableImportPolicy,
) -> PortableBundle {
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    PortableBundle::build(PortableExportRequest {
        export_id: id("export:s7-import"),
        created_at: timestamp(50),
        import_policy,
        stream,
        schemas_lock: json!({"schema_version": "0.2.0", "schemas": ["event-envelope"]}),
        upstream_locks: BTreeMap::from([(
            "gskills.json".to_owned(),
            json!({"product": "G-Skills", "status": "PENDING_REFRESH"}),
        )]),
        resume: ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap(),
        evidence_references: json!([]),
        radar_projection: json!({"profile": "project-ledger-radar-artifact/0.1.1"}),
        generated_markdown: b"<!-- GENERATED FILE. DO NOT EDIT. -->\n# Ledger\n".to_vec(),
    })
    .unwrap()
}

pub fn stream_with_title(
    title: &str,
    event_count: usize,
) -> Vec<project_ledger_domain::EventEnvelope> {
    assert!((1..=3).contains(&event_count));
    let project = project();
    let genesis_command = command(
        1,
        0,
        Command::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
    );
    let mut stream = vec![
        build_event(
            &genesis_command,
            1,
            None,
            EventDraft {
                event_id: event_id(1),
                event: Event::ProjectCreated(ProjectCreatedEvent { project }),
                correlation_ref: id("correlation:portable"),
                occurred_at: timestamp(1),
                recorded_at: timestamp(1),
            },
        )
        .unwrap(),
    ];
    if event_count >= 2 {
        append_metadata(&mut stream, title, 2);
    }
    if event_count >= 3 {
        append_metadata(&mut stream, &format!("{title}-third"), 3);
    }
    stream
}

fn append_metadata(
    stream: &mut Vec<project_ledger_domain::EventEnvelope>,
    title: &str,
    number: u8,
) {
    let command = command(
        number,
        stream.len() as u64,
        Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some(title.to_owned()),
            summary: None,
        }),
    );
    let previous = stream.last().map(|event| event.event_digest);
    stream.push(
        build_event(
            &command,
            stream.len() as u64 + 1,
            previous,
            EventDraft {
                event_id: event_id(number),
                event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                    title: Some(title.to_owned()),
                    summary: None,
                }),
                correlation_ref: id("correlation:portable"),
                occurred_at: timestamp(u64::from(number)),
                recorded_at: timestamp(u64::from(number)),
            },
        )
        .unwrap(),
    );
}

fn command(number: u8, expected: u64, payload: Command) -> project_ledger_domain::CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(&format!(
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df{number:02x}"
        ))
        .unwrap(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(&format!("idempotency:portable:{number}")),
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
        title: "Portable Ledger".to_owned(),
        summary: "S7 roundtrip".to_owned(),
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

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T17:{:02}:00-04:00", minute % 60)).unwrap()
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
