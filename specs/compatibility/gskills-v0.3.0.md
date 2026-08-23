# Compatibilidad G-Skills v0.3.0

- Source revision actual observada: `PL-UPSTREAM-REFRESH-1`.
- Source spec SHA-256 actual: `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`.
- Snapshot histórico del handoff: `E2E-CONSOLIDATED-1` / `8ddbb2a9c23bf7355a1e577d93a0c541b0986df7416ab64069aea16295376928`.
- Profile: `gskills-context-provider/0.3.0`.
- Estado: `LOCK_GATED`.
- Compatibilidad certificada: no.
- Lock recíproco de spec: `CURRENT`; G-Skills fija Ledger `31cb7cd4...` y Ledger fija G-Skills `957e62be...`.
- Schemas/fixtures read-only S9: exactos y vendorizados para seis contratos.
- Bloqueo actual S9: el schema upstream cerrado rechaza campos normativos materiales de `active_phases[]` y `next_actions.ordered[]`; ver `integrations/gskills/reports/provider-field-mapping.json` y `provider-loss-report.json`.
- Bloqueos posteriores: provider conformance, RecordSink S10 y E2E bilateral.
- Capabilities anunciadas: `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION`, `CREATE_CONTINUITY_CHECKPOINT`.
- Capabilities no anunciadas: `RECORD_DECISION_REFERENCE`, `RECORD_BLOCKER_REFERENCE`.

No se implementa un provider con pérdida ni se relaja el schema vendorizado. Este documento informa estado; no sustituye los contratos o fixtures upstream.
