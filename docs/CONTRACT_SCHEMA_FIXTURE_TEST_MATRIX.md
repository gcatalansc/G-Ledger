# Matriz contrato → schema → fixture → prueba

Esta matriz resume la cobertura contractual de S1, la ejecución transaccional de S2, la proyección determinista de S3, lifecycle/catálogo de S4, gates/evidencia/Phase Engine de S5, continuidad/render/audit de S6, portable/legacy import de S7, protocolo/SDK genérico de S8 y provider G-Skills exacto de S9. La relación individual y canónica de los 46 schemas con sus 92 fixtures Ledger está en `schemas/v0.2/schema-registry.json` y `schemas/v0.2/fixtures/manifest.json`; ambos ficheros son generados de forma determinista y se verifican byte a byte. Los seis schemas/12 fixtures externos seleccionados por S9 viven bajo `integrations/gskills/contracts/v0.3` y se fijan por digests upstream separados.

| Familia contractual | Contratos tipados | Schemas | Fixtures | Pruebas principales |
|---|---:|---:|---:|---|
| Protocolo de comandos/eventos | 53 comandos, 67 eventos y 4 envelopes/resultados auxiliares | 6 | 12 | `closed_unions.rs`, `command_event_matrix.rs`, `fixtures.rs` |
| Proyecto, lifecycle y evidencia | 20 raíces durables | 20 | 40 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Continuidad, import y export | 9 raíces durables | 9 | 18 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Integración genérica | 6 raíces durables | 6 | 12 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Perfil Project Radar 0.1.1 | 5 raíces durables | 5 | 10 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| **Total de raíces registradas** | **46** | **46** | **92** | `generation.rs`, `contract_commands.rs` |

## Invariantes automatizados

- Cada raíz tipada produce exactamente una entrada de registro con `$id` único.
- Cada entrada del registro tiene exactamente un fixture positivo y uno negativo.
- Todos los `$ref` son fragmentos internos resolubles sin red.
- Los positivos validan y los negativos fallan por semántica closed-world.
- La matriz contiene exactamente una fila por cada uno de los 53 comandos y asigna los 67 eventos a un origen de comando o a un owner derivado.
- La regeneración produce 142 ficheros idénticos, JSON con LF final y digests canónicos estables.

## Cobertura ejecutable S2

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| I-01, I-27, AC-006, AC-008 | `event_chain.rs`, `recovery.rs` | Génesis, append-only, versiones contiguas, previous digest, roundtrip y corrupción fail-closed |
| I-04 y §15–16 | `integrity.rs`, `command_transaction.rs` | Toda escritura canónica nace de `CommandEnvelope`; payload/envelope/result usan digest canónico |
| I-06, AC-038 | `command_transaction.rs` | Expected version stale produce resultado persistente y cero append |
| I-07, AC-036–037 | `command_transaction.rs` | Mismo scope/key/digest: un efecto; digest distinto: rechazo cerrado |
| Matriz comando-evento, AC-039 | `command_transaction.rs` | Un handler no puede emitir un evento fuera del outcome/possible set cerrado |
| I-26, PL-08, PL-20, AC-040–041 | `submission_reconciliation.rs` | `OUTCOME_UNKNOWN` sólo local; historial CAS; accepted/rejected/not-found sin reintento ciego |
| AC-001 y §39–40 | `init.rs`, `bootstrap.rs`, `recovery.rs` | Init offline, SQLite/WAL real, migración idempotente, rollback y recovery report |

## Cobertura ejecutable S3

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| I-02, I-20, AC-007 | `project_state.rs`, `incremental_equivalence.rs` | Mismo stream produce mismo estado/digest; incremental y rebuild desde cero son equivalentes |
| I-27 y §13/§45 | `project_state.rs`, `incremental_equivalence.rs` | Eventos y estado se validan antes de aplicar; gaps, cross-project, génesis duplicada y estado corrupto fallan cerrados |
| I-05, PL-09, AC-009 | `project_state_query.rs`, `projection_cache.rs` | 100 consultas mantienen versión y conteo de eventos; sólo puede cambiar la caché derivada |
| §39–40, AC-010 | `bootstrap.rs`, `projection_cache.rs`, `project_state_query.rs` | Migración v2 verificable; caché ausente/stale/corrupta se descarta, reconstruye y repara desde el stream |
| Orden S3/S4 de §51 | `incremental_equivalence.rs` | El entry point stateless de S3 rechaza un evento lifecycle; el cursor S4 es la única ruta que conserva sus índices privados |

