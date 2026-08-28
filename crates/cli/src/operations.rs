use crate::errors::CliError;
use clap::{Args, Subcommand};
use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_domain::{
    Actor, ActorKind, Command, CommandType, LedgerTimestamp, ProjectId, SchemaVersion, StableId,
    TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

mod evidence_run;
mod legacy;
mod payloads;
mod portability;

#[derive(Clone, Debug, Default, Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[arg(long)]
    pub database: Option<PathBuf>,
    #[arg(long)]
    pub project_ref: Option<String>,
    #[arg(long)]
    #[serde(skip)]
    pub locator: Option<PathBuf>,
}
impl Selection {
    pub fn resolve(&self) -> Result<(PathBuf, ProjectId), CliError> {
        let selected = if self.database.is_some() && self.project_ref.is_some() {
            self.clone()
        } else if let Some(file) = &self.locator {
            let mut located: Self =
                serde_json::from_slice(&fs::read(file).map_err(CliError::input)?)
                    .map_err(CliError::input)?;
            if self.database.is_some() {
                located.database.clone_from(&self.database);
            }
            if self.project_ref.is_some() {
                located.project_ref.clone_from(&self.project_ref);
            }
            located
        } else {
            self.clone()
        };
        let database = selected.database.ok_or_else(|| {
            CliError::new(
                4,
                "PROJECT_NOT_INITIALIZED",
                "provide --database and --project-ref, or --locator",
            )
        })?;
        if !database.is_file() {
            return Err(CliError::new(
                4,
                "PROJECT_NOT_INITIALIZED",
                "Project Ledger database does not exist",
            ));
        }
        let project =
            ProjectId::parse(selected.project_ref.as_deref().ok_or_else(|| {
                CliError::new(7, "IDENTITY_UNRESOLVED", "project identity required")
            })?)
            .map_err(CliError::input)?;
        Ok((database, project))
    }
    pub fn load(&self) -> Result<(SqliteEventStore, ProjectStateProjection), CliError> {
        let (database, project) = self.resolve()?;
        let store = SqliteEventStore::open_read_only(&database)?;
        store.verify_event_chain(&project)?;
        let stream = store.load_stream(&project)?;
        if stream.is_empty() {
            return Err(CliError::new(
                4,
                "PROJECT_NOT_INITIALIZED",
                "project is not in this database",
            ));
        }
        let projection = ProjectStateProjection::rebuild(&stream).map_err(CliError::domain)?;
        Ok((store, projection))
    }
}

