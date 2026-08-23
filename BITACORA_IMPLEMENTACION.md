# Bitácora de implementación de Project Ledger

> Este fichero registra la vida de la implementación. No es el estado canónico del producto ni sustituye la futura proyección generada `BITACORA_PROYECTO.md`.

## Estado actual

| Campo | Valor |
|---|---|
| Fecha de actualización | 2026-08-23 |
| Estado | FASE 5 COMPLETADA — S8 COMPLETADO |
| Macrofases completadas (§50) | 6/12 |
| Slices completados (§51) | 9/15 |
| Última macrofase completada | Fase 5 — Generic integration protocol |
| Macrofase actual | Fase 6 — G-Skills provider (pendiente) |
| Último slice completado | S8 — generic integration protocol + SDK |
| Slice actual | S9 — G-Skills provider (pendiente) |
| Último commit de baseline | `6584c04227356cb156ff2614f8564f2ce2e1a68a` |
| Commit de cierre más reciente | Este commit: `feat: add generic integration protocol and conformance` |

## Autoridad y preservación

- Handoff operativo leído completamente: `referencias/PROJECT_LEDGER_v0.2.0_CODEX/`.
- Spec v0.2.0 verificada: SHA-256 `31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9`.
- Antecedente v0.1.0 localizado en el archivo externo preservado de G-Ledger Docs.
- Antecedente v0.1.0 verificado antes de copiar: SHA-256 `772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865`.
- Los locks G-Skills, GSP y Radar permanecen `PENDING`/`LOCK_GATED`; S0+S1 no declara compatibilidad runtime.

## Historial

### 2026-08-22 — Inicio profesional de implementación

**Acuerdos aplicados**

- Git inicializado en rama `main`.
- Commit inicial creado antes del código: `6584c04 chore: establish Project Ledger specification baseline`.
- Como no existía identidad Git global, se configuró solo para este repositorio `Project Ledger Codex <codex@project-ledger.local>`; no se atribuyeron commits a la identidad personal del usuario.
- Se adoptaron dos indicadores de avance derivados de la spec: 12 macrofases (§50) y 15 slices (§51). Los denominadores no provienen del ejemplo conversacional.
- Cada macrofase se cerrará con bitácora, evidencia y commit propio.

**Plan**

- Plan ejecutable: `docs/superpowers/plans/2026-08-22-project-ledger-phase-0-s0-s1.md`.
- Alcance autorizado: S0 + S1 únicamente.
- Exclusiones: SQLite/EventStore, ejecución de comandos, projection/gate engines, renderer/import, stdio, adapters reales, Agent Skill, MCP, UI, daemon, network y AI.

**Próximo paso**

Materializar el workspace y los guards de preservación; después iniciar TDD de JSON canónico, IDs, timestamps y digests.

### 2026-08-22 — Corrección de resolución del toolchain

- Síntoma: las primeras ejecuciones de `cargo metadata` agotaron el tiempo mientras `rustup` intentaba instalar un segundo toolchain exacto y dos procesos solapados colisionaron al renombrar componentes.
- Causa verificada: `stable` ya contenía Rust 1.97.1, pero `channel = "1.97.1"` creó una instalación nominal distinta e incompleta.
- Corrección mínima: `rust-toolchain.toml` usa `channel = "stable"`; `Cargo.toml` conserva `rust-version = "1.97.1"` y CI conserva Rust 1.97.1 exacto.
- Alcance de limpieza: solo se detuvieron los procesos huérfanos iniciados por esta tarea; no se eliminó ni modificó manualmente ningún toolchain del usuario.
- Evidencia posterior: `rustc 1.97.1`, `cargo 1.97.1` y `cargo metadata --no-deps --format-version 1` terminaron con exit code 0.

### 2026-08-23 — Cierre de Fase 0 (S0 + S1)

**Implementación materializada**

- Workspace Rust 2024 con crates separados para JSON canónico, dominio, schemas, CLI y `xtask`.
- Preservación byte-for-byte de v0.1.0 y v0.2.0, con verificación read-only desde la CLI.
- JSON canónico RFC 8785, SHA-256 tipado, IDs UUIDv7 con prefijo, timestamps RFC 3339 y referencias/identificadores validados.
- Contratos durables S1 sin payloads abiertos; uniones discriminadas cerradas de 53 comandos y 67 eventos.
- Registro offline de 46 schemas, 46 fixtures positivos, 46 negativos, 321 clasificaciones de arrays y matriz exhaustiva comando-evento.
- Generador determinista de 142 artefactos y comprobación de drift byte a byte.
- CLI limitada a `specs verify` y `contracts verify`; no ejecuta comandos de dominio ni crea estado.
- CI configurada para Windows, Linux y macOS con formato, Clippy, pruebas, schemas y checksums; job separado para `cargo-deny`.
- Matriz de cobertura persistente en `docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`.

**Evidencia del gate local**

| Verificación | Resultado |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 38 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

`cargo-deny` mantiene en nivel warning tres duplicidades transitivas (`getrandom`, `r-efi`, `syn`); provienen de ramas de dependencias publicadas y no son vulnerabilidades ni violaciones de licencia/origen. La política rechaza wildcards, advisories, licencias no autorizadas y fuentes desconocidas.

**Ambigüedades resueltas sin ampliar alcance**

- `REFRESH_AUTHORITY_DOCUMENT_DIGEST` modela resultados alternativos explícitos para match, indisponibilidad y mismatch.
- `REQUEST_PHASE_COMPLETION` modela alternativas aceptadas `READY_TO_CLOSE` o `PHASE_COMPLETED`, sin exigir eventos mutuamente excluyentes en una misma transacción.
- El perfil Radar conserva versión contractual 0.1.1 aunque el registro general sea v0.2.0.
- `project-ledger init` no se implementó: el primer prompt normativo prohíbe ejecución de comandos en S0+S1; queda para S2 junto con persistencia.

**Límites y estado operacional**

- No existe remote Git configurado; por ello el workflow remoto no pudo ejecutarse desde este workspace. La evidencia anterior es local en Windows. La CI multiplataforma está configurada, pero su primera ejecución en GitHub sigue siendo una confirmación operacional pendiente y no se presenta como evidencia ya obtenida.
- G-Skills permanece `PENDING_REFRESH`, GSP `PENDING` y Project Radar `PENDING`; ninguna compatibilidad bilateral se declara certificada.
- No se implementaron SQLite/EventStore, mutaciones, proyecciones, gates, renderer/import, stdio, adapters, Agent Skill, MCP, UI, daemon, red ni AI.

**Pendientes explícitos de S2 (sin implementar)**

1. SQLite/WAL y migraciones iniciales del event store.
2. Append-only con hash chain obligatoria, versiones contiguas y rebuild desde cero.
3. Command bus para las uniones ya cerradas, transacciones y `CommandResult` ejecutable.
4. Idempotency key/payload digest, expected version y rechazo de conflictos sin overwrite.
5. Persistencia y reconciliación de `ExternalSubmissionRecord`, incluida la capa transport `UNKNOWN`.
6. Pruebas de append/replay/rebuild/version/idempotencia/crash recovery exigidas por el gate de Fase 1.

**Siguiente paso según la especificación**

Iniciar Fase 1 / S2 con un plan TDD limitado a event store, comandos y submission reconciliation, reutilizando sin alterar los contratos S1 congelados y sin adelantar el projection engine de S3.

### 2026-08-23 — Inicio de Fase 1 (S2)

