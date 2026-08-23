# Project Ledger Phase 2 (S4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next step.

**Goal:** Complete Fase 2 with deterministic lifecycle, authority, sequential roadmap, phase-reference transitions, decisions, blockers, risks, next actions, project-status derivation and a rebuildable project catalog.

**Architecture:** `projection-engine` gains a cursor that retains the private indexes needed to fold S4 events while exposing only canonical `ProjectState`. A new `lifecycle` crate validates S4 commands against that cursor and emits deterministic required/derived event drafts; `application` passes the verified stream to pure handlers. A new `project-catalog` crate derives sorted `ProjectCatalogEntry` values from trusted states and explicit ledger locations without becoming lifecycle authority.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, `BTreeMap`, `serde`, RFC 8785 canonical JSON, SHA-256, existing SQLite/WAL ports and real tempfile integration tests.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; preserved specifications and S1 contracts are not edited.
- Fase 2 closes only if rebuild equivalence and deterministic lifecycle pass; S5 gate/evidence/Phase Engine semantics remain excluded.
- The event stream remains canonical; projection cursor, snapshots and catalog are discardable derived state.
- Every accepted mutation still traverses `CommandEnvelope`, expected version, scoped idempotency, the closed matrix and one atomic append.
- `PAUSED`/`ARCHIVED` and explicit reopen/skip require human actors; no generic status setter or arbitrary append is introduced.
- Derived status precedence is `ARCHIVED → PAUSED → BLOCKED → COMPLETED → ACTIVE → INITIALIZING` from explicit facts, never wall clock or insertion order.
- Risks never block automatically. A blocker affects project status only when scoped to the project or current phase, or when a canonical `PHASE_BLOCKED` fact exists.
- Stable-set outputs are deterministically sorted; next actions use `AUTHORIZED`, `ELIGIBLE`, `RECOMMENDED`, `PROPOSED`, with stable ID tie-breaks. Blocked/completed/cancelled actions are excluded.
- `AUTHORIZED` next actions require a current `authority_ref`; superseded decisions are never current.
- Contract limitation: no command/event carries a complete `Phase`, so S4 validates transitions against `roadmap.phase_order` and stores phase statuses privately; `PhaseKind` remains a closed contract and, per §17.4, does not alter lifecycle.
- Contract limitation: supersession events carry only old/new refs. Replacement authority documents and decisions must already be durable and exactly match the command payload before supersession is accepted.
- No gate evaluation, evidence, checkpoint/resume workflow, Markdown, portable/legacy import, stdio, integrations, UI, daemon, network runtime or AI is implemented.
- Tests use real components and literal expectations; no mocks or network.

---

### Task 1: Stateful projection cursor for authority, roadmap, phases and status

**Files:**
- Modify: `crates/projection-engine/src/lib.rs`
- Modify: `crates/projection-engine/src/project_state.rs`
- Create: `crates/projection-engine/src/lifecycle.rs`
- Test: `crates/projection-engine/tests/lifecycle_projection.rs`

**Interfaces:**
- Produces: `ProjectStateProjection::{rebuild, apply, state, into_state}`, lookup accessors for phase status/records and `derived_lifecycle_status`.
- Supports: project pause/resume/archive/status, authority register/refresh/unavailable/mismatch/supersede, roadmap define/revise and phase start/block/ready/complete/reopen/skip.

- [x] **Step 1: Write failing projection tests**

Build literal event chains and assert authority ordering, immutable canonical authority digest on mismatch, sequential phase cardinality, phase completion/reopen refs, explicit status transitions and cursor incremental output equal to rebuild output.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-projection-engine --test lifecycle_projection
```

Expected: compilation fails because `ProjectStateProjection` and S4 reducers do not exist.

- [x] **Step 3: Implement minimal cursor/reducers**

Retain private `BTreeMap` indexes for phase status and S4 records, validate every event before reducing, canonicalize arrays before digesting and advance material/version metadata for each S4 event.

- [x] **Step 4: Run GREEN**

Run S3 plus S4 projection tests and require byte/digest equivalence.

### Task 2: Decisions, blockers, risks, actions and conflict reducers

**Files:**
- Modify: `crates/projection-engine/src/lifecycle.rs`
- Test: `crates/projection-engine/tests/continuity_records.rs`

**Interfaces:**
- Extends: `ProjectStateProjection` with decision/blocker/risk/action/conflict indexes and public read-only lookups needed by domain validation.
- Produces: canonical current/open/ordered reference arrays and preferred action.

- [x] **Step 1: Write failing AC-022/023/026 tests**

Assert accepted→superseded decision replacement, blocker/risk history surviving only in events while open refs update, risk never changing project status, authorized action rejection without authority, literal classification ordering, completion/cancellation removal and conflict open/resolved refs.

- [x] **Step 2: Run RED**

Expected: reducers are unsupported or produce empty S4 reference arrays.

- [x] **Step 3: Implement closed reducers and canonical ordering**

Reject duplicate/unknown transitions, preserve private record history, update only derived refs and recompute state digest after every event.

- [x] **Step 4: Run GREEN**

Run both S4 projection targets and the S3 differential regression tests.

### Task 3: Stream-aware lifecycle command engine

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/lifecycle/Cargo.toml`
- Create: `crates/lifecycle/src/lib.rs`
- Create: `crates/lifecycle/src/engine.rs`
- Modify: `crates/application/Cargo.toml`
- Modify: `crates/application/src/processor.rs`
- Modify: `crates/application/src/create_project.rs`
- Create: `crates/application/src/lifecycle_handler.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/lifecycle/tests/lifecycle_commands.rs`
- Test: `crates/application/tests/lifecycle_transaction.rs`
- Modify: `crates/application/tests/command_transaction.rs`

