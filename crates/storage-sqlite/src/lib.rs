//! SQLite/WAL adapter for the Project Ledger event-store ports.

mod events;
mod migrations;
mod projections;
mod recovery;
mod submissions;

#[cfg(test)]
mod disk_full_tests;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

use std::{fs, path::Path, time::Duration};

use rusqlite::Connection;
use thiserror::Error;

pub use recovery::RecoveryReport;

/// Failures produced by the `SQLite` storage adapter.
#[derive(Debug, Error)]
pub enum SqliteStoreError {
    /// A database directory could not be created.
    #[error("database directory creation failed: {0}")]
    Directory(#[from] std::io::Error),
    /// `SQLite` rejected an operation or integrity query.
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The database was created by a newer, unsupported migration set.
    #[error("unsupported schema migration version {found}; maximum supported is {supported}")]
    UnsupportedMigration { found: u32, supported: u32 },
    /// Applied migration metadata no longer matches the compiled migration.
    #[error("migration {version} metadata failed its integrity check")]
    MigrationIntegrity { version: u32 },
}

/// A single-project `SQLite` event store configured for WAL durability.
#[derive(Debug)]
pub struct SqliteEventStore {
    connection: Connection,
}

impl SqliteEventStore {
    /// Opens or creates a database and applies all supported migrations.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when the directory, connection policy, or
    /// migrations cannot be established.
    pub fn open(path: &Path) -> Result<Self, SqliteStoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        migrations::apply(&mut connection)?;
        Ok(Self { connection })
    }

    /// Returns the active `SQLite` journal mode.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when the pragma cannot be queried.
    pub fn journal_mode(&self) -> Result<String, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?)
    }

    /// Returns the highest applied schema migration.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when migration metadata cannot be queried.
    pub fn migration_version(&self) -> Result<u32, SqliteStoreError> {
        Ok(self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?)
    }

    /// Reports whether `SQLite` foreign-key enforcement is active.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when the pragma cannot be queried.
    pub fn foreign_keys_enabled(&self) -> Result<bool, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?)
    }

    /// Returns the configured `SQLite` busy timeout.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when the pragma cannot be queried.
    pub fn busy_timeout(&self) -> Result<Duration, SqliteStoreError> {
        let milliseconds: u32 = self
            .connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))?;
        Ok(Duration::from_millis(u64::from(milliseconds)))
    }

    /// Executes `SQLite`'s bounded structural quick check.
    ///
    /// # Errors
    ///
    /// Returns [`SqliteStoreError`] when the integrity pragma cannot run.
    pub fn quick_check(&self) -> Result<String, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?)
    }
}