**Autoridad revisada**

- Se releyeron los invariantes I-01, I-04, I-06, I-07, I-26 e I-27; las secciones §15, §16, §39 y §40; los escenarios PL-06, PL-07, PL-08 y PL-20; y AC-006, AC-008 y AC-036…041.
- Gate de fase adoptado literalmente: append/rebuild/replay/version/unknown-layer PASS.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-1-s2.md`.

**Límites acordados para esta fase**

- Se crean puertos storage-independent, una implementación SQLite/WAL y el orquestador transaccional de comandos.
- `CREATE_PROJECT` será el handler real mínimo para probar un recorrido completo y habilitar `project-ledger init` sin red.
- Las uniones de los 53 comandos siguen cerradas y el processor podrá consumir handlers puros, pero las reglas de lifecycle/proyección/gates de los comandos restantes no se inventan en S2: pertenecen a S3–S5.
- La incertidumbre de transporte vive exclusivamente en `ExternalSubmissionRecord`; nunca se añade `UNKNOWN` a `CommandResult`.
- No se implementan projection engine, lifecycle derivado, gates, renderer, adapters, stdio, daemon, red ni AI.

**Siguiente acción**

Crear mediante TDD el puerto del event store y demostrar primero que una base SQLite real abre en WAL, migra de forma determinista y rechaza `UPDATE`/`DELETE` de eventos.

### 2026-08-23 — Cierre de Fase 1 (S2)

**Implementación materializada**

- Nuevos crates `event-store`, `application` y `storage-sqlite`, con dominio libre de dependencias de persistencia y application libre de SQLite.
- SQLite nativo bundled con WAL, `foreign_keys=ON`, `synchronous=FULL`, busy timeout de cinco segundos y migración v1 transaccional bajo `BEGIN IMMEDIATE`.
- Tablas para streams, eventos, comandos, resultados, scopes de idempotencia y submissions; triggers defensivos impiden `UPDATE`/`DELETE` de eventos y del historial de transiciones.
- Append atómico gap-free: génesis versión 1, `previous_event_digest=null` sólo en génesis, SHA-256 canónico de payload/envelope, head verificado y lectura fail-closed.
- `CommandProcessor` storage-independent con handler puro, matriz comando-evento cerrada, expected version, idempotencia por proyecto/tipo/actor/trust-level/key y resultados servidor persistentes.
- Handler real mínimo `CREATE_PROJECT`; las reglas ejecutables de los otros 52 comandos no se inventaron antes de S3–S5.
- `ExternalSubmissionRecord` con latest snapshot, historial append-only, compare-and-set, `OUTCOME_UNKNOWN` local y reconciliación accepted/rejected/not-found sin reintento ciego.
- `project-ledger init` offline escribe una base real en user-data o `--data-root`, ejecuta el camino común y valida recovery antes de informar éxito.
- `RecoveryReport` read-only cubre `quick_check`, WAL, migración, cadena, versiones/conteos y submissions aún inciertas.

**Evidencia del gate local**

| Verificación | Resultado |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 61 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

El primer pase de política rechazó `directories 6.0.0` porque su dependencia transitiva `option-ext` usa MPL-2.0, licencia fuera de la allowlist bloqueada. Se retiró la dependencia y se resolvió user-data mediante convenciones estándar de Windows/macOS/XDG; el segundo pase aprobó sin debilitar `deny.toml`. Permanecen warnings no bloqueantes por duplicidades transitivas publicadas de `getrandom`, `r-efi` y `syn`.

**Pruebas materiales de S2**

- Integridad: mutar payload o metadata invalida el digest correspondiente; génesis se recomputa a mano con JSON canónico.
- Event store: roundtrip de versiones 1 y 2, rechazo de missing genesis/gap/prior incorrecto/versión duplicada, lecturas no mutantes y corrupción controlada detectada.
- Transacción: un efecto para replay idéntico, conflicto para digest distinto, stale version sin append y par command/event prohibido rechazado antes de storage.
- Unknown layer: timeout se registra sin fabricar resultado, reconciliación contra accepted/rejected durable, not-found conserva unknown y replay exacto no duplica historial.
- Recovery/init: rollback WAL conserva la última verdad, reopen valida digests de migración, metadata indexada y resultados; `init` crea un proyecto sin red con un evento, un comando y un resultado.

**Límites y estado operacional**

- La evidencia es local en Windows. No existe remote Git configurado, por lo que la CI Windows/Linux/macOS está configurada pero no se declara ejecutada ni verde remotamente.
- S2 demuestra ingeniería del event store; no certifica rentabilidad, operación productiva, integración bilateral ni validez científica.
- G-Skills, GSP y Project Radar mantienen sus estados `PENDING`/`LOCK_GATED`; no se abrió red ni se ejecutaron adapters.
- No se implementaron projection engine, lifecycle derivado, gates/evidence engine, checkpoints semánticos, renderer, import/export, stdio, UI, daemon, network ni AI.

**Siguiente paso según la especificación**

Iniciar Fase 2 por S3: construir mediante TDD un projection engine determinista y completamente reconstruible desde el stream canónico, sin introducir todavía las reglas de lifecycle de S4.

### 2026-08-23 — Inicio de Fase 2 (S3)

**Autoridad revisada**

- Se releyeron I-02, I-05, I-20, I-23 e I-27; §13, §15.5, §25.4, §39, §40.5 y §45; PL-09; AC-007, AC-009 y AC-010; y el orden normativo S3/S4 de §51.
- La proyección y el Markdown son derivados reconstruibles. El stream de eventos continúa siendo la única verdad canónica.
- El gate adoptado para S3 exige determinismo, equivalencia incremental/rebuild, recuperación desde cero, descarte de caché corrupta y consultas sin mutación de la versión ni del conteo de eventos.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-2-s3.md`.

**Límites acordados para este slice**

- S3 implementa la mecánica de proyección con `PROJECT_CREATED` y `PROJECT_METADATA_UPDATED`, los únicos reducers previos a lifecycle con semántica suficiente para una prueba diferencial real.
- Todo evento sin reducer S3 se rechaza explícitamente; no se ignora ni se interpreta de forma parcial.
- La caché SQLite será derivada, mutable y descartable. Una discrepancia de versión, head, projector o digest obliga a reconstruir desde el stream íntegro.
- Pausa/reanudación/archivo, autoridad, roadmap, fases, estado derivado, decisiones, blockers, risks, next actions y catálogo permanecen reservados para S4.
- No se implementan gates/evidence engine, renderer, import/export, stdio, adapters, UI, daemon, red ni AI.

**Siguiente acción**

Escribir primero las pruebas RED de génesis y digest de `ProjectState`; después implementar el crate `projection-engine` mínimo hasta obtener GREEN.

### 2026-08-23 — Cierre de S3 (Fase 2 continúa)

**Implementación materializada**

- Nuevo crate `projection-engine`, storage-independent y sin reloj, aleatoriedad, filesystem ni orden de mapas como entradas del fold.
- `ProjectStateProjector::rebuild` parte obligatoriamente de `PROJECT_CREATED`; valida integridad, proyecto/stream, versiones y previous digest antes de derivar estado.
- `ProjectStateProjector::apply` demuestra equivalencia incremental/rebuild con `PROJECT_METADATA_UPDATED`; estado o eventos corruptos, gaps, cross-project, génesis duplicada y reducers posteriores fallan cerrados.
- `project_state_id` se deriva determinísticamente como `state:<uuid-del-proyecto>`; `state_digest` cubre JSON canónico excluyendo sólo su propio campo.
- Puerto `ProjectionSnapshotStore` separado de la verdad canónica, con resultados `Missing`, `Snapshot` y `Corrupt`.
- Migración SQLite v2 secuencial y verificada sin modificar los bytes de v1; `project_state_snapshots` conserva versión/head/projector/digest y JSON canónico.
- `ProjectStateQuery` verifica primero la cadena, acepta sólo una caché completamente coincidente y reconstruye/repara ante ausencia, obsolescencia o corrupción.

