use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand};
use project_ledger_application::{
    CommandDraft, CommandProcessor, CreateProjectHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::sha256_bytes;
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, LedgerTimestamp, Project, ProjectId, ProjectLifecycleStatus,
    SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_schemas::{check_generated, validate_contracts};
use project_ledger_storage_sqlite::SqliteEventStore;
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
    Specs {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Contracts {
        #[command(subcommand)]
        command: VerificationCommand,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        TopLevelCommand::Init(args) => initialize_project(args),
        TopLevelCommand::Specs {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_specs(&root)),
        TopLevelCommand::Contracts {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_contract_artifacts(&root)),
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
