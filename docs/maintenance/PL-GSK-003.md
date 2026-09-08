# PL-GSK-003: recuperar el resultado en su versión histórica

Estado: **corrección interna recalificada bilateralmente; adenda de confirmación externa pendiente**. No se declara cierre integral de PL-GSK-003 ni de S24.

## Autorización y alcance

G-Skills Engine III comunicó la autorización de Alfredo «Autorizo, procede» para el plan `docs/context-orchestration/PL-GSK-003-proposed-plan.md` de G-Skills. El plan exige revisar primero la superficie histórica y obtener autorización adicional si hace falta ampliar contratos públicos. Base local comprobada: `33455323975ae4397d719f4d40c83706e14096a4`, `codex/history-core`, árbol limpio.

Se autorizaron implementación interna, pruebas en fixtures propias, commits locales y entrega de binario. Se mantienen fuera publicación, push, merge, tags, migraciones, datos operativos y la nueva superficie contractual aún no aprobada. La entrega rc.1 permanece intacta.

## Corrección interna

`SqliteRecordSinkHost::confirmed_projection_ref` comprobaba que la versión actual fuera igual a la del resultado guardado. Después de otro avance, impedía recuperar incluso un resultado íntegro y persistido.

Ahora verifica explícitamente el proyecto seleccionado, carga un snapshot SQLite/WAL consistente y verificado y reconstruye `HistoryView::at(result.new_state_version)`. La referencia sigue siendo `continuity:ledger-V`, con V del resultado. No sustituye V por HEAD ni modifica `CommandResult`, su digest o los eventos. Cero, versión inexistente, corte intermedio de comando, identidad distinta o cadena/cabecera corrupta no producen confirmación.

Sólo cambia esta composición interna en `crates/cli/src/main.rs`; los nuevos tests están en `crates/cli/src/sink_history_tests.rs` y `crates/cli/tests/gskills_sink.rs`. No cambian dependencias, schemas, catálogos, métodos stdio, specs ni locks contractuales.

## Revisión contractual

La CLI existente `history at --version V --database <ruta local fijada> --project-ref <identidad nativa fijada> --json` produce `history-response` v0.3, `kind=STATE`. Schema cerrado: `schemas/v0.3/history-response.schema.json`, `$id=urn:project-ledger:schema:history-response:0.3.0`, SHA-256 `034ac4027ac95ebf492db7c8af6b5febcfacc76c637348fd453df5b379d81415`.

Los campos son `schema_version`, `kind`, `project_ref`, `state_version`, `state_digest`, `head_digest`, `read_only`, `state` y `episodes`. El estado interior conserva v0.2. Proyecto y versión exteriores deben coincidir con los interiores y la selección; `project_state_id=state:<UUID del proyecto>`. `state_digest` se calcula sobre JSON canónico del estado quitando sólo `state_digest`. No cubre episodios ni el envelope completo; `head_digest` es el digest del último evento del corte.

La CLI abre read-only, verifica índices/envelopes, secuencia y cadena completa en una transacción consistente; la proyección valida las invariantes del prefijo seleccionado. Rechaza cortes incompletos o inexistentes. La consulta procesa el stream completo, no pagina STATE y no impone su propio límite de bytes/tiempo. La salida contiene estado y episodios completos; no aporta binding de solicitud/provenance ni comprobación autocontenida de todos los eventos emitidos por un comando.

Se acordó que consumir STATE desde G-Skills **no está habilitado por el perfil/allowlist/lock vigente**. Tampoco basta `current >= V` o una referencia `continuity:ledger-V` para probar el efecto. G-Skills conserva resultado, contexto actual y confirmación separados mediante metadata local; el avance posterior sin prueba histórica permanece sin confirmar.

La propuesta adicional está en los documentos G-Skills `PL-GSK-003-contract-review.md` y `PL-GSK-003-confirmation-addendum.md`. Propone un perfil separado, lectura nativa minimizada ligada a solicitud, scope nativo, resultado durable y corte verificados en un snapshot único, con autoridad/provenance, digests y límites explícitos. No importa STATE/episodes ni habilita acceso directo del adapter a SQLite. Ningún código de esa superficie se implementó en G-Ledger.

## Verificación local

- RED: las dos pruebas CLI de recuperación tras avance 2→3 reproducen `-32603`, `confirmed projection version mismatch`; la prueba del host rechaza incorrectamente el corte válido 1 con HEAD=3.
- GREEN: 3 pruebas del binario y 7 de `gskills_sink` pasan. Las dos recuperaciones, aceptación y rechazo, devuelven el mismo objeto/digest tras otro checkpoint y procesos nuevos, incluso después de caducar la aprobación del envío. El stream no cambia por consultar.
- Negativos: cortes 0, 2 intermedio, 4 inexistente y `u64::MAX`, otro proyecto y cabecera actual corrupta. No confirman; los cortes 1 y 3 íntegros sí.
- Regresión: `cargo test --locked --all-targets --all-features -p project-ledger-cli -p project-ledger-interop-gskills -p project-ledger-projection-engine -p project-ledger-storage-sqlite --no-fail-fast`: **141 PASS, 0 fallos, 9 omitidas**, 41 harnesses, exit 0. Las omisiones preexistentes requieren helpers/entorno GSP o filesystem efímero de ENOSPC.
- `cargo fmt --all -- --check`, Clippy de todo el workspace con todas las features/targets y `-D warnings`, schemas, specs, contratos y 461 recursos: PASS. Los contratos y dependencias coinciden con la base.