**Evidencia del gate local**

| Verificación | Resultado |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 74 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

`cargo-deny` conserva únicamente los tres warnings transitivos no bloqueantes ya registrados (`getrandom`, `r-efi`, `syn`); S3 no añadió una dependencia externa nueva.

**Pruebas materiales de S3**

- Determinismo: el mismo stream produce exactamente el mismo `ProjectState` y digest recomputado a mano.
- Diferencial: aplicar metadata sobre el estado de génesis equivale byte por byte a reconstruir ambos eventos desde cero.
- Fail-closed: se rechazan stream vacío, génesis incorrecta, versión contractual Radar 0.1.1, identidad derivada inválida, estado con digest roto, gap, prior digest incorrecto, proyecto ajeno, segunda génesis y evento lifecycle aún no soportado.
- Caché: roundtrip/overwrite conserva un evento y versión 1; JSON malformado o metadata redundante divergente se clasifica como `Corrupt`.
- Recovery: caché missing/stale/corrupta se reconstruye desde el head actual y la siguiente lectura usa el snapshot reparado.
- No mutación: 100 consultas consecutivas devuelven el mismo estado sin cambiar versión ni conteo de eventos.

**Límites y estado operacional**

- S3 cierra un slice, no la macrofase 2. Lifecycle/authority/roadmap/status/catalog permanecen sin implementar hasta S4.
- Una caché es sólo una optimización derivada; no constituye evidencia ni verdad canónica y puede eliminarse sin pérdida.
- La evidencia es local en Windows. No existe remote Git configurado; no se declara CI remota ejecutada ni verde.
- No se certifica operación productiva, integración bilateral, rentabilidad ni validez científica; no se abrió red ni se ejecutaron adapters.
- Gates/evidence engine, checkpoints semánticos, renderer/import/export, stdio, UI, daemon, network y AI siguen pendientes.

**Siguiente paso según la especificación**

Continuar Fase 2 con S4: implementar mediante TDD lifecycle, autoridad, roadmap secuencial, fases/kinds, decisiones, blockers, risks, next actions, estado de proyecto derivado y catálogo, extendiendo el projector sin debilitar el gate S3.

### 2026-08-23 — Inicio de S4 (cierre previsto de Fase 2)

**Autoridad y alcance revisados**

- Se releyeron I-09, I-13, I-14, I-20 e I-23; §9.2, §10, §13–17, §19, §23–24 y §39.6; AC-022, AC-023, AC-026 y AC-028; Fase 2 y el orden S4/S5.
- Gate literal de Fase 2: rebuild equivalence and deterministic lifecycle PASS.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-2-s4.md`.
- S4 incluye authority, roadmap secuencial, transiciones por `phase_ref`, decisiones, blockers, risks, actions, status derivado y catálogo; gates/evidence/Phase Engine de cierre permanecen S5.

**Resoluciones fail-closed de contratos incompletos**

- Ningún command/event transporta un objeto `Phase` completo. S4 no inventará un `DEFINE_PHASE`: validará existencia/orden por `roadmap.phase_order`; `PhaseKind` conserva su contrato cerrado y no influye en lifecycle conforme a §17.4.
- `AUTHORITY_DOCUMENT_SUPERSEDED` y `DECISION_SUPERSEDED` sólo persisten refs aunque sus commands contienen reemplazos completos. Para evitar pérdida de verdad, el reemplazo deberá estar registrado/propuesto previamente y coincidir exactamente con el payload.
- El catálogo será una proyección pura de ledgers conocidos explícitamente; no escaneará rutas implícitas ni adquirirá autoridad de continuidad.

**Siguiente acción**

Escribir las pruebas RED del cursor de proyección para autoridad, roadmap, fases y status; después implementar únicamente los reducers necesarios para volverlas GREEN.

### 2026-08-23 — Cierre de Fase 2 (S4)

**Implementación materializada**

- `ProjectStateProjection` conserva índices privados reconstruibles para estados de fase, decisiones, blockers, riesgos, próximas acciones y conflictos, mientras el contrato público `ProjectState` mantiene sólo refs canónicas.
- `ProjectStateProjector` v2 delega el rebuild al cursor completo, invalida snapshots de S3 por versión y rechaza streams cuyo `lifecycle_status` final no coincide con la derivación normativa.
- Autoridad ordenada por rank/ID, digest canónico inmutable ante mismatch, roadmap secuencial, una fase activa, reapertura explícita y bloqueo de inicio ante conflicto crítico de autoridad.
- Precedencia de estado fija: `ARCHIVED → PAUSED → BLOCKED → COMPLETED → ACTIVE → INITIALIZING`; no existe setter genérico de status.
- Decisiones superseded dejan de ser vigentes sin borrar historia; blockers y riesgos actualizan refs abiertos; un riesgo no bloquea por severidad implícita.
- Acciones ordenadas por `AUTHORIZED`, `ELIGIBLE`, `RECOMMENDED`, `PROPOSED`, con desempate por ID; `AUTHORIZED` exige autoridad actual y las terminales/bloqueadas quedan fuera de la secuencia ejecutable.
- Nuevo crate `lifecycle`: valida comandos S4 contra el stream ya verificado por `CommandProcessor`, produce lotes deterministas y deja que expected version, idempotencia, matriz cerrada y SQLite conserven la frontera atómica.
- Nuevo crate `project-catalog`: deriva entradas ordenadas desde `KnownProject` explícitos, valida cada estado y no escanea filesystem ni adquiere autoridad.

**Evidencia del gate local**

| Verificación | Resultado |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 90 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

`cargo-deny` conserva solamente los tres warnings transitivos no bloqueantes ya conocidos (`getrandom`, `r-efi`, `syn`). S4 añadió `sha2` al crate lifecycle reutilizando la versión ya fijada en el workspace; no introdujo una familia externa nueva.

**Pruebas materiales de S4**

- Lifecycle: autoridad/roadmap/fases producen el mismo estado incremental y desde cero; una segunda fase activa y un stream con status derivado obsoleto fallan cerrados.
- Continuidad: supersesión de decisiones, resolución de blocker, cierre de risk, terminación de acción y resolución de conflicto preservan historia en eventos y actualizan refs deterministas.
- Comandos: pausa humana y phase start emiten el evento requerido más `PROJECT_STATUS_DERIVED`; un actor agente no puede pausar y un conflicto crítico de autoridad impide iniciar fase.
- Transacción/E2E: create→authority→roadmap→phase→risk→blocker→resolve→decision supersession→actions atraviesa SQLite real; un comando inválido no anexa eventos.
- Recovery: la proyección desde cero coincide con la lectura, una caché corrupta se reemplaza y el conteo/versión canónicos permanecen sin cambio.
- Catálogo: el orden no depende de la entrada y estados inválidos o proyectos duplicados se rechazan.

**Defecto detectado y corregido durante el gate**

- El primer diseño derivaba el ID de evento sumando el ordinal al UUID del comando. El evento derivado de un comando terminado en `…df90` colisionó exactamente con el primario de un comando `…df91`, y SQLite lo evidenció con `UNIQUE constraint failed: events.event_id`.
- Se conservó una regresión mínima y el E2E de storage. La derivación ahora usa SHA-256 con separación de dominio sobre UUID de comando + ordinal, preserva bits UUIDv7 y evita relaciones aritméticas entre comandos adyacentes.

**Límites contractuales y operacionales**

- Ningún command/event S1 transporta una definición completa de `Phase`; S4 usa exclusivamente `roadmap.phase_order` y estados privados derivados. No inventa `DEFINE_PHASE` ni semántica de `PhaseKind`.
- Los eventos de supersesión sólo contienen refs. Para no perder datos, el replacement de authority/decision debe existir durablemente y coincidir exactamente con el payload del command.
- El cierre/waiver/evaluación de gates y la reapertura por staleness pertenecen a S5. Los eventos correspondientes siguen sin reducer y fallan cerrados.
- La evidencia es local en Windows. No existe remote Git; no se declara CI remota ejecutada ni verde, operación productiva, integración bilateral, rentabilidad o validez científica.
- G-Skills, GSP y Project Radar continúan `PENDING`/`LOCK_GATED`; no se abrió red ni se ejecutaron adapters.

**Siguiente paso según la especificación**

Iniciar Fase 3 / S5 mediante TDD: Evidence runner/import, `PredicateAST`, `GateDefinition`/`GateEvaluation`, waivers, Phase Engine y freshness/revalidation, con el gate obligatorio de que ningún gate sea `VERIFIED` sin evaluación válida y ninguna fase quede `COMPLETED` con gates requeridos faltantes.

### 2026-08-23 — Inicio de Fase 3 (S5)

**Autoridad y alcance revisados**

- Se releyeron REQ-GAT-001/002, I-08–12, I-15, I-18 e I-20; §14.4–14.5, §15.4/15.4.1, §17–18, §20–22 y §47–51; PL-02, PL-03 y PL-05; y AC-013–021.
- Gate literal de Fase 3: cero gates verificados sin evaluación válida y cero fases completas con gates requeridos insatisfechos.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-3-s5.md`.
- La ejecución continúa inline porque el usuario ya autorizó el siguiente paso; no se delega trabajo ni se amplía a S6.

