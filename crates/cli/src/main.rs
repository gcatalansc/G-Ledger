use std::{
    cell::RefCell,
    collections::BTreeMap,
    env,
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use project_ledger_agent_skill::{
    AGENT_SKILL_PROFILE, run_agent_skill_conformance, verify_agent_skill_package,
};
use project_ledger_application::{
    Clock, CommandDraft, CommandProcessor, CoreCommandHandler, CreateProjectHandler, FixedClock,
    prepare_command,
};
use project_ledger_canonical_json::{sha256_bytes, to_canonical_vec};
use project_ledger_continuity::{ResumeBudget, ResumeBuilder, render_agent_markdown};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult,
    CommandResultStatus, CreateContinuityCheckpointPayload, CreateProjectPayload, LedgerTimestamp,
    Phase, Project, ProjectId, ProjectLifecycleStatus, ResourceRef, SchemaVersion, StableId,
    StorageProfile, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_integration_protocol::{build_default_manifest, inspect_upstream_lock};
use project_ledger_interop_gskills::{
    GSKILLS_PROFILE, GSkillsJsonRpcHandler, GSkillsProvider, GSkillsRecordSink,
    GSkillsRecordSinkJsonRpcHandler, ProviderContext, RecordSinkHost, RecordSinkHostError,
    RecordSinkPolicy, RecordSinkStateIdentity,
    verify_vendored_contracts as verify_gskills_vendored_contracts,
};
use project_ledger_interop_gsp::{
    GSP_CONTRACT_TAG, GspBridge, GspCompilationReceipt, GspCorpus, GspCorpusRequest,
    verify_vendored_contracts as verify_gsp_vendored_contracts,
};
use project_ledger_interop_radar::{
    RADAR_PROFILE, RadarContractCatalog, RadarEnvelopeRequest, RadarExportContext,
    build_project_projection, build_sync_envelope, validate_sync_envelope,
    verify_profile_contracts as verify_radar_profile_contracts,
};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_release::verify_release;
use project_ledger_schemas::{check_generated, validate_contracts};
use project_ledger_storage_sqlite::SqliteEventStore;
use project_ledger_transport_stdio::{FrameLimits, JsonRpcHandler, dispatch_frame};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const V0_1_DIGEST: &str = "sha256:772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865";
const V0_2_DIGEST: &str = "sha256:31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9";

#[derive(Debug, Parser)]
#[command(
    name = "project-ledger",
    version,
    about = "Verifiable local project continuity"
)]
struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

