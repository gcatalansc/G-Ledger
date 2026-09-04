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

**Interface:** `run-operational-pilot.ps1 -LedgerZip <zip> -PilotRoot <new-dir> -Stage <1..8> [-GspPython <exe> -GspSourceRoot <dir>] [-OwnerDecision OPERATIONALLY_ACCEPTED]`.

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

- [x] Ejecutar auditoría final, verificar freshness y conciliar todas las filas.
- [x] Ejecutar el gate completo del repositorio si hubo cambios de código; ante corrección, recalificar artefacto y repetir etapas afectadas.
- [x] Presentar resultados al propietario sin prellenar aprobación.
- [x] Tras decisión humana explícita, registrar S32 y el estado correcto; actualizar 8/8 sólo si satisface la fórmula.
- [x] Commit final `docs: record operational pilot decision`.

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

## 2026-09-03 — Stage 8 conciliado para decisión, 7/8

- RED/GREEN: la prueba dedicada comenzó con `PILOT_STAGE_NOT_IMPLEMENTED: stage 8` y terminó GREEN. El runner exige sujeto, etapas, escenarios, defectos, identidad y límites exactos antes de emitir `READY_FOR_OWNER_DECISION`.
- Ejecución viva: la conciliación usó exclusivamente el ZIP alojado `e2509d48abefe2211261b645604d4b08f13d5751d182ad158a0059d3c222a851` y `project:01a0697c-34b6-7042-a751-fbfb30a06a49`. Leyó la matriz, el reporte, las siete evidencias de etapa, `status`, `resume`, los gates, el checkpoint y `doctor`/`verify`/`audit`.
- Resultado: siete etapas y OPA-S01…OPA-S30 están `PASS`; OPA-S31 continúa `EXCLUDED_BY_SCOPE`; los tres defectos están `FIXED_AND_REQUALIFIED`; hay cero defectos abiertos, conflictos, unknowns críticos, waivers críticos y findings.
- Inmutabilidad: la reconciliación fue read-only y mantuvo versión 96, digest `sha256:679f8497e36101d6169353d9ddaac500331e8dbab98101ff7561e98ef96b622a` y checkpoint `checkpoint:5c1cd93b-dffe-7b92-878c-171e71602819` `CURRENT`, con delta cero.
- Verificación del repositorio: 11/11 pruebas PowerShell, formato, Clippy, workspace completo, schemas, specs, contratos, build release locked, release check/verify y `cargo deny` pasaron. Sólo cambió el harness y la documentación; no hubo cambio de producto, contratos ni lock, por lo que el candidato exacto no requirió recalificación.
- Límite humano: OPA-S32 queda `AWAITING_OWNER_DECISION`; `blocker:opa-stage8-human-decision`, la fase activa, cero fases completadas, un gate verificado y siete gates pendientes se preservan. Avance oficial: **7/8, 87,5 %, READY_FOR_OWNER_DECISION**. No se declaró aceptación, merge, release, GA ni certificación externa.

**Siguiente paso:** presentar este resultado al propietario. Sólo una decisión humana posterior y separada podrá autorizar la transacción final, resolver el blocker, evaluar gates, cerrar la fase y registrar `OPERATIONALLY_ACCEPTED`, `BLOCKED` o `REJECTED`.

## 2026-09-03 local / 2026-09-04 UTC — Cierre histórico 8/8, invalidado después por OPA-DEFECT-004

