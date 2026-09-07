#![cfg(windows)]

use std::{fs, os::windows::ffi::OsStrExt, path::Path};

use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::{BackupError, SqliteEventStore};

#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

#[test]
fn long_paths_preserve_wal_reads_writes_and_full_backup_recovery() {
    let temporary = tempfile::tempdir().unwrap();
    for database_length in [257, 1150] {
        let mut directory = temporary.path().join(format!("ruta á {database_length}"));
        while wide_length(&directory.join("ledger.db")) + 49 < database_length {
            directory.push("x".repeat(48));
        }
        let remaining = database_length - wide_length(&directory.join("ledger.db")) - 1;
        directory.push("y".repeat(remaining));
        let database = directory.join("ledger.db");
        assert_eq!(wide_length(&database), database_length);
        assert!(!database.to_str().unwrap().starts_with(r"\\?\"));

        let mut writer = SqliteEventStore::open(&database).unwrap();
        let genesis = support::create_commit();
        let digest = genesis.events[0].event_digest;
        writer.commit_command(genesis).unwrap();
        let reader = SqliteEventStore::open_read_only(&database).unwrap();
        writer
            .commit_command(support::metadata_commit(digest, 4096))
            .unwrap();
        assert!(
            fs::metadata(database.with_extension("db-wal"))
                .unwrap()
                .len()
                > 0
        );
        assert_eq!(reader.current_version(&support::project_id()).unwrap(), 2);
        let events = reader.load_stream(&support::project_id()).unwrap();

        let backup = directory.join("respaldo á");
        let manifest = reader.backup_to(&backup).unwrap();
        assert_eq!(manifest.projects, vec![support::project_id()]);
        assert!(matches!(
            reader.backup_to(&backup),
            Err(BackupError::DestinationExists)
        ));
        let restored_path = directory.join("recuperado.db");
        assert_eq!(
            SqliteEventStore::restore_backup(&backup, &restored_path).unwrap(),
            manifest
        );
        assert!(matches!(
            SqliteEventStore::restore_backup(&backup, &restored_path),
            Err(BackupError::DestinationExists)
        ));
        drop(reader);
        drop(writer);

        for path in [&database, &restored_path] {
            let reopened = SqliteEventStore::open_existing(path).unwrap();
            assert_eq!(
                reopened.load_stream(&support::project_id()).unwrap(),
                events
            );
            let recovery = SqliteEventStore::open_for_recovery(path).unwrap();
            let report = recovery.recovery_report(&support::project_id()).unwrap();
            assert!(report.chain_verified);
            assert_eq!(report.current_version, 2);
            assert_eq!(report.command_result_count, 2);
        }
        SqliteEventStore::migrate_with_backup(&restored_path, &directory.join("pre-migration"))
            .unwrap();
        assert_eq!(
            SqliteEventStore::open_read_only(&restored_path)
                .unwrap()
                .load_stream(&support::project_id())
                .unwrap(),
            events
        );
        let missing = directory.join("missing.db");
        assert!(SqliteEventStore::open_existing(&missing).is_err());
        assert!(SqliteEventStore::open_read_only(&missing).is_err());
        assert!(SqliteEventStore::open_for_recovery(&missing).is_err());
        assert!(!missing.exists());
    }
}

fn wide_length(path: &Path) -> usize {
    path.as_os_str().encode_wide().count()
}

#[test]
fn short_paths_keep_windows_filename_normalization() {
    let temporary = tempfile::tempdir().unwrap();
    let database = temporary.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&database).unwrap();
    store.commit_command(support::create_commit()).unwrap();
    // Ordinary Win32 filenames trim trailing dots. The long-path fix must not
    // silently create a different stream when this existing alias is selected.
    let alias = temporary.path().join("ledger.db.");
    let reopened = SqliteEventStore::open_existing(&alias).unwrap();
    assert_eq!(reopened.current_version(&support::project_id()).unwrap(), 1);
}
