# PL-GSK-003: adenda de confirmación histórica

Estado: **adenda implementada y recalificada bilateralmente; SCOPED_PASS**. Cierre acotado de PL-GSK-003. Se conserva la limitación de la regresión ampliada indicada abajo; no se declara cierre de S24/Fase 6 ni release.

## Autorización y contrato

El 2026-09-08, G-Skills Engine III comunicó la autorización adicional explícita de Alfredo «Autorizo, procede» para `G-Skills/docs/context-orchestration/PL-GSK-003-confirmation-addendum.md`, fijada en G-Skills `a0c9382b0f4fe137e9e3a8fb2bc08386617e93e4`. Habilita perfil, schemas, manifest/policy/lock candidatos, implementación bilateral y pruebas locales. No habilita publicación, push/merge, tags, migración, DB operativa ni Fase 7.

Base Ledger: `ec952a84f4aac2e8244b10bdf32bbe080f4f7942`, árbol inicialmente limpio en `codex/history-core`. Los contratos se entregaron antes del método en commit `ab9faf83d00a8098a6461309f1c39ea953774fc8`. G-Skills confirmó la copia exacta de 22 archivos y los 21 hashes internos del lock. Los perfiles/specs/locks previos y rc.1 se conservan.

Perfil independiente `gskills-record-confirmation/0.1.0`; documentos de consumo en `integrations/gskills-confirmation/README.md`. Schemas cerrados draft 2020-12, mapping, loss report y fixtures minimizadas en `integrations/gskills-confirmation/v0.1/`:

| Artefacto | SHA-256 de bytes |
|---|---|
| request.schema.json | `1e37cfa11a46f0999676eaa2b00e6e6e07c5f7af484303131b63071a93b64bb6` |
| response.schema.json | `3a0d70c32cb0dc80625db7dae4f265c68584c0d2e1eda3710ceb6c83fe9fb12f` |
| manifest.schema.json | `65a0ffa30277d1bc585d37b24a75ed8b87c7812add357d9400bc5b6c4763d507` |
| policy.schema.json | `af6849bfaf677c6a8daf1117344dd495031cc623411ef39f51c71fc750873527` |
| candidate-lock.json | `5d1beaad02150a2d8f8998319f2ebc7cfaaa778e31171be523f180d440d08165` |

## Implementación

`serve confirmation --stdio --profile gskills-record-confirmation/0.1.0 --database <DB> --policy <policy.json> --project-ref <project:UUIDv7> --gskills-project-ref <project:mempilot> --observed-at <UTC>` expone únicamente `ledger.manifest` y `ledger.command_confirmation`. Params respectivos `{}` y `{"request":...}`. Manifest `created_at` coincide con el de la policy, y `authority_binding_ref` liga el digest completo de esa policy de lectura. La identidad de proyecto y actor/scope nativos proviene de configuración local; no adquiere permisos de escritura.

`SqliteEventStore::load_command_snapshot` abre una sola transacción, limita bytes/eventos antes de decodificar, valida índices completos del comando/resultado/scope y verifica la cadena y su cabecera. La evaluación posterior opera sólo sobre los valores de ese snapshot. `HistoryView::cut_identity` expone únicamente versión, versión material, state digest y head digest de un corte atómico reconstruido.

La prueba exige el resultado durable y su digest exactos; intervalo completo de eventos en orden; causalidad de cada evento con proyecto, comando y command digest; ausencia de eventos del comando fuera del intervalo. Una aceptación requiere un intervalo no vacío coherente con la versión esperada original. Un rechazo exige versión sin avance y ausencia total de eventos/checkpoint del comando. El checkpoint declarado debe corresponder al evento creado. Un replay no reemplaza al resultado durable ni iguala sus digests.

