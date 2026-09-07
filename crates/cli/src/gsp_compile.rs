//! Explicit, bounded compilation; no Ledger database capability or automatic linking.
use crate::errors::CliError;
use clap::Args;
use project_ledger_canonical_json::to_canonical_vec;
use project_ledger_evidence::supervisor::{ProcessPolicy, ProcessRequest, supervise};
use project_ledger_interop_gsp::{GspBridge, GspContractCatalog, GspCorpus};
use project_ledger_release::PrivacyScanner;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const INPUT_LIMIT: u64 = 1_048_576;
pub const RESULT_LIMIT: u64 = 8_388_608;
const SIDECAR: &[u8] = include_bytes!("../../../integrations/gsp/sidecar/compile_bundle.py");

#[derive(Clone, Debug, Args)]
pub struct CompileArgs {
    #[arg(long)]
    pub python: PathBuf,
    #[arg(long)]
    pub source_root: PathBuf,
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub policy: PathBuf,
    #[arg(long)]
    pub cwd: PathBuf,
    #[arg(long)]
    pub cancel_file: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    allowed_programs: BTreeSet<PathBuf>,
    #[serde(default)]
    process: ProcessPolicy,
}

#[allow(clippy::too_many_lines)] // Preflight, isolated compile, validation and publication stay in order.
pub fn compile(root: &Path, args: &CompileArgs) -> Result<String, CliError> {
    if fs::symlink_metadata(&args.output).is_ok() {
        return Err(CliError::input("GSP output already exists"));
    }
    let policy: Policy = read_json(&args.policy, 65_536)?;
    if !policy.allowed_programs.contains(&args.python) {
        return Err(CliError::new(
            9,
            "POLICY_DENIED",
            "Python is not in the explicit executable allowlist",
        ));
    }
    if !args.python.is_file() || !args.source_root.is_dir() {
        return Err(CliError::new(
            12,
            "GSP_RUNTIME_UNAVAILABLE",
            "explicit GSP runtime is absent; no download attempted",
        ));
    }
    if !args.source_root.is_absolute() {
        return Err(CliError::input("GSP source root must be absolute"));
    }
    let script = root.join("integrations/gsp/sidecar/compile_bundle.py");
    if fs::symlink_metadata(&script)
        .map_err(|_| rejected("GSP_SIDECAR_INTEGRITY"))?
        .file_type()
        .is_symlink()
        || read_bytes(&script, 65_536)? != SIDECAR
    {
        return Err(rejected("GSP_SIDECAR_INTEGRITY"));
    }
    let input = read_bytes(&args.input, INPUT_LIMIT)?;
    scan(&input)?;
    let corpus: GspCorpus = parse(&input)?;
    corpus
        .verify_integrity()
        .map_err(|_| rejected("GSP_CORPUS_REJECTED"))?;
    GspContractCatalog::load_vendored(root)
        .map_err(|_| rejected("GSP_CONTRACT_REJECTED"))?
        .validate("evidence.bundle", &corpus.evidence_bundle)
        .map_err(|_| rejected("GSP_CORPUS_REJECTED"))?;
    // Validate paths/policy before creating any temporary process artifacts.
    policy.process.validate(&ProcessRequest {
        program: args.python.clone(),
        args: vec![],
        working_directory: args.cwd.clone(),
    })?;
    let cancel = crate::process_cancel_token(args.cancel_file.clone())?;
    let temporary = tempfile::tempdir().map_err(CliError::storage)?;
    let input_path = temporary.path().join("corpus.json");
    let result_path = temporary.path().join("result.json");
    fs::write(&input_path, input).map_err(CliError::storage)?;
    let request = ProcessRequest {
        program: args.python.clone(),
        args: vec![
            "-I".into(),
            "-B".into(),
            script.to_string_lossy().into_owned(),
            "--source-root".into(),
            args.source_root.to_string_lossy().into_owned(),
            "--input".into(),
            input_path.to_string_lossy().into_owned(),
            "--output".into(),
            result_path.to_string_lossy().into_owned(),
        ],
        working_directory: args.cwd.clone(),
    };
    let output = supervise(&policy.process, &request, &cancel)?;
    if output.exit_code != Some(0) {
        // Whitelist data-free adapter diagnostics; never persist or echo raw output.
        let error = String::from_utf8_lossy(&output.stderr);
        if [
            "SOURCE_COMMIT_MISMATCH",
            "SOURCE_VERSION_MISMATCH",
            "SOURCE_CONTRACT_MISMATCH",
            "SOURCE_NOT_CLEAN",
        ]
        .iter()
        .any(|code| error.trim() == format!("GSP_SIDECAR_REJECTED:{code}"))
        {
            return Err(CliError::new(
                2,
                "GSP_RUNTIME_INCOMPATIBLE",
                "GSP source/version/contract is not the pinned clean runtime",
            ));
        }
        return Err(CliError::new(
            12,
            "GSP_PROCESS_FAILED",
            "GSP process failed; no result published or linked",
        ));
    }
    let bytes = read_bytes(&result_path, RESULT_LIMIT)?;
    scan(&bytes)?;
    let raw: Value = parse(&bytes)?;
    let validated = GspBridge::load(root)
        .map_err(|_| rejected("GSP_CONTRACT_REJECTED"))?
        .validate_result(&corpus, raw)
        .map_err(|_| rejected("GSP_RESULT_REJECTED"))?;
    publish(&args.output, &validated)?;
    crate::canonical_stdout(&json!({
        "status":"COMPILED_UNLINKED", "exact_contract_count":validated.report.exact_contract_count,
        "requires_human_review":validated.report.conflict_visibility.requires_human_review,
        "receipt_digest":validated.receipt.receipt_digest, "duration_ms":output.duration_ms,
        "ledger_event_delta":0, "authority_elevation_count":0, "certified_compatibility":false
    }))
    .map_err(CliError::input)
}

