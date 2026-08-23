# Project Ledger Phase 7 / S10 G-Skills RecordSink Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the phase commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user explicitly requested the next phase and did not request delegation.

**Goal:** Complete the Ledger-owned S10 RecordSink boundary for the exact `gskills-context-provider/0.3.0` profile with explicit write trust, five allowlisted command mappings, deterministic result mapping, status/reconciliation after uncertain transport, and zero duplicate effects or blind retries.

**Architecture:** `interop-gskills` remains independent from SQLite and owns exact G-Skills wire validation, semantic mapping and a storage-independent `RecordSinkHost` port. A separate sink stdio handler exposes only manifest/validate/submit/status/reconcile; the CLI composition root owns SQLite, `CommandProcessor`, `CoreCommandHandler` and post-write projection confirmation. Transport uncertainty remains client/submission state and never becomes a native `CommandResult` or an inferred external acceptance.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, RFC 8785/SHA-256, closed Ledger domain contracts, existing event-store/application/SQLite command path, JSON Schema 2020-12 and S8 `Content-Length` JSON-RPC framing. No network, async runtime, shared database, unsafe code, arbitrary command mapping or hidden retry.

## Global Constraints

- Project Ledger v0.2.0, revision `GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT`, remains implementation authority.
- G-Skills input is revision `PL-UPSTREAM-REFRESH-1` plus corrective commit `6d63b60`; spec SHA-256 is `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`.
- Pin registry file SHA-256 `a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9`, fixture manifest file SHA-256 `316c056480f3ab613f937426946da08276e9673ec6781d86f5a8b1e1e5e614ee` and reciprocal lock SHA-256 `49ff8bf60b33deb47cc688f288f7c2e1124d99fb223d0d8d4d2668ea4c60929d`.
- Vendor exactly the five additional S10 contracts: `ExternalRecordCommand`, `ExternalRecordResult`, `ExternalSubmissionRecord`, `GateRevalidationSignal` and `CrossSystemReference`, with their paired fixtures and registry canonical digests.
- Only `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION` and `CREATE_CONTINUITY_CHECKPOINT` may map to native commands.
- `RECORD_DECISION_REFERENCE`, `RECORD_BLOCKER_REFERENCE`, phase/gate completion, project status transitions and arbitrary append remain rejected.
- Projection trust never grants write trust. Submit requires an explicit, digest-bound `TRUSTED_FOR_RECORDING` policy and a command allowlist.
- Expected state version, idempotency key and the exact cross-system scope are mandatory. Same key/same external command has at most one effect; same key/different canonical command is rejected.
- `ledger.command_validate` is side-effect-free. `GateRevalidationSignal` alone is side-effect-free and can only cause a request after explicit `ExternalRecordCommand` mapping to `REQUEST_GATE_REVALIDATION`.
- `ExternalRecordResult` maps only native `CommandResult`. Timeout, unavailable and `OUTCOME_UNKNOWN` belong exclusively to client-side submission/reconciliation state.
- Status/reconcile look up the original durable result by canonical command identity; they never submit or retry.
- `interop-gskills` may depend on storage-independent Ledger ports but not `storage-sqlite`, WAL, migrations or private tables.
- The CLI sink requires an explicit policy file supplied by the operator; repository metadata cannot silently enable writing.
- Local Ledger RecordSink conformance may close Fase 7/S10 under its §50 gate, but bilateral G-Skills E2E and certified compatibility remain pending until the counterpart S24 implementation exists and is exercised.

---

### Task 1: Exact S10 contracts and lock extension

**Files:**
- Modify: `crates/interop-gskills/src/contracts.rs`
- Modify: `crates/interop-gskills/src/lib.rs`
- Modify: `crates/interop-gskills/tests/upstream_contracts.rs`
- Create: `integrations/gskills/contracts/v0.3/{external-record-command,external-record-result,external-submission-record,gate-revalidation-signal,cross-system-reference}.schema.json`
- Create: `integrations/gskills/contracts/v0.3/fixtures/{positive,negative}/` paired S10 fixtures
- Modify: `integrations/gskills/profile-contract-lock.json`

**Interfaces:**
- Produces `VENDORED_GSKILLS_CONTRACTS`, `validate_gskills_contract(name, value)` and `verify_vendored_contracts(root)` over all eleven exact S9+S10 contracts.
- Preserves compatibility exports for existing read-provider callers while making the complete profile lock authoritative.

- [x] **Step 1: Write the failing contract-lock test**

Add literal expectations for eleven contracts and the five S10 canonical digests:

```rust
assert_eq!(report.schema_count, 11);
assert_contract("external-record-command", "sha256:8b6319d8...");
assert_contract("external-record-result", "sha256:54daeed6...");
assert_contract("external-submission-record", "sha256:48f8a00c...");
assert_contract("gate-revalidation-signal", "sha256:c8d80e7d...");
assert_contract("cross-system-reference", "sha256:6054bd16...");
```

The break caught is accepting a reduced S9-only lock or drifting from upstream S10 bytes.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gskills --test upstream_contracts
```

Expected: the count/digests fail because the five S10 artifacts and APIs are absent.

- [x] **Step 3: Vendor the exact five schemas and fixtures**

Copy only tracked bytes from G-Skills commit `6d63b60`; extend the embedded validator and profile lock. Do not relax nested schemas or modify the dirty G-Skills worktree.

- [x] **Step 4: Run GREEN and independent digest checks**

Require every positive fixture valid, every negative fixture invalid, all `$id` values exact and all canonical digests equal the upstream registry.

### Task 2: Write policy, semantic preparation and five exact command mappings

**Files:**
- Create: `crates/interop-gskills/src/sink_model.rs`
- Create: `crates/interop-gskills/src/sink_mapping.rs`
- Create: `crates/interop-gskills/src/sink.rs`
- Modify: `crates/interop-gskills/src/lib.rs`
- Modify: `crates/interop-gskills/Cargo.toml`
- Test: `crates/interop-gskills/tests/sink_mapping.rs`
- Create: `integrations/gskills/reports/record-sink-field-mapping.json`
- Create: `integrations/gskills/reports/record-sink-loss-report.json`

**Interfaces:**
- Produces closed `RecordSinkPolicy`, `RecordSinkStateIdentity`, `PreparedRecordCommand`, `RecordSinkHost`, `GSkillsRecordSink` and `RecordSinkError`.
- Produces `GSkillsRecordSink::manifest`, `validate`, `submit`, `status` and `reconcile` without importing SQLite.
- Maps the accepted external ID to a canonical Ledger `CommandId`, exact native schema ref, payload, expected version, idempotency key and configured `Actor { trust_level: TrustedForRecording }`.

- [x] **Step 1: Write failing trust and mapping tests**

Use hand-built literal external commands and a host with observable call counters. Assert:

```rust
assert!(sink_with_projection_only_trust.validate(&command).is_err());
assert_eq!(prepared.command.command_type(), CommandType::RecordWorkProgress);
assert_eq!(prepared.command.expected_state_version, 1);
assert_eq!(host.execute_calls(), 0);
```

Cover all five allowlisted payloads, exact identity/scope/digests, missing key/version, stale state, schema-ref mismatch, legacy capability, direct lifecycle command and approval-binding drift. The break caught is any authority elevation, analogous mapping or prepare-time side effect.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gskills --test sink_mapping
```

Expected: sink model/policy/mapping APIs do not exist.

- [x] **Step 3: Implement minimal semantic boundary**

Deserialize closed semantic substructures after exact outer-schema validation; verify the external digest, configured identities, `TRUSTED_FOR_RECORDING`, allowlist, expected state tuple, idempotency scope, privacy and approval-binding digests. Map only the five exact native payload variants.

For `REQUEST_GATE_REVALIDATION`, validate the complete `GateRevalidationSignal`, require its project/gate/recommended kind to agree, then map only to native `{ gate_ref, signal_ref }`; validating the signal alone never calls the host.

- [x] **Step 4: Run GREEN and mutation review**

Mutating trust to projection-only, command kind, native schema ref, gate ref, expected digest/version, actor scope or payload shape must break at least one test. Publish mapping/loss rows without claiming bilateral certification.

### Task 3: Submit/result/status/reconcile, stdio and mutable conformance

**Files:**
- Create: `crates/interop-gskills/src/sink_stdio.rs`
- Create: `crates/interop-gskills/src/sink_conformance.rs`
- Modify: `crates/interop-gskills/src/lib.rs`
- Test: `crates/interop-gskills/tests/sink_protocol.rs`
- Test: `crates/interop-gskills/tests/sink_stdio_e2e.rs`
- Create: `integrations/gskills/reports/record-sink-conformance.json`

**Interfaces:**
- Produces `GSkillsRecordSinkJsonRpcHandler` and `run_gskills_record_sink_conformance`.
- Registers only `ledger.manifest`, `ledger.command_validate`, `ledger.command_submit`, `ledger.command_status` and `ledger.command_reconcile`.
- Maps `CommandResult` to exact `ExternalRecordResult`; status/reconcile return the same durable result or explicit `NOT_FOUND` and never invoke submit.

