# Project Ledger Phase 2 (S3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the slice commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next step.

**Goal:** Implement the deterministic, storage-independent `ProjectState` projection engine and its discardable SQLite snapshot cache, proving that rebuild, incremental projection and cache recovery preserve canonical event truth.

**Architecture:** A new `projection-engine` crate folds integrity-validated event streams into `ProjectState` and computes its canonical digest. The event-store layer exposes a storage-independent snapshot-cache port; SQLite migration v2 implements that port without weakening append-only event truth. An application query service accepts a valid cache hit or rebuilds from the verified stream and repairs a missing/corrupt/stale cache. S3 supports only `PROJECT_CREATED` and `PROJECT_METADATA_UPDATED`; lifecycle, authority, roadmap, phases, status derivation and catalog reducers remain S4.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, `serde`, RFC 8785 canonical JSON, SHA-256, `rusqlite 0.40.2` and real tempfile integration tests.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; spec/schema/implementation disagreement blocks closure.
- Preserve specification bytes, migration v1 bytes and the immutable event stream.
- The event stream is canonical; `project_state_snapshots` is derived, mutable, discardable and never accepted after a digest/head/version mismatch.
- Projection validates the complete hash chain and fails closed on empty, non-genesis, duplicate-genesis, noncontiguous, cross-project or unsupported-event input.
- `state_digest` covers canonical `ProjectState` JSON after removing only `state_digest` itself.
- `state_version` advances on every supported event; `material_state_version` advances only on material events. Both supported S3 events are material.
- Incremental application and rebuild from zero must produce semantically identical state and digest.
- Query/cache repair may mutate only the derived snapshot. It must not change event count, stream version, event bytes, results, evidence, checkpoints or mappings.
- No lifecycle/authority/roadmap/phase/gate/decision/blocker/risk/action/catalog reducer, renderer, import, stdio, integration, UI, daemon, network runtime or AI is implemented in S3.
- Tests use real temporary SQLite files and no mocks, network, Python, Node, JVM or Docker.

---

### Task 1: Deterministic genesis projection and state digest

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/projection-engine/Cargo.toml`
- Create: `crates/projection-engine/src/lib.rs`
- Create: `crates/projection-engine/src/project_state.rs`
- Test: `crates/projection-engine/tests/project_state.rs`

**Interfaces:**
- Consumes: integrity-valid `EventEnvelope` values.
- Produces: `ProjectStateProjector::rebuild`, `validate_state_integrity`, `PROJECTOR_VERSION` and typed `ProjectionError`.

- [x] **Step 1: Write failing genesis/digest tests**

Assert exact genesis fields, deterministic `state:{project_uuid}` identity, hand-recomputed state digest, same input/same state and rejection of empty/non-`PROJECT_CREATED` genesis.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-projection-engine --test project_state
```

Expected: compilation fails because the crate/API does not exist.

- [x] **Step 3: Implement the minimal genesis fold**

Validate the event envelope, require project/stream consistency, initialize all non-S3 collections empty and derive timestamps/versions solely from the event.

- [x] **Step 4: Run GREEN**

Run the focused test and require all genesis/digest assertions to pass.

### Task 2: Incremental equivalence and fail-closed event handling

**Files:**
- Modify: `crates/projection-engine/src/project_state.rs`
- Test: `crates/projection-engine/tests/incremental_equivalence.rs`

**Interfaces:**
- Consumes: an integrity-valid prior `ProjectState`, its verified head digest and the exact next event.
- Produces: `ProjectStateProjector::apply` with semantic equivalence to full rebuild.

- [x] **Step 1: Write failing incremental/differential tests**

Build a two-event stream (`PROJECT_CREATED`, `PROJECT_METADATA_UPDATED`) and assert rebuild equals incremental apply byte-for-byte and by digest. Reject wrong prior state digest, gaps, project mismatch, duplicate genesis and a valid but unsupported lifecycle event.

- [x] **Step 2: Run RED**

Expected: tests fail because incremental application and metadata reduction are absent.

- [x] **Step 3: Implement the minimum closed reducer set**

Apply optional title/summary changes, advance state/material versions and last-material metadata, recompute the digest, and return `UnsupportedEvent` for every reducer owned by later slices.

