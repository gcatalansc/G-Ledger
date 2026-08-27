# Project Ledger Live GSP Chaos Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> Execution here is inline under the user's authorization, with `test-driven-development` and `verification-before-completion`. The named execution subskills are not available in this session; no delegation or upstream changes are required.

**Goal:** Qualify the remaining `live-sidecar-process-crash` chaos scenario against the actual locked GSP compiler, preserving Ledger truth across process termination and recovery.

**Architecture:** An optional one-shot Python artifact adapter invokes the unchanged `GSPCompiler.compile_bundle` from a clean checkout at the exact pinned commit. A Rust CLI integration test exports a minimized corpus from disposable SQLite, starts the real compiler, terminates only its own child after entering compilation, verifies the original chain/projection, restarts compilation, validates all six artifacts and explicitly links/replays the receipt. Test-only fault synchronization stays outside the adapter and outside upstream. A dedicated hosted job provisions the public pinned counterpart and its frozen Python environment; release evidence changes only after that exact source SHA passes.

**Tech Stack:** Rust 1.97.1, Python 3.12, GSP v0.2.0, frozen `uv.lock`, SQLite/WAL, existing artifact-exchange CLI, GitHub Actions.

## Conciliación posterior al commit — 2026-08-27

RQ4 está cerrado **3/3** para su alcance de crash/recovery. La consulta fresca del run `33062468586` confirma `completed/success` sobre `dea208d294dfddc282b84dcb5a58b9d8d9887b91`; seis jobs success, incluido GSP real `98484355574` y ENOSPC `98484355673`. Los checkpoints 2/3 del documento se conservan como historia pre-commit, no como estado vigente. Esta calificación no acredita la CLI cotidiana ni un paquete portable, y no certifica GSP. El trabajo siguiente lo gobierna el plan pre-piloto de ocho etapas autorizado el 2026-08-27.

## Global Constraints

- Ledger authority: §32.2/32.3, §35.1–35.7, §45.7–45.8 of `PROJECT_LEDGER_SPEC_v0.2.0.md`.
- Counterpart implementation: `7a43559c02417641691dbb5f2a6e79c13d74d1a1` / `0.2.0`; contract authority: `004b14d7b64837025671970eac2469f092c49475` / `gsp-exact-2026.04.12`.
- Preserve all 28 registry contracts, 31 schemas and 140 fixtures; no reduced public GSP contract.
- No changes to the active GSP checkout, its published tags or canonical contracts. Provision a separate ignored checkout.
- Follow GSP's workflow: execute the compiler and validate outputs, never synthesize canonical result bundles by assistant judgement. Historical handoff links to removed planning documents do not authorize changes to that completed upstream implementation.
- No Python, model, network or daemon requirement in Ledger's default golden path. Python is opt-in for this adapter/test only.
- Use literal argv, explicit source/input/output paths, bounded files and create-new output. The child receives no Ledger database path or handle.
- Kill/reap only a child created by the test. Fault synchronization belongs in test support, never production/upstream APIs.
- Keep `certified_compatibility=false` and `ENGINEERING_RELEASE_CANDIDATE`. One bounded chaos qualification is not full runtime certification, scientific validation or GA promotion.

---

### Task 1: Qualify the exact counterpart and prepare the bounded runtime test

**Files:** `crates/cli/tests/gsp_bridge.rs`, new `crates/cli/tests/gsp_live/mod.rs`; optional adapter `integrations/gsp/sidecar/compile_bundle.py`.

**Interfaces:** Existing `Fixture` supplies disposable SQLite, a five-source corpus request and the real `export-corpus`, `validate-result`, `link-result` CLI. Explicit `PROJECT_LEDGER_GSP_SOURCE_ROOT` and `PROJECT_LEDGER_GSP_PYTHON` select the isolated counterpart/interpreter. Missing opt-in must fail, not silently substitute fixtures.

- [x] Provision a separate checkout, require exact HEAD and clean tracked source, then run `uv sync --frozen` in that checkout. Do not install into global Python or mutate the active GSP environment.
- [x] Write the ignored live test first. Its first observable requirement is a real compiler artifact accepted by Ledger:

  ```rust
  let mut child = OwnedChild(runtime.command(&fixture).spawn().unwrap());
  assert!(child.wait_bounded().success());
  validate(&fixture);
  assert_eq!(event_count(&fixture.database), 7);
  ```

  Run the exact new test with `--ignored --exact --nocapture --test-threads=1`; verify failure because the artifact adapter is absent.
- [x] Implement the minimal artifact adapter: reject wrong/dirty source, verify vendored schema bytes, bound input, call the actual `GSPCompiler(schema_root=...).compile_bundle(query=..., evidence_bundle=...)`, serialize `result.as_dict()` with all seven top-level fields, and create output exclusively. Do not call `run_corpus`, answer providers or model endpoints.
- [x] Verify real compile/validation GREEN plus negative wrong-source, malformed/oversized input and pre-existing-output cases without modifying canonical upstream.

### Task 2: Inject a real process crash and verify recovery

**Files:** `crates/cli/tests/gsp_live/mod.rs`, new `crates/cli/tests/gsp_live/crash_probe.py`.

**Interfaces:** A test-only Python profiling hook observes entry to the real `gspgemma.compiler.pipeline.compile_evidence`, signals an explicit temporary marker and pauses. Rust waits with a deadline, calls `Child::kill`, waits/reaps, and checks no successful artifact was published. This is synchronized OS process termination, not a fixture error return.

