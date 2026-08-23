# Project Ledger Phase 4 (S6) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the slice commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next step.

**Goal:** Complete S6 with exact checkpoint N/N+1 semantics, append-only checkpoint freshness, bounded resume projection, deterministic Markdown rendering and internal audit/verification without entering S7 import/export.

**Architecture:** `projection-engine` remains the only deterministic fold and gains private indexes for checkpoints, freshness records, work progress and unknowns. A new storage-independent `continuity` crate owns checkpoint commands, compatible freshness invalidation, resume construction, Markdown rendering and audit reports. `CoreCommandHandler` composes this engine after the existing bounded owner so checkpoint invalidation remains in the same matrix-checked SQLite transaction.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, canonical JSON/SHA-256 already fixed in the workspace, `BTreeMap`/`BTreeSet`, existing append-only event store and real SQLite integration tests. No tokenizer, template engine, filesystem watcher or new external dependency.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; preserved specs, schemas, fixtures and closed v0.2.0 command/event contracts are not edited.
- S6 implements checkpoints/resume/Markdown/audit only. Portable bundles and legacy review/import remain S7.
- `ContinuityCheckpoint.captured_state_version=N`, `checkpoint_event_version=N+1`, `checkpoint_event_ref` equals the actual event ID, and `captured_state_digest` is the digest before the checkpoint event.
- `CHECKPOINT_CREATED`, `CHECKPOINT_INVALIDATED`, `SESSION_STARTED` and `SESSION_ENDED` are continuity metadata and do not advance `material_state_version`; all other currently supported facts remain material.
- Freshness is append-only and never mutates checkpoint bytes. A newer checkpoint derives `SUPERSEDED_BY_NEWER_CHECKPOINT` for the prior latest checkpoint.
- Only commands whose frozen matrix permits `CHECKPOINT_INVALIDATED` may emit it. Compatible material facts may advance `material_state_version` without invalidating a checkpoint.
- `ContinuityCheckpoint` has one optional `RepositorySnapshot`, while `ProjectState` permits multiple bindings. Zero current snapshots maps to project-only freshness, one maps exactly, and more than one fails closed instead of selecting implicitly.
- The contracts do not name a tokenizer. `rendered_token_count` uses a documented conservative UTF-8 byte upper bound; compact output is capped at 220 budget units and agent Markdown at 450.
- The contracts contain `GeneratedArtifactRecord` but no command/event that persists it. S6 returns the record with rendered bytes and verifies its digest; no durable event is invented.
- Markdown always starts with the normative generated warning, uses LF, has all thirteen ordered sections and never reads manual edits back into canonical state.
- Full/local rendering excludes raw evidence output and secrets. Shareable rendering omits authority/repository paths and human identity details.
- Tests exercise the real fold, real command processor and real SQLite store; expected IDs/digests/budgets are literal or independently recomputed.

---

### Task 1: Projection indexes, materiality and checkpoint invariants

**Files:**
- Modify: `crates/projection-engine/src/lifecycle.rs`
- Modify: `crates/projection-engine/src/project_state.rs`
- Test: `crates/projection-engine/tests/checkpoints.rs`

**Interfaces:**
- Produces: `checkpoint`, `checkpoint_freshness`, `freshness_records`, `latest_work_record`, `unknown`, `critical_unknown_refs` and repository snapshot accessors.
- Enforces: exact pre-event snapshot equality and non-material continuity events.

- [x] **Step 1: Write failing projection tests**

Create literal streams proving: checkpoint captures state N and is registered at N+1; forged state digest/event ref/version/vector fails; checkpoint creation leaves `material_state_version=N`; invalidation leaves checkpoint immutable and appends freshness; a newer checkpoint supersedes the prior record; work progress and unknown resolution rebuild deterministically.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-projection-engine --test checkpoints
```

Expected: S6 events are unsupported and checkpoint/work/unknown accessors are absent.

- [x] **Step 3: Implement the minimal reducers**

Add private maps and exact validators. Split state advancement into material and non-material paths selected only by the closed event variant. Derive freshness IDs from immutable event UUIDs and digests from canonical records with the digest field excluded.

- [x] **Step 4: Run GREEN**

Run all projection targets and require incremental/rebuild equivalence plus S3–S5 regression suites to pass.

### Task 2: Checkpoint command engine and compatible freshness extension

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/continuity/Cargo.toml`
- Create: `crates/continuity/src/lib.rs`
- Create: `crates/continuity/src/checkpoint.rs`
- Test: `crates/continuity/tests/checkpoint_engine.rs`

**Interfaces:**
- Produces: `ContinuityEngine::handle(command, verified_stream)` for work progress, unknown record/resolve and checkpoint creation.
- Produces: `ContinuityEngine::extend_checkpoint_freshness(command, verified_stream, base_drafts)` for matrix-compatible invalidation in the same atomic batch.

- [x] **Step 1: Write failing command tests**