#[derive(Clone, Debug, Args)]
pub struct WriteArgs {
    pub id: Option<String>,
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub expected_version: u64,
    #[arg(long)]
    pub idempotency_key: String,
    #[arg(long)]
    pub actor_id: String,
    /// Explicit local human action. Repository locators cannot enable it.
    #[arg(long)]
    pub as_human: bool,
    #[arg(long)]
    pub observed_at: String,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub phase: Option<String>,
    #[arg(long, num_args=1..)]
    pub files: Vec<String>,
    #[arg(long)]
    pub record_id: Option<String>,
    #[arg(long)]
    pub uri: Option<String>,
    #[arg(long)]
    pub digest: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub statement: Option<String>,
    #[arg(long)]
    pub classification: Option<String>,
    #[arg(long)]
    pub subject: Option<String>,
    #[arg(long)]
    pub criticality: Option<String>,
    #[arg(long)]
    pub severity: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub set: Option<String>,
    #[arg(long)]
    pub evidence: Vec<String>,
    #[arg(long)]
    pub resolution: Option<String>,
    #[arg(long)]
    pub authority: Option<String>,
    #[arg(long)]
    pub approved_by: Option<String>,
    #[arg(long)]
    pub with: Option<String>,
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub remote: Option<String>,
    #[arg(long)]
    pub snapshot_ref: Option<String>,
    #[arg(long)]
    pub snapshot_digest: Option<String>,
    #[arg(long)]
    pub system: Option<String>,
    #[arg(long)]
    pub artifact_ref: Option<String>,
    #[arg(long)]
    pub schema_ref: Option<String>,
    #[arg(long)]
    pub signal: Option<PathBuf>,
    #[arg(long)]
    pub supersedes: Option<String>,
}
#[derive(Clone, Debug, Args)]
pub struct ReconcileArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub command_type: String,
    #[arg(long)]
    pub actor_id: String,
    #[arg(long)]
    pub as_human: bool,
    #[arg(long)]
    pub idempotency_key: String,
    #[arg(long)]
    pub trust_domain: Option<String>,
}
#[derive(Debug, Subcommand)]
pub enum Operation {
    Status(Selection),
    Doctor(Selection),
    Audit(portability::VerifyArgs),
    Verify(portability::VerifyArgs),
    Render(portability::RenderArgs),
    Export(portability::ExportArgs),
    Import(portability::ImportArgs),
    Backup(portability::BackupArgs),
    Restore(portability::RestoreArgs),
    Migrate(portability::MigrateArgs),
    ImportMarkdown(legacy::LegacyArgs),
    Pause(WriteArgs),
    ResumeProject(WriteArgs),
    Archive(WriteArgs),
    Reconcile(ReconcileArgs),
    Record {
        #[command(subcommand)]
        command: RecordCommand,
    },
    Authority {
        #[command(subcommand)]
        command: AuthorityCommand,
    },
    Repository {
        #[command(subcommand)]
        command: RepositoryCommand,
    },
    Roadmap {
        #[command(subcommand)]
        command: RoadmapCommand,
    },
    Phase {
        #[command(subcommand)]
        command: PhaseCommand,
    },
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    Unknown {
        #[command(subcommand)]
        command: UnknownCommand,
    },
    Decision {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    Blocker {
        #[command(subcommand)]
        command: BlockerCommand,
    },
    Risk {
        #[command(subcommand)]
        command: RiskCommand,
    },
    Next {
        #[command(subcommand)]
        command: NextCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    #[command(flatten)]
    pub selection: Selection,
    pub id: Option<String>,
    #[arg(long)]
    pub phase: Option<String>,
}

macro_rules! family {
    ($name:ident { $($write:ident => $kind:literal),* }; { $($query:ident => $view:literal),* }) => {
        #[derive(Debug, Subcommand)]
        pub enum $name { $($write(Box<WriteArgs>),)* $($query(ViewArgs),)* }
        impl $name {
            fn run(self) -> Result<String, CliError> { match self {
                $(Self::$write(args) => mutate(&args, $kind),)*
                $(Self::$query(args) => query(&args, $view),)*
            } }
        }
    }
}
family!(RecordCommand { Note => "RECORD_NOTE", Progress => "RECORD_WORK_PROGRESS" }; { Notes => "notes" });
family!(AuthorityCommand { Add => "REGISTER_AUTHORITY_DOCUMENT", Refresh => "REFRESH_AUTHORITY_DOCUMENT_DIGEST", Supersede => "SUPERSEDE_AUTHORITY_DOCUMENT" }; { List => "authority" });
family!(RepositoryCommand { Bind => "BIND_REPOSITORY", Refresh => "REFRESH_REPOSITORY_BINDING", Revoke => "REVOKE_REPOSITORY_BINDING" }; { List => "repository" });
family!(RoadmapCommand { Import => "DEFINE_ROADMAP" }; { Show => "roadmap" });
family!(PhaseCommand { Start => "START_PHASE", RequestClose => "REQUEST_PHASE_COMPLETION", Reopen => "REOPEN_PHASE" }; { List => "phase" });
family!(GateCommand { Define => "DEFINE_GATE", RequestRevalidation => "REQUEST_GATE_REVALIDATION", Evaluate => "REQUEST_GATE_EVALUATION", Waive => "WAIVE_GATE" }; { List => "gate", Show => "gate-show" });
family!(ArtifactCommand { Add => "ADD_ARTIFACT_REFERENCE" }; { List => "artifact" });
family!(ClaimCommand { Record => "RECORD_CLAIM", Status => "UPDATE_CLAIM_STATUS" }; {});
family!(UnknownCommand { Record => "RECORD_UNKNOWN", Resolve => "RESOLVE_UNKNOWN" }; {});
family!(DecisionCommand { Propose => "PROPOSE_DECISION", Accept => "ACCEPT_DECISION", Supersede => "SUPERSEDE_DECISION" }; { Show => "decision-show" });
family!(BlockerCommand { Open => "OPEN_BLOCKER", Resolve => "RESOLVE_BLOCKER" }; {});
family!(RiskCommand { Open => "OPEN_RISK", Mitigate => "MITIGATE_RISK" }; {});
family!(NextCommand { Set => "SET_NEXT_ACTION" }; {});
family!(SessionCommand { Start => "START_SESSION", End => "END_SESSION" }; {});
#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    Import(WriteArgs),
    Link(WriteArgs),
    Unlink(WriteArgs),
    Show(ViewArgs),
    ExternalList(ViewArgs),
    Run(evidence_run::RunArgs),
}
impl EvidenceCommand {
    fn run(self) -> Result<String, CliError> {
        match self {
            Self::Import(a) => mutate(&a, "RECORD_VERIFICATION_RESULT"),
            Self::Link(a) => mutate(&a, "LINK_EXTERNAL_EVIDENCE"),
            Self::Unlink(a) => mutate(&a, "UNLINK_EXTERNAL_EVIDENCE"),
            Self::Show(a) => query(&a, "evidence-show"),
            Self::ExternalList(a) => query(&a, "external-evidence"),
            Self::Run(a) => evidence_run::run(&a),
        }
    }
}

pub fn run(operation: Operation) -> Result<String, CliError> {
    match operation {
        Operation::Status(selection) => {
            let (_, p) = selection.load()?;
            crate::canonical_stdout(p.state()).map_err(CliError::input)
        }
        Operation::Doctor(selection) => {
            let (store, p) = selection.load()?;
            let recovery = store.recovery_report(&p.state().project.project_id)?;
            crate::canonical_stdout(&json!({"integrity":recovery.quick_check, "project_ref":p.state().project.project_id, "state_version":p.state().state_version,"head_digest":p.head_digest(),"migration_version":recovery.migration_version,"command_count":recovery.command_count,"command_result_count":recovery.command_result_count,"unresolved_submission_count":recovery.unresolved_submission_count,"read_only":true})).map_err(CliError::input)
        }
        Operation::Pause(args) => mutate(&args, "PAUSE_PROJECT"),
        Operation::ResumeProject(args) => mutate(&args, "RESUME_PROJECT"),
        Operation::Archive(args) => mutate(&args, "ARCHIVE_PROJECT"),
        Operation::Record { command } => command.run(),
        Operation::Authority { command } => command.run(),
        Operation::Repository { command } => command.run(),
        Operation::Roadmap { command } => command.run(),
        Operation::Phase { command } => command.run(),
        Operation::Gate { command } => command.run(),
        Operation::Artifact { command } => command.run(),
        Operation::Claim { command } => command.run(),
        Operation::Unknown { command } => command.run(),
        Operation::Decision { command } => command.run(),
        Operation::Blocker { command } => command.run(),
        Operation::Risk { command } => command.run(),
        Operation::Next { command } => command.run(),
        Operation::Session { command } => command.run(),
        Operation::Evidence { command } => command.run(),
        Operation::Reconcile(args) => reconcile(&args),
        Operation::Audit(args) | Operation::Verify(args) => portability::verify(&args),
        Operation::Render(args) => portability::render(&args),
        Operation::Export(args) => portability::export(&args),
        Operation::Import(args) => portability::import(&args),
        Operation::Backup(args) => portability::backup(&args),
        Operation::Restore(args) => portability::restore(&args),
        Operation::Migrate(args) => portability::migrate(&args),
        Operation::ImportMarkdown(args) => legacy::run(&args),
    }
}
fn mutate(args: &WriteArgs, kind: &str) -> Result<String, CliError> {
    let payload = if args.file.is_some() {
        Value::Null
    } else {
        payloads::build(args, kind)?
    };
    submit(args, kind, payload)
}

fn query(args: &ViewArgs, kind: &str) -> Result<String, CliError> {
    let (store, p) = args.selection.load()?;
    let value = match kind {
        "authority" => json!(p.state().authority_documents),
        "repository" => json!(p.repository_bindings()),
        "roadmap" => json!(p.state().roadmap),
        "notes" => json!(p.notes()),
        "artifact" => json!(p.artifacts()),
        "phase" => json!(
            p.state()
                .roadmap
                .iter()
                .flat_map(|r| r.phase_order.iter())
                .map(|id| json!({"phase_ref":id,"status":p.phase_status(id)}))
                .collect::<Vec<_>>()
        ),
        "gate" => json!(
            p.gate_definitions()
                .into_iter()
                .filter(|g| args
                    .phase
                    .as_ref()
                    .is_none_or(|phase| g.phase_ref.as_str() == phase))
                .map(|g| json!({"definition":g,"status":p.gate_status(&g.gate_id)}))
                .collect::<Vec<_>>()
        ),
        "gate-show" => {
            let id = StableId::parse(
                args.id
                    .as_deref()
                    .ok_or_else(|| CliError::input("gate id required"))?,
            )
            .map_err(CliError::input)?;
            let gate = p
                .gate(&id)
                .ok_or_else(|| CliError::domain("unknown gate"))?;
            json!({"definition":gate,"status":p.gate_status(&id),"evaluation":p.current_gate_evaluation(&id)})
        }
        "evidence-show" => {
            let id = StableId::parse(
                args.id
                    .as_deref()
                    .ok_or_else(|| CliError::input("evidence id required"))?,
            )
            .map_err(CliError::input)?;
            json!(
                p.evidence(&id)
                    .ok_or_else(|| CliError::domain("unknown evidence"))?
            )
        }
        "decision-show" => {
            let id = StableId::parse(
                args.id
                    .as_deref()
                    .ok_or_else(|| CliError::input("decision id required"))?,
            )
            .map_err(CliError::input)?;
            json!(
                p.decision(&id)
                    .ok_or_else(|| CliError::domain("unknown decision"))?
            )
        }
        "external-evidence" => {
            let mut active = std::collections::BTreeMap::new();
            for event in store.load_stream(&p.state().project.project_id)? {
                match event.event {
                    project_ledger_domain::Event::ExternalEvidenceLinked(link) => {
                        active.insert(link.evidence_ref.clone(), link);
                    }
                    project_ledger_domain::Event::ExternalEvidenceUnlinked(link) => {
                        active.remove(&link.evidence_ref);
                    }
                    _ => {}
                }
            }
            json!(active.values().collect::<Vec<_>>())
        }
        _ => return Err(CliError::input("unknown query")),
    };
    crate::canonical_stdout(&value).map_err(CliError::input)
}
pub fn submit(args: &WriteArgs, kind: &str, payload: Value) -> Result<String, CliError> {
    let (_, project) = args.selection.resolve()?;
    let payload = if let Some(file) = &args.file {
        payloads::read(file, kind)?
    } else {
        payload
    };
    let payload = if kind == "RECORD_VERIFICATION_RESULT" {
        let bytes = serde_json::to_vec(payload.get("evidence").unwrap_or(&payload))
            .map_err(CliError::input)?;
        let uri = project_ledger_domain::ResourceRef::parse(&format!(
            "urn:project-ledger:import:{}",
            project_ledger_canonical_json::sha256_bytes(&bytes)
        ))
        .map_err(CliError::input)?;
        let evidence = project_ledger_evidence::EvidenceImporter::import_unsigned_json(
            &bytes,
            &project_ledger_evidence::EvidenceImportContext {
                expected_project_ref: project.clone(),
                artifact_uri: uri.clone(),
                provenance_ref: uri,
                importer_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        )
        .map_err(|error| match error {
            project_ledger_evidence::EvidenceImporterError::Privacy => {
                CliError::new(13, "PRIVACY_REJECTED", error)
            }
            other => CliError::input(other),
        })?;
        json!({"evidence":evidence})
    } else {
        payload
    };
    let command: Command = serde_json::from_value(json!({"command_type":kind,"payload":payload}))
        .map_err(CliError::input)?;
    execute(args, command)
}
pub fn execute(args: &WriteArgs, command: Command) -> Result<String, CliError> {
    let (path, project) = args.selection.resolve()?;
    let kind = serde_json::to_value(command.command_type()).map_err(CliError::input)?;
    let kind = kind
        .as_str()
        .ok_or_else(|| CliError::input("invalid command type"))?;
    let key = StableId::parse(&args.idempotency_key).map_err(CliError::input)?;
    let at = LedgerTimestamp::parse(&args.observed_at).map_err(CliError::input)?;
    let envelope = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: crate::deterministic_cli_command_id(
            &format!("{kind}:{}:{}", args.actor_id, args.as_human),
            &project,
            &key,
        )
        .map_err(CliError::input)?,
        project_ref: project.clone(),
        expected_state_version: args.expected_version,
        idempotency_key: key,
        actor: Actor {
            actor_id: StableId::parse(&args.actor_id).map_err(CliError::input)?,
            actor_kind: if args.as_human {
                ActorKind::Human
            } else {
                ActorKind::Agent
            },
            trust_level: if args.as_human {
                TrustLevel::LocalUser
            } else {
                TrustLevel::TrustedForRecording
            },
            display_name: None,
        },
        command,
        submitted_at: at.clone(),
    })
    .map_err(CliError::input)?;
    let mut store = SqliteEventStore::open_existing(&path)?;
    if store.current_version(&project)? == 0 {
        return Err(CliError::new(
            4,
            "PROJECT_NOT_INITIALIZED",
            "project is not initialized",
        ));
    }
    let result = CommandProcessor::new(&mut store, &FixedClock::new(at))
        .execute(envelope, &CoreCommandHandler)?;
    if kind == "REQUEST_GATE_EVALUATION" && store.load_stream(&project)?.iter().any(|e| result.emitted_event_refs.contains(&e.event_id) && matches!(&e.event, project_ledger_domain::Event::GateEvaluated(g) if g.evaluation.result != project_ledger_domain::GateEvaluationResult::Pass)) {
        let mut error = CliError::new(6,"GATE_INCOMPLETE","gate evaluation did not pass; result is durably recorded");
        error.result = Some(json!(result)); return Err(error);
    }
    CliError::command(result).map_err(|mut error| {
        if kind == "REQUEST_PHASE_COMPLETION" && error.exit == 3 {
            error.exit = 6;
            error.code = "GATE_INCOMPLETE";
        }
        error
    })
}
fn reconcile(args: &ReconcileArgs) -> Result<String, CliError> {
    let (store, projection) = args.selection.load()?;
    let kind: CommandType =
        serde_json::from_value(json!(args.command_type)).map_err(CliError::input)?;
    let trust = args.trust_domain.as_deref().unwrap_or(if args.as_human {
        "LOCAL_USER"
    } else {
        "TRUSTED_FOR_RECORDING"
    });
    let _: TrustLevel = serde_json::from_value(json!(trust)).map_err(CliError::input)?;
    let result = store.reconcile_command(
        &projection.state().project.project_id,
        kind,
        &StableId::parse(&args.actor_id).map_err(CliError::input)?,
        trust,
        &StableId::parse(&args.idempotency_key).map_err(CliError::input)?,
    )?;
    match result {
        Some(result) => crate::canonical_stdout(&json!({"outcome":"RECONCILED", "result":result}))
            .map_err(CliError::input),
        None => Err(CliError::new(
            12,
            "OUTCOME_UNKNOWN",
            "no durable result for this full scope; do not retry with a new key",
        )),
    }
}