- [x] Extend the test before adding the fault helper:

  ```rust
  let before = fingerprint(&fixture.database);
  let mut crash = Command::new(&runtime.python);
  crash.args(["-I", "-B"])
      .arg(repository_root().join("crates/cli/tests/gsp_live/crash_probe.py"))
      .arg(&marker)
      .args(runtime.command(&fixture).get_args().skip(2))
      .stdin(Stdio::null());
  let mut child = OwnedChild(crash.spawn().unwrap());
  await_compile_marker(&mut child, &marker);
  child.0.kill().unwrap();
  assert!(!child.wait_bounded().success());
  assert!(!fixture.result.exists());
  assert_eq!(fingerprint(&fixture.database), before);
  ```

  Verify RED at the missing synchronization/child evidence, then implement only the test helper and bounded child ownership/cleanup.
- [x] Restart a fresh real compiler, validate six exact artifacts and preserved evidence/request identity. Require identical Ledger version/event digests/projection before explicit link; then require link delta `1` and same-key replay delta `0`, with no lifecycle/gate/phase elevation.
- [x] Emit compact evidence lines for counterpart SHA, process entry/termination/reaping, chain preservation, valid restart, explicit link and replay. No private corpus or raw environment values in logs.

### Task 3: Enforce evidential closure and execute hosted qualification

**Files:** `.github/workflows/ci.yml`, `crates/release/src/report.rs`, `crates/release/tests/release_gate.rs`, `BITACORA_IMPLEMENTACION.md`.

**Interfaces:** Existing hash-locked chaos report; a covered live crash requires an exact counterpart commit, observed process-termination discriminator and immutable hosted run/job locator. Missing evidence fails closed.

- [x] Add a re-locked release mutation that moves the live scenario to covered without runtime evidence and assert verification fails; observe RED before implementing the coverage guard.
- [x] Add the corresponding guard while retaining the current waiver. Add a hosted Ubuntu job that checks out GSP at the fixed SHA into an ignored directory, uses Python 3.12 plus a pinned `uv` tool and `uv sync --frozen`, verifies the clean source, and runs the exact opt-in test. Bound the job duration; no private credential or model is required.
- [x] Run format, strict Clippy, full workspace/all-targets/all-features tests, locked release build, schema/spec/contract/integration/benchmark/release gates, dependency policy, 13-checksum replay and diff checks. Result: 230 passed, 0 failed, 4 intentionally ignored; full gate PASS. The harness snapshot is prepared for commit/push with its waiver intact.
- [x] Require every hosted job to finish successfully at that source commit; inspect the actual live crash/recovery log before changing the waiver. Run `33060749387` passed all six jobs on `30c17ac0253a581db69b7fe38e001da07c9216d8`; live job `98478615297` showed real process termination/reaping, preserved chain/projection, fresh compilation, link delta 1 and replay delta 0.

### Task 4: Close only the observed chaos qualification

**Files:** `release/chaos-report.json`, `release/compatibility-report.json`, generated manifest/checksums, release library/CLI tests, `integrations/gsp/upstream-lock.json`, `specs/compatibility/gsp-v0.2.md`, README, CHANGELOG, contract matrix and bitácora. Reconcile only Ledger-owned qualification metadata in the upstream lock; preserve every upstream pin/digest and the historical S11 conformance report.

**Interfaces:** Successful immutable source run/job/SHA evidence; report counts become zero chaos waivers/GA chaos blockers/GSP chaos blockers. Compatibility flags remain false.

- [x] First require `chaos_waived_count=0` and `gsp_runtime_certification_blocker_count=0` in library/CLI tests and observe RED against the existing waiver. Both consumers failed with exit 101 while the official report still returned one waiver.
- [x] Move only the live crash scenario to covered using the observed fault, counterpart SHA and hosted locator; status becomes `PASS`. Regenerate manifests/checksums and verify GREEN. Release library 11/11, CLI 2/2 and live counterpart 3/3 passed after the evidence update.
- [x] Reconcile documentation with the exact artifact-exchange/crash scope, leaving full integration certification separate. Run the complete fresh gate and prepare the closing commit. Final local gate: 231 passed, 0 failed, 4 intentionally ignored; strict Clippy, locked release build, schema/spec/contract/integration/benchmark/release/dependency gates and 13 checksums passed.
- [ ] Publish the closing commit and verify all hosted jobs on its exact SHA. Report that final read-only observation in the delivery message; preserve the already-successful immutable source run in committed evidence.

## Progress and Completion

- Existing implementation: 12/12 macro phases; 15/15 slices. AC-086/089 qualification: 2/2. Physical disk-full: 1/1.
- RQ4 milestones: 0/3 at opening — real local counterpart/crash recovery; source hosted qualification; evidence-qualified closure.
- Checkpoint: 2/3 achieved, with the evidence-qualified closing snapshot ready. Local and hosted live tests passed, as did exact upstream hash/contract/fixture commands and 18 unchanged upstream API/compiler/contract tests. All six source jobs and the fresh local closing gate passed. Milestone 3 requires publication and final exact-SHA CI; its post-commit observation belongs in the delivery, not a prediction embedded in this snapshot.
- Success requires evidence for every milestone, final HEAD equal to remote main and hosted head SHA, and a clean worktree. An unavailable/mismatching counterpart leaves the waiver intact and is reported explicitly.
- Next after RQ4: assess remaining counterpart/bilateral evidence for the integration profile chosen for certification; do not infer certification or implement a different repository without authorization.
