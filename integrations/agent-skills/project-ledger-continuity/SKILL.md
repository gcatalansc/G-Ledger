---
name: project-ledger-continuity
description: Resume, record material continuity, evaluate evidence-backed gates, and create handoffs for a Project Ledger repository through the governed project-ledger CLI. Use when work explicitly depends on Project Ledger continuity; do not use for an ordinary project diary or unrelated task tracking.
---

# Project Ledger Continuity

Use `manifest.json` to check the supported tool range and `workflow.json` as the machine-readable procedure contract.

## Start or resume

1. Confirm that `project-ledger` exists and its version is within the manifest range. If the tool or required subcommand is absent, stop Ledger operations and report `DEGRADED_MISSING_TOOL`; do not substitute direct database access or Markdown parsing.
2. Run `project-ledger resume --database <path> --project-ref <id> --format codex` before planning work. Treat stale, conflicted, invalidated, or unknown freshness as a reason to reconcile, not as current state.
3. Use only authority, phase, gate, blocker, unknown, conflict, checkpoint, and next-action references returned by the engine. Never copy the returned snapshot into this skill.

## During work

- Record material progress, evidence, limitations, decisions, and unknowns through supported `project-ledger` commands. Every mutating command uses explicit expected version and idempotency.
- Request gate evaluation before making a completion claim. A successful local command or test is evidence, not authority to mark a gate verified or a phase complete.
- Never edit generated Ledger Markdown, open SQLite directly, invent missing canonical facts, clear omissions, or retry an unknown submission outcome blindly.

If a version conflict occurs, resume and replan. If an outcome is unknown, reconcile the original idempotency key before any retry. If a required subcommand is unavailable, report the unsupported operation and continue only with non-Ledger work already authorized by the user.

## Handoff

Create a checkpoint through `project-ledger checkpoint create` with explicit version/idempotency, then run resume again and report the returned checkpoint and freshness. State remaining limitations and the next eligible action; do not declare phase or gate completion yourself.
