# Especificación del Piloto de Aceptación Operativa de G-Ledger v1.0.0

| Campo | Valor |
|---|---|
| Perfil | `project-ledger-operational-pilot/1` |
| Producto bajo prueba | Project Ledger 0.2.0 |
| Estado inicial | `NOT_STARTED` |
| Resultado admisible | `OPERATIONALLY_ACCEPTED`, `BLOCKED` o `REJECTED` |
| Plataforma primaria | Windows / PowerShell |
| Autoridad de producto | `specs/PROJECT_LEDGER_SPEC_v0.2.0.md` |
| Autoridad del piloto | Este documento |
| Datos operativos | `.tooling/g-ledger-operational-pilot/` |

## 1. Propósito

Este piloto DEBE determinar si G-Ledger puede adoptarse para gobernar el primer proyecto real. El proyecto del piloto es una campaña de validación de G-Ledger sobre sí mismo; NO implementa una aplicación de muestra ni utiliza un proyecto productivo como sujeto descartable.

El piloto DEBE comprobar el recorrido longitudinal de una bitácora automatizada de alto nivel: identidad, autoridad, trabajo material, conocimiento, evidencia, gates, lifecycle, continuidad, concurrencia, reconciliación, proyecciones, privacidad, recuperación y handoff entre agentes.

Una aprobación del piloto NO cambia `ENGINEERING_RELEASE_CANDIDATE`, NO constituye GA y DEBE conservar `certified_compatibility=false`.

## 2. Orden de autoridad

1. `specs/PROJECT_LEDGER_SPEC_v0.2.0.md` gobierna contratos, comandos, eventos e invariantes del producto.
2. Esta especificación gobierna alcance, escenarios, gates y decisión del piloto.
3. `docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md` gobierna la trazabilidad de cobertura.
4. `docs/pilot/results/operational-pilot-report.json` registra resultados observados; nunca modifica requisitos.
5. El plan de ejecución gobierna orden, pruebas y commits, pero no puede dispensar un gate.

Ante contradicción se aplica el documento de mayor autoridad. Los resultados históricos prepiloto son contexto, no evidencia `LIVE_PILOT`.

## 3. Sujeto, aislamiento y reproducibilidad

El sujeto inicial DEBE ser el ZIP alojado:

- ruta local: `dist/project-ledger-0.2.0-windows-x86_64.zip`;
- SHA-256: `a83624f2e795c8b8d3d9e15a316be38cce8999e437731b73f5f8c8964963ad80`;
- commit fuente: `b171f51cb58c60ed8c8a99b60386117f9fed6822`;
- versión esperada: `0.2.0`.

El binario de `target/` NO PUEDE sustituir al artefacto. La extracción DEBE ocurrir en un destino nuevo con espacios y caracteres no ASCII. La base, fixtures generados, backups, restores, exports y salida cruda DEBEN permanecer bajo `.tooling/g-ledger-operational-pilot/` o un temporal nuevo y no se versionarán.

El repositorio G-Ledger sólo puede recibir especificación, runner, fixtures no sensibles, evidencia sanitizada, bitácora y correcciones verificadas.

## 4. Actores y autoridad

| Actor | Confianza | Facultades |
|---|---|---|
| `human:owner` | `LOCAL_USER` | Bootstrap explícito, aceptación de decisiones, waiver opcional y decisión final |
| `agent:codex-pilot` | `TRUSTED_FOR_RECORDING` | Progreso, evidencia, evaluaciones solicitadas y continuidad |
| `agent:codex-handoff` | `TRUSTED_FOR_RECORDING` | Reanudación ciega y un avance material acotado |
| `agent:gskills-pilot` | `TRUSTED_FOR_RECORDING` mediante policy | Cinco comandos RecordSink permitidos, sin lifecycle |
| GSP fijado | Fuente externa de artefactos | Compilar y producir receipt; sin autoridad sobre gates, fases o proyecto |

Todo uso de `--as-human` DEBE mostrar antes el comando, actor, intención, digest y alcance. Un rechazo de autoridad NO PUEDE resolverse añadiendo el flag desde un flujo agente.

## 5. Clases de evidencia y estados

Clases permitidas:

- `LIVE_PILOT`: proceso real contra el ZIP sujeto y la DB descartable.
- `LIVE_HANDOFF`: tarea Codex nueva, sin contexto narrativo previo.
- `LIVE_LOCAL_INTEGRATION`: provider, RecordSink o GSP local fijado.
- `EXECUTABLE_REGRESSION`: prueba del repositorio ejecutada en el turno actual.
- `HISTORICAL_CONTEXT`: evidencia previa; nunca cierra por sí sola un escenario.
- `EXCLUDED_BY_SCOPE`: capacidad expresamente no ejecutada; nunca equivale a PASS.

Estados de escenario: `NOT_RUN`, `PASS`, `FAIL`, `BLOCKED`, `REJECTED_AS_EXPECTED` y `EXCLUDED_BY_SCOPE`. Sólo `PASS` y, en negativos definidos, `REJECTED_AS_EXPECTED`, satisfacen cobertura ejecutada.

## 6. Requisitos transversales

- **OPA-001 — Identidad exacta:** cada resultado se liga al hash del ZIP, commit fuente, versión, proyecto y state version observados.
- **OPA-002 — CLI autoritativa:** no se abre SQLite directamente ni se editan vistas generadas.
- **OPA-003 — Historia preservada:** resolución, mitigación y supersesión conservan los registros anteriores.
- **OPA-004 — Lectura sin efectos:** toda consulta comparará versión y digest antes/después.
- **OPA-005 — Escritura gobernada:** toda mutación usa actor, tiempo, expected version e idempotency key explícitos.
- **OPA-006 — Resultado incierto:** se reconcilia el scope original; queda prohibido reenviar con una clave nueva.
- **OPA-007 — Evidencia no es afirmación:** `OBSERVED` y exit 0 no verifican gates que exigen `VERIFIED_SOURCE`.
- **OPA-008 — Engines propietarios:** ningún actor externo declara gate verificado o fase completa.
- **OPA-009 — Freshness visible:** cambios materiales invalidan checkpoint y dependencias cuando corresponde.
- **OPA-010 — Privacidad acotada:** canaries ficticios no aparecen en superficies soportadas; no se promete ausencia universal en bytes internos.
- **OPA-011 — Recuperación no destructiva:** import/restore usan destinos nuevos y rechazan corrupción, divergencia y overwrite.
- **OPA-012 — Continuidad real:** una tarea nueva reconstruye estado y siguiente acción sólo mediante G-Ledger.
- **OPA-013 — Integraciones sin elevación:** provider es read-only; RecordSink queda allowlisted; GSP enlaza evidencia sin lifecycle.
- **OPA-014 — Cierre humano:** ningún proceso automático emite la aceptación operacional final.
- **OPA-015 — Evidencia fresca:** una etapa no se cierra con resultados de otra etapa o de la preparación prepiloto.
- **OPA-016 — Sin degradación:** umbrales, gates críticos y contratos no se reducen para obtener PASS.

## 7. Roadmap y gates del piloto

El roadmap contiene ocho fases ordenadas `phase:opa-01` a `phase:opa-08`. Cada fase tiene un gate requerido con waiver prohibido:

| Gate | Criterio de cierre |
|---|---|
| `gate:opa-01-identity` | Artefacto exacto, capacidades e inicialización verificadas |
| `gate:opa-02-governance` | Autoridad, binding, roadmap, lifecycle y lecturas sin efectos |
| `gate:opa-03-continuity` | Registros materiales, checkpoint y resume completo |
| `gate:opa-04-evidence` | Fail-closed de OBSERVED, VERIFIED_SOURCE por sink, gate y revalidación |
| `gate:opa-05-transaction` | Replay, conflictos, permisos y reconciliación sin duplicados |
| `gate:opa-06-recovery` | Render, portable, backup/restore, privacidad y audit |
| `gate:opa-07-handoff` | Handoff ciego, provider/RecordSink local y GSP fijado |
| `gate:opa-08-adoption` | Matriz completa, cero bloqueos críticos y decisión humana final |

Un gate opcional separado, `gate:opa-waiver-exercise`, DEBE demostrar la ceremonia de waiver humano. Ese waiver NO cuenta para ninguno de los ocho gates requeridos.

## 8. Escenarios obligatorios

### Etapa 1 — identidad y preflight

- `OPA-S01`: hash, source commit, versión y recursos coinciden.
- `OPA-S02`: las capacidades requeridas existen en `--help`; una capacidad ausente bloquea.
- `OPA-S03`: init, autoridad, binding, roadmap y gates se crean sólo en data-root nuevo.

### Etapa 2 — gobierno

