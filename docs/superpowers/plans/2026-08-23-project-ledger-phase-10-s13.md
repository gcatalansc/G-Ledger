# Project Ledger Phase 10 / S13 Agent Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This authorized run executes inline in the current thread, preserves one phase commit, and does not start S14.

**Goal:** Ship a versioned Project Ledger continuity skill whose observable procedure, degradation and authority boundaries satisfy AC-061 through AC-064 and whose checkpoint-to-resume handoff runs against the real CLI and SQLite store.

**Architecture:** A small `project-ledger-agent-skill` crate validates the package schemas, raw-file lock, trigger cases and policy cases offline. The package under `integrations/agent-skills/project-ledger-continuity/` keeps procedure in `SKILL.md` plus a machine-readable workflow; the CLI composes existing continuity/application/storage engines for `resume`, `checkpoint create`, and skill verification without moving authority into the skill.

**Tech Stack:** Rust 1.97.1, serde/serde_json, jsonschema, SHA-256 canonical JSON, clap, SQLite, Agent Skills `SKILL.md` conventions.

## Global Constraints

- Normative authority remains `referencias/PROJECT_LEDGER_v0.2.0_CODEX/PROJECT_LEDGER_SPEC_v0.2.0.md`, especially §27, §36, §50, §51 and AC-061…064.
- The skill is procedural only: `state_authority=false`, `core_dependency=false`, no canonical state, no direct database access, no generated-Markdown edits and no phase/gate completion bypass.
- Every write uses the `project-ledger` command boundary with explicit expected version and idempotency when applicable; resume is read-only.
- Missing tool/subcommand, version conflict and unknown transport outcome degrade explicitly and fail closed.
- Tests are offline and cross-platform; no network, daemon, AI inference, Python, Node or companion product is required.
- S14 hardening/release is outside this plan.

---

### Task 1: Exact skill package contract and raw-byte lock

**Files:**
- Create: `crates/agent-skill/Cargo.toml`
- Create: `crates/agent-skill/src/lib.rs`
- Create: `crates/agent-skill/src/contracts.rs`
- Create: `crates/agent-skill/tests/package_contract.rs`
- Create: `integrations/agent-skills/project-ledger-continuity/schemas/manifest.schema.json`
- Create: `integrations/agent-skills/project-ledger-continuity/schemas/workflow.schema.json`
- Create: `integrations/agent-skills/project-ledger-continuity/manifest.json`
- Create: `integrations/agent-skills/project-ledger-continuity/workflow.json`
- Create: `integrations/agent-skills/profile-lock.json`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

**Interfaces:**
- Produces `AgentSkillCatalog::load(root)`, `verify_agent_skill_package(root)` and `AgentSkillContractReport`.
- Validates exact inventory, raw SHA-256 lock, JSON Schema 2020-12, self-digests, profile/version/tool/platform/compatibility fields and absence of concrete current-state identities.

- [x] **Step 1: Write the failing package test**

The test requires profile `project-ledger-agent-skill/0.2.0`, tool `project-ledger` in `[0.2.0,0.3.0)`, Windows/Linux/macOS, `state_authority=false`, `core_dependency=false`, exact file hashes and rejection after one-byte drift.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-agent-skill --test package_contract
```

Expected: the crate/package APIs do not exist.

- [x] **Step 3: Implement the minimal offline validator and package artifacts**

Use closed JSON schemas and canonical self-digests. Scan executable instruction artifacts for concrete `project:<uuid>`, `checkpoint:<uuid>` and state-digest values; fixtures use placeholders rather than live state.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-agent-skill --test package_contract
```

Expected: exact package passes and drift/current-state injection fails closed.

### Task 2: Behavioral trigger, degradation and no-authority harness

**Files:**
- Create: `crates/agent-skill/src/workflow.rs`
- Create: `crates/agent-skill/tests/conformance.rs`
- Create: `integrations/agent-skills/project-ledger-continuity/tests/trigger-cases.json`
- Create: `integrations/agent-skills/project-ledger-continuity/tests/policy-cases.json`
- Create: `integrations/agent-skills/project-ledger-continuity/examples/resume-start.json`
- Create: `integrations/agent-skills/project-ledger-continuity/examples/session-handoff.json`
- Create: `integrations/agent-skills/project-ledger-continuity/SKILL.md`

**Interfaces:**
- Produces `SkillIntent`, `RuntimeConditions`, `SkillPlan`, `plan_skill_actions` and `run_agent_skill_conformance`.
- Observable outcomes include `NOT_TRIGGERED`, `READY`, `DEGRADED_MISSING_TOOL`, `REPLAN_REQUIRED`, `RECONCILE_REQUIRED` and explicit action plans that never claim completion themselves.

- [x] **Step 1: Write failing behavioral tests**

