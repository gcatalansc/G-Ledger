use rusqlite::{Connection, TransactionBehavior, params};

use crate::SqliteStoreError;
use project_ledger_canonical_json::sha256_bytes;

const SUPPORTED_VERSION: u32 = 1;

pub(crate) fn apply(connection: &mut Connection) -> Result<(), SqliteStoreError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            migration_digest TEXT NOT NULL
        ) STRICT;",
    )?;
    let current: u32 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if current > SUPPORTED_VERSION {
        return Err(SqliteStoreError::UnsupportedMigration {
            found: current,
            supported: SUPPORTED_VERSION,
        });
    }
    if current >= 1 {
        let (name, digest): (String, String) = connection.query_row(
            "SELECT name, migration_digest FROM schema_migrations WHERE version = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if name != "s2-event-store" || digest != sha256_bytes(MIGRATION_V1.as_bytes()).to_string() {
            return Err(SqliteStoreError::MigrationIntegrity { version: 1 });
        }
    }
    if current == 0 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(MIGRATION_V1)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, name, migration_digest) VALUES (?1, ?2, ?3)",
            params![
                1_u32,
                "s2-event-store",
                sha256_bytes(MIGRATION_V1.as_bytes()).to_string()
            ],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

const MIGRATION_V1: &str = r"
CREATE TABLE event_streams (
    project_ref TEXT PRIMARY KEY NOT NULL,
    current_version INTEGER NOT NULL DEFAULT 0 CHECK (current_version >= 0),
    head_event_digest TEXT,
    CHECK ((current_version = 0 AND head_event_digest IS NULL) OR
           (current_version > 0 AND head_event_digest IS NOT NULL))
) STRICT;

CREATE TABLE events (
    event_id TEXT PRIMARY KEY NOT NULL,
    project_ref TEXT NOT NULL,
    stream_version INTEGER NOT NULL CHECK (stream_version >= 1),
    event_type TEXT NOT NULL,
    command_ref TEXT NOT NULL,
    previous_event_digest TEXT,
    payload_digest TEXT NOT NULL,
    event_digest TEXT NOT NULL UNIQUE,
    event_json BLOB NOT NULL,
    recorded_at TEXT NOT NULL,
    UNIQUE(project_ref, stream_version),
    FOREIGN KEY(project_ref) REFERENCES event_streams(project_ref) ON DELETE RESTRICT,
    CHECK ((stream_version = 1 AND previous_event_digest IS NULL) OR
           (stream_version > 1 AND previous_event_digest IS NOT NULL))
) STRICT;

CREATE TRIGGER events_forbid_update
BEFORE UPDATE ON events BEGIN
    SELECT RAISE(ABORT, 'events are append-only');
END;

CREATE TRIGGER events_forbid_delete
BEFORE DELETE ON events BEGIN
    SELECT RAISE(ABORT, 'events are append-only');
END;

CREATE TABLE commands (
    command_id TEXT PRIMARY KEY NOT NULL,
    project_ref TEXT NOT NULL,
    command_type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    trust_domain TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    expected_state_version INTEGER NOT NULL CHECK (expected_state_version >= 0),
    command_digest TEXT NOT NULL,
    command_json BLOB NOT NULL,
    submitted_at TEXT NOT NULL
) STRICT;

CREATE TABLE command_results (
    command_result_id TEXT PRIMARY KEY NOT NULL,
    command_ref TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    result_digest TEXT NOT NULL,
    result_json BLOB NOT NULL,
    processed_at TEXT NOT NULL,
    FOREIGN KEY(command_ref) REFERENCES commands(command_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE idempotency_keys (
    project_ref TEXT NOT NULL,
    command_type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    trust_domain TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    command_digest TEXT NOT NULL,
    command_ref TEXT NOT NULL UNIQUE,
    command_result_ref TEXT NOT NULL,
    PRIMARY KEY(project_ref, command_type, actor_id, trust_domain, idempotency_key),
    FOREIGN KEY(command_ref) REFERENCES commands(command_id) ON DELETE RESTRICT,
    FOREIGN KEY(command_result_ref) REFERENCES command_results(command_result_id) ON DELETE RESTRICT
) WITHOUT ROWID, STRICT;

CREATE TABLE external_submissions (
    submission_id TEXT PRIMARY KEY NOT NULL,
    command_ref TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    state TEXT NOT NULL,
    record_digest TEXT NOT NULL,
    record_json BLOB NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE external_submission_transitions (
    transition_id INTEGER PRIMARY KEY AUTOINCREMENT,
    submission_ref TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 1),
    prior_state TEXT,
    new_state TEXT NOT NULL,
    record_digest TEXT NOT NULL,
    record_json BLOB NOT NULL,
    transitioned_at TEXT NOT NULL,
    UNIQUE(submission_ref, ordinal),
    FOREIGN KEY(submission_ref) REFERENCES external_submissions(submission_id) ON DELETE RESTRICT
) STRICT;

CREATE TRIGGER submission_transitions_forbid_update
BEFORE UPDATE ON external_submission_transitions BEGIN
    SELECT RAISE(ABORT, 'submission transitions are append-only');
END;

CREATE TRIGGER submission_transitions_forbid_delete
BEFORE DELETE ON external_submission_transitions BEGIN
    SELECT RAISE(ABORT, 'submission transitions are append-only');
END;
";