- Autoridad: tras recibir el informe final, `human:owner` decidió separadamente `OPERATIONALLY_ACCEPTED` para G-Ledger. El runner exige esa decisión exacta mediante `-OwnerDecision`; otros valores no se interpretan ni se convierten implícitamente.
- TDD y depuración: el RED inicial demostró que el runner carecía de la interfaz de decisión. Los fallos intermedios se localizaron en el harness —estado propuesto con autoridad prematura, colisión de `command_id`, selector de fase y cierre lifecycle de dos pasos— y se corrigieron sin modificar G-Ledger, sus 53 comandos, 67 eventos, contratos, schemas o locks.
- Ensayo seguro: antes de mutar el proyecto autorizado se clonó el estado exacto v96 a un destino descartable y se ejecutó la transacción completa. El ensayo terminó `OPERATIONALLY_ACCEPTED`; luego la misma secuencia se ejecutó una sola vez sobre el proyecto gobernado.
- Evidencia y gates: siete registros `VERIFIED_SOURCE` se enviaron mediante la superficie RecordSink local, cada uno con efecto durable uno y replay `DUPLICATE_REPLAY`/delta cero. Gate Engine verificó `gate:opa-01`…`gate:opa-08`; ningún gate requerido fue dispensado y el waiver se mantuvo limitado al gate opcional de ejercicio.
- Lifecycle: la decisión `decision:opa-operational-acceptance-v1` quedó `ACCEPTED`; se resolvió `blocker:opa-stage8-human-decision`; la solicitud del agente dejó la fase `READY_TO_CLOSE` y la confirmación humana separada permitió a Phase Engine completar fase y proyecto. La próxima acción es `action:opa-first-controlled-project`.
- Conciliación final: v134, digest `sha256:bf493e895aef07482f05513e5da89cdd3acaa8001c3971103bb7110ab835e882`, checkpoint `checkpoint:69e46f7b-15dc-7ca3-b89a-bc3a259ca901` `CURRENT`, ocho gates requeridos verificados, cero pendientes, cero waivers requeridos, cero blockers y cero findings de `doctor`, `verify` y `audit`. `risk:opa-stage3-continuity` conserva su historia `MITIGATED` como riesgo residual monitorizado y no bloqueante. OPA-S32 pasa como `LIVE_PILOT_HUMAN_DECISION`.
- Límite de producto: el producto bajo prueba y aceptado es exclusivamente G-Ledger. G-Skills permanece en desarrollo y no fue validado como producto; su superficie local sólo actuó como contraparte de desarrollo para probar las fronteras de integración de G-Ledger. Project Radar sigue `EXCLUDED_BY_SCOPE`, `certified_compatibility=false`, y no hubo merge, release ni declaración GA.

**Resultado histórico, no vigente:** **8/8, 100 %, `OPERATIONALLY_ACCEPTED` para uso operativo controlado de G-Ledger**. OPA-DEFECT-004 invalidó posteriormente este sujeto; la sección siguiente gobierna el estado actual.

## 2026-09-04 — Reapertura gobernada por OPA-DEFECT-004, 2/8

- Hallazgo: la permanencia de `risk:opa-stage3-continuity` en `open_risk_refs` era coherente con el estado `MITIGATED`, pero reveló una omisión del CLI público: el contrato v0.2, el lifecycle y la proyección ya soportaban `CLOSE_RISK`, mientras `risk` sólo exponía `open` y `mitigate`.
- TDD: `risk_close_is_public_idempotent_and_removes_a_mitigated_risk_from_resume` reprodujo RED con exit 2 y `unrecognized subcommand 'close'`. El fix mínimo `138eabe715d209da5f527340a34492bff3f6fd1a` quedó GREEN: cierre aceptado, replay `DUPLICATE_REPLAY`, cero riesgos abiertos en `status`/`resume` y cero findings de audit.
- Harness: `3696ea69ae9b03b7866c5c5cd48ac7d2e91deabe` exige ahora la secuencia `OPEN_RISK` → `MITIGATE_RISK` → `CLOSE_RISK`, conserva ambos eventos y falla Stage 8 si existe cualquier `open_risk_ref`. La cadena local 1–8 pasó 11/11 sobre el paquete exacto local SHA-256 `ff6c4ef49d1d2e206d27740fc97c45dc8bdca79e02117e32a6fcf851f0f6e8cc`.
- Verificación: formato, Clippy estricto, workspace completo, schemas, specs, contratos, build locked, release check/verify, `cargo deny`, paquete Windows y dos harness externos pasaron. No cambiaron los 53 comandos normativos, 67 eventos, schemas, locks ni contratos v0.2.
- Gobierno: el ZIP alojado `e2509d48…a851` queda invalidado como sujeto vigente. Su evidencia y la decisión humana anterior permanecen históricas, pero no se transfieren. El estado actual vuelve a **`BLOCKED`, 2/8, 25 %**; Stage 3…Stage 8 y la decisión humana final deben repetirse sobre un ZIP alojado exacto nuevo.