Positive start/progress/evidence/gate/handoff cases must trigger; unrelated requests must not. Missing tool yields zero mutating actions, version conflict requires refresh/replan, unknown outcome requires reconciliation, and phase/gate close requests remain engine requests rather than completion declarations.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-agent-skill --test conformance
```

Expected: workflow types and planner do not exist.

- [x] **Step 3: Implement the pure planner and skill instructions**

The `SKILL.md` entrypoint routes to the machine-readable workflow, requires resume before work, records only material progress, evaluates before claims, checkpoints on handoff, and reports limitations. It never embeds a project snapshot or instructs direct storage/Markdown mutation.

- [x] **Step 4: Run GREEN and adversarial mutations**

```powershell
cargo test -p project-ledger-agent-skill --test conformance
cargo test -p project-ledger-agent-skill
```

Expected: all cases pass; added forbidden authority/direct-write actions or missing-tool writes are rejected.

### Task 3: Real CLI verification and checkpoint-to-resume handoff E2E

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Create: `crates/cli/tests/agent_skill_workflow.rs`
- Modify: `crates/cli/tests/contract_commands.rs`
- Modify: `crates/integration-protocol/tests/contracts.rs`

**Interfaces:**
- Adds `integration agent-skill verify-contracts`.
- Adds `resume --database <path> --project-ref <id> --format codex|json` as a zero-event read.
- Adds `checkpoint create --database <path> --project-ref <id> --reason <text> --expected-version <n> --idempotency-key <id> --actor-id <id> --observed-at <timestamp>` through `CommandProcessor<SqliteEventStore>` and `CoreCommandHandler`.

- [x] **Step 1: Write failing CLI tests**

Assert missing commands first, then require init→resume(no write)→checkpoint(one canonical write)→resume(current checkpoint), deterministic duplicate replay and explicit version conflict. Verify skill contracts through the CLI and keep compatibility uncertified.

- [x] **Step 2: Run RED**

```powershell
cargo test -p project-ledger-cli --test agent_skill_workflow
```

Expected: `resume`, `checkpoint` and `integration agent-skill` are unrecognized.

- [x] **Step 3: Compose existing engines without new authority**

Resume rebuilds the verified event stream and renders the selected existing budget. Checkpoint uses explicit actor/version/idempotency, returns the canonical command result, and accepts only `ACCEPTED` or `DUPLICATE_REPLAY` as successful execution.

- [x] **Step 4: Run GREEN**

```powershell
cargo test -p project-ledger-cli --test agent_skill_workflow
```

Expected: handoff E2E passes with event counts and freshness assertions.

### Task 4: Documentation, matrix, complete gate and phase commit

**Files:**
- Create: `specs/compatibility/agent-skills.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: this plan

**Interfaces:**
- Documents installation/use, explicit degradation and evidence boundaries.
- Closes only Fase 10 / S13, advancing to 11/12 macro phases and 14/15 slices.

- [x] **Step 1: Reconcile docs and persistent records**

Record RED/GREEN evidence, package hashes, handoff E2E, known limits and the non-authoritative boundary. Do not call procedural conformance an operational certification.

- [x] **Step 2: Execute the complete gate**

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
cargo run -p project-ledger-cli -- integration agent-skill verify-contracts
cargo run -p project-ledger-cli -- integration compatibility
cargo deny --offline check
git diff --check
git status --short
```

Expected: every command exits 0, AC-061…064 have local evidence, skill remains non-authoritative, and only S13 files are changed.

- [x] **Step 3: Verify tracked bytes and commit the phase**

Recompute every skill lock hash, mark the plan complete, update the bitácora and create `feat: add governed Agent Skill workflow`. Do not begin S14.

## Completion Evidence Required

- AC-061: package manifest/profile, schemas, inventory and raw digests validate offline.
- AC-062: no concrete current project/checkpoint/state snapshot exists in executable skill instructions.
- AC-063: missing tool/subcommand, version conflict and unknown outcome have explicit fail-closed plans.
- AC-064: the skill cannot directly persist state, edit generated Markdown or assert phase/gate completion; it submits requests to the engine.
- Triggers: positive and negative cases are evaluated as behavior, not only text matching.
- Handoff: real SQLite init/resume/checkpoint/resume preserves chain integrity and demonstrates resume is read-only.
- Boundary: conformance is local procedural evidence, not proof of agent obedience, external runtime certification or S14 release readiness.

## Closure

- Closed on 2026-08-23 after the complete gate passed from the reconciled tree.
- Inventory: 207 Rust `#[test]` attributes, with zero failures in the workspace gate.
- Agent Skills validator: `quick_validate.py` PASS on the final package.
- Locked bytes: schema registry `8b543f807789a3c0aeec0ee8d3b1d3b7400205fafbcd5ff7d5929293a2cfc99c`; fixture manifest `af685beb9a26ea4cfa4ddc24863919a3bf1dbd3df1bbbfa7064fd42f42432904`.
- Result: Fase 10 / S13 is complete locally; progress is 11/12 macro phases and 14/15 implementation slices.
- Evidence boundary: procedural conformance and local SQLite handoff are PASS; `state_authority=false`, `core_dependency=false` and `certified_compatibility=false` remain unchanged.
- Phase commit: `feat: add governed Agent Skill workflow`.
- Next authorized-by-spec slice, not started here: Fase 11 / S14 hardening, benchmark and release.
