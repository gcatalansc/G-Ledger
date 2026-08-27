use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_bytes, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AddArtifactReferencePayload, ArtifactKind, ArtifactReference,
    AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult, CommandResultStatus,
    ConfirmExternalMappingPayload, CorrectionField, CreateContinuityCheckpointPayload,
    CreateProjectPayload, EndSessionPayload, Event, EventEnvelope, EventId, IdentityMapping,
    IdentityMappingStatus, IdentityMethod, LedgerTimestamp, Project, ProjectId,
    ProjectLifecycleStatus, RecordEventCorrectionPayload, RecordNotePayload,
    RecordWorkProgressPayload, ResourceRef, RevokeExternalMappingPayload, ScalarValue,
    SchemaVersion, Sensitivity, Session, StableId, StartSessionPayload, StorageProfile, TrustLevel,
    UpdateProjectMetadataPayload, WorkRecord,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::{TempDir, tempdir};

// These tests catch missing auxiliary handlers/reducers at the real transaction
// boundary. No test injects events directly into the store.
#[test]
fn notes_append_once_and_rebuild_without_changing_the_original_fact() {
    let mut fixture = Fixture::new();
    let command = fixture.command(note("A limitation, not a completed gate"));
    let result = fixture.execute(command.clone());
    accepted(&result);
    assert_eq!(result.new_state_version, 2);
    let first = fixture.stream();
    assert!(
        matches!(&first[1].event, Event::NoteRecorded(note) if note.text == "A limitation, not a completed gate")
    );
    let replay = fixture.execute(command);
    assert_eq!(replay.status, CommandResultStatus::DuplicateReplay);
    assert_eq!(fixture.stream(), first);
    fixture.assert_rebuild();
}

#[test]
fn artifact_identity_cannot_be_reused_for_different_bytes() {
    let mut fixture = Fixture::new();
    accepted(
        &fixture.send(Command::AddArtifactReference(AddArtifactReferencePayload {
            artifact: artifact(),
        })),
    );
    let mut changed = artifact();
    changed.digest = sha256_bytes(b"different artifact");
    fixture.reject(Command::AddArtifactReference(AddArtifactReferencePayload {
        artifact: changed,
    }));
    fixture.assert_rebuild();
}

#[test]
fn session_end_without_checkpoint_is_non_material_and_cannot_be_repeated() {
    let mut fixture = Fixture::new();
    accepted(&fixture.send(Command::StartSession(StartSessionPayload {
        session: session(),
    })));
    accepted(&fixture.send(Command::EndSession(EndSessionPayload {
        session_ref: id("session:prepilot"),
        create_checkpoint: false,
    })));
    let projection = fixture.assert_rebuild();
    assert_eq!(projection.state().state_version, 3);
    assert_eq!(projection.state().material_state_version, 1);
    fixture.reject(Command::EndSession(EndSessionPayload {
        session_ref: id("session:prepilot"),
        create_checkpoint: false,
    }));
}

#[test]
fn session_end_with_checkpoint_is_one_atomic_replayable_batch() {
    let mut fixture = Fixture::new();
    accepted(&fixture.send(Command::StartSession(StartSessionPayload {
        session: session(),
    })));
    let command = fixture.command(Command::EndSession(EndSessionPayload {
        session_ref: id("session:prepilot"),
        create_checkpoint: true,
    }));
    let result = fixture.execute(command.clone());
    accepted(&result);
    assert_eq!(result.emitted_event_refs.len(), 2);
    let stream = fixture.stream();
    let projection = fixture.assert_rebuild();
    let checkpoint_ref = projection.state().checkpoint_ref.as_ref().unwrap();
    assert_eq!(
        result.resulting_checkpoint_ref.as_ref(),
        Some(checkpoint_ref)
    );
    let checkpoint = projection.checkpoint(checkpoint_ref).unwrap();
    assert_eq!(
        checkpoint.checkpoint_event_version,
        checkpoint.captured_state_version + 1
    );
    assert!(stream.iter().any(|event| matches!(&event.event, Event::SessionEnded(ended) if ended.ending_checkpoint_ref.as_ref() == Some(checkpoint_ref))));
    assert_eq!(
        fixture.execute(command).status,
        CommandResultStatus::DuplicateReplay
    );
    assert_eq!(fixture.stream(), stream);
}