#[derive(Debug, Subcommand)]
enum TopLevelCommand {
    Init(InitArgs),
    Resume(ResumeArgs),
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommand,
    },
    Specs {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Contracts {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    Integration {
        #[command(subcommand)]
        command: IntegrationCommand,
    },
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
    Serve {
        #[command(subcommand)]
        command: ServeCommand,
    },
}

#[derive(Debug, Args)]
struct InitArgs {
    #[arg(long)]
    title: String,
    #[arg(long, default_value = "")]
    summary: String,
    #[arg(long, default_value = "UTC")]
    timezone: String,
    #[arg(long)]
    data_root: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum VerificationCommand {
    Verify,
}

#[derive(Clone, Debug, Subcommand)]
enum ReleaseCommand {
    Verify(ReleaseVerifyArgs),
}

#[derive(Clone, Debug, Args)]
struct ReleaseVerifyArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Clone, Debug, Subcommand)]
enum IntegrationCommand {
    Manifest,
    Compatibility,
    AgentSkill {
        #[command(subcommand)]
        command: AgentSkillIntegrationCommand,
    },
    Gskills {
        #[command(subcommand)]
        command: GSkillsIntegrationCommand,
    },
    Gsp {
        #[command(subcommand)]
        command: GspIntegrationCommand,
    },
    Radar {
        #[command(subcommand)]
        command: RadarIntegrationCommand,
    },
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum GSkillsIntegrationCommand {
    VerifyContracts,
}

#[derive(Clone, Debug, Subcommand)]
enum GspIntegrationCommand {
    VerifyContracts,
    ExportCorpus(GspExportCorpusArgs),
    ValidateResult(GspValidateResultArgs),
    LinkResult(GspLinkResultArgs),
}

#[derive(Clone, Debug, Subcommand)]
enum RadarIntegrationCommand {
    VerifyContracts,
    Export(RadarExportArgs),
    ValidateExport(RadarValidateExportArgs),
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum AgentSkillIntegrationCommand {
    VerifyContracts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ResumeFormat {
    Codex,
    Json,
}

#[derive(Clone, Debug, Args)]
struct ResumeArgs {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long, value_enum, default_value = "codex")]
    format: ResumeFormat,
}

#[derive(Clone, Debug, Subcommand)]
enum CheckpointCommand {
    Create(CheckpointCreateArgs),
}

#[derive(Clone, Debug, Args)]
struct CheckpointCreateArgs {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    reason: String,
    #[arg(long)]
    expected_version: u64,
    #[arg(long)]
    idempotency_key: String,
    #[arg(long)]
    actor_id: String,
    #[arg(long)]
    observed_at: String,
}

#[derive(Clone, Debug, Subcommand)]
enum CatalogCommand {
    RadarExport(RadarCatalogExportArgs),
}

#[derive(Clone, Debug, Args)]
struct RadarExportArgs {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    phase_catalog: PathBuf,
    #[arg(long)]
    source_instance_ref: String,
    #[arg(long)]
    observed_at: String,
    #[arg(long = "source-alias")]
    source_aliases: Vec<String>,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct RadarValidateExportArgs {
    #[arg(long)]
    input: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct RadarCatalogExportArgs {
    #[arg(long)]
    all: bool,
    #[arg(long)]
    catalog: PathBuf,
    #[arg(long)]
    source_instance_ref: String,
    #[arg(long)]
    observed_at: String,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RadarCatalogSource {
    database: PathBuf,
    project_ref: String,
    phase_catalog: PathBuf,
    #[serde(default)]
    source_aliases: Vec<String>,
}

#[derive(Clone, Debug, Args)]
struct GspExportCorpusArgs {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    request: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct GspValidateResultArgs {
    #[arg(long)]
    evidence_bundle: PathBuf,
    #[arg(long)]
    result: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct GspLinkResultArgs {
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    receipt: PathBuf,
    #[arg(long)]
    receipt_uri: String,
    #[arg(long)]
    expected_state_version: u64,
    #[arg(long)]
    idempotency_key: String,
    #[arg(long)]
    actor_id: String,
    #[arg(long)]
    observed_at: String,
}

#[derive(Debug, Subcommand)]
enum ServeCommand {
    Provider(ServeProviderArgs),
    Sink(ServeSinkArgs),
}

#[derive(Debug, Args)]
struct ServeProviderArgs {
    #[arg(long)]
    stdio: bool,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    gskills_project_ref: String,
    #[arg(long)]
    phase_catalog: PathBuf,
    #[arg(long)]
    observed_at: String,
}

#[derive(Debug, Args)]
struct ServeSinkArgs {
    #[arg(long)]
    stdio: bool,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    database: PathBuf,
    #[arg(long)]
    project_ref: String,
    #[arg(long)]
    gskills_project_ref: String,
    #[arg(long)]
    policy: PathBuf,
    #[arg(long)]
    observed_at: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        TopLevelCommand::Init(args) => initialize_project(args),
        TopLevelCommand::Resume(args) => resume_project(&args),
        TopLevelCommand::Checkpoint {
            command: CheckpointCommand::Create(args),
        } => create_checkpoint(&args),
        TopLevelCommand::Specs {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_specs(&root)),
        TopLevelCommand::Contracts {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_contract_artifacts(&root)),
        TopLevelCommand::Release {
            command: ReleaseCommand::Verify(args),
        } => verify_release_cli(&args),
        TopLevelCommand::Integration {
            command: IntegrationCommand::Manifest,
        } => integration_manifest(),
        TopLevelCommand::Integration {
            command: IntegrationCommand::Compatibility,
        } => repository_root().and_then(|root| integration_compatibility(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::AgentSkill {
                    command: AgentSkillIntegrationCommand::VerifyContracts,
                },
        } => repository_root().and_then(|root| verify_agent_skill_contracts(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gskills {
                    command: GSkillsIntegrationCommand::VerifyContracts,
                },
        } => repository_root().and_then(|root| verify_gskills_contracts(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gsp {
                    command: GspIntegrationCommand::VerifyContracts,
                },
        } => repository_root().and_then(|root| verify_gsp_contracts(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gsp {
                    command: GspIntegrationCommand::ExportCorpus(args),
                },
        } => export_gsp_corpus(&args),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gsp {
                    command: GspIntegrationCommand::ValidateResult(args),
                },
        } => repository_root().and_then(|root| validate_gsp_result(&root, &args)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gsp {
                    command: GspIntegrationCommand::LinkResult(args),
                },
        } => link_gsp_result(&args),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Radar {
                    command: RadarIntegrationCommand::VerifyContracts,
                },
        } => repository_root().and_then(|root| verify_radar_contracts(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Radar {
                    command: RadarIntegrationCommand::Export(args),
                },
        } => export_radar_project(&args),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Radar {
                    command: RadarIntegrationCommand::ValidateExport(args),
                },
        } => repository_root().and_then(|root| validate_radar_export(&root, &args)),
        TopLevelCommand::Catalog {
            command: CatalogCommand::RadarExport(args),
        } => export_radar_catalog(&args),
        TopLevelCommand::Serve {
            command: ServeCommand::Provider(args),
        } => serve_gskills_provider(args),
        TopLevelCommand::Serve {
            command: ServeCommand::Sink(args),
        } => serve_gskills_sink(&args),
    };
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn repository_root() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("cannot determine repository root: {error}"))
}

fn verify_release_cli(args: &ReleaseVerifyArgs) -> Result<String, String> {
    let report = verify_release(&args.root).map_err(|error| error.to_string())?;
    canonical_stdout(&report)
}

fn initialize_project(args: InitArgs) -> Result<String, String> {
    let timestamp = current_timestamp()?;
    let project_ref = ProjectId::new();
    let base = match args.data_root {
        Some(path) => path,
        None => default_data_root()?,
    };
    // The UUID component is platform-safe; the canonical `project:` prefix remains in contracts.
    let database = base
        .join("project-ledger")
        .join("projects")
        .join(project_ref.uuid().to_string())
        .join("ledger.db");
    let project = Project {
        project_id: project_ref.clone(),
        title: args.title,
        summary: args.summary,
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp.clone(),
        timezone: args.timezone,
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    };
    let command_id = CommandId::new();
    let idempotency_key = StableId::parse(&format!("init:{}", command_id.uuid()))
        .map_err(|error| error.to_string())?;
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id,
        project_ref: project_ref.clone(),
        expected_state_version: 0,
        idempotency_key,
        actor: Actor {
            actor_id: StableId::parse("system:project-ledger-cli")
                .map_err(|error| error.to_string())?,
            actor_kind: ActorKind::System,
            trust_level: TrustLevel::LocalUser,
            display_name: Some("Project Ledger CLI".to_owned()),
        },
        command: Command::CreateProject(CreateProjectPayload { project }),
        submitted_at: timestamp.clone(),
    })
    .map_err(|error| error.to_string())?;
    let mut store = SqliteEventStore::open(&database).map_err(|error| error.to_string())?;
    let clock = FixedClock::new(timestamp);
    let result = CommandProcessor::new(&mut store, &clock)
        .execute(command, &CreateProjectHandler)
        .map_err(|error| error.to_string())?;
    if result.status != CommandResultStatus::Accepted {
        return Err(format!(
            "project initialization was not accepted: {:?}",
            result.status
        ));
    }
    let recovery = store
        .recovery_report(&project_ref)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "project_ref={project_ref}\ndatabase={}\nstate_version={}\n",
        database.display(),
        recovery.current_version
    ))
}

fn resume_project(args: &ResumeArgs) -> Result<String, String> {
    require_file(&args.database, "Project Ledger database")?;
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(&project_ref)
        .map_err(|error| error.to_string())?;
    let stream = store
        .load_stream(&project_ref)
        .map_err(|error| error.to_string())?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let budget = match args.format {
        ResumeFormat::Codex => ResumeBudget::AgentMarkdown,
        ResumeFormat::Json => ResumeBudget::Compact,
    };
    let resume = ResumeBuilder::build(&projection, budget).map_err(|error| error.to_string())?;
    match args.format {
        ResumeFormat::Codex => Ok(render_agent_markdown(&resume)),
        ResumeFormat::Json => canonical_stdout(&resume),
    }
}

fn create_checkpoint(args: &CheckpointCreateArgs) -> Result<String, String> {
    require_file(&args.database, "Project Ledger database")?;
    if args.reason.trim().is_empty() {
        return Err("checkpoint reason cannot be empty".to_owned());
    }
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let idempotency_key =
        StableId::parse(&args.idempotency_key).map_err(|error| error.to_string())?;
    let actor_id = StableId::parse(&args.actor_id).map_err(|error| error.to_string())?;
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let command_id = deterministic_cli_command_id("checkpoint", &project_ref, &idempotency_key)?;
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id,
        project_ref: project_ref.clone(),
        expected_state_version: args.expected_version,
        idempotency_key,
        actor: Actor {
            actor_id,
            actor_kind: ActorKind::Agent,
            trust_level: TrustLevel::LocalUser,
            display_name: Some("Project Ledger Agent Skill".to_owned()),
        },
        command: Command::CreateContinuityCheckpoint(CreateContinuityCheckpointPayload {
            reason: args.reason.clone(),
        }),
        submitted_at: observed_at.clone(),
    })
    .map_err(|error| error.to_string())?;
    let mut store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    let result = CommandProcessor::new(&mut store, &FixedClock::new(observed_at))
        .execute(command, &CoreCommandHandler)
        .map_err(|error| error.to_string())?;
    if !matches!(
        result.status,
        CommandResultStatus::Accepted | CommandResultStatus::DuplicateReplay
    ) {
        let status = serde_json::to_value(result.status)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{:?}", result.status));
        return Err(format!(
            "checkpoint was not accepted: {status} {:?}",
            result.rejection
        ));
    }
    canonical_stdout(&result)
}

