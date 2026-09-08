use project_ledger_canonical_json::{Sha256Digest, to_canonical_vec};
use project_ledger_domain::{
    CommandEnvelope, CommandId, CommandRejection, CommandResult, CommandResultStatus, CommandType,
    ErrorCode, Event, EventEnvelope, LedgerTimestamp, ProjectId, StableId,
};
use project_ledger_event_store::{
    CommandCommit, CommandResultDraft, CommitDisposition, EventImportStore, EventStore,
    IdempotencyLookup, StoreError, build_command_result, validate_command_integrity,
    validate_command_result_integrity, validate_event_integrity,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use crate::SqliteEventStore;

impl SqliteEventStore {
    /// Reads and verifies one consistent WAL snapshot, including the stored head.
    /// # Errors
    /// Returns corruption or storage failures without canonical mutations.
    pub fn load_verified_snapshot(
        &self,
        project_ref: &ProjectId,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(persistence)?;
        self.verify_event_chain(project_ref)?;
        let events = self.load_stream(project_ref)?;
        transaction.commit().map_err(persistence)?;
        Ok(events)
    }
}

impl EventStore for SqliteEventStore {
    fn current_version(&self, project_ref: &ProjectId) -> Result<u64, StoreError> {
        let version = self
            .connection
            .query_row(
                "SELECT current_version FROM event_streams WHERE project_ref = ?1",
                [project_ref.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(persistence)?
            .unwrap_or(0);
        nonnegative(version, "stream version")
    }

    fn event_count(&self, project_ref: &ProjectId) -> Result<u64, StoreError> {
        let count = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM events WHERE project_ref = ?1",
                [project_ref.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(persistence)?;
        nonnegative(count, "event count")
    }

    fn load_stream(&self, project_ref: &ProjectId) -> Result<Vec<EventEnvelope>, StoreError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT stream_version, event_type, command_ref, previous_event_digest,
                        payload_digest, event_digest, event_json, recorded_at
                 FROM events
                 WHERE project_ref = ?1 ORDER BY stream_version",
            )
            .map_err(persistence)?;
        let rows = statement
            .query_map([project_ref.as_str()], |row| {
                Ok(StoredEventRow {
                    stream_version: row.get(0)?,
                    event_type: row.get(1)?,
                    command_ref: row.get(2)?,
                    previous_event_digest: row.get(3)?,
                    payload_digest: row.get(4)?,
                    event_digest: row.get(5)?,
                    event_json: row.get(6)?,
                    recorded_at: row.get(7)?,
                })
            })
            .map_err(persistence)?;
        let mut events = Vec::new();
        for row in rows {
            let stored = row.map_err(persistence)?;
            let version = nonnegative(stored.stream_version, "stored stream version")?;
            let event: EventEnvelope =
                serde_json::from_slice(&stored.event_json).map_err(|error| {
                    StoreError::CorruptStream {
                        version: Some(version),
                        message: format!("event JSON cannot be decoded: {error}"),
                    }
                })?;
            validate_event_integrity(&event).map_err(|error| StoreError::CorruptStream {
                version: Some(version),
                message: error.to_string(),
            })?;
            if event.stream_version != version
                || event.project_ref != *project_ref
                || stored.event_type != contract_name(&event.event_type())?
                || stored.command_ref != event.causation.command_ref.as_str()
                || stored.previous_event_digest
                    != event.previous_event_digest.map(|digest| digest.to_string())
                || stored.payload_digest != event.payload_digest.to_string()
                || stored.event_digest != event.event_digest.to_string()
                || stored.recorded_at != event.recorded_at.as_str()
            {
                return Err(StoreError::CorruptStream {
                    version: Some(version),
                    message: "indexed event metadata disagrees with its canonical envelope"
                        .to_owned(),
                });
            }
            events.push(event);
        }
        Ok(events)
    }

    fn verify_event_chain(&self, project_ref: &ProjectId) -> Result<(), StoreError> {
        let events = self.load_stream(project_ref)?;
        let mut previous_digest = None;
        for (index, event) in events.iter().enumerate() {
            let expected_version = index as u64 + 1;
            if event.stream_version != expected_version
                || event.previous_event_digest != previous_digest
                || event.stream_id != *project_ref
            {
                return Err(StoreError::CorruptStream {
                    version: Some(event.stream_version),
                    message: "event chain has a gap, wrong stream, or wrong previous digest"
                        .to_owned(),
                });
            }
            previous_digest = Some(event.event_digest);
        }

        let stored_head = stream_head(&self.connection, project_ref)?;
        let observed_version = events.len() as u64;
        match stored_head {
            None if events.is_empty() => Ok(()),
            Some((version, digest)) if version == observed_version && digest == previous_digest => {
                Ok(())
            }
            _ => Err(StoreError::CorruptStream {
                version: Some(observed_version),
                message: "stream head disagrees with the immutable event chain".to_owned(),
            }),
        }
    }

    fn lookup_idempotency(
        &self,
        project_ref: &ProjectId,
        command_type: CommandType,
        actor_id: &StableId,
        trust_domain: &str,
        idempotency_key: &StableId,
        command_digest: Sha256Digest,
    ) -> Result<IdempotencyLookup, StoreError> {
        lookup_idempotency_on(
            &self.connection,
            project_ref,
            command_type,
            actor_id,
            trust_domain,
            idempotency_key,
            command_digest,
        )
    }

    fn commit_command(&mut self, commit: CommandCommit) -> Result<CommitDisposition, StoreError> {
        validate_commit_shape(&commit)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(persistence)?;
        let command_type = commit.command.command_type();
        let trust_domain = contract_name(&commit.command.actor.trust_level)?;
        match lookup_idempotency_on(
            &transaction,
            &commit.command.project_ref,
            command_type,
            &commit.command.actor.actor_id,
            &trust_domain,
            &commit.command.idempotency_key,
            commit.command.command_digest,
        )? {
            IdempotencyLookup::Vacant => {}
            IdempotencyLookup::Replay(result) => {
                return Ok(CommitDisposition::Replayed(result));
            }
            IdempotencyLookup::Conflict {
                existing_command_digest,
            } => {
                return Err(StoreError::IdempotencyConflict {
                    existing_command_digest,
                });
            }
        }

        let (current_version, head_digest) =
            stream_head(&transaction, &commit.command.project_ref)?.unwrap_or((0, None));
        if commit.command.expected_state_version != current_version
            && commit.result.status != CommandResultStatus::VersionConflict
        {
            return Err(StoreError::StreamVersionConflict {
                expected: commit.command.expected_state_version,
                actual: current_version,
            });
        }
        validate_sequence(&commit, current_version, head_digest)?;
        persist_commit(&transaction, &commit, &trust_domain)?;
        transaction.commit().map_err(persistence)?;
        Ok(CommitDisposition::Committed(commit.result))
    }

    fn finalize_version_conflict(
        &mut self,
        command: CommandEnvelope,
        processed_at: LedgerTimestamp,
    ) -> Result<CommitDisposition, StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(persistence)?;
        validate_command_integrity(&command)?;
        let trust_domain = contract_name(&command.actor.trust_level)?;
        match lookup_idempotency_on(
            &transaction,
            &command.project_ref,
            command.command_type(),
            &command.actor.actor_id,
            &trust_domain,
            &command.idempotency_key,
            command.command_digest,
        )? {
            IdempotencyLookup::Vacant => {}
            IdempotencyLookup::Replay(result) => return Ok(CommitDisposition::Replayed(result)),
            IdempotencyLookup::Conflict {
                existing_command_digest,
            } => {
                return Err(StoreError::IdempotencyConflict {
                    existing_command_digest,
                });
            }
        }
        let (current, head_digest) =
            stream_head(&transaction, &command.project_ref)?.unwrap_or((0, None));
        if command.expected_state_version == current {
            return invalid_commit(
                "cannot finalize VERSION_CONFLICT when the expected version matches",
            );
        }
        let command_result_id = StableId::parse(&format!("result:{}", command.command_id.uuid()))
            .map_err(|error| StoreError::InvalidCommit {
            message: error.to_string(),
        })?;
        let result = build_command_result(CommandResultDraft {
            command_result_id,
            command_ref: command.command_id.clone(),
            status: CommandResultStatus::VersionConflict,
            previous_state_version: current,
            new_state_version: current,
            emitted_event_refs: Vec::new(),
            resulting_checkpoint_ref: None,
            rejection: Some(CommandRejection {
                code: ErrorCode::VersionConflict,
                message: format!(
                    "expected state version {}, actual {current}",
                    command.expected_state_version
                ),
                subject_refs: Vec::new(),
            }),
            replayed: false,
            processed_at,
        })?;
        let commit = CommandCommit {
            command,
            events: Vec::new(),
            result,
        };
        validate_commit_shape(&commit)?;
        validate_sequence(&commit, current, head_digest)?;
        persist_commit(&transaction, &commit, &trust_domain)?;
        transaction.commit().map_err(persistence)?;
        Ok(CommitDisposition::Committed(commit.result))
    }

    fn command_result(&self, command_ref: &CommandId) -> Result<Option<CommandResult>, StoreError> {
        let stored = self
            .connection
            .query_row(
                "SELECT result_digest, result_json FROM command_results WHERE command_ref = ?1",
                [command_ref.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(persistence)?;
        stored
            .map(|(digest, bytes)| decode_command_result(&digest, &bytes))
            .transpose()
    }
}

impl SqliteEventStore {
    /// Reads and verifies a stored command for reconciliation, without mutation.
    ///
    /// # Errors
    /// Rejects mismatched identity, digest, malformed JSON or storage failures.
    pub fn recorded_command(&self, id: &CommandId) -> Result<Option<CommandEnvelope>, StoreError> {
        let bytes: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT command_json FROM commands WHERE command_id=?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(persistence)?;
        bytes
            .map(|bytes| {
                let command: CommandEnvelope = serde_json::from_slice(&bytes)?;
                validate_command_integrity(&command)?;
                if command.command_id != *id {
                    return Err(StoreError::InvalidCommit {
                        message: "stored command identity mismatch".to_owned(),
                    });
                }
                Ok(command)
            })
            .transpose()
    }

    /// Resolves a durable result using the complete canonical idempotency scope.
    ///
    /// # Errors
    /// Rejects corrupt command/result data or storage failures. Does not write.
    pub fn reconcile_command(
        &self,
        project: &ProjectId,
        kind: CommandType,
        actor: &StableId,
        trust: &str,
        key: &StableId,
    ) -> Result<Option<CommandResult>, StoreError> {
        let stored: Option<Vec<u8>> = self.connection.query_row(
            "SELECT c.command_json FROM idempotency_keys i JOIN commands c ON c.command_id=i.command_ref
             WHERE i.project_ref=?1 AND i.command_type=?2 AND i.actor_id=?3 AND i.trust_domain=?4 AND i.idempotency_key=?5",
            rusqlite::params![project.as_str(), contract_name(&kind)?, actor.as_str(), trust, key.as_str()],
            |row| row.get(0),
        ).optional().map_err(persistence)?;
        let Some(bytes) = stored else { return Ok(None) };
        let command: CommandEnvelope = serde_json::from_slice(&bytes)?;
        validate_command_integrity(&command)?;
        if command.project_ref != *project
            || command.command_type() != kind
            || command.actor.actor_id != *actor
            || contract_name(&command.actor.trust_level)? != trust
            || command.idempotency_key != *key
        {
            return Err(StoreError::InvalidCommit {
                message: "reconciliation scope differs from stored command".to_owned(),
            });
        }
        match self.lookup_idempotency(project, kind, actor, trust, key, command.command_digest)? {
            IdempotencyLookup::Replay(result) if result.command_ref == command.command_id => {
                Ok(Some(result))
            }
            _ => Err(StoreError::InvalidCommit {
                message: "reconciliation result is inconsistent".to_owned(),
            }),
        }
    }
}

impl EventImportStore for SqliteEventStore {
    fn append_imported_events(
        &mut self,
        project_ref: &ProjectId,
        expected_version: u64,
        events: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        if events.is_empty() {
            return Ok(());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(persistence)?;
        let (current_version, mut previous_digest) =
            stream_head(&transaction, project_ref)?.unwrap_or((0, None));
        if current_version != expected_version {
            return Err(StoreError::StreamVersionConflict {
                expected: expected_version,
                actual: current_version,
            });
        }
        if current_version == 0 && !matches!(events[0].event, Event::ProjectCreated(_)) {
            return invalid_sequence("a portable new stream requires PROJECT_CREATED genesis");
        }
        for (index, event) in events.iter().enumerate() {
            validate_event_integrity(event)?;
            let expected_event_version = current_version + index as u64 + 1;
            if event.project_ref != *project_ref
                || event.stream_id != *project_ref
                || event.stream_version != expected_event_version
                || event.previous_event_digest != previous_digest
            {
                return invalid_sequence(
                    "portable event project, version, stream, or previous digest is inconsistent",
                );
            }
            previous_digest = Some(event.event_digest);
        }

        transaction
            .execute(
                "INSERT INTO event_streams(project_ref, current_version, head_event_digest)
                 VALUES (?1, 0, NULL) ON CONFLICT(project_ref) DO NOTHING",
                [project_ref.as_str()],
            )
            .map_err(persistence)?;
        for event in events {
            let stream_version = sqlite_integer(event.stream_version, "event stream version")?;
            transaction
                .execute(
                    "INSERT INTO events(
                        event_id, project_ref, stream_version, event_type, command_ref,
                        previous_event_digest, payload_digest, event_digest, event_json, recorded_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        event.event_id.as_str(),
                        event.project_ref.as_str(),
                        stream_version,
                        contract_name(&event.event_type())?,
                        event.causation.command_ref.as_str(),
                        event.previous_event_digest.map(|digest| digest.to_string()),
                        event.payload_digest.to_string(),
                        event.event_digest.to_string(),
                        to_canonical_vec(event).map_err(|error| StoreError::Persistence {
                            message: error.to_string(),
                        })?,
                        event.recorded_at.as_str(),
                    ],
                )
                .map_err(persistence)?;
        }
        let last = events
            .last()
            .ok_or_else(|| StoreError::InvalidEventSequence {
                message: "portable suffix unexpectedly became empty".to_owned(),
            })?;
        transaction
            .execute(
                "UPDATE event_streams SET current_version = ?2, head_event_digest = ?3
                 WHERE project_ref = ?1",
                params![
                    project_ref.as_str(),
                    sqlite_integer(last.stream_version, "stream head version")?,
                    last.event_digest.to_string(),
                ],
            )
            .map_err(persistence)?;
        transaction.commit().map_err(persistence)?;
        Ok(())
    }
}

fn validate_commit_shape(commit: &CommandCommit) -> Result<(), StoreError> {
    validate_command_integrity(&commit.command)?;
    validate_command_result_integrity(&commit.result)?;
    if commit.result.command_ref != commit.command.command_id {
        return invalid_commit("result references a different command");
    }
    if commit.result.status == CommandResultStatus::Accepted {
        if commit.events.is_empty() || commit.result.rejection.is_some() {
            return invalid_commit("ACCEPTED commits require events and no rejection");
        }
    } else if !commit.events.is_empty()
        || commit.result.rejection.is_none()
        || commit.result.status == CommandResultStatus::DuplicateReplay
    {
        return invalid_commit("rejected commits require no events and a rejection");
    }
    let event_refs: Vec<_> = commit
        .events
        .iter()
        .map(|event| event.event_id.clone())
        .collect();
    if event_refs != commit.result.emitted_event_refs {
        return invalid_commit("result event references do not match the event batch");
    }
    Ok(())
}

fn validate_sequence(
    commit: &CommandCommit,
    current_version: u64,
    mut previous_digest: Option<Sha256Digest>,
) -> Result<(), StoreError> {
    if current_version == 0 && !commit.events.is_empty() {
        let Some(first) = commit.events.first() else {
            return invalid_sequence("a new stream requires a genesis event");
        };
        if !matches!(first.event, Event::ProjectCreated(_)) {
            return invalid_sequence("genesis must be PROJECT_CREATED");
        }
    }

    for (index, event) in commit.events.iter().enumerate() {
        validate_event_integrity(event)?;
        let expected_version = current_version + index as u64 + 1;
        if event.project_ref != commit.command.project_ref
            || event.stream_id != commit.command.project_ref
            || event.stream_version != expected_version
            || event.previous_event_digest != previous_digest
            || event.causation.command_ref != commit.command.command_id
            || event.causation.command_digest != commit.command.command_digest
        {
            return invalid_sequence(
                "event project, version, previous digest, or causation is inconsistent",
            );
        }
        previous_digest = Some(event.event_digest);
    }

    let new_version = current_version + commit.events.len() as u64;
    if commit.result.previous_state_version != current_version
        || commit.result.new_state_version != new_version
    {
        return invalid_commit("result versions do not match the committed event batch");
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // SQL columns stay explicit and adjacent for auditability.
fn persist_commit(
    transaction: &Transaction<'_>,
    commit: &CommandCommit,
    trust_domain: &str,
) -> Result<(), StoreError> {
    let command_type = contract_name(&commit.command.command_type())?;
    let expected_state_version = sqlite_integer(
        commit.command.expected_state_version,
        "expected state version",
    )?;
    transaction
        .execute(
            "INSERT INTO commands(
                command_id, project_ref, command_type, actor_id, trust_domain,
                idempotency_key, expected_state_version, command_digest, command_json, submitted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                commit.command.command_id.as_str(),
                commit.command.project_ref.as_str(),
                command_type,
                commit.command.actor.actor_id.as_str(),
                trust_domain,
                commit.command.idempotency_key.as_str(),
                expected_state_version,
                commit.command.command_digest.to_string(),
                to_canonical_vec(&commit.command).map_err(|error| StoreError::Persistence {
                    message: error.to_string(),
                })?,
                commit.command.submitted_at.as_str(),
            ],
        )
        .map_err(persistence)?;

    if !commit.events.is_empty() {
        transaction
            .execute(
                "INSERT INTO event_streams(project_ref, current_version, head_event_digest)
                 VALUES (?1, 0, NULL) ON CONFLICT(project_ref) DO NOTHING",
                [commit.command.project_ref.as_str()],
            )
            .map_err(persistence)?;
    }
    for event in &commit.events {
        let stream_version = sqlite_integer(event.stream_version, "event stream version")?;
        transaction
            .execute(
                "INSERT INTO events(
                    event_id, project_ref, stream_version, event_type, command_ref,
                    previous_event_digest, payload_digest, event_digest, event_json, recorded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.event_id.as_str(),
                    event.project_ref.as_str(),
                    stream_version,
                    contract_name(&event.event_type())?,
                    event.causation.command_ref.as_str(),
                    event.previous_event_digest.map(|digest| digest.to_string()),
                    event.payload_digest.to_string(),
                    event.event_digest.to_string(),
                    to_canonical_vec(event).map_err(|error| StoreError::Persistence {
                        message: error.to_string(),
                    })?,
                    event.recorded_at.as_str(),
                ],
            )
            .map_err(persistence)?;
    }
    if let Some(last) = commit.events.last() {
        let stream_version = sqlite_integer(last.stream_version, "stream head version")?;
        transaction
            .execute(
                "UPDATE event_streams SET current_version = ?2, head_event_digest = ?3
                 WHERE project_ref = ?1",
                params![
                    commit.command.project_ref.as_str(),
                    stream_version,
                    last.event_digest.to_string()
                ],
            )
            .map_err(persistence)?;
    }

    transaction
        .execute(
            "INSERT INTO command_results(
                command_result_id, command_ref, status, result_digest, result_json, processed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                commit.result.command_result_id.as_str(),
                commit.result.command_ref.as_str(),
                contract_name(&commit.result.status)?,
                commit.result.digest.to_string(),
                to_canonical_vec(&commit.result).map_err(|error| StoreError::Persistence {
                    message: error.to_string(),
                })?,
                commit.result.processed_at.as_str(),
            ],
        )
        .map_err(persistence)?;
    transaction
        .execute(
            "INSERT INTO idempotency_keys(
                project_ref, command_type, actor_id, trust_domain, idempotency_key,
                command_digest, command_ref, command_result_ref
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                commit.command.project_ref.as_str(),
                command_type,
                commit.command.actor.actor_id.as_str(),
                trust_domain,
                commit.command.idempotency_key.as_str(),
                commit.command.command_digest.to_string(),
                commit.command.command_id.as_str(),
                commit.result.command_result_id.as_str(),
            ],
        )
        .map_err(persistence)?;
    Ok(())
}

fn lookup_idempotency_on(
    connection: &rusqlite::Connection,
    project_ref: &ProjectId,
    command_type: CommandType,
    actor_id: &StableId,
    trust_domain: &str,
    idempotency_key: &StableId,
    command_digest: Sha256Digest,
) -> Result<IdempotencyLookup, StoreError> {
    let command_type = contract_name(&command_type)?;
    let found = connection
        .query_row(
            "SELECT i.command_digest, r.result_json
             FROM idempotency_keys i
             JOIN command_results r ON r.command_result_id = i.command_result_ref
             WHERE i.project_ref = ?1 AND i.command_type = ?2 AND i.actor_id = ?3
               AND i.trust_domain = ?4 AND i.idempotency_key = ?5",
            params![
                project_ref.as_str(),
                command_type,
                actor_id.as_str(),
                trust_domain,
                idempotency_key.as_str(),
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(persistence)?;
    let Some((digest_text, result_bytes)) = found else {
        return Ok(IdempotencyLookup::Vacant);
    };
    let existing_command_digest =
        Sha256Digest::parse(&digest_text).map_err(|error| StoreError::CorruptStream {
            version: None,
            message: format!("invalid idempotency digest: {error}"),
        })?;
    if existing_command_digest != command_digest {
        return Ok(IdempotencyLookup::Conflict {
            existing_command_digest,
        });
    }
    let result = decode_command_result_from_bytes(&result_bytes)?;
    Ok(IdempotencyLookup::Replay(result))
}

#[derive(Debug)]
struct StoredEventRow {
    stream_version: i64,
    event_type: String,
    command_ref: String,
    previous_event_digest: Option<String>,
    payload_digest: String,
    event_digest: String,
    event_json: Vec<u8>,
    recorded_at: String,
}

fn decode_command_result(digest: &str, bytes: &[u8]) -> Result<CommandResult, StoreError> {
    let result = decode_command_result_from_bytes(bytes)?;
    if result.digest.to_string() != digest {
        return Err(StoreError::CorruptStream {
            version: None,
            message: "indexed command-result digest disagrees with its envelope".to_owned(),
        });
    }
    Ok(result)
}

fn decode_command_result_from_bytes(bytes: &[u8]) -> Result<CommandResult, StoreError> {
    let result: CommandResult = serde_json::from_slice(bytes)?;
    validate_command_result_integrity(&result).map_err(|error| StoreError::CorruptStream {
        version: None,
        message: format!("command-result integrity failed: {error}"),
    })?;
    Ok(result)
}

fn stream_head(
    connection: &rusqlite::Connection,
    project_ref: &ProjectId,
) -> Result<Option<(u64, Option<Sha256Digest>)>, StoreError> {
    let found = connection
        .query_row(
            "SELECT current_version, head_event_digest FROM event_streams WHERE project_ref = ?1",
            [project_ref.as_str()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(persistence)?;
    found
        .map(|(version, digest)| {
            let version = nonnegative(version, "stream version")?;
            let digest = digest
                .map(|value| {
                    Sha256Digest::parse(&value).map_err(|error| StoreError::CorruptStream {
                        version: Some(version),
                        message: format!("invalid head digest: {error}"),
                    })
                })
                .transpose()?;
            Ok((version, digest))
        })
        .transpose()
}

fn contract_name<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| StoreError::Persistence {
            message: "closed contract enum did not serialize as text".to_owned(),
        })
}

fn nonnegative(value: i64, kind: &str) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::CorruptStream {
        version: None,
        message: format!("negative {kind}"),
    })
}

fn sqlite_integer(value: u64, kind: &str) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::InvalidCommit {
        message: format!("{kind} exceeds SQLite's signed integer range"),
    })
}

fn invalid_commit<T>(message: &str) -> Result<T, StoreError> {
    Err(StoreError::InvalidCommit {
        message: message.to_owned(),
    })
}

fn invalid_sequence<T>(message: &str) -> Result<T, StoreError> {
    Err(StoreError::InvalidEventSequence {
        message: message.to_owned(),
    })
}

#[allow(clippy::needless_pass_by_value)] // `Result::map_err` transfers ownership here.
fn persistence(error: rusqlite::Error) -> StoreError {
    StoreError::Persistence {
        message: error.to_string(),
    }
}
