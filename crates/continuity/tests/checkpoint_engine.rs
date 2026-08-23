use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_continuity::{ContinuityEngine, ContinuityError};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, BindRepositoryPayload, Command, CommandEnvelope, CommandId, ConflictKind,
    ConflictOpenedEvent, ConflictRecord, ConflictStatus, CreateContinuityCheckpointPayload,
    CreateProjectPayload, Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, RecordNotePayload, RecordUnknownPayload,
    RegisterAuthorityDocumentPayload, RepositoryBinding, RepositoryBindingStatus,
    RepositoryBoundEvent, RepositoryIdentityMethod, ResourceRef, SchemaVersion, Severity, StableId,
    StorageProfile, TrustLevel, UnknownCriticality, UnknownRecord, UnknownStatus,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use serde::Serialize;

#[test]
fn checkpoint_engine_builds_exact_deterministic_n_plus_one_event() {
    let mut stream = genesis();
    let create_command = command(
        10,
        1,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "session end".to_owned(),
        }),
    );
    let first = ContinuityEngine::handle(&create_command, &stream).unwrap();
    let second = ContinuityEngine::handle(&create_command, &stream).unwrap();
    assert_drafts_eq(&first, &second);
    assert_eq!(first.len(), 1);
    let Event::CheckpointCreated(created) = &first[0].event else {
        panic!("checkpoint command must emit CHECKPOINT_CREATED");
    };
    assert_eq!(created.checkpoint.captured_state_version, 1);
    assert_eq!(created.checkpoint.captured_material_state_version, 1);
    assert_eq!(created.checkpoint.checkpoint_event_version, 2);
    assert_eq!(created.checkpoint.checkpoint_event_ref, first[0].event_id);
    assert_eq!(created.checkpoint.created_by, id("actor:test"));
    assert_eq!(created.checkpoint.created_at, timestamp(10));
    assert_eq!(created.checkpoint.project_ref, project_id());
    assert_eq!(
        created.checkpoint.checkpoint_id,
        id("checkpoint:0198f8c8-7417-7fb0-b88f-0fe0c0a7df0a")
    );

    let empty = command(
        11,
        1,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "   ".to_owned(),
        }),
    );
    assert!(matches!(
        ContinuityEngine::handle(&empty, &stream),
        Err(ContinuityError::InvalidCommand { .. })
    ));
    append(&mut stream, &create_command, first);
    let projection =
        project_ledger_projection_engine::ProjectStateProjection::rebuild(&stream).unwrap();
    assert_eq!(
        projection.checkpoint_freshness(projection.state().checkpoint_ref.as_ref().unwrap()),
        Some(project_ledger_domain::CheckpointFreshnessStatus::CurrentForProjectOnly)
    );
}

#[test]
fn material_unknown_invalidates_current_checkpoint_once_in_the_same_batch() {
    let mut stream = genesis();
    let create = command(
        20,
        1,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "handoff".to_owned(),
        }),
    );
    let created = ContinuityEngine::handle(&create, &stream).unwrap();
    append(&mut stream, &create, created);

    let unknown = with_digest(
        UnknownRecord {
            schema_version: SchemaVersion::V0_2_0,
            unknown_id: id("unknown:material"),
            subject_kind: "CHECKPOINT".to_owned(),
            subject_ref: id("checkpoint:future"),
            field_path: "authority".to_owned(),
            statement: "authority status requires review".to_owned(),
            criticality: UnknownCriticality::Material,
            status: UnknownStatus::Open,
            blocks: Vec::new(),
            required_evidence: vec!["authority digest".to_owned()],
            evidence_refs: Vec::new(),
            opened_at: timestamp(21),
            resolved_at: None,
            resolution_ref: None,
            provenance_ref: ResourceRef::parse("project-ledger://provenance/unknown/material")
                .unwrap(),
            digest: zero(),
        },
        |value, digest| value.digest = digest,
    );
    let record = command(
        21,
        stream.len() as u64,
        Command::RecordUnknown(RecordUnknownPayload { unknown }),
    );
    let base = ContinuityEngine::handle(&record, &stream).unwrap();
    assert_eq!(base.len(), 1);
    let extended = ContinuityEngine::extend_checkpoint_freshness(&record, &stream, base).unwrap();
    assert_eq!(extended.len(), 2);
    let Event::CheckpointInvalidated(invalidated) = &extended[1].event else {
        panic!("material unknown must append checkpoint invalidation");
    };
    assert_eq!(
        invalidated.freshness,
        project_ledger_domain::CheckpointFreshnessStatus::Invalidated
    );
    assert_eq!(invalidated.reason_codes, vec!["RECORD_UNKNOWN"]);
    append(&mut stream, &record, extended);

    let compatible = command(
        22,
        stream.len() as u64,
        Command::RecordUnknown(RecordUnknownPayload {
            unknown: with_digest(
                UnknownRecord {
                    schema_version: SchemaVersion::V0_2_0,
                    unknown_id: id("unknown:second"),
                    subject_kind: "PROJECT".to_owned(),
                    subject_ref: id("project-subject:local"),
                    field_path: "summary".to_owned(),
                    statement: "second unknown".to_owned(),
                    criticality: UnknownCriticality::Low,
                    status: UnknownStatus::Open,
                    blocks: Vec::new(),
                    required_evidence: Vec::new(),
                    evidence_refs: Vec::new(),
                    opened_at: timestamp(22),
                    resolved_at: None,
                    resolution_ref: None,
                    provenance_ref: ResourceRef::parse(
                        "project-ledger://provenance/unknown/second",
                    )
                    .unwrap(),
                    digest: zero(),
                },
                |value, digest| value.digest = digest,
            ),
        }),
    );
    let base = ContinuityEngine::handle(&compatible, &stream).unwrap();
    let unchanged =
        ContinuityEngine::extend_checkpoint_freshness(&compatible, &stream, base).unwrap();
    assert_eq!(unchanged.len(), 1);
}