#[test]
fn external_mapping_requires_human_confirmation_and_revocation_preserves_history() {
    let mut fixture = Fixture::new();
    accepted(&fixture.send(Command::ConfirmExternalMapping(
        ConfirmExternalMappingPayload { mapping: mapping() },
    )));
    let mut ambiguous = mapping();
    ambiguous.mapping_id = id("mapping:ambiguous");
    ambiguous.local_ref = uri("project-ledger://project/other");
    seal_mapping(&mut ambiguous);
    fixture.reject(Command::ConfirmExternalMapping(
        ConfirmExternalMappingPayload { mapping: ambiguous },
    ));
    accepted(&fixture.send(Command::RevokeExternalMapping(
        RevokeExternalMappingPayload {
            mapping_ref: id("mapping:prepilot"),
            reason: "explicit user revocation".to_owned(),
        },
    )));
    fixture.reject(Command::RevokeExternalMapping(
        RevokeExternalMappingPayload {
            mapping_ref: id("mapping:prepilot"),
            reason: "already revoked".to_owned(),
        },
    ));
    fixture.assert_rebuild();
    let mut agent_mapping = mapping();
    agent_mapping.mapping_id = id("mapping:agent");
    agent_mapping.external_ref = uri("external://project/43");
    seal_mapping(&mut agent_mapping);
    let mut command = fixture.command(Command::ConfirmExternalMapping(
        ConfirmExternalMappingPayload {
            mapping: agent_mapping,
        },
    ));
    command.actor.actor_kind = ActorKind::Agent;
    let command = reseal(command);
    let before = fixture.stream();
    assert_ne!(
        fixture.execute(command).status,
        CommandResultStatus::Accepted
    );
    assert_eq!(fixture.stream(), before);
}

#[test]
fn descriptive_correction_is_append_only_and_cannot_change_authority_or_identity() {
    let mut fixture = Fixture::new();
    accepted(&fixture.send(note("Typo in a description")));
    let original = fixture.stream();
    let target = original[1].event_id.clone();
    accepted(&fixture.send(Command::RecordEventCorrection(
        RecordEventCorrectionPayload {
            corrects_event_ref: target.clone(),
            reason: "correct descriptive wording".to_owned(),
            replacement_field_values: vec![CorrectionField {
                field_path: "text".to_owned(),
                replacement: ScalarValue::String("Corrected description".to_owned()),
            }],
        },
    )));
    assert_eq!(fixture.stream()[..2], original);
    assert_eq!(
        fixture.assert_rebuild().notes()[0].text,
        "Corrected description"
    );
    for path in [
        "note_id",
        "kind",
        "actor.actor_id",
        "state_version",
        "event_digest",
        "project_ref",
    ] {
        fixture.reject(Command::RecordEventCorrection(
            RecordEventCorrectionPayload {
                corrects_event_ref: target.clone(),
                reason: "forbidden correction".to_owned(),
                replacement_field_values: vec![CorrectionField {
                    field_path: path.to_owned(),
                    replacement: ScalarValue::String("forged".to_owned()),
                }],
            },
        ));
    }
    fixture.assert_rebuild();
}

