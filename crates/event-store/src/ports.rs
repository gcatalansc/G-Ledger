use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    CommandEnvelope, CommandResult, CommandType, EventEnvelope, ExternalSubmissionRecord,
    ExternalSubmissionState, LedgerTimestamp, ProjectId, ProjectState, StableId,
};
use thiserror::Error;

use crate::IntegrityError;

/// One atomic command write: immutable input, resulting events, and server result.
#[derive(Clone, Debug)]
pub struct CommandCommit {
    pub command: CommandEnvelope,
    pub events: Vec<EventEnvelope>,
    pub result: CommandResult,
}

/// Outcome of an atomic commit after its idempotency scope is resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum CommitDisposition {
    /// This call created the only durable effect for the scope/key.
    Committed(CommandResult),
    /// An equal command digest already owned the scope/key.
    Replayed(CommandResult),
}

/// Result of a read-only lookup in the normative idempotency scope.
#[derive(Clone, Debug, PartialEq)]
pub enum IdempotencyLookup {
    Vacant,
    Replay(CommandResult),
    Conflict {
        existing_command_digest: Sha256Digest,
    },
}

/// Outcome of an idempotent submission snapshot/transition write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionDisposition {
    Recorded,
    Replayed,
}

/// A discardable projection snapshot tied to one exact canonical stream head.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectStateSnapshot {
    pub state: ProjectState,
    pub source_head_event_digest: Sha256Digest,
    pub projector_version: u32,
}

/// Classification of a derived snapshot read.
#[derive(Clone, Debug, PartialEq)]
pub enum ProjectionSnapshotLoad {
    Missing,
    Snapshot(Box<ProjectStateSnapshot>),
    /// Bytes or redundant metadata disagree and the cache must be rebuilt.
    Corrupt,
}

/// Durable failures shared by storage adapters and the application boundary.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("storage serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("storage adapter failed: {message}")]
    Persistence { message: String },
    #[error("stream version conflict: expected {expected}, actual {actual}")]
    StreamVersionConflict { expected: u64, actual: u64 },
    #[error("invalid event sequence: {message}")]
    InvalidEventSequence { message: String },
    #[error("idempotency key already belongs to a different command digest")]
    IdempotencyConflict {
        existing_command_digest: Sha256Digest,
    },
    #[error("corrupt stream at version {version:?}: {message}")]
    CorruptStream {
        version: Option<u64>,
        message: String,
    },
    #[error("invalid command commit: {message}")]
    InvalidCommit { message: String },
    #[error("external submission state conflict: expected {expected}, actual {actual}")]
    SubmissionStateConflict { expected: String, actual: String },
}

/// Storage-independent event-stream and command-result boundary.
pub trait EventStore {
    /// Reads the durable stream head, treating an absent stream as version zero.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage or indexed stream metadata is invalid.
    fn current_version(&self, project_ref: &ProjectId) -> Result<u64, StoreError>;

    /// Counts immutable events belonging to a project stream.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage cannot complete the query.
    fn event_count(&self, project_ref: &ProjectId) -> Result<u64, StoreError>;

    /// Loads and integrity-validates a project stream in version order.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage, decoding, or event integrity fails.
    fn load_stream(&self, project_ref: &ProjectId) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Audits versions, previous digests, envelopes, and the stored stream head.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for divergence, corruption, or storage failures.
    fn verify_event_chain(&self, project_ref: &ProjectId) -> Result<(), StoreError>;

    /// Resolves the full idempotency scope before command handling.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the lookup or stored result cannot be trusted.
    fn lookup_idempotency(
        &self,
        project_ref: &ProjectId,
        command_type: CommandType,
        actor_id: &StableId,
        trust_domain: &str,
        idempotency_key: &StableId,
        command_digest: Sha256Digest,
    ) -> Result<IdempotencyLookup, StoreError>;

    /// Atomically stores one command, its events, result, and idempotency owner.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for conflicts, invalid batches, or persistence failures.
    fn commit_command(&mut self, commit: CommandCommit) -> Result<CommitDisposition, StoreError>;

    /// Finalizes a version rejection against the head locked by this transaction.
    /// Resolves idempotency first, then atomically stores the unchanged command, its
    /// `VERSION_CONFLICT` result and scope owner, with no events. Never reruns the handler.
    ///
    /// # Errors
    /// Returns [`StoreError`] for invalid input, a matching version, a different scoped
    /// digest, corruption or persistence failure. An uncommitted result is never returned.
    fn finalize_version_conflict(
        &mut self,
        command: CommandEnvelope,
        processed_at: LedgerTimestamp,
    ) -> Result<CommitDisposition, StoreError>;

    /// Looks up a durable server result by command identity.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage or result decoding fails.
    fn command_result(
        &self,
        command_ref: &project_ledger_domain::CommandId,
    ) -> Result<Option<CommandResult>, StoreError>;
}

/// Narrow append boundary for event-only portable bundles.
///
/// The adapter must revalidate the current head and every immutable envelope
/// inside the same transaction. Portable bundles intentionally do not expose
/// private command/result tables.
pub trait EventImportStore: EventStore {
    /// Atomically appends a verified contiguous event suffix.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the destination changed after planning or
    /// any event identity, version, digest, causation metadata or genesis rule
    /// is invalid. No event may remain appended on failure.
    fn append_imported_events(
        &mut self,
        project_ref: &ProjectId,
        expected_version: u64,
        events: &[EventEnvelope],
    ) -> Result<(), StoreError>;
}

/// Storage-independent boundary for mutable, discardable projection caches.
pub trait ProjectionSnapshotStore {
    /// Loads a snapshot without treating cache corruption as canonical corruption.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] only when the storage operation itself fails.
    fn load_project_state_snapshot(
        &self,
        project_ref: &ProjectId,
    ) -> Result<ProjectionSnapshotLoad, StoreError>;

    /// Replaces the derived snapshot for its project.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when serialization, bounds or persistence fail.
    fn store_project_state_snapshot(
        &mut self,
        snapshot: &ProjectStateSnapshot,
    ) -> Result<(), StoreError>;
}

/// Storage-independent local transport-uncertainty boundary.
pub trait SubmissionStore {
    /// Returns the latest local submission snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage or snapshot integrity fails.
    fn submission(
        &self,
        submission_ref: &StableId,
    ) -> Result<Option<ExternalSubmissionRecord>, StoreError>;

    /// Returns append-only submission snapshots in transition order.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage or any historical snapshot is invalid.
    fn submission_history(
        &self,
        submission_ref: &StableId,
    ) -> Result<Vec<ExternalSubmissionRecord>, StoreError>;

    /// Creates the idempotent initial snapshot and first history entry.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for identity conflicts, invalid integrity, or storage failures.
    fn record_submission(
        &mut self,
        record: ExternalSubmissionRecord,
    ) -> Result<SubmissionDisposition, StoreError>;

    /// Performs one compare-and-set snapshot transition and appends its history.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for stale state, invalid replacement, or storage failures.
    fn transition_submission(
        &mut self,
        expected_state: ExternalSubmissionState,
        replacement: ExternalSubmissionRecord,
        transitioned_at: &LedgerTimestamp,
    ) -> Result<SubmissionDisposition, StoreError>;

    /// Counts local records whose transport outcome still requires reconciliation.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when storage cannot complete the query.
    fn unresolved_submission_count(&self) -> Result<u64, StoreError>;
}