#[test]
fn checkpoint_preserves_one_repository_snapshot_and_rejects_ambiguous_current_bindings() {
    let mut one_repository = genesis();
    append_binding(&mut one_repository, 2, binding("main", 1));
    let create = command(
        10,
        one_repository.len() as u64,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "repository handoff".to_owned(),
        }),
    );
    let drafts = ContinuityEngine::handle(&create, &one_repository).unwrap();
    let Event::CheckpointCreated(created) = &drafts[0].event else {
        panic!("checkpoint event expected");
    };
    let snapshot = created.checkpoint.repository_binding.as_ref().unwrap();
    assert_eq!(snapshot.repository_ref, id("repository:main"));
    assert_eq!(snapshot.snapshot_ref, id("snapshot:main"));
    append(&mut one_repository, &create, drafts);
    let projection =
        project_ledger_projection_engine::ProjectStateProjection::rebuild(&one_repository).unwrap();
    assert_eq!(
        projection.checkpoint_freshness(projection.state().checkpoint_ref.as_ref().unwrap()),
        Some(project_ledger_domain::CheckpointFreshnessStatus::Current)
    );

    let mut ambiguous = genesis();
    append_binding(&mut ambiguous, 2, binding("main", 1));
    append_binding(&mut ambiguous, 3, binding("secondary", 2));
    let rejected = command(
        11,
        ambiguous.len() as u64,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "ambiguous repository handoff".to_owned(),
        }),
    );
    assert!(matches!(
        ContinuityEngine::handle(&rejected, &ambiguous),
        Err(ContinuityError::InvalidCommand { .. })
    ));
}

#[test]
fn authority_and_repository_changes_select_exact_stale_statuses() {
    let mut authority_stream = checkpointed_stream(12);
    let document = AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:canonical"),
        uri: ResourceRef::parse("file:///authority/spec.md").unwrap(),
        digest: Sha256Digest::from([7; 32]),
        version: "1.0.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: timestamp(13),
        supersedes_ref: None,
    };
    let authority_command = command(
        13,
        authority_stream.len() as u64,
        Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
            document: document.clone(),
        }),
    );
    let authority_drafts = ContinuityEngine::extend_checkpoint_freshness(
        &authority_command,
        &authority_stream,
        vec![draft(
            13,
            Event::AuthorityDocumentRegistered(AuthorityDocumentRegisteredEvent { document }),
        )],
    )
    .unwrap();
    assert_invalidation(
        &authority_drafts,
        project_ledger_domain::CheckpointFreshnessStatus::StaleForAuthority,
        "REGISTER_AUTHORITY_DOCUMENT",
    );
    append(&mut authority_stream, &authority_command, authority_drafts);

    let mut repository_stream = checkpointed_stream(14);
    let repository = binding("main", 1);
    let repository_command = command(
        15,
        repository_stream.len() as u64,
        Command::BindRepository(BindRepositoryPayload {
            binding: repository.clone(),
        }),
    );
    let repository_drafts = ContinuityEngine::extend_checkpoint_freshness(
        &repository_command,
        &repository_stream,
        vec![draft(
            15,
            Event::RepositoryBound(RepositoryBoundEvent {
                binding: repository,
            }),
        )],
    )
    .unwrap();
    assert_invalidation(
        &repository_drafts,
        project_ledger_domain::CheckpointFreshnessStatus::StaleForRepository,
        "BIND_REPOSITORY",
    );
    append(
        &mut repository_stream,
        &repository_command,
        repository_drafts,
    );
}

