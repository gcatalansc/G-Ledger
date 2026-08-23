# Project Ledger Phase 4 (S7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use test-driven development for every production behavior and verification-before-completion before the slice commit. Steps use checkbox (`- [ ]`) syntax for tracking. Execution is inline because the user authorized the next step and did not request delegation.

**Goal:** Complete S7 with a deterministic portable directory bundle, verified identical/new-project/fast-forward import, fail-closed stream divergence, explicit fork provenance, and review-first legacy Markdown import without `TRUST_ALL`.

**Architecture:** Two storage-independent crates follow the normative repository blueprint: `portable-bundle` owns artifact assembly, checksums, chain verification and import planning; `legacy-import` owns conservative extraction, contradiction detection and per-candidate review. A narrow event-import port allows SQLite to append only an already verified suffix in one transaction. Canonical domain contracts and closed command/event unions remain unchanged.

**Tech Stack:** Stable Rust 1.97.1, Rust 2024, existing RFC 8785/SHA-256 primitives, `BTreeMap`/`BTreeSet`, standard-library filesystem APIs, existing projection rebuild and SQLite/WAL adapter. No archive, parser, network or migration dependency is added.

## Global Constraints

- Authority remains `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`; preserved specs, schemas, fixtures and closed v0.2.0 command/event contracts are not edited.
- The portable representation is the §31.1 directory tree. Every JSON entry is canonical, `events.jsonl` is one canonical envelope plus LF per line, and `checksums.sha256` is stable path order.
- To avoid a checksum cycle, `checksums.sha256` covers every payload entry except `manifest.json` and itself; the manifest self-validates through `digest` and binds the checksum file through `checksums_digest`.
- The builder transports the required Radar/upstream artifacts supplied by their owners but does not implement S12 mapping logic early.
- Exact v0.2 schemas are accepted. Any v0.1 material returns an explicit migration-required failure; S7 does not silently reinterpret it.
- Import validates checksums, schema lock, manifest digest, event envelopes, hash chain, identity, stream continuity, state rebuild and same-ID/same-digest before mutation.
- Empty destination becomes `NEW_PROJECT`; an exact chain is idempotent; an exact local prefix becomes `FAST_FORWARD`; a newer local chain is never overwritten.
- Same-project divergence is `STREAM_DIVERGENCE`. `FORK_EXPLICIT` produces a new target identity plus immutable source project/event/digest provenance, but does not rewrite source events or authority. Materializing a fully re-identified fork would require a durable contract absent from the frozen v0.2 schemas, so S7 proves the mandatory fail-closed branch and exposes the explicit handoff without inventing an event.
- SQLite imports only verified event envelopes because §31 exports `events.jsonl`, not private command/result tables. Original causation refs and digests remain intact.
- Legacy parsing is conservative and line-addressed. Preview recommendations are always `REVIEW`; applying requires one explicit decision for every candidate, with no unresolved `REVIEW` and no bulk trust path.
- Legacy contradictions remain linked candidates. No imported text marks gates verified, closes phases, elevates authority or deletes contradictory history.
- Default bundle inputs reject known secret/raw/transcript fields and absolute filesystem paths. Writing refuses an existing destination instead of overwriting it.

---

### Task 1: Portable bundle contracts and deterministic export

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/portable-bundle/Cargo.toml`
- Create: `crates/portable-bundle/src/lib.rs`
- Create: `crates/portable-bundle/src/export.rs`
- Test: `crates/portable-bundle/tests/export.rs`

- [x] **Step 1: Write failing AC-005/PL-15 export tests**

Prove the exact required tree, canonical JSON/JSONL, stable checksums and manifest digests, byte-identical repeated export, correct project/state/event bindings, safe relative paths, privacy rejection and refusal to overwrite a destination.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-portable-bundle --test export
```

Expected: the crate and export APIs do not exist.

- [x] **Step 3: Implement the minimal deterministic builder and directory writer**

