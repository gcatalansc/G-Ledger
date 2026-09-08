//! Real processor/`SQLite` races. Only the clock port is synchronized by the harness.
use std::{
    cell::Cell,
    path::Path,
    sync::mpsc::{Receiver, SyncSender, sync_channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use project_ledger_application::{
    Clock, CommandHandler, CommandHandlerError, CommandProcessor, CoreCommandHandler, FixedClock,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandEnvelope, CommandId, CommandResult,
    CommandResultStatus, CreateProjectPayload, ErrorCode, EventEnvelope, LedgerTimestamp, Project,
    ProjectId, ProjectLifecycleStatus, SchemaVersion, StableId, StorageProfile, TrustLevel,
    UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{
    CommandDraft, CommitDisposition, EventDraft, EventStore, StoreError, prepare_command,
};
use project_ledger_storage_sqlite::SqliteEventStore;
use tempfile::tempdir;

#[test]
fn commit_race_returns_a_durable_conflict_without_reexecuting_the_handler() {
    assert_race(false, false);
}

#[test]
fn initially_stale_command_records_the_head_at_finalization_after_another_advance() {
    assert_race(true, false);
}

#[test]
fn domain_rejection_race_is_finalized_as_a_durable_version_conflict() {
    assert_race(false, true);
}

fn assert_race(initially_stale: bool, reject_in_handler: bool) {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = seeded(&path);
    let candidate = metadata(u64::from(!initially_stale), "candidate");
    let original = candidate.clone();
    let worker_path = path.clone();
    let (clock, worker) = RaceClock::new(move || {
        let mut competing = SqliteEventStore::open(&worker_path).unwrap();
        let result = execute(&mut competing, metadata(1, "competing"));
        assert_eq!(result.status, CommandResultStatus::Accepted);
        assert_eq!(result.new_state_version, 2);
    });
    let handler = ObservedHandler {
        calls: Cell::new(0),
        reject: reject_in_handler,
    };
    let outcome = CommandProcessor::new(&mut store, &clock).execute(candidate.clone(), &handler);
    worker.join().unwrap();
    let result = outcome.expect("a version race must return a persisted native conflict");
    assert_eq!(result.status, CommandResultStatus::VersionConflict);
    assert_eq!(
        result.rejection.as_ref().unwrap().code,
        ErrorCode::VersionConflict
    );
    assert_eq!(
        (result.previous_state_version, result.new_state_version),
        (2, 2)
    );
    assert!(result.emitted_event_refs.is_empty());
    assert!(result.resulting_checkpoint_ref.is_none());
    assert_eq!(handler.calls.get(), u32::from(!initially_stale));
    assert_eq!(candidate, original);
    assert_eq!(
        store.command_result(&candidate.command_id).unwrap(),
        Some(result.clone()),
        "the returned result must be durable, not a synthetic conflict"
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
    assert!(
        store
            .load_stream(&project_id())
            .unwrap()
            .iter()
            .all(|event| event.causation.command_ref != candidate.command_id)
    );
    drop(store);

    let mut reopened = SqliteEventStore::open(&path).unwrap();
    assert_eq!(
        execute(&mut reopened, metadata(2, "later")).new_state_version,
        3
    );
    assert_eq!(
        reopened.command_result(&candidate.command_id).unwrap(),
        Some(result.clone())
    );
    let replay = execute(&mut reopened, candidate);
    assert_eq!(replay.status, CommandResultStatus::VersionConflict);
    assert!(replay.idempotency.replayed);
    assert_eq!(
        (replay.previous_state_version, replay.new_state_version),
        (2, 2)
    );
    assert_eq!(
        reopened.command_result(&original.command_id).unwrap(),
        Some(result.clone())
    );
    reopened.verify_event_chain(&project_id()).unwrap();
    assert_eq!(reopened.event_count(&project_id()).unwrap(), 3);
    println!(
        "PL_GSK_004_RACE {}",
        serde_json::json!({"initially_stale":initially_stale,"domain_rejection":reject_in_handler,"result":result,"head_after_later_advance":3,"handler_calls":handler.calls.get(),"lookup_after_restart":"EXACT_DURABLE_RESULT","rejected_command_events":0})
    );
}

#[test]
fn conflict_persistence_failure_rolls_back_command_result_and_idempotency() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = seeded(&path);
    let candidate = metadata(1, "candidate");
    let worker_path = path.clone();
    let (clock, worker) = RaceClock::new(move || {
        let mut competing = SqliteEventStore::open(&worker_path).unwrap();
        assert_eq!(
            execute(&mut competing, metadata(1, "competing")).status,
            CommandResultStatus::Accepted
        );
        // Abort after the command insert, before result/idempotency can commit.
        rusqlite::Connection::open(&worker_path).unwrap().execute_batch(
            "CREATE TRIGGER test_result_failure BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'controlled result persistence failure'); END;"
        ).unwrap();
    });
    let outcome =
        CommandProcessor::new(&mut store, &clock).execute(candidate.clone(), &CoreCommandHandler);
    worker.join().unwrap();
    assert!(
        outcome.is_err(),
        "failed persistence must not return a supposedly durable VERSION_CONFLICT: {outcome:?}"
    );
    assert!(
        outcome
            .unwrap_err()
            .to_string()
            .contains("controlled result persistence failure")
    );
    assert!(
        store
            .command_result(&candidate.command_id)
            .unwrap()
            .is_none()
    );
    let observer = rusqlite::Connection::open(&path).unwrap();
    for table in ["commands", "command_results", "idempotency_keys", "events"] {
        let count: i64 = observer
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2, "partial write in {table}");
    }
    store.verify_event_chain(&project_id()).unwrap();
}

#[test]
fn finalization_replays_accepted_result_before_testing_expected_version() {
    let directory = tempdir().unwrap();
    let mut store = seeded(&directory.path().join("ledger.db"));
    let candidate = metadata(1, "accepted");
    let accepted = execute(&mut store, candidate.clone());
    assert_eq!(accepted.status, CommandResultStatus::Accepted);
    assert_eq!(
        store
            .finalize_version_conflict(candidate.clone(), timestamp())
            .unwrap(),
        CommitDisposition::Replayed(accepted.clone())
    );
    assert_eq!(
        store.command_result(&candidate.command_id).unwrap(),
        Some(accepted)
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
}

#[test]
fn finalization_preserves_rejected_result_and_refuses_different_scoped_payload() {
    let directory = tempdir().unwrap();
    let mut store = seeded(&directory.path().join("ledger.db"));
    let candidate = metadata(0, "rejected");
    let result = execute(&mut store, candidate.clone());
    assert_eq!(result.status, CommandResultStatus::VersionConflict);
    assert_eq!(
        execute(&mut store, metadata(1, "later")).new_state_version,
        2
    );
    assert_eq!(
        store
            .finalize_version_conflict(candidate.clone(), timestamp())
            .unwrap(),
        CommitDisposition::Replayed(result.clone())
    );
    let different = command(
        0,
        "rejected",
        Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some("different payload".to_owned()),
            summary: None,
        }),
    );
    assert!(matches!(
        store.finalize_version_conflict(different.clone(), timestamp()),
        Err(StoreError::IdempotencyConflict { .. })
    ));
    assert!(
        store
            .command_result(&different.command_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.command_result(&candidate.command_id).unwrap(),
        Some(result)
    );
    assert_eq!(store.event_count(&project_id()).unwrap(), 2);
}

#[test]
fn finalization_refuses_matching_version_and_invalid_command_integrity() {
    let directory = tempdir().unwrap();
    let mut store = seeded(&directory.path().join("ledger.db"));
    let matching = metadata(1, "matching");
    assert!(matches!(
        store.finalize_version_conflict(matching.clone(), timestamp()),
        Err(StoreError::InvalidCommit { .. })
    ));
    let mut tampered = metadata(0, "tampered");
    tampered.idempotency_key = StableId::parse("race:altered-after-digest").unwrap();
    assert!(matches!(
        store.finalize_version_conflict(tampered.clone(), timestamp()),
        Err(StoreError::Integrity(_))
    ));
    for candidate in [matching, tampered] {
        assert!(
            store
                .command_result(&candidate.command_id)
                .unwrap()
                .is_none()
        );
    }
    let report = store.recovery_report(&project_id()).unwrap();
    assert_eq!(report.command_count, 1);
    assert_eq!(report.command_result_count, 1);
    assert_eq!(report.event_count, 1);
}

#[test]
fn finalization_cannot_replace_a_command_id_under_a_different_key_or_scope() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = seeded(&path);
    let original = metadata(0, "original");
    let result = execute(&mut store, original.clone());
    assert_eq!(result.status, CommandResultStatus::VersionConflict);
    for change_scope in [false, true] {
        let mut altered = original.clone();
        if change_scope {
            altered.actor.actor_id = StableId::parse("actor:other-scope").unwrap();
        } else {
            altered.idempotency_key = StableId::parse("race:another-key").unwrap();
        }
        let altered = prepare_command(CommandDraft {
            schema_version: altered.schema_version,
            command_id: altered.command_id,
            project_ref: altered.project_ref,
            expected_state_version: altered.expected_state_version,
            idempotency_key: altered.idempotency_key,
            actor: altered.actor,
            command: altered.command,
            submitted_at: altered.submitted_at,
        })
        .unwrap();
        assert!(matches!(
            store.finalize_version_conflict(altered, timestamp()),
            Err(StoreError::Persistence { .. })
        ));
        assert_eq!(
            store.recorded_command(&original.command_id).unwrap(),
            Some(original.clone())
        );
        assert_eq!(
            store.command_result(&original.command_id).unwrap(),
            Some(result.clone())
        );
    }
    let observer = rusqlite::Connection::open(&path).unwrap();
    for table in ["commands", "command_results", "idempotency_keys"] {
        let count: i64 = observer
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2, "new owner or replacement in {table}");
    }
    assert_eq!(store.event_count(&project_id()).unwrap(), 1);
}