**Resoluciones fail-closed de contratos incompletos**

- Ningún command/event transporta un objeto `Phase` completo. La pertenencia requerida se derivará de `GateDefinition.phase_ref` y `criticality=REQUIRED`; no se inventará `DEFINE_PHASE`.
- La spec permite auto-cierre sólo bajo `AUTO_ON_ALL_REQUIRED_GATES`, pero ese valor no existe en los contratos cerrados v0.2.0. S5 no auto-completará fases por evidencia, evaluación o waiver: exigirá `REQUEST_PHASE_COMPLETION` explícito.
- Como tampoco existe un hecho durable de approval policy, el cierre explícito se limitará a actor humano. Una fase sólo cerrará si sus gates requeridos están `VERIFIED/WAIVED`, su autoridad está current y no existen blockers/conflictos incompatibles.
- `DEFINE_GATE` no permite emitir `PHASE_REOPENED`; por ello un gate requerido nuevo sobre una fase ya completa/ready se rechazará en vez de invalidar silenciosamente el cierre.
- `RECORD_VERIFICATION_RESULT` registrará evidencia, nunca estado de gate/fase. El Gate Engine será la única frontera que interpreta `PredicateAst` y emite evaluación.
- S5 no emitirá `CHECKPOINT_INVALIDATED`: los checkpoints durables y su freshness pertenecen a S6.

**Siguiente acción**

Escribir las pruebas RED de los reducers de gate/evidencia y de las invariantes AC-013/014/016/017; después implementar sólo los índices privados y transiciones necesarios para volverlas GREEN.

### 2026-08-23 — Cierre de Fase 3 (S5)

**Implementación materializada**

- `ProjectStateProjection` v3 reconstruye índices privados de definiciones/status/evaluaciones/waivers de gate, evidencia, claims y repository bindings; el estado público conserva refs compactas y deterministas.
- Reducers fail-closed impiden `GATE_VERIFIED` sin evaluación PASS actual, definition version/digest coincidente, rule results coherentes y evidencia `VERIFIED_SOURCE`; una fase completa no puede terminar con gate requerido insatisfecho.
- Nuevo crate `gate-engine`: implementa todos los operadores cerrados de `PredicateAst`, reglas `ALL/ANY`, selección por kind/status, binding de snapshot, resultados PASS/FAIL/INCOMPLETE/STALE/CONFLICTED/ERROR y digests/IDs/provenance deterministas.
- Phase Engine exige `REQUEST_PHASE_COMPLETION` humano explícito; evidencia, evaluación y waiver no auto-completan porque el contrato v0.2.0 no materializa `AUTO_ON_ALL_REQUIRED_GATES`.
- Waivers validan actor submitter, reason, scope, review condition, authority actual y versión/digest exactos del gate; no fabrican evidencia.
- Nuevo crate `evidence`: `std::process::Command` recibe programa+argv exactos, allowlist y cwd explícito; no usa shell, no persiste raw output, redacta/acota display/summary y captura exit code, duración, métricas y payload digest.
- Import unsigned valida JSON cerrado, project/schema/digest, registra artifact/provenance y degrada siempre a `OBSERVED`; JSON alterado, campo desconocido o proyecto distinto falla cerrado.
- Claims se registran inicialmente sólo como `ASSERTED`; una transición `VERIFIED` requiere actor humano y evidencia `VERIFIED_SOURCE` existente.
- Freshness cubre los tres triggers contractuales: evidencia invalidada, cambio material de autoridad y cambio de snapshot/repository dependency. Los gates afectados pasan explícitamente por `STALE → REVALIDATION_REQUIRED`.
- `INVALIDATE_EVIDENCE` reabre una fase completa a `REVALIDATION_REQUIRED` con el evidence ref en el reason; tras nueva evidencia y revalidación, la fase sólo vuelve a completar por request explícito.
- `CoreCommandHandler` compone genesis, lifecycle S4 y gates/evidencia S5 sobre el mismo stream verificado; expected version, idempotencia, matriz cerrada y append SQLite siguen siendo una única frontera atómica.

**Evidencia del gate local**

