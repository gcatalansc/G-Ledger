# Project Ledger Phase 3 (S5) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next step.

**Goal:** Complete Fase 3 with safe command evidence capture/import, deterministic declarative gate evaluation, waivers, explicit phase completion and evidence-driven freshness/revalidation.

**Architecture:** `projection-engine` gains rebuildable private indexes for gate definitions/statuses/evaluations/waivers and evidence while keeping `ProjectState` as the compact public projection. New storage-independent crates `evidence` and `gate-engine` own command execution/import policy and Gate/Phase Engine decisions respectively. The application layer exposes a composite core handler that routes already verified streams to the bounded engine without bypassing expected version, idempotency, the closed command-event matrix or atomic append.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, `std::process::Command` without shell interpolation, `serde_json`, `BTreeMap`/`BTreeSet`, RFC 8785 canonical JSON, SHA-256, existing SQLite/WAL ports and real tempfile integration tests.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; preserved specifications and frozen S1 durable contracts are not edited.
- Fase 3 closes only if no gate can become `VERIFIED` without a valid immutable `GateEvaluation`, and no phase can become `COMPLETED` while a required gate is unsatisfied.
- Only Gate Engine emits evaluated gate state and only Phase Engine emits phase completion or gate/evidence-driven reopen events.
- `RECORD_VERIFICATION_RESULT` records evidence only. A green command result never directly verifies a gate or completes a phase.
- Contract limitation: no command/event carries a complete `Phase`. Required gates are derived exclusively from current `GateDefinition` records with matching `phase_ref` and `criticality=REQUIRED`; no `DEFINE_PHASE` or synthetic phase object is invented.
- Contract limitation: `Roadmap.completion_policy` only models `ALL_REQUIRED_PHASES`; the referenced phase policy `AUTO_ON_ALL_REQUIRED_GATES` is absent from the closed v0.2.0 contracts. S5 therefore never auto-completes after evidence/evaluation/waiver and requires explicit `REQUEST_PHASE_COMPLETION`.
- `REQUEST_PHASE_COMPLETION` is admitted only from a human actor because the durable contracts contain no separate approval-policy fact. It closes immediately only when every derived required gate is `VERIFIED` or `WAIVED`, authority is current, and no incompatible blocker/conflict remains.
- A required gate cannot be introduced into a completed or ready-to-close phase because `DEFINE_GATE` has no allowed `PHASE_REOPENED` outcome. The command fails closed instead of creating an invalid completed phase.
- Gate evaluation consumes only evidence from the same project with accepted source status. Unknown paths, incompatible scalar operations, missing repository bindings or mixed snapshots never evaluate true.
- `CUSTOM_DECLARATIVE` runs only the closed `PredicateAst`; no scripts, shell interpolation, templated shell strings or dynamic evaluators are introduced.
- Command evidence execution uses an exact program plus argv, an explicit allowlist and no shell. Raw stdout/stderr are never persisted by the default runner; summaries are bounded and redacted.
- Unsigned local imports are validated but never promoted automatically to `VERIFIED_SOURCE`; invalid schema/digest imports fail closed.
- Freshness transitions are explicit events. Invalidating selected evidence marks dependent verified gates stale/revalidation-required and reopens a completed phase to `REVALIDATION_REQUIRED` with the evidence reference in its reason.
- S6 checkpoint semantics remain excluded. S5 does not fabricate `CHECKPOINT_INVALIDATED` without a durable checkpoint implementation.
- No renderer, portable/legacy import, stdio, integrations, UI, daemon, network runtime or AI is implemented.
- Tests use real components and literal expectations; no mocks of the canonical store or projection are used.

---

### Task 1: Gate/evidence projection indexes and invariant reducers

**Files:**
- Modify: `crates/projection-engine/src/lifecycle.rs`
- Modify: `crates/projection-engine/src/project_state.rs`
- Test: `crates/projection-engine/tests/gates_and_evidence.rs`

**Interfaces:**
- Extends: `ProjectStateProjection` with read-only accessors for definitions, statuses, current evaluations, waivers and evidence.
- Produces: canonical `pending_gate_refs`, `verified_gate_refs` and `revalidation_required_gate_refs` plus phase gate-satisfaction checks.

- [x] **Step 1: Write failing AC-013/014/016/017 projection tests**

Assert definition/evidence/evaluation/verified ordering, rejected forged verification, required-gate phase closure, waiver projection, evidence invalidation, gate revalidation and phase reopen. Rebuild and incremental cursors must agree.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-projection-engine --test gates_and_evidence
```

Expected: S5 events are unsupported or required accessors are absent.

- [x] **Step 3: Implement minimal closed reducers**

Validate canonical record digests, unique identities and exact state transitions; keep immutable evaluation history while exposing only current refs/statuses.

- [x] **Step 4: Run GREEN**

Run all projection targets and require S3/S4 regressions to remain green.

### Task 2: Declarative PredicateAST and Gate Engine

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/gate-engine/Cargo.toml`
- Create: `crates/gate-engine/src/lib.rs`
- Create: `crates/gate-engine/src/predicate.rs`
- Create: `crates/gate-engine/src/engine.rs`
- Test: `crates/gate-engine/tests/predicate_ast.rs`
- Test: `crates/gate-engine/tests/gate_commands.rs`

