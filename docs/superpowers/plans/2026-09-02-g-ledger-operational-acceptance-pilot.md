# G-Ledger Operational Acceptance Pilot Implementation Plan

> **For agentic workers:** Execute this plan inline, stage by stage. Use `test-driven-development` for harness or product behavior, `systematic-debugging` for unexpected results, and `verification-before-completion` before every commit or closure. Do not delegate the governed pilot state.

**Goal:** Construir y ejecutar un metaproyecto descartable que determine con evidencia operacional si G-Ledger 0.2.0 puede adoptarse para el primer proyecto real.

**Architecture:** La especificación y la matriz viven en documentación versionada; el ZIP publicado es el sujeto inmutable. Un runner PowerShell y un cliente de ensayo no distribuido conducen exclusivamente la CLI/stdio, mientras DB, exports y salida cruda permanecen en `.tooling/`. El reporte sanitizado conserva resultados, no secretos ni autoridad inventada.

**Tech Stack:** Rust 1.97.1 / edition 2024, PowerShell 7, SQLite/WAL a través de la CLI, JSON canónico, JSON-RPC stdio y Python/GSP fijado sólo en la etapa 7.

## Global Constraints

- Autoridad de producto: `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`.
- Autoridad del piloto: `docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md`.
- Sujetos invalidados: ZIP inicial `a83624f2e795c8b8d3d9e15a316be38cce8999e437731b73f5f8c8964963ad80`, fuente `b171f51cb58c60ed8c8a99b60386117f9fed6822`; y ZIP `cf504e128386e782cc394c9c30bef699f94dd4be97ccd7e252402029bbfdcad0`, fuente `bd564cdba07b616337681c15801d22478824108b`, invalidado por OPA-DEFECT-003. Sujeto vigente recalificado: ZIP alojado `e2509d48abefe2211261b645604d4b08f13d5751d182ad158a0059d3c222a851`, fuente exacta `99fdaab52bcec7003bab02e4cfee45059a4bf737`.
- No abrir SQLite, editar Markdown generado, reutilizar destinos, bajar gates, ampliar contratos ni atribuir autoridad a Codex/GSP.
- Cada etapa termina con prueba fresca, reporte, bitácora y commit; un fallo detiene el contador.
- Las ocho etapas comparten una fase Ledger `phase:operational-acceptance`; sus ocho gates se verifican progresivamente y la fase sólo se cierra en Task 8.

---

### Task 0: Autoridad y baseline documental

**Files:** spec, matriz, reporte inicial, este plan, README y bitácora.

**Produces:** estado `NOT_STARTED`, 0/8, sin evidencia del piloto atribuida.

- [x] Verificar checkout limpio, branch y hashes del artefacto.
- [x] Persistir autoridad, matriz y reporte JSON parseable.
- [x] Validar links, diff y consistencia 0/8; commit `b4b0c64774bf2eb7fe79a8471e9b2f75b14a05a2`.

### Task 1: Runner — RED/GREEN y preflight

**Files:** `scripts/run-operational-pilot.ps1`, pruebas PowerShell bajo `scripts/tests/` y fixtures del piloto.

**Interface:** `run-operational-pilot.ps1 -LedgerZip <zip> -PilotRoot <new-dir> -Stage <1..8> [-GspPython <exe> -GspSourceRoot <dir>]`.

- [x] RED/GREEN: probar hash incorrecto y destino existente; validar binario, versión/capacidades y evitar uso de `target` durante el preflight real.
- [x] GREEN: implementar extracción nueva, entorno mínimo, captura sanitizada y manifest de evidencia.
- [x] Ejecutar Stage 1 desde el ZIP hosted exacto, inicializar y configurar el proyecto mediante CLI; validar S01…S03.
- [x] Actualizar 1/8 en el commit `fd92e43134b779ddd09613fb74924d82b3259937`.

### Task 2: Gobierno del proyecto

- [x] Ejecutar diez queries con tupla versión/digest antes/después y delta cero.
- [x] Probar `exit 9` y delta cero para un agente, más refresh/supersesión auxiliar humana autorizada con historia preservada.
- [x] Vincular un repositorio-canary declarativamente, registrar artifact por agente y verificar que archivos/pruebas no cierran lifecycle; validar S04…S07.
- [x] Actualizar 2/8 y registrar el cierre en el commit independiente de Stage 2.

### Task 3: Bitácora y continuidad