- `OPA-S04`: status/doctor/list/show no cambian versión ni digest.
- `OPA-S05`: un agente recibe exit 9 al intentar una acción humana.
- `OPA-S06`: autoridad auxiliar se refresca y supersede sin borrar historia.
- `OPA-S07`: archivos, pruebas o documentación no cierran por sí solos fase/proyecto.

### Etapa 3 — bitácora y continuidad

- `OPA-S08`: progreso, nota, claim, unknown, decisión, blocker, riesgo y next action quedan representados.
- `OPA-S09`: resolución, mitigación y supersesión preservan antecedentes.
- `OPA-S10`: checkpoint y resume recuperan referencias completas y siguiente acción.

### Etapa 4 — evidencia y lifecycle

- `OPA-S11`: evidencia `OBSERVED` con exit 0 deja gate incompleto y retorna exit 6.
- `OPA-S12`: RecordSink local acepta una evidencia `VERIFIED_SOURCE` exacta y replay no duplica.
- `OPA-S13`: sólo Gate Engine verifica y sólo Phase Engine permite cierre.
- `OPA-S14`: cambio material vuelve stale el gate/checkpoint y la revalidación es explícita.
- `OPA-S15`: waiver humano sólo afecta el gate opcional declarado.

### Etapa 5 — transacciones

- `OPA-S16`: replay idéntico produce delta cero.
- `OPA-S17`: misma clave con otra intención retorna exit 10 y delta cero.
- `OPA-S18`: expected version obsoleta retorna exit 5 y obliga a replanificar.
- `OPA-S19`: autoridad insuficiente retorna exit 9 y delta cero.
- `OPA-S20`: respuesta descartada se recupera mediante reconcile sin reejecución.
- `OPA-S21`: resultado inexistente retorna exit 12 y no autoriza retry ciego.

### Etapa 6 — vistas, privacidad y recuperación

- `OPA-S22`: divergencia de Markdown se detecta y la regeneración recupera la vista.
- `OPA-S23`: portable roundtrip conserva historia y estado contractual.
- `OPA-S24`: backup/restore conserva resultados e idempotencia.
- `OPA-S25`: corrupción, divergencia y destino existente se rechazan sin overwrite.
- `OPA-S26`: canaries no aparecen en resume, render, export ni reporte sanitizado.
- `OPA-S27`: doctor/verify/audit terminan sin findings.

### Etapa 7 — agente e integraciones

- `OPA-S28`: tarea Codex nueva reconstruye estado sin relato previo y produce handoff gobernado.
- `OPA-S29`: provider tiene delta cero; RecordSink sólo ejecuta capacidades allowlisted.
- `OPA-S30`: GSP export/compile/validate delta cero; link delta uno; replay delta cero; lifecycle delta cero.
- `OPA-S31`: Radar queda `EXCLUDED_BY_SCOPE`, nunca PASS.

### Etapa 8 — aceptación

- `OPA-S32`: reporte, matriz, audit, checkpoint y decisión humana son coherentes y frescos.

## 9. Fórmula de aceptación

`OPERATIONALLY_ACCEPTED` requiere simultáneamente:

1. 8/8 etapas cerradas y `OPA-S01…S30` satisfechos según su semántica positiva/negativa;
2. `OPA-S31=EXCLUDED_BY_SCOPE`, sin presentarlo como evidencia de compatibilidad;
3. cero escenarios críticos `FAIL`, `BLOCKED` o `NOT_RUN`;
4. cero conflictos, blockers o unknowns críticos abiertos;
5. cero waivers sobre gates requeridos;
6. audit sin findings, checkpoint vigente y handoff ciego PASS;
7. decisión final explícita de `human:owner`.

Sin la condición 7 el estado máximo es `READY_FOR_OWNER_DECISION`. Cualquier fallo crítico produce `BLOCKED` hasta corrección y repetición, o `REJECTED` por decisión del propietario.

## 10. Protocolo de defectos

Un fallo inesperado detiene la etapa. Se conserva evidencia sanitizada, se abre blocker y no se marca avance adicional. Un defecto corregible sin cambiar contratos v0.2 exige prueba RED, corrección mínima, GREEN, gate completo, candidato exact-SHA nuevo y repetición desde la etapa afectada. Cambios contractuales o en repositorios externos requieren alcance nuevo.

## 11. Comunicación y commits

Cada cierre informa etapa/8, porcentaje, interpretación, bloqueos, pendientes, siguiente paso y autorización humana. La preparación documental y cada etapa tienen commit propio; toda corrección tiene commit separado. No se hace push ni se crea una release sin solicitud explícita.
