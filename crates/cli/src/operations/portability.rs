use super::{CliError, Selection};
use clap::Args;
use project_ledger_canonical_json::{sha256_digest, to_canonical_vec};
use project_ledger_continuity::{
    MarkdownRenderRequest, MarkdownRenderer, RenderedMarkdown, RendererPrivacyProfile,
    ResumeBudget, ResumeBuilder, audit_projection, verify_stream_and_artifact,
};
use project_ledger_domain::{
    EventEnvelope, GeneratedArtifactRecord, PortableImportPolicy, ResourceRef, StableId,
};
use project_ledger_event_store::EventStore;
use project_ledger_portable_bundle::{
    PortableBundle, PortableBundleError, PortableExportRequest, PortableImportOutcome, apply_import,
};
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_release::PrivacyScanner;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub artifact: Option<PathBuf>,
}
#[derive(Debug, Args)]
pub struct RenderArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub regenerate: bool,
    #[arg(long, default_value="minimal-local", value_parser=["minimal-local","shareable","full-local"])]
    pub privacy: String,
}
#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long, default_value="portable", value_parser=["portable"])]
    pub format: String,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub phase_catalog: Option<PathBuf>,
}
#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub database: PathBuf,
    #[arg(long)]
    pub project_ref: Option<String>,
    #[arg(long, default_value="verify-fast-forward", value_parser=["verify-fast-forward","fork"])]
    pub mode: String,
}
#[derive(Debug, Args)]
pub struct BackupArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub output: PathBuf,
}
#[derive(Debug, Args)]
pub struct RestoreArgs {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub database: PathBuf,
}
#[derive(Debug, Args)]
pub struct MigrateArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub backup: PathBuf,
}

pub fn backup(args: &BackupArgs) -> Result<String, CliError> {
    let (path, id) = args.selection.resolve()?;
    let store = SqliteEventStore::open_for_recovery(&path)?;
    if !store.project_ids()?.contains(&id) {
        return Err(CliError::new(7, "IDENTITY_UNRESOLVED", "project is absent"));
    }
    json_out(&store.backup_to(&args.output)?)
}
pub fn restore(args: &RestoreArgs) -> Result<String, CliError> {
    json_out(&SqliteEventStore::restore_backup(
        &args.file,
        &args.database,
    )?)
}
pub fn migrate(args: &MigrateArgs) -> Result<String, CliError> {
    let (path, id) = args.selection.resolve()?;
    if !SqliteEventStore::open_for_recovery(&path)?
        .project_ids()?
        .contains(&id)
    {
        return Err(CliError::new(7, "IDENTITY_UNRESOLVED", "project is absent"));
    }
    let backup = SqliteEventStore::migrate_with_backup(&path, &args.backup)?;
    let current = SqliteEventStore::open_read_only(&path)?.migration_version()?;
    json_out(
        &json!({"previous_migration":backup.migration_version,"migration_version":current,"backup":backup}),
    )
}

pub fn verify(args: &VerifyArgs) -> Result<String, CliError> {
    let (store, p) = args.selection.load()?;
    let recovery = store.recovery_report(&p.state().project.project_id)?;
    if let Some(path) = &args.artifact {
        let record: GeneratedArtifactRecord =
            serde_json::from_slice(&read_bounded(&sidecar(path), 65_536)?)
                .map_err(|_| divergence())?;
        let bytes = read_bounded(path, 4 * 1024 * 1024)?;
        let stream = store.load_stream(&p.state().project.project_id)?;
        let expected = markdown(&p, &stream, profile(&record.privacy_profile)?)?;
        if !verify_stream_and_artifact(&stream, &record, &bytes).valid
            || record != expected.record
            || bytes != expected.content.as_bytes()
        {
            return Err(divergence());
        }
    }
    let findings = audit_projection(&p).findings.into_iter().map(|f| json!({
        "code":format!("{:?}", f.code),"severity":format!("{:?}",f.severity),"message":f.message,"subject_refs":f.subject_refs})).collect::<Vec<_>>();
    json_out(
        &json!({"integrity":recovery.quick_check,"project_ref":p.state().project.project_id,
        "state_version":p.state().state_version,"head_digest":p.head_digest(),"migration_version":recovery.migration_version,
        "command_count":recovery.command_count,"command_result_count":recovery.command_result_count,
        "unresolved_submission_count":recovery.unresolved_submission_count,"execution_attempt_count":recovery.execution_attempt_count,
        "findings":findings,"read_only":true}),
    )
}