- [x] **Step 1: Write failing protocol/conformance tests**

Exercise real native command processing through a test host and assert:

```rust
let first = sink.submit(&command).unwrap();
let replay = sink.submit(&command).unwrap();
assert_eq!(host.event_count(), 1);
assert_eq!(replay["status"], "DUPLICATE_REPLAY");

let reconciled = sink.reconcile(&command).unwrap();
assert_eq!(host.execute_calls(), 2); // reconcile adds no execution
assert_eq!(reconciled["result"]["native_result_digest"], first["native_result_digest"]);
```

Also cover same-key/different-digest rejection, expected-version conflict, lost response followed by reconcile, `NOT_FOUND`, exact emitted-event/result/checkpoint mapping, malformed frames, unknown methods and recovery after rejected requests. The break caught is duplicate effect, hidden retry, transport uncertainty encoded as a domain result or invented lifecycle.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-interop-gskills --test sink_protocol --test sink_stdio_e2e
```

Expected: submit/status/reconcile/handler/conformance APIs are absent.

- [x] **Step 3: Implement minimal submit and reconciliation path**

Revalidate the prepared command immediately before execution. Map the native durable result byte-for-byte by field; obtain post-submit projection references only through the host's confirmed read. `status` and `reconcile` call `command_result` only and return `NOT_FOUND` without retry.

- [x] **Step 4: Run GREEN twice**

Run focused protocol/stdio/conformance tests twice consecutively. Require deterministic results, one native effect for replay, zero submit calls during status/reconcile and no `OUTCOME_UNKNOWN`/`SINK_UNAVAILABLE` external domain result.

### Task 4: Real SQLite CLI composition, documentation, full gate and phase commit

**Files:**
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/Cargo.toml`
- Test: `crates/cli/tests/gskills_sink.rs`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `specs/compatibility/gskills-v0.3.0.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `integrations/gskills/upstream-lock.json`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

**Interfaces:**
- Produces `project-ledger serve sink --stdio --profile gskills-context-provider/0.3.0 --database ... --project-ref ... --gskills-project-ref ... --policy ... --observed-at ...`.
- CLI host verifies the Ledger chain, executes through `CommandProcessor<SqliteEventStore, FixedClock>` plus `CoreCommandHandler`, looks up durable results and rebuilds a confirmed post-write projection.

- [x] **Step 1: Write failing real-CLI E2E tests**

Create a real SQLite project and explicit digest-valid policy. Send framed validate, submit, replay/status/reconcile and forbidden commands; assert exact JSON-RPC results and unchanged event count for prepare, denied writes, status and reconcile.

- [x] **Step 2: Implement the composition root and docs**

Keep policy loading explicit and outside repository discovery. Update the lock to local S10 PASS with bilateral counterpart pending, record report hashes and preserve `certified_compatibility=false`.

- [x] **Step 3: Execute the complete gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo run -p project-ledger-cli -- integration gskills verify-contracts
cargo run -p project-ledger-cli -- integration compatibility
.tooling/bin/cargo-deny.exe --offline check
git diff --check
```

Expected: every command exits 0; duplicate effects and blind retries are zero; G-Skills local read/write evidence is complete while bilateral E2E/certification remains false.

- [x] **Step 4: Update persistent evidence and commit**

Set progress to `8/12` macrofases and `11/15` slices only after the full gate passes. Commit `feat: add exact G-Skills record sink` and verify a clean worktree.

## Self-review

- Spec coverage: Ledger §16, §32–34, §40–41, REQ-CON-001/002, AC-039/040/042–051, PL-06/07/08/11/17/20 and Fase 7 map to Tasks 1–4.
- Exactness: all eleven G-Skills profile schemas remain bound to the unchanged upstream registry/fixture manifest; semantic validation tightens submit eligibility without rewriting public schemas.
- Authority: write trust is configured independently; no external signal or test result can directly verify a gate or complete a phase.
- Idempotency: external scope is checked before mapping, native scope retains project/command/actor/trust/key, and status/reconcile never execute.
- Unknown outcome: only the client-side submission state may be unknown; the server exposes durable result lookup and no blind retry.
- Independence: `interop-gskills` has no SQLite dependency; the CLI is the sole storage composition root.
- Honesty: local S10 conformance is not bilateral E2E because G-Skills `crates/record-sink` remains a reserved boundary at `6d63b60`.
