# Project Ledger Phase 1 (S2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next phase.

**Goal:** Implement the transactional SQLite/WAL event store, command execution boundary, optimistic concurrency, idempotency, and external submission reconciliation required by Fase 1 / S2.

**Architecture:** `event-store` defines storage-independent ports and result types. `storage-sqlite` owns migrations, WAL, append-only constraints, transactions and recovery verification. `application` validates canonical digests and the command-event matrix, builds the event chain, orchestrates pure handlers, and separates transport uncertainty from server results. The CLI adds only the offline `init` entrypoint backed by a real `CREATE_PROJECT` handler.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, `rusqlite 0.40.2` with bundled native SQLite, standard OS user-data conventions, `serde`, RFC 8785 canonical JSON, SHA-256 and real tempfile integration tests.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; spec/schema/implementation disagreement blocks closure.
- Preserve v0.1.0 and v0.2.0 specification bytes and their recorded SHA-256 digests.
- S2 only: no projection engine, lifecycle derivation, gate/evidence engine, Markdown, portable import, stdio, integrations, UI, daemon, network runtime or AI.
- Domain remains independent of SQLite and CLI; application remains independent of SQLite.
- All canonical writes traverse `CommandEnvelope`; there is no arbitrary external event append API.
- Events are immutable, gap-free and chained; genesis is version 1 with `previous_event_digest = null`.
- Every concurrent/external write has resolved expected version and scoped idempotency key.
- Same scope/key/digest produces at most one effect; same scope/key/different digest fails closed.
- `OUTCOME_UNKNOWN` exists only in `ExternalSubmissionRecord`, never in `CommandResult`.
- Tests use real temporary SQLite files and no mocks, network, Python, Node, JVM or Docker.
- Phase closure requires format, Clippy, all tests, schema drift, spec digests, dependency policy, SQLite integrity and clean Git evidence.

---

### Task 1: Storage-independent ports and canonical integrity

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/event-store/Cargo.toml`
- Create: `crates/event-store/src/lib.rs`
- Create: `crates/application/Cargo.toml`
- Create: `crates/application/src/lib.rs`
- Create: `crates/application/src/integrity.rs`
- Test: `crates/application/tests/integrity.rs`

**Interfaces:**
- Consumes: S1 command, event, result and submission contracts.
- Produces: `EventStore`, `SubmissionStore`, `CommandCommit`, `CommitDisposition`, `StoreError`, `prepare_command`, `validate_command_integrity`, `build_event`, `validate_event_integrity`, and `build_command_result`.

- [x] **Step 1: Write failing digest tests**

Use literal drafts. Changing a prepared payload must yield `PayloadDigestMismatch`; changing envelope metadata must yield `EnvelopeDigestMismatch`; a built genesis event must have version 1, no prior digest and hand-recomputed canonical digests.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-application --test integrity
```

Expected: compilation fails because the crates and integrity APIs do not exist.

- [x] **Step 3: Implement minimal ports and builders**

Payload digest covers the discriminated payload value only. Envelope digest covers canonical JSON after removing only its own digest field; validation never rewrites input.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-application --test integrity
```

Expected: all command/event integrity tests pass.

### Task 2: SQLite migration, WAL and append-only boundary

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/storage-sqlite/Cargo.toml`
- Create: `crates/storage-sqlite/src/lib.rs`
- Create: `crates/storage-sqlite/src/migrations.rs`
- Test: `crates/storage-sqlite/tests/bootstrap.rs`

**Interfaces:**
- Consumes: a database path and the S2 ports.
- Produces: `SqliteEventStore::open`, `journal_mode`, `migration_version`, `quick_check`, and schema v1 tables for streams, events, commands, results, idempotency, submissions and transition history.

- [x] **Step 1: Write failing bootstrap tests**

On a real tempfile assert WAL, migration version 1, bounded busy timeout, foreign keys and `quick_check=ok`. After one event exists, a second raw SQLite connection must prove defensive triggers reject `UPDATE` and `DELETE`.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-storage-sqlite --test bootstrap
```

Expected: compilation fails because storage does not exist.

- [x] **Step 3: Implement migration v1 and connection policy**

Use native bundled SQLite, `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=FULL`, five-second busy timeout and migration under `BEGIN IMMEDIATE`. Expose no event mutation API.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-storage-sqlite --test bootstrap
```

Expected: bootstrap and append-only trigger tests pass.

### Task 3: Gap-free append, load and mandatory chain

**Files:**
- Modify: `crates/storage-sqlite/src/lib.rs`
- Create: `crates/storage-sqlite/src/events.rs`
- Test: `crates/storage-sqlite/tests/event_chain.rs`

**Interfaces:**
- Consumes: one validated command, its contiguous events and accepted result.
- Produces: `current_version`, `load_stream`, `event_count`, `commit_command`, and `verify_event_chain`.

- [x] **Step 1: Write failing chain tests**

Assert genesis and version 2 roundtrip; reject a missing genesis, gap, wrong prior digest and duplicate version; fail verification after controlled tampering; prove reads do not change count/version.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-storage-sqlite --test event_chain
```

Expected: tests fail because stream operations are absent.

- [x] **Step 3: Implement transactional append and verification**

Within `BEGIN IMMEDIATE`, compare stream head, enforce exact versions/prior digest, persist canonical JSON bytes, update only the head, and verify payload/envelope digests during read/audit.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-storage-sqlite --test event_chain
```

Expected: append/load/query/chain tests pass with zero gaps or mutations.

