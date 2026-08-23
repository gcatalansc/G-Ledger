# Project Ledger Phase 11 / S14 Hardening and Release Plan

**Goal:** Close the implementation roadmap with a reproducible, fail-closed engineering release gate for AC-081 through AC-090, PL-01 through PL-32, privacy/security, chaos, performance, packaging, SBOM, migrations and compatibility reporting.

**Architecture:** A dependency-light `project-ledger-release` crate validates declared privacy surfaces and a hash-locked release evidence set. `xtask` generates deterministic SBOM/checksum material and runs the local benchmark; the CLI exposes read-only release verification. Existing behavioral tests remain the evidence source for previously implemented AC/PL rows, while S14 adds adversarial tests only for new release/security behavior.

**Tech stack:** Rust 1.97.1, serde/serde_json, SHA-256, SQLite, GitHub Actions, SPDX JSON, deterministic JSON reports.

## Global constraints

- The normative authority is `referencias/PROJECT_LEDGER_v0.2.0_CODEX/PROJECT_LEDGER_SPEC_v0.2.0.md`, especially sections 41-49, 50-51 and 56.
- Network, telemetry, raw terminal output, transcripts and private reasoning remain off by default.
- No repository-local file may enable adapters, writes, authority, credentials, hooks or network.
- Timing results are engineering measurements tied to a declared environment, not universal guarantees.
- Missing cross-platform or counterpart execution is an explicit non-critical waiver or release blocker; it is never converted into PASS.
- `certified_compatibility=false` remains mandatory until bilateral/live evidence exists.
- One phase commit closes S14; no post-S14 feature work is included.

---

### Task 1: Release contract, privacy scanner and trust policy (TDD)

**Files:**
- Create: `crates/release/Cargo.toml`
- Create: `crates/release/src/lib.rs`
- Create: `crates/release/src/privacy.rs`
- Create: `crates/release/src/policy.rs`
- Create: `crates/release/src/report.rs`
- Create: `crates/release/tests/hardening.rs`
- Modify: `Cargo.toml`

- [x] Write failing AC-081/082/083/085 tests for untrusted capabilities, exact argv/no-shell policy, secret/transcript detection, traversal rejection and redacted values.
- [x] Observe RED before production APIs exist.
- [x] Implement the minimal pure policy and scanner, then obtain GREEN.

### Task 2: Acceptance, migration and release evidence gate (TDD)

**Files:**
- Create: `crates/release/tests/release_gate.rs`
- Create: `release/release-manifest.json`
- Create: `release/acceptance-report.json`
- Create: `release/chaos-report.json`
- Create: `release/migrations/v0.1-to-v0.2.json`
- Create: `release/privacy/corpus-manifest.json`
- Create: `release/privacy/surfaces/*.json`
- Create: `docs/THREAT_MODEL.md`

- [x] Write failing tests for exact AC-001..090 and PL-01..32 coverage, referenced evidence existence, explicit reversible migration strategy, non-critical waiver shape, compatibility honesty and raw-digest drift.
- [x] Implement report validation and publish the closed evidence set.
- [x] Mutate one digest/ID/waiver/certification field and confirm fail-closed behavior.

### Task 3: Deterministic SBOM, checksums, packaging and local benchmark

**Files:**
- Modify: `xtask/Cargo.toml`
- Modify: `xtask/src/main.rs`
- Create: `xtask/tests/release_command.rs`
- Create: `release/package-manifest.json`
- Generate: `release/sbom.spdx.json`
- Generate: `release/checksums.sha256`
- Generate: `benchmark/project-ledger-v0.2.0-windows-x86_64.json`

- [x] Write failing command tests for deterministic SBOM/checksums, non-clobber packaging metadata and benchmark report validation.
- [x] Add `cargo xtask release generate|check` and `cargo xtask benchmark run` with exact argv and offline Cargo metadata.
- [x] Build the single-binary release profile, measure p95 budgets on the current declared Windows environment and record honest pass/fail outcomes.

### Task 4: CLI, CI and release documentation

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/release.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `SECURITY.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`

- [x] Add a read-only `project-ledger release verify` command and test success plus adversarial drift.
- [x] Make CI run the release gate on Windows/Linux/macOS while retaining dependency policy.
- [x] Document threat boundaries, benchmark environment, packaging, migration/rollback and certification limits.

### Task 5: Full gate, persistent log and phase commit

**Files:**
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

- [x] Run format, strict Clippy, all workspace tests, schema/spec/contracts, all integration contract gates, release check, dependency policy, diff and Git status.
- [x] Recompute locked bytes only after all reports are final and repeat the complete gate from a cleanly reconciled tree.
- [x] Record exact PASS/FAIL/WAIVED evidence and remaining release conditions.
- [x] Commit exactly once as `feat: harden and prepare v0.2.0 release`.

## Completion evidence required

- AC-081..090 have executable or report-backed evidence without overclaim.
- PL-01..32 are each materialized and point to an existing behavioral test/report.
- Secret persistence count is zero on the declared, hash-locked surface corpus.
- Security defaults, limits, threat model, chaos coverage, migration/rollback, SBOM, checksums, package manifest, benchmark and compatibility report are published.
- Local implementation progress reaches 12/12 macro phases and 15/15 slices only after the gate and phase commit.
- Project completion/release certification remains distinct from implementation completion when counterpart or cross-platform evidence is still waived/pending.

## Closure evidence

- Fresh full gate on 2026-08-23: format, strict Clippy, 221 Rust `#[test]` attributes, locked release build, schemas/spec/contracts, all four integration profiles, benchmark/release gates, dependency policy, 13 checksum replay and diff check passed.
- Acceptance aggregation: 90 AC, 32 PL, zero missing evidence, zero blockers, zero declared-corpus secrets and two explicit non-critical waivers.
- Benchmark: eight measured targets pass; peak-RSS remains waived and blocks General Availability.
- Cross-platform: CI matrix is configured; absent hosted Linux/macOS run evidence remains waived and blocks General Availability.
- Compatibility: four local profiles are complete, all `certified_compatibility=false`; counterpart/live evidence remains pending.
- Implementation roadmap result: 12/12 macro phases and 15/15 slices implemented locally as an engineering release candidate, distinct from GA/certified compatibility.
