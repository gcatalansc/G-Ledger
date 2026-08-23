use std::{
    cell::RefCell,
    collections::BTreeMap,
    env,
    fmt::Write as _,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand};
use project_ledger_application::{
    Clock, CommandDraft, CommandProcessor, CoreCommandHandler, CreateProjectHandler, FixedClock,
    prepare_command,
};
use project_ledger_canonical_json::{sha256_bytes, to_canonical_vec};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult,
    CommandResultStatus, CreateProjectPayload, LedgerTimestamp, Phase, Project, ProjectId,
    ProjectLifecycleStatus, ResourceRef, SchemaVersion, StableId, StorageProfile, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_integration_protocol::{build_default_manifest, inspect_upstream_lock};
use project_ledger_interop_gskills::{
    GSKILLS_PROFILE, GSkillsJsonRpcHandler, GSkillsProvider, GSkillsRecordSink,
    GSkillsRecordSinkJsonRpcHandler, ProviderContext, RecordSinkHost, RecordSinkHostError,
    RecordSinkPolicy, RecordSinkStateIdentity, verify_vendored_contracts,
};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_schemas::{check_generated, validate_contracts};
use project_ledger_storage_sqlite::SqliteEventStore;
use project_ledger_transport_stdio::{FrameLimits, JsonRpcHandler, dispatch_frame};
use serde::Serialize;
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
    Specs {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Contracts {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Integration {
        #[command(subcommand)]
        command: IntegrationCommand,
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
enum IntegrationCommand {
    Manifest,
    Compatibility,
    Gskills {
        #[command(subcommand)]
        command: GSkillsIntegrationCommand,
    },
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum GSkillsIntegrationCommand {
    VerifyContracts,
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
        TopLevelCommand::Specs {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_specs(&root)),
        TopLevelCommand::Contracts {
            command: VerificationCommand::Verify,
        } => repository_root().and_then(|root| verify_contract_artifacts(&root)),
        TopLevelCommand::Integration {
            command: IntegrationCommand::Manifest,
        } => integration_manifest(),
        TopLevelCommand::Integration {
            command: IntegrationCommand::Compatibility,
        } => repository_root().and_then(|root| integration_compatibility(&root)),
        TopLevelCommand::Integration {
            command:
                IntegrationCommand::Gskills {
                    command: GSkillsIntegrationCommand::VerifyContracts,
                },
        } => repository_root().and_then(|root| verify_gskills_contracts(&root)),
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
    let report = verify_vendored_contracts(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "gskills contracts MATCH schemas={} fixtures={} profile={}\n",
        report.schema_count,
        report.positive_fixture_count + report.negative_fixture_count,
        GSKILLS_PROFILE
    ))
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
