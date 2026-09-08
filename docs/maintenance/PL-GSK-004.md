# PL-GSK-004 — Conflictos de versión completos y recuperables

## Autoridad y alcance

El 2026-09-08 G-Skills Engine III comunicó la autorización explícita de Alfredo «Autorizo, procede» para ejecutar `G-Skills/docs/context-orchestration/PL-GSK-004-conflict-plan.md`, SHA-256 `7eacec90fe54401db877fad7c6de29c368ff02e88d263f8d5268edbf2ce95487`. Habilita implementación bilateral, fixtures nuevas y commits locales. Excluye publicación, push/merge, tags, release, migraciones, bases operativas y Fase 7.

Base Ledger: `64343adc0f123b660fb85b7611049eff28f2a886`, producto anterior `4f720e11e799fc868560feb2836125289e90777c`, rama `codex/history-core`. La entrega `v0.2.0-rc.1` y sus locks, los contratos congelados y la evidencia de PL-GSK-002/003 permanecen intactos. Esta corrección requiere una selección candidata y un ejecutable nuevos.

## Comportamiento

`GSkillsRecordSink::submit` usa la misma preparación que `validate`. `VALID` confirma admisibilidad para enviar; no garantiza aceptación ni que HEAD conserve la versión preparada. Schema, digests, identidad, ámbito, policy, privacidad y aprobación se validan siempre. Si HEAD coincide con expected version, se compara además la identidad completa del estado. Si difiere, el envelope aprobado se conserva para que el núcleo produzca el rechazo. Validate no persiste ni fabrica un CommandResult; tampoco actualiza expected version ni cambia la aprobación.

`EventStore::finalize_version_conflict` es una operación interna. Su implementación SQLite abre `BEGIN IMMEDIATE`, valida el comando, resuelve la idempotencia, lee HEAD y exige mismatch. Construye el resultado con `previous_state_version == new_state_version == HEAD`, sin eventos ni checkpoint, y persiste comando, resultado e índice juntos. Sólo devuelve el resultado después del commit. Un resultado previo gana por idempotencia y se conserva sin alteración. No se reejecuta el handler ni se reenvía la mutación.

El processor usa esta finalización para la versión inicialmente obsoleta y para `StreamVersionConflict` al confirmar un comando aceptado o un rechazo de dominio. Un nuevo avance antes de finalizar se observa dentro del lock. Las validaciones existentes de `commit_command`, las PK/UNIQUE y `persist_commit` se conservan. Un fallo de almacenamiento revierte la transacción y se propaga como error; no se presenta como resultado durable.

No hay nuevos métodos RPC, capabilities, wire schemas, tablas, migraciones ni hooks de prueba en el producto. El perfil de confirmación histórica mantiene sus contratos independientes.

La revisión bilateral posterior al primer commit detectó otra brecha de identidad: un resultado recuperado sólo por command_id podía mapearse contra otra clave o payload. El puerto interno RecordSinkHost ahora entrega el comando original junto al resultado. El sink verifica su integridad y compara el envelope nativo completo —incluidos proyecto, actor/scope, clave, payload, expected version y timestamp preparado— antes de reutilizarlo en Validate, Submit, status o reconcile. Una identidad reutilizada con otro comando se rechaza como IdentityMismatch, sin inventar resultado ni depender de la confirmación opt-in. Un nuevo command_id con clave ocupada sigue llegando al rechazo nativo de idempotencia. El wire permanece idéntico.

## Evidencia de regresión

Los logs locales están en `.tooling/pl-gsk-004/`; el inventario de entrega fija sus hashes.