Request 32 KiB, response 256 KiB, 256 refs; cabecera 1024 bytes, snapshot máximo 100000 eventos/64 MiB. Lectura read-only sin crear/migrar. El proceso usa `observed_at` fijado localmente y valida UTC/frescura de solicitud (30 s y tolerancia 2 s); el host verifica frescura con reloj local y mata/recolecta al vencer 5 s. La evaluación nativa no emite VERIFIED si excedió su comprobación de 5 s. Errores no incluyen cuerpos del estado, eventos ni resultado inventado.

## Verificación local

- RED: el ejecutable anterior rechazaba `serve confirmation` como subcomando inexistente. Los contratos y sus 14 fixtures se validaron antes de implementar el método; todos los digests positivos coinciden con el canonicalizador Rust.
- Ocho pruebas CLI PASS: manifest exacto/allowlist; aceptación y rechazo tras avance sin mutación; identidad/scope/digest/frescura; corrupción de índices/cadena y refs de eventos falsas; cortes parciales 4/5 dentro del comando 4–6 y corte inexistente 7; límite de refs; replay original; framing/policy y DB ausente sin creación. El comando 4–6 incluye la invalidación normal del checkpoint anterior, conservando válido el corte histórico original.
- Snapshot concurrente: lectura WAL fijada en V1, escritor independiente confirma V2, la lectura conserva resultado/cabecera V1 y una nueva lectura ve V2. La prueba usa transacciones SQLite reales y la misma rutina de carga de producción. Límite: JSON de comando de 64 MiB + 1 byte se rechaza antes de decodificar.
- Pasada focalizada: ocho CLI y tres unitarias storage (dos nuevas y una preexistente ENOSPC) PASS, exit 0. Clippy de todo el workspace/all-targets/all-features con `-D warnings` PASS.

- Regresión ampliada de CLI, interop-gskills, projection-engine, storage-sqlite y transport-stdio, todos los targets/features: **153 PASS, 1 fallo, 9 omitidas**, 44 harnesses. El fallo preexistente `supervised_execution_is_bounded_private_and_uncertain_attempts_never_retry` recibió el timeout esperado de 700 ms, pero su auxiliar no alcanzó a crear el marcador (`operations.rs:1311`). La repetición aislada pasó, 1 PASS en 5.4 s. Se conservan ambos logs: no se declara corregida esa sensibilidad ni una pasada integral sin fallos. El código de supervisión y esa prueba permanecen intactos.
- Formato, Clippy workspace, schemas generados y preservación exacta de Cargo.lock/specs/schemas/perfiles anteriores PASS. La dependencia jsonschema existente pasa de dev a runtime directo en CLI, sin cambiar versiones ni lock.

Logs locales en `.tooling/pl-gsk-003-confirmation/`, excluidos de Git: `regression.log`, `operations-isolated.log`, `targeted-final.log`, `clippy-final.log`. Las nueve omisiones requieren helpers/entorno GSP o filesystem efímero, como en la base.

## Entrega optimizada y consumo recíproco

Build `cargo build --locked --release -p project-ledger-cli`, exit 0, 2m 18s. Copia inmutable `.tooling/pl-gsk-003-confirmation/delivery/project-ledger.exe`, SHA-256 `33f56f787af58812621eef443aba9f189c011050fa43e29bb7e1a173a0fa76a6`, versión declarada **0.2.0**, sin tag/release nuevo.

Contra esa copia exacta, seleccionada con `CARGO_BIN_EXE_project-ledger`, pasan ocho pruebas de confirmación y siete del sink anterior: **15 PASS**, sin fallos/omitidas. `specs verify`, `contracts verify`, `resources verify` y `release verify` también pasan con `--resources <raíz de este checkout>`; 484 recursos verificados. Es una copia de ejecutable para calificación local, no un paquete autónomo; el resultado de release verify no certifica el nuevo perfil ni compatibilidad universal.

