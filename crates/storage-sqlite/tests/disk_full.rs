use std::{
    env, fs,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

use project_ledger_event_store::{EventStore, StoreError};
use project_ledger_storage_sqlite::SqliteEventStore;

mod support;

const MARKER_NAME: &str = "project-ledger-ephemeral-disk-full.marker";
const MARKER_CONTENT: &str = "project-ledger-disk-full/0.2.0\n";
const RUN_DIRECTORY_NAME: &str = "project-ledger-disk-full-run";
const FILLER_NAME: &str = "capacity-filler.bin";
const FILL_CHUNK_BYTES: usize = 1024 * 1024;
const MAX_FILL_BYTES: u64 = 64 * 1024 * 1024;

#[test]
fn physical_harness_requires_a_marked_non_workspace_root() {
    let workspace = workspace_root();
    let directory = tempfile::tempdir().unwrap();
    let unmarked = validate_ephemeral_root(directory.path(), &workspace).unwrap_err();
    assert!(unmarked.contains("marker metadata"));

    fs::write(directory.path().join(MARKER_NAME), MARKER_CONTENT).unwrap();
    assert_eq!(
        validate_ephemeral_root(directory.path(), &workspace).unwrap(),
        directory.path().canonicalize().unwrap()
    );

    let workspace_error = validate_ephemeral_root(&workspace, &workspace).unwrap_err();
    assert!(workspace_error.contains("workspace"));
}

#[test]
#[ignore = "requires an explicitly marked, bounded ephemeral filesystem"]
fn physical_kernel_enospc_rolls_back_and_reopens_with_the_committed_chain() {
    let root = validated_ephemeral_root();
    let run_directory = root.join(RUN_DIRECTORY_NAME);
    assert!(
        !run_directory.exists(),
        "refusing to reuse an existing chaos run directory: {}",
        run_directory.display()
    );
    fs::create_dir(&run_directory).unwrap();
    let mut cleanup = ExactRunCleanup::new(run_directory.clone());
    let database = run_directory.join("ledger.db");
    let filler = run_directory.join(FILLER_NAME);

    let mut store = SqliteEventStore::open(&database).unwrap();
    let genesis = support::create_commit();
    let previous_digest = genesis.events[0].event_digest;
    store.commit_command(genesis).unwrap();

    let filled_bytes = fill_until_enospc(&filler).unwrap();
    eprintln!("physical-disk-full: observed ENOSPC(28) after {filled_bytes} filler bytes");

    let error = store
        .commit_command(support::metadata_commit(
            previous_digest,
            support::OVERSIZED_SUMMARY_BYTES,
        ))
        .unwrap_err();
    assert!(
        matches!(&error, StoreError::Persistence { message } if message.to_ascii_lowercase().contains("full")),
        "kernel ENOSPC must surface as a closed persistence failure: {error:?}"
    );
    eprintln!("physical-disk-full: append failed closed with {error}");

    fs::remove_file(&filler).unwrap();
    support::assert_recovered_genesis(&store, previous_digest);
    eprintln!("physical-disk-full: same-process rollback preserved version/count/chain 1/1/valid");
    drop(store);

    let reopened = SqliteEventStore::open(&database).unwrap();
    support::assert_recovered_genesis(&reopened, previous_digest);
    eprintln!("physical-disk-full: reopened quick_check=ok WAL chain version/count 1/1");
    drop(reopened);

    cleanup.clean().unwrap();
}

fn validated_ephemeral_root() -> PathBuf {
    let configured = env::var_os("PROJECT_LEDGER_DISK_FULL_ROOT")
        .map(PathBuf::from)
        .expect("PROJECT_LEDGER_DISK_FULL_ROOT is required for the ignored physical test");
    validate_ephemeral_root(&configured, &workspace_root())
        .unwrap_or_else(|error| panic!("invalid ephemeral root: {error}"))
}

fn validate_ephemeral_root(configured: &Path, workspace: &Path) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(configured)
        .map_err(|error| format!("ephemeral root metadata failed: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("ephemeral root must be a real directory, not a symlink".to_owned());
    }
    let root = configured
        .canonicalize()
        .map_err(|error| format!("ephemeral root canonicalization failed: {error}"))?;
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("workspace canonicalization failed: {error}"))?;
    if root.parent().is_none()
        || root == workspace
        || root.starts_with(&workspace)
        || workspace.starts_with(&root)
    {
        return Err(
            "ephemeral root cannot be the workspace, its ancestor or its descendant".to_owned(),
        );
    }

    let marker = root.join(MARKER_NAME);
    let marker_metadata = fs::symlink_metadata(&marker)
        .map_err(|error| format!("ephemeral marker metadata failed: {error}"))?;
    if !marker_metadata.is_file() || marker_metadata.file_type().is_symlink() {
        return Err("ephemeral marker must be a regular non-symlink file".to_owned());
    }
    let marker_content = fs::read_to_string(&marker)
        .map_err(|error| format!("ephemeral marker read failed: {error}"))?;
    if marker_content != MARKER_CONTENT {
        return Err("ephemeral marker content mismatch".to_owned());
    }
    Ok(root)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fill_until_enospc(path: &Path) -> io::Result<u64> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let chunk = vec![0xA5_u8; FILL_CHUNK_BYTES];
    let mut written = 0_u64;
    while written < MAX_FILL_BYTES {
        match file.write_all(&chunk).and_then(|()| file.sync_all()) {
            Ok(()) => written += FILL_CHUNK_BYTES as u64,
            Err(error) if error.raw_os_error() == Some(28) => return Ok(written),
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other(format!(
        "no ENOSPC observed within the {MAX_FILL_BYTES}-byte safety cap"
    )))
}

#[derive(Debug)]
struct ExactRunCleanup {
    run_directory: PathBuf,
    active: bool,
}

impl ExactRunCleanup {
    fn new(run_directory: PathBuf) -> Self {
        Self {
            run_directory,
            active: true,
        }
    }

    fn clean(&mut self) -> io::Result<()> {
        clean_known_files(&self.run_directory)?;
        fs::remove_dir(&self.run_directory)?;
        self.active = false;
        Ok(())
    }
}

impl Drop for ExactRunCleanup {
    fn drop(&mut self) {
        if self.active {
            let _ = clean_known_files(&self.run_directory);
            let _ = fs::remove_dir(&self.run_directory);
        }
    }
}

fn clean_known_files(run_directory: &Path) -> io::Result<()> {
    for name in [
        FILLER_NAME,
        "ledger.db-wal",
        "ledger.db-shm",
        "ledger.db-journal",
        "ledger.db",
    ] {
        let path = run_directory.join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