## Cobertura ejecutable S4

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| I-09, I-13, I-14, §17, AC-028 | `lifecycle_projection.rs`, `lifecycle_commands.rs` | Roadmap secuencial, una fase activa, reapertura explícita, precedencia de status y rechazo de streams con derivación obsoleta |
| §10 y §17.1 | `lifecycle_projection.rs`, `lifecycle_commands.rs` | Autoridad ordenada, mismatch sin reemplazar digest, conflicto crítico y bloqueo de inicio de fase |
| AC-022, AC-023 | `continuity_records.rs`, `lifecycle_rebuild.rs` | Decisión superseded deja de ser vigente; blocker/risk conservan historia en eventos y actualizan refs derivados |
| §24, AC-026 | `continuity_records.rs`, `lifecycle_rebuild.rs` | Riesgo no bloquea; acciones `AUTHORIZED/ELIGIBLE/RECOMMENDED/PROPOSED` se ordenan por policy y autoridad verificable |
| I-02, I-05, I-20, I-23 | `lifecycle_rebuild.rs`, `lifecycle_transaction.rs` | Comandos S4 consumen el stream verificado, append atómico, rebuild exacto y reparación de caché sin mutar verdad canónica |
| §9.2, §14.18, §39.6 | `project-catalog/tests/rebuild.rs` | Catálogo puro, ordenado, reconstruible, sin duplicados ni discovery implícito |
| Regresión de identidad | `lifecycle_commands.rs` | Eventos derivados de comandos UUIDv7 adyacentes mantienen IDs distintos y deterministas |

## Cobertura ejecutable S5

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| I-08, AC-014, REQ-GAT-001 | `gates_and_evidence.rs`, `gate_commands.rs` | Sólo una evaluación PASS estructural, con definition digest y evidencia `VERIFIED_SOURCE`, puede preceder `GATE_VERIFIED`; evaluaciones forjadas fallan cerradas |
| I-09, I-10, AC-013, AC-021 | `gate_commands.rs`, `gate_lifecycle_transaction.rs` | Evidencia verde sólo se registra; fase con gate requerido pendiente rechaza cierre; el Phase Engine humano explícito es el único owner de completion |
| AC-015 | `gate_commands.rs` | Waiver conserva actor/reason/scope/review/authority/version/digest y actor submitter coincidente |
| I-11, AC-018, AC-019 | `evidence_runner.rs`, `evidence_import.rs` | Programa+argv sin shell, allowlist, exit/duración/digest, resumen redactado/acotado, raw off e import unsigned nunca promovido automáticamente |
| I-12, I-18, AC-020 | `gate_commands.rs` | Claim de agente nace `ASSERTED`; `VERIFIED` exige humano y evidencia aceptada |
| I-15, AC-016, AC-017, PL-05 | `gates_and_evidence.rs`, `gate_lifecycle_transaction.rs` | Invalidación, cambio material de autoridad y refresh de snapshot producen stale/revalidation; evidencia invalidada reabre fase completa con provenance en reason |
| §21.3–21.4 | `predicate_ast.rs`, `gate_commands.rs` | Los 17 operadores cerrados se evalúan sin scripts; `ALL/ANY` se auditan con la misma semántica en motor y proyección; missing/tipos incompatibles/snapshot conflict nunca se convierten en true |
| I-02, I-20 | `gates_and_evidence.rs`, `gate_lifecycle_transaction.rs` | Índices privados de gate/evidence/claim/repository reconstruyen el mismo estado; SQLite conserva lotes atómicos y cadena íntegra |

## Cobertura ejecutable S6

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| REQ-CHK-001, I-22, AC-027, PL-18 | `checkpoints.rs`, `checkpoint_engine.rs` | El checkpoint captura exactamente estado/digest/material N, referencia el evento real N+1, no muta sus bytes y falla ante vectors forjados o múltiples repositorios vigentes |
| §25.3–25.5, AC-025 | `checkpoint_engine.rs`, `continuity_transaction.rs` | Freshness append-only distingue project-only/current/authority/repository/conflict/invalidated/superseded; unknown+invalidación se persisten en un lote SQLite |
| §26.1–26.2, AC-024, AC-033 | `resume.rs` | Salida compacta ≤220 y agent Markdown ≤450 según cota UTF-8; stale status y unknown/conflict crítico conservan prioridad |
| I-03, §28, AC-029–030, PL-14 | `markdown_audit.rs` | Warning y trece secciones exactas, LF/digest deterministas, render read-only, perfiles de privacidad y edición manual sin mutación canónica |
| §30.1–30.3, AC-031–033 | `markdown_audit.rs` | Rebuild de stream, source version/digest y bytes verificables; modificación produce `GENERATED_ARTIFACT_MODIFIED`, regeneración restaura MATCH y hallazgos críticos permanecen visibles |