pub fn render(args: &RenderArgs) -> Result<String, CliError> {
    let (store, p) = args.selection.load()?;
    let stream = store.load_stream(&p.state().project.project_id)?;
    let output = markdown(&p, &stream, profile(&args.privacy)?)?;
    scan(output.content.as_bytes())?;
    let record_path = sidecar(&args.output);
    let exists =
        fs::symlink_metadata(&args.output).is_ok() || fs::symlink_metadata(&record_path).is_ok();
    if exists {
        if !args.regenerate {
            return Err(CliError::input(
                "output already exists; use --regenerate for a generated artifact",
            ));
        }
        for path in [&args.output, &record_path] {
            if fs::symlink_metadata(path)
                .map_err(|_| divergence())?
                .file_type()
                .is_symlink()
            {
                return Err(divergence());
            }
        }
        let previous: GeneratedArtifactRecord =
            serde_json::from_slice(&read_bounded(&record_path, 65_536).map_err(|_| divergence())?)
                .map_err(|_| divergence())?;
        let count = usize::try_from(previous.source_state_version).map_err(|_| divergence())?;
        let prefix = stream.get(..count).ok_or_else(divergence)?;
        let old_projection = ProjectStateProjection::rebuild(prefix).map_err(|_| divergence())?;
        let old = markdown(&old_projection, prefix, profile(&previous.privacy_profile)?)?;
        if old.record != previous {
            return Err(divergence());
        }
        // Only an explicitly selected, proven generated pair may be replaced.
        fs::write(&args.output, &output.content).map_err(CliError::storage)?;
        fs::write(
            &record_path,
            to_canonical_vec(&output.record).map_err(CliError::input)?,
        )
        .map_err(CliError::storage)?;
    } else {
        write_new(&args.output, output.content.as_bytes())?;
        write_new(
            &record_path,
            &to_canonical_vec(&output.record).map_err(CliError::input)?,
        )?;
    }
    json_out(&output.record)
}

fn markdown(
    p: &ProjectStateProjection,
    stream: &[EventEnvelope],
    privacy: RendererPrivacyProfile,
) -> Result<RenderedMarkdown, CliError> {
    let head = stream.last().ok_or_else(divergence)?;
    MarkdownRenderer::render(
        p,
        stream,
        &MarkdownRenderRequest {
            artifact_id: StableId::parse(&format!("generated:{}", p.state().state_digest))
                .map_err(CliError::input)?,
            uri: ResourceRef::parse(&format!(
                "urn:project-ledger:generated:{}",
                p.state().state_digest
            ))
            .map_err(CliError::input)?,
            renderer_version: env!("CARGO_PKG_VERSION").to_owned(),
            privacy_profile: privacy,
            created_at: head.recorded_at.clone(),
        },
    )
    .map_err(CliError::domain)
}
fn profile(value: &str) -> Result<RendererPrivacyProfile, CliError> {
    match value {
        "minimal-local" | "MINIMAL_LOCAL" => Ok(RendererPrivacyProfile::MinimalLocal),
        "shareable" | "SHAREABLE" => Ok(RendererPrivacyProfile::Shareable),
        "full-local" | "FULL_LOCAL" => Ok(RendererPrivacyProfile::FullLocal),
        _ => Err(divergence()),
    }
}
fn sidecar(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".record.json");
    PathBuf::from(name)
}
fn divergence() -> CliError {
    CliError::new(
        15,
        "GENERATED_ARTIFACT_DIVERGED",
        "generated artifact or its source differs; review and explicitly regenerate",
    )
}

pub fn export(args: &ExportArgs) -> Result<String, CliError> {
    let (store, p) = args.selection.load()?;
    let stream = store.load_stream(&p.state().project.project_id)?;
    let created_at = stream.last().ok_or_else(divergence)?.recorded_at.clone();
    let markdown = markdown(&p, &stream, RendererPrivacyProfile::Shareable)?;
    let schemas = project_ledger_schemas::schema_catalog().map_err(CliError::input)?.into_iter().map(|entry| {
        Ok(json!({"schema_id":entry.schema_id,"digest":sha256_digest(&entry.schema).map_err(CliError::input)?}))
    }).collect::<Result<Vec<_>,CliError>>()?;
    // These shipped locks are immutable resources, never read from the project cwd.
    let upstream_locks = [
        (
            "gsp.json",
            include_bytes!("../../../../integrations/gsp/upstream-lock.json").as_slice(),
        ),
        (
            "gskills.json",
            include_bytes!("../../../../integrations/gskills/upstream-lock.json").as_slice(),
        ),
        (
            "radar.json",
            include_bytes!("../../../../integrations/project-radar/upstream-lock.json").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        Ok((
            name.to_owned(),
            serde_json::from_slice(bytes).map_err(CliError::input)?,
        ))
    })
    .collect::<Result<BTreeMap<_, _>, CliError>>()?;
    let phase_catalog = args.phase_catalog.as_ref().map_or_else(
        || Ok(BTreeMap::new()),
        |path| crate::load_phase_catalog(path),
    )?;
    let radar = project_ledger_interop_radar::build_project_projection(
        &p,
        &project_ledger_interop_radar::RadarExportContext {
            phase_catalog,
            source_aliases: Vec::new(),
            observed_at: created_at.clone(),
            provenance_ref: ResourceRef::parse("urn:project-ledger:portable")
                .map_err(CliError::input)?,
        },
    )
    .map_err(CliError::domain)?;
    let bundle = PortableBundle::build(PortableExportRequest {
        export_id: StableId::parse(&format!("export:{}", p.state().state_digest))
            .map_err(CliError::input)?,
        created_at,
        import_policy: PortableImportPolicy::FastForward,
        stream,
        schemas_lock: json!({"schema_version":"0.2.0","schemas":schemas}),
        upstream_locks,
        resume: ResumeBuilder::build(&p, ResumeBudget::Structured).map_err(CliError::domain)?,
        evidence_references: json!(p.evidence_records()),
        radar_projection: json!(radar),
        generated_markdown: markdown.content.into_bytes(),
    })
    .map_err(portable_error)?;
    for bytes in bundle.entries().values() {
        scan(bytes)?;
    }
    bundle
        .write_directory(&args.output)
        .map_err(portable_error)?;
    json_out(bundle.manifest())
}