**Siguiente paso:** solicitar autorización explícita para publicar una rama candidata exacta apuntando a `138eabe`, abrir un PR privado draft exclusivamente para activar la CI de recalificación, descargar/verificar su ZIP alojado y ejecutar Stage 3/8. No se autoriza implícitamente Stage 4, merge, release ni aceptación operativa.

### Calificación alojada completada; ejecución Stage 3 pendiente de precondición autorizada

- PR privado draft: `https://github.com/gcatalansc/G-Ledger/pull/4`. El primer run `33836827729` pasó 6/6, pero su paquete declaró el merge sintético `4f72c1a6…`; se rechazó antes de cualquier mutación del piloto.
- El commit calificador separado `97507a39cb70e0f6bc044123a9d7cefcf64ab412` fijó los cuatro checkouts al candidato `138eabe715d209da5f527340a34492bff3f6fd1a`. No modificó producto ni contratos.
- El run exacto `33838359314` pasó 6/6. El artefacto `9924403634` tiene digest GitHub `sha256:bd36d90840b102f6bf6515ef06890b93b0c508302540c575964e652e922cc6f2`; su ZIP interior tiene SHA-256 `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`, declara `source_commit=138eabe…`, árbol limpio y pasó 2/2 harness externos en una reverificación independiente.
- Stage 3 no se ejecutó. El runner exige un Ledger fresco exactamente en el estado terminal de Stage 2; el proyecto histórico está completado en v134 y no es reutilizable como esa precondición. Crear el nuevo estado ejecutaría Stage 1 y Stage 2 con sus ceremonias humanas, alcance que la autorización vigente excluyó expresamente.

**Siguiente paso:** obtener autorización explícita para ejecutar Stage 1 y Stage 2 únicamente como bootstrap fresco en una carpeta descartable nueva usando el ZIP alojado `7160d31d…0162`, y continuar después con Stage 3/8. Stage 4, merge, release y aceptación operativa permanecen fuera de alcance.

## 2026-09-04 — Recalificación Stage 1→3 ejecutada y conciliada, 3/8

