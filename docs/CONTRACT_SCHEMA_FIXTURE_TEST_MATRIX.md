# Matriz contrato → schema → fixture → prueba

Esta matriz resume la cobertura contractual de S1, la ejecución transaccional de S2, la proyección determinista de S3, lifecycle/catálogo de S4, gates/evidencia/Phase Engine de S5, continuidad/render/audit de S6, portable/legacy import de S7, protocolo/SDK genérico de S8, provider G-Skills exacto de S9 y RecordSink G-Skills de S10. La relación individual y canónica de los 46 schemas con sus 92 fixtures Ledger está en `schemas/v0.2/schema-registry.json` y `schemas/v0.2/fixtures/manifest.json`; ambos ficheros son generados de forma determinista y se verifican byte a byte. Los once schemas/22 fixtures externos seleccionados por S9–S10 viven bajo `integrations/gskills/contracts/v0.3` y se fijan por digests upstream separados.

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
| REQ-INT-001, I-28 | `integration-protocol/tests/contracts.rs`, `cli/tests/contract_commands.rs` | Locks completos requieren spec/schema/fixtures/range; null/missing queda `PENDING`; overclaim falla; los cuatro locks actuales siguen no certificados |
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

## Cobertura ejecutable S10

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| REQ-SNK-001/009, AC-042, §34.3/34.6 | `interop-gskills/tests/upstream_contracts.rs`, `sink_mapping.rs` | Once schemas/22 fixtures exactos; sólo cinco capabilities con variante/payload/schema canónicos; legacy `*_REFERENCE` y lifecycle directo fallan cerrados |
| REQ-SNK-003/004/012/013, §40–41 | `sink_mapping.rs`, `record-sink-field-mapping.json` | Policy explícita, write trust separado, expected state, scope cross-system, privacidad y ocho digests de aprobación; validate produce cero ejecuciones |
| REQ-SNK-002/008/011, AC-039/040, §34.7–34.8 | `sink_protocol.rs`, `record-sink-conformance.json` | Un efecto ante replay, conflicto de key/digest y versión sin overwrite, signal→command explícito, mapping exacto de `CommandResult` y outcome incierto fuera del resultado |
| §33.5, AC-003/050, Fase 7 | `sink_stdio_e2e.rs`, `cli/tests/gskills_sink.rs` | Cinco métodos framed, failure isolation, SQLite real sólo en composition root, policy obligatoria y status/reconcile sin submit |
| §34.9, Fase 7 | `record-sink-loss-report.json`, `specs/compatibility/gskills-v0.3.0.md` | S10 local PASS conserva `certified_compatibility=false` hasta counterpart y E2E bilateral |

El gate integral S10 vuelve a ejecutar el workspace completo. La evidencia local demuestra provider y RecordSink exactos dentro de Ledger; no certifica implementación counterpart, integración bilateral, sandbox universal, operación productiva, rentabilidad ni validez científica.

## Cobertura ejecutable S11

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| AC-052/053, REQ-INT-001, §35.2 | `interop-gsp/tests/upstream_contracts.rs`, `integration gsp verify-contracts` | Tag/commit/digests exactos; 28 contratos, 31 schemas y 140 fixtures raw verificados/replay offline; un byte divergente falla antes del replay |
| AC-057, §35.3–35.4 | `interop-gsp/tests/corpus.rs`, `conformance.rs`, `gsp-field-mapping.json`, `gsp-loss-report.json` | Scope por refs, budgets, allowlist, seis directorios, no timeline/raw output/paths; EvidenceBundle upstream completo y determinista, unido al manifest por digest canónico |
| AC-054/055/060, PL-12/13 | `interop-gsp/tests/bridge.rs`, `conformance.rs` | Seis outputs exactos, request identity y digests; partial/fail, contradicciones e incertidumbre visibles; claims GSP no adquieren lifecycle authority |
| AC-056/058/059, REQ-INT-003 | `bridge.rs`, `cli/tests/gsp_bridge.rs` | Sidecar sin storage/Python/shared DB; error o receipt con state binding falso producen 0 eventos; export/validate read-only; link explícito produce un evento y replay cero adicionales |
| Fase 8 gate | `gsp-conformance.json`, `upstream-lock.json`, `specs/compatibility/gsp-v0.2.md` | Degradación semántica 0 y elevación de autoridad 0; lock local COMPLETE con `certified_compatibility=false` hasta E2E live-sidecar |

El gate integral S11 vuelve a ejecutar el workspace completo. La evidencia demuestra exactitud contractual y aislamiento local; no certifica un proceso GSP Python externo vivo, validez semántica/científica, operación productiva, rentabilidad ni compatibilidad runtime certificada.

## Cobertura ejecutable S12

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| PL-21, PL-31, AC-065/066 | `interop-radar/tests/profile_contracts.rs`, `export.rs` | Cinco schemas/diez fixtures con hash crudo y replay offline; export canónico, privacy mínima, sin tombstones, DB compartida, red ni IndexedDB |
| PL-22/23, AC-067–069 | `reference_importer.rs`, `radar-project-link.schema.json` | Sólo link confirmado con uno de cinco métodos fuertes permite apply; nombre/alias produce candidato y nunca identity automática |
| PL-24–27, AC-070–074 | `reference_importer.rs` | Duplicate, stale, higher-version y same-version divergence son resultados distintos; project omission es NO_ACTION y no existe clear/delete/last-write-wins |
| PL-28, AC-075/076 | `reference_importer.rs`, `radar-field-mapping.json` | Priority/tags/notes/ideas/chats/tasks/artifacts/presentation permanecen consumer-owned y byte-equivalent en preview/conflicto |
| PL-29/30, AC-077/078 | `export.rs`, `radar-conformance.json` | Tabla de status cerrada; combinación no normativa falla; porcentaje sólo con denominador lineal completo y de otro modo unavailable con razón |
| PL-32, AC-079 | `reference_importer.rs`, `radar-loss-report.json` | Provenance Ledger se conserva y Radar no se convierte en fuente independiente de lifecycle |
| Fase 9, AC-080 | `cli/tests/radar_workflow.rs`, `upstream-lock.json`, `specs/compatibility/project-radar-v0.1.md` | Export/validate/catalog sobre SQLite real añaden cero eventos y rechazan overwrite/tampering; lock local COMPLETE pero counterpart/E2E/certificación siguen pendientes |