| Verificación | Resultado |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 111 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --cached --check` | PASS |

**Pruebas materiales de S5**

- AC-013/014/021: tests verdes con build faltante producen `INCOMPLETE`; no verifican gate ni completan fase. Evaluación PASS forjada o sin rule PASS/evidencia aceptada se rechaza.
- AC-015: waiver conserva y valida actor/reason/scope/review/authority/definition version+digest; actor divergente falla.
- AC-016/017 y PL-05: invalidación, mismatch de autoridad y refresh de snapshot marcan dependencias; reapertura y reparación conservan timeline/provenance.
- AC-018/019: subprocess real `rustc --version` demuestra exit/duración/digest, allowlist y raw off; ejecutable no permitido se rechaza antes del spawn.
- AC-020: claim registrada por agente permanece `ASSERTED`; elevación sin evidencia se rechaza.
- Rebuild/E2E: projection incremental/rebuild y transacciones SQLite reales preservan estado, IDs deterministas, cadena y atomicidad.

**Hallazgos y decisiones durante la fase**

- Clippy detectó funciones exhaustivas superiores al límite de líneas. Se confirmó que la causa era la cobertura lineal completa de uniones cerradas, no duplicación o fallo funcional; se documentaron excepciones locales auditables y el gate quedó verde.
- La primera prueba de authority freshness no marcó el gate porque el fixture declaraba únicamente `EVIDENCE_INVALIDATED`. Se corrigió el fixture para declarar `AUTHORITY_DOCUMENT_CHANGED`; la lógica no fue relajada.
- La revisión cruzada final detectó que el motor aceptaba correctamente `completion_rule=ANY`, pero el proyector validaba todo PASS como `ALL`; también rechazaba el resultado `CONFLICTED` que el motor producía ante snapshots seleccionados distintos. Dos regresiones RED reprodujeron ambos defectos y la validación se corrigió para recalcular exactamente `ALL/ANY` y auditar cero, uno o múltiples bindings.
- `REVALIDATION_REQUIRED + gates reparados` atiende `REQUEST_PHASE_COMPLETION` directamente, conforme a §17.3, sin inventar un estado durable `REOPENED`.

**Límites contractuales y operacionales**

- Los contratos siguen sin transportar un objeto `Phase`; required gates se derivan de `GateDefinition.phase_ref + criticality=REQUIRED`.
- Sin `AUTO_ON_ALL_REQUIRED_GATES` durable, no existe auto-cierre. La opción más estricta exige request humano explícito.
- Los command matrices de authority/repository refresh no permiten `PHASE_REOPENED`. Si un cambio afectaría un gate requerido de fase ya `COMPLETED/READY_TO_CLOSE`, el comando falla cerrado; `INVALIDATE_EVIDENCE` sí dispone del outcome de reapertura.
- `EXTERNAL_EVIDENCE_LINKED` sólo contiene una referencia opaca sin kind/result; no participa en predicates hasta existir un `EvidenceRecord` cerrado/importado.
- Runner/importador y motores están embebidos en el core; la CLI operativa completa de gates/evidencia queda para la articulación de superficies posteriores, evitando selección implícita de proyecto/ledger.
- No existe remote Git; la evidencia es local Windows. No se declara CI remota, operación productiva, integración bilateral, rentabilidad ni validez científica.
- G-Skills, GSP y Project Radar continúan `PENDING`/`LOCK_GATED`; no se abrió red ni se ejecutaron adapters.

**Siguiente paso según la especificación**

Iniciar Fase 4 por S6: checkpoint N/N+1, freshness de checkpoint, `ResumeProjection`, renderer Markdown determinista y audit/verify, manteniendo legacy/portable import reservado para S7.

### 2026-08-23 — Inicio de Fase 4 (S6)

**Estado al inicio**

- HEAD limpio: `7da3f54 feat: complete deterministic gates and evidence`.
- Avance conservado: 4/12 macrofases y 6/15 slices; Fase 4 no se cerrará hasta completar también S7.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-4-s6.md`.
- La ejecución continúa inline por autorización expresa del usuario y se limita a S6.

**Autoridad y gate revisados**

- Se releyeron REQ-CHK-001, §25–28, §30, AC-024/025/027/029–033, Fase 4 y el orden S6/S7.
- Gate S6: Markdown determinista, checkpoint N/N+1 exacto, resume dentro de presupuesto y divergencia de artefacto detectable/regenerable.
- Legacy import/review y portable chain/fork permanecen reservados a S7; no se editan contratos congelados.

**Resoluciones fail-closed de contratos incompletos**

- El checkpoint captura el `ProjectState` previo N y referencia el ID real del `CHECKPOINT_CREATED` N+1. El evento se construye con ID determinista antes de calcular el digest del checkpoint.
- Freshness se deriva como registros inmutables de `CHECKPOINT_CREATED`/`CHECKPOINT_INVALIDATED`; el checkpoint nunca se modifica. Un checkpoint nuevo supersede al anterior en la proyección de freshness.
- El contrato admite una sola `repository_binding` en checkpoint. Cero snapshots vigentes produce `CURRENT_FOR_PROJECT_ONLY`, uno se copia exactamente y más de uno se rechaza sin selección implícita.
- No existe tokenizer/version contractual. S6 usará bytes UTF-8 como cota conservadora reproducible para los presupuestos 220/450 y lo declarará como limitación.
- `GeneratedArtifactRecord` no tiene command/event de persistencia. El renderer devuelve record+bytes y audit verifica el digest exacto; no se inventa un evento durable.
- S6 completará reducers/commands de work progress y unknowns porque resume y la sección normativa de conflictos/unknowns no pueden derivarlos de texto ni fabricarlos.
- Sólo filas cuya matriz congelada permite `CHECKPOINT_INVALIDATED` podrán anexarlo en el mismo lote atómico; un hecho material compatible puede avanzar la versión material sin invalidar.

**Siguiente acción**

Escribir las pruebas RED del fold N/N+1, materialidad y freshness; implementar exclusivamente los reducers privados necesarios hasta GREEN.

### 2026-08-23 — Cierre de S6 (Fase 4 continúa abierta)

**Implementación materializada**

- `ProjectStateProjection` v4 reconstruye progreso material, unknowns, checkpoints y freshness desde cero; `CHECKPOINT_CREATED`/`CHECKPOINT_INVALIDATED` avanzan `state_version` pero conservan `material_state_version`.
- El nuevo `ContinuityEngine` construye primero el ID determinista del evento y después el checkpoint, por lo que captura estado/digest N y referencia exactamente el evento N+1 sin circularidad.
- Los checkpoints son inmutables. Cada evaluación de freshness genera un registro derivado append-only; se cubren `CURRENT`, `CURRENT_FOR_PROJECT_ONLY`, `STALE_FOR_REPOSITORY`, `STALE_FOR_AUTHORITY`, `CONFLICTED`, `INVALIDATED` y `SUPERSEDED_BY_NEWER_CHECKPOINT`.
- Un snapshot de repositorio vigente se copia exactamente al checkpoint. Más de un binding `CURRENT` se rechaza sin elegir implícitamente; cero binding produce alcance project-only.
- `ResumeBuilder` prioriza freshness, siguiente acción y unknowns/conflictos críticos; produce texto compacto ≤220 y Markdown para agente ≤450. Ante la ausencia de tokenizer contractual, el conteo usa bytes UTF-8 como cota conservadora determinista.
- `MarkdownRenderer` genera LF y warning normativo, las trece secciones en orden fijo y perfiles `FULL_LOCAL`, `MINIMAL_LOCAL` y `SHAREABLE`. Los textos humanos se normalizan/acotan, se redactan patrones de credenciales `key=value` conocidos y nunca se incluye raw evidence.
- Cada render devuelve bytes y `GeneratedArtifactRecord`. `verify_stream_and_artifact` reconstruye el stream y compara versión/digest fuente, contrato del renderer y digest de bytes; una edición produce `GENERATED_ARTIFACT_MODIFIED` y regenerar recupera el contenido original.
- `audit_projection` expone checkpoint no current, unknown material abierto y conflicto crítico. Render/audit son read-only y no incorporan ediciones Markdown al estado canónico.
- `CoreCommandHandler` enruta progreso, unknown record/resolve y checkpoint al nuevo owner, y anexa como máximo una invalidación permitida por la matriz al lote del owner existente. SQLite persiste unknown+freshness en una sola transacción.

**Evidencia RED→GREEN y gate local**

