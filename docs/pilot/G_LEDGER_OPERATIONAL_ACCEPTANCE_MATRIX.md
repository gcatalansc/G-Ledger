# Matriz del Piloto de Aceptación Operativa

Estado vigente: **`READY_FOR_OWNER_DECISION`, 7/8, 87,5 %**. El candidato exacto `138eabe`/`7160d31d…0162` fue conciliado en `project:01a06adc-c603-7291-aabe-240d71809978`: Stage 1…7 y OPA-S01…OPA-S30 pasaron; OPA-S31 conserva `EXCLUDED_BY_SCOPE`; OPA-S32 espera una decisión humana separada. Stage 8 predecisión leyó matriz, reporte, bitácora, `resume`, checkpoint, gates, `doctor`, `verify`, `audit` y siete manifiestos de etapa sin mutar el Ledger: v97→v97, checkpoint `CURRENT`, cero riesgos, unknowns críticos, conflictos, defectos abiertos, waivers críticos o findings. La evidencia 8/8 y la decisión anteriores permanecen históricas, no se borran ni se transfieren. No hubo decisión nueva, cierre de fase, merge, release ni aceptación vigente; G-Skills queda fuera de la validación de producto.

## Recalificación vigente y cierre histórico conservado

| Control | Resultado |
|---|---|
| Etapas | 1–7 acreditadas; 8 `AWAITING_OWNER_DECISION` tras conciliación read-only |
| Escenarios | S01…S30 `PASS`; S32 `AWAITING_OWNER_DECISION`; OPA-S31 `EXCLUDED_BY_SCOPE` |
| Defectos | 4 `FIXED_AND_REQUALIFIED`; OPA-DEFECT-004 pasó ejecución viva Stage 3 |
| Integridad | `doctor`, `verify` y `audit`: `ok`, 0 findings, 0 submissions sin resolver |
| Continuidad | v97; digest `sha256:044fa214…9eb350`; checkpoint `checkpoint:c85fb171-02ec-76d4-9990-366c3ac6f21d` `CURRENT` |
| Gates requeridos | `gate:opa-04` `VERIFIED`; 7 `PENDING`; 0 `WAIVED`; sólo gate opcional `WAIVED` |
| Lifecycle | fase activa; proyecto `BLOCKED` por decisión humana de Stage 8; 0 fases completadas |
| Riesgo | `risk:opa-stage3-continuity` `CLOSED`; mitigación preservada; 0 `open_risk_refs` |
| Autoridad | Decisión previa preservada como histórica; nueva decisión `human:owner` ausente y no prellenada |
| Lectura principal | `docs/pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md` |
| Evidencia técnica | `docs/pilot/results/stage-8-risk-close-owner-decision-readiness.json` y `operational-pilot-report.json` |

G-Skills permanece en desarrollo y **no fue validado como producto**. Su superficie local se utilizó únicamente como contraparte de desarrollo para probar las fronteras de integración de G-Ledger; no hubo contraparte externa, certificación bilateral ni aceptación operativa de G-Skills.

## Principios P-01…P-10

| Principio | Escenarios | Demostración exigida |
|---|---|---|
| P-01 Markdown es vista | S22, S26 | Divergencia detectada y regeneración desde stream |
| P-02 Eventos no se reescriben | S06, S09, S16, S24 | Historia y replay preservados |
| P-03 Código no completa fase | S07, S11, S13 | Gate y Phase Engine conservan autoridad |
| P-04 Evidencia != afirmación | S08, S11, S12 | ASSERTED/OBSERVED/VERIFIED_SOURCE separados |
| P-05 Autoridad por dominio | S05, S13, S15, S19, S29, S30 | Rechazo y no elevación |
| P-06 Lectura/escritura separadas | S04, S29, S30 | Queries/provider/export delta cero |
| P-07 Idempotencia/concurrencia | S12, S16…S21 | Replay, conflictos y reconcile |
| P-08 Integración por contratos | S12, S29, S30 | stdio/artifact exchange, sin DB compartida |
| P-09 Continuidad compacta | S10, S28, S32 | Resume y handoff ciego |
| P-10 IA opcional/no autoritativa | S05, S13, S28 | Codex registra; engines/humano deciden |

## Casos de uso UC-01…UC-17