## Cobertura ejecutable S7

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| AC-005, PL-15, §31.1–31.2 | `portable-bundle/tests/export.rs`, `import.rs` | Árbol fijo, bytes canónicos y repetibles, manifest/checksums/schema lock, no overwrite, privacy fail-closed y roundtrip desde directorio |
| AC-005, §31.5 | `portable-bundle/tests/import.rs`, `storage-sqlite/tests/portable_import.rs` | Checksums, cadena, identity, continuidad y estado reconstruido se verifican antes de new-project/fast-forward SQLite atómico |
| AC-035, §31.3–31.4 | `portable-bundle/tests/import.rs`, `storage-sqlite/tests/portable_import.rs` | Identical es idempotente; destino más nuevo no se sobrescribe; siblings divergen sin merge; fork explícito devuelve target+provenance y no muta el stream original |
| REQ-REL-002, §31.5 | `portable-bundle/src/export.rs`, `import.rs` | Sólo v0.2.0 se interpreta; otro schema exige migración explícita y no se reetiqueta silenciosamente |
| AC-034, §29, ADR-014 | `legacy-import/tests/review.rs` | Preview por línea con conflictos preservados, toda recomendación `REVIEW`, decisión exacta por candidato, `TRUST_ALL` inválido y proposals sólo `ASSERTED` |

## Cobertura ejecutable S8

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| §32.4–32.5, REQ-INT-002 | `integration-protocol/tests/contracts.rs` | Manifest/health/envelope/loss canónicos, side effects declarados, payload/state/schema/digest ligados, bounds y current/stale sin elevar autoridad |
| REQ-INT-001, I-28 | `integration-protocol/tests/contracts.rs`, `cli/tests/contract_commands.rs` | Locks completos requieren spec/schema/fixtures/range; null/missing queda `PENDING`; overclaim falla; los tres locks actuales siguen no certificados |
| AC-045, PL-17 | `integration-protocol/tests/contracts.rs`, `conformance.rs` | Sólo mapping exacto `CONFIRMED`; candidate/nombre no resuelven y múltiples targets producen ambigüedad fail-closed |
| AC-043, PL-09, §38.5 | `integration-protocol/tests/conformance.rs` | 100 projections current + stale/post-failure conservan exactamente version/event-count/state-digest; mutación, timeout, nondeterminismo o secreto tipado fallan |
| AC-044/050, §38.5 | `integration-protocol/tests/conformance.rs` | 7 requests maliciosos cubren negotiation, budgets, duplicados, authority, privacidad e identidad; el provider recupera una respuesta válida después de errores |
| §33.5, AC-003 | `transport-stdio/tests/framing.rs`, `cli/tests/contract_commands.rs` | `Content-Length` estricto, bounded UTF-8 JSON, malformed/trailing fail-closed y manifest CLI/direct/stdio semánticamente equivalente |
| §32.2, §38.5 | `artifact-exchange/tests/artifacts.rs` | Artifact JSON canónico/digest-bound, filename/root acotado, no traversal, tampering, noncanonical, oversized ni overwrite; publicación atómica no-clobber |
| REQ-INT-003, AC-051 | manifests Cargo y boundaries públicas de las tres crates | Protocolo/transporte/exchange no dependen de storage-sqlite, WAL, tablas ni migraciones privadas; adapters específicos dependen hacia adentro |

## Cobertura ejecutable S9

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| REQ-INT-001/003, AC-042, §34.2–34.3 | `interop-gskills/tests/upstream_contracts.rs`, `integration gskills verify-contracts` | Seis schemas/12 fixtures exactos, IDs/digests/registry/manifest/lock recíproco fijados; el adapter no depende de storage-sqlite |
| REQ-CNT-005, PL-10, §34.4 | `interop-gskills/tests/provider.rs`, `provider-field-mapping.json` | State/material/digest, fases, gates separados, acciones, checkpoint N/N+1, repository snapshot, unknowns/conflicts y progreso se preservan sin reescritura taxonómica |
| REQ-PVD-008, AC-045, PL-17 | `interop-gskills/tests/provider.rs`, `provider-loss-report.json` | Metadata de fase faltante/contradictoria y múltiples bindings current fallan cerrados; la solicitud completa tiene cero pérdida material |
| AC-043–045, AC-050, §34.5/34.8 | `interop-gskills/tests/conformance.rs` | 100 current + stale/post-failure, nueve requests hostiles, bounds/privacy/identity/authority y cero mutaciones observables |
| AC-003, §33.5, Fase 6 | `interop-gskills/tests/stdio_e2e.rs`, `cli/tests/gskills_provider.rs` | Manifest/health/projection directos y framed equivalentes; SQLite sólo en composition root; profile alias y métodos mutables rechazados |
| §34.9, Fase 6/Fase 7 | `provider-conformance.json`, `specs/compatibility/gskills-v0.3.0.md` | S9 read profile PASS permanece distinto de RecordSink S10, E2E bilateral y `certified_compatibility=false` |

El gate integral S9 ejecuta 162 pruebas, todas en verde. La evidencia local demuestra el provider read-only exacto; no certifica RecordSink, implementación counterpart, integración bilateral, sandbox universal, operación productiva, rentabilidad ni validez científica.