fn deterministic_cli_command_id(
    operation: &str,
    project_ref: &ProjectId,
    idempotency_key: &StableId,
) -> Result<CommandId, String> {
    let seed = format!("cli:{operation}:{project_ref}:{idempotency_key}");
    let digest = sha256_bytes(seed.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let uuid = uuid::Uuid::from_bytes(bytes);
    CommandId::parse(&format!("command:{uuid}")).map_err(|error| error.to_string())
}

fn current_timestamp() -> Result<LedgerTimestamp, String> {
    let encoded = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("cannot format current timestamp: {error}"))?;
    LedgerTimestamp::parse(&encoded).map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn default_data_root() -> Result<PathBuf, String> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "cannot resolve the Windows user-data directory from APPDATA".to_owned())
}

#[cfg(target_os = "macos")]
fn default_data_root() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support"))
        .ok_or_else(|| "cannot resolve the macOS user-data directory from HOME".to_owned())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_data_root() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share"))
        .ok_or_else(|| "cannot resolve the Unix user-data directory".to_owned())
}

#[cfg(not(any(unix, target_os = "windows")))]
fn default_data_root() -> Result<PathBuf, String> {
    Err("this operating system has no configured user-data convention".to_owned())
}

fn verify_specs(root: &Path) -> Result<String, String> {
    let versions = [
        (
            "v0.1.0",
            root.join("specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md"),
            V0_1_DIGEST,
        ),
        (
            "v0.2.0",
            root.join("specs/PROJECT_LEDGER_SPEC_v0.2.0.md"),
            V0_2_DIGEST,
        ),
    ];
    let mut output = String::new();
    for (version, path, expected) in versions {
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let actual = sha256_bytes(&bytes).to_string();
        if actual != expected {
            return Err(format!(
                "{version} MISMATCH expected={expected} actual={actual}"
            ));
        }
        writeln!(&mut output, "{version} MATCH {}", &actual[7..15])
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

fn verify_contract_artifacts(root: &Path) -> Result<String, String> {
    check_generated(root).map_err(|error| error.to_string())?;
    let report = validate_contracts().map_err(|error| error.to_string())?;
    Ok(format!(
        "contracts MATCH schemas={} fixtures={} matrix_rows={}\n",
        report.schema_count,
        report.positive_fixture_count + report.negative_fixture_count,
        report.matrix_row_count
    ))
}

fn integration_manifest() -> Result<String, String> {
    let manifest = build_default_manifest(Vec::new()).map_err(|error| error.to_string())?;
    canonical_stdout(&manifest)
}

#[derive(Debug, Serialize)]
struct CompatibilityReport {
    schema_version: &'static str,
    certified_compatibility: bool,
    integrations: Vec<project_ledger_integration_protocol::UpstreamLockInspection>,
}

fn integration_compatibility(root: &Path) -> Result<String, String> {
    let mut integrations = Vec::new();
    for relative in [
        "integrations/gskills/upstream-lock.json",
        "integrations/gsp/upstream-lock.json",
        "integrations/project-radar/upstream-lock.json",
        "integrations/agent-skills/profile-lock.json",
    ] {
        let path = root.join(relative);
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        integrations.push(inspect_upstream_lock(&value).map_err(|error| error.to_string())?);
    }
    canonical_stdout(&CompatibilityReport {
        schema_version: "1.0.0",
        certified_compatibility: integrations
            .iter()
            .all(|inspection| inspection.certified_compatibility),
        integrations,
    })
}

fn verify_gskills_contracts(root: &Path) -> Result<String, String> {
    let report = verify_gskills_vendored_contracts(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "gskills contracts MATCH schemas={} fixtures={} profile={}\n",
        report.schema_count,
        report.positive_fixture_count + report.negative_fixture_count,
        GSKILLS_PROFILE
    ))
}

fn verify_gsp_contracts(root: &Path) -> Result<String, String> {
    let report = verify_gsp_vendored_contracts(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "gsp contracts MATCH registry={} schemas={} fixtures={} profile={}\n",
        report.registry_contract_count,
        report.schema_artifact_count,
        report.valid_fixture_count + report.invalid_fixture_count,
        GSP_CONTRACT_TAG
    ))
}