#[derive(Clone, Debug, Args)]
pub struct CatalogArgs {
    #[arg(long)]
    pub data_root: Option<PathBuf>,
}

pub fn catalog(args: &CatalogArgs, rebuild: bool) -> Result<String, CliError> {
    use project_ledger_project_catalog::{KnownProject, ProjectCatalog};
    let root = args
        .data_root
        .clone()
        .map_or_else(|| crate::default_data_root().map_err(CliError::input), Ok)?
        .join("project-ledger");
    let projects = root.join("projects");
    let mut known = Vec::new();
    if projects.is_dir() {
        for entry in fs::read_dir(projects).map_err(CliError::storage)? {
            let entry = entry.map_err(CliError::storage)?;
            let database = entry.path().join("ledger.db");
            if !database.is_file() {
                continue;
            }
            let project_ref = format!("project:{}", entry.file_name().to_string_lossy());
            let (_, projection) = Selection {
                database: Some(database.clone()),
                project_ref: Some(project_ref),
                locator: None,
            }
            .load()?;
            known.push(KnownProject {
                state: projection.into_state(),
                ledger_location_ref: project_ledger_domain::ResourceRef::parse(&payloads::uri(
                    &database.to_string_lossy(),
                )?)
                .map_err(CliError::input)?,
            });
        }
    }
    let entries = ProjectCatalog::rebuild(&known).map_err(CliError::domain)?;
    let output = crate::canonical_stdout(&entries).map_err(CliError::input)?;
    if rebuild {
        fs::create_dir_all(&root).map_err(CliError::storage)?;
        fs::write(root.join("catalog.json"), &output).map_err(CliError::storage)?;
    }
    Ok(output)
}