- Autoridad: se autorizó Stage 1 y Stage 2 sólo como bootstrap fresco y, posteriormente, Stage 3; también se autorizaron las ceremonias `--as-human` documentadas para esas etapas. Stage 4, merge, release y aceptación operativa quedaron expresamente excluidos.
- Sujeto exacto: se utilizó únicamente el ZIP alojado SHA-256 `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`, fuente `138eabe715d209da5f527340a34492bff3f6fd1a`, árbol limpio y `certified_compatibility=false`.
- Bootstrap fresco: el destino `.tooling/g-ledger-operational-pilot/rq hosted 138eabe á/` contiene espacios y carácter no ASCII. Creó `project:01a06adc-c603-7291-aabe-240d71809978`; Stage 1 cerró v15/digest `sha256:63317211…dfec512` y Stage 2 v21/digest `sha256:4fe7fb6b…f1d758`, con lecturas delta cero, fase activa, cero fases completadas y cero gates verificados.
- Continuidad: Stage 3 avanzó v21→v45, registró únicamente los records y transiciones previstos, cerró la sesión y creó `checkpoint:843caa1c-1721-7d35-abe7-1590596b1824` `CURRENT`. La decisión predecesora, resolución de unknown/blocker y eventos `RISK_MITIGATED`/`RISK_CLOSED` permanecen en el timeline; `resume.open_risk_refs` está vacío.
- Reconciliación independiente: `status`, `doctor`, `verify`, `audit`, `resume` JSON/Codex y un segundo `status` conservaron versión, digest y checkpoint con delta cero. La integridad es `ok`; el único finding `CriticalUnknownOpen` corresponde a `unknown:opa-stage4-provider-policy`, precondición fail-closed esperada para la etapa no autorizada.
- Observación ambiental: un primer destino descartado generó rutas de `ledger.db-wal`/`ledger.db-shm` de 261 caracteres y SQLite falló con exit 11 antes de inicializar el Ledger. No se reutilizó. La ruta normativa exitosa —espacios y Unicode, sidecar proyectado de 222 caracteres— pasó; la compatibilidad con rutas Win32 heredadas mayores de 260 queda registrada como limitación externa al escenario, no como PASS ni como defecto corregido.
- Resultado: OPA-S01…OPA-S10 `PASS`, OPA-DEFECT-004 `FIXED_AND_REQUALIFIED`, 0 riesgos abiertos y avance **3/8, 37,5 %, `IN_PROGRESS`**. La evidencia sanitizada está en `docs/pilot/results/stage-3-risk-close-hosted-requalification.json`.

**Siguiente paso:** solicitar autorización humana explícita para Stage 4/8 sobre este proyecto descartable y el mismo ZIP alojado exacto. No iniciar Stage 5, merge, release ni aceptación operativa.

## 2026-09-04 — Recalificación Stage 4 ejecutada y conciliada, 4/8

- Autoridad: se ejecutó únicamente Stage 4 sobre `project:01a06adc-c603-7291-aabe-240d71809978` y el ZIP alojado `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`. Stage 5, merge, release y aceptación operativa permanecieron fuera de alcance.
- Evidencia fail-closed: una evidencia `OBSERVED` con exit 0 mantuvo `gate:opa-04` sin verificar; Gate Engine devolvió exit 6 y Phase Engine mantuvo la fase activa con exit 6.
- RecordSink local: policy digest `sha256:1f2d9037c0e60b8de128fc3f10fc63e072918e006bbc199d7a586652f9f8862c`; validate `VALID`, submit `ACCEPTED`, replay `DUPLICATE_REPLAY`, status/reconcile `FOUND`. La evidencia `VERIFIED_SOURCE` produjo exactamente un efecto durable y el replay delta cero, sin almacenar salida cruda ni reclamar autoridad lifecycle.
- Gate, freshness y waiver: `gate:opa-04` quedó `VERIFIED`; un cambio auxiliar volvió el gate `REVALIDATION_REQUIRED` y el checkpoint `STALE_FOR_AUTHORITY`; la revalidación devolvió el gate a `VERIFIED` y el checkpoint reemplazante `checkpoint:8d613d30-08a6-7700-ae19-a1ec534cf9ef` quedó `CURRENT`. El gate opcional quedó `WAIVED`; `gate:opa-08` rechazó waiver con exit 3 y delta cero.
- Cierre controlado: estado v67, digest `sha256:33d16f221947c155a01aec1be68c360198bbdf8c27b6afd8a721f63e23be22c0`, proyecto `BLOCKED`, fase activa, cero fases completadas, siete gates requeridos pendientes, cero riesgos y cero unknowns críticos. `doctor`, `verify` y `audit` informaron integridad `ok`, cero findings y cero submissions sin resolver; la reconciliación completa tuvo delta cero.
- Resultado: OPA-S11…OPA-S15 `PASS`; avance **4/8, 50 %, `IN_PROGRESS`**. La evidencia sanitizada está en `docs/pilot/results/stage-4-risk-close-hosted-requalification.json`.

**Siguiente paso:** solicitar autorización humana explícita para Stage 5/8 sobre el mismo proyecto y ZIP alojado exacto. No iniciar Stage 6, merge, release ni aceptación operativa.