Use fixed entry names, stable lock ordering, exact bytes for generated Markdown and canonical bytes for JSON values. Build checksums before the manifest and compute the manifest digest with only its `digest` field excluded.

- [x] **Step 4: Run GREEN**

Run the export test twice and require identical files, hashes and manifest.

### Task 2: Verification, import policy and SQLite fast-forward

**Files:**
- Create: `crates/portable-bundle/src/import.rs`
- Modify: `crates/event-store/src/ports.rs`
- Modify: `crates/storage-sqlite/Cargo.toml`
- Modify: `crates/storage-sqlite/src/events.rs`
- Test: `crates/portable-bundle/tests/import.rs`
- Test: `crates/storage-sqlite/tests/portable_import.rs`

- [x] **Step 1: Write failing roundtrip/divergence tests**

Cover directory read/verify, checksum and schema tampering, broken event chain, state mismatch, identical idempotency, empty-store new project, exact-prefix fast-forward, same-ID/different-digest, divergent sibling, newer-local refusal, explicit fork provenance and SQLite rollback on invalid suffix.

- [x] **Step 2: Run RED**

Expected: verification/planning/import-store APIs are absent.

- [x] **Step 3: Implement verification before planning and mutation**

Rebuild `ProjectStateProjection` from exported events, compare every manifest field, classify chains by exact prefix, and expose only a verified append suffix to the storage port. Insert the suffix and update the stream head inside one `BEGIN IMMEDIATE` transaction.

- [x] **Step 4: Run GREEN**

Require AC-005/035 and PL-15 tests to pass with equivalent rebuilt state and zero mutation on every rejected import.

### Task 3: Conservative legacy preview and reviewed import

**Files:**
- Create: `crates/legacy-import/Cargo.toml`
- Create: `crates/legacy-import/src/lib.rs`
- Test: `crates/legacy-import/tests/review.rs`

- [x] **Step 1: Write failing AC-034 tests**

Use contradictory phase statements and gate-like prose to prove stable source digest/line provenance, explicit conflict links, all-preview `REVIEW`, no `TRUST_ALL`, one decision per candidate, rejection of missing/duplicate/foreign/unresolved decisions, and preservation of `IMPORT_AS_ASSERTED` as unverified proposals only.

- [x] **Step 2: Run RED**

Expected: legacy review APIs are absent.

- [x] **Step 3: Implement the minimal review-first workflow**

Parse headings and explicit labeled lines only; never infer from chronology. Generate deterministic candidate IDs, stable-set ordering and review digest. Finalization validates exact per-candidate decisions and returns proposals, not hidden commands/events.

- [x] **Step 4: Run GREEN**

Run legacy tests twice and require identical candidate/conflict/review bytes.

### Task 4: S7 integration gate, documentation and commit

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
.tooling/bin/cargo-deny check
git diff --cached --check
```

Expected: all commands exit 0; roundtrip preserves chain/state, divergence never mutates, and legacy review has no implicit apply path.

- [x] **Step 2: Update persistent evidence and commit**

Record exact counts, privacy boundaries, fork-contract limitation and RED-to-GREEN evidence. If the Fase 4 gate is green, set progress to `5/12` macrofases and `8/15` slices and commit `feat: add verified portable and legacy imports`.

## Self-review

- Spec coverage: REQ-REL-002, §29, §31, AC-005/034/035, PL-15 and S7 map directly to Tasks 1-3.
- Ownership: exported projections remain derived; canonical truth remains the event chain; importing cannot infer lifecycle from prose.
- Atomicity: the store sees only a verified suffix and still validates project/version/prior digest inside its transaction.
- Security/privacy: fixed relative paths, no overwrite, no network, no archive traversal, no raw outputs/secrets/transcripts/absolute paths by default.
- Contract honesty: no command, event or fork field absent from the frozen schema is fabricated; divergent chains remain fail-closed.
- S8 boundary: no manifest protocol, stdio transport, identity resolver, generic SDK or conformance adapter is implemented.
