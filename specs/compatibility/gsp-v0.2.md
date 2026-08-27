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

La conformance local S11 usa los contratos/fixtures upstream y un puerto sidecar in-process. Demuestra corpus read-only, resultado exacto, failure isolation y link SQLite explícito. Su reporte histórico se conserva sin reinterpretarlo como ejecución externa.

RQ4 añade evidencia separada de un proceso Python externo que ejecuta el compilador real fijado, sin provider de respuestas, modelos ni red durante la compilación. El adapter opcional `integrations/gsp/sidecar/compile_bundle.py` recibe sólo archivos, no una DB Ledger. Exige checkout limpio/pin exacto y schemas originales; limita la entrada a 1 MiB y la salida a 8 MiB, y nunca sobrescribe un resultado existente. Una salida incompleta no se considera evidencia válida: debe pasar `validate-result`.

El [job hosted de RQ4](https://github.com/gcatalansc/G-Ledger/actions/runs/33060749387/job/98478615297), en el SHA Ledger `30c17ac0253a581db69b7fe38e001da07c9216d8`, ejecutó tres tests reales y los 18 tests upstream de API/compilador/contratos, además de hashes/contratos/fixtures. En SQLite desechable, la terminación OS del proceso propio tras entrar a `compile_evidence` conservó cadena/proyección y cero eventos adicionales. Un proceso nuevo produjo los seis artefactos aceptados; el enlace explícito añadió uno y su replay cero. La prueba local Windows también pasó; el entorno hosted fue Ubuntu 24.04, imagen `20260816.277.1`, Python 3.12.14 y uv 0.11.16.

Esta calificación acotada retira únicamente el waiver de caos `live-sidecar-process-crash`. No demuestra toda la conformidad runtime, supervisión de un daemon, calidad semántica/científica, operación productiva ni compatibilidad universal. `certified_compatibility=false` se mantiene; quedan pendientes la conformidad runtime más allá del ensayo de crash y la revisión explícita de certificación.