**Interfaces:**
- Produces: deterministic predicate results over closed evidence JSON and `GateEngine::handle(command, verified_stream) -> Result<Vec<EventDraft>, GateEngineError>`.
- Supports: `DEFINE_GATE`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION`, `WAIVE_GATE` and `REQUEST_PHASE_COMPLETION`.

- [x] **Step 1: Write failing predicate and PL-02 command tests**

Cover all closed predicate operators, missing/invalid fields, `ALL`/`ANY`, evidence kind/status filtering, snapshot agreement, tests-pass/build-missing `INCOMPLETE`, verified/rejected event ownership and deterministic evaluation digest/ID.

- [x] **Step 2: Run RED**

Expected: `gate-engine` does not exist.

- [x] **Step 3: Implement the minimum deterministic evaluator and engines**

Filter candidate evidence fail-closed, evaluate without effects, build immutable `GateEvaluation`, emit only matrix-allowed transitions, validate human waivers and require explicit human phase completion.

- [x] **Step 4: Run GREEN**

Run gate-engine and projection suites; verify repeated handling of the same command produces identical event drafts.

### Task 3: Safe evidence runner and importer

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/evidence/Cargo.toml`
- Create: `crates/evidence/src/lib.rs`
- Create: `crates/evidence/src/runner.rs`
- Create: `crates/evidence/src/import.rs`
- Test: `crates/evidence/tests/evidence_runner.rs`
- Test: `crates/evidence/tests/evidence_import.rs`

**Interfaces:**
- Produces: `EvidenceRunner` over exact program/argv plus allowlist, and `EvidenceImporter` for closed JSON records with trust downgrade.
- Guarantees: exit code, duration and canonical payload digest captured; raw output off; no shell; unsigned import cannot become verified source.

- [x] **Step 1: Write failing AC-018/019/import-trust tests**

Execute a real harmless subprocess, reject a non-allowlisted executable, prove argv is not shell-expanded, assert privacy fields and digest integrity, reject malformed/tampered imports and downgrade unsigned verified claims.

- [x] **Step 2: Run RED**

Expected: `evidence` crate/API is absent.

- [x] **Step 3: Implement bounded runner/import policy**

Use `std::process::Command::new(program).args(argv)`, a configurable exact allowlist, bounded redacted summaries, canonical digest construction and schema-version/project/provenance checks.

- [x] **Step 4: Run GREEN**

Run evidence tests on the actual platform and confirm no raw output field is written.

### Task 4: Composite application handler and real SQLite gate lifecycle

**Files:**
- Modify: `crates/application/Cargo.toml`
- Create: `crates/application/src/core_handler.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/gate_lifecycle_transaction.rs`

**Interfaces:**
- Produces: `CoreCommandHandler` routing S5 commands to Gate Engine and existing S4 commands to Lifecycle Engine.
- Consumes: the exact integrity-checked stream supplied by `CommandProcessor`; produces one atomic matrix-checked batch.

- [x] **Step 1: Write failing real-SQLite E2E test**

Execute create→authority→roadmap→phase→gate definitions→evidence records→evaluations→waiver→explicit phase close. Assert green evidence alone does not move gates/phases and incomplete required gates reject close without appending events.

- [x] **Step 2: Run RED**

Expected: composite handler and S5 transaction path do not exist.

- [x] **Step 3: Wire bounded engines without bypasses**

Route by closed `CommandType`; preserve expected version, idempotency, command-event matrix, event chaining and atomic store commit in the existing processor.

- [x] **Step 4: Run GREEN**

Run the E2E plus every application/event-store/storage regression.

### Task 5: Freshness, revalidation and phase reopening

**Files:**
- Modify: `crates/gate-engine/src/engine.rs`
- Modify: `crates/projection-engine/src/lifecycle.rs`
- Test: `crates/application/tests/gate_lifecycle_transaction.rs`

**Interfaces:**
- Supports: `RECORD_VERIFICATION_RESULT`, `INVALIDATE_EVIDENCE`, explicit dependent gate stale/revalidation events and completed-phase reopen.

- [x] **Step 1: Write failing PL-05 and AC-016/017 tests**

Invalidate evidence selected by a verified gate, assert explicit stale→revalidation-required, reopen a completed phase to `REVALIDATION_REQUIRED` with provenance in the reason, then record replacement evidence and explicitly revalidate back to verified.

- [x] **Step 2: Run RED**

Expected: evidence commands remain unsupported or do not propagate freshness.

- [x] **Step 3: Implement dependency traversal and exact transitions**

Record evidence without lifecycle elevation, locate only current evaluations that selected the invalidated evidence, emit deterministic matrix-allowed transitions and never use wall-clock freshness inference.

- [x] **Step 4: Run GREEN**

Run application, gate and projection differential/rebuild tests.

### Task 6: Fase 3 gate, documentation and commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: this plan

**Interfaces:**
- Produces: Fase 3 closure evidence, explicit S6 exclusions, progress `4/12` macrofases and `6/15` slices, and one phase commit.

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

Expected: all commands exit 0; AC-013–021 and the two numeric Fase 3 quality gates are green locally.

- [x] **Step 2: Update persistent evidence and commit**

Record exact results and contract limitations; mark Fase 3/S5 complete but S6 pending; commit with `feat: complete deterministic gates and evidence`.

## Self-review

- Spec coverage: REQ-GAT-001/002, I-08–12, I-15, I-18, I-20, §14.4–14.5, §15.4.1, §17–18, §20–22, PL-02/03/05, AC-013–021 and Fase 3/S5 map to Tasks 1–5.
- Ownership: evidence records remain facts; Gate Engine alone interprets them; Phase Engine alone closes/reopens; Project Status Engine remains the final derived-status owner.
- Missing phase contract: required gate membership is derived from current gate definitions; no durable object is synthesized.
- Missing auto-close policy: explicit completion is the sole S5 close path, preserving the stricter normative branch.
- Security/privacy: exact argv plus allowlist, no shell, no raw default, bounded summaries, no secrets or external network.
- S6 boundary: checkpoints/resume/Markdown/audit remain unsupported and fail closed.
- Placeholder scan: no deferred implementation marker or undefined durable payload is introduced.
