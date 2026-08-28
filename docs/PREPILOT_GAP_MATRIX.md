# Matriz de brechas de preparación para el piloto

Baseline: `dea208d294dfddc282b84dcb5a58b9d8d9887b91`, 2026-08-27. Alcance autorizado: Windows, Ledger + Codex + GSP fijado, contratos v0.2 conservados, sólo datos descartables.

## Cómo interpretar la evidencia

`LIBRARY` demuestra componentes reales en proceso; `CLI` demuestra el ejecutable; `RUNTIME` demuestra un proceso externo real; `PACKAGE` demuestra el ZIP extraído; `HOSTED` identifica SHA/run/job; `FIXTURE` no demuestra ejecución de la contraparte. Un inventario o un archivo presente no demuestra su comportamiento. Los reports de release anteriores conservan valor histórico, no prueban por sí solos preparación operacional.

## Brechas y pruebas de cierre

| ID / etapa | Requisito | Implementación e interfaz actual | Evidencia baseline / limitación | Prueba exigida para cerrar |
|---|---|---|---|---|
| PP-01 / 1 | §48/49, AC-090: no sobreafirmar | Reports de release, README, bitácora y planes | HOSTED RQ4 final success, pero bitácora seguía en 2/3; 90 AC/32 PL no equivalen a recorridos completos | Conciliar fuente exacta; etiquetar alcance/estado; cobertura de cada AC/PL en esta matriz |
| PP-02 / 2 | §15/16: 53 comandos / 67 eventos | `application/core_handler`, motores y `projection-engine/lifecycle` | STATIC: siete variantes sin handler/reducer: notas, artefactos, start/end session, confirm/revoke mapping, event correction | Submit real, append esperado, invalid/replay y rebuild incremental equivalente; originales intactos |
| PP-03 / 3 | §27: CLI cotidiana | `cli/main.rs`, familias normativas ausentes | CLI: `record progress --help` exit 2, unrecognized record | Cada familia invocada en proceso; no sólo help ni llamada directa a biblioteca |
| PP-04 / 3 | §27.9/10: errores y JSON | CLI retorna `Result<String,String>` y exit 1 general | CLI: resume con DB inexistente exit 1, no crea archivo; esperado 4 | Códigos normativos diferenciados; stdout sólo JSON; stderr diagnóstico |
| PP-05 / 3,5 | §27/39: queries/locator | DB/ref explícitos; `SqliteEventStore::open` aplica migraciones | STATIC: no apertura read-only separada ni locator operativo | Query no crea/migra/escribe canonical rows; selector explícito gana; binding ambiguo rechazado |
| PP-06 / 3,6 | §16/40: idempotencia/reconciliación | Biblioteca `SubmissionService`; checkpoint CLI; RecordSink | LIBRARY/CLI parcial, sin consulta CLI independiente para Codex | Resultado incierto recupera resultado original con misma clave; payload distinto rechazado; cero replay duplicado |
| PP-07 / 4 | §22/41.2/42: ejecución acotada | `evidence/runner`: `Command::output()`; policy programa/redacciones/resumen | STATIC: sin timeout/límite de captura/allowlist de entorno/limpieza de hijos | Helper real: bloqueo, flood, child, cancel; termina acotado, no fuga, no falso gate PASS |
| PP-08 / 4,8 | §41/42: privacidad aplicada | Escáner y corpus declarados de release; runner/redacción | LIBRARY/FIXTURE, no todos los canales generados durante flujo | Canary ficticio ausente en DB, resumen, errores, Markdown, export y artefactos; no output raw por defecto |
| PP-09 / 5 | §28–31: Markdown, legacy y portable | Bibliotecas existen; no CLI público completo; legacy produce propuestas no durables | LIBRARY: roundtrip/revisión; fork es sólo plan sin escritura | CLI nuevo/existente, preview sin efectos, revisión completa, provenance, divergencia, no fork materializado |
| PP-10 / 5 | §39.5: backup/migraciones/restore | SQLite/WAL y migrations; sin API/CLI backup online | STATIC; copiar sólo ledger.db no cubre WAL ni resultados | Snapshot consistente; restore nuevo; cadena, command results e idempotencia; backup previo; no overwrite |
| PP-11 / 6 | §36/AC-061–064: skill ejecutable | Skill enumera siete acciones; test CLI sólo init/resume/checkpoint | FIXTURE procedimental + CLI parcial | Recorrido completo CLI y Codex descartable; missing-tool/conflict/outcome; sin autoridad inventada |
| PP-12 / 7 | §35: GSP más allá de crash | Sidecar real fijado y bridge; compilación externa separada, sin supervisor operativo CLI | RUNTIME RQ4 3 casos; conformance adicional usa fixtures | Matriz finita runtime/adversarial del plan; compile read-only, link explícito y replay; no elevación |
| PP-13 / 8 | §27/39/AC-004: recursos portables | `repository_root()` usa cwd; manifiesto de paquete no es ZIP probado | CLI fuera del checkout falla al buscar schemas/locks en cwd | ZIP desde otro cwd con recursos de instalación; no toolchain para núcleo, rutas Unicode/espacios |
| PP-14 / 8 | §45–49: cierre agregado | CI matrix + dependency + ENOSPC + GSP live | HOSTED baseline 6/6, no certifica cambios futuros | Gate fresco en SHA entregado, ZIP probado, pruebas opt-in explícitas y NOT_READY ante pendientes |

