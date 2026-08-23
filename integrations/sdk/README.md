# Project Ledger generic integration SDK (S8)

This directory documents the storage-independent Rust SDK implemented by the workspace crates:

- `project-ledger-integration-protocol`: canonical manifest, health, projection envelope and loss-report builders; exact identity resolution; conservative upstream-lock inspection; malicious read fixtures and observable read-conformance harness.
- `project-ledger-transport-stdio`: bounded JSON-RPC 2.0 `Content-Length` framing and generic dispatch.
- `project-ledger-artifact-exchange`: canonical local JSON exchange with digest verification and atomic no-clobber publication.

The generic SDK has no dependency on SQLite, WAL files, private migrations or any external product runtime. Product-specific adapters consume this boundary out-of-process or through artifact exchange; the generic crates never import those adapters.

## Read conformance

`run_read_conformance` verifies, in stable order:

1. protocol negotiation and manifest integrity;
2. health binding and digest;
3. 100 deterministic current projections by the normative suite;
4. exact state/project/payload/schema/digest bindings;
5. current/stale freshness;
6. byte/item bounds and measured call timeout;
7. exact identity with ambiguous repository identity rejected;
8. no authority-only or raw-private request fields;
9. secret marker redaction in typed output;
10. failure isolation followed by a successful deterministic request;
11. the same observable `state_version`, `event_count` and `state_digest` after every call.

The observable tuple is supplied by the adapter under test. This is executable evidence inside the harness boundary; it is not a universal sandbox against an equally privileged local process. Runtime deployments additionally require explicit trust, out-of-process execution, minimum permissions, limits and quarantine.

## Compatibility status

`project-ledger integration compatibility` inspects the distributed lock skeletons. A lock is complete only when upstream version, spec digest, schema-registry digest, fixture-manifest digest and compatibility range are exact. Missing/null evidence remains `PENDING`, and a certification claim with incomplete evidence is rejected.

S8 does not certify G-Skills, GSP or Project Radar. Their current locks remain pending and `certified_compatibility=false`. G-Skills provider and RecordSink conformance are S9/S10, the exact GSP bridge is S11 and Radar mapping/importer conformance is S12. Mutable prepare/submit/reconcile conformance belongs to the S10 RecordSink slice because S8 intentionally exposes no partial write runtime.

## Minimal usage

```rust
use project_ledger_integration_protocol::{
    build_default_manifest, inspect_upstream_lock, run_read_conformance,
};

let manifest = build_default_manifest(Vec::new())?;
// Supply a ReadOnlyProvider and a ReadConformanceSuite owned by the adapter.
let report = run_read_conformance(&provider, &suite)?;
assert_eq!(report.observable_mutations, 0);
```
