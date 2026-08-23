# Project Ledger Phase 6 (S9) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user authorized the next step and did not request delegation.

**Goal:** Complete S9 with a storage-independent, read-only Project Ledger provider for the exact `gskills-context-provider/0.3.0` profile, including upstream locks, field mapping, budgets, freshness, strict stdio dispatch and provider conformance.

**Architecture:** A new `interop-gskills` crate owns exact G-Skills read-contract types and maps an immutable `ProjectStateProjection`; it has no SQLite dependency and cannot mutate canonical state. The CLI composition root may load Ledger-owned storage, then inject an immutable projection. S9 schemas and fixtures are vendored and verified against their upstream canonical digests; the complete upstream registry and fixture-manifest files remain pinned in the integration lock.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, existing RFC 8785/SHA-256 primitives, `serde`/`serde_json`, `jsonschema` in contract tests and the S8 `transport-stdio` framing. No network, async runtime, AI, shared database, unsafe code or mutable adapter surface.

## Global Constraints

- Project Ledger spec v0.2.0 remains implementation authority; G-Skills is an external contract input and has no Ledger lifecycle authority.
- Pin G-Skills revision `PL-UPSTREAM-REFRESH-1`, spec SHA-256 `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`, registry file SHA-256 `de9131a8e9940ccfae4cbb505edc91980e14db106b3773dc84845cc0068028aa` and fixture-manifest file SHA-256 `316c056480f3ab613f937426946da08276e9673ec6781d86f5a8b1e1e5e614ee`.
- Pin counterpart lock SHA-256 `49ff8bf60b33deb47cc688f288f7c2e1124d99fb223d0d8d4d2668ea4c60929d`; it fixes Ledger spec SHA-256 `31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9`.
- S9 implements only `ProviderManifest`, `ProviderHealthReport`, `ContextProjectionRequest`, `ContextProjectionEnvelope`, `ProviderFreshnessRecord` and `ProjectContinuityProjection`.
- External records, submit/status/reconcile, write trust and gate-revalidation writes remain S10. Record submit eligibility stays `DENY`; certified compatibility stays false.
- The adapter has no `storage-sqlite`, WAL, private migration or shared-product-database dependency.
- Projection, manifest and health have zero semantic side effects. Ambiguous or mismatched project/repository identity fails closed.
- State/material versions, gate buckets, action classification, checkpoint N/N+1, repository binding, unknowns/conflicts and provenance remain distinct.
- Omissions require explicit coverage/loss reporting; material loss prevents the S9 gate from passing.
- Output enforces non-zero bounded items/tokens/bytes/timeout after canonical serialization.
- Authority claims are advisory only. Repository-domain authority and mutable JSON-RPC methods remain absent.
- Stdio uses `gskills-stdio-jsonrpc-content-length/1.0.0` and only `ledger.manifest`, `ledger.health`, `ledger.project_continuity`.
- Provider conformance proves the S9 read profile only, not S10, bilateral certification, production operation or scientific validity.

---

### Task 1: Exact upstream contract lock and vendored read schemas

**Files:**
- Modify: `integrations/gskills/upstream-lock.json`
- Create: `integrations/gskills/contracts/v0.3/*.schema.json`
- Create: `integrations/gskills/contracts/v0.3/fixtures/{positive,negative}/*.json`
- Create: `integrations/gskills/profile-contract-lock.json`
- Create: `crates/interop-gskills/Cargo.toml`
- Create: `crates/interop-gskills/src/contracts.rs`
- Test: `crates/interop-gskills/tests/upstream_contracts.rs`

**Interfaces:**
- Produces exact typed read contracts and `verify_vendored_contracts(root) -> ContractVerificationReport`.
- Produces a profile lock binding upstream artifacts, selected schema canonical digests and fixture paths.

- [x] **Step 1: Write failing lock/schema/fixture tests**

