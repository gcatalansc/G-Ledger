use project_ledger_event_store::{EventStore, StoreError};
use tempfile::tempdir;

use crate::{SqliteEventStore, test_support};

#[test]
fn deterministic_sqlite_full_rolls_back_the_whole_append_and_preserves_the_chain() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    let genesis = test_support::create_commit();
    let previous_digest = genesis.events[0].event_digest;
    store.commit_command(genesis).unwrap();

    store
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    let page_count: i64 = store
        .connection
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .connection
        .pragma_update(None, "max_page_count", page_count)
        .unwrap();

    let error = store
        .commit_command(test_support::metadata_commit(
            previous_digest,
            test_support::OVERSIZED_SUMMARY_BYTES,
        ))
        .unwrap_err();
    assert!(
        matches!(&error, StoreError::Persistence { message } if message.to_ascii_lowercase().contains("full")),
        "capacity fault must surface as a closed persistence failure: {error:?}"
    );

    store
        .connection
        .pragma_update(None, "max_page_count", 1_073_741_823_i64)
        .unwrap();
    test_support::assert_recovered_genesis(&store, previous_digest);
    drop(store);

    let reopened = SqliteEventStore::open(&path).unwrap();
    test_support::assert_recovered_genesis(&reopened, previous_digest);
}