## 2026-09-04 — Recalificación Stage 5 ejecutada y conciliada, 5/8

- Autoridad: se ejecutó únicamente Stage 5 sobre `project:01a06adc-c603-7291-aabe-240d71809978` y el ZIP alojado `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`. Stage 6, merge, release y aceptación operativa permanecieron fuera de alcance.
- Idempotencia: el primer `RECORD_PROGRESS` fue `ACCEPTED`; la repetición idéntica devolvió `DUPLICATE_REPLAY`, mismo `command_result_id`, delta cero y digest sin cambio. La misma clave con intención distinta devolvió exit 10/`IDEMPOTENCY_CONFLICT`, también con delta cero.
- Concurrencia y autoridad: una versión obsoleta devolvió exit 5/`VERSION_CONFLICT`; luego de `resume`, el comando fue replanificado con expected version 68 y clave nueva, y fue aceptado. El intento de registrar autoridad por un agente devolvió exit 9/`POLICY_DENIED`; no se añadió `--as-human` y no hubo mutación.
- Resultado incierto: la respuesta de un checkpoint se descartó deliberadamente tras un único envío. La reconciliación con actor, tipo, trust domain y clave originales devolvió `RECONCILED`/`ACCEPTED` y delta cero. Una clave inexistente devolvió exit 12/`OUTCOME_UNKNOWN`, sin reenvío ciego.
- Cierre controlado: v77, digest `sha256:de544dfb9aa6e1a7e79d9bc1b5af0f0e446de53233e8cebc08e540ce60dba3a0`, head `sha256:777eac22e45058b0817dd85953d4feda7bf6829dc4782e2d85ce4b16ecdd07fc` y checkpoint `checkpoint:55d95cec-d39a-748c-b53f-0107d6618c84` `CURRENT`. Proyecto `BLOCKED`, fase activa, cero fases completadas, cero riesgos/unknowns críticos; `doctor`, `verify` y `audit` quedaron `ok`, sin findings ni submissions sin resolver.
- Resultado: OPA-S16…OPA-S21 `PASS`; avance **5/8, 62,5 %, `IN_PROGRESS`**. Evidencia sanitizada: `docs/pilot/results/stage-5-risk-close-hosted-requalification.json`.

**Siguiente paso:** solicitar autorización humana explícita para Stage 6/8 sobre el mismo proyecto y ZIP alojado exacto. No iniciar Stage 7, merge, release ni aceptación operativa.

## 2026-09-04 — Recalificación Stage 6 ejecutada y conciliada, 6/8

- Autoridad: se ejecutó únicamente Stage 6 sobre `project:01a06adc-c603-7291-aabe-240d71809978` y el ZIP alojado exacto `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`. Stage 7, merge, release y aceptación operativa permanecieron fuera de alcance.
- Proyección: el render `SHAREABLE` fue verificado; una alteración de copia se detectó con exit 15 y la regeneración volvió a coincidir con la vista autoritativa. Un destino existente devolvió exit 2 sin overwrite. Render, verificación y regeneración conservaron versión y digest con delta cero; Markdown no adquirió autoridad.
- Portabilidad y recuperación: el roundtrip portable se ejecutó sobre un proyecto auxiliar descartable y limpio de locators privados, conservó versión, digest, head, historia y `resume`; por contrato portable no restaura resultados de comando ni reservations. El backup/restore SQLite sí conservó resultados e idempotencia: reconcile `RECONCILED`/`ACCEPTED`, replay `DUPLICATE_REPLAY`, delta cero.
- Fallo cerrado: checksum corrupto, cadena divergente y destinos existentes se rechazaron sin sobrescritura ni mutación del destino. Tras introducir dos canaries, `resume`, render, roundtrip limpio, rechazo portable posterior y manifiesto sanitizado registraron cero coincidencias; no se afirma inexistencia universal en bytes internos. La salida cruda no fue almacenada.
- Cierre controlado: estado v86, digest `sha256:639494490d93f7b406f20d4d15c289b67f99148986c29d01dd5d517df6872e45`, head `sha256:0e47e907229130d160e760d5f4d8f8d6fc85522dba433f21af4fec6cc89b941c` y checkpoint `checkpoint:167b6373-8352-7747-ad36-042c618f295d` `CURRENT`. Proyecto `BLOCKED` por `blocker:opa-stage7-integrations`, fase activa, cero fases completadas, cero riesgos/unknowns críticos; `doctor`, `verify` y `audit` quedaron `ok`, sin findings ni submissions sin resolver.
- Resultado: OPA-S22…OPA-S27 `PASS`; avance **6/8, 75 %, `IN_PROGRESS`**. Evidencia sanitizada: `docs/pilot/results/stage-6-risk-close-hosted-requalification.json`.