- `red-races.log`: cuatro fallos esperados sobre la base. Dos devolvían VERSION_CONFLICT sin fila durable, el rechazo inicialmente obsoleto terminaba en InvalidCommit tras otro avance y el fallo de almacenamiento todavía devolvía un resultado aparentemente válido.
- `red-validate.log`: la segunda Validate devuelve -32602 por `project_state_version is stale` antes de implementar la corrección.
- `green-races.log`: las cuatro regresiones del núcleo pasan tras la corrección, con resultados estructurados `PL_GSK_004_RACE` en stdout.
- `targeted.log`: 26 pruebas PASS: siete del núcleo, siete CLI y doce de mapping/protocolo/framing. Esta pasada inicial conserva las advertencias de imports redundantes, corregidas antes de la verificación final.
- `regression.log`: 340 PASS, 0 FAIL, 14 omitidas, 112 harnesses del workspace/all-targets/all-features para el intermedio `c7f9288`. `core-final.log` añade el control de reutilización de command_id con otra key/actor-scope, 8 PASS. Esa regresión completa no se atribuye a la corrección posterior del binding de resultados.
- `red-result-binding.log`: la CLI del intermedio admite en Validate la misma identidad con otra clave. La regresión nueva exige rechazo en Validate/status/reconcile/Submit, conserva todas las tablas y comprueba que el lookup original sigue funcionando. Su ejecución posterior y la regresión de CLI/interop se registran aparte.
- `green-result-binding.log`: 20 PASS. `regression-binding.log`: regresión final de CLI e interop-gskills/all-targets/all-features, 106 PASS, 0 FAIL, 8 omitidas, 25 harnesses. Clippy final de todo el workspace PASS en `clippy-binding.log`. No se repite ni se reasigna al producto final la pasada integral del intermedio.
- Se conservan también los diagnósticos iniciales del arnés (`u64` no es FromSql y una tabla no tiene rowid) y la primera observación de Clippy; son fallos de preparación de pruebas/lint, no evidencia RED del defecto productivo.

Las pruebas de carreras usan `CommandProcessor`, `CoreCommandHandler` y SQLite reales, con una segunda conexión en otro hilo. Un Clock de prueba coordina canales con timeout; no hay sleeps ni cambios para sincronización dentro del ejecutable. Se comprueban las ventanas entre lectura y commit, entre detección inicial y finalización, y al persistir un rechazo de dominio. También se observa el rollback de una inserción de resultado abortada, replay, clave/ámbito e integridad. La recuperación se verifica tras reabrir y después de un avance adicional, manteniendo el resultado original y cero eventos del comando rechazado.

La prueba CLI amplía el caso de segunda Validate y compara todas las tablas antes y después de la lectura; los controles negativos se ejercitan tanto en Validate como en Submit. Cada consulta de recuperación CLI arranca un proceso nuevo.

La evidencia de carreras pertenece al núcleo compilado; no se presenta como una carrera inyectada dentro del ejecutable CLI distribuido. G-Skills recalifica por separado su flujo completo contra el ejecutable candidato fijado por hash.

## Límites

La durabilidad cubierta corresponde a VERSION_CONFLICT de estas rutas con persistencia exitosa. No se declara durabilidad universal para errores de integridad, contrato, clave o identidad. En particular, el rechazo por `IdempotencyConflict` conserva el comportamiento previo; no crea un nuevo propietario de una clave ocupada. Un fallo de transporte/almacenamiento conserva la necesidad de reconciliar sin reintento ciego.

La sensibilidad previa del test `supervised_execution_is_bounded_private_and_uncertain_attempts_never_retry` a su plazo de 700 ms sigue documentada en PL-GSK-003-CONFIRMATION. Una pasada posterior no demuestra su reparación; no se modificó ese código ni esa prueba. La indisponibilidad intermitente de claves informada por G-Skills también queda fuera de esta corrección.

S24/Fase 6 no se cierran por completar la parte nativa. La autoridad operativa de rc.1 no se transfiere a este candidato. La reversión consiste en retirar su selección y revertir únicamente sus commits locales, conservando la evidencia. No abrir bases candidatas con rc.1.

## Entrega y verificación final

Estado al emitir la entrega nativa: **NATIVE_VERIFIED_PENDING_BILATERAL**; cierre bilateral posterior registrado abajo. Producto `952cf77c7f92fc3c3ceedb3b01383b8c40ffaf57`: integra el core `c7f9288153ae2d3bafeadc1b4a2fe397b8d8465a` y la protección posterior del binding. La parte application/event-store/storage no cambió entre ambos commits.

Entrega local inmutable en `.tooling/pl-gsk-004/delivery/`:

| Artefacto | SHA-256 y alcance |
|---|---|
| `project-ledger.exe` | `5cde5898ba6e3bc6decb3245295b808cc0813904585b26b63dd2fce14b64775b`; release optimizado, versión declarada 0.2.0, sin tag |
| `version-conflict-races.exe` | `6f0d658e558f3ca79cecad0d951ed47f1fbd45cac106d3b368dfb1e16dc6a3dc`; harness Rust debug del núcleo, 8 pruebas |
| `manifest.json` | `c8b9e76b0a386b22025e2e1093034c973244276c8b67a74a4986a3de50adb059`; fija 4 binarios y 26 logs/metadatos con hashes, sujetos de cada pasada y límites |

