# Project Ledger Phase 5 (S8) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the slice commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user authorized the next step and did not request delegation.

**Goal:** Complete S8 with a deterministic generic integration protocol/SDK, bounded `Content-Length` stdio framing, safe local artifact exchange, exact identity and upstream-lock validation, and a conformance harness that proves observable read side effects zero and rejects malicious fixtures fail-closed.

**Architecture:** `integration-protocol` owns storage-independent builders, integrity validators, identity/lock policy and the read-only conformance harness over the frozen domain contracts. `transport-stdio` owns generic JSON-RPC 2.0 framing/dispatch without project or database knowledge. `artifact-exchange` owns canonical, size-bounded, no-overwrite local JSON exchange. Product adapters remain out-of-process consumers introduced only by S9-S12.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, existing RFC 8785/SHA-256 primitives, `serde`/`serde_json`, standard library I/O/time/path APIs and `tempfile` for atomic no-clobber persistence. No network, async runtime, database dependency, unsafe code or product-specific SDK is added.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; the preserved specs and frozen v0.2.0 schemas/fixtures are not edited.
- S8 implements §32-33 and §38.5 infrastructure only. G-Skills mapping/runtime, RecordSink, GSP, Radar mapping/importer and Agent Skill remain S9-S13.
- The golden path remains fully functional with every external integration, network, AI and daemon disabled.
- Generic crates never import SQLite/WAL/private migrations and never share a product database. Providers/adapters are out-of-process or artifact-based.
- Manifest projection/health side effects are `NONE`, RecordSink effects are only `DECLARED_ONLY`, and shared database is always false.
- Every manifest, health, projection and loss-report digest is RFC 8785/SHA-256 over the object with its own digest field omitted. Projection payload digests cover canonical payload bytes.
- Requests require exact protocol, exact project identity, bounded unique fields, a supported privacy profile and non-zero output/item budgets. Names or aliases never establish identity.
- Confirmed identity mappings are the only resolvable mappings; zero matches are unresolved and multiple distinct matches are ambiguous. Both disable canonical read/write.
- Upstream locks are complete only with exact upstream version, spec digest, schema-registry digest, fixture-manifest digest and compatibility range. Null/missing evidence remains `PENDING`; metadata never upgrades it to certified.
- Stdio accepts JSON-RPC 2.0 over one exact `Content-Length` header, bounded ASCII headers and bounded UTF-8 JSON. Duplicate/invalid lengths, extra/trailing bytes and oversized/malformed payloads fail closed.
- Artifact exchange accepts one safe `.json` filename under an explicit root, canonical JSON only, bounded bytes, no symlink and no overwrite. Writes publish atomically with no-clobber behavior.
- Conformance observes an adapter-defined canonical state tuple before and after every call. This proves only observable harness boundaries, not a universal sandbox against an equally privileged local process.
- Read conformance covers negotiation, manifest/health integrity, 100 deterministic projections, bounds, timeout measurement, exact identity, current/stale freshness, no authority escalation, redaction, malicious request rejection and recovery after failure.
- CLI adds only generic `integration manifest` and `integration compatibility` read surfaces. It does not advertise a product provider/sink runtime before the corresponding adapter slices.

---

### Task 1: Generic protocol builders, identity and upstream locks

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/integration-protocol/Cargo.toml`
- Create: `crates/integration-protocol/src/lib.rs`
- Create: `crates/integration-protocol/src/integrity.rs`
- Create: `crates/integration-protocol/src/identity.rs`
- Create: `crates/integration-protocol/src/upstream_lock.rs`
- Test: `crates/integration-protocol/tests/contracts.rs`

**Interfaces:**
- Produces: `build_manifest`, `validate_manifest_integrity`, `build_health`, `validate_health_integrity`, `build_projection_envelope`, `validate_projection_envelope`, `build_loss_report`, `validate_loss_report_integrity`.
- Produces: `IdentityResolver::resolve(&IdentityQuery, &[IdentityMapping]) -> IdentityResolution` and `inspect_upstream_lock(&serde_json::Value) -> UpstreamLockInspection`.

- [x] **Step 1: Write failing contract tests**

Cover default manifest bytes/digest and fixed side-effect/storage semantics; health digest/protocol binding; projection payload digest, budget, privacy, coverage/omissions and current/stale freshness; loss count/digest; explicit identity with unresolved/ambiguous fail-closed; complete/pending/tampered upstream locks including the three current repository locks.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-integration-protocol --test contracts
```

Expected: the crate and wished-for APIs do not exist.

- [x] **Step 3: Implement the minimal storage-independent protocol SDK**

Use typed drafts plus shared `digest_without` integrity logic. Validate fixed protocol/profile semantics, canonical payload bytes and privacy keys recursively. Lock inspection preserves incomplete evidence as explicit missing fields and never derives certification.

- [x] **Step 4: Run GREEN and refactor**

