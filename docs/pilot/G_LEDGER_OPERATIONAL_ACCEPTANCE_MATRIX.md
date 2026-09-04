# Matriz del Piloto de Aceptación Operativa

Estado vigente: **`IN_PROGRESS`, 3/8, 37,5 %**. OPA-DEFECT-004 invalidó el candidato alojado `99fdaab`; el fix exacto `138eabe` fue calificado mediante PR privado draft #4, seis jobs success en el run `33838359314` y ZIP alojado `7160d31d…0162`, verificado independientemente con 2/2 harness externos. Un bootstrap nuevo creó `project:01a06adc-c603-7291-aabe-240d71809978`; Stage 1…3 y OPA-S01…OPA-S10 pasaron. El flujo endurecido conserva `RISK_MITIGATED` y `RISK_CLOSED`, termina v45 con checkpoint `CURRENT`, sesión cerrada y cero `open_risk_refs`. La evidencia 8/8 y la decisión anterior permanecen históricas, no se borran ni se transfieren. Stage 4…Stage 8 aún requieren evidencia viva; OPA-S31 sigue `EXCLUDED_BY_SCOPE` y G-Skills queda fuera de la validación de producto. No hubo Stage 4, merge, release ni nueva aceptación.

## Recalificación vigente y cierre histórico conservado

| Control | Resultado |
|---|---|
| Etapas | 1–3 acreditadas; 4–8 `REQUALIFICATION_REQUIRED` |
| Escenarios | S01…S10 `PASS`; S11…S30 y S32 requieren recalificación; OPA-S31 `EXCLUDED_BY_SCOPE` |
| Defectos | 4 `FIXED_AND_REQUALIFIED`; OPA-DEFECT-004 pasó ejecución viva Stage 3 |
| Integridad | `doctor`, `verify` y `audit`: `ok`; único finding esperado `CriticalUnknownOpen` para Stage 4 |
| Continuidad | v45; digest `sha256:0772e8fb…c337445`; checkpoint `checkpoint:843caa1c-1721-7d35-abe7-1590596b1824` `CURRENT` |
| Gates requeridos | 0 `VERIFIED`; 8 `PENDING`; 0 `WAIVED`; gate opcional también pendiente |
| Lifecycle | fase activa; proyecto `BLOCKED` por Stage 4; 0 fases completadas |
| Riesgo | `risk:opa-stage3-continuity` `CLOSED`; mitigación preservada; 0 `open_risk_refs` |
| Autoridad | Decisión previa preservada como histórica; nueva decisión pendiente después de Stage 8 |
| Evidencia vigente | `docs/pilot/results/stage-3-risk-close-hosted-requalification.json` |

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

La fórmula de cierre quedó satisfecha: ningún escenario crítico está `FAIL`, `BLOCKED` o `NOT_RUN`; no existen conflictos ni unknowns críticos abiertos; no hay waivers sobre gates requeridos; y la decisión humana separada está aceptada en el stream.
