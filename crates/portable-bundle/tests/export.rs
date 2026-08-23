use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::sha256_bytes;
use project_ledger_continuity::{ResumeBudget, ResumeBuilder};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventId,
    LedgerTimestamp, PortableImportPolicy, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_portable_bundle::{PortableBundle, PortableBundleError, PortableExportRequest};
use project_ledger_projection_engine::ProjectStateProjection;
use serde_json::json;

#[test]
fn ac_005_and_pl_15_export_is_complete_deterministic_and_self_verifying() {
    let stream = genesis();
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    let resume = ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap();
    let request = request(stream, resume);

    let first = PortableBundle::build(request.clone()).unwrap();
    let second = PortableBundle::build(request).unwrap();

    assert_eq!(first.manifest(), second.manifest());
    assert_eq!(first.entries(), second.entries());
    assert_eq!(
        first.manifest().chain_digest,
        first.events()[0].event_digest
    );
    assert_eq!(first.manifest().project_state_version, 1);
    assert_eq!(
        first.manifest().project_state_digest,
        projection.state().state_digest
    );

    let expected = BTreeSet::from([
        "checksums.sha256",
        "events.jsonl",
        "evidence/references.json",
        "generated/BITACORA_PROYECTO.md",
        "integrations/radar-project-projection.json",
        "manifest.json",
        "project.json",
        "projections/resume.json",
        "projections/state.json",
        "schemas-lock.json",
        "upstream-locks/gskills.json",
    ]);
    assert_eq!(
        first
            .entries()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(first.verify().is_ok());

    let checksums = std::str::from_utf8(first.entry("checksums.sha256").unwrap()).unwrap();
    assert!(!checksums.contains("manifest.json"));
    assert!(!checksums.contains("checksums.sha256"));
    assert_eq!(
        first.manifest().checksums_digest,
        sha256_bytes(first.entry("checksums.sha256").unwrap())
    );
}

#[test]
fn portable_write_refuses_overwrite_and_default_export_rejects_sensitive_material() {
    let stream = genesis();
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    let resume = ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap();
    let bundle = PortableBundle::build(request(stream.clone(), resume.clone())).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("project-ledger-export");

    bundle.write_directory(&destination).unwrap();
    assert!(bundle.write_directory(&destination).is_err());
    assert_eq!(
        std::fs::read(destination.join("manifest.json")).unwrap(),
        bundle.entry("manifest.json").unwrap()
    );

    let mut unsafe_request = request(stream, resume);
    unsafe_request.evidence_references = json!({"secret": "do-not-export"});
    assert!(PortableBundle::build(unsafe_request).is_err());
}

#[test]
fn req_rel_002_v0_1_schema_lock_requires_explicit_migration() {
    let stream = genesis();
    let projection = ProjectStateProjection::rebuild(&stream).unwrap();
    let resume = ResumeBuilder::build(&projection, ResumeBudget::Compact).unwrap();
    let mut legacy = request(stream, resume);
    legacy.schemas_lock = json!({"schema_version": "0.1.1", "schemas": []});

    assert!(matches!(
        PortableBundle::build(legacy),
        Err(PortableBundleError::MigrationRequired {
            schema_version: SchemaVersion::V0_1_1
        })
    ));
}

fn request(
    stream: Vec<project_ledger_domain::EventEnvelope>,
    resume: project_ledger_domain::ResumeProjection,
) -> PortableExportRequest {
    PortableExportRequest {
        export_id: id("export:s7"),
        created_at: timestamp(),
        import_policy: PortableImportPolicy::FastForward,
        stream,
        schemas_lock: json!({"schema_version": "0.2.0", "schemas": ["event-envelope"]}),
        upstream_locks: BTreeMap::from([(
            "gskills.json".to_owned(),
            json!({"product": "G-Skills", "status": "PENDING_REFRESH"}),
        )]),
        resume,
        evidence_references: json!([]),
        radar_projection: json!({"profile": "project-ledger-radar-artifact/0.1.1"}),
        generated_markdown: b"<!-- GENERATED FILE. DO NOT EDIT. -->\n# Ledger\n".to_vec(),
    }
}

fn genesis() -> Vec<project_ledger_domain::EventEnvelope> {
    let project = project();
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse("command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        project_ref: project.project_id.clone(),
        expected_state_version: 0,
        idempotency_key: id("idempotency:portable:genesis"),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
        submitted_at: timestamp(),
    })
    .unwrap();
    vec![
        build_event(
            &command,
            1,
            None,
            EventDraft {
                event_id: EventId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
                event: Event::ProjectCreated(ProjectCreatedEvent { project }),
                correlation_ref: id("correlation:portable"),
                occurred_at: timestamp(),
                recorded_at: timestamp(),
            },
        )
        .unwrap(),
    ]
}

fn project() -> Project {
    Project {
        project_id: ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        title: "Portable Ledger".to_owned(),
        summary: "S7 roundtrip".to_owned(),
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
    LedgerTimestamp::parse("2026-08-23T17:00:00-04:00").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