fn rejected(code: &'static str) -> CliError {
    CliError::new(
        15,
        code,
        "GSP artifact integrity, shape or byte budget rejected",
    )
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CliError> {
    serde_json::from_slice(bytes).map_err(|_| rejected("GSP_ARTIFACT_REJECTED"))
}

pub fn read_bytes(path: &Path, limit: u64) -> Result<Vec<u8>, CliError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| rejected("GSP_ARTIFACT_UNREADABLE"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return Err(rejected("GSP_ARTIFACT_REJECTED"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| rejected("GSP_ARTIFACT_UNREADABLE"))?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| rejected("GSP_ARTIFACT_UNREADABLE"))?;
    if bytes.len() as u64 > limit {
        return Err(rejected("GSP_ARTIFACT_REJECTED"));
    }
    Ok(bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T, CliError> {
    parse(&read_bytes(path, limit)?)
}

pub fn read_artifact<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T, CliError> {
    let bytes = read_bytes(path, limit)?;
    scan(&bytes)?;
    parse(&bytes)
}

pub fn scan(bytes: &[u8]) -> Result<(), CliError> {
    if !PrivacyScanner.scan_bytes("gsp-artifact", bytes).is_empty() {
        return Err(CliError::new(
            13,
            "PRIVACY_REJECTED",
            "sensitive material cannot enter a GSP artifact",
        ));
    }
    Ok(())
}

pub fn publish(path: &Path, value: &impl Serialize) -> Result<(), CliError> {
    let bytes = to_canonical_vec(value).map_err(|_| rejected("GSP_ARTIFACT_REJECTED"))?;
    if bytes.len() as u64 + 1 > RESULT_LIMIT {
        return Err(rejected("GSP_ARTIFACT_REJECTED"));
    }
    scan(&bytes)?;
    if fs::symlink_metadata(path).is_ok() {
        return Err(CliError::input("GSP output already exists"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).map_err(CliError::storage)?;
    let destination = parent.join(
        path.file_name()
            .ok_or_else(|| CliError::input("GSP output filename is required"))?,
    );
    let mut temporary = tempfile::NamedTempFile::new_in(&parent).map_err(CliError::storage)?;
    temporary.write_all(&bytes).map_err(CliError::storage)?;
    temporary.write_all(b"\n").map_err(CliError::storage)?;
    temporary.as_file().sync_all().map_err(CliError::storage)?;
    temporary
        .persist_noclobber(&destination)
        .map_err(|_| CliError::input("GSP output cannot be published without overwriting"))?;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::ffi::OsStrExt;

    #[test]
    fn long_path_publication_preserves_output_and_rejects_overwrite() {
        let temporary = tempfile::tempdir().unwrap();
        let mut parent = temporary.path().to_path_buf();
        while parent.as_os_str().encode_wide().count() < 1100 {
            parent.push("publicación local del resultado á");
        }
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("result.json");
        // Tests local publication only; no external GSP runtime is involved.
        publish(&path, &serde_json::json!({"a": 1})).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{\"a\":1}\n");
        assert!(publish(&path, &serde_json::json!({"a": 2})).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{\"a\":1}\n");
    }
}