- [x] Registrar objetos materiales definidos por S08 con claves únicas y versiones frescas.
- [x] Resolver/mitigar/supersede y comprobar preservación mediante vistas.
- [x] Crear checkpoint, cerrar sesión y verificar resume JSON/Codex; validar S08…S10.
- [x] Recalificar OPA-DEFECT-003 mediante CI alojada 6/6 y repetir Stage 1→3 desde el ZIP exacto en un proyecto nuevo.
- [x] Actualizar 3/8 y commit independiente de cierre de continuidad.

### Task 4: Evidencia, gates y RecordSink

**Files:** cliente de ensayo no distribuido, fixtures exactos de policy/comando y tests de proceso real.

- [x] RED: el cliente aún ausente debe impedir demostrar validate/submit/status/reconcile sobre el ZIP.
- [x] GREEN: implementar framing Content-Length, digests canónicos y fixture dinámico sin acceso SQLite.
- [x] Probar OBSERVED incompleto, VERIFIED_SOURCE por sink, gate/phase, stale/revalidation y waiver opcional; validar S11…S15.
- [x] Actualizar 4/8 y commit `test: validate evidence and lifecycle boundaries`.

### Task 5: Idempotencia y reconciliación

- [x] Ejecutar replay, key conflict, version conflict y policy denial con delta cero.
- [x] Descartar stdout de checkpoint y reconciliar el scope original; consultar key inexistente.
- [x] Validar S16…S21 sin retry ciego.
- [x] Actualizar 5/8 y commit `test: validate transactional safety`.

### Task 6: Proyecciones, privacidad y recuperación

- [x] Renderizar, alterar copia y demostrar divergencia/regeneración.
- [x] Ejecutar portable roundtrip y backup/restore; rechazar corrupción y overwrite.
- [x] Escanear superficies soportadas contra canaries; ejecutar doctor/verify/audit.
- [x] Validar S22…S27, actualizar 6/8 y commit `test: validate recovery privacy and projections`.

### Task 7: Handoff ciego e integraciones

- [x] Crear una tarea Codex nueva sólo con binario, database, project_ref y skill.
- [x] Verificar su resume, avance material y checkpoint sin contexto narrativo.
- [x] Ejecutar provider/RecordSink local y GSP fijado con deltas contractuales; registrar Radar excluido.
- [x] Validar S28…S31, actualizar 7/8 y commit `test: validate blind handoff and bounded integrations`.

### Task 8: Decisión de adopción

- [ ] Ejecutar auditoría final, verificar freshness y conciliar todas las filas.
- [ ] Ejecutar el gate completo del repositorio si hubo cambios de código; ante corrección, recalificar artefacto y repetir etapas afectadas.
- [ ] Presentar resultados al propietario sin prellenar aprobación.
- [ ] Tras decisión humana explícita, registrar S32 y el estado correcto; actualizar 8/8 sólo si satisface la fórmula.
- [ ] Commit final `docs: record operational pilot decision`.

## Failure protocol

Ante un resultado inesperado: detener etapa, preservar evidencia sanitizada, abrir blocker, aplicar `systematic-debugging`, reproducir RED y corregir sólo dentro de contratos v0.2. Un nuevo candidato invalida evidencia posterior del anterior. No hacer push ni release sin solicitud explícita.

## 2026-09-03 — Stage 4 ejecutado y conciliado, 4/8

- RED real: la suite falló con `PILOT_STAGE_NOT_IMPLEMENTED: stage 4`. El cliente no distribuido quedó separado en `scripts/invoke-record-sink-rpc.ps1`; usa framing `Content-Length` por bytes UTF-8 y emite exactamente un documento JSON.
- GREEN: 6/6 pruebas PowerShell pasaron, incluido el recorrido completo Stage 1→4 sobre una copia temporal. Los errores intermedios del harness se aislaron con depuración sistemática y no exigieron modificar el producto, sus contratos, schemas, locks, 53 comandos ni 67 eventos.
- Ejecución viva: el proyecto `project:01a0697c-34b6-7042-a751-fbfb30a06a49` y el ZIP alojado `e2509d48abefe2211261b645604d4b08f13d5751d182ad158a0059d3c222a851` pasaron OPA-S11…OPA-S15. `gate:opa-04` quedó `VERIFIED`; el gate opcional quedó `WAIVED`; el gate crítico `gate:opa-08` permaneció `PENDING` tras un rechazo exit 3/delta cero.
- Cierre controlado: state version 66, digest `sha256:5780053af5bcb8db14268170a825a5eaaf9cd0f38e6afb794ca64dd08aa85975`, checkpoint `checkpoint:74eefb11-ad42-7d14-ac57-b1efea643b4b` `CURRENT`, fase activa y cero fases completadas. `doctor`, `verify` y `audit` informaron integridad `ok`, cero findings y cero submissions sin resolver.
- Límite: `blocker:opa-stage5-transaction` mantiene el proyecto `BLOCKED`; `action:opa-stage5-transaction` sólo es `RECOMMENDED`. No se ejecutaron Stage 5, merge, release ni aceptación operativa.