#[test]
fn correcting_work_updates_only_the_description_and_invalidates_the_checkpoint() {
    let mut fixture = Fixture::new();
    let mut work = WorkRecord {
        schema_version: SchemaVersion::V0_2_0,
        work_record_id: id("work:prepilot"),
        summary: "Original description".to_owned(),
        phase_ref: None,
        task_ref: None,
        files_or_artifacts: vec![],
        commit_refs: vec![],
        claim_refs: vec![],
        evidence_refs: vec![],
        actor_ref: id("actor:prepilot"),
        influence: None,
        occurred_at: at(),
        recorded_at: at(),
        provenance_ref: uri("project-ledger://provenance/work"),
        digest: Sha256Digest::from([0; 32]),
    };
    let mut value = serde_json::to_value(&work).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    work.digest = sha256_digest(&value).unwrap();
    accepted(
        &fixture.send(Command::RecordWorkProgress(RecordWorkProgressPayload {
            work_record: work.clone(),
        })),
    );
    accepted(&fixture.send(Command::CreateContinuityCheckpoint(
        CreateContinuityCheckpointPayload {
            reason: "before correction".to_owned(),
        },
    )));
    let before = fixture.stream();
    let prior_state = fixture.assert_rebuild().state().clone();
    accepted(&fixture.send(Command::RecordEventCorrection(
        RecordEventCorrectionPayload {
            corrects_event_ref: before[1].event_id.clone(),
            reason: "Description typo".to_owned(),
            replacement_field_values: vec![CorrectionField {
                field_path: "work_record.summary".to_owned(),
                replacement: ScalarValue::String("Corrected work".to_owned()),
            }],
        },
    )));
    assert_eq!(fixture.stream()[..before.len()], before);
    let projection = fixture.assert_rebuild();
    let corrected = projection.latest_work_record().unwrap();
    assert_eq!(corrected.summary, "Corrected work");
    assert_eq!(corrected.actor_ref, work.actor_ref);
    assert_eq!(corrected.evidence_refs, work.evidence_refs);
    assert_ne!(corrected.digest, work.digest);
    assert_eq!(
        projection.state().active_phase_refs,
        prior_state.active_phase_refs
    );
    assert!(
        fixture
            .stream()
            .iter()
            .any(|event| matches!(event.event, Event::CheckpointInvalidated(_)))
    );
    fixture.reject(Command::RecordEventCorrection(
        RecordEventCorrectionPayload {
            corrects_event_ref: before[1].event_id.clone(),
            reason: "Forbidden state patch".to_owned(),
            replacement_field_values: vec![CorrectionField {
                field_path: "work_record.actor_ref".to_owned(),
                replacement: ScalarValue::String("human:forged".to_owned()),
            }],
        },
    ));
}

#[test]
fn auxiliary_facts_reject_foreign_references_and_invalid_initial_states() {
    let mut fixture = Fixture::new();
    fixture.reject(Command::RecordNote(RecordNotePayload {
        kind: "LIMITATION".to_owned(),
        text: "Unknown scope".to_owned(),
        scope_ref: Some(id("phase:absent")),
    }));
    let mut ended = session();
    ended.ended_at = Some(at());
    fixture.reject(Command::StartSession(StartSessionPayload {
        session: ended,
    }));
    fixture.reject(Command::EndSession(EndSessionPayload {
        session_ref: id("session:absent"),
        create_checkpoint: true,
    }));
    let mut candidate = mapping();
    candidate.status = IdentityMappingStatus::Candidate;
    seal_mapping(&mut candidate);
    fixture.reject(Command::ConfirmExternalMapping(
        ConfirmExternalMappingPayload { mapping: candidate },
    ));
    fixture.reject(Command::RecordEventCorrection(
        RecordEventCorrectionPayload {
            corrects_event_ref: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a70099")
                .unwrap(),
            reason: "absent event".to_owned(),
            replacement_field_values: vec![CorrectionField {
                field_path: "text".to_owned(),
                replacement: ScalarValue::String("No".to_owned()),
            }],
        },
    ));
    assert_eq!(fixture.stream().len(), 1);
}

#[test]
fn session_end_checks_actor_and_actual_time_across_offsets() {
    let mut fixture = Fixture::new();
    accepted(&fixture.send(Command::StartSession(StartSessionPayload {
        session: session(),
    })));
    let end = Command::EndSession(EndSessionPayload {
        session_ref: id("session:prepilot"),
        create_checkpoint: true,
    });
    let mut foreign = fixture.command(end.clone());
    foreign.actor.actor_id = id("actor:other");
    let before = fixture.stream();
    assert_ne!(
        fixture.execute(reseal(foreign)).status,
        CommandResultStatus::Accepted
    );
    assert_eq!(fixture.stream(), before);
    let mut command = fixture.command(end.clone());
    let too_early = LedgerTimestamp::parse("2026-08-27T15:59:59Z").unwrap();
    command.submitted_at = too_early.clone();
    let result = CommandProcessor::new(&mut fixture.store, &FixedClock::new(too_early))
        .execute(reseal(command), &CoreCommandHandler)
        .unwrap();
    assert_ne!(
        result.status,
        CommandResultStatus::Accepted,
        "session ended before it started"
    );
    assert_eq!(
        fixture.stream(),
        before,
        "invalid end must not even create a checkpoint"
    );
    let mut command = fixture.command(end);
    let later = LedgerTimestamp::parse("2026-08-27T12:00:01-04:00").unwrap();
    command.submitted_at = later.clone();
    accepted(
        &CommandProcessor::new(&mut fixture.store, &FixedClock::new(later.clone()))
            .execute(reseal(command), &CoreCommandHandler)
            .unwrap(),
    );
    assert_eq!(
        fixture
            .assert_rebuild()
            .session(&id("session:prepilot"))
            .unwrap()
            .ended_at,
        Some(later)
    );
}

