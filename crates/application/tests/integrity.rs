use project_ledger_application::{
    CommandDraft, EventDraft, IntegrityError, build_event, prepare_command,
    validate_command_integrity, validate_event_integrity,
};
use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use serde::Serialize;
use serde_json::Value;

#[test]
fn payload_mutation_breaks_the_payload_digest() {
    let mut command = prepared_create_project();
    let Command::CreateProject(payload) = &mut command.command else {
        panic!("fixture must be CREATE_PROJECT");
    };
    payload.project.title = "mutated".to_owned();

    assert!(matches!(
        validate_command_integrity(&command),
        Err(IntegrityError::PayloadDigestMismatch)
    ));
}

#[test]
fn metadata_mutation_breaks_only_the_envelope_digest() {
    let mut command = prepared_create_project();
    command.expected_state_version = 7;

    assert!(matches!(
        validate_command_integrity(&command),
        Err(IntegrityError::EnvelopeDigestMismatch)
    ));
}

#[test]
fn genesis_event_has_hand_recomputed_canonical_digests() {
    let command = prepared_create_project();
    let project = project();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project }),
            correlation_ref: StableId::parse("correlation:test").unwrap(),
            occurred_at: timestamp(),
            recorded_at: timestamp(),
        },
    )
    .unwrap();

    assert_eq!(event.stream_version, 1);
    assert_eq!(event.previous_event_digest, None);
    assert_eq!(
        event.payload_digest,
        sha256_digest(&payload_value(&event.event)).unwrap()
    );
    assert_eq!(event.event_digest, digest_without(&event, "event_digest"));
    validate_event_integrity(&event).unwrap();
}

fn prepared_create_project() -> project_ledger_domain::CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        project_ref: project_id(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("project-ledger:test:create").unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:test").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: timestamp(),
    })
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Ledger test".to_owned(),
        summary: "Literal fixture".to_owned(),
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

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}

fn payload_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value)
        .unwrap()
        .get("payload")
        .cloned()
        .unwrap()
}

fn digest_without<T: Serialize>(
    value: &T,
    field: &str,
) -> project_ledger_canonical_json::Sha256Digest {
    let mut value = serde_json::to_value(value).unwrap();
    value.as_object_mut().unwrap().remove(field);
    sha256_digest(&value).unwrap()
}