| Verificación | Resultado |
|---|---|
| Pruebas RED focalizadas | APIs de checkpoint/resume/Markdown/audit ausentes; Core rechazó comandos de continuidad; modificación de artefacto y perfil inválido no verificaban antes de implementar |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 122 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

Durante el gate, Clippy hizo visible que una prueba agrupaba tres escenarios en 116 líneas; se separó sin cambiar producción. `cargo-deny` detectó cuatro dependencias internas `path` sin versión exacta en el crate nuevo; se fijaron a `=0.2.0` y el gate completo se reinició hasta PASS. Permanecen como warnings permitidos las tres duplicidades transitivas ya conocidas (`getrandom`, `r-efi`, `syn`); no hay advisory, wildcard, licencia ni source rechazado.

**Límites contractuales y operacionales**

- `GeneratedArtifactRecord` no se persiste: v0.2.0 no define command/event para hacerlo. El API devuelve record+bytes verificables y no inventa contrato durable.
- El renderer, resume y audit existen como core embebible; la escritura de `BITACORA_PROYECTO.md` y la superficie CLI completa de continuidad no se adelantaron.
- La auditoría S6 cubre integridad de stream/origen/artefacto y findings de continuidad; no se presenta como implementación completa de cada detector futuro de §30.2.
- Legacy review/import y portable bundle/chain/fork pertenecen a S7. Por ello Fase 4 sigue abierta: avance real 4/12 macrofases y 7/15 slices.
- La evidencia es local en Windows. No existe remote Git y no se declara CI remota, operación productiva ni compatibilidad bilateral ejecutada.

**Siguiente paso según la especificación**

Implementar S7: portable export/import con verificación de cadena, fast-forward/fork explícito y legacy Markdown review sin `TRUST_ALL`; después ejecutar el gate de roundtrip/divergencia para cerrar Fase 4.

### 2026-08-23 — Inicio de S7 (cierre previsto de Fase 4)

**Estado al inicio**

