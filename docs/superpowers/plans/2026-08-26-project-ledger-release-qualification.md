# Project Ledger Release Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven-development task-by-task. This checkout is being executed inline because the user authorized the next step and did not request subagent delegation.

**Goal:** Resolve the two acceptance/performance evidence waivers AC-086 and AC-089 without changing the completed S0–S14 implementation roadmap or overstating compatibility or General Availability.

**Architecture:** The first qualification gate measures the resident set of the real release binary from the external `xtask` process and folds the result into the deterministic benchmark/release evidence chain. The second gate remains evidence-only: AC-086 can change from `WAIVED` to `PASS` only after identifiable hosted Windows, Linux and macOS executions exist for the exact candidate commit.

**Tech Stack:** Rust 1.97.1, `sysinfo` 0.39.6 with default features disabled, serde/serde_json, SHA-256, GitHub Actions, SPDX 2.3.

## Global Constraints

- The normative authority is `referencias/PROJECT_LEDGER_v0.2.0_CODEX/PROJECT_LEDGER_SPEC_v0.2.0.md`, especially §44 and AC-086/AC-089.
- The implementation roadmap remains 12/12 macro phases and 15/15 slices; qualification progress is tracked separately as resolved waivers out of 2.
- `memory one-shot p95` must be measured against the release `project-ledger` process and remain below 100 MiB on a declared environment.
- The sampler must remain outside the product binary, use no shell, add no network/daemon behavior and introduce no repository-owned `unsafe`.
- A sampled resident-set maximum is reported as an environment-specific engineering measurement, not as a universal or kernel-exact lifetime guarantee.
- AC-086 remains `WAIVED` until hosted runs for the exact commit can be identified; workflow configuration alone is not execution evidence.
- `certified_compatibility=false` remains mandatory until counterpart/bilateral/live evidence exists.
- Generated SBOM, release manifest and checksums must be regenerated after dependency or evidence changes.

---

