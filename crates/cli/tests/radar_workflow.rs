use std::{fs, path::Path};

use assert_cmd::Command;
use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command as LedgerCommand, CommandId, CreateProjectPayload,
    Event, EventId, LedgerTimestamp, Project, ProjectCreatedEvent, ProjectId,
    ProjectLifecycleStatus, RadarSyncEnvelope, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::{
    CommandDraft, EventDraft, EventImportStore, EventStore, build_event, prepare_command,
};
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::{Value, json};
use tempfile::TempDir;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
}

#[test]
fn cli_verifies_exact_radar_contracts() {
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "radar", "verify-contracts"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "radar contracts MATCH schemas=5 fixtures=10 profile=project-ledger-radar-artifact/0.1.1",
        ));
}

#[test]
fn cli_exports_and_validates_one_sqlite_project_without_mutation_or_overwrite() {
    let fixture = Fixture::new();
    let before = event_count(&fixture.alpha_database, &alpha_id());

    radar_export_command(
        &fixture,
        &fixture.alpha_database,
        &alpha_id(),
        &fixture.alpha_export,
    )
    .assert()
    .success();
    assert_eq!(event_count(&fixture.alpha_database, &alpha_id()), before);
    let envelope: RadarSyncEnvelope = read_json(&fixture.alpha_export);
    assert_eq!(envelope.projects.len(), 1);
    assert_eq!(envelope.projects[0].project.canonical_title, "Alpha");

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "radar", "validate-export", "--input"])
        .arg(&fixture.alpha_export)
        .assert()
        .success()
        .stdout(predicates::str::contains("radar export VALID"));
    assert_eq!(event_count(&fixture.alpha_database, &alpha_id()), before);

    radar_export_command(
        &fixture,
        &fixture.alpha_database,
        &alpha_id(),
        &fixture.alpha_export,
    )
    .assert()
    .failure()
    .stderr(predicates::str::contains("refusing to overwrite"));
    assert_eq!(event_count(&fixture.alpha_database, &alpha_id()), before);

    let mut altered: Value = read_json(&fixture.alpha_export);
    altered["projects"][0]["project"]["canonical_title"] = json!("Altered");
    fs::write(
        &fixture.altered_export,
        serde_json::to_vec(&altered).unwrap(),
    )
    .unwrap();
    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["integration", "radar", "validate-export", "--input"])
        .arg(&fixture.altered_export)
        .assert()
        .failure()
        .stderr(predicates::str::contains("digest"));
}

#[test]
fn catalog_radar_export_all_aggregates_only_explicit_known_ledgers() {
    let fixture = Fixture::new();
    let alpha_before = event_count(&fixture.alpha_database, &alpha_id());
    let beta_before = event_count(&fixture.beta_database, &beta_id());

    Command::cargo_bin("project-ledger")
        .unwrap()
        .current_dir(repository_root())
        .args(["catalog", "radar-export", "--all", "--catalog"])
        .arg(&fixture.catalog)
        .args([
            "--source-instance-ref",
            "project-ledger://instances/catalog-test",
            "--observed-at",
            "2026-08-23T12:00:00-04:00",
            "--output",
        ])
        .arg(&fixture.catalog_export)
        .assert()
        .success();

    let envelope: RadarSyncEnvelope = read_json(&fixture.catalog_export);
    assert_eq!(envelope.projects.len(), 2);
    assert_eq!(envelope.projects[0].project.canonical_title, "Alpha");
    assert_eq!(envelope.projects[1].project.canonical_title, "Beta");
    assert_eq!(
        event_count(&fixture.alpha_database, &alpha_id()),
        alpha_before
    );
    assert_eq!(event_count(&fixture.beta_database, &beta_id()), beta_before);
}

struct Fixture {
    _root: TempDir,
    alpha_database: std::path::PathBuf,
    beta_database: std::path::PathBuf,
    phase_catalog: std::path::PathBuf,
    catalog: std::path::PathBuf,
    alpha_export: std::path::PathBuf,
    altered_export: std::path::PathBuf,
    catalog_export: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let alpha_database = root.path().join("alpha.db");
        let beta_database = root.path().join("beta.db");
        let phase_catalog = root.path().join("phases.json");
        create_database(&alpha_database, &alpha_id(), "Alpha");
        create_database(&beta_database, &beta_id(), "Beta");
        fs::write(&phase_catalog, b"[]").unwrap();
        let catalog = root.path().join("catalog.json");
        fs::write(
            &catalog,
            serde_json::to_vec(&json!([
                {
                    "database": alpha_database,
                    "project_ref": alpha_id(),
                    "phase_catalog": phase_catalog,
                    "source_aliases": ["alpha"]
                },
                {
                    "database": beta_database,
                    "project_ref": beta_id(),
                    "phase_catalog": phase_catalog,
                    "source_aliases": ["beta"]
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        let alpha_export = root.path().join("alpha-radar.json");
        let altered_export = root.path().join("altered-radar.json");
        let catalog_export = root.path().join("catalog-radar.json");
        Self {
            _root: root,
            alpha_database,
            beta_database,
            phase_catalog,
            catalog,
            alpha_export,
            altered_export,
            catalog_export,
        }
    }
}

fn radar_export_command(
    fixture: &Fixture,
    database: &Path,
    project: &ProjectId,
    output: &Path,
) -> Command {
    let mut command = Command::cargo_bin("project-ledger").unwrap();
    command
        .current_dir(repository_root())
        .args(["integration", "radar", "export", "--database"])
        .arg(database)
        .args(["--project-ref", project.as_str(), "--phase-catalog"])
        .arg(&fixture.phase_catalog)
        .args([
            "--source-instance-ref",
            "project-ledger://instances/test",
            "--observed-at",
            "2026-08-23T12:00:00-04:00",
            "--source-alias",
            "Alpha",
            "--output",
        ])
        .arg(output);
    command
}

fn create_database(path: &Path, project_ref: &ProjectId, title: &str) {
    let project = Project {
        project_id: project_ref.clone(),
        title: title.to_owned(),
        summary: format!("{title} continuity"),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    };
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_ref.clone(),
        expected_state_version: 0,
        idempotency_key: id(&format!("radar-cli:{title}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: LedgerCommand::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
        submitted_at: at(),
    })
    .unwrap();
    let event = build_event(
        &command,
        1,
        None,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project }),
            correlation_ref: id("correlation:radar-cli"),
            occurred_at: at(),
            recorded_at: at(),
        },
    )
    .unwrap();
    let mut store = SqliteEventStore::open(path).unwrap();
    store
        .append_imported_events(project_ref, 0, &[event])
        .unwrap();
}

fn event_count(path: &Path, project: &ProjectId) -> usize {
    SqliteEventStore::open(path)
        .unwrap()
        .load_stream(project)
        .unwrap()
        .len()
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn alpha_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7da01").unwrap()
}

fn beta_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7db02").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap()
}

#[allow(dead_code)]
fn zero() -> Sha256Digest {
    Sha256Digest::from([0; 32])
}