**Interfaces:**
- Changes: `CommandHandler::handle(&self, command, verified_stream)` so stateful validation consumes exactly the stream already checked by `CommandProcessor`.
- Produces: `LifecycleEngine::handle(command, stream) -> Result<Vec<EventDraft>, LifecycleError>` and an application trait implementation.

- [x] **Step 1: Write failing real-SQLite command tests**

Exercise metadata, human pause/resume/archive, authority register/observe/supersede, roadmap define/revise, phase start/reopen/skip, decision lifecycle, blocker/risk/action transitions and deterministic derived status. Assert invalid actor, duplicate IDs, stale replacement, illegal sequence and missing authority return persisted `VALIDATION_FAILED` with zero event append.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-application --test lifecycle_transaction
```

Expected: imports/API fail because the lifecycle engine and stream-aware handler contract are absent.

- [x] **Step 3: Implement deterministic command decisions**

Use a domain-separated SHA-256 derivation over command UUID plus event ordinal for deterministic, collision-resistant UUIDv7 event IDs. Preview required drafts through the projection cursor, derive status with fixed precedence/reason codes, append `PROJECT_STATUS_DERIVED` only when status changes and let the existing matrix reject any illegal output.

- [x] **Step 4: Run GREEN**

Run lifecycle commands plus every existing application transaction/replay test.

### Task 4: End-to-end lifecycle rebuild and snapshot recovery

**Files:**
- Create: `crates/application/tests/lifecycle_rebuild.rs`
- Reuse: `crates/application/src/project_state_query.rs`

**Interfaces:**
- Consumes: a real SQLite stream produced only through `CommandProcessor + LifecycleEngine`.
- Produces: evidence that cached reads, rebuild from zero and cursor incremental projection have identical state/digest across a multi-command lifecycle.

- [x] **Step 1: Write failing E2E differential test**

Execute create→authority→roadmap→phase→blocker→resolve→decision supersession→risk→actions, corrupt the cache, reload and assert exact state/status/reference/action ordering plus unchanged event count/version after repeated queries.

- [x] **Step 2: Run RED**

Expected: the full S4 stream cannot yet rebuild or repair through the query service.

- [x] **Step 3: Close projection/query integration gaps**

Make `ProjectStateProjector::rebuild` delegate to the full cursor and bump `PROJECTOR_VERSION` so all prior snapshots are deterministically invalidated.

- [x] **Step 4: Run GREEN**

Run all application/projection/storage tests and require cache repair plus differential equivalence.

### Task 5: Rebuildable project catalog

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/project-catalog/Cargo.toml`
- Create: `crates/project-catalog/src/lib.rs`
- Test: `crates/project-catalog/tests/rebuild.rs`

**Interfaces:**
- Consumes: `KnownProject { state: ProjectState, ledger_location_ref: ResourceRef }` from explicit known ledgers.
- Produces: `ProjectCatalog::rebuild(&[KnownProject]) -> Result<Vec<ProjectCatalogEntry>, CatalogError>`.

- [x] **Step 1: Write failing catalog tests**

Assert deterministic project-ref ordering independent of input order, exact title/version/digest/location mapping, invalid state rejection and duplicate project rejection.

- [x] **Step 2: Run RED**

Expected: crate/API does not exist.

- [x] **Step 3: Implement pure derived catalog rebuild**

Validate every state with the current projector, derive canonical `project-ledger://projects/<project-id>` refs, sort entries and never scan/write outside caller-provided inputs.

- [x] **Step 4: Run GREEN**

Run catalog tests and a workspace Clippy pass.

### Task 6: Fase 2 gate, evidence and commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: this plan

**Interfaces:**
- Produces: Fase 2 closure evidence, explicit S5 exclusions, progress `3/12` macrofases and `5/15` slices, and one phase commit.

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

Expected: all commands exit 0; rebuild equivalence and deterministic lifecycle are green.

- [x] **Step 2: Update persistent evidence and commit**

Record exact results and contract limitations; mark Fase 2/S4 complete but S5 pending; commit with `feat: complete deterministic project lifecycle`.

## Self-review

- Spec coverage: I-02/I-05/I-09/I-13/I-14/I-20/I-23, §9.2, §10, §13–17, §19, §23–24, §39.6, AC-022/023/026/028 and Fase 2/S4 map to Tasks 1–5.
- S5 boundary: required-gate closure, evaluation, waiver, freshness and gate-driven phase reopen remain unsupported and fail closed.
- Phase gap: no durable Phase-definition event exists; the plan uses only roadmap refs and never manufactures title/kind/status records outside events.
- Supersession gap: pre-registration/proposal makes replacement data durable before the ref-only supersession event.
- Type consistency: projection cursor is the read model consumed by lifecycle; application supplies the already verified stream; catalog consumes only validated public states.
- Placeholder scan: no deferred implementation marker or undefined interface remains.