fn verify_radar_contracts(root: &Path) -> Result<String, String> {
    let report = verify_radar_profile_contracts(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "radar contracts MATCH schemas={} fixtures={} profile={}\n",
        report.schema_count,
        report.valid_fixture_count + report.invalid_fixture_count,
        RADAR_PROFILE
    ))
}

fn verify_agent_skill_contracts(root: &Path) -> Result<String, String> {
    let package = verify_agent_skill_package(root).map_err(|error| error.to_string())?;
    let conformance = run_agent_skill_conformance(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "agent skill MATCH schemas={} triggers={} policies={} profile={}\n",
        package.schema_count,
        package.trigger_case_count,
        conformance.policy_case_count,
        AGENT_SKILL_PROFILE
    ))
}

fn export_radar_project(args: &RadarExportArgs) -> Result<String, String> {
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let source_instance_ref =
        ResourceRef::parse(&args.source_instance_ref).map_err(|error| error.to_string())?;
    let project = radar_project_from_sqlite(
        &args.database,
        &project_ref,
        &args.phase_catalog,
        &args.source_aliases,
        &observed_at,
    )?;
    let state_version = project.source.state_version;
    let envelope = build_sync_envelope(
        vec![project],
        &RadarEnvelopeRequest {
            sync_envelope_id: StableId::parse(&format!(
                "radar-sync:{}:{state_version}",
                project_ref.uuid()
            ))
            .map_err(|error| error.to_string())?,
            source_instance_ref,
            exported_at: observed_at,
        },
    )
    .map_err(|error| error.to_string())?;
    validate_sync_envelope(&envelope).map_err(|error| error.to_string())?;
    write_new_json(&args.output, &envelope)?;
    Ok(format!(
        "radar export EXPORTED projects=1 output={}\n",
        args.output.display()
    ))
}

