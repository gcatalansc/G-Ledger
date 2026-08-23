# Project Ledger Phase 9 / S12 Project Radar Artifact Profile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user authorized the next phase and did not request delegation.

**Goal:** Complete the Ledger-owned S12 articulation with Project Radar through the exact `project-ledger-radar-artifact/0.1.1` profile, deterministic read-only exports, explicit identity binding, conservative status/progress mapping and a storage-independent reference importer.

**Architecture:** A new pure `interop-radar` crate owns the locked profile manifests, contract replay, projection/export builder and reference merge evaluator. It consumes validated Ledger projections and returns JSON artifacts or import decisions; it has no SQLite, filesystem discovery, network, browser or Radar IndexedDB access. The CLI remains the filesystem/SQLite composition root and can export one explicit project or an explicitly supplied catalog. No S12 operation writes Ledger or Radar state.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, RFC 8785/SHA-256, JSON Schema Draft 2020-12, existing projection engine/project catalog/SQLite read path and canonical local artifact exchange.

## Global Constraints

- Project Ledger v0.2.0 revision `GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT` remains implementation authority.
- The profile is exactly `project-ledger-radar-artifact/0.1.1`; source Project Radar spec SHA-256 is `277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b`.
- Direction is Ledger to Radar only, through local artifact exchange. Ledger never opens or writes Radar IndexedDB and does not implement Radar product code.
- `LedgerRadarProjectProjection` contains no `radar_project_ref`. A confirmed `RadarProjectLink` is consumer-owned input to apply; names and aliases can only produce candidates.
- Omission means `NOT_PROVIDED`, never clear/delete/archive. Exports contain no tombstones.
- Radar owns priority, tags, notes, ideas, chats, tasks, artifacts and presentation metadata; reference import must preserve them byte-for-byte.
- Same project/version/digest is `DUPLICATE_REPLAY`; lower version is `STALE_IGNORED`; higher version is `APPLY_OVERLAY_UPDATE`; same version/different digest is `SOURCE_DIVERGENCE` and never last-write-wins.
- Status mapping follows the normative rules only. Progress percentage is unavailable unless the required-phase denominator is stable, linear and versioned.
- Export privacy is minimal: no event log, evidence payloads, repository paths, secrets or free-form Radar-owned fields.
- Local S12 conformance may close Fase 9 while `counterpart_extension_status=PENDING` and `certified_compatibility=false` remain explicit. No bilateral E2E or runtime certification is claimed.

---