### Task 4: Command processor, matrix, idempotency and expected version

**Files:**
- Create: `crates/application/src/processor.rs`
- Create: `crates/application/src/create_project.rs`
- Modify: `crates/application/src/lib.rs`
- Create: `crates/storage-sqlite/src/commands.rs`
- Modify: `crates/storage-sqlite/src/lib.rs`
- Test: `crates/application/tests/command_transaction.rs`

**Interfaces:**
- Consumes: prepared command, pure `CommandHandler`, real `EventStore`, `Clock`, and the S1 matrix.
- Produces: `CommandProcessor::execute`, `CommandHandler`, `EventDraft`, `CreateProjectHandler`, and accepted/replayed/version/idempotency outcomes.

- [x] **Step 1: Write failing AC-036…040 tests**

Against real SQLite assert: first create emits one event; same scoped key/digest returns replay with one effect; same key/different digest fails `IDEMPOTENCY_CONFLICT`; stale expected version returns `VERSION_CONFLICT` without append; forbidden event fails `COMMAND_EVENT_CONTRACT_VIOLATION`; transport `OUTCOME_UNKNOWN` cannot deserialize as a server result.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-application --test command_transaction
```

Expected: tests fail because processor and transactional command persistence are absent.

- [x] **Step 3: Implement processor and create-project handler**

Flow is digest validation → idempotency lookup → expected version → pure handler → matrix validation → event chain → one SQLite command/events/result transaction. Scope is project, command type, actor ID/trust domain and key.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-application --test command_transaction
```

Expected: AC-036…040 behaviors pass.

### Task 5: External submission state and reconciliation

**Files:**
- Create: `crates/application/src/submission.rs`
- Modify: `crates/application/src/lib.rs`
- Create: `crates/storage-sqlite/src/submissions.rs`
- Modify: `crates/storage-sqlite/src/lib.rs`
- Test: `crates/application/tests/submission_reconciliation.rs`

**Interfaces:**
- Consumes: submission record, expected prior state, command/result lookup and reconciliation time.
- Produces: `SubmissionService::record`, `mark_outcome_unknown`, `reconcile`, `submission_history`, and `ReconciliationOutcome::{Accepted, Rejected, NotFound}`.

- [x] **Step 1: Write failing PL-08/PL-20 tests**

Assert `SUBMITTED → OUTCOME_UNKNOWN` persists without fabricating a server result; accepted/rejected reconciliation appends one transition; no result leaves unknown and returns `NotFound`; illegal/stale transitions fail; replay does not duplicate history.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-application --test submission_reconciliation
```

Expected: tests fail because submission persistence/service are absent.

- [x] **Step 3: Implement compare-and-set transitions and reconciliation**

Keep a latest snapshot plus append-only transition history. Accepted results map only to reconciled accepted; explicit server rejections map to reconciled rejected; absence never retries or changes unknown.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-application --test submission_reconciliation
```

Expected: unknown-layer tests pass.

### Task 6: Offline CLI init and recovery verification

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/init.rs`
- Create: `crates/storage-sqlite/tests/recovery.rs`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `project-ledger init --title <TEXT> [--summary <TEXT>] [--timezone <IANA>] [--data-root <PATH>]`.
- Produces: offline user-data DB with one project event/result and a recovery report covering SQLite integrity, chain and unresolved submissions.

- [x] **Step 1: Write failing CLI/recovery tests**

Run the real binary against a tempfile root. Assert exit 0, one project DB, WAL, event/result count one and no network dependency. Reopen after rollback and assert truth unchanged; controlled corruption must fail closed.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-cli --test init
cargo test -p project-ledger-storage-sqlite --test recovery
```

Expected: parser rejects init and recovery APIs are absent.

- [x] **Step 3: Implement minimal init and recovery report**

Resolve the default path from standard OS user-data conventions and use `--data-root` for explicit placement. Generate UUIDv7 IDs, prepare `CREATE_PROJECT`, execute the common processor/store path and never contact the network. The initial `directories 6.0.0` candidate was removed because its transitive MPL-2.0 dependency violated the repository's locked license policy.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-cli --test init
cargo test -p project-ledger-storage-sqlite --test recovery
```

Expected: offline init and recovery tests pass.

### Task 7: Phase gate, evidence and commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`

**Interfaces:**
- Consumes: definitive S2 tree.
- Produces: gate evidence, explicit S3 pending list, progress `2/12` and `3/15`, and one phase commit.

- [x] **Step 1: Execute the complete gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
.tooling/bin/cargo-deny check
git diff --cached --check
```

Expected: all commands exit 0 and append/rebuild/replay/version/unknown-layer tests are green.

- [x] **Step 2: Update persistent evidence and commit**

Record exact results, migration/WAL evidence, limits, CI distinction and S3 exclusions; then stage, check and commit with `feat: complete transactional event store`.

## Self-review

- Spec coverage: I-01/I-04/I-06/I-07/I-26/I-27, §15–16, §39–40, PL-06/07/08/20 and AC-006/008/036…041 map to Tasks 1–6.
- Scope boundary: rebuild here means replaying and verifying the canonical stream from version 1; semantic `ProjectState` projection/rebuild remains S3.
- Type consistency: application consumes ports; SQLite implements them; CLI assembles concrete components without leaking SQLite into domain.
- Integrity ambiguity: the spec mandates canonical digests but does not define self-field input. The non-circular rule excludes only the envelope's own digest field and retains payload/prior digests.
- No placeholders: each task has exact files, interfaces, RED evidence and a concrete GREEN boundary.