**Siguiente paso:** solicitar autorización humana explícita para Stage 5. Sin ella, no ejecutar replay/conflict/version/outcome-unknown del piloto ni cambiar el contador de 4/8.

## 2026-09-03 — Stage 5 ejecutado y conciliado, 5/8

- RED/GREEN: Stage 5 comenzó con `PILOT_STAGE_NOT_IMPLEMENTED: stage 5`. Tras implementar el harness, la suite completa pasó 7/7 sobre proyectos temporales nuevos. Las discrepancias intermedias fueron del harness: URI relativa inválida, digest distinto y normativo para `DUPLICATE_REPLAY`, invalidación adicional del checkpoint al resolver el único blocker, y lectura del head digest desde la superficie incorrecta. No se cambió G-Ledger ni sus contratos.
- Ejecución viva: el proyecto `project:01a0697c-34b6-7042-a751-fbfb30a06a49` y el ZIP alojado `e2509d48abefe2211261b645604d4b08f13d5751d182ad158a0059d3c222a851` pasaron OPA-S16…OPA-S21. Replay exacto: `DUPLICATE_REPLAY`, mismo `command_result_id`, delta cero. Conflicto de intención: exit 10/delta cero. Versión obsoleta: exit 5/delta cero, seguida de `resume` y comando replanificado con clave nueva.
- Autoridad y resultado incierto: un agente recibió exit 9/delta cero al intentar registrar autoridad y no se añadió `--as-human`. Se descartó stdout de un checkpoint aceptado, se reconcilió una sola vez con `agent:opa-stage5`, `CREATE_CONTINUITY_CHECKPOINT`, `LOCAL_USER` y la clave original, sin reenvío. La consulta de una clave inexistente devolvió exit 12/delta cero y no provocó retry ciego.
- Cierre controlado: state version 76, digest `sha256:b6ba0140df804e9227ea8ec3c07be92a899dc9f8b5947ea1044079bda7a40cd0`, checkpoint `checkpoint:98e9fdf3-2ff8-7274-8b9e-0df11b9bd6a4` `CURRENT`, fase activa y cero fases completadas. `doctor`, `verify` y `audit` informaron integridad `ok`, cero findings y cero submissions sin resolver.
- Límite: `blocker:opa-stage6-recovery` mantiene el proyecto `BLOCKED`; `action:opa-stage6-recovery` sólo es `ELIGIBLE` para solicitar autorización. No se ejecutaron Stage 6, merge, release ni aceptación operativa.

**Siguiente paso:** solicitar autorización humana explícita para Stage 6. Sin ella, no ejecutar proyecciones, portabilidad, privacidad, backup/restore ni cambiar el contador de 5/8.

## 2026-09-03 — Stage 6 ejecutado y conciliado, 6/8