- [x] **Step 4: Run GREEN**

Run both projection-engine test targets and require deterministic equivalence.

### Task 3: Discardable projection-cache port and SQLite migration v2

**Files:**
- Modify: `crates/event-store/src/ports.rs`
- Modify: `crates/event-store/src/lib.rs`
- Modify: `crates/storage-sqlite/Cargo.toml`
- Modify: `crates/storage-sqlite/src/lib.rs`
- Modify: `crates/storage-sqlite/src/migrations.rs`
- Create: `crates/storage-sqlite/src/projections.rs`
- Modify: `crates/storage-sqlite/tests/bootstrap.rs`
- Create: `crates/storage-sqlite/tests/projection_cache.rs`

**Interfaces:**
- Produces: `ProjectStateSnapshot`, `ProjectionSnapshotLoad::{Missing, Snapshot, Corrupt}` and `ProjectionSnapshotStore`.
- Persists: migration v2 table `project_state_snapshots`, keyed by project, with source head digest, state version/digest, projector version and canonical state JSON.

- [x] **Step 1: Write failing migration/cache tests**

Assert migration version 2, idempotent reopen, v1 digest preservation, snapshot roundtrip/overwrite and explicit `Corrupt` classification for malformed or metadata-divergent cached bytes.

- [x] **Step 2: Run RED**

Expected: tests fail because migration v2 and cache port are absent.

- [x] **Step 3: Implement sequential immutable migrations and cache adapter**

Apply v1 and v2 one at a time under `BEGIN IMMEDIATE`, verify every applied migration digest, serialize the state canonically and keep snapshot writes independent of canonical event writes.

- [x] **Step 4: Run GREEN**

Run storage bootstrap/cache tests and require migration/cache integrity behavior to pass.

### Task 4: Side-effect-bounded application query and cache recovery

**Files:**
- Modify: `crates/application/Cargo.toml`
- Modify: `crates/application/src/lib.rs`
- Create: `crates/application/src/project_state_query.rs`
- Create: `crates/application/tests/project_state_query.rs`

**Interfaces:**
- Consumes: `EventStore + ProjectionSnapshotStore`, project ID and the S3 projector.
- Produces: `ProjectStateQuery::load` and `ProjectionSource::{Snapshot, Rebuilt}`.

- [x] **Step 1: Write failing AC-007/009/010 and PL-09 tests**

Against real SQLite, assert rebuild from zero, snapshot hit, stale/corrupt cache recovery, identical returned state, and 100 consecutive queries with unchanged stream version/event count.

- [x] **Step 2: Run RED**

Expected: tests fail because the application query service is absent.

- [x] **Step 3: Implement verified load/rebuild/repair**

Verify/load the canonical chain, accept a snapshot only when projector version, project, source head, state version and state digest all agree; otherwise rebuild and replace only the derived cache.

- [x] **Step 4: Run GREEN**

Run the focused application test and require all recovery/non-mutation assertions to pass.

### Task 5: Slice gate, evidence and commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: this plan

**Interfaces:**
- Consumes: definitive S3 tree.
- Produces: gate evidence, explicit S4 pending boundary, progress `2/12` macrofases and `4/15` slices, and one S3 commit.

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

Expected: all commands exit 0; deterministic rebuild, incremental equivalence, cache recovery and query non-mutation tests are green.

- [x] **Step 2: Update persistent evidence and commit**

Record exact results and limitations; mark S3 complete but Fase 2 still in progress; then stage, inspect and commit with `feat: add deterministic project state projection`.

## Self-review

- Spec coverage: I-02/I-05/I-20/I-23/I-27, §13, §15.5, §25.4, §39, §40.5, §45, PL-09 and AC-007/009/010 map to Tasks 1–4.
- Scope boundary: S3 proves projection mechanics and cache recovery using only genesis and metadata semantics. S4 owns lifecycle and every collection/status rule listed in Fase 2.
- Cache authority: a cache hit is an optimization, never evidence; every query first trusts only the verified immutable event stream and its head.
- Determinism: no wall clock, randomness, filesystem metadata or insertion-order map contributes to projected state.
- No placeholders: every task names files, interfaces, RED evidence and a concrete GREEN boundary.
