# Project Ledger Phase 8 / S11 GSP Exact-Contract Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user already authorized the next phase and did not request delegation.

**Goal:** Complete the Ledger-owned S11 bridge for GSP-Gemma Compiler v0.2 using the immutable `gsp-exact-2026.04.12` contracts, a minimized explicitly scoped corpus, exact runtime bundle validation, visible contradiction/uncertainty, explicit result linking and no lifecycle authority elevation.

**Architecture:** A new storage-independent `interop-gsp` Rust crate owns upstream byte locks, offline schema/fixture replay, corpus minimization, exact `EvidenceBundle` generation, `CompilationResult` validation and a sidecar port. The CLI is the only SQLite composition root and exposes explicit artifact-exchange commands; GSP remains optional and out of process. The adapter never converts a GSP claim into lifecycle state: a validated compilation can only prepare or execute the existing `LINK_EXTERNAL_EVIDENCE` command through the canonical command processor.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, RFC 8785/SHA-256, JSON Schema Draft 2020-12, exact YAML fixture replay, existing SQLite/event-store/application path and canonical local artifact exchange. No network in tests, no Python dependency in the core, no shared database, no shell interpolation, no reduced GSP schemas and no hidden write.

## Global Constraints

- Project Ledger v0.2.0 revision `GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT` remains implementation authority.
- Target GSP implementation is tag `v0.2.0`, commit `7a43559c02417641691dbb5f2a6e79c13d74d1a1`; its technical spec SHA-256 is `c838bdd8eee3fb6648db5e7dd757d08d8ced81d35be8b9bf8001e321f49ffea1`.
- Canonical contracts come from `gsp-spec` tag `gsp-exact-2026.04.12`, commit `004b14d7b64837025671970eac2469f092c49475`.
- Pin archive SHA-256 `59395d1ecd0c573451f2fcdaa6cf04ce70fb1f4fe129ba1a3d2ec2b754ab074b`, `HASH_MANIFEST.csv` SHA-256 `c6ca0f6df33bda69e7e73eb201058884a3111bce506c9328aca5c421836e2e8c` and schema registry SHA-256 `523faabcc2e120a847849b582c9dbb97d758a6c34f78b96e153b7b67a622d7b3`.
- Preserve all 31 upstream JSON schema artifacts and all 140 contractual YAML fixtures byte-for-byte; the registry exposes 28 named contracts.
- Do not redefine `EvidenceBundle`, `CandidateBundle`, `CompilationBundle`, `ValidationReport`, `UncertaintyMap` or `TaskProjectionBundle`.
- Corpus export requires explicit stable references and hard item/byte budgets. It never exports the full event timeline, raw evidence output, repository paths, transcripts, secrets or private reasoning.
- Every selected source retains its Ledger status, digest and provenance. Legacy, stale, contradiction and uncertainty signals are preserved, never silently normalized away.
- `GspCompilerSidecar` consumes an exact `EvidenceBundle` and returns the GSP v0.2 `CompilationResult` shape. Sidecar failure changes no Ledger state and the golden path works with GSP disabled.
- The only Ledger mutation derived from a result is an explicit `LINK_EXTERNAL_EVIDENCE` command with expected state version and idempotency key. GSP cannot complete/reopen phases, verify/waive gates, change project status or acquire policy/code-guard authority.
- Local S11 conformance may close Fase 8 when semantic degradation and authority elevation are zero. `certified_compatibility` remains false unless an external live-sidecar E2E is separately evidenced.

---

