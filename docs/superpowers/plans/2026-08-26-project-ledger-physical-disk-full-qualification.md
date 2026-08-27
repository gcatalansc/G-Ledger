# Project Ledger Physical Disk-Full Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven-development task-by-task. This checkout is being executed inline because the user authorized the next step and did not request subagent delegation.

**Goal:** Replace the `physical-disk-full` General Availability waiver with reproducible evidence that a real SQLite/WAL append fails closed under kernel `ENOSPC`, rolls back atomically and reopens with its previously committed hash chain intact.

**Architecture:** The release verifier will parse the hash-locked chaos report instead of treating it as opaque inventory and will expose separate counts for total chaos waivers, General Availability blockers and GSP runtime-certification blockers. A deterministic local SQLite `max_page_count` fault will protect the transactional invariant on every platform. A separate ignored integration test will accept only an explicitly marked, bounded path and will fill at most 64 MiB; GitHub Actions will run it inside an ext4 filesystem on a fixed 32 MiB loopback image to observe real kernel `ENOSPC`. The waiver changes only after a successful hosted job is identified at an exact commit SHA.

**Tech Stack:** Rust 1.97.1, rusqlite/SQLite WAL, GitHub Actions Ubuntu hosted runner, Linux ext4/loopback, serde/serde_json, SHA-256.

## Global Constraints

- The normative authority is `referencias/PROJECT_LEDGER_v0.2.0_CODEX/PROJECT_LEDGER_SPEC_v0.2.0.md`, especially §45.7 and Fase 11/S14.
- Never fill the workstation disk. The physical test requires a marker, writes only below its supplied root and has a hard 64 MiB cap.
- Local `PRAGMA max_page_count` evidence proves SQLite capacity-fault atomicity but is not represented as physical kernel `ENOSPC` evidence.
- `physical-disk-full` may move from `waived` to `covered` only after the hosted test observes OS error 28 and verifies rollback, reopen, `quick_check` and chain preservation.
- `live-sidecar-process-crash` remains waived and continues to block only `GSP_RUNTIME_CERTIFICATION`.
- `certified_compatibility=false` remains mandatory for all profiles; this qualification supplies no counterpart, bilateral or live-sidecar evidence.
- `release_status` remains `ENGINEERING_RELEASE_CANDIDATE` until a separate, authoritative promotion procedure is defined and executed.
- Generated release manifest and checksums must be regenerated after chaos-evidence changes.

---

### Task 1: Make chaos waivers executable release semantics (RED→GREEN)

**Files:**
- Modify: `crates/release/src/report.rs`
- Modify: `crates/release/tests/release_gate.rs`
- Modify: `crates/cli/tests/release.rs`

**Interfaces:**
- Consumes: `release/chaos-report.json`, including exact scenario inventory, evidence paths and explicit waiver blockers.
- Produces: `chaos_waived_count`, `general_availability_blocker_count`, `gsp_runtime_certification_blocker_count` in `ReleaseVerification` and CLI JSON.

- [x] **Step 1: Write the failing behavioral expectations**

  Assert the current locked report yields 2 chaos waivers, 1 GA blocker and 1 GSP runtime-certification blocker through the real library and CLI.

- [x] **Step 2: Run the focused tests and verify RED**

  Run: `cargo test -p project-ledger-release --test release_gate chaos -- --nocapture`

  Run: `cargo test -p project-ledger-cli --test release chaos -- --nocapture`

  Expected: compilation/assertion failure because `ReleaseVerification` does not yet expose or validate chaos semantics.

- [x] **Step 3: Implement the minimal fail-closed parser**

  Require the exact §45.7 scenario inventory, unique scenario names, existing safe evidence paths for covered scenarios, explicit non-critical waiver metadata and non-empty recognized blocking scopes.

- [x] **Step 4: Verify GREEN and negative mutations**

  Run both focused tests. Add controlled copied-release cases showing an unknown/duplicate scenario or missing evidence path is rejected after re-locking the copied manifest digest.

### Task 2: Protect rollback and chain preservation under a deterministic capacity fault

**Files:**
- Modify: `crates/storage-sqlite/src/lib.rs`
- Create: `crates/storage-sqlite/src/disk_full_tests.rs`
- Create: `crates/storage-sqlite/tests/support/mod.rs`

**Interfaces:**
- Consumes: a real `SqliteEventStore`, one committed genesis transaction and SQLite `PRAGMA max_page_count` on a temporary database.
- Produces: a failed oversized metadata append and a recovery report proving exactly one command/result/event and chain version 1 remain.

- [x] **Step 1: Add the real same-connection store test**

  Build literal command/event/result envelopes independently, commit genesis, restrict the active store connection's database page budget, attempt a second multi-row transaction and require `StoreError::Persistence` with a full-capacity SQLite error. The first cross-connection attempt was retained in the bitácora as an invalid injection design and was replaced without adding a test-only production API.

- [x] **Step 2: Verify the invariant**

  Remove the test-only page limit by reopening as needed; require `quick_check=ok`, WAL mode, chain verification and counts/version exactly 1 after recovery.