#[test]
fn projection_or_repository_trust_never_grants_canonical_recording() {
    for trust in [
        TrustLevel::Untrusted,
        TrustLevel::TrustedForProjection,
        TrustLevel::TrustedProject,
        TrustLevel::Revoked,
    ] {
        let mut fixture = Fixture::new();
        let mut command = fixture.command(Command::UpdateProjectMetadata(
            UpdateProjectMetadataPayload {
                title: Some("Unauthorized replacement".to_owned()),
                summary: None,
            },
        ));
        command.actor.trust_level = trust;
        let before = fixture.stream();
        let result = fixture.execute(reseal(command));
        assert_ne!(
            result.status,
            CommandResultStatus::Accepted,
            "trust {trust:?} gained write authority"
        );
        assert_eq!(fixture.stream(), before);
    }
}

#[test]
fn policy_denial_is_typed_and_recording_trust_cannot_edit_project_authority() {
    for trust in [TrustLevel::Untrusted, TrustLevel::TrustedForRecording] {
        let mut fixture = Fixture::new();
        let mut command = fixture.command(Command::UpdateProjectMetadata(
            UpdateProjectMetadataPayload {
                title: Some("Unauthorized replacement".to_owned()),
                summary: None,
            },
        ));
        command.actor.trust_level = trust;
        let before = fixture.stream();
        let result = fixture.execute(reseal(command));
        assert_eq!(
            result.status,
            CommandResultStatus::PolicyDenied,
            "{trust:?}"
        );
        assert_eq!(fixture.stream(), before);
    }
}

struct Fixture {
    _directory: TempDir,
    store: SqliteEventStore,
    number: u32,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
        let mut fixture = Self {
            _directory: directory,
            store,
            number: 0,
        };
        accepted(&fixture.send(Command::CreateProject(CreateProjectPayload {
            project: Project {
                project_id: project_id(),
                title: "Prepilot auxiliary records".to_owned(),
                summary: String::new(),
                lifecycle_status: ProjectLifecycleStatus::Initializing,
                created_at: at(),
                timezone: "UTC".to_owned(),
                authority_policy: AuthorityPolicy::SpecFirst,
                storage_profile: StorageProfile::UserData,
                repository_binding_refs: vec![],
                paused_reason: None,
                archived_at: None,
            },
        })));
        fixture
    }

    fn command(&mut self, command: Command) -> CommandEnvelope {
        self.number += 1;
        prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: CommandId::parse(&format!(
                "command:0198f8c8-7417-7fb0-b88f-0fe0{number:08x}",
                number = self.number
            ))
            .unwrap(),
            project_ref: project_id(),
            expected_state_version: self.store.current_version(&project_id()).unwrap(),
            idempotency_key: id(&format!("prepilot:aux:{}", self.number)),
            actor: Actor {
                actor_id: id("actor:prepilot"),
                actor_kind: ActorKind::Human,
                trust_level: TrustLevel::LocalUser,
                display_name: None,
            },
            command,
            submitted_at: at(),
        })
        .unwrap()
    }

    fn execute(&mut self, command: CommandEnvelope) -> CommandResult {
        CommandProcessor::new(&mut self.store, &FixedClock::new(at()))
            .execute(command, &CoreCommandHandler)
            .unwrap()
    }

    fn send(&mut self, command: Command) -> CommandResult {
        let envelope = self.command(command);
        self.execute(envelope)
    }

    fn stream(&self) -> Vec<EventEnvelope> {
        self.store.load_stream(&project_id()).unwrap()
    }

    fn reject(&mut self, command: Command) {
        let before = self.stream();
        let result = self.send(command);
        assert_ne!(result.status, CommandResultStatus::Accepted, "{result:?}");
        assert!(result.emitted_event_refs.is_empty());
        assert_eq!(self.stream(), before);
    }

    fn assert_rebuild(&self) -> ProjectStateProjection {
        let stream = self.stream();
        self.store.verify_event_chain(&project_id()).unwrap();
        let rebuilt = ProjectStateProjection::rebuild(&stream).unwrap();
        let mut cursor = ProjectStateProjection::rebuild(&stream[..1]).unwrap();
        for event in &stream[1..] {
            cursor = cursor.clone().apply(cursor.head_digest(), event).unwrap();
        }
        assert_eq!(rebuilt.state(), cursor.state());
        assert_eq!(rebuilt.notes(), cursor.notes());
        assert_eq!(rebuilt.latest_work_record(), cursor.latest_work_record());
        assert_eq!(rebuilt.artifacts(), cursor.artifacts());
        assert_eq!(rebuilt.identity_mappings(), cursor.identity_mappings());
        assert_eq!(
            rebuilt.session(&id("session:prepilot")),
            cursor.session(&id("session:prepilot"))
        );
        rebuilt
    }
}

