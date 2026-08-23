# Project Ledger Phase 0 (S0 + S1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use test-driven development for each production behavior and verification-before-completion before every commit or completion claim. This plan is executed inline in the current task because the user already authorized implementation.

**Goal:** Establish a reproducible Rust workspace that preserves the normative specifications and materializes Project Ledger v0.2.0 durable contracts as closed, offline-verifiable schemas with deterministic digests and fixtures.

**Architecture:** A small Rust workspace separates canonical JSON/digest behavior, core contract types, schema generation/validation, and the CLI shell. Schemas and fixtures are generated deterministically from typed Rust contracts and checked into Git; no storage, event execution, projections, adapters, network runtime, UI, daemon, or product Markdown renderer is implemented in this phase.

**Tech Stack:** Stable Rust 1.97, Cargo workspace, `serde`, `serde_json`, `schemars`, `sha2`, `uuid`, `time`, `thiserror`, `clap`, `jsonschema`, GitHub Actions.

## Global Constraints

- Authority order starts with `PROJECT_LEDGER_SPEC_v0.2.0.md`; spec/schema disagreement blocks the affected contract.
- Preserve `PROJECT_LEDGER_SPEC_v0.1.0.md` byte-for-byte with SHA-256 `772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865`.
- The first implementation phase is limited to S0 + S1.
- Stable Rust, cross-platform; no Python, Node, JVM, Docker, unsafe Rust, custom cryptography, networked tests, SQLite, event execution, adapters, Agent Skill, UI, daemon, or AI.
- All durable JSON objects reject unknown fields; event and command payloads are discriminated closed unions.
- All JSON Schema references resolve offline and every registered schema has positive and negative fixtures.
- `BITACORA_IMPLEMENTACION.md` is an implementation log, not canonical Project Ledger product state.
- Completion requires fresh `fmt`, `clippy`, test, schema, checksum, and dependency-policy evidence.

---

### Task 1: Preserve authority and establish the workspace

**Files:**
- Create: `.gitattributes`
- Create: `.gitignore`
- Create: `README.md`
- Create: `SPEC.md`
- Create: `CHANGELOG.md`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Create: `rust-toolchain.toml`
- Create: `Cargo.toml`
- Create: `deny.toml`
- Create: `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`
- Create: `specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md`
- Create: `specs/checksums.txt`
- Create: `BITACORA_IMPLEMENTACION.md`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: the immutable v0.2 handoff and the located v0.1 artifact whose digest matches the normative checksum.
- Produces: a Cargo workspace, immutable spec copies, checksum guard inputs, CI contract, and persistent implementation history.

- [x] **Step 1: Verify the v0.1 source artifact before copying**

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath "C:\Users\Alfredo\OneDrive\Documentos\Proyectos\Trading-Inversión\IA\Proyectos\5 - Projects Docs\G-Ledger Docs\PROJECT_LEDGER_SPEC_v0.1.0.md"
```

Expected digest: `772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865`.

- [x] **Step 2: Materialize workspace/configuration and copy the two specs without transformation**

The root workspace declares only `canonical-json`, `domain`, `schemas`, `cli`, and `xtask`. CI runs format, Clippy with `-D warnings`, tests on Windows/Linux/macOS, schema/checksum verification, and `cargo deny check` on Linux.

- [x] **Step 3: Verify copied bytes and baseline build metadata**

```powershell
Get-FileHash -Algorithm SHA256 specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md
Get-FileHash -Algorithm SHA256 specs/PROJECT_LEDGER_SPEC_v0.2.0.md
cargo metadata --no-deps --format-version 1
```

Expected: both normative digests match and Cargo metadata resolves without networked runtime behavior.

### Task 2: Canonical JSON and digest primitives

**Files:**
- Create: `crates/canonical-json/Cargo.toml`
- Create: `crates/canonical-json/src/lib.rs`
- Create: `crates/canonical-json/tests/canonical_json.rs`

**Interfaces:**
- Consumes: any `serde::Serialize` value.
- Produces: `to_canonical_vec<T: Serialize>(&T) -> Result<Vec<u8>, CanonicalJsonError>`, `sha256_digest<T: Serialize>(&T) -> Result<Sha256Digest, CanonicalJsonError>`, and validated `Sha256Digest` parsing/display.

- [x] **Step 1: Write failing tests**

```rust
#[test]
fn object_keys_are_sorted_recursively() {
    let value = serde_json::json!({"z": 1, "a": {"y": 2, "b": 3}});
    assert_eq!(
        String::from_utf8(to_canonical_vec(&value).unwrap()).unwrap(),
        r#"{"a":{"b":3,"y":2},"z":1}"#
    );
}