fn validate_radar_export(root: &Path, args: &RadarValidateExportArgs) -> Result<String, String> {
    require_file(&args.input, "Project Radar export")?;
    let value: Value = read_json_file(&args.input)?;
    let catalog = RadarContractCatalog::load(root).map_err(|error| error.to_string())?;
    catalog
        .validate("radar-sync-envelope", &value)
        .map_err(|error| error.to_string())?;
    let envelope = serde_json::from_value(value).map_err(|error| error.to_string())?;
    validate_sync_envelope(&envelope).map_err(|error| error.to_string())?;
    Ok(format!(
        "radar export VALID projects={} profile={}\n",
        envelope.projects.len(),
        envelope.profile
    ))
}

fn export_radar_catalog(args: &RadarCatalogExportArgs) -> Result<String, String> {
    if !args.all {
        return Err("catalog radar-export requires explicit --all".to_owned());
    }
    require_file(&args.catalog, "Project Radar known-ledger catalog")?;
    let catalog_bytes =
        fs::read(&args.catalog).map_err(|error| format!("{}: {error}", args.catalog.display()))?;
    let sources: Vec<RadarCatalogSource> = serde_json::from_slice(&catalog_bytes)
        .map_err(|error| format!("{}: {error}", args.catalog.display()))?;
    if sources.is_empty() {
        return Err("Project Radar known-ledger catalog is empty".to_owned());
    }
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let source_instance_ref =
        ResourceRef::parse(&args.source_instance_ref).map_err(|error| error.to_string())?;
    let mut projects = Vec::with_capacity(sources.len());
    for source in sources {
        let project_ref =
            ProjectId::parse(&source.project_ref).map_err(|error| error.to_string())?;
        projects.push(radar_project_from_sqlite(
            &source.database,
            &project_ref,
            &source.phase_catalog,
            &source.source_aliases,
            &observed_at,
        )?);
    }
    let digest = sha256_bytes(&catalog_bytes).to_string();
    let envelope = build_sync_envelope(
        projects,
        &RadarEnvelopeRequest {
            sync_envelope_id: StableId::parse(&format!(
                "radar-sync:catalog:{}",
                digest.trim_start_matches("sha256:")
            ))
            .map_err(|error| error.to_string())?,
            source_instance_ref,
            exported_at: observed_at,
        },
    )
    .map_err(|error| error.to_string())?;
    write_new_json(&args.output, &envelope)?;
    Ok(format!(
        "radar catalog EXPORTED projects={} output={}\n",
        envelope.projects.len(),
        args.output.display()
    ))
}