Build definitivo `cargo build --locked --release -p project-ledger-cli`, exit 0, 1m 18s. El build intermedio se conserva aparte en `intermediate-c7f9288`; no es la selección final. El ejecutable anterior `33f56f787af58812621eef443aba9f189c011050fa43e29bb7e1a173a0fa76a6` permanece idéntico.

Las copias de los harnesses CLI se ejecutaron con `CARGO_BIN_EXE_project-ledger` fijado al nuevo ejecutable release: sink **8 PASS**, confirmation **8 PASS**, sin fallos. El harness copiado del núcleo obtuvo **8 PASS** y tres observaciones JSON de carreras. `resources verify --resources <checkout Ledger>` verificó los 484 recursos. No se suman las pasadas repetidas como casos independientes. Se conservan la regresión completa del intermedio y la regresión de los componentes afectados del producto final como evidencias distintas.

El manifest y las copias se entregaron a G-Skills Engine III para su runner final de doce casos sobre fixtures nuevas, junto con los negativos de identidad en los cuatro métodos. El manifest permanece inmutable como registro de esa entrega anterior al resultado conjunto.

## Cierre bilateral — 2026-09-08

**PL-GSK-004 SCOPED_PASS: 12/12 casos, 120 invocaciones superiores**, sobre fixtures nuevas, producto Ledger `952cf77` / ejecutable `5cde5898…`. G-Skills: host `ace06001e6115cfc157edd0f973a3f252fd0d15e`, helper `abaf59e169072c5223d3d6e8deee6ca09eff229b` / `74296b044df10c0ed360fb87963a06465f711454cb197291371955dfdbe40d26`, runner `3a851aceab7b5dcd9fa0f390941cc98342f92d90`.

- Selección `G-Skills/integrations/project-ledger/candidates/pl-gsk-004-selection.json`, SHA-256 `7c526d88bead5166f8f004d9c0762104dcdd7758c1210274061f59080bc85271`.
- Reporte `G-Skills/release/evidence/pl-gsk-004-qualification.json`, SHA-256 `ffc4105f007600eb287f8b55c0ef4fa9e0e9dc94c3570d66ba9247616f27c9fb`.
- Binding de implementación candidato: `sha256:a2c085ddc8508422e2e54206f40cb59d15336c694c63b340bdd691e9f151156f`. No reemplaza los locks de rc.1 ni concede permisos operativos.

Ledger contrastó estos artefactos en sólo lectura: cuatro binarios seleccionados por hash; los doce resultados PASS; catorce pares de tablas invariantes durante lecturas; siete observaciones prepare sin mutación; cinco mappings nativos con digest y lista ordenada de eventos exactos; doce negativos de identidad con -32602, sin resultado ni cambios en tablas.

La ventana anterior a la segunda Validate conserva expected=21, obtiene un rechazo V22→22, attempt_count=1 y exige REFRESH_AND_REPLAN con nuevo comando/aprobación y sin retry original. El reinicio y avance posterior conservan el digest nativo `sha256:c42b5987c85658183caeaa41cc0f4b941b5c7a4f6856c150778907cb21bfbb4d`. El rechazo cuya respuesta se pierde pasa de OUTCOME_UNKNOWN a RECONCILED_REJECTED y conserva `sha256:3777fb1ad4a63c375840e2f6b1bab425c446698bda7ef85e4a90d5d461afd3f0`, con un intento y sin reenvío ciego. Durante el rechazo observado sólo cambian las tablas commands/command_results/idempotency_keys; las demás permanecen idénticas.

El alcance bilateral autorizado queda satisfecho para **LOCAL_CANDIDATE_OWNED_FIXTURES**. G-Skills informa UB-52/ACU-105 VERIFIED y S24 técnico SATISFIED sólo para este candidato. Fase 6 sigue 6/10 según su trazabilidad; no hay publicación, certificación de release ni autoridad operativa nueva. Permanecen explícitos los límites e incidencias anteriores.
