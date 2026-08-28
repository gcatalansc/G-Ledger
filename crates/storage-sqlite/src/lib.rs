//! SQLite/WAL adapter for the Project Ledger event-store ports.

mod backup;
mod events;
mod executions;
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

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

pub use backup::{BackupError, BackupManifest};
pub use executions::ExecutionReservation;
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
    /// Opens an existing current-schema store without creating or migrating it.
    ///
    /// # Errors
    /// Rejects missing, old, future or tampered databases.
    pub fn open_read_only(path: &Path) -> Result<Self, SqliteStoreError> {
        Self::open_current(path, true)
    }

    /// Opens an existing current-schema store for explicit command writes.
    ///
    /// # Errors
    /// Rejects missing or incompatible databases; never applies migrations.
    pub fn open_existing(path: &Path) -> Result<Self, SqliteStoreError> {
        Self::open_current(path, false)
    }

    fn open_current(path: &Path, read_only: bool) -> Result<Self, SqliteStoreError> {
        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        let connection = Connection::open_with_flags(path, flags)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        migrations::verify_current(&connection)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if !read_only {
            connection.pragma_update(None, "synchronous", "FULL")?;
        }
        Ok(Self { connection })
    }
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