fn radar_project_from_sqlite(
    database: &Path,
    project_ref: &ProjectId,
    phase_catalog_path: &Path,
    source_aliases: &[String],
    observed_at: &LedgerTimestamp,
) -> Result<project_ledger_domain::LedgerRadarProjectProjection, String> {
    require_file(database, "Project Radar export database")?;
    require_file(phase_catalog_path, "Project Radar phase catalog")?;
    let phase_catalog = load_phase_catalog(phase_catalog_path)?;
    let store = SqliteEventStore::open(database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(project_ref)
        .map_err(|error| error.to_string())?;
    let stream = store
        .load_stream(project_ref)
        .map_err(|error| error.to_string())?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let provenance_ref = ResourceRef::parse(&format!(
        "project-ledger://provenance/radar-export/{}/{}",
        project_ref.uuid(),
        projection.state().state_version
    ))
    .map_err(|error| error.to_string())?;
    build_project_projection(
        &projection,
        &RadarExportContext {
            phase_catalog,
            source_aliases: source_aliases.to_vec(),
            observed_at: observed_at.clone(),
            provenance_ref,
        },
    )
    .map_err(|error| error.to_string())
}

fn export_gsp_corpus(args: &GspExportCorpusArgs) -> Result<String, String> {
    require_file(&args.database, "GSP corpus database")?;
    require_file(&args.request, "GSP corpus request")?;
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let request: GspCorpusRequest = read_json_file(&args.request)?;
    if request.project_ref != project_ref {
        return Err("GSP corpus CLI project differs from the explicit request".to_owned());
    }
    let store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(&project_ref)
        .map_err(|error| error.to_string())?;
    let stream = store
        .load_stream(&project_ref)
        .map_err(|error| error.to_string())?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let corpus = project_ledger_interop_gsp::build_minimized_corpus(&projection, &stream, &request)
        .map_err(|error| error.to_string())?;
    write_new_json(&args.output, &corpus)?;
    Ok(format!(
        "gsp corpus EXPORTED files={} output={}\n",
        corpus.manifest.files.len(),
        args.output.display()
    ))
}

fn validate_gsp_result(root: &Path, args: &GspValidateResultArgs) -> Result<String, String> {
    require_file(&args.evidence_bundle, "GSP EvidenceBundle corpus")?;
    require_file(&args.result, "GSP CompilationResult")?;
    let corpus: GspCorpus = read_json_file(&args.evidence_bundle)?;
    let raw: Value = read_json_file(&args.result)?;
    let bridge = GspBridge::load(root).map_err(|error| error.to_string())?;
    let validated = bridge
        .validate_result(&corpus, raw)
        .map_err(|error| error.to_string())?;
    write_new_json(&args.output, &validated)?;
    Ok(format!(
        "gsp result VALIDATED contracts={} review={} output={}\n",
        validated.report.exact_contract_count,
        validated.report.conflict_visibility.requires_human_review,
        args.output.display()
    ))
}

fn link_gsp_result(args: &GspLinkResultArgs) -> Result<String, String> {
    require_file(&args.database, "GSP link database")?;
    require_file(&args.receipt, "GSP compilation receipt")?;
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let value: Value = read_json_file(&args.receipt)?;
    let receipt_value = value.get("receipt").cloned().unwrap_or(value);
    let receipt: GspCompilationReceipt =
        serde_json::from_value(receipt_value).map_err(|error| error.to_string())?;
    if receipt.project_ref != project_ref {
        return Err("GSP receipt project differs from the explicit CLI project".to_owned());
    }
    if receipt.project_state_version != args.expected_state_version {
        return Err("GSP receipt state version differs from --expected-state-version".to_owned());
    }
    if !receipt.verify_digest().map_err(|error| error.to_string())? {
        return Err("GSP compilation receipt digest mismatch".to_owned());
    }
    let artifact_uri = ResourceRef::parse(&args.receipt_uri).map_err(|error| error.to_string())?;
    let idempotency_key =
        StableId::parse(&args.idempotency_key).map_err(|error| error.to_string())?;
    let actor_id = StableId::parse(&args.actor_id).map_err(|error| error.to_string())?;
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let mut store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(&project_ref)
        .map_err(|error| error.to_string())?;
    let stream = store
        .load_stream(&project_ref)
        .map_err(|error| error.to_string())?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let state = projection.state();
    if state.state_version == receipt.project_state_version
        && (state.material_state_version != receipt.material_state_version
            || state.state_digest != receipt.project_state_digest)
    {
        return Err(format!(
            "GSP receipt state binding differs from the current Ledger projection: receipt=({},{},{}) ledger=({},{},{})",
            receipt.project_state_version,
            receipt.material_state_version,
            receipt.project_state_digest,
            state.state_version,
            state.material_state_version,
            state.state_digest
        ));
    }
    let command_id =
        deterministic_gsp_link_command_id(&project_ref, &idempotency_key, receipt.receipt_digest)?;
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id,
        project_ref: project_ref.clone(),
        expected_state_version: args.expected_state_version,
        idempotency_key,
        actor: Actor {
            actor_id,
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::LinkExternalEvidence(receipt.link_payload(artifact_uri)),
        submitted_at: observed_at.clone(),
    })
    .map_err(|error| error.to_string())?;
    let result = CommandProcessor::new(&mut store, &FixedClock::new(observed_at))
        .execute(command, &CoreCommandHandler)
        .map_err(|error| error.to_string())?;
    if !matches!(
        result.status,
        CommandResultStatus::Accepted | CommandResultStatus::DuplicateReplay
    ) {
        return Err(format!(
            "GSP receipt link was not accepted: {:?} {:?}",
            result.status, result.rejection
        ));
    }
    canonical_stdout(&result)
}