/// A channel rendezvous is a bounded barrier; there is no timing sleep or product hook.
#[derive(Debug)]
struct RaceClock {
    armed: Cell<bool>,
    release: SyncSender<()>,
    committed: Receiver<()>,
}

impl RaceClock {
    fn new(action: impl FnOnce() + Send + 'static) -> (Self, JoinHandle<()>) {
        let (release, released) = sync_channel(1);
        let (done, committed) = sync_channel(1);
        let worker = thread::spawn(move || {
            released
                .recv_timeout(Duration::from_secs(10))
                .expect("processor must reach the result boundary");
            action();
            done.send(()).unwrap();
        });
        (
            Self {
                armed: Cell::new(true),
                release,
                committed,
            },
            worker,
        )
    }
}

impl Clock for RaceClock {
    fn now(&self) -> LedgerTimestamp {
        if self.armed.replace(false) {
            self.release.send(()).unwrap();
            self.committed
                .recv_timeout(Duration::from_secs(10))
                .expect("the competing transaction must finish before the candidate commit");
        }
        timestamp()
    }
}

#[derive(Debug)]
struct ObservedHandler {
    calls: Cell<u32>,
    reject: bool,
}

impl CommandHandler for ObservedHandler {
    fn handle(
        &self,
        command: &CommandEnvelope,
        stream: &[EventEnvelope],
    ) -> Result<Vec<EventDraft>, CommandHandlerError> {
        self.calls.set(self.calls.get() + 1);
        if self.reject {
            return Err(CommandHandlerError::new(
                ErrorCode::InvalidContract,
                "controlled domain rejection",
            ));
        }
        CoreCommandHandler.handle(command, stream)
    }
}

fn execute(store: &mut SqliteEventStore, command: CommandEnvelope) -> CommandResult {
    CommandProcessor::new(store, &FixedClock::new(timestamp()))
        .execute(command, &CoreCommandHandler)
        .unwrap()
}

fn seeded(path: &Path) -> SqliteEventStore {
    let mut store = SqliteEventStore::open(path).unwrap();
    let project = Project {
        project_id: project_id(),
        title: "Conflict races".to_owned(),
        summary: "Isolated fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    };
    assert_eq!(
        execute(
            &mut store,
            command(
                0,
                "genesis",
                Command::CreateProject(CreateProjectPayload { project })
            )
        )
        .status,
        CommandResultStatus::Accepted
    );
    store
}

fn metadata(expected: u64, title: &str) -> CommandEnvelope {
    command(
        expected,
        title,
        Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
            title: Some(title.to_owned()),
            summary: None,
        }),
    )
}

fn command(expected: u64, key: &str, command: Command) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: StableId::parse(&format!("race:{key}")).unwrap(),
        actor: Actor {
            actor_id: StableId::parse("actor:race").unwrap(),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command,
        submitted_at: timestamp(),
    })
    .unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-09-08T06:00:00Z").unwrap()
}