#[test]
fn digest_matches_a_hand_checked_sha256_fixture() {
    let value = serde_json::json!({"a": 1});
    assert_eq!(
        sha256_digest(&value).unwrap().to_string(),
        "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
}
```

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-canonical-json --test canonical_json
```

Expected: compilation fails because the public functions and digest type do not exist.

- [x] **Step 3: Implement the minimal deterministic serializer and SHA-256 wrapper**

Reject floating-point values that are non-finite, serialize objects with recursively sorted keys, preserve array sequence, and use the `sha2` crate rather than custom cryptography.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-canonical-json --test canonical_json
```

Expected: all canonical JSON and digest fixtures pass.

### Task 3: Core identifiers, timestamps, actors, errors, and domain contracts

**Files:**
- Create: `crates/domain/Cargo.toml`
- Create: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/id.rs`
- Create: `crates/domain/src/time.rs`
- Create: `crates/domain/src/error.rs`
- Create: `crates/domain/src/common.rs`
- Create: `crates/domain/src/project.rs`
- Create: `crates/domain/src/lifecycle.rs`
- Create: `crates/domain/src/evidence.rs`
- Create: `crates/domain/src/continuity.rs`
- Create: `crates/domain/src/integration.rs`
- Create: `crates/domain/tests/primitives.rs`
- Create: `crates/domain/tests/contracts.rs`

**Interfaces:**
- Consumes: RFC 3339 strings, UUIDv7 identifiers, and typed constructor arguments.
- Produces: validated typed IDs, `LedgerTimestamp`, `Actor`, `ErrorCode`, and all S1 durable models named in §14, §21, §22, §25, §26, §29, §31, §32, and §37.

- [x] **Step 1: Write failing primitive tests**

```rust
#[test]
fn event_id_rejects_a_non_v7_uuid() {
    let error = EventId::parse("event:550e8400-e29b-41d4-a716-446655440000").unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidIdentifier);
}

#[test]
fn timestamp_requires_an_explicit_offset() {
    assert!(LedgerTimestamp::parse("2026-08-22T12:00:00").is_err());
}
```

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-domain --test primitives
```

Expected: compilation fails because the typed primitives do not exist.

- [x] **Step 3: Implement validated primitives and closed enums**

Each durable struct uses `#[serde(deny_unknown_fields)]`; identifiers validate prefixes and UUIDv7; timestamps require RFC 3339 offsets; every state/classification is a Rust enum rather than a free string.

- [x] **Step 4: Run primitive GREEN and add contract roundtrip tests**

```powershell
cargo test -p project-ledger-domain --test primitives
cargo test -p project-ledger-domain --test contracts
```

The contract tests serialize and deserialize real representative `ProjectState`, `GateEvaluation`, `EvidenceRecord`, `ContinuityCheckpoint`, `IntegrationManifest`, and `RadarSyncEnvelope` values and reject unknown fields.

### Task 4: Closed command and event unions

**Files:**
- Create: `crates/domain/src/command.rs`
- Create: `crates/domain/src/event.rs`
- Create: `crates/domain/tests/closed_unions.rs`

**Interfaces:**
- Consumes: JSON `CommandEnvelope` and `EventEnvelope` documents.
- Produces: `CommandType` with exactly 53 variants, `CommandPayload` with exactly 53 typed variants, `EventType` with exactly 67 variants, `EventPayload` with exactly 67 typed variants, and envelopes that reject mismatched discriminators or extra fields.

- [x] **Step 1: Write failing closed-union tests**

```rust
#[test]
fn unknown_command_variant_is_rejected() {
    let value = valid_command_json_with_type("COMPLETE_PHASE_DIRECTLY");
    assert!(serde_json::from_value::<CommandEnvelope>(value).is_err());
}

#[test]
fn command_payload_with_unknown_field_is_rejected() {
    let value = create_project_command_with_extra_field("hidden", true);
    assert!(serde_json::from_value::<CommandEnvelope>(value).is_err());
}
```

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-domain --test closed_unions
```

Expected: compilation fails because envelopes and unions do not exist.

- [x] **Step 3: Implement minimal typed variants and envelope consistency validation**

No payload uses an unbounded map or `serde_json::Value`. `PHASE_COMPLETED`, `GATE_VERIFIED`, and `PROJECT_STATUS_DERIVED` remain event-only variants and never command variants.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-domain --test closed_unions
```

Expected: valid variants roundtrip; unknown, mismatched, and extra-field cases fail.

### Task 5: Offline schema registry, command-event matrix, and fixtures

**Files:**
- Create: `crates/schemas/Cargo.toml`
- Create: `crates/schemas/src/lib.rs`
- Create: `crates/schemas/src/catalog.rs`
- Create: `crates/schemas/src/fixtures.rs`
- Create: `crates/schemas/tests/registry.rs`
- Create: `crates/schemas/tests/fixtures.rs`
- Create: `xtask/Cargo.toml`
- Create: `xtask/src/main.rs`
- Generate: `schemas/v0.2/*.schema.json`
- Generate: `schemas/v0.2/radar/*.schema.json`
- Generate: `schemas/v0.2/schema-registry.json`
- Generate: `schemas/v0.2/array-semantics.json`
- Generate: `schemas/v0.2/command-event-matrix.json`
- Generate: `schemas/v0.2/fixtures/positive/*.json`
- Generate: `schemas/v0.2/fixtures/negative/*.json`
- Generate: `schemas/v0.2/fixtures/manifest.json`

**Interfaces:**
- Consumes: all `JsonSchema` contract roots and literal, hand-reviewed example values.
- Produces: `schema_catalog()`, `fixture_catalog()`, deterministic `cargo xtask schema generate`, and read-only `cargo xtask schema check`.

- [x] **Step 1: Write failing registry tests**

```rust
#[test]
fn every_registry_entry_resolves_without_network() {
    let registry = load_checked_in_registry();
    for entry in registry.entries {
        compile_schema_offline(entry.schema_path).unwrap();
    }
}

#[test]
fn every_schema_has_positive_and_negative_fixtures() {
    let registry = load_checked_in_registry();
    let fixtures = load_checked_in_fixture_manifest();
    assert_eq!(fixtures.schema_ids(), registry.schema_ids());
}
```

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-schemas --test registry --test fixtures
```

Expected: compilation or file-not-found failures because registry, generator, and fixtures do not exist.

- [x] **Step 3: Implement generator and materialize outputs**

```powershell
cargo xtask schema generate
```

The generator uses sorted paths and pretty JSON with LF endings. Negative fixtures add one unknown top-level field to a valid fixture, proving `additionalProperties: false`. The command-event matrix contains one row for each of the 53 commands and explicit derived-only ownership for every event without an external command origin.

- [x] **Step 4: Run GREEN and determinism check**

```powershell
cargo test -p project-ledger-schemas --test registry --test fixtures
cargo xtask schema check
git diff --exit-code -- schemas/v0.2
```

Expected: all positive fixtures validate, all negative fixtures fail for the intended unknown-field reason, all refs resolve offline, and regeneration yields no diff.

### Task 6: CLI shell, checksum verification, and phase gate

**Files:**
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/contract_commands.rs`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: checked-in schema/fixture/spec files.
- Produces: `project-ledger --version`, `project-ledger contracts verify`, and `project-ledger specs verify`; no project state mutation commands are implemented.

- [x] **Step 1: Write failing CLI behavior tests**

```rust
#[test]
fn specs_verify_reports_both_normative_digests() {
    let output = Command::cargo_bin("project-ledger").unwrap()
        .args(["specs", "verify"])
        .output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("v0.1.0 MATCH"));
}
```

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-cli --test contract_commands
```

Expected: compilation fails because the CLI binary does not exist.

- [x] **Step 3: Implement read-only verification commands**

The CLI reads files only, emits diagnostics to stderr and stable human output to stdout, and returns non-zero when a checksum, schema, registry, or fixture check fails.

- [x] **Step 4: Execute the complete phase gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo xtask schema check
cargo deny check
git diff --check
```

Expected: every command exits 0 with no warnings, test failures, checksum mismatches, schema drift, or dependency-policy violations.

- [x] **Step 5: Update lifecycle evidence and commit the completed phase**

Record the exact commands/results, scope exclusions, known integration locks, progress denominator, and next specification step in `BITACORA_IMPLEMENTACION.md`; then create one phase-closing commit only after the full gate is green.

```powershell
git add --all
git commit -m "feat: complete Project Ledger contract foundation"
git status --short --branch
```

## Self-review

- Spec coverage: S0 workspace/preservation/CI and S1 typed contracts/closed unions/registry/fixtures/matrix are assigned; S2+ persistence and runtime behavior are explicitly excluded.
- Placeholder scan: the plan contains no deferred implementation markers; upstream adapter certification is an explicit later-slice boundary, not unfinished S1 work.
- Type consistency: canonical JSON feeds digests; domain contracts feed schemas; schemas feed CLI verification; no product-specific adapter enters the core.
- Known specification ambiguity: command-event branching for `REFRESH_AUTHORITY_DOCUMENT_DIGEST` and `REQUEST_PHASE_COMPLETION` must be encoded as explicit alternatives in the matrix data model, preserving §14.2 and §17.2 rather than treating mutually exclusive events as simultaneously required.