**Siguiente paso:** obtener autorización explícita para Stage 7/8, incluida la tarea Codex nueva y ciega acotada al ejecutable, database, project_ref y skill, sobre este mismo proyecto y ZIP. Stage 8, merge, release y aceptación operativa permanecen fuera de alcance.

## 2026-09-04 — Recalificación Stage 7 ejecutada y conciliada, 7/8

- Autoridad: se ejecutó únicamente Stage 7 sobre `project:01a06adc-c603-7291-aabe-240d71809978` y el ZIP alojado exacto `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`. Stage 8, merge, release y aceptación operativa permanecieron fuera de alcance.
- Handoff ciego: la tarea Codex projectless `task:01a06c53-2603-7be0-b94b-c1f7a7eb57a7` recibió exactamente ejecutable, database, project_ref y skill. Sin relato previo reconstruyó v86, fase, siete gates pendientes, blocker, decisión, próximo paso y checkpoint; registró un progreso v86→v87 y un checkpoint v87→v88. No usó Git, SQLite directo, `--as-human` ni comandos lifecycle.
- Integraciones locales: el provider `gskills-context-provider/0.3.0` fue estrictamente read-only, con método mutable rechazado `-32601` y delta cero. El RecordSink requirió política explícita de cinco capacidades, no reclamó autoridad, produjo un único efecto durable y su replay fue `DUPLICATE_REPLAY` con delta cero. Esto valida fronteras de G-Ledger, no el producto G-Skills ni una contraparte externa.
- GSP fijado: implementación `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, contrato `gsp-exact-2026.04.12`/`004b14d7b64837025671970eac2469f092c49475`, 28 contratos y 31 schemas. Export/compile/validate/link/replay produjeron deltas `0/0/0/1/0`; no hubo elevación de autoridad, cambio lifecycle ni certificación. Project Radar quedó `EXCLUDED_BY_SCOPE`, sin ejecución ni PASS.
- Cierre gobernado: los usos `--as-human` se limitaron a resolver el blocker de Stage 7, fijar la acción siguiente y abrir el blocker de autorización humana de Stage 8; no verificaron gates, no cerraron fase y no emitieron decisión operativa. El checkpoint final `checkpoint:c85fb171-02ec-76d4-9990-366c3ac6f21d` quedó `CURRENT`.
- Conciliación: estado v97, digest `sha256:044fa21419743f9b81305b734192304c47733467a30cc33b516176f7219eb350`, head `sha256:71b74a1d305dc7aa725c5662ca1b38b8a4d38aaf3fd0c501218860576e500769`, proyecto `BLOCKED` por `blocker:opa-stage8-human-decision`, fase activa y cero fases completadas. `doctor`, `verify` y `audit` quedaron `ok`, sin findings ni submissions sin resolver.
- Verificación fresca: la regresión Pester etiquetada Stage 7 terminó 1/1 y `cargo xtask release check` confirmó 90 AC, 32 PL, 0 secretos, 9 métricas y 0 waivers explícitos.
- Resultado: OPA-S28…OPA-S30 `PASS`; OPA-S31 `EXCLUDED_BY_SCOPE`; avance **7/8, 87,5 %, `IN_PROGRESS`**. Evidencia sanitizada: `docs/pilot/results/stage-7-risk-close-hosted-requalification.json`.

**Siguiente paso:** obtener autorización explícita para Stage 8/8, limitada a reconciliar y presentar el informe final. Cualquier nueva declaración `OPERATIONALLY_ACCEPTED` exige después una decisión humana separada; merge y release permanecen fuera de alcance.

## 2026-09-04 — Recalificación Stage 8 conciliada para decisión, 7/8

- Autoridad: se ejecutó Stage 8 únicamente para conciliar y presentar resultados sobre `project:01a06adc-c603-7291-aabe-240d71809978` y el ZIP alojado exacto `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162`. No se autorizó registrar una decisión, cerrar la fase, hacer merge ni release.
- Incidencia del harness: el primer intento se detuvo antes de leer el Ledger porque PowerShell strict mode accedía directamente a la propiedad opcional ausente `owner_decision`. La regresión se reprodujo RED (1 fallida), se corrigió en el commit separado `0758f7c` y terminó GREEN (1/1). No fue un defecto del producto, no mutó el Ledger y no alteró contratos ni el binario bajo prueba.
- Conciliación viva: matriz, reporte, bitácora, siete manifiestos de etapa, `status`, `resume`, gates, checkpoint, `doctor`, `verify` y `audit` fueron reconciliados. Los siete manifiestos coinciden por SHA-256; Stage 1…7 y OPA-S01…OPA-S30 pasan; OPA-S31 permanece `EXCLUDED_BY_SCOPE`; cuatro defectos están `FIXED_AND_REQUALIFIED` y cero permanecen abiertos.
- Resultado técnico: cero conflictos, unknowns críticos, riesgos abiertos, waivers críticos, findings o submissions sin resolver. `gate:opa-04` está `VERIFIED`; otros siete gates requeridos siguen `PENDING`, ninguno `WAIVED`; sólo el gate opcional de ejercicio conserva waiver.
- Inmutabilidad y continuidad: la ejecución fue read-only, v97→v97, digest `sha256:044fa21419743f9b81305b734192304c47733467a30cc33b516176f7219eb350`, head `sha256:71b74a1d305dc7aa725c5662ca1b38b8a4d38aaf3fd0c501218860576e500769` y checkpoint `checkpoint:c85fb171-02ec-76d4-9990-366c3ac6f21d` `CURRENT`. El proyecto continúa `BLOCKED`, la fase activa y cero fases completadas.
- Frontera humana: OPA-S32 queda `AWAITING_OWNER_DECISION`; el máximo resultado es **`READY_FOR_OWNER_DECISION`, 7/8, 87,5 %**. La decisión histórica no se transfirió, Stage 8 no está completado y no se declaró `OPERATIONALLY_ACCEPTED`.
- Evidencia sanitizada: `docs/pilot/results/stage-8-risk-close-owner-decision-readiness.json`; manifiesto vivo SHA-256 `sha256:2931eaa54df4847a9983b56b73981bc83e296f49473d1abfd3e1163b79f9ac54`.

**Siguiente paso:** presentar el informe final al propietario y esperar una decisión humana separada. Sólo esa nueva autoridad podrá habilitar la transacción final de gates/lifecycle; merge y release permanecen fuera de alcance.

### Corrección del entregable para lectura humana

El JSON `operational-pilot-report.json` conserva su función normativa machine-readable, pero no sustituye el informe para el propietario. Se añadió `docs/pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md` como lectura principal, con dictamen ejecutivo, alcance, resultados por etapa, defectos, integridad, gates, límites y espacio de decisión sin prellenar. Esta corrección es exclusivamente documental y no muta el Ledger ni cambia `READY_FOR_OWNER_DECISION`.