### Task 1: Exact upstream lock, all schemas and full fixture replay

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/interop-gsp/Cargo.toml`
- Create: `crates/interop-gsp/src/lib.rs`
- Create: `crates/interop-gsp/src/contracts.rs`
- Create: `crates/interop-gsp/tests/upstream_contracts.rs`
- Create: `integrations/gsp/contracts/gsp-exact-2026.04.12/schemas/**`
- Create: `integrations/gsp/contracts/gsp-exact-2026.04.12/fixtures/contracts/**`
- Create: `integrations/gsp/contracts/gsp-exact-2026.04.12/HASH_MANIFEST.csv`
- Modify: `integrations/gsp/upstream-lock.json`

**Interfaces:**
- Produces `GSP_CONTRACT_TAG`, `GSP_IMPLEMENTATION_VERSION`, `GSP_SCHEMA_REGISTRY_SHA256`, `GspContractCatalog::load_vendored(root)`, `GspContractCatalog::validate(contract, value)` and `verify_vendored_contracts(root)`.
- `GspContractVerificationReport` reports 28 registry contracts, 31 JSON schema artifacts, 56 valid fixtures and 84 invalid fixtures.

- [x] **Step 1: Write the failing exact-lock test**

Assert literal upstream tag/commit/digests and counts. Verify every vendored schema/fixture raw SHA-256 against the corresponding `HASH_MANIFEST.csv` row, registry paths/IDs exactly, registry validity against `schema-registry.schema.json`, all schemas offline-resolvable, and all 140 YAML fixtures produce their filename-declared outcome.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gsp --test upstream_contracts
```

Expected: the workspace package and `GspContractCatalog` do not exist.

- [x] **Step 3: Implement the exact catalog and fixture replay**

Add the crate and YAML parser, import only the tracked bytes from clean `gsp-spec` commit `004b14d`, build an in-memory offline JSON Schema registry that resolves physical filename and `schema://contracts/*` aliases, and reject any manifest row, schema ID/path or fixture outcome drift.

- [x] **Step 4: Run GREEN and independent raw-hash checks**

Require 28/31/56/84 exact counts and independently compare the vendored manifest, registry and every imported byte with the clean upstream checkout.

### Task 2: Explicit minimized corpus and exact EvidenceBundle

**Files:**
- Create: `crates/interop-gsp/src/corpus.rs`
- Create: `crates/interop-gsp/tests/corpus.rs`
- Modify: `crates/interop-gsp/src/lib.rs`
- Create: `integrations/gsp/schemas/ledger-gsp-corpus-manifest.schema.json`
- Create: `integrations/gsp/reports/gsp-field-mapping.json`
- Create: `integrations/gsp/reports/gsp-loss-report.json`

**Interfaces:**
- Produces `GspCorpusRequest`, `GspCorpusBudget`, `GspCorpus`, `GspCorpusManifest`, `GspCorpusEntry` and `build_minimized_corpus(projection, stream, request)`.
- `GspCorpusRequest` carries a non-empty query plus explicit authority, decision, gate, checkpoint and selected-material-event references. It has no discovery-all mode.
- `GspCorpus::evidence_bundle()` returns the complete upstream `EvidenceBundle` as JSON and validates it through `GspContractCatalog`.

- [x] **Step 1: Write failing corpus behavior tests**

Build a real Ledger projection and hand-select references. Assert deterministic canonical output, exact GSP validation, stable UUID evidence IDs, preserved status/digest/provenance, directory classes `authority/decisions/gates/checkpoints/selected-material-events/sidecars`, query/state digests, and hard budget enforcement.

The same tests reject duplicate/unknown/cross-project references, empty scope, full-timeline requests, unallowlisted event types and oversize text. Assert that raw evidence output, workspace paths, transcripts and event payloads never appear.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gsp --test corpus
```

Expected: corpus request/export and exact EvidenceBundle APIs are absent.

- [x] **Step 3: Implement minimal deterministic mapping**

Serialize only explicitly selected canonical records into bounded JSON text blocks. Preserve Ledger classification/status/provenance as content; set GSP routing to the explicit Ledger scope and evidence sufficiency conservatively to `partial`. Selected event summaries contain only type, reference, version, timestamp and digests, never the raw event payload.

- [x] **Step 4: Run GREEN and mutation review**

Mutating a selected ref, source status, state digest, provenance, item order, budget, GSP required field or privacy exclusion must fail at least one behavior test. Publish mapping/loss rows without claiming semantic truth or live GSP execution.

### Task 3: Exact CompilationResult validation, sidecar isolation, conflict visibility and explicit link

**Files:**
- Create: `crates/interop-gsp/src/bridge.rs`
- Create: `crates/interop-gsp/src/result.rs`
- Create: `crates/interop-gsp/tests/bridge.rs`
- Create: `crates/interop-gsp/tests/conformance.rs`
- Modify: `crates/interop-gsp/src/lib.rs`
- Create: `integrations/gsp/schemas/ledger-gsp-compilation-receipt.schema.json`
- Create: `integrations/gsp/reports/gsp-conformance.json`

**Interfaces:**
- Produces `GspCompilerSidecar`, `GspSidecarError`, `GspBridge`, `GspCompilationResult`, `GspCompilationReceipt`, `GspArtifactReceipt`, `GspConflictVisibility` and `GspBridgeReport`.
- `GspBridge::compile(sidecar, corpus)` validates the exact input before calling the port, validates all six exact result artifacts afterward, checks cross-artifact request identity and returns no Ledger event or command.
- `GspCompilationReceipt::link_payload(uri, digest)` produces only `LinkExternalEvidencePayload` with fixed Ledger receipt schema reference.

- [x] **Step 1: Write failing result/conformance tests**

Use literal upstream valid fixtures to form a GSP v0.2 `CompilationResult`. Assert all six artifacts and `validation.contracts_checked` are exact; query/evidence/output digests and provenance survive in the receipt; contradictions, partial/fail validation and uncertainty become visible review signals rather than disappearing.

Reject missing/reduced artifacts, schema drift, request-ID mismatch, validation overclaim, digest mismatch and any attempt to map a GSP claim to phase/gate/status commands. A failing sidecar must leave a real Ledger event store unchanged.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gsp --test bridge --test conformance
```

Expected: bridge/result/sidecar types and exact result validation are absent.

- [x] **Step 3: Implement minimal validation and linking boundary**

Keep exact artifacts as unmodified JSON values validated by their upstream schemas. Compute canonical digests without projecting a reduced model. Derive only adapter metadata, counts and review flags. Link the canonical receipt through the existing command payload; do not create a GSP lifecycle command family.

- [x] **Step 4: Run GREEN twice**

Require deterministic receipts, zero semantic field loss, visible contradictions/uncertainty, zero authority elevation, zero events on sidecar failure and exactly one `EXTERNAL_EVIDENCE_LINKED` event when an explicit link command is later submitted through the native processor.

### Task 4: SQLite CLI composition, artifact workflow, documentation, full gate and phase commit

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/gsp_bridge.rs`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `specs/compatibility/gsp-v0.2.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `integrations/gsp/upstream-lock.json`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

**Interfaces:**
- Produces `project-ledger integration gsp verify-contracts`.
- Produces `project-ledger integration gsp export-corpus --database ... --project-ref ... --request ... --output ...` with an explicit request file and no write to Ledger.
- Produces `project-ledger integration gsp validate-result --evidence-bundle ... --result ... --output ...` for exact artifact validation and canonical receipt/report publication.
- Produces `project-ledger integration gsp link-result --database ... --project-ref ... --receipt ... --receipt-uri ... --expected-state-version ... --idempotency-key ... --actor-id ... --observed-at ...`; this is the sole explicit write and uses `CommandProcessor<SqliteEventStore, FixedClock>`.

- [x] **Step 1: Write failing real-CLI tests**

Create a real SQLite project with canonical decisions/gates/checkpoint, export a scoped corpus, validate a fixture-backed result, and explicitly link its receipt. Assert corpus/result commands do not alter event count, link appends one event, replay has one effect, and phase/gate/lifecycle state does not change.

Also assert missing scope, unknown refs, output overwrite, invalid/reduced result, stale expected version and missing explicit link parameters fail closed.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-cli --test gsp_bridge
```

Expected: GSP CLI subcommands do not exist.

- [x] **Step 3: Implement CLI and documentation**

Keep SQLite and filesystem composition in CLI. Publish corpus/results without overwrite, use canonical JSON receipts, update the GSP lock to local S11 PASS and keep `certified_compatibility=false` with live external-sidecar E2E explicitly pending.

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
cargo run -p project-ledger-cli -- integration compatibility
.tooling/bin/cargo-deny.exe --offline check
git diff --check
```

Expected: every command exits 0; the S11 report records semantic degradation 0 and authority elevation 0; GSP lock evidence is complete while runtime certification remains false.

- [x] **Step 5: Update persistent evidence and commit**

Set progress to `9/12` macrofases and `12/15` slices only after the full gate passes. Commit `feat: add exact GSP bridge` and verify a clean worktree.

## Self-review

- Spec coverage: Ledger §35, REQ-INT-001/002/003, AC-052…060, PL-12/13, ADR-011, R-10 and Fase 8 map to Tasks 1–4.
- Exactness: every public GSP schema and contractual fixture is preserved byte-for-byte; runtime artifacts remain full JSON values and are never represented by a reduced public schema.
- Corpus privacy: export is explicit, bounded and allowlisted; full history, raw evidence output and paths are absent.
- Authority: compilation can create only an external evidence link request; phase, gate, lifecycle, policy and code-guard authority remain Ledger-owned.
- Conflict semantics: contradictions, uncertainty, partial/fail validation and source conflicts are preserved as visible review signals.
- Isolation: the crate has no SQLite or Python production dependency; sidecar failure is data/error only and the standalone Ledger path remains operational.
- Honesty: local exact-contract conformance is not a scientific claim or a live GSP runtime certification.
