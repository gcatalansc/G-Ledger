---
name: project-ledger-continuity
description: Resume, record material continuity, evaluate evidence-backed gates, and create handoffs for a Project Ledger repository through the governed project-ledger CLI. Use when work explicitly depends on Project Ledger continuity; do not use for an ordinary project diary or unrelated task tracking.
---

# Project Ledger Continuity

Use `manifest.json` to check the supported tool range and `workflow.json` as the machine-readable procedure contract.

Read `examples/operational-session.json` when exercising the complete procedure. It contains argv arrays for the actual CLI; they are templates, not a batch to execute blindly. No current project state belongs in this package.

## Start or resume

1. Run the selected executable with `--version`; require >=0.2.0 and <0.3.0. Also probe the needed subcommands with `--help` (the examples list them): the version alone cannot distinguish earlier incomplete builds. If missing, incompatible or unsupported, stop Ledger operations and report `DEGRADED_MISSING_TOOL` with the reason. Never install, download or substitute a tool without authorization, and never substitute direct database access or Markdown parsing.
2. Run `project-ledger resume --database <path> --project-ref <id> --format json` before planning work. JSON preserves complete reference lists; `--format codex` is a bounded display that may omit items and shorten IDs, never an exhaustive state or a source of command arguments. Treat stale, conflicted, invalidated, or unknown freshness as a reason to reconcile, not as current state. If text cannot fit its budget, use JSON instead of assuming an empty project.
3. Use only authority, phase, gate, blocker, unknown, conflict, checkpoint, and next-action references returned by the engine. Never copy the returned snapshot into this skill.

## During work

- Record material progress, evidence, limitations, decisions, and unknowns through supported `project-ledger` commands. Every mutating command uses explicit expected version and idempotency.
- Obtain a fresh version from `resume --format json` or `status --json`. Keep the complete original argv, observed time and key outside generated views until the outcome is reconciled; never generate a new key just because a response was lost.
- Execution requires an explicit policy, absolute authorized executable and cwd. The example probes `--version`; it is OBSERVED evidence, not a qualified test result. Do not invent metrics or upgrade imported evidence.
- Request gate evaluation before making a completion claim. A successful local command or test is evidence, not authority to mark a gate verified or a phase complete.
- Never edit generated Ledger Markdown, open SQLite directly, invent missing canonical facts, clear omissions, or retry an unknown submission outcome blindly.

If a version conflict occurs, resume and replan. If an outcome is unknown, reconcile the original idempotency key before any retry. If a required subcommand is unavailable, report the unsupported operation and continue only with non-Ledger work already authorized by the user.

Read JSON and the exit code together. Exit 4 means no initialized project; 5 requires resume/replan; 6 means an unsatisfied gate even when its evaluation command was ACCEPTED; 9 is a permission denial (never add `--as-human` to bypass it); 10 means changed intent under an existing key; 12 is unresolved. A reconciled durable result is authoritative; absence is not proof that an external process did not run. An unresolved execution reservation requires investigation, not replay with a new key.

Use `reconcile --command-type <type> --actor-id <same actor> --idempotency-key <same key>` with the same project and trust domain. Normal agent records use TRUSTED_FOR_RECORDING; the existing checkpoint CLI uses LOCAL_USER, so pass `--trust-domain LOCAL_USER` for CREATE_CONTINUITY_CHECKPOINT. This lookup scope does not authorize human lifecycle changes. No dependency on G-Skills is needed.

## Handoff

Create a checkpoint through `project-ledger checkpoint create` with explicit version/idempotency, then run resume again and report the returned checkpoint and freshness. State remaining limitations and the next eligible action; do not declare phase or gate completion yourself.