Assert current source/counterpart digests, exact protocol/range/transport, closed outer objects, positive fixture acceptance, negative fixture rejection and schema digest equality with upstream registry entries.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gskills --test upstream_contracts
```

Expected: the crate, artifacts and verification API do not exist.

- [x] **Step 3: Implement the minimal exact contract boundary**

Vendor only the six S9 schemas and paired fixtures. Keep the complete registry/manifest hashes and counterpart lock exact; do not redefine reduced public schemas.

- [x] **Step 4: Run GREEN and verify digests independently**

Require every positive fixture valid, every negative fixture invalid and all schema IDs/digests exact.

### Task 2: Continuity mapping, budgets, freshness and loss report

**Files:**
- Create: `crates/interop-gskills/src/provider.rs`
- Create: `crates/interop-gskills/src/mapping.rs`
- Create: `crates/interop-gskills/src/lib.rs`
- Test: `crates/interop-gskills/tests/provider.rs`
- Create: `integrations/gskills/reports/provider-field-mapping.json`
- Create: `integrations/gskills/reports/provider-loss-report.json`

**Interfaces:**
- Produces `GSkillsProvider::new(ProjectStateProjection, ProviderContext)`, `manifest`, `health`, `project` and `observable_state`.
- Produces deterministic continuity, freshness and envelope objects bound to the request/state.

- [ ] **Step 1: Write failing mapping tests**

Use a real event-built projection. Assert literal state/material versions, phases, gate buckets, decision/blocker/risk/action refs and classification, checkpoint N/N+1 fields, repository snapshot, unknowns/conflicts, recent progress and provenance.

- [ ] **Step 2: Verify RED**

Expected: mapping/provider APIs are absent.

- [ ] **Step 3: Implement minimal deterministic mapping**

Resolve exact identity and at most one current binding. Enforce requested fields/privacy/budgets, label state mismatch stale, preserve repository freshness separately and emit zero-material-loss evidence for the normative full request.

- [ ] **Step 4: Run GREEN and mutation review**

Verify that collapsing versions, moving a gate bucket, changing classification, simplifying checkpoint fields, accepting ambiguity or hiding an omission breaks a test.

### Task 3: G-Skills conformance and stdio profile E2E

**Files:**
- Create: `crates/interop-gskills/src/conformance.rs`
- Create: `crates/interop-gskills/src/stdio.rs`
- Test: `crates/interop-gskills/tests/conformance.rs`
- Test: `crates/interop-gskills/tests/stdio_e2e.rs`
- Create: `integrations/gskills/reports/provider-conformance.json`

**Interfaces:**
- Produces `run_gskills_read_conformance` and `GSkillsJsonRpcHandler` over S8 `dispatch_frame`.

- [ ] **Step 1: Write failing conformance/E2E tests**

Cover repeated current, stale state/repository, ambiguous identity, authority escalation, unsupported fields, invalid budgets and forced failure. Assert zero mutation, deterministic canonical output, recovery and exact schema validation. Compare framed results with direct calls.

- [ ] **Step 2: Verify RED**

Expected: exact-profile harness and handler are absent.

- [ ] **Step 3: Implement minimal harness and handler**

Use stable checks, typed JSON-RPC errors, output bounds and failure isolation. Register no mutable method.

- [ ] **Step 4: Run GREEN twice**

Require byte-stable results, no state mutation and recovery after rejected/failed calls.

### Task 4: CLI composition, documentation, gate and phase commit

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/contract_commands.rs`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `specs/compatibility/gskills-v0.3.0.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

**Interfaces:**
- Produces `project-ledger integration gskills verify-contracts` and `serve provider --stdio --profile gskills-context-provider/0.3.0`.

- [ ] **Step 1: Write failing CLI tests**

Assert contract verification output, rejection of version aliases and one real framed projection with no mutable operation available.

- [ ] **Step 2: Implement minimal composition and docs**

CLI owns Ledger storage lookup and injects an immutable projection; document exact source/digests, provider PASS scope and remaining S10/bilateral blockers.

- [ ] **Step 3: Execute the complete gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo run -p project-ledger-cli -- integration gskills verify-contracts
.tooling/bin/cargo-deny --offline check
git diff --check
```

Expected: every command exits 0; S9 fixtures/conformance/E2E pass, mutations/material losses are zero and certified compatibility remains false.

- [ ] **Step 4: Update persistent evidence and commit**

Set progress to `7/12` macrofases and `10/15` slices only after the gate passes. Commit `feat: add exact G-Skills continuity provider`.

## Self-review

- Coverage: REQ-INT-001/002/003, §34, AC-042 through AC-045, AC-050/051, PL-10/16/17, Fase 6 and S9 map to Tasks 1-3.
- Exactness: schemas are verified by upstream canonical digest; full registry/fixture manifest and reciprocal lock are pinned separately.
- Mapping: every normative minimum field has a literal test and report row; full projection has zero material loss.
- Authority/writes: effective repository authority is empty, mutable methods are absent and RecordSink remains denied.
- Independence: adapter has no storage dependency; the golden path works with G-Skills disabled.
- Honesty: S9 provider/profile E2E can pass while overall compatibility remains uncertified pending S10 and bilateral E2E.

## Execution finding — upstream schema blocker

Task 1 discovered an executable incompatibility before provider production began. The locked `ProjectContinuityProjection` schema closes `active_phases[]` and `next_actions.ordered[]` to `ref` plus `extensions`, while the normative G-Skills §28.11 mapping requires phase `title/kind/status/started_at` and action `title/classification/status/authority_ref/gskills_derivation` at their canonical paths. The characterization test `pl_10_upstream_schema_currently_rejects_normative_phase_and_action_fields` proves both paths are rejected as additional properties.

Under Project Ledger §34.3/34.4, REQ-PVD-008 and PL-10, dropping those fields, moving them into an unversioned extension or rewriting the vendored schema would be a material-loss/contract overclaim. Tasks 2-4 therefore remain unchecked. Resume only after an authorized G-Skills contract revision publishes corrected schema, positive/negative fixtures and refreshed registry/manifest digests.
