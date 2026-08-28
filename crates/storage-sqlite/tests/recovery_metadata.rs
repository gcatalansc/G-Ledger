use project_ledger_event_store::EventStore;
use project_ledger_storage_sqlite::SqliteEventStore;
#[allow(dead_code)]
mod support;

#[test]
fn recovery_rejects_missing_or_forged_idempotency_and_command_metadata() {
    for sql in [
        "DELETE FROM idempotency_keys",
        "UPDATE commands SET actor_id='forged:actor'",
        "UPDATE idempotency_keys SET trust_domain='UNTRUSTED'",
        "UPDATE idempotency_keys SET command_ref='command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df00'",
        "UPDATE command_results SET command_result_id='forged:result'",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ledger.db");
        let mut store = SqliteEventStore::open(&path).unwrap();
        store.commit_command(support::create_commit()).unwrap();
        assert!(store.recovery_report(&support::project_id()).is_ok());
        let tamper = rusqlite::Connection::open(&path).unwrap();
        tamper.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        tamper.execute_batch(sql).unwrap();
        assert!(
            store.recovery_report(&support::project_id()).is_err(),
            "accepted {sql}"
        );
    }
}
