use std::time::Duration;

use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn new_database_applies_the_locked_connection_and_migration_policy() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");

    let store = SqliteEventStore::open(&path).unwrap();

    assert_eq!(store.journal_mode().unwrap(), "wal");
    assert_eq!(store.migration_version().unwrap(), 2);
    assert!(store.foreign_keys_enabled().unwrap());
    assert_eq!(store.busy_timeout().unwrap(), Duration::from_secs(5));
    assert_eq!(store.quick_check().unwrap(), "ok");
}

#[test]
fn reopening_an_initialized_database_is_idempotent() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    drop(SqliteEventStore::open(&path).unwrap());

    let reopened = SqliteEventStore::open(&path).unwrap();

    assert_eq!(reopened.migration_version().unwrap(), 2);
    assert_eq!(reopened.quick_check().unwrap(), "ok");
}

#[test]
fn reopening_rejects_tampered_migration_metadata() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    drop(SqliteEventStore::open(&path).unwrap());
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE schema_migrations SET migration_digest =
         'sha256:0000000000000000000000000000000000000000000000000000000000000000'
         WHERE version = 1",
        [],
    )
    .unwrap();
    drop(raw);

    assert!(SqliteEventStore::open(&path).is_err());
}

#[test]
fn reopening_rejects_tampered_projection_migration_metadata() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    drop(SqliteEventStore::open(&path).unwrap());
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE schema_migrations SET migration_digest =
         'sha256:0000000000000000000000000000000000000000000000000000000000000000'
         WHERE version = 2",
        [],
    )
    .unwrap();
    drop(raw);

    assert!(SqliteEventStore::open(&path).is_err());
}
