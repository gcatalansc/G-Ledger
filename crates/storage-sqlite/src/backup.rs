use crate::{SqliteEventStore, SqliteStoreError, migrations};
use project_ledger_canonical_json::{Sha256Digest, to_canonical_vec};
use project_ledger_domain::ProjectId;
use project_ledger_event_store::{EventStore, StoreError};
use project_ledger_projection_engine::ProjectStateProjection;
use rusqlite::{
    Connection, OpenFlags, TransactionBehavior,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Local full-fidelity snapshot; intentionally not a shareable portable export.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupManifest {
    pub format: String,
    pub database_digest: Sha256Digest,
    pub migration_version: u32,
    pub projects: Vec<ProjectId>,
}

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("backup/restore destination already exists")]
    DestinationExists,
    #[error("backup integrity verification failed")]
    Integrity,
    #[error("backup exceeded its 60 second deadline")]
    Timeout,
    #[error("backup filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("backup SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Store(#[from] SqliteStoreError),
    #[error(transparent)]
    Recovery(#[from] StoreError),
}

impl SqliteEventStore {
    /// Opens known historical schemas for backup/recovery only, without migration.
    /// # Errors
    /// Rejects missing databases, unsupported versions and corrupt migration metadata.
    pub fn open_for_recovery(path: &Path) -> Result<Self, SqliteStoreError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        migrations::verify_known(&connection)?;
        Ok(Self { connection })
    }

    /// Lists explicit stream identities, without consulting names or a catalog.
    /// # Errors
    /// Rejects malformed stored identities or a failed read.
    pub fn project_ids(&self) -> Result<Vec<ProjectId>, BackupError> {
        let mut stmt = self
            .connection
            .prepare("SELECT project_ref FROM event_streams ORDER BY project_ref")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .map(|row| ProjectId::parse(&row?).map_err(|_| BackupError::Integrity))
            .collect()
    }

    /// Takes a bounded online `SQLite` snapshot, including committed WAL pages.
    /// The manifest is written last. An interrupted directory is not a valid backup.
    /// # Errors
    /// Rejects existing destinations, timeout and corrupt snapshots without overwriting.
    pub fn backup_to(&self, directory: &Path) -> Result<BackupManifest, BackupError> {
        if directory.try_exists()? {
            return Err(BackupError::DestinationExists);
        }
        fs::create_dir(directory)?;
        let temporary = NamedTempFile::new_in(directory)?;
        let copied = copy_snapshot(&self.connection, temporary.path())?;
        let projects = validate_snapshot(&copied)?;
        let version = copied.migration_version()?;
        drop(copied);
        temporary.as_file().sync_all()?;
        let final_path = directory.join("ledger.db");
        temporary
            .persist_noclobber(&final_path)
            .map_err(|e| BackupError::Io(e.error))?;
        let manifest = BackupManifest {
            format: "project-ledger-sqlite-backup/1".to_owned(),
            database_digest: file_digest(&final_path)?,
            migration_version: version,
            projects,
        };
        let bytes = to_canonical_vec(&manifest).map_err(|_| BackupError::Integrity)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("manifest.json"))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(manifest)
    }

    /// Verifies and restores a full snapshot to a new database, never migrating it.
    /// # Errors
    /// Rejects corruption, symlinks, existing destinations or unsupported schemas.
    pub fn restore_backup(
        directory: &Path,
        destination: &Path,
    ) -> Result<BackupManifest, BackupError> {
        if fs::symlink_metadata(destination).is_ok() {
            return Err(BackupError::DestinationExists);
        }
        for path in [
            directory.to_path_buf(),
            directory.join("manifest.json"),
            directory.join("ledger.db"),
        ] {
            if fs::symlink_metadata(path)?.file_type().is_symlink() {
                return Err(BackupError::Integrity);
            }
        }
        // Backups are closed snapshots: live WAL/SHM sidecars or extra files are invalid.
        let mut names = fs::read_dir(directory)?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        if names
            != [
                std::ffi::OsString::from("ledger.db"),
                std::ffi::OsString::from("manifest.json"),
            ]
        {
            return Err(BackupError::Integrity);
        }
        let mut bytes = Vec::new();
        fs::File::open(directory.join("manifest.json"))?
            .take(65_537)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            return Err(BackupError::Integrity);
        }
        let manifest: BackupManifest =
            serde_json::from_slice(&bytes).map_err(|_| BackupError::Integrity)?;
        let source_path = directory.join("ledger.db");
        if manifest.format != "project-ledger-sqlite-backup/1"
            || file_digest(&source_path)? != manifest.database_digest
        {
            return Err(BackupError::Integrity);
        }
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let temporary = NamedTempFile::new_in(parent)?;
        // The source is a closed checksummed snapshot, not a live DB. Copy first,
        // verify those exact bytes, then inspect only the private copy with SQLite.
        fs::copy(&source_path, temporary.path())?;
        if file_digest(temporary.path())? != manifest.database_digest {
            return Err(BackupError::Integrity);
        }
        let copied = Self {
            connection: Connection::open(temporary.path())?,
        };
        let projects = validate_snapshot(&copied)?;
        if projects != manifest.projects
            || copied.migration_version()? != manifest.migration_version
            || file_digest(&source_path)? != manifest.database_digest
        {
            return Err(BackupError::Integrity);
        }
        drop(copied);
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(destination)
            .map_err(|e| BackupError::Io(e.error))?;
        Ok(manifest)
    }

    /// Excludes writers, snapshots the old schema, then migrates in one transaction.
    /// # Errors
    /// Any backup or integrity failure prevents schema writes; migration failure rolls back.
    pub fn migrate_with_backup(
        path: &Path,
        directory: &Path,
    ) -> Result<BackupManifest, BackupError> {
        let mut writer = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        writer.busy_timeout(Duration::from_secs(5))?;
        writer.pragma_update(None, "foreign_keys", true)?;
        writer.pragma_update(None, "synchronous", "FULL")?;
        let transaction = writer.transaction_with_behavior(TransactionBehavior::Immediate)?;
        migrations::verify_known(&transaction)?;
        let reader = Self::open_for_recovery(path)?;
        let manifest = reader.backup_to(directory)?;
        migrations::apply_inside(&transaction)?;
        transaction.commit()?;
        Ok(manifest)
    }
}

