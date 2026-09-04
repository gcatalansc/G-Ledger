# Matriz del Piloto de Aceptación Operativa

Estado vigente: **`IN_PROGRESS`, 6/8, 75 %**. El candidato exacto `138eabe`/`7160d31d…0162` continúa en `project:01a06adc-c603-7291-aabe-240d71809978`; Stage 1…6 y OPA-S01…OPA-S27 pasaron. Stage 6 demostró que Markdown es una vista regenerable, que portable y backup/restore conservan sus garantías contractuales, que corrupción, divergencia y destinos existentes fallan cerrados, y que los canaries no aparecen en las superficies soportadas sin convertirlo en una afirmación universal sobre bytes internos. Terminó v86 con checkpoint `CURRENT`, cero riesgos, cero unknowns críticos y cero findings. La evidencia 8/8 y la decisión anterior permanecen históricas, no se borran ni se transfieren. Stage 7 y Stage 8 aún requieren evidencia viva; OPA-S31 sigue `EXCLUDED_BY_SCOPE` y G-Skills queda fuera de la validación de producto. No hubo Stage 7, merge, release ni nueva aceptación.

## Recalificación vigente y cierre histórico conservado

| Control | Resultado |
|---|---|
| Etapas | 1–6 acreditadas; 7–8 `REQUALIFICATION_REQUIRED` |
| Escenarios | S01…S27 `PASS`; S28…S30 y S32 requieren recalificación; OPA-S31 `EXCLUDED_BY_SCOPE` |
| Defectos | 4 `FIXED_AND_REQUALIFIED`; OPA-DEFECT-004 pasó ejecución viva Stage 3 |
| Integridad | `doctor`, `verify` y `audit`: `ok`, 0 findings, 0 submissions sin resolver |
| Continuidad | v86; digest `sha256:63949449…872e45`; checkpoint `checkpoint:167b6373-8352-7747-ad36-042c618f295d` `CURRENT` |
| Gates requeridos | `gate:opa-04` `VERIFIED`; 7 `PENDING`; 0 `WAIVED`; sólo gate opcional `WAIVED` |
| Lifecycle | fase activa; proyecto `BLOCKED` por Stage 7; 0 fases completadas |
| Riesgo | `risk:opa-stage3-continuity` `CLOSED`; mitigación preservada; 0 `open_risk_refs` |
| Autoridad | Decisión previa preservada como histórica; nueva decisión pendiente después de Stage 8 |
| Evidencia vigente | `docs/pilot/results/stage-6-risk-close-hosted-requalification.json` |

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

La fórmula de cierre se satisfizo sólo para el candidato histórico invalidado. En la recalificación vigente aún no se satisface: OPA-S28…OPA-S30 y OPA-S32 requieren evidencia fresca, Stage 7 y Stage 8 permanecen pendientes, y no existe una nueva decisión humana transferible.