#[test]
fn critical_conflict_makes_a_new_checkpoint_conflicted() {
    let mut conflict_stream = genesis();
    let conflict_command = command(
        16,
        conflict_stream.len() as u64,
        Command::RecordNote(RecordNotePayload {
            kind: "AUDIT".to_owned(),
            text: "critical conflict".to_owned(),
            scope_ref: None,
        }),
    );
    append(
        &mut conflict_stream,
        &conflict_command,
        vec![draft(
            16,
            Event::ConflictOpened(ConflictOpenedEvent {
                conflict: ConflictRecord {
                    schema_version: SchemaVersion::V0_2_0,
                    conflict_id: id("conflict:critical"),
                    kind: ConflictKind::SourceDivergence,
                    severity: Severity::Critical,
                    domain: "continuity".to_owned(),
                    subject_refs: Vec::new(),
                    source_refs: vec![
                        ResourceRef::parse("project-ledger://source/conflict/critical").unwrap(),
                    ],
                    status: ConflictStatus::Open,
                    required_action: "review".to_owned(),
                    opened_at: timestamp(16),
                    resolved_at: None,
                    resolution_ref: None,
                },
            }),
        )],
    );
    let checkpoint_command = command(
        17,
        conflict_stream.len() as u64,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "conflicted handoff".to_owned(),
        }),
    );
    let checkpoint_drafts =
        ContinuityEngine::handle(&checkpoint_command, &conflict_stream).unwrap();
    append(&mut conflict_stream, &checkpoint_command, checkpoint_drafts);
    let projection =
        project_ledger_projection_engine::ProjectStateProjection::rebuild(&conflict_stream)
            .unwrap();
    assert_eq!(
        projection.checkpoint_freshness(projection.state().checkpoint_ref.as_ref().unwrap()),
        Some(project_ledger_domain::CheckpointFreshnessStatus::Conflicted)
    );
}

fn checkpointed_stream(number: u8) -> Vec<project_ledger_domain::EventEnvelope> {
    let mut stream = genesis();
    let create = command(
        number,
        stream.len() as u64,
        Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: "handoff".to_owned(),
        }),
    );
    let drafts = ContinuityEngine::handle(&create, &stream).unwrap();
    append(&mut stream, &create, drafts);
    stream
}

fn draft(number: u8, event: Event) -> EventDraft {
    EventDraft {
        event_id: event_id(number),
        event,
        correlation_ref: id("correlation:continuity:freshness"),
        occurred_at: timestamp(u64::from(number)),
        recorded_at: timestamp(u64::from(number)),
    }
}

fn assert_invalidation(
    drafts: &[EventDraft],
    expected: project_ledger_domain::CheckpointFreshnessStatus,
    reason_code: &str,
) {
    assert_eq!(drafts.len(), 2);
    let Event::CheckpointInvalidated(invalidated) = &drafts[1].event else {
        panic!("checkpoint invalidation expected");
    };
    assert_eq!(invalidated.freshness, expected);
    assert_eq!(invalidated.reason_codes, vec![reason_code]);
}

fn append_binding(
    stream: &mut Vec<project_ledger_domain::EventEnvelope>,
    number: u8,
    binding: RepositoryBinding,
) {
    let command = command(
        number,
        stream.len() as u64,
        Command::BindRepository(BindRepositoryPayload {
            binding: binding.clone(),
        }),
    );
    append(
        stream,
        &command,
        vec![EventDraft {
            event_id: event_id(number),
            event: Event::RepositoryBound(RepositoryBoundEvent { binding }),
            correlation_ref: id("correlation:continuity:repository"),
            occurred_at: timestamp(u64::from(number)),
            recorded_at: timestamp(u64::from(number)),
        }],
    );
}

fn binding(name: &str, digest_byte: u8) -> RepositoryBinding {
    with_digest(
        RepositoryBinding {
            schema_version: SchemaVersion::V0_2_0,
            repository_binding_id: id(&format!("binding:{name}")),
            repository_ref: id(&format!("repository:{name}")),
            workspace_root_ref: ResourceRef::parse(&format!("workspace://repository/{name}"))
                .unwrap(),
            identity_method: RepositoryIdentityMethod::ExplicitId,
            remote_identity_digest: None,
            current_snapshot_ref: Some(id(&format!("snapshot:{name}"))),
            current_snapshot_digest: Some(Sha256Digest::from([digest_byte; 32])),
            status: RepositoryBindingStatus::Current,
            confirmed_by: Some(id("actor:test")),
            created_at: timestamp(2),
            updated_at: timestamp(2),
            provenance_ref: ResourceRef::parse(&format!(
                "project-ledger://provenance/repository/{name}"
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
                correlation_ref: id("correlation:continuity"),
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
        idempotency_key: id(&format!("idempotency:continuity:{number}")),
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
        title: "Continuity engine".to_owned(),
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
    json.as_object_mut().unwrap().remove("digest");
    let digest = sha256_digest(&json).unwrap();
    set_digest(&mut value, digest);
    value
}

fn timestamp(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T14:{:02}:00-04:00", minute % 60)).unwrap()
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

fn assert_drafts_eq(left: &[EventDraft], right: &[EventDraft]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        assert_eq!(left.event_id, right.event_id);
        assert_eq!(left.event, right.event);
        assert_eq!(left.correlation_ref, right.correlation_ref);
        assert_eq!(left.occurred_at, right.occurred_at);
        assert_eq!(left.recorded_at, right.recorded_at);
    }
}
