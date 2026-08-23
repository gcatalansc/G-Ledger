# Project Ledger v0.2.0 threat model

## Scope and assets

The protected assets are the append-only event chain, command results and idempotency history, authority documents, evidence metadata, continuity checkpoints, generated views, portable/integration artifacts and compatibility labels. Raw terminal output, transcripts, private reasoning and secrets are out of the default persistence model.

## Trust boundaries

- Repository content is untrusted input. It cannot enable adapters, writes, authority, network, credentials or executable hooks.
- The local human/engine boundary owns lifecycle changes. Agent Skills, G-Skills, GSP and Radar cannot declare gates or phases complete.
- SQLite is the canonical local store. Markdown, projection caches, exports and counterpart databases are derived or external and never become canonical by observation.
- Integration payloads and Radar artifacts are untrusted bytes until path, size, schema, identity and digest checks pass.
- External process execution uses an exact executable and argv, explicit cwd, allowlisted environment, timeout/output bounds and no shell by default.

## Primary threats and controls

| Threat | Control | Residual boundary |
|---|---|---|
| Event mutation, gap or forged replay | append-only triggers, contiguous versions, mandatory digest chain and rebuild | physical storage compromise still requires backup recovery |
| Duplicate write after timeout | scoped idempotency plus submission reconciliation | unknown remains unknown until reconciled |
| Repository prompt/config injection | capabilities derive from trusted state, not repository files | user can explicitly grant trust outside repository content |
| Path traversal or symlink escape | normalized relative paths, root containment and no symlink-following | platform filesystem bugs are outside the model |
| Shell/meta-character injection | executable plus literal argv; no shell command string | explicitly added future shells require a separate reviewed policy |
| Secret/transcript/reasoning persistence | raw-off defaults, redaction, references-only exports and declared-corpus scan | scanner proves only its declared corpus and supported patterns |
| GSP/Radar/G-Skills authority elevation | exact contracts, provenance, field ownership and engine-owned lifecycle | counterpart/runtime behavior remains uncertified until bilateral evidence |
| False compatibility claim | all local profiles report `certified_compatibility=false` with pending evidence | GA remains blocked by explicit waivers/pending counterpart evidence |
| Oversized/malformed input | closed schemas and size/item/time budgets | denial outside configured resource ceilings is not accepted input |

## Release response

Any critical finding, secret persistence, chain break, schema mismatch, authority elevation or certified-compatibility overclaim is NO-GO. The engineering release candidate may carry only explicit non-critical waivers with owner, reason, scope, review date and the release/certification level they block.