fn deterministic_gsp_link_command_id(
    project_ref: &ProjectId,
    idempotency_key: &StableId,
    receipt_digest: project_ledger_canonical_json::Sha256Digest,
) -> Result<CommandId, String> {
    let seed = format!("gsp-link:{project_ref}:{idempotency_key}:{receipt_digest}");
    let digest = sha256_bytes(seed.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let uuid = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    );
    CommandId::parse(&format!("command:{uuid}")).map_err(|error| error.to_string())
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} does not exist: {}", path.display()))
    }
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = to_canonical_vec(value).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("refusing to overwrite {}: {error}", path.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(b"\n")
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn serve_gskills_provider(args: ServeProviderArgs) -> Result<String, String> {
    if !args.stdio {
        return Err("provider service requires --stdio".to_owned());
    }
    if args.profile != GSKILLS_PROFILE {
        return Err(format!(
            "unsupported provider profile: expected {GSKILLS_PROFILE}, received {}",
            args.profile
        ));
    }
    if !args.database.is_file() {
        return Err(format!(
            "provider database does not exist: {}",
            args.database.display()
        ));
    }
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let phase_catalog = load_phase_catalog(&args.phase_catalog)?;
    let external_project_ref =
        ResourceRef::parse(&format!("project-ledger://projects/{}", project_ref.uuid()))
            .map_err(|error| error.to_string())?;
    let process_instance_id =
        StableId::parse("provider-process:cli").map_err(|error| error.to_string())?;

    let store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(&project_ref)
        .map_err(|error| error.to_string())?;
    let stream = store
        .load_stream(&project_ref)
        .map_err(|error| error.to_string())?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let provider = GSkillsProvider::new(
        projection,
        ProviderContext {
            gskills_project_ref: args.gskills_project_ref,
            external_project_ref,
            phase_catalog,
            process_instance_id,
            observed_at,
        },
    )
    .map_err(|error| error.to_string())?;
    let handler = GSkillsJsonRpcHandler::new(&provider);
    serve_stdio(&handler, FrameLimits::default())?;
    Ok(String::new())
}

fn serve_gskills_sink(args: &ServeSinkArgs) -> Result<String, String> {
    if !args.stdio {
        return Err("record sink service requires --stdio".to_owned());
    }
    if args.profile != GSKILLS_PROFILE {
        return Err(format!(
            "unsupported record sink profile: expected {GSKILLS_PROFILE}, received {}",
            args.profile
        ));
    }
    if !args.database.is_file() {
        return Err(format!(
            "record sink database does not exist: {}",
            args.database.display()
        ));
    }
    if !args.policy.is_file() {
        return Err(format!(
            "explicit record sink policy does not exist: {}",
            args.policy.display()
        ));
    }
    let project_ref = ProjectId::parse(&args.project_ref).map_err(|error| error.to_string())?;
    let observed_at =
        LedgerTimestamp::parse(&args.observed_at).map_err(|error| error.to_string())?;
    let policy = load_record_sink_policy(&args.policy)?;
    if policy.native_project_ref != project_ref
        || policy.gskills_project_ref != args.gskills_project_ref
    {
        return Err("record sink CLI identity differs from the explicit policy".to_owned());
    }

    let store = SqliteEventStore::open(&args.database).map_err(|error| error.to_string())?;
    store
        .verify_event_chain(&project_ref)
        .map_err(|error| error.to_string())?;
    let host = SqliteRecordSinkHost {
        store: RefCell::new(store),
        project_ref,
        clock: FixedClock::new(observed_at),
    };
    // Only this explicit, digest-valid policy can construct a mutable sink.
    let sink = GSkillsRecordSink::new(&host, policy).map_err(|error| error.to_string())?;
    let handler = GSkillsRecordSinkJsonRpcHandler::new(&sink);
    serve_stdio(&handler, FrameLimits::default())?;
    Ok(String::new())
}

fn load_record_sink_policy(path: &Path) -> Result<RecordSinkPolicy, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    RecordSinkPolicy::from_value(&value).map_err(|error| error.to_string())
}

#[derive(Debug)]
struct SqliteRecordSinkHost {
    store: RefCell<SqliteEventStore>,
    project_ref: ProjectId,
    clock: FixedClock,
}