- RED/GREEN: Stage 6 comenzó con `PILOT_STAGE_NOT_IMPLEMENTED: stage 6`. La suite incorporó pruebas de proyección, portabilidad, backup/restore, corrupción, privacidad y una regresión específica de reanudación tras interrupción; el cierre final exige 9/9 pruebas PowerShell.
- Proyección: un Markdown `SHAREABLE` verificó contra el stream, una copia alterada devolvió exit 15/`GENERATED_ARTIFACT_DIVERGED`, y `--regenerate` reconstruyó bytes idénticos sin mutar versión ni digest. Un destino ya existente devolvió exit 2 y permaneció intacto.
- Portabilidad: el proyecto principal contiene locators absolutos privados y su export portable falla cerrado en vez de reescribir historia. Por ello el round-trip normativo se ejecutó en `project:01a069e2-bd81-73b1-a654-719bb6444e3d`, Ledger auxiliar limpio y descartable: `NEW_PROJECT`, estado/digest/head/resume iguales, reimport idéntico `IDENTICAL_CHAIN` con delta cero, y corrupción/divergencia/overwrite rechazados sin alterar destino.
- Recuperación completa: backup/restore SQLite del proyecto gobernado conservó estado, digest, head, resultados e idempotencia. `reconcile` recuperó el progreso original y su repetición devolvió `DUPLICATE_REPLAY` con delta cero; corrupción y destinos existentes se rechazaron sin sobrescritura.
- Privacidad: dos canaries ficticios produjeron evidencia aceptada con `raw_output_stored=false`; no aparecieron en `resume`, Markdown compartible, bundle limpio ni reporte sanitizado. El export posterior del historial privado devolvió exit 13/`PRIVACY_REJECTED` y no creó destino. Esto cubre superficies soportadas, no ausencia universal en bytes internos.
- Interrupción controlada: el primer intento real alcanzó sólo el registro de progreso y falló al crear la DB auxiliar por longitud de ruta Windows. El resultado aceptado se reconcilió con el scope original; no se reenvió ciegamente, no se reescribió historia y el runtime parcial se conservó. El harness selecciona ahora un scratch corto bajo `.tooling/g-ledger-operational-pilot/` y la regresión correspondiente pasa.
- Cierre controlado: state version 85, digest `sha256:a48d21a8d594bba4ebe9fa00b298c156726599a2359f46faa324d0bb0c7c9652`, head `sha256:1e6854daebda895e97e2c6138a69d6cfa7061c04bc31aeeafdecba7b65c76342` y checkpoint `checkpoint:624442ea-7360-77df-9f92-ddc4e73fde32` `CURRENT`. `doctor`, `verify` y `audit` quedaron `ok`, sin findings ni submissions sin resolver.
- Límite: `blocker:opa-stage7-integrations` mantiene el proyecto `BLOCKED`; `action:opa-stage-07-integrations` es `ELIGIBLE`, no `AUTHORIZED`. No se ejecutaron Stage 7, merge, release ni aceptación operativa.

**Siguiente paso:** solicitar autorización humana explícita para Stage 7. Esa autorización deberá incluir la creación de una tarea Codex nueva y ciega, acotada al ejecutable, database, project_ref y skill. Sin ella, no iniciar el handoff ni las integraciones.

## 2026-09-04 — Stage 7 ejecutado y conciliado, 7/8

- RED/GREEN: Stage 7 comenzó con `PILOT_STAGE_NOT_IMPLEMENTED: stage 7`; la prueba dedicada terminó GREEN y cubre el handoff, provider/RecordSink, GSP exacto, Radar excluido y el cierre fail-closed hacia Stage 8.
- Handoff ciego: la tarea Codex `01a06a10-c398-7b22-b749-7515fe66efea` recibió exactamente ejecutable, database, project_ref y skill. Sin relato previo, reconstruyó v85, fase, gates, blocker, decisión, acción y checkpoint; registró un progreso v85→v86 y un checkpoint v86→v87, ambos reconciliados con sus claves originales. No usó Git, SQLite directo, `--as-human` ni comandos de lifecycle.
- Contención de orquestación: por visibilidad tardía del primer task se creó un reintento local redundante. Se abortó antes de mutar G-Ledger y se excluyó de la evidencia; no se reenvió ninguna clave ni se duplicó efecto durable.
- Integraciones locales: el provider G-Skills rechazó una llamada mutante con JSON-RPC `-32601` y mantuvo delta cero. El RecordSink expuso sólo cinco command kinds, no reclamó autoridad, aceptó una mutación explícita, devolvió `DUPLICATE_REPLAY` y conservó status/reconcile `FOUND` con deltas uno/cero.
- GSP fijado: commit `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, contrato `gsp-exact-2026.04.12` y commit contractual `004b14d7b64837025671970eac2469f092c49475`. Export/compile/validate/link/replay produjeron deltas 0/0/0/1/0; no hubo elevación de autoridad, cambio de fase ni certificación. Project Radar quedó `EXCLUDED_BY_SCOPE`, nunca PASS.
- Cierre controlado: OPA-S28…OPA-S30 pasan; OPA-S31 conserva exclusión. Estado v96, digest `sha256:679f8497e36101d6169353d9ddaac500331e8dbab98101ff7561e98ef96b622a`, checkpoint `checkpoint:5c1cd93b-dffe-7b92-878c-171e71602819` `CURRENT`, fase activa y cero fases completadas. `doctor`, `verify` y `audit` quedaron `ok`, sin findings.
- Límite: `blocker:opa-stage8-human-decision` mantiene el proyecto `BLOCKED`; `action:opa-next-stage8-decision` identifica el siguiente punto de control, pero no autoriza Stage 8. No hubo merge, release, aceptación operativa, GA ni certificación externa.

**Siguiente paso:** solicitar autorización humana explícita para ejecutar la conciliación de Stage 8. La decisión final `OPERATIONALLY_ACCEPTED`, `BLOCKED` o `REJECTED` requerirá después una autorización humana separada basada en los resultados presentados.