Código Ledger entregado: `4f720e11e799fc868560feb2836125289e90777c`. G-Skills fijó su consumo en `integrations/project-ledger/confirmation-v0.1/gskills-consumption.json`, SHA-256 `ac6925d9cb1619e0a771cba250e1e63f73b9111b3bcab1432131f2bc07ea190f`, revalidado directamente desde Ledger. Código host `ace06001e6115cfc157edd0f973a3f252fd0d15e`, binario optimizado `636791b2ea3703165cff6117a32f2f5d296ec7033f7053e4b7e6aa695b8ff689`. El helper inicial `3a3537db63010ea1381bce0ffdf8bbce1f8f8b6afc0299e386cb103a5771bf05` queda preservado; el helper final se identifica abajo.

## Resultado bilateral final

Reporte G-Skills `release/evidence/pl-gsk-003a-final-qualification.json`, SHA-256 `dff6f447b74a6342a2b35cea726466af746766652ccdb7e8d100b181da679202`: **SCOPED_PASS, 9/9 PASS**, 77 invocaciones superiores registradas. Runner/helper fijados en `12694d7fef9b86256366dc86dbe21138cbb74a3c`, helper optimizado `fcb7227fda40299d7c98e3c1b7cbe687a1f7edfb19dff7ba0b5af143f8d42623`; binarios productivos Ledger y G-Skills permanecen los ya entregados.

- Cinco mappings, conflicto anterior a submit, refresco fallido, recuperación tras avance y replay/original PASS. El rechazo conocido seguido de avance también conserva su resultado e intento único.
- Aceptación con respuesta perdida, avance 22→23 y reinicio: `RECONCILED_ACCEPTED` / `ADVANCED_AFTER_RESULT`, digest original `sha256:5c0cdc52d2be58eb6d047bfc1f42a3a64536a102bdaf336d646a543c76cade50`, intento 1. Corte V22 y HEAD V23 comprobados por la nueva prueba nativa.
- Control sin opt-in: resultado conocido preservado, confirmación `UNAVAILABLE` y `OUTCOME_UNKNOWN`. No se obtiene permiso por el estado actual ni por la existencia del contrato candidato.
- Replay: dos entregas nativas inyectadas explícitamente por la fixture después de Validate, un envío/intento lógico del host, un único evento nuevo (21→22), cero reintentos ciegos. Observación `DUPLICATE_REPLAY` digest `sha256:0f959ae1637899ffdb4495abbe7ed20e3993b9d45479c1c4b755ee23cd212d21` conservada aparte; lookup durable `ACCEPTED` digest `sha256:17cd54b797c9ca49f736f03724b46c890b553599b5ab1ecf5f6f1723bf0ce2f5`, `RECONCILED_ACCEPTED` / `CONFIRMED_AT_RESULT`.
- El primer armado del replay aceptaba externamente antes de Validate y recibió correctamente StateMismatch. Se conserva esa corrida/diagnóstico. La corrección afectó únicamente la inyección de la fixture; no se debilitó la validación ni cambió el producto para obtener PASS.

Contraste directo desde Ledger: hash del reporte y cuatro ejecutables coincidentes; nueve estados PASS y nueve pares de snapshots antes/después de consultas idénticos; ambos lookups nativos y el resultado conservado por el host coinciden completos con el original. Se reconstruyeron independientemente, desde los bindings registrados, el digest del efecto y el de la respuesta nativa completa del caso histórico; ambos coinciden. Cero escrituras operativas/upstream; `release_certified=false`, `phase_6_closed=false`.

G-Skills comunica UB-51/53 VERIFIED en este alcance. PL-GSK-003 y su adenda quedan satisfechos por esta evidencia local bilateral; S24/Fase 6 continúan sujetos a sus demás gates. La incidencia de 700 ms de la suite nativa se conserva como limitación, no queda arreglada por la calificación conjunta.

## Límites de la entrega

La respuesta es evidencia del proveedor fijado por hash y autoridad local; no es firma ni prueba autónoma de la cadena. No certifica la verdad semántica de lo registrado ni importa STATE/episodios/SQLite al host G-Skills. Permanece fuera resolver resultados no persistidos durante una carrera de commit.

Rollback: retirar selección candidata y revertir commits locales preservando evidencia; no abrir una base de la línea histórica nueva con rc.1.
