# Etapa 3 — evidencia de la CLI operativa

Alcance: procesos reales del ejecutable `project-ledger`, SQLite descartable y contratos v0.2 sin modificaciones. Las entradas fixture sirven para construir datos; no se presentan como compilación GSP ni interacción con otro producto.

`crates/cli/tests/operations.rs` contiene 17 pruebas. Se observaron sus fallos iniciales por interfaz ausente, códigos de error incorrectos, colisiones de identidad y auditoría incompleta; las 17 pasan tras las correcciones.

| Superficie | Prueba de proceso / resultado |
|---|---|
| Diagnóstico y errores | DB ausente: 4 y ningún archivo creado; auditoría de resultado manipulado: 15 sin escritura; stdout JSON separado de diagnóstico |
| Consultas y selección | DB antigua rechazada sin migración; selector explícito prevalece; locator no concede autoridad; catálogo list sin escritura y rebuild explícito |
| Núcleo cotidiano | Roadmap, fases, gates, notas, progreso, artefactos, claims, unknowns, decisiones, blockers, riesgos, next y sesiones; 26 operaciones canónicas con replay y consultas, más variantes por flags/JSON/YAML |
| Autoridad/repositorio | Registro, refresh y supersesión explícitos; vincular/refrescar/revocar sin ejecutar contenido del repositorio ni alterar sus archivos |
| Evidencia/gates | Importación rebajada a OBSERVED; evaluación no satisfactoria y cierre rechazado: 6; waiver humano explícito; reopen según lifecycle |
| Idempotencia | Resultado original reconciliado por scope completo; versión obsoleta: 5; payload distinto: 10; actores y dominios de confianza aislados |
| Continuidad | Resume con locator y JSON; checkpoint replay/reconcile aislado entre actores; IDs históricos conservados al reconciliar |
| Ejecución explícita | Política obligatoria, rechazo de programa no autorizado, resultado OBSERVED y replay sin volver a ejecutar. Los límites de proceso se cierran en etapa 4 |

Las consultas usan apertura read-only y las mutaciones apertura existing: ninguna migra implícitamente. Solamente init crea el almacén. La guía ejecutable está en `docs/CLI_OPERATIONS.md`.

## Límites que no se consideran cerrados aquí

- PP-05 y PP-06 tienen componentes de recuperación y recorrido Codex en etapas 5/6; esta etapa demuestra su parte CLI, no todo el flujo final.
- Timeout, cancelación, límites de captura, aislamiento del entorno y resultado incierto durante la ejecución externa siguen en etapa 4.
- Render/import/export/backup/migrate/restore: etapa 5; GSP supervisado y códigos específicos de integración: etapa 7; recursos desde instalación y ZIP: etapa 8.
- No hubo proyecto real, aceptación externa, nueva CI hosted ni prueba del paquete distribuible en esta etapa.