### Task 1: Portable peak-RSS qualification (RED→GREEN)

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `xtask/Cargo.toml`
- Modify: `xtask/src/benchmark.rs`
- Modify: `xtask/tests/release_command.rs`
- Modify: `benchmark/project-ledger-v0.2.0-windows-x86_64.json`
- Modify: `release/acceptance-report.json`
- Regenerate: `release/sbom.spdx.json`
- Regenerate: `release/release-manifest.json`
- Regenerate: `release/checksums.sha256`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`

**Interfaces:**
- Consumes: release binary at `target/release/project-ledger[.exe]`, literal command `contracts verify`, `sysinfo::System`, `sysinfo::ProcessesToUpdate`, `sysinfo::ProcessRefreshKind`.
- Produces: benchmark metric `{name:"memory_one_shot_p95",unit:"MiB",budget:100.0,p95:17.855,samples:20,passed:true}` and an AC-089 `PASS` evidence row with no memory waiver.

- [x] **Step 1: Write the failing behavioral test**

  Change the real `xtask benchmark check` expectation to `9 measured metrics, 0 explicit waivers` and add a temporary-root case showing that the previously accepted 8-metric/peak-RSS-waiver report is rejected.

- [x] **Step 2: Run the focused test and verify RED**

  Run: `cargo test -p xtask --test release_command ac_089 -- --nocapture`

  Expected: FAIL because the current checker still accepts eight metrics and one peak-RSS waiver.

- [x] **Step 3: Add the minimal external sampler**

  Spawn the release binary with literal argv `contracts verify` from the repository root; refresh only that PID's memory through `ProcessRefreshKind::nothing().with_memory().without_tasks()`; sample every 1 ms until exit; retain each run's maximum resident bytes; fail on unsupported/absent measurements or a non-zero child exit. `--version` was rejected during GREEN because its lifetime was shorter than the first observable Windows RSS sample.

- [x] **Step 4: Tighten validation and verify GREEN**

  Require the exact nine-metric inventory, `measurement_status=COMPLETE`, zero waivers, 20 samples per metric and every p95 under its declared budget. Run the focused command test until it passes.

- [x] **Step 5: Generate evidence and close only AC-089**

  Build the release binary; run `cargo xtask benchmark run --root .`; record the observed p95 and method; change AC-089 to `PASS`; retain AC-086 as the remaining acceptance-report waiver.

- [x] **Step 6: Regenerate the evidence chain and run the complete gate**

  Regenerate SBOM/release manifest/checksums, run format, strict Clippy, all workspace tests, locked release build, schema/spec/contract/integration/benchmark/release/CLI gates, dependency policy, checksum replay, diff check and Git status.

- [x] **Step 7: Commit the completed qualification gate**

  Commit message: `perf: qualify peak RSS budget`

### Task 2: Hosted cross-platform qualification

**Files:**
- Evidence-only update after an external run: `release/acceptance-report.json`
- Regenerate after evidence change: `release/release-manifest.json`
- Regenerate after evidence change: `release/checksums.sha256`
- Modify after evidence change: `BITACORA_IMPLEMENTACION.md`

**Interfaces:**
- Consumes: immutable hosted run URLs/IDs for the exact candidate SHA, with successful Windows, Linux and macOS jobs covering the workflow's specified gates.
- Produces: AC-086 `PASS` with run identifiers; zero acceptance-report waivers only if every hosted requirement is observed. Other declared release gates remain authoritative for General Availability.

- [x] **Step 1: Establish a verifiable remote**

  Configure or identify the repository remote and confirm the target repository/branch without creating one implicitly.

- [x] **Step 1a: Diagnose and remediate the first hosted execution**

  Preserve failed run `33030810589`; reproduce the Rust 1.98 Clippy failure, align hosted Cargo with Rust 1.97.1, hydrate all locked target dependencies before offline release generation, and verify the focused regressions locally before retrying.

- [x] **Step 2: Execute the hosted matrix for the exact qualification commit**

  Run the existing GitHub Actions matrix through an authorized push or pull request and capture the exact commit SHA plus run/job URLs.

- [x] **Step 3: Verify the evidence before changing status**

  Confirm every required Windows/Linux/macOS job concludes successfully at the same SHA; any missing, skipped or failed job leaves AC-086 `WAIVED`.

- [x] **Step 4: Close AC-086, regenerate and commit**

  Replace the waiver with immutable CI evidence, regenerate the evidence chain, rerun the full release gate and commit the hosted qualification closure. Do not change any integration certification flag.

## Completion Evidence Required

- Qualification progress is reported independently as 0/2, 1/2 or 2/2 resolved AC-086/AC-089 waivers.
- A local PASS for AC-089 includes a measured p95, declared Windows/Rust/build/sample environment and a method that distinguishes sampled RSS from a kernel-exact lifetime maximum.
- A PASS for AC-086 includes externally resolvable hosted run evidence for the exact candidate commit, not merely YAML configuration.
- General Availability is not declared while either waiver remains or while another release gate fails.

## Task 1 Closure Evidence

- RED: both AC-089 command tests failed against the accepted 8-metric/1-waiver report; the aggregate release gate also rejected the new report until its contract was updated.
- GREEN: `memory_one_shot_p95=17.855 MiB` over 20 Windows x86_64/Rust 1.97.1 release samples, measured on 2026-08-26; nine metrics pass and benchmark waivers are empty.
- Aggregate: 90 AC, 32 PL, zero missing evidence, zero blockers, zero declared-corpus secrets and one remaining waiver (AC-086).
- Isolation: `sysinfo` is an `xtask`-only dependency; the locked `project-ledger-cli` dependency tree excludes it.
- Full gate: format, strict Clippy, 222 Rust test attributes, workspace tests, locked release build, schema/spec/contracts/integrations, benchmark/release reproducibility, dependency policy and 13 checksum replay pass.
- Status: implementation remains 12/12 macro phases and 15/15 slices; release qualification advances to 1/2 resolved AC-086/AC-089 waivers; compatibility certification remains false for every profile.

## Task 2 Closure Evidence

- Hosted run `33031982562` is `success` at exact SHA `5d34294c1f06b59e635c84d734b3156950940ae5`; Windows job `98386337267`, Ubuntu job `98386337436`, macOS job `98386337365` and dependency-policy job `98386337495` all succeeded.
- RED: release-library and CLI tests both observed `waived_count=1` before the acceptance report changed.
- GREEN: both tests report `waived_count=0`; AC-086 carries immutable hosted run/SHA/job locators and no waiver.
- Aggregate: 90 AC, 32 PL, zero missing evidence, zero blockers, zero declared-corpus secrets and zero acceptance-report waivers.
- Status: qualification is 2/2, while release status remains `ENGINEERING_RELEASE_CANDIDATE` because `physical-disk-full` is still a declared chaos waiver blocking `GENERAL_AVAILABILITY`; all four compatibility certifications remain false.