- [x] **Step 3: Run the focused test across normal local gates**

  Run: `cargo test -p project-ledger-storage-sqlite deterministic_sqlite_full -- --nocapture`

### Task 3: Add the bounded physical ENOSPC harness and hosted job

**Files:**
- Modify: `crates/storage-sqlite/tests/disk_full.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `BITACORA_IMPLEMENTACION.md`

**Interfaces:**
- Requires: `PROJECT_LEDGER_DISK_FULL_ROOT` containing the exact marker `project-ledger-disk-full/0.2.0`.
- Safety: rejects symlink/missing/unmarked roots, creates one fixed child directory, writes at most 64 MiB and removes only that exact child on drop.
- Hosted environment: fixed 32 MiB ext4 loopback image mounted with `nosuid,nodev,noexec` under `RUNNER_TEMP`, unmounted and its exact backing image removed in an unconditional cleanup step.
- Observable pass: filler receives Linux `ENOSPC`/OS error 28; SQLite append fails closed; after filler removal, same-process and reopened recovery both preserve version/count/digests.

- [x] **Step 1: Implement the ignored physical test with hard safety boundaries**

  The ignored test must fail closed without the opt-in root/marker and must never infer or clean a broad path.

- [x] **Step 2: Add the dedicated Ubuntu hosted job**

  Mount the ephemeral volume, create the marker, execute only the exact ignored physical test, and always unmount the exact mountpoint.

- [x] **Step 3: Run local formatting, Clippy, focused tests and the full gate**

  The normal workspace suite must compile but skip the physical test. The deterministic capacity test and release semantics must pass.

- [x] **Step 4: Commit and publish the harness phase**

  Commit message: `test: add bounded disk-full chaos gate`

### Task 4: Observe physical evidence before changing the waiver

**Files:**
- No repository changes before successful evidence exists.

**Interfaces:**
- Consumes: GitHub run/job conclusion, exact `headSha`, job ID/URL and physical test log.
- Produces: an immutable evidence locator only if the dedicated job concludes `success` and its log demonstrates OS error 28 plus recovery assertions.

- [x] **Step 1: Wait for the exact published SHA**

  Identify the workflow run by commit SHA; do not reuse earlier runs or infer success from workflow configuration.

- [x] **Step 2: Inspect the dedicated job and log**

  Require the mount, ENOSPC observation, failed append, rollback and reopened chain-recovery output. Any skipped, neutral, cancelled or failed state retains the waiver.

### Task 5: Close only the physical disk-full waiver (RED→GREEN)

**Files:**
- Modify: `crates/release/tests/release_gate.rs`
- Modify: `crates/cli/tests/release.rs`
- Modify: `release/chaos-report.json`
- Regenerate: `release/release-manifest.json`
- Regenerate: `release/checksums.sha256`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `BITACORA_IMPLEMENTACION.md`

**Interfaces:**
- Consumes: immutable successful hosted run/job/SHA evidence.
- Produces: `physical-disk-full` as covered with test path, `ENOSPC(28)` and hosted locator; chaos counts 1 total waiver, 0 GA blockers and 1 GSP runtime-certification blocker.

- [x] **Step 1: Change the executable expectations and verify RED**

  Expect zero GA blockers and one remaining GSP-specific waiver before modifying the report; run library and CLI tests to observe the current GA count of 1.

- [x] **Step 2: Update only the observed chaos evidence**

  Move `physical-disk-full` from `waived` to `covered`; retain `live-sidecar-process-crash` unchanged and retain every compatibility flag as false.

- [x] **Step 3: Regenerate the evidence chain and verify GREEN**

  Run release generation, focused release/CLI tests and checksum replay.

- [x] **Step 4: Execute the complete fresh gate**

  Run format, strict Clippy, all workspace tests/all targets/all features, locked release build, schema/spec/contract/integration/benchmark/release gates, dependency policy, checksum replay, diff check and clean-status inspection.

- [x] **Step 5: Prepare the evidence-qualified closing snapshot**

  Local closing gate: 228 tests passed, 0 failed, 1 physical test intentionally ignored locally; all release/contract/dependency gates and 13 checksums passed. The physical test passed separately in hosted run `33035586838` on source SHA `7464e2e3ff17fa3c5bdc58b198ae8b01f8ab4ae8`.

  Commit message: `chore: qualify physical disk-full recovery`.

**Post-commit delivery gate:** Push the closing snapshot and confirm its workflow, including the physical job, passes on the exact closing SHA before reporting the phase complete. This read-only final observation is reported in the delivery message; the committed evidence above retains the already-successful immutable source run rather than claiming a future result or creating a self-referential commit.

## Completion Evidence Required

- Implementation remains 12/12 macro phases and 15/15 slices.
- Acceptance qualification remains 2/2 for AC-086/AC-089.
- Physical disk-full GA qualification advances from 0/1 to 1/1 only after real hosted ENOSPC evidence.
- The release verifier reports zero General Availability chaos blockers, one GSP runtime-certification blocker and no compatibility certification overclaim.
- The final Git commit, `origin/main` and successful workflow `headSha` are identical.