| Caso | Cobertura del piloto | Escenarios |
|---|---|---|
| UC-01 Inicio desde spec | Ejecutada | S03 |
| UC-02 Avance material | Ejecutada | S08, S28 |
| UC-03 Verificación ejecutada | Ejecutada | S11, S12 |
| UC-04 Gate incompleto | Ejecutada | S11 |
| UC-05 Fase cerrable | Ejecutada | S13 |
| UC-06 Reapertura | Ejecutada | S14 |
| UC-07 Reanudación | Ejecutada | S10, S28 |
| UC-08 Contradicción heredada | Ejecutada mediante registros y supersesión | S06, S09 |
| UC-09 G-Skills consulta continuidad | Frontera local de G-Ledger; no valida G-Skills | S29 |
| UC-10 G-Skills registra resultado | Frontera local de G-Ledger; no valida G-Skills | S12, S29 |
| UC-11 GSP aporta evidencia | Runtime fijado local | S30 |
| UC-12 Radar importa overlay | Excluida | S31 |
| UC-13 Radar recibe export stale | Excluida | S31 |
| UC-14 Divergencia Radar | Excluida | S31 |
| UC-15 Radar reexpone portfolio | Excluida | S31 |
| UC-16 Checkpoint sin ambigüedad | Ejecutada | S10, S20, S24 |
| UC-17 Unknown outcome | Ejecutada | S20, S21 |

## Invariantes I-01…I-28

| Invariantes | Escenarios operativos | Evidencia complementaria permitida |
|---|---|---|
| I-01, I-02, I-20, I-27 | S16, S23, S24, S27 | Regresión executable del stream/rebuild |
| I-03 | S22 | Ninguna sustitución por Markdown |
| I-04, I-06, I-07 | S03, S16…S21 | CommandResult y versión observados |
| I-05, I-06, I-17 | S04, S29, S30 | Deltas de versión/digest |
| I-08, I-09, I-10 | S07, S11, S13 | Estados de gate/fase |
| I-11, I-12 | S08, S11, S12 | Provenance y EvidenceStatus |
| I-13, I-14 | S06, S09 | Originales visibles tras transición |
| I-15, I-22 | S10, S14, S32 | Freshness de checkpoint |
| I-16 | S01, S03, S29, S30 | IDs/digests exactos, sin fallback nominal |
| I-18 | S05, S13, S28 | Sin cierre por agente |
| I-19 | S26 | Salida cruda no versionada/persistida en superficies |
| I-21, I-28 | S01, S02, S29, S30 | Contratos/locks verificados |
| I-23 | S07, S13, S32 | Lifecycle derivado y cierre disciplinado |
| I-24, I-25 | S31 | Excluido; no se atribuye PASS |
| I-26 | S20, S21 | Outcome unknown separado del dominio |

## Familias públicas de CLI

| Familia | Escenarios |
|---|---|
| init/proyecto/catálogo | S03, S04, S07 |
| autoridad/repositorio/roadmap | S03, S04, S06, S14 |
| fases/gates | S03, S07, S11…S15 |
| registros/artifacts/claims/unknowns | S08, S09 |
| decisiones/blockers/risks/next/session | S08…S10 |
| evidence/reconcile | S11, S12, S16…S21 |
| resume/checkpoint | S10, S20, S24, S28, S32 |
| render/audit/verify | S22, S26, S27 |
| export/import/backup/restore | S23…S25 |
| provider/RecordSink | S12, S29 |
| GSP | S30 |
| Radar | S31 `EXCLUDED_BY_SCOPE` |

## Regla de cierre

Una fila cubierta por evidencia histórica sigue pendiente hasta que su escenario `LIVE_PILOT`, `LIVE_HANDOFF` o `LIVE_LOCAL_INTEGRATION` correspondiente se ejecute. Los negativos sólo pasan cuando el rechazo y el delta cero coinciden con la especificación.

La conciliación técnica de la fórmula pasó para el sujeto vigente: 30 escenarios previos `PASS`, OPA-S31 excluido por alcance, cuatro defectos corregidos y recalificados, cero defectos abiertos, conflictos, unknowns críticos, riesgos abiertos, waivers críticos o findings. La fórmula completa aún **no** se satisface porque OPA-S32 y Stage 8 requieren una decisión humana nueva, explícita y no transferible. Hasta entonces el máximo resultado es `READY_FOR_OWNER_DECISION`, el contador permanece 7/8 y los siete gates requeridos pendientes no se suavizan.