Run the contract test twice and require identical manifest/envelope bytes and stable diagnostics.

### Task 2: Bounded stdio JSON-RPC and safe artifact exchange

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/transport-stdio/Cargo.toml`
- Create: `crates/transport-stdio/src/lib.rs`
- Test: `crates/transport-stdio/tests/framing.rs`
- Create: `crates/artifact-exchange/Cargo.toml`
- Create: `crates/artifact-exchange/src/lib.rs`
- Test: `crates/artifact-exchange/tests/artifacts.rs`

**Interfaces:**
- Produces: `encode_frame`, `decode_frame`, `dispatch_frame`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcHandler`.
- Produces: `write_canonical_json`, `read_canonical_json`, `ArtifactReceipt`, `ArtifactExchangeError`.

- [x] **Step 1: Write failing transport and filesystem tests**

Prove byte-exact `Content-Length` roundtrip and direct-vs-stdio semantic equality; reject duplicate/invalid/oversized/truncated/trailing/non-UTF-8/malformed JSON. Prove canonical artifact roundtrip, exact digest, atomic no-overwrite and rejection of traversal, symlinks, noncanonical JSON, tampering and oversized files.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-transport-stdio --test framing
cargo test -p project-ledger-artifact-exchange --test artifacts
```

Expected: both crates and APIs are absent.

- [x] **Step 3: Implement minimal bounded codecs and no-clobber exchange**

The stdio layer handles generic JSON values only and owns no methods. Artifact writes use a same-directory temporary file and `persist_noclobber`; reads verify regular-file/no-symlink, byte bound, parseability and equality with reserialized canonical JSON.

- [x] **Step 4: Run GREEN and mutation review**

Verify wrong lengths, relaxed filenames, overwrite, missing canonical checks or ignored trailing bytes each break at least one test.

### Task 3: Read-only conformance harness, malicious fixtures and generic CLI

**Files:**
- Create: `crates/integration-protocol/src/conformance.rs`
- Test: `crates/integration-protocol/tests/conformance.rs`
- Create: `integrations/sdk/README.md`
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/contract_commands.rs`

**Interfaces:**
- Consumes: protocol integrity/identity/lock APIs and generic stdio framing.
- Produces: `ReadOnlyProvider`, `ObservableState`, `ReadConformanceSuite`, `ReadConformanceReport`, `run_read_conformance`, `malicious_read_requests`.
- Produces CLI commands `project-ledger integration manifest` and `project-ledger integration compatibility` with canonical JSON stdout.

- [x] **Step 1: Write failing conformance and CLI tests**

Use a real in-memory provider implementation, not mocks. Prove 100 current projections plus manifest/health/stale/failure calls leave observable version/event/digest unchanged; identical input/state returns identical canonical bytes; tiny/oversized/unsupported/ambiguous/authority/private requests fail; a provider failure is isolated and the following valid call succeeds. Compare CLI manifest JSON with the same direct builder and a stdio-dispatched `ledger.manifest` response.

- [x] **Step 2: Run RED**

Expected: harness, malicious fixtures and CLI integration commands are absent.

- [x] **Step 3: Implement the minimal conformance engine and read-only CLI surfaces**

Run all checks in stable order, time every provider call, compare the observable tuple after each call and return typed findings without mutating canonical state. Compatibility reads the three lock files, reports explicit missing evidence/status and leaves every current adapter uncertified.

- [x] **Step 4: Run GREEN twice**

Require deterministic reports, zero hidden observable mutations and stable machine JSON.

### Task 4: S8 gate, documentation and phase commit

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: this plan

- [x] **Step 1: Execute the complete gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
.tooling/bin/cargo-deny --offline check
git diff --check
```

Expected: every command exits 0; the read-conformance report has no failed check, observed mutations remain zero and all malicious fixtures are rejected.

- [x] **Step 2: Update persistent evidence and commit**

Record exact test counts, lock-gated compatibility, harness limitation and RED→GREEN evidence. If the Fase 5 gate is green, set progress to `6/12` macrofases and `9/15` slices and commit `feat: add generic integration protocol and conformance`.

## Self-review

- Spec coverage: REQ-INT-001/002/003, §32-33, §38.5, AC-003/043/045/050/051, PL-09/16/17 and S8 gate map directly to Tasks 1-3.
- Protocol honesty: the default manifest announces only the generic Ledger protocol; no external product is marked certified and current incomplete upstream locks remain explicit.
- Side effects: the harness compares version/event count/state digest before and after each observable call; generic crates have no SQLite dependency.
- Security/privacy: bounded framing, canonical artifacts, exact identity, no name fallback, privacy denylist, no symlink/traversal/overwrite and no network.
- Failure isolation: malformed transport/artifacts and provider errors cannot alter the following valid request or the observable source tuple.
- Adapter boundary: no G-Skills field mapping, sink submission, GSP contract, Radar overlay or Agent Skill behavior is implemented in S8.