impl SqliteRecordSinkHost {
    fn projection(&self) -> Result<ProjectStateProjection, RecordSinkHostError> {
        let store = self.store.borrow();
        store
            .verify_event_chain(&self.project_ref)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))?;
        let stream = store
            .load_stream(&self.project_ref)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))?;
        ProjectStateProjection::rebuild(&stream)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }
}

impl RecordSinkHost for SqliteRecordSinkHost {
    fn observed_at(&self) -> LedgerTimestamp {
        self.clock.now()
    }

    fn state_identity(
        &self,
        project_ref: &ProjectId,
    ) -> Result<RecordSinkStateIdentity, RecordSinkHostError> {
        if project_ref != &self.project_ref {
            return Err(RecordSinkHostError::new(
                "record sink requested an unexpected project",
            ));
        }
        let projection = self.projection()?;
        Ok(RecordSinkStateIdentity {
            state_version: projection.state().state_version,
            material_state_version: projection.state().material_state_version,
            state_digest: projection.state().state_digest,
        })
    }

    fn execute(&self, command: CommandEnvelope) -> Result<CommandResult, RecordSinkHostError> {
        CommandProcessor::new(&mut *self.store.borrow_mut(), &self.clock)
            .execute(command, &CoreCommandHandler)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }

    fn command_result(
        &self,
        command_ref: &CommandId,
    ) -> Result<Option<CommandResult>, RecordSinkHostError> {
        EventStore::command_result(&*self.store.borrow(), command_ref)
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }

    fn confirmed_projection_ref(
        &self,
        project_ref: &ProjectId,
        state_version: u64,
    ) -> Result<ResourceRef, RecordSinkHostError> {
        let state = self.state_identity(project_ref)?;
        if state.state_version != state_version {
            return Err(RecordSinkHostError::new(format!(
                "confirmed projection version mismatch: expected {state_version}, actual {}",
                state.state_version
            )));
        }
        ResourceRef::parse(&format!("continuity:ledger-{state_version}"))
            .map_err(|error| RecordSinkHostError::new(error.to_string()))
    }
}

fn load_phase_catalog(path: &Path) -> Result<BTreeMap<StableId, Phase>, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let phases: Vec<Phase> =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut catalog = BTreeMap::new();
    for phase in phases {
        if catalog.insert(phase.phase_id.clone(), phase).is_some() {
            return Err("phase catalog contains a duplicate phase identifier".to_owned());
        }
    }
    Ok(catalog)
}

fn serve_stdio<H: JsonRpcHandler>(handler: &H, limits: FrameLimits) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    while let Some(frame) = read_frame(&mut reader, limits)? {
        let response =
            dispatch_frame(&frame, limits, handler).map_err(|error| error.to_string())?;
        writer
            .write_all(&response)
            .map_err(|error| format!("cannot write stdio response: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("cannot flush stdio response: {error}"))?;
    }
    Ok(())
}

fn read_frame<R: BufRead>(reader: &mut R, limits: FrameLimits) -> Result<Option<Vec<u8>>, String> {
    let mut header = Vec::new();
    if reader
        .read_until(b'\n', &mut header)
        .map_err(|error| format!("cannot read stdio header: {error}"))?
        == 0
    {
        return Ok(None);
    }
    if header.len() > limits.max_header_bytes || !header.ends_with(b"\r\n") {
        return Err("invalid or oversized stdio Content-Length header".to_owned());
    }
    let mut terminator = Vec::new();
    reader
        .read_until(b'\n', &mut terminator)
        .map_err(|error| format!("cannot read stdio header terminator: {error}"))?;
    if terminator != b"\r\n" || header.len() + terminator.len() > limits.max_header_bytes {
        return Err("stdio framing requires one Content-Length header".to_owned());
    }
    let header_text = std::str::from_utf8(&header)
        .map_err(|_| "stdio Content-Length header is not UTF-8".to_owned())?;
    let content_length = header_text
        .strip_prefix("Content-Length: ")
        .and_then(|value| value.strip_suffix("\r\n"))
        .ok_or_else(|| "invalid stdio Content-Length header".to_owned())?
        .parse::<usize>()
        .map_err(|_| "invalid stdio Content-Length value".to_owned())?;
    if content_length > limits.max_content_bytes {
        return Err(format!(
            "stdio content exceeds {} bytes",
            limits.max_content_bytes
        ));
    }
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("cannot read complete stdio body: {error}"))?;
    header.extend_from_slice(&terminator);
    header.extend_from_slice(&body);
    Ok(Some(header))
}

fn canonical_stdout<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = to_canonical_vec(value).map_err(|error| error.to_string())?;
    let mut output = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    output.push('\n');
    Ok(output)
}