### Task 1: Exact local profile registry and fixture replay

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/interop-radar/Cargo.toml`
- Create: `crates/interop-radar/src/lib.rs`
- Create: `crates/interop-radar/src/contracts.rs`
- Create: `crates/interop-radar/tests/profile_contracts.rs`
- Create: `integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1/schema-registry.json`
- Create: `integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1/fixture-manifest.json`
- Create: `integrations/project-radar/profile/project-ledger-radar-artifact-0.1.1/fixtures/**`
- Modify: `integrations/project-radar/upstream-lock.json`

**Interfaces:**
- Produces `RADAR_PROFILE`, `RADAR_SOURCE_SPEC_SHA256`, `RadarContractCatalog::load(root)`, `RadarContractCatalog::validate(contract, value)` and `verify_profile_contracts(root)`.
- Verification reports exact schema and valid/invalid fixture counts and checks every raw digest before schema evaluation.

- [x] **Step 1: Write the failing profile-lock/replay test**

Assert exact profile/source digest, registry paths/IDs/digests, fixture manifest digests, offline reference resolution, all valid fixtures accepted and all negative fixtures rejected. Add a one-byte schema drift regression.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-radar --test profile_contracts
```

Expected: the workspace package and contract catalog do not exist.

- [x] **Step 3: Implement the minimal exact catalog**

Use the canonical generated Radar schemas under `schemas/v0.2/radar`, publish a deterministic registry/fixture manifest, verify raw SHA-256 first, and compile all schemas with offline references only.

- [x] **Step 4: Run GREEN and independent digest checks**

Require exact counts and independently hash every registry/fixture path. Keep the counterpart status and certification false.

### Task 2: Deterministic Ledger projection and sync envelope

**Files:**
- Create: `crates/interop-radar/src/export.rs`
- Create: `crates/interop-radar/tests/export.rs`
- Modify: `crates/interop-radar/src/lib.rs`
- Modify only if contract gaps are proven: `crates/domain/src/integration.rs`

**Interfaces:**
- Produces an explicit `RadarProjectionRequest`, `build_project_projection(state, request)` and `build_sync_envelope(projections, request)`.
- Projection derives identity, lifecycle summary, conservative status/progress, attention signals, freshness, privacy and provenance from validated Ledger state.

- [x] **Step 1: Write failing mapping/export tests**

Cover every normative lifecycle/phase status rule, stable and unavailable progress, all attention signals, no invented `NOT_IMPLEMENTED`, stable ordering, project/version/digest identity, canonical self-digests and repeat-byte determinism.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-radar --test export
```

Expected: projection and envelope builders do not exist.

- [x] **Step 3: Implement pure derivation**

Validate state integrity first; derive only allowlisted facts. Treat missing or ambiguous information as unavailable, preserve source version/digest/provenance and emit no Radar identity or owned metadata.

- [x] **Step 4: Run GREEN plus privacy/determinism review**

Mutating phase/gate/conflict/freshness inputs must change the appropriate output/digest. Assert event payloads, evidence bodies, repository paths and Radar-owned fields are absent.

### Task 3: Storage-independent reference importer and conformance fixtures

**Files:**
- Create: `crates/interop-radar/src/import.rs`
- Create: `crates/interop-radar/tests/reference_importer.rs`
- Create: `integrations/project-radar/reports/radar-field-mapping.json`
- Create: `integrations/project-radar/reports/radar-loss-report.json`
- Create: `integrations/project-radar/reports/radar-conformance.json`

**Interfaces:**
- Produces `RadarImportSnapshot`, `RadarImportEvaluation`, `RadarMergeDecision` and `evaluate_import(snapshot, envelope, link)`.
- Evaluation is a preview/result model only; it has no persistence port and cannot mutate either product.

- [x] **Step 1: Write failing identity/merge/ownership tests**

Assert confirmed strong links are required to apply; aliases/name yield candidates only. Cover duplicate, stale, higher-version overlay, divergence, omitted-project `NO_ACTION`, omitted-field `NO_CLEAR`, owned-field preservation and transitive Ledger to Radar to G-Skills provenance.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-radar --test reference_importer
```

Expected: import evaluation types and function do not exist.

- [x] **Step 3: Implement the minimal pure evaluator**

Return explicit decisions, conflicts and a preview overlay. Reject revoked/conflicted/candidate links for apply, reject name-only mappings and never construct delete/archive operations.

- [x] **Step 4: Run GREEN and publish honest reports**

Reports must record semantic degradation and authority elevation, distinguish consumer-owned state, and leave counterpart runtime/E2E pending.

### Task 4: Read-only CLI export, compatibility evidence, documentation and phase commit

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/radar_workflow.rs`
- Modify: `specs/compatibility/project-radar-v0.1.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

**Interfaces:**
- Adds `integration radar verify-contracts`, `radar export --db --project --output`, `radar validate-export --input` and `catalog radar-export --all ...` or the closest existing CLI-compatible explicit-known-project shape.
- Every export validates the real SQLite chain/projection, writes canonical JSON with no-clobber semantics and appends zero events.

- [x] **Step 1: Write failing real-SQLite CLI tests**

Assert verify/export/validate/catalog behavior, deterministic bytes, explicit scope, unknown project rejection, no overwrite and event count unchanged before/after every operation.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-cli --test radar_workflow
```

Expected: Radar CLI commands do not exist.

- [x] **Step 3: Implement composition and reconcile locks/docs**

Keep storage and file I/O in CLI. Update profile/report hashes, local mapping/conformance statuses and coverage without changing `counterpart_extension_status` or `certified_compatibility=false`.

- [x] **Step 4: Execute the complete gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo run -p project-ledger-cli -- integration gskills verify-contracts
cargo run -p project-ledger-cli -- integration gsp verify-contracts
cargo run -p project-ledger-cli -- integration radar verify-contracts
cargo run -p project-ledger-cli -- integration compatibility
cargo deny --offline check
git diff --check
git status --short
```

Expected: all commands exit 0; PL-21 through PL-32 and AC-065 through AC-080 have local evidence; the Radar lock is locally complete but uncertified and the working tree contains only S12 changes.

- [x] **Step 5: Close the persistent records and commit the phase**

Record fresh evidence and limitations in the bitácora, mark this plan complete, verify all lock/report hashes from tracked bytes, and create one phase commit. Do not begin S13.

## Completion Evidence Required

- Profile: exact schema/fixture registry replay is offline, raw-digest locked and adversarial drift is rejected.
- Export: same validated Ledger state plus request produces byte-identical projection/envelope and zero events.
- Identity: no name-only or alias-only automatic mapping; apply requires an explicit confirmed strong link.
- Merge: duplicate/stale/update/divergence/omission outcomes match the normative matrix without last-write-wins or deletes.
- Ownership: Radar-only fields survive reference import unchanged; Ledger-only fields never silently acquire Radar authority.
- Provenance: Radar projections retain Ledger project/version/digest/provenance, including when surfaced transitively by G-Skills.
- Boundary: no Radar IndexedDB, product code, shared DB, network or bilateral E2E claim exists in S12.
- Gate: full workspace/spec/schema/contract/dependency/diff checks pass from the final tree immediately before commit.

## Closure

- Closed on 2026-08-23 after the complete gate passed from the reconciled tree.
- Inventory: 199 Rust `#[test]` attributes, with zero failures in the workspace gate.
- Locked bytes: schema registry, fixture manifest and all three Radar reports match the SHA-256 values in `integrations/project-radar/upstream-lock.json`.
- Result: Fase 9 / S12 is complete locally; progress is 10/12 macro phases and 13/15 implementation slices.
- Evidence boundary: `counterpart_extension_status=PENDING`, `bilateral_e2e_status=PENDING_SEPARATE_EVIDENCE` and `certified_compatibility=false` remain unchanged.
- Phase commit: `feat: add Project Radar artifact profile`.
- Next authorized-by-spec slice, not started here: Fase 10 / S13 Agent Skill/Codex.