pub fn import(args: &ImportArgs) -> Result<String, CliError> {
    if args.mode == "fork" {
        return Err(CliError::new(
            2,
            "FORK_UNAVAILABLE",
            "v0.2 has no durable fork command; no data written",
        ));
    }
    let bundle = PortableBundle::read_directory(&args.file).map_err(portable_error)?;
    for bytes in bundle.entries().values() {
        scan(bytes)?;
    }
    let id = &bundle.manifest().project_ref;
    if args
        .project_ref
        .as_ref()
        .is_some_and(|expected| expected != id.as_str())
    {
        return Err(CliError::new(
            7,
            "IDENTITY_UNRESOLVED",
            "explicit project differs from bundle",
        ));
    }
    let outcome = if args.database.exists() {
        let mut store = SqliteEventStore::open_existing(&args.database)?;
        if store.project_ids()? != [id.clone()] {
            return Err(CliError::new(
                7,
                "IDENTITY_UNRESOLVED",
                "destination has another project identity",
            ));
        }
        let plan = bundle
            .plan_import(&store.load_stream(id)?, None)
            .map_err(portable_error)?;
        apply_import(&mut store, &plan).map_err(portable_error)?
    } else {
        let plan = bundle.plan_import(&[], None).map_err(portable_error)?;
        let parent = args
            .database
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let temp = tempfile::NamedTempFile::new_in(parent).map_err(CliError::storage)?;
        let mut store = SqliteEventStore::open(temp.path())?;
        let outcome = apply_import(&mut store, &plan).map_err(portable_error)?;
        store.recovery_report(id)?;
        drop(store);
        temp.as_file().sync_all().map_err(CliError::storage)?;
        temp.persist_noclobber(&args.database)
            .map_err(CliError::storage)?;
        outcome
    };
    let disposition = match outcome {
        PortableImportOutcome::IdenticalChain { .. } => "IDENTICAL_CHAIN",
        PortableImportOutcome::NewProject { .. } => "NEW_PROJECT",
        PortableImportOutcome::FastForward { .. } => "FAST_FORWARD",
        PortableImportOutcome::ForkExplicit { .. } => {
            return Err(CliError::input("fork unavailable"));
        }
    };
    json_out(
        &json!({"disposition":disposition,"project_ref":id,"state_version":bundle.manifest().project_state_version,
        "restores_command_results":false,"restores_execution_reservations":false}),
    )
}

#[allow(clippy::needless_pass_by_value)] // Result::map_err transfers ownership; diagnostic deliberately omits raw input.
fn portable_error(error: PortableBundleError) -> CliError {
    match error {
        PortableBundleError::PrivateMaterial { .. } => CliError::new(
            13,
            "PRIVACY_REJECTED",
            "portable history contains private material; use a local backup, not a rewritten event chain",
        ),
        PortableBundleError::DestinationExists { .. } => CliError::input("destination exists"),
        _ => CliError::new(
            15,
            "PORTABLE_INTEGRITY",
            "portable integrity, identity or chain transition rejected",
        ),
    }
}
pub(super) fn scan(bytes: &[u8]) -> Result<(), CliError> {
    if !PrivacyScanner.scan_bytes("export", bytes).is_empty() {
        return Err(CliError::new(
            13,
            "PRIVACY_REJECTED",
            "sensitive material cannot be exported",
        ));
    }
    Ok(())
}
pub(super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, CliError> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(CliError::input)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(CliError::input)?;
    if bytes.len() as u64 > limit {
        return Err(CliError::input("input exceeds explicit size limit"));
    }
    Ok(bytes)
}
pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(CliError::input)?;
    file.write_all(bytes).map_err(CliError::storage)?;
    file.sync_all().map_err(CliError::storage)
}
pub(super) fn json_out(value: &impl serde::Serialize) -> Result<String, CliError> {
    crate::canonical_stdout(value).map_err(CliError::input)
}
