use project_ledger_canonical_json::sha256_bytes;
use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::{ExecutionReservation, SqliteEventStore};
#[allow(dead_code)]
#[path = "support/mod.rs"]
mod support;

#[test]
fn execution_attempt_is_durable_unique_and_not_a_canonical_event() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(support::create_commit()).unwrap();
    let scope = sha256_bytes(b"project+actor+trust+command+key");
    let request = sha256_bytes(b"explicit request+version+timestamp");
    assert_eq!(
        store
            .reserve_execution(&support::project_id(), scope, request)
            .unwrap(),
        ExecutionReservation::Reserved
    );
    assert_eq!(store.current_version(&support::project_id()).unwrap(), 1);
    drop(store);
    let mut reopened = SqliteEventStore::open_existing(&path).unwrap();
    assert_eq!(
        reopened
            .reserve_execution(&support::project_id(), scope, request)
            .unwrap(),
        ExecutionReservation::OutcomeUnknown
    );
    assert_eq!(
        reopened
            .reserve_execution(
                &support::project_id(),
                scope,
                sha256_bytes(b"different request")
            )
            .unwrap(),
        ExecutionReservation::Conflict
    );
    assert_eq!(reopened.current_version(&support::project_id()).unwrap(), 1);
    assert_eq!(
        reopened
            .recovery_report(&support::project_id())
            .unwrap()
            .execution_attempt_count,
        1
    );
}

#[test]
fn concurrent_attempts_reserve_once_and_tampering_fails_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    store.commit_command(support::create_commit()).unwrap();
    let scope = sha256_bytes(b"shared execution scope");
    let request = sha256_bytes(b"shared execution request");
    let mut threads = Vec::new();
    for _ in 0..2 {
        let path = path.clone();
        threads.push(std::thread::spawn(move || {
            SqliteEventStore::open_existing(&path)
                .unwrap()
                .reserve_execution(&support::project_id(), scope, request)
                .unwrap()
        }));
    }
    let results = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == ExecutionReservation::Reserved)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == ExecutionReservation::OutcomeUnknown)
            .count(),
        1
    );
    let raw = rusqlite::Connection::open(&path).unwrap();
    assert!(
        raw.execute("UPDATE execution_attempts SET request_digest='invalid'", [])
            .is_err()
    );
    raw.execute_batch("DROP TRIGGER execution_attempts_forbid_update; UPDATE execution_attempts SET request_digest='invalid';").unwrap();
    assert!(store.recovery_report(&support::project_id()).is_err());
}