- HEAD limpio: `f598080 feat: add deterministic project continuity views`.
- Avance conservado: 4/12 macrofases y 7/15 slices; S7 es el segundo y último slice de Fase 4.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-4-s7.md`.
- La ejecución continúa inline por autorización expresa del usuario y se limita a portable bundle + legacy import.

**Autoridad y gate revisados**

- Se releyeron REQ-REL-002, §29, §31, AC-005/034/035, PL-15, Fase 4 y el orden S7→S8.
- Gate de cierre: roundtrip con cadena/estado equivalentes; checksums, schemas, identity y stream verificados; divergencia fail-closed; legacy review sin confianza masiva.
- La CLI completa, el protocolo genérico, stdio, SDK e integraciones de producto siguen fuera de S7.

**Resoluciones conservadoras antes de código**

- El bundle será el árbol de directorios normativo; el core transportará el artefacto Radar suministrado, pero no adelantará la lógica de mapping de S12.
- `checksums.sha256` cubrirá payloads, no `manifest.json` ni a sí mismo, para evitar circularidad. El manifest se auto-verifica por `digest` y enlaza el fichero de checksums por `checksums_digest`.
- El importador verifica completamente antes de exponer un sufijo a persistencia. SQLite revalida versión, identidad, prior digest y secuencia dentro de una transacción inmediata.
- Una cadena idéntica es idempotente, un prefijo exacto admite fast-forward y un destino más nuevo nunca se sobrescribe.
- La spec exige provenance tipada para materializar un fork, pero los contratos v0.2 congelados no incluyen esos campos ni un command/event de fork. No se reescribirán eventos ni authority: la divergencia quedará fail-closed y `FORK_EXPLICIT` producirá un handoff con nueva identidad y provenance de origen, sin fingir una mutación durable inexistente.
- Legacy preview reconocerá sólo declaraciones explícitamente etiquetadas. Toda recomendación inicial será `REVIEW`; el reviewed import exigirá una decisión individual por candidato y sólo propondrá hechos `ASSERTED`, nunca gates/fases verificados o completados.

**Siguiente acción**

Escribir las pruebas RED del árbol portable determinista, checksums y manifest; después implementar el mínimo exportador puro hasta GREEN.

**Hito S7.1 — Export portable GREEN**

- RED confirmado: `PortableBundle` y `PortableExportRequest` no existían.
- GREEN focalizado: 2 pruebas verifican árbol §31.1, JSON/JSONL canónico, bytes repetibles, manifest/digests/checksums, privacy fail-closed y no overwrite.
- Resolución de checksums documentada y ejecutable: control files fuera de su propio listado; el manifest enlaza el digest exacto del listado.
- La corrección de compilación se limitó a retirar un `const` inválido de un constructor de digest; el test focalizado volvió a pasar 2/2.
- Siguiente: lectura/verificación hostil, política identical/new/fast-forward/divergence y append SQLite transaccional.

**Hito S7.2 — Roundtrip, divergencia y SQLite GREEN**

- RED confirmado: no existían lectura de directorio, plan de importación ni puerto de append portable.
- GREEN portable: 5/5 pruebas cubren lectura segura, checksum/state tampering, `IDENTICAL_CHAIN`, `NEW_PROJECT`, `FAST_FORWARD`, destino más nuevo, `STREAM_DIVERGENCE` y provenance de `FORK_EXPLICIT`.
- GREEN SQLite: 2/2 pruebas reales demuestran import de proyecto nuevo, fast-forward por sufijo, estado reconstruido equivalente, identical sin efecto y rollback íntegro ante expected version inválida.
- El nuevo `EventImportStore` es estrecho: no importa tablas privadas de commands/results ausentes del bundle; preserva causation original y revalida evento, identidad, versión y prior digest dentro de `BEGIN IMMEDIATE`.
- `FORK_EXPLICIT` sigue siendo deliberadamente no mutante: devuelve target nuevo + project/event/digest de origen. La rama divergente ordinaria nunca produce merge ni last-write-wins.
- Siguiente: legacy Markdown preview/review con provenance por línea, contradicciones explícitas y una decisión obligatoria por candidato.

**Hito S7.3 — Legacy review GREEN**

- RED confirmado: no existían workflow, request ni errores de legacy import.
- GREEN focalizado: 3/3 pruebas cubren extracción determinista, digest del source exacto, sección/rango de línea, conflictos bidireccionales, recomendaciones sólo `REVIEW` y rechazo contractual de `TRUST_ALL`.
- `finalize` exige una decisión terminal, individual y razonada por cada candidato; missing, duplicate, foreign, empty reason o `REVIEW` residual fallan sin propuesta parcial.
- `IMPORT_AS_ASSERTED` sólo produce una propuesta no durable con clasificación `ASSERTED`. No emite command/event, no verifica gate, no completa fase y no eleva authority.
- Siguiente: revisar código y placeholders, actualizar documentación/matriz, ejecutar el gate completo y cerrar S7/Fase 4 sólo si todo queda verde.

### 2026-08-23 — Cierre de S7 y Fase 4

**Implementación materializada**

- Nuevo crate `portable-bundle` con el árbol fijo de §31.1, JSON/JSONL canónico, manifest auto-verificable, checksums estables, schema/upstream locks transportados, perfiles de entrada fail-closed y escritura sin overwrite.
- El listado de checksums excluye `manifest.json` y `checksums.sha256` para eliminar circularidad; el manifest enlaza el digest exacto del listado y se auto-verifica por su propio digest canónico.
- La lectura sólo acepta las rutas normativas y locks JSON de un componente; rechaza extras, symlinks, JSONL sin LF, bytes alterados, schema lock incompatible, chain/state/identity/version divergence y checksums incorrectos.
- La política distingue `IDENTICAL_CHAIN`, `NEW_PROJECT`, `FAST_FORWARD`, destino más nuevo y `STREAM_DIVERGENCE`. Same ID/same digest se compara por evento; no existe merge por timestamp ni event ID aislado.
- `EventImportStore` revalida expected version, genesis, project/stream identity, versiones contiguas, previous digest e integridad de cada envelope dentro de una transacción SQLite inmediata. Un fallo revierte el sufijo completo.
- El bundle normativo contiene events pero no tablas privadas de commands/results; el import conserva causation refs/digests originales sin fabricar commands históricos.
- `FORK_EXPLICIT` devuelve nueva identidad de target y provenance exacta `forked_from_project_ref/event_ref/digest`; no reescribe eventos ni authority y no simula una mutación durable ausente de los contratos v0.2.0 congelados.
- Nuevo crate `legacy-import` con parser conservador de declaraciones etiquetadas, provenance de source digest/sección/rango de línea, IDs deterministas y contradicciones bidireccionales preservadas.
- Preview recomienda exclusivamente `REVIEW`. Reviewed import exige una decisión terminal y razonada por candidato; `IMPORT_AS_ASSERTED` produce sólo propuestas no durables `ASSERTED`. `ARCHIVE_ONLY` registra una decisión explícita por candidato y `TRUST_ALL` no existe.
- Un schema lock v0.1.1 produce `MigrationRequired`; no se reinterpreta silenciosamente como v0.2.0.

**Evidencia RED→GREEN y gate local**

| Verificación | Resultado |
|---|---|
| Pruebas RED focalizadas | APIs de export/read/plan/apply y legacy review ausentes; cada ciclo falló primero por la frontera que debía implementarse |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 134 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — 142 artefactos sincronizados |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

La revisión intermedia detectó tres detalles mecánicos: un constructor `const` inválido, una asignación clonada y dos patrones señalados por Clippy (`expect` evitable y range-loop). Cada causa se aisló, se corrigió mínimamente y se volvió a probar. El primer `cargo-deny check` pasó online; al repetir el gate, GitHub no respondió durante el refresh de RustSec, por lo que se reejecutó `cargo-deny --offline check` contra la base local ya validada y pasó. Permanecen como warnings permitidos las duplicidades transitivas conocidas de `getrandom`, `r-efi` y `syn`.

**Gate de Fase 4 y límites operacionales**

- El gate de Fase 4 queda verde: Markdown determinista y checkpoint N/N+1 ya estaban probados en S6; S7 añade roundtrip con cadena/estado equivalentes y divergencia fail-closed.
- Avance real: 5/12 macrofases y 8/15 slices. Los denominadores proceden de §50/§51, no del ejemplo conversacional.
- La superficie core portable/legacy está implementada; los subcomandos CLI completos se conectarán con las superficies posteriores sin seleccionar implícitamente un ledger o proyecto.
- El handoff de fork es explícito pero no durable por ausencia de un command/event/receipt cerrado para materializarlo. Se satisface AC-035 por la rama obligatoria fail-closed y no se sobredeclara soporte de reidentificación.
- La evidencia es local en Windows. No existe remote Git y no se declara CI remota, operación productiva, integración bilateral, rentabilidad ni validez científica.

**Siguiente paso según la especificación**

Iniciar Fase 5 / S8: generic integration protocol + SDK, con manifest/health/projection envelope, stdio/artifact exchange, identity, upstream locks y conformance; el gate deberá demostrar read side-effects cero y fixtures maliciosos fail-closed.

### 2026-08-23 — Inicio de S8 (Fase 5)

**Estado al inicio**

- HEAD limpio: `1975933 feat: add verified portable and legacy imports`.
- Avance conservado hasta superar el gate: 5/12 macrofases y 8/15 slices.
- Plan ejecutable persistido en `docs/superpowers/plans/2026-08-23-project-ledger-phase-5-s8.md`.
- La ejecución continúa inline por autorización expresa del usuario y sin delegación.

**Autoridad y fronteras revalidadas**

- Se releyeron `00_START_HERE.md`, `INTEGRATION_LOCKS.json`, REQ-INT-001/002/003, §32-33, §38.5, Fase 5, S8 y la matriz AC/PL asociada.
- El gate de fase exige efectos laterales de lectura observables iguales a cero y fixtures maliciosos rechazados.
- S8 implementa protocolo/SDK genérico, stdio y artifact exchange; los adapters reales G-Skills/GSP/Radar, RecordSink y Agent Skill permanecen en S9-S13.
- Los locks actuales siguen correctamente `PENDING_REFRESH`/`PENDING` y `certified_compatibility=false`; metadata, tests genéricos o un manifest no pueden elevar esa condición.

**Decisiones de diseño antes de producción**

- `integration-protocol` no dependerá de SQLite ni de crates de adapters. Builders e integrity validators usarán los contratos v0.2.0 ya congelados y RFC 8785/SHA-256 existente.
- La identidad será exacta: sólo mappings `CONFIRMED`; ausencia o multiplicidad deshabilita lectura/escritura canónica y nunca se resuelve por nombre.
- El harness medirá una tupla observable de versión/event count/state digest antes y después de cada llamada. Se documenta expresamente que esto no constituye un sandbox universal.
- Stdio será JSON-RPC 2.0 con `Content-Length` estricto y límites de header/body. Artifact exchange será JSON canónico, acotado, sin symlink/traversal/overwrite y con publicación atómica no-clobber.
- La CLI sólo expondrá manifest/compatibility genéricos; no anunciará un runtime parcial de un adapter externo.

**Siguiente acción**

Escribir las pruebas RED de builders/digests, exact identity y upstream-lock inspection; implementar después el mínimo protocolo puro hasta GREEN.

**Hito S8.1 — Protocolo puro GREEN**

- RED confirmado: el crate existía sólo como frontera vacía y faltaban builders, integrity validators, resolución de identidad e inspección de locks.
- GREEN focalizado: 4/4 pruebas cubren manifest/health/projection/loss canónicos, límites de request/payload, freshness current/stale, identidad confirmada exacta, ambigüedad fail-closed y evidencia de upstream incompleta.
- El manifest genérico fija `projection=NONE`, `health=NONE`, `record_submit=DECLARED_ONLY`, `shared_database_required=false` y sólo el protocolo Ledger v0.2.0; no anuncia compatibilidad de producto.
- Los tres locks distribuidos permanecen `PENDING`: faltan schema/fixture/compatibility y, en G-Skills, refresh recíproco. Un `certified_compatibility=true` con evidencia ausente falla como overclaim.
- La primera compilación GREEN detectó un constructor de digest incorrectamente `const`; se aisló y retiró sólo ese calificador. Luego el validador rechazó un draft de prueba cuyo digest cero no coincidía con el fixture `015abd…`; se corrigió el dato de prueba, conservando intacta la validación estricta.
- Siguiente: escribir RED de framing `Content-Length` y artifact exchange canónico/no-clobber.

**Hito S8.2 — Stdio y artifact exchange GREEN**

- RED confirmado por ausencia de ambos crates/APIs.
- GREEN focalizado: 3/3 pruebas stdio y 2/2 de artifact exchange.
- El framing exige un único header ASCII `Content-Length`, JSON UTF-8 acotado y exactamente un payload; rechaza header duplicado/case alterado, longitud inválida, oversized, truncado, trailing bytes y JSON defectuoso antes del handler.
- El dispatcher conserva equivalencia del resultado directo y JSON-RPC, devuelve errores de método tipados y una llamada fallida no contamina la siguiente.
- Artifact exchange acepta sólo un filename `.json` seguro bajo un root explícito, bytes RFC 8785, digest exacto y límites previos/post lectura. Publica con temp file sincronizado + `persist_noclobber`, por lo que un destino existente permanece intacto.
- Los casos hostiles cubren traversal, extensión inválida, JSON no canónico, tampering, oversized y overwrite. La implementación también rechaza roots/files symlink mediante `symlink_metadata`.
- Siguiente: escribir RED del harness observable de conformance, malicious request suite y CLI genérica manifest/compatibility.

**Hito S8.3 — Harness y CLI genérica GREEN**

- RED confirmado: faltaban `ReadOnlyProvider`, estado observable, suite/report, malicious fixtures y comandos CLI de integración.
- GREEN focalizado final: 5/5 pruebas de conformance, 4/4 de contratos, 3/3 stdio, 2/2 artifact exchange y 5/5 CLI contract commands.
- El caso AC-043 ejecuta 100 projections actuales más stale y post-failure: 102 respuestas válidas, 7 requests maliciosos rechazados y cero cambios en `state_version/event_count/state_digest`.
- El harness detecta la primera mutación observable, timeout excedido, respuesta no determinista, identidad ambigua, negotiation/budget/privacy/authority inválidos y falla del provider; una llamada válida posterior demuestra aislamiento.
- La primera prueba de secreto tipado expuso que la privacidad revisaba keys pero no strings. Se añadió detección de marcadores `api_key/token/secret/password/bearer` no redactados; la prueba pasó sin relajar los contratos tipados.
- `project-ledger integration manifest` emite el mismo objeto canónico que el builder y el resultado stdio `ledger.manifest`. `integration compatibility` mantiene los tres locks en `PENDING` y la compatibilidad global en false.
- Clippy exigió contratos `# Errors`, eliminó un `expect` mediante matching total y motivó separar el loop de 100 lecturas y el corpus hostil en helpers con una responsabilidad única. Tras el ajuste, el pase focalizado quedó limpio.
- `integrations/sdk/README.md` documenta crates, checks, límites del harness y que mutable prepare/submit/reconcile pertenece al RecordSink S10; S8 no publica un runtime de escritura parcial.
- La revisión de arquitectura añadió un caso RED de binding observable: una respuesta podía ser autoconsistente pero declarar otro state digest. El harness ahora exige que `state_version/state_digest` del envelope coincidan con el snapshot observable capturado; la suite pasó 5/5.
- Siguiente: actualizar README/changelog/matriz, revisar diff y ejecutar el gate integral de Fase 5.

