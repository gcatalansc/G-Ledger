use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, ProjectMetadataUpdatedEvent, SchemaVersion, StableId, StorageProfile,
    TrustLevel, UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{
    CommandCommit, CommandDraft, CommandResultDraft, EventDraft, EventStore, build_command_result,
    build_event, prepare_command,
};

use crate::SqliteEventStore;

pub const OVERSIZED_SUMMARY_BYTES: usize = 2 * 1024 * 1024;

pub fn create_commit() -> CommandCommit {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("disk-full:create").unwrap(),
        actor: actor(),
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(),
    })
    .unwrap();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            correlation_ref: StableId::parse("correlation:disk-full-create").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    accepted_commit(command, event, "result:disk-full-create", 0, 1)
}

pub fn metadata_commit(previous_digest: Sha256Digest, summary_bytes: usize) -> CommandCommit {
    let summary = "x".repeat(summary_bytes);
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e").unwrap(),
        project_ref: project_id(),
        expected_state_version: 1,
        idempotency_key: StableId::parse("disk-full:update").unwrap(),
        actor: actor(),
        command: Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: None,
            summary: Some(summary.clone()),
        }),
        submitted_at: timestamp(),
    })
    .unwrap();
    let event = build_event(
        &command,
        2,
        Some(previous_digest),
        EventDraft {
            event_id: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e").unwrap(),
            event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                title: None,
                summary: Some(summary),
            }),
            correlation_ref: StableId::parse("correlation:disk-full-update").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();
    accepted_commit(command, event, "result:disk-full-update", 1, 2)
}

pub fn assert_recovered_genesis(store: &SqliteEventStore, expected_digest: Sha256Digest) {
    let report = store.recovery_report(&project_id()).unwrap();
    assert_eq!(report.quick_check, "ok");
    assert_eq!(report.journal_mode, "wal");
    assert!(report.chain_verified);
    assert_eq!(report.current_version, 1);
    assert_eq!(report.event_count, 1);
    assert_eq!(report.command_count, 1);
    assert_eq!(report.command_result_count, 1);
    assert_eq!(report.unresolved_submission_count, 0);
    let events = store.load_stream(&project_id()).unwrap();
    assert_eq!(events[0].event_digest, expected_digest);
    assert!(
        store
            .command_result(
                &CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e").unwrap(),
            )
            .unwrap()
            .is_none()
    );
}

pub fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn accepted_commit(
    command: project_ledger_domain::CommandEnvelope,
    event: project_ledger_domain::EventEnvelope,
    result_id: &str,
    previous_state_version: u64,
    new_state_version: u64,
) -> CommandCommit {
    let result = build_command_result(CommandResultDraft {
        command_result_id: StableId::parse(result_id).unwrap(),
        command_ref: command.command_id.clone(),
        status: CommandResultStatus::Accepted,
        previous_state_version,
        new_state_version,
        emitted_event_refs: vec![event.event_id.clone()],
        resulting_checkpoint_ref: None,
        rejection: None,
        replayed: false,
        processed_at: timestamp(),
    })
    .unwrap();
    CommandCommit {
        command,
        events: vec![event],
        result,
    }
}

fn actor() -> Actor {
    Actor {
        actor_id: StableId::parse("actor:disk-full-test").unwrap(),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    }
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Disk-full recovery".to_owned(),
        summary: "Committed truth before the capacity fault".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-26T20:00:00-04:00").unwrap()
}