fn reseal(command: CommandEnvelope) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: command.schema_version,
        command_id: command.command_id,
        project_ref: command.project_ref,
        expected_state_version: command.expected_state_version,
        idempotency_key: command.idempotency_key,
        actor: command.actor,
        command: command.command,
        submitted_at: command.submitted_at,
    })
    .unwrap()
}

fn note(text: &str) -> Command {
    Command::RecordNote(RecordNotePayload {
        kind: "LIMITATION".to_owned(),
        text: text.to_owned(),
        scope_ref: None,
    })
}

fn artifact() -> ArtifactReference {
    ArtifactReference {
        schema_version: SchemaVersion::V0_2_0,
        artifact_ref_id: id("artifact:prepilot"),
        name: "Report".to_owned(),
        kind: ArtifactKind::Report,
        uri: uri("workspace://reports/result.json"),
        digest: sha256_bytes(b"report bytes"),
        version: None,
        sensitivity: Sensitivity::Internal,
        source_ref: None,
        created_at: at(),
        provenance_ref: uri("project-ledger://provenance/aux"),
    }
}

fn session() -> Session {
    Session {
        schema_version: SchemaVersion::V0_2_0,
        session_id: id("session:prepilot"),
        actor_ref: id("actor:prepilot"),
        host: "test".to_owned(),
        started_at: at(),
        ended_at: None,
        starting_checkpoint_ref: None,
        ending_checkpoint_ref: None,
    }
}

fn mapping() -> IdentityMapping {
    let mut mapping = IdentityMapping {
        schema_version: SchemaVersion::V0_2_0,
        mapping_id: id("mapping:prepilot"),
        local_ref: uri(&format!(
            "project-ledger://projects/{}",
            project_id().uuid()
        )),
        external_system: "test-system".to_owned(),
        external_ref: uri("external://project/42"),
        method: IdentityMethod::UserConfirmed,
        status: IdentityMappingStatus::Confirmed,
        confirmed_by: Some(id("actor:prepilot")),
        created_at: at(),
        updated_at: at(),
        digest: Sha256Digest::from([0; 32]),
    };
    seal_mapping(&mut mapping);
    mapping
}

fn seal_mapping(mapping: &mut IdentityMapping) {
    let mut value = serde_json::to_value(&mapping).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    mapping.digest = sha256_digest(&value).unwrap();
}

fn accepted(result: &CommandResult) {
    assert_eq!(result.status, CommandResultStatus::Accepted, "{result:?}");
}
fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-27T16:00:00Z").unwrap()
}
fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}
fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
fn uri(value: &str) -> ResourceRef {
    ResourceRef::parse(value).unwrap()
}