El gate integral S12 vuelve a ejecutar el workspace completo. La evidencia demuestra el lado Ledger y un evaluador de referencia sin persistencia; no certifica código Project Radar, IndexedDB, migración/backup counterpart, integración bilateral, operación productiva, rentabilidad ni validez científica.

## Cobertura ejecutable S13

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| AC-061, §36.1/36.4 | `agent-skill/tests/package_contract.rs`, `conformance.rs`, `integration agent-skill verify-contracts` | Manifest/profile, dos schemas, 7 triggers, 5 políticas, examples y raw locks validan offline; un byte divergente falla antes de semántica |
| AC-062, §36.3 | `conformance.rs`, `SKILL.md`, `manifest.json` | Placeholders son válidos; `project:<uuid>`/`checkpoint:<uuid>` concretos en instrucciones se rechazan y el manifest fija `state_authority=false` |
| AC-063, §36.2/36.4 | `conformance.rs`, `trigger-cases.json`, `policy-cases.json` | Tool ausente no genera writes; version conflict exige resume/replan y outcome unknown exige reconcile sin blind retry |
| AC-064, ADR-012 | `conformance.rs`, `workflow.json` | Completion pertenece al engine; direct DB, Markdown generado, gate verified y phase complete están prohibidos; mutación adversarial se rechaza |
| Fase 10 handoff E2E | `cli/tests/agent_skill_workflow.rs` | Init→resume→checkpoint→replay→resume usa SQLite real; resume añade cero eventos, replay cero adicionales y stale expected version no altera el stream |

El gate integral S13 vuelve a ejecutar el workspace completo. Esta evidencia demuestra conformance estructural/conductual local del procedimiento y el handoff CLI; no prueba obediencia de todos los agentes, certificación externa, sandbox universal ni readiness de release S14.

## Cobertura ejecutable S14

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| AC-081/082 | `release/tests/hardening.rs`, `docs/THREAT_MODEL.md` | Repo no confiable obtiene cero capacidades; programa/cwd/argv literal no construyen shell ni conceden authority/network/hooks |
| AC-083/085, REQ-PRV-001 | `release/tests/hardening.rs`, `release/privacy/corpus-manifest.json` | Siete canales raw-locked; secret/transcript/reasoning/raw output se rechazan sin eco del valor; resultado declarado cero |
| AC-084/087 | `portable-bundle/tests/export.rs`, `interop-radar/tests/export.rs`, `schemas/tests/registry.rs` | Exports mínimos y refs offline siguen cubiertos por conducta real previa y el gate agregado |
| AC-086 | `.github/workflows/ci.yml`, `release/acceptance-report.json` | Windows/Linux/macOS y dependency-policy hosted PASS sobre el SHA exacto `5d34294c1f06b59e635c84d734b3156950940ae5`, run `33031982562`; sin waiver de aceptación |
| AC-088, PL-01…32 | `release/acceptance-report.json`, tests referenciados | Inventario cerrado exige exactamente 90 AC y 32 PL, sin duplicados ni evidencia inexistente |
| AC-089 | `benchmark/project-ledger-v0.2.0-windows-x86_64.json`, `xtask/tests/release_command.rs` | Nueve métricas medidas pasan en entorno declarado; peak-RSS p95 17,855 MiB mediante máximo RSS muestreado cada 1 ms sobre el proceso release real |
| AC-090, REQ-INT-002/REQ-REL-001 | `release/compatibility-report.json`, `release/tests/release_gate.rs` | Cuatro perfiles locales completos, pending evidence visible y cero certificaciones |
| REQ-REL-002 | `release/migrations/v0.1-to-v0.2.json`, `legacy-import/tests/review.rs` | Migración explícita hacia store nuevo, source preservado, sin reinterpretación silenciosa y rollback por descarte del destino |
| §45.7 disk full | `storage-sqlite/src/disk_full_tests.rs`, `storage-sqlite/tests/disk_full.rs`, `release/chaos-report.json` | Cuota SQLite determinista y kernel ENOSPC real en ext4/loopback acotado; append fail-closed, rollback completo, digest previo preservado y reapertura WAL/quick_check/cadena válidos |
| §45.7 inventario de caos | `release/tests/release_gate.rs`, `cli/tests/release.rs` | Inventario hash-locked obligatorio, escenarios únicos, evidencia existente y scopes GA/GSP runtime separados; quitar el lock, inventar escenario o perder evidencia falla cerrado |
| Packaging/SBOM/checksums | `release/package-manifest.json`, `sbom.spdx.json`, `checksums.sha256` | Binario default único/offline, grafo Cargo inventariado y artefactos reproducibles ligados por SHA-256 |

El gate S14 representa un engineering release candidate. AC-086/AC-089 están calificados; el cierre de un waiver de caos no ejecuta una promoción GA automática. La evidencia counterpart/bilateral/live pendiente mantiene `certified_compatibility=false`, y `live-sidecar-process-crash` sigue bloqueando la certificación runtime GSP.