Todos los PP están OPEN en baseline salvo la conciliación PP-01, que se cierra con la etapa 1. Las etapas posteriores actualizan esta sección y el informe; no reescriben la evidencia baseline.

Estado de ejecución: **PP-01…PP-05, PP-07, PP-09/10 CLOSED; PP-06/08 parcialmente verificadas; PP-11…PP-14 OPEN; 5/8 etapas**. Evidencia: `docs/PREPILOT_CORE_COVERAGE.md`, `docs/PREPILOT_CLI_COVERAGE.md`, `docs/PREPILOT_PROCESS_SAFETY.md` y `docs/PREPILOT_PORTABILITY_RECOVERY.md`. Incorporación, exportación revisada, backup WAL, restauración y migración explícita están verificados por CLI. PP-06 conserva el recorrido Codex y PP-08 el paquete. Formato, Clippy estricto, suite workspace completa, build locked, schemas/specs/contratos, release check, dependencias e integridad pasaron localmente. Los cuatro tests opt-in no se cuentan como nuevas ejecuciones físicas/runtime; tampoco se ha probado un ZIP ni observado nueva CI hosted.

## Cobertura completa de aceptación (sin añadir certificación al alcance)

| Criterios | Responsable / prueba requerida | Evidencia histórica conservada |
|---|---|---|
| AC-001–005 | 3/5/8: init offline, standalone, misma operación CLI/JSON/stdio, ZIP y portable | CLI init; biblioteca portable; equivalencia previa sólo manifest/handler de test |
| AC-006–012 | 2/3/5: cadena, rebuild, gaps, query sin efectos, cache corrupta, unions/registry | event-chain, projection-cache y schemas reales |
| AC-013–021 | 2/3/4: gates/evidencia/waiver/phase por CLI y ejecución supervisada | gate-engine y evidence library; no confundir exit 0 con gate completo |
| AC-022–028 | 2/3/6: decisiones, blockers/riesgos, resume, next, checkpoint y status | continuity/lifecycle library; CLI previo parcial |
| AC-029–035 | 3/5: render/audit/regeneración/importación revisada/divergencia | Markdown, legacy, portable library; fork durable fuera de contrato |
| AC-036–041 | 2/3/5/6: replay, conflictos, reconciliación, crash/restore | transacciones/submissions; ENOSPC físico histórico |
| AC-042–051 | Regresión 8; integración G-Skills deshabilitada para piloto | provider/sink local y fixtures; no aceptación de contraparte |
| AC-052–060 | 7: exact-contract, loss/contradiction/uncertainty, aislamiento y enlace | schemas/fixtures fijados y RQ4 real; ampliación runtime pendiente |
| AC-061–064 | 6: skill contrastada con flujo ejecutable completo | conformance procedimental y CLI init/resume/checkpoint |
| AC-065–080 | Regresión 8; Radar deshabilitado para piloto | export/reference importer; no IndexedDB/UI de contraparte |
| AC-081–085 | 3/4/5/8: repo no confiable, argv, secretos, minimización, no transcripts | políticas y corpus; ampliar a superficies generadas |
| AC-086–090 | 1/8: CI fresca, schemas offline, todos PL, benchmark, etiquetas honestas | CI/RSS ya calificados; mantener separación con preparación y certificación |

## Cobertura completa de escenarios PL

| Escenarios | Responsable y recorrido |
|---|---|
| PL-01–05 | 2/3/5: contradicción, gate incompleto, blocker, supersesión y revalidación |
| PL-06–09 | 2/3/6: replay, concurrencia, unknown outcome, 100 queries sin mutación |
| PL-10–11 | Regresión G-Skills 8, no certificación ni habilitación |
| PL-12–13 | 7: GSP separación/conflicto con runtime y adversarial etiquetados |
| PL-14–15 | 5: Markdown divergente y portable roundtrip por CLI |
| PL-16–17 | 3/7/8: standalone sin provider/runtime; identidad ambigua no mezcla |
| PL-18–20 | 2/3/6: checkpoint N/N+1, payload cerrado, timeout no inventa server result |
| PL-21–32 | Regresión Radar 8, export/reference importer, no implementación de contraparte |

## Fuente RQ4 final conciliada

Consulta read-only fresca: `gh run view 33062468586 --repo gcatalansc/G-Ledger --json headSha,conclusion,jobs`.

- SHA `dea208d294dfddc282b84dcb5a58b9d8d9887b91`, completed/success.
- Dependency `98484355390`, macOS `98484355534`, GSP real `98484355574`, ENOSPC `98484355673`, Ubuntu `98484355727`, Windows `98484355753`: todos success.
- URL: https://github.com/gcatalansc/G-Ledger/actions/runs/33062468586
- RQ4 = 3/3 para crash/recovery acotado; conformidad general y certificación siguen sin declararse.

## Exclusiones explícitas

No GA, certificación bilateral universal, operación sobre proyecto real, UI/daemon/MCP nuevo, migración a GSP 0.3.x, cambio de contratos v0.2, fork materializado, instalación global o modificación de otros repositorios. Las integraciones excluidas mantienen sus tests existentes; un defecto que comprometa el núcleo sí bloquea el cierre.
