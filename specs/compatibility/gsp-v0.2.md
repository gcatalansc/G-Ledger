# Compatibilidad GSP-Gemma Compiler v0.2

- Frontera: exact-contract, corpus explícitamente acotado y artifact exchange opcional.
- Implementación objetivo: `v0.2.0` / `7a43559c02417641691dbb5f2a6e79c13d74d1a1`.
- Autoridad contractual: `gsp-exact-2026.04.12` / `004b14d7b64837025671970eac2469f092c49475`.
- Registry: 28 contratos; superficie: 31 schemas JSON y 140 fixtures YAML exactos (56 válidos, 84 inválidos).
- Shared database: no.
- Python/red/daemon en el core: no.
- Mapping `EvidenceBundle`: exacto, validado contra upstream y unido al manifest/contenidos por digest canónico.
- `CompilationResult`: seis artefactos completos, validados y digest-addressed.
- Contradicción/incertidumbre: visibles; nunca normalizadas a lifecycle truth.
- Única mutación posible: comando nativo y explícito `LINK_EXTERNAL_EVIDENCE` sobre un receipt verificado contra la proyección Ledger vigente.
- Degradación semántica local: 0.
- Elevación de autoridad: 0.
- Lock/evidencia local S11: `COMPLETE`.
- Compatibilidad certificada: no.

La conformance local usa los contratos/fixtures upstream y un puerto sidecar in-process. Demuestra corpus read-only, resultado exacto, failure isolation y link SQLite explícito; no demuestra un proceso Python externo vivo, validez semántica/científica del contenido, operación productiva ni E2E live-sidecar. `certified_compatibility=false` se mantiene hasta evidencia separada.