fn copy_snapshot(source: &Connection, destination: &Path) -> Result<SqliteEventStore, BackupError> {
    let mut target = Connection::open(destination)?;
    target.busy_timeout(Duration::from_millis(100))?;
    let backup = Backup::new(source, &mut target)?;
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        if Instant::now() >= deadline {
            return Err(BackupError::Timeout);
        }
        match backup.step(128)? {
            StepResult::Done => break,
            StepResult::More => {}
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    drop(backup);
    target.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
    target.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    Ok(SqliteEventStore { connection: target })
}

fn validate_snapshot(store: &SqliteEventStore) -> Result<Vec<ProjectId>, BackupError> {
    migrations::verify_known(&store.connection)?;
    let invalid_fk = store
        .connection
        .prepare("PRAGMA foreign_key_check")?
        .exists([])?;
    if invalid_fk || store.quick_check()? != "ok" {
        return Err(BackupError::Integrity);
    }
    let projects = store.project_ids()?;
    if projects.is_empty() {
        return Err(BackupError::Integrity);
    }
    for id in &projects {
        store.recovery_report(id)?;
        ProjectStateProjection::rebuild(&store.load_stream(id)?)
            .map_err(|_| BackupError::Integrity)?;
    }
    Ok(projects)
}

fn file_digest(path: &Path) -> Result<Sha256Digest, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 65_536];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    Ok(Sha256Digest::from(<[u8; 32]>::from(digest.finalize())))
}