Logs locales preservados en `.tooling/pl-gsk-003/`, fuera de Git. Esta pasada focaliza las suites afectadas; no se presenta como otra ejecución de todo el workspace ni como CI multiplataforma.

## Candidato optimizado

`cargo build --locked --release -p project-ledger-cli`: exit 0, 1m 03s. Copia exacta preservada en `.tooling/pl-gsk-003/delivery/project-ledger.exe`, SHA-256 `f20be221dca54dcee2777e6624494fbd2c229063a07b3884ae6177c43cd4f8e7`. Sigue declarando `project-ledger 0.2.0`; no se crea una versión publicada.

Las siete pruebas CLI de `gskills_sink` también pasan contra esa copia mediante `CARGO_BIN_EXE_project-ledger`, exit 0. `specs verify`, `contracts verify`, `resources verify` y `release verify` pasan contra el mismo ejecutable con `--resources <raíz de este checkout>`; los 461 recursos están verificados. Al copiar sólo el ejecutable fuera de su instalación, omitir esa ruta falla con `RESOURCE_INTEGRITY`/exit 15, según el contrato de recursos. No es un paquete distribuible autónomo.

Log de regresión: `.tooling/pl-gsk-003/regression.log`, SHA-256 `cd9b541c62dedb6af90d7b7acc9979450c53250947466fe2d726e33350844990`. Pruebas del ejecutable: `.tooling/pl-gsk-003/delivery-cli-tests.log`. La calificación bilateral se realiza por commit y hash exactos, sin promover la nueva superficie contractual.

## Calificación bilateral (2026-09-08)

Commit Ledger entregado: `d0661b4242e492881c279c9ce3434aa8b2655c31`. G-Skills fijó su código en `26a6eb641187d3d4ca28544ce17d087a0b689b64`, host dev SHA-256 `d4a71ba9ca0d2eda73db0590340706f1b73cf52f15d973e0da9a20c0e4a2a152` y helper de recuperación `27e6772ababb689b38c7f8860a321a4c500a7c53945061c826633b77a91afd09`. No se presentan esos artefactos de prueba como releases.

Reporte G-Skills `release/evidence/pl-gsk-003-candidate-qualification.json`, SHA-256 `bec3df620e9690f9846162214b32c7ac151271876416476a9fb7f86eb55029bd`: **SCOPED_PASS_HISTORY_CONTRACT_PENDING**. Cinco mappings y dos escenarios de conflicto/refresco PASS, incluyendo rechazo conocido seguido de avance. El octavo caso, aceptación con respuesta perdida y avance 22→23, registra **NATIVE_RECOVERY_PASS_HISTORY_CONTRACT_PENDING**.

Después del avance y con procesos nuevos, ambos métodos nativos devuelven FOUND y el objeto completo original, incluido su digest. En el rechazo conocido, G-Skills mantiene `RECONCILED_REJECTED`, intento 1, y confirmación `UNAVAILABLE` separada. En la aceptación con respuesta perdida, conserva `ACCEPTED` conocido e intento 1, pero mantiene `OUTCOME_UNKNOWN` y `UNAVAILABLE/HISTORICAL_PROOF_CONTRACT_REQUIRED`: resultado V22 y contexto actual V23 son distintos. No reenvía ni afirma confirmación histórica por comparar `current >= V`.

Control con el mismo host y Ledger anterior `50e6d555`: `release/evidence/pl-gsk-003-baseline-comparison.json`, SHA-256 `f246956643e8ccd51a98aa36b8700b9ddf93327592cc1554c512dd1e8c4d0856`, sigue reproduciendo GAP_REPRODUCED. La corrección nativa queda contrastada sin atribuirle capacidades aún no implementadas.

Revisión directa desde G-Ledger: hashes de los cuatro ejecutables seleccionados coinciden; los ocho pares de snapshots antes/después de consultas son idénticos; ambos resultados nativos recuperados y el conservado por el host coinciden completos con el original. Cero escrituras operativas; `release_certified=false`, `phase_6_closed=false`. El reporte no promueve UB51/53 ni S24.

## Límites y rollback

La corrección elimina el bloqueo interno del lookup; la confirmación histórica automática en G-Skills requiere la adenda revisada y una autorización nueva. Las carreras durante el propio commit que pueden devolver resultados no persistidos siguen fuera de alcance. No se declara certificación integral, GA ni cierre de Fase 6.

Rollback: revertir el commit candidato y retirar su selección en G-Skills, conservando pruebas/evidencia. No abrir una base con eventos v0.3 mediante rc.1.