### 2026-08-23 — Cierre de S8 y Fase 5

**Implementación materializada**

- Nuevo crate `integration-protocol`, sin dependencia de SQLite, con builders e integrity validators para `IntegrationManifest`, `ProviderHealthReport`, `ProjectionEnvelope` y `LossReport`.
- El manifest genérico fija protocolo `project-ledger-integration/0.2.0`, dos projection capabilities, cinco record capabilities cerradas, reads sin side effects, writes sólo declarados y cero storage compartido.
- Requests exigen protocolo exacto, fields únicos/conocidos, budgets no nulos y acotados, privacy profile conocido y partición exacta coverage/omissions. Payload, schema, project, state version/digest y manifest quedan enlazados.
- `IdentityResolver` usa exclusivamente mappings `CONFIRMED` con system/ref exactos. Candidate/revoked/name-only no resuelven; múltiples targets distintos producen `AMBIGUOUS` sin canonical read/write.
- La inspección de upstream locks exige versión, spec digest, schema-registry digest, fixture-manifest digest y compatibility range. Los tres locks reales siguen `PENDING`; G-Skills conserva además `reciprocal_lock=PENDING_REFRESH`.
- Nuevo crate `transport-stdio` con JSON-RPC 2.0 genérico, bytes RFC 8785 de salida, un `Content-Length` ASCII exacto y límites de header/content; el handler no conoce storage ni productos.
- Nuevo crate `artifact-exchange` con filename JSON seguro, root/file no symlink, bounds, canonical JSON, digest exacto y publicación atómica temp+sync+`persist_noclobber`.
- El read-conformance harness compara `state_version/event_count/state_digest` antes y después de manifest, health, 100 projections, stale, requests hostiles y recuperación post-failure. Además liga cada envelope al snapshot observable.
- El corpus hostil contiene siete requests: protocol drift, zero/oversized budget, fields duplicados, authority overclaim, raw-private profile e identidad ambigua. Mutación, timeout medido, secreto no redactado, respuesta desligada y nondeterminismo también fallan.
- La CLI incorpora `integration manifest` e `integration compatibility`; manifest directo, stdout canónico y dispatch stdio son semánticamente equivalentes para la misma operación.
- `integrations/sdk/README.md`, README, changelog y matriz contractual documentan uso, gates y límites.

**Evidencia RED→GREEN y gate local definitivo**

| Verificación | Resultado |
|---|---|
| Ciclos RED focalizados | APIs de protocolo, framing/exchange, harness/CLI y binding observable ausentes o insuficientes antes de cada implementación |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 150 pruebas, 0 fallos |
| Conteo ejecutable `cargo test ... -- --list` | PASS — `workspace_test_count=150` |
| `cargo xtask schema check` | PASS — árbol generado sincronizado |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `cargo-deny 0.20.2 --offline check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

El primer gate agregado ejecutó formato, Clippy, 150 tests y schema check correctamente, pero el orquestador apuntó por error a `crates/specs-validator/Cargo.toml`, ruta inexistente. Se verificó la autoridad del plan, se corrigió sólo la invocación a `project-ledger specs/contracts verify` y se repitió el gate completo; el run definitivo terminó `gate=PASS`. Permanecen como warnings permitidos de `cargo-deny` las duplicidades transitivas conocidas de `getrandom`, `r-efi` y `syn`.

**Gate de Fase 5 y límites operacionales**

- El gate de Fase 5 queda verde: 102 projection calls válidas en el caso normativo, siete malicious requests rechazados, cero mutaciones observables y todos los fixtures de framing/artifact hostiles fail-closed.
- Avance real: 6/12 macrofases y 9/15 slices, según §50/§51.
- La comprobación de side effects cubre únicamente la tupla observable aportada al harness; no es sandbox universal ni impide por sí sola acciones de otro proceso con iguales privilegios.
- El timeout se mide y falla después de que la llamada retorna; la cancelación/preemption del proceso pertenece al runtime de adapter. S8 entrega framing/dispatch, no anuncia todavía un servidor G-Skills operativo.
- Mutable prepare/submit/reconcile, write trust, unknown outcome y result mapping permanecen en S10, donde existe el RecordSink concreto. No se publica un runtime de escritura parcial.
- Los adapters G-Skills/GSP/Radar siguen lock-gated y `certified_compatibility=false`. No se declara integración bilateral, CI remota, operación productiva, rentabilidad ni validez científica.

**Siguiente paso según la especificación**

Iniciar Fase 6 / S9: G-Skills provider exacto v0.3.0, comenzando por verificar/hidratar los contratos upstream, schema registry y fixtures exactos y mantener la compatibilidad lock-gated mientras el lock recíproco continúe `PENDING_REFRESH`; después implementar mapping de continuidad, budget, freshness y provider conformance sin acceso a DB compartida.
