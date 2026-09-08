# PL-GSK-004 — Conflictos de versión completos y recuperables

## Autoridad y alcance

El 2026-09-08 G-Skills Engine III comunicó la autorización explícita de Alfredo «Autorizo, procede» para ejecutar `G-Skills/docs/context-orchestration/PL-GSK-004-conflict-plan.md`, SHA-256 `7eacec90fe54401db877fad7c6de29c368ff02e88d263f8d5268edbf2ce95487`. Habilita implementación bilateral, fixtures nuevas y commits locales. Excluye publicación, push/merge, tags, release, migraciones, bases operativas y Fase 7.

Base Ledger: `64343adc0f123b660fb85b7611049eff28f2a886`, producto anterior `4f720e11e799fc868560feb2836125289e90777c`, rama `codex/history-core`. La entrega `v0.2.0-rc.1` y sus locks, los contratos congelados y la evidencia de PL-GSK-002/003 permanecen intactos. Esta corrección requiere una selección candidata y un ejecutable nuevos.

## Comportamiento

`GSkillsRecordSink::submit` usa la misma preparación que `validate`. `VALID` confirma admisibilidad para enviar; no garantiza aceptación ni que HEAD conserve la versión preparada. Schema, digests, identidad, ámbito, policy, privacidad y aprobación se validan siempre. Si HEAD coincide con expected version, se compara además la identidad completa del estado. Si difiere, el envelope aprobado se conserva para que el núcleo produzca el rechazo. Validate no persiste ni fabrica un CommandResult; tampoco actualiza expected version ni cambia la aprobación.

`EventStore::finalize_version_conflict` es una operación interna. Su implementación SQLite abre `BEGIN IMMEDIATE`, valida el comando, resuelve la idempotencia, lee HEAD y exige mismatch. Construye el resultado con `previous_state_version == new_state_version == HEAD`, sin eventos ni checkpoint, y persiste comando, resultado e índice juntos. Sólo devuelve el resultado después del commit. Un resultado previo gana por idempotencia y se conserva sin alteración. No se reejecuta el handler ni se reenvía la mutación.

El processor usa esta finalización para la versión inicialmente obsoleta y para `StreamVersionConflict` al confirmar un comando aceptado o un rechazo de dominio. Un nuevo avance antes de finalizar se observa dentro del lock. Las validaciones existentes de `commit_command`, las PK/UNIQUE y `persist_commit` se conservan. Un fallo de almacenamiento revierte la transacción y se propaga como error; no se presenta como resultado durable.

No hay nuevos métodos RPC, capabilities, wire schemas, tablas, migraciones ni hooks de prueba en el producto. El perfil de confirmación histórica mantiene sus contratos independientes.

## Evidencia de regresión

Los logs locales están en `.tooling/pl-gsk-004/`; el inventario de entrega fija sus hashes.

- `red-races.log`: cuatro fallos esperados sobre la base. Dos devolvían VERSION_CONFLICT sin fila durable, el rechazo inicialmente obsoleto terminaba en InvalidCommit tras otro avance y el fallo de almacenamiento todavía devolvía un resultado aparentemente válido.
- `red-validate.log`: la segunda Validate devuelve -32602 por `project_state_version is stale` antes de implementar la corrección.
- `green-races.log`: las cuatro regresiones del núcleo pasan tras la corrección, con resultados estructurados `PL_GSK_004_RACE` en stdout.
- `targeted.log`: 26 pruebas PASS: siete del núcleo, siete CLI y doce de mapping/protocolo/framing. Esta pasada inicial conserva las advertencias de imports redundantes, corregidas antes de la verificación final.
- Se conservan también los diagnósticos iniciales del arnés (`u64` no es FromSql y una tabla no tiene rowid) y la primera observación de Clippy; son fallos de preparación de pruebas/lint, no evidencia RED del defecto productivo.

Las pruebas de carreras usan `CommandProcessor`, `CoreCommandHandler` y SQLite reales, con una segunda conexión en otro hilo. Un Clock de prueba coordina canales con timeout; no hay sleeps ni cambios para sincronización dentro del ejecutable. Se comprueban las ventanas entre lectura y commit, entre detección inicial y finalización, y al persistir un rechazo de dominio. También se observa el rollback de una inserción de resultado abortada, replay, clave/ámbito e integridad. La recuperación se verifica tras reabrir y después de un avance adicional, manteniendo el resultado original y cero eventos del comando rechazado.

La prueba CLI amplía el caso de segunda Validate y compara todas las tablas antes y después de la lectura; los controles negativos se ejercitan tanto en Validate como en Submit. Cada consulta de recuperación CLI arranca un proceso nuevo.

La evidencia de carreras pertenece al núcleo compilado; no se presenta como una carrera inyectada dentro del ejecutable CLI distribuido. G-Skills recalifica por separado su flujo completo contra el ejecutable candidato fijado por hash.

## Límites

La durabilidad cubierta corresponde a VERSION_CONFLICT de estas rutas con persistencia exitosa. No se declara durabilidad universal para errores de integridad, contrato, clave o identidad. En particular, el rechazo por `IdempotencyConflict` conserva el comportamiento previo; no crea un nuevo propietario de una clave ocupada. Un fallo de transporte/almacenamiento conserva la necesidad de reconciliar sin reintento ciego.

La sensibilidad previa del test `supervised_execution_is_bounded_private_and_uncertain_attempts_never_retry` a su plazo de 700 ms sigue documentada en PL-GSK-003-CONFIRMATION. Una pasada posterior no demuestra su reparación; no se modificó ese código ni esa prueba. La indisponibilidad intermitente de claves informada por G-Skills también queda fuera de esta corrección.

S24/Fase 6 no se cierran por completar la parte nativa. La autoridad operativa de rc.1 no se transfiere a este candidato. La reversión consiste en retirar su selección y revertir únicamente sus commits locales, conservando la evidencia. No abrir bases candidatas con rc.1.

## Entrega y verificación final

Pendiente de completar con commit de producto, hashes del ejecutable y binario de pruebas, resultados de regresión y evidencia bilateral.
