# G-Ledger Operational Acceptance Pilot Implementation Plan

> **For agentic workers:** Execute this plan inline, stage by stage. Use `test-driven-development` for harness or product behavior, `systematic-debugging` for unexpected results, and `verification-before-completion` before every commit or closure. Do not delegate the governed pilot state.

**Goal:** Construir y ejecutar un metaproyecto descartable que determine con evidencia operacional si G-Ledger 0.2.0 puede adoptarse para el primer proyecto real.

**Architecture:** La especificación y la matriz viven en documentación versionada; el ZIP publicado es el sujeto inmutable. Un runner PowerShell y un cliente de ensayo no distribuido conducen exclusivamente la CLI/stdio, mientras DB, exports y salida cruda permanecen en `.tooling/`. El reporte sanitizado conserva resultados, no secretos ni autoridad inventada.

**Tech Stack:** Rust 1.97.1 / edition 2024, PowerShell 7, SQLite/WAL a través de la CLI, JSON canónico, JSON-RPC stdio y Python/GSP fijado sólo en la etapa 7.

## Global Constraints

- Autoridad de producto: `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`.
- Autoridad del piloto: `docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md`.
- Sujeto invalidado inicial: ZIP SHA-256 `a83624f2e795c8b8d3d9e15a316be38cce8999e437731b73f5f8c8964963ad80`, fuente `b171f51cb58c60ed8c8a99b60386117f9fed6822`. Sujeto vigente recalificado: ZIP alojado `cf504e128386e782cc394c9c30bef699f94dd4be97ccd7e252402029bbfdcad0`, fuente exacta `bd564cdba07b616337681c15801d22478824108b`.
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

- [ ] Registrar objetos materiales definidos por S08 con claves únicas y versiones frescas.
- [ ] Resolver/mitigar/supersede y comprobar preservación mediante vistas.
- [ ] Crear checkpoint, cerrar sesión y verificar resume JSON/Codex; validar S08…S10.
- [ ] Actualizar 3/8 y commit `test: validate high-level continuity`.

### Task 4: Evidencia, gates y RecordSink

**Files:** cliente de ensayo no distribuido, fixtures exactos de policy/comando y tests de proceso real.

- [ ] RED: el cliente aún ausente debe impedir demostrar validate/submit/status/reconcile sobre el ZIP.
- [ ] GREEN: implementar framing Content-Length, digests canónicos y fixture dinámico sin acceso SQLite.
- [ ] Probar OBSERVED incompleto, VERIFIED_SOURCE por sink, gate/phase, stale/revalidation y waiver opcional; validar S11…S15.
- [ ] Actualizar 4/8 y commit `test: validate evidence and lifecycle boundaries`.

### Task 5: Idempotencia y reconciliación

- [ ] Ejecutar replay, key conflict, version conflict y policy denial con delta cero.
- [ ] Descartar stdout de checkpoint y reconciliar el scope original; consultar key inexistente.
- [ ] Validar S16…S21 sin retry ciego.
- [ ] Actualizar 5/8 y commit `test: validate transactional safety`.

### Task 6: Proyecciones, privacidad y recuperación

- [ ] Renderizar, alterar copia y demostrar divergencia/regeneración.
- [ ] Ejecutar portable roundtrip y backup/restore; rechazar corrupción y overwrite.
- [ ] Escanear superficies soportadas contra canaries; ejecutar doctor/verify/audit.
- [ ] Validar S22…S27, actualizar 6/8 y commit `test: validate recovery privacy and projections`.

### Task 7: Handoff ciego e integraciones

- [ ] Crear una tarea Codex nueva sólo con binario, database, project_ref y skill.
- [ ] Verificar su resume, avance material y checkpoint sin contexto narrativo.
- [ ] Ejecutar provider/RecordSink local y GSP fijado con deltas contractuales; registrar Radar excluido.
- [ ] Validar S28…S31, actualizar 7/8 y commit `test: validate blind handoff and bounded integrations`.

### Task 8: Decisión de adopción

- [ ] Ejecutar auditoría final, verificar freshness y conciliar todas las filas.
- [ ] Ejecutar el gate completo del repositorio si hubo cambios de código; ante corrección, recalificar artefacto y repetir etapas afectadas.
- [ ] Presentar resultados al propietario sin prellenar aprobación.
- [ ] Tras decisión humana explícita, registrar S32 y el estado correcto; actualizar 8/8 sólo si satisface la fórmula.
- [ ] Commit final `docs: record operational pilot decision`.

## Failure protocol

Ante un resultado inesperado: detener etapa, preservar evidencia sanitizada, abrir blocker, aplicar `systematic-debugging`, reproducir RED y corregir sólo dentro de contratos v0.2. Un nuevo candidato invalida evidencia posterior del anterior. No hacer push ni release sin solicitud explícita.