Prove deterministic checkpoint/event IDs, exact N/N+1 fields, creator identity, current/project-only/repository/authority/conflict freshness, rejection of empty reason, rejection of multiple current snapshots, no duplicate invalidation after stale, and reason/status selection for authority, repository and generic changes.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-continuity --test checkpoint_engine
```

Expected: the crate and engine API do not exist.

- [x] **Step 3: Implement checkpoint construction and extension**

Build the event ID before the checkpoint so `checkpoint_event_ref` is exact, hash the checkpoint with `checkpoint_digest` removed, preview every draft through `ProjectStateProjection`, and whitelist only frozen matrix rows that allow `CHECKPOINT_INVALIDATED`.

- [x] **Step 4: Run GREEN**

Run continuity and projection tests; repeated handling of the same command must produce identical drafts.

### Task 3: Bounded ResumeProjection

**Files:**
- Create: `crates/continuity/src/resume.rs`
- Test: `crates/continuity/tests/resume.rs`

**Interfaces:**
- Produces: `ResumeBuilder::build(&ProjectStateProjection, ResumeBudget) -> ResumeProjection`.
- Produces: `render_compact(&ResumeProjection) -> String` and `ConservativeTokenCounter`.

- [x] **Step 1: Write failing AC-024/025/033 tests**

Use long IDs/lists and assert compact output is at most 220 conservative units, latest stale checkpoint is labeled with its exact status, current decisions/blockers/risks/pending gates are deterministic, preferred authorized/eligible action is preserved, and critical unknown/conflict visibility has priority over optional lists.

- [x] **Step 2: Run RED**

Expected: resume APIs are absent.

- [x] **Step 3: Implement deterministic budget reduction**

Start from sorted projection refs, render without digest/count circularity, and remove lowest-priority tail entries until the conservative UTF-8 bound fits. Compute `rendered_token_count`, then canonical digest excluding `digest`.

- [x] **Step 4: Run GREEN**

Run resume tests twice and require identical JSON, text, count and digest.

### Task 4: Deterministic Markdown and audit/verify

**Files:**
- Create: `crates/continuity/src/markdown.rs`
- Create: `crates/continuity/src/audit.rs`
- Test: `crates/continuity/tests/markdown_audit.rs`

**Interfaces:**
- Produces: `MarkdownRenderer::render(projection, stream, request) -> RenderedMarkdown` containing bytes plus `GeneratedArtifactRecord`.
- Produces: `verify_stream_and_artifact(stream, artifact, bytes) -> VerificationReport` and `audit_projection(projection) -> AuditReport`.

- [x] **Step 1: Write failing AC-029–033 tests**

Assert the exact warning and thirteen ordered headings, byte-identical repeated output, renderer/profile participation, no raw evidence/path leakage in shareable mode, manual-byte modification reported as `GENERATED_ARTIFACT_MODIFIED`, regeneration restoring MATCH, canonical stream unchanged by rendering, stale checkpoint labeled, and critical unknown/conflict refs visible.

- [x] **Step 2: Run RED**

Expected: renderer and verifier APIs are absent.

- [x] **Step 3: Implement pure renderer and reports**

Render from the immutable projection/stream only, normalize LF, bound summaries, hash exact bytes, rebuild the stream in verification, compare source version/digest and artifact digest, and return stable severity/code/subject records without mutating storage.

- [x] **Step 4: Run GREEN**

Run Markdown/audit tests and the full continuity crate.

### Task 5: Core routing and real SQLite transaction

**Files:**
- Modify: `crates/application/Cargo.toml`
- Modify: `crates/application/src/core_handler.rs`
- Test: `crates/application/tests/continuity_transaction.rs`

**Interfaces:**
- Routes: work progress, unknowns and checkpoint creation to `ContinuityEngine`.
- Extends: all already-supported base batches with at most one matrix-permitted checkpoint invalidation.

- [x] **Step 1: Write failing real-SQLite E2E**

Execute create→progress→checkpoint and prove the checkpoint event is N+1 while material N is retained; then record an invalidating unknown and verify its fact plus freshness event commit atomically and rebuild exactly. Cover repository snapshot preservation and rejected multi-binding checkpoint in the engine suite; keep the existing lifecycle/cache recovery suites as regressions in the workspace gate.

- [x] **Step 2: Run RED**

Expected: Core handler rejects continuity commands or omits freshness.

- [x] **Step 3: Compose bounded owners**

Obtain the existing owner batch first, then run the continuity extension against the same verified stream. Preserve expected version, idempotency, command/event matrix, deterministic event chaining and one SQLite commit.

- [x] **Step 4: Run GREEN**

Run every application/event-store/storage test with real SQLite.

### Task 6: S6 gate, documentation and commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: this plan

**Interfaces:**
- Produces: S6 closure evidence, explicit S7 exclusions, progress `4/12` macrofases and `7/15` slices, and one slice commit.

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

Expected: all commands exit 0 and AC-024/025/027/029–033 are green locally.

- [x] **Step 2: Update persistent evidence and commit**

Record exact counts, limitations and RED→GREEN findings; keep Fase 4 open with S7 pending; commit `feat: add deterministic project continuity views`.

## Self-review

- Spec coverage: REQ-CHK-001, §25–28, §30, AC-024/025/027/029–033 and S6 map to Tasks 1–5.
- Ownership: checkpoint facts remain event-sourced; freshness records are derived append-only; resume/Markdown/audit are read-only projections.
- Contract gaps: no tokenizer and no generated-artifact event are resolved conservatively without schema invention.
- Materiality: only continuity metadata keeps the prior material version; compatible facts may leave freshness current as permitted by the frozen matrix.
- Security/privacy: no filesystem scan, watcher, shell, network, raw evidence or secrets; shareable output excludes paths/identities.
- S7 boundary: legacy candidates/reviews and portable chain/fork remain untouched.
- Placeholder scan: no deferred implementation marker or undefined durable payload is introduced.
