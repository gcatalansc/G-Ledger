# Bitácora de implementación de Project Ledger

> Este fichero registra la vida de la implementación. No es el estado canónico del producto ni sustituye la futura proyección generada `BITACORA_PROYECTO.md`.

## Estado actual

| Campo | Valor |
|---|---|
| Fecha de actualización | 2026-08-23 |
| Estado | FASE 6 COMPLETADA — S9 CERRADO |
| Macrofases completadas (§50) | 7/12 |
| Slices completados (§51) | 10/15 |
| Última macrofase completada | Fase 6 — G-Skills provider |
| Macrofase actual | Fase 7 — G-Skills RecordSink (pendiente) |
| Último slice completado | S9 — G-Skills provider exacto read-only |
| Slice actual | S10 — G-Skills RecordSink (pendiente) |
| Último commit de baseline | `6584c04227356cb156ff2614f8564f2ce2e1a68a` |
| Commit de cierre más reciente | Este commit: `feat: add exact G-Skills continuity provider` |

## Autoridad y preservación

- Handoff operativo leído completamente: `referencias/PROJECT_LEDGER_v0.2.0_CODEX/`.
- Spec v0.2.0 verificada: SHA-256 `31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9`.
- Antecedente v0.1.0 localizado en el archivo externo preservado de G-Ledger Docs.
- Antecedente v0.1.0 verificado antes de copiar: SHA-256 `772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865`.
- El lock read-only de G-Skills está `COMPLETE` y sigue sin certificación bilateral; GSP y Radar permanecen `PENDING`/`LOCK_GATED`.

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

### 2026-08-23 — Inicio de S9 (Fase 6)

**Estado al inicio**

- HEAD limpio: `c9665f2 feat: add generic integration protocol and conformance`.
- Avance conservado hasta superar el gate: 6/12 macrofases y 9/15 slices.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-6-s9.md`.
- La ejecución continúa inline por autorización expresa del usuario y sin delegación.

**Reconciliación upstream verificada antes de producción**

- El handoff Ledger conserva como snapshot histórico G-Skills `E2E-CONSOLIDATED-1` (`8ddbb2a9...`) y marcaba el refresh recíproco pendiente.
- El repositorio hermano G-Skills está limpio y ya publica la revisión posterior `PL-UPSTREAM-REFRESH-1`: spec SHA-256 `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`.
- Su counterpart lock SHA-256 `49ff8bf60b33deb47cc688f288f7c2e1124d99fb223d0d8d4d2668ea4c60929d` fija el Ledger actual `31cb7cd4...` y preserva `e7acc496...`/`8ddbb2a9...` exclusivamente como historia pre-refresh.
- El registry y manifest de fixtures G-Skills existen localmente y fueron medidos por bytes: `de9131a8...` y `316c0564...`. S9 puede fijarlos sin inferir contratos por nombre.
- Los crates provider de G-Skills siguen reservados, por lo que el E2E de S9 se validará contra schemas/fixtures exactos y framing normativo, no contra una implementación counterpart inexistente.

**Fronteras de S9**

- Se implementará solo `ProjectContinuityProvider` read-only. RecordSink, writes, unknown outcome y revalidation mutable permanecen S10.
- El adapter no dependerá de SQLite; recibirá una proyección inmutable desde la composición Ledger.
- La fase puede cerrar su gate `projection/profile E2E PASS`, pero `certified_compatibility` seguirá en false hasta S10 y E2E bilateral.
- El mapping preservará versiones state/material, buckets de gates, classification, checkpoint N/N+1, repository binding, unknowns/conflicts y provenance; toda pérdida material bloqueará el gate.

**Siguiente acción**

Escribir las pruebas RED del lock, seis schemas/fixtures read-only y verificación de digests; después materializar el crate y los contratos vendorizados mínimos hasta GREEN.

**Hito S9.1 — Contratos upstream exactos GREEN**

- RED confirmado primero por paquete inexistente y después por nueve APIs/constantes ausentes en el crate ya registrable.
- Se vendorizaron los seis contratos read-only de S9 y sus 12 fixtures emparejados; no se copió ni redefinió un schema reducido.
- GREEN focalizado: 2/2 pruebas, con seis positivos aceptados, seis negativos `additionalProperties` rechazados y seis IDs/digests canónicos iguales al registry upstream.
- `upstream-lock.json` fija G-Skills `PL-UPSTREAM-REFRESH-1`, spec `957e62be...`, registry `de9131a8...`, fixture manifest `316c0564...`, counterpart lock `49ff8bf6...` y Ledger `31cb7cd4...`.
- El refresh recíproco queda `CURRENT` a nivel de spec/lock. Mapping, loss y conformance siguen `PENDING_S9`; RecordSink y bilateral E2E siguen `PENDING_S10`; `certified_compatibility=false`.
- La verificación es autocontenida en Ledger y no depende de que el repositorio hermano exista durante CI/release.
- Siguiente: pruebas RED del mapping literal, budget, freshness e identidad sobre `ProjectStateProjection` real.

**Hallazgo bloqueante S9.2 — conflicto spec/schema upstream reproducido**

- Antes de escribir el mapper se comparó el ejemplo y tabla normativa G-Skills §28.11 con el schema exacto fijado `urn:gskills:schema:project-continuity-projection:0.3.0`.
- El schema usa `additionalProperties=false` en `active_phases[]` y `next_actions.ordered[]`, admitiendo solo `ref` y `extensions`; rechaza `title/kind/status/started_at` de fase y `title/classification/status/authority_ref/gskills_derivation` de acciones exigidos por la spec.
- La caracterización ejecutable `pl_10_upstream_schema_currently_rejects_normative_phase_and_action_fields` pasa porque confirma el rechazo en ambas rutas. El pase focalizado total queda 3/3.
- `provider-field-mapping.json` registra las filas compatibles y dos grupos bloqueados. `provider-loss-report.json` registra `material_loss_count=2` y prohíbe drop, extensión no versionada, schema relajado o certificación.
- El lock conserva contratos read-only exactos, pero cambia mapping a `BLOCKED_UPSTREAM_SCHEMA_CONFLICT`, loss a `MATERIAL_LOSS` y conformance a `BLOCKED_BY_MATERIAL_LOSS`; compatibilidad certificada permanece false.
- No se inició producción del provider: TDD y REQ-PVD-008 impiden implementar una API cuyo comportamiento requerido no puede validar contra el contrato exacto.
- Resolver exige una nueva revisión autorizada en G-Skills con schema/fixtures/registry corregidos, seguida de refresh del lock Ledger y reanudación RED→GREEN de Tasks 2-4.

**Checkpoint verificable de S9 bloqueado**

| Verificación | Resultado |
|---|---|
| RED→GREEN de contratos upstream | PASS — paquete/API ausentes primero; luego 3/3 pruebas, incluida la caracterización del conflicto |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 153 pruebas enumeradas, 0 fallos |
| `cargo xtask schema check` | PASS — árbol generado sincronizado |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `project-ledger integration compatibility` | PASS conservador — G-Skills `PENDING`, faltan mapping/loss/conformance; global false |
| `cargo-deny 0.20.2 --offline check` | PASS — advisories/bans/licenses/sources |
| `git diff --check` | PASS |

El primer pase de `cargo-deny` detectó que el nuevo crate interno omitía la versión exacta de `project-ledger-canonical-json` y lo interpretaba como dependencia wildcard. Se añadió `version = "=0.2.0"`, siguiendo todos los crates existentes, y se repitieron los gates posteriores con resultado verde. Las advertencias permitidas de duplicados transitivos `getrandom`, `r-efi` y `syn` permanecen sin cambio.

La revisión pre-commit observó cambios ajenos de S8 en el worktree hermano G-Skills y los dejó intactos. Los cuatro inputs de autoridad usados por este checkpoint —spec, registry, fixture manifest y lock recíproco— se rehashearon pese a ello y siguen coincidiendo exactamente con `957e62be...`, `de9131a8...`, `316c0564...` y `49ff8bf6...`.

Este checkpoint se versionará como `chore: pin exact G-Skills read contracts and blocker`; no es un commit de cierre de S9/Fase 6. El avance permanece deliberadamente en 6/12 macrofases y 9/15 slices. No existe provider operativo, profile E2E aprobado ni compatibilidad certificada.

**Siguiente paso según la especificación**

Publicar bajo autoridad de G-Skills una revisión corregida y coherente de `ProjectContinuityProjection`, sus fixtures positivo/negativo y registry/manifest; después refrescar sus digests exactos en Ledger y reanudar Tasks 2-4 del plan S9 sin omitir ni relegar campos materiales.

### 2026-08-23 — Reanudación de S9 tras corrección contractual upstream

**Resolución autorizada del bloqueo**

- El repositorio G-Skills publicó el commit correctivo `6d63b60 fix(contracts): reconcile continuity projection schema` con worktree limpio y gate integral verde: formato, Clippy, 101 pruebas, build, generación sincronizada, cargo-deny, handoff 10/10 y diff check.
- La corrección incorpora en las rutas normativas de `ProjectContinuityProjection` los campos completos de fases y acciones; conserva `additionalProperties=false`, enums cerrados y exige `authority_ref` no vacío cuando la clasificación es `AUTHORIZED`.
- Ledger actualizó exclusivamente el schema de continuidad, sus fixtures emparejados, el lock de perfil y el hash del registry. No se relajó ni redefinió ningún contrato público.
- Identidades nuevas verificadas: registry de archivo `a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9`, registry canónico `sha256:c1451596f2d40ffebdb308c76c4b7ae8cfd4468f2a6a2f60b2a33d3b688776f1` y continuity schema canónico `sha256:99c2651ac831c41b3918639ad53117016e83e8baec06c80699901fde43fe6ee1`.
- El ciclo RED→GREEN de Ledger reprodujo primero los hashes obsoletos y el rechazo de campos normativos; tras el refresh, `upstream_contracts` quedó 3/3 PASS, incluyendo rechazo de propiedad anidada desconocida, taxonomía reescrita y `AUTHORIZED` sin autoridad.

**Estado y frontera**

- El bloqueo upstream queda resuelto y Tasks 2-4 de S9 pueden continuar.
- Avance conservado: 6/12 macrofases y 9/15 slices hasta que mapping, budgets, freshness, conformance, stdio/CLI y gate integral terminen en verde.
- `certified_compatibility` continúa false; esta reconciliación contractual no prueba provider operativo, RecordSink, E2E bilateral ni uso productivo.

**Siguiente acción**

Escribir las pruebas RED de mapping literal sobre una `ProjectStateProjection` construida desde eventos reales, incluyendo catálogo de fases explícito, versiones, gates, acciones, checkpoint, repository binding, unknowns/conflicts, progreso y provenance; implementar después el mínimo provider determinista sin SQLite.

**Hito S9.2 — Mapping, budgets y freshness GREEN**

- RED confirmado: `GSkillsProvider`, `ProviderContext`, `ProviderError` y la validación embebida de contratos no existían.
- Se implementó un provider storage-independent que recibe `ProjectStateProjection` inmutable; el crate no incorpora SQLite ni una superficie mutable.
- `ProviderContext.phase_catalog` aporta los campos de `Phase` que el estado compacto no conserva. La construcción falla cerrada si falta metadata activa, si el status contradice la proyección o si existen dos bindings `CURRENT`.
- El mapping literal preserva state/material versions y digest, authority docs, fases, decisiones, blockers/risks, buckets pending/revalidation/stale/verified, classification, checkpoint N/N+1, snapshot, unknowns/conflicts y el último progreso material resumido.
- `NextAction.status` se renderiza como espejo de presentación de su `classification`, porque Ledger v0.2.0 no posee un segundo campo de status; `gskills_derivation` permanece null y no se inventa una taxonomía externa.
- Request parsing valida schema y digest exactos, identidad proyecto/repositorio, write capability `NONE`, privacidad minimizada y bounds positivos para items/tokens/bytes/timeout. Una expected state anterior se conserva como freshness `SUPERSEDED_BY_NEWER_CHECKPOINT` sin alterar delivery.
- Manifest, health, projection, freshness y envelope se digieren canónicamente y se validan contra los schemas G-Skills vendorizados; `effective_authority_domains` queda vacío.
- `provider-field-mapping.json` registra el alcance ejecutable de 20 campos y `provider-loss-report.json` declara cero pérdida material dentro de ese alcance, sin elevar compatibilidad certificada.
- GREEN focalizado: 3/3 pruebas provider + 3/3 contratos upstream; formato, Clippy estricto y diff check PASS.
- La depuración de fixtures confirmó tres invariantes del aggregate: decisión inicial versión 1, `WorkRecord.recorded_at` igual al evento y checkpoint incapaz de elegir implícitamente entre dos repositorios actuales. Se corrigieron sólo los datos de prueba.

**Siguiente acción**

Escribir las pruebas RED del harness exacto y del dispatch stdio: repetición determinista current/stale, requests hostiles, cero mutaciones observables, aislamiento tras fallo y equivalencia entre llamada directa y JSON-RPC framed.

**Hito S9.3 — Conformance exacta y stdio GREEN**

- RED confirmado por ausencia de `GSkillsReadProvider`, suite/report de conformance y `GSkillsJsonRpcHandler`.
- El harness ejecuta manifest/health, 100 proyecciones current, una stale y recuperación post-fallo: 102 llamadas válidas ligadas a la misma tupla observable y bytes canónicos deterministas.
- Nueve requests hostiles cubren provider drift, budget cero/oversized, fields duplicados, escalamiento de write, raw events, identidad errónea, tampering sin digest y propiedad outer desconocida; todos son rechazados.
- Un provider de prueba que incrementa el event count observable después de proyectar demuestra que el harness detecta mutación y bloquea el gate.
- El handler registra únicamente `ledger.manifest`, `ledger.health` y `ledger.project_continuity`. Métodos mutables devuelven JSON-RPC `-32601`; requests inválidos devuelven `-32602` y una llamada válida posterior prueba aislamiento.
- Manifest, health y bundle de proyección framed son semánticamente idénticos a sus llamadas directas bajo el framing S8 `Content-Length` estricto.
- Los tests conformance/stdout se ejecutaron dos veces consecutivas: 2/2 + 2/2 PASS en ambos runs. Clippy focalizado estricto y formato PASS.
- `provider-conformance.json` registra S9 read profile PASS, 102 calls, nueve rechazos, cero mutaciones y cero pérdidas; `certified_compatibility` permanece false por S10 y E2E bilateral pendientes.

**Siguiente acción**

Integrar los comandos CLI exactos de verificación/servicio, documentar el perfil y ejecutar el gate integral de Fase 6 antes de actualizar avance o crear el commit de cierre.

### 2026-08-23 — Cierre de Fase 6 / S9: G-Skills provider exacto

**Implementación cerrada**

- La CLI incorpora `integration gskills verify-contracts` y `serve provider --stdio --profile gskills-context-provider/0.3.0`; cualquier alias de profile se rechaza.
- La composition root verifica la cadena SQLite, reconstruye la proyección completa e inyecta al adapter una vista inmutable junto con un catálogo explícito de fases. El crate `interop-gskills` sigue sin dependencia de SQLite.
- El servidor procesa frames `Content-Length` sucesivos y registra exclusivamente `ledger.manifest`, `ledger.health` y `ledger.project_continuity`; no existe método mutable.
- La prueba CLI E2E usa SQLite real y compara el bundle framed con la llamada directa. El lock G-Skills pasa a `COMPLETE` sólo para evidencia read-only exacta, mientras `certified_compatibility=false`, RecordSink y E2E bilateral continúan pendientes.
- Los reportes fijados por digest declaran mapping exacto de los 20 campos de la solicitud completa, cero pérdida material y conformance S9 PASS con 102 proyecciones, nueve requests hostiles rechazados, cero mutaciones observables y dos ejecuciones consecutivas.

**Corrección durante el gate**

- El primer gate integral detectó una expectativa histórica que exigía que todos los locks permanecieran `PENDING`. La causa fue semántica de test obsoleta: S9 ya aporta toda la evidencia read-only de G-Skills.
- Se actualizó la aserción genérica y su prueba CLI para exigir G-Skills `COMPLETE` sin faltantes, GSP/Radar `PENDING`, y `certified_compatibility=false` tanto por integración como globalmente. No se relajó el verificador ni se elevó el alcance certificado.

**Gate integral definitivo**

| Verificación | Resultado |
|---|---|
| Ciclos RED→GREEN focalizados | PASS — contratos, mapping/provider, conformance/stdio y CLI fallaron por APIs ausentes antes de cada implementación mínima |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 162 pruebas, 0 fallos |
| Conteo ejecutable `cargo test ... -- --list` | PASS — `workspace_test_count=162` |
| `cargo xtask schema check` | PASS — árbol generado sincronizado |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `project-ledger integration gskills verify-contracts` | PASS — 6 schemas, 12 fixtures, profile exacto v0.3.0 |
| `project-ledger integration compatibility` | PASS — G-Skills `COMPLETE`; GSP/Radar `PENDING`; global no certificado |
| `cargo-deny 0.20.2 --offline check` | PASS — advisories/bans/licenses/sources; sólo duplicados transitivos permitidos conocidos |
| `git diff --check` | PASS |

**Estado de cierre y límites**

- Avance real: 7/12 macrofases y 10/15 slices, según §50/§51.
- S9 prueba el provider read-only local exacto y fail-closed. No prueba un counterpart G-Skills operativo, E2E bilateral, sandbox universal, RecordSink, operación productiva, rentabilidad ni validez científica.
- `NextAction.status` continúa documentado como espejo de presentación de `classification`, no como un segundo estado durable inventado.
- Las advertencias permitidas de `cargo-deny` por duplicados transitivos `getrandom`, `r-efi` y `syn` no cambiaron.

**Siguiente paso según la especificación**

Iniciar Fase 7 / S10: implementar el `RecordSink` G-Skills con prepare/submit/reconcile, write trust explícito, mapping de comandos/resultados, revalidación de gates y persistencia de outcome desconocido; su gate exige cero efectos duplicados y cero reintentos ciegos, manteniendo fail-closed ante ambigüedad.

### 2026-08-23 — Inicio de Fase 7 / S10: G-Skills RecordSink

**Estado al inicio**

- HEAD limpio: `2b09a85 feat: add exact G-Skills continuity provider`.
- Avance conservado hasta superar el gate: 7/12 macrofases y 10/15 slices.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-7-s10.md`.
- La ejecución continúa inline por autorización expresa del usuario y sin delegación.

**Autoridad y contratos reconciliados**

- Project Ledger §16, §32–34, §40–41 y Fase 7 exigen escritura externa explícita, expected version, idempotencia, write trust separado, mapping cerrado, resultado de dominio separado de incertidumbre de transporte y cero blind retry/double effect.
- G-Skills `PL-UPSTREAM-REFRESH-1` define cinco contratos adicionales relevantes: `ExternalRecordCommand`, `ExternalRecordResult`, `ExternalSubmissionRecord`, `GateRevalidationSignal` y `CrossSystemReference`, junto con cinco capabilities allowlisted.
- Los cinco schemas/fixtures, registry y manifest están sin cambios en el commit G-Skills `6d63b60`. El worktree hermano contiene cambios ajenos de otra etapa; se leerán únicamente bytes rastreados desde `HEAD` y no se modificará ese repositorio.
- El registry upstream conserva SHA-256 de archivo `a6c3a12c...` y el fixture manifest `316c0564...`; los digests canónicos S10 medidos son command `8b6319d8...`, result `54daeed6...`, submission `48f8a00c...`, revalidation `c8d80e7d...` y cross-system `6054bd16...`.

**Frontera de implementación**

- `interop-gskills` validará/mapeará mediante un puerto storage-independent; sólo la CLI abrirá SQLite y ejecutará el `CommandProcessor` real.
- Projection trust no habilitará submit. Una policy explícita y digest-bound deberá conceder `TRUSTED_FOR_RECORDING` y allowlist de command kinds.
- `ledger.command_validate` y un `GateRevalidationSignal` aislado tendrán cero side effects. `status/reconcile` sólo consultarán el resultado durable y nunca reintentarán.
- `ExternalRecordResult` no podrá contener timeout, unavailable ni `OUTCOME_UNKNOWN`; esa incertidumbre pertenece al registro de submission del cliente.
- La etapa puede cerrar su gate local de Fase 7, pero `certified_compatibility=false` y E2E bilateral seguirán pendientes porque el counterpart G-Skills `crates/record-sink` aún es una frontera reservada.

**Siguiente acción**

Escribir el RED del lock ampliado de once contratos y comprobar que los cinco schemas/fixtures S10 aún no existen en Ledger; después vendorizar exactamente los bytes rastreados de G-Skills `6d63b60` y verificar sus IDs/digests sin relajar contratos.

**Hito S10.1 — Contratos write exactos GREEN**

- RED confirmado: el test no compiló porque `VENDORED_GSKILLS_CONTRACTS` y los cinco contratos S10 no existían.
- Se añadieron `ExternalRecordCommand`, `ExternalRecordResult`, `ExternalSubmissionRecord`, `GateRevalidationSignal` y `CrossSystemReference` con sus fixtures positivos/negativos emparejados.
- El lock de perfil reúne ahora once contratos S9+S10 y conserva el mismo spec/registry/fixture-manifest/counterpart lock; no se cambió el repositorio hermano G-Skills.
- GREEN focalizado: 3/3 pruebas. Los once `$id`, digests canónicos, positivos y negativos coinciden con el registry upstream.
- La validación de schema externo no habilita submit por sí sola: la semántica de trust, allowlist, identidad, expected state, idempotencia y payload cerrado se implementará en Task 2.

**Siguiente acción**

Escribir el RED de `RecordSinkPolicy` y del mapping de las cinco capabilities, demostrando que projection-only trust, lifecycle directo, schema refs pendientes, identidad/scope/digests divergentes y señales aisladas no pueden ejecutar el host.

**Hito S10.2 — Política y mapping semántico GREEN**

- RED confirmado: `GSkillsRecordSink`, `RecordSinkPolicy`, el puerto `RecordSinkHost` y los modelos preparados no existían.
- La policy cerrada, suministrada explícitamente, fija identidad cross-system, actor nativo, allowlist, trust domain y los digests de manifest, ejecutable, compatibility lock e identity map. `TRUSTED_FOR_PROJECTION` no se eleva: sólo `TRUSTED_FOR_RECORDING` permite construir el sink.
- `ledger.command_validate` valida schema y digest externos, expected project/material versions y state digest, scope de idempotencia, privacidad, schema ref `EXACT` y binding de aprobación; no llama a `execute`.
- Los IDs `external-command:<UUIDv7>` se normalizan a `command:<mismo UUIDv7>`; el envelope nativo se materializa canónicamente sólo en la frontera de submit y conserva un timestamp estable para replay determinista.
- Se mapearon únicamente `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION` y `CREATE_CONTINUITY_CHECKPOINT` a payloads Ledger cerrados. Las capabilities legacy `*_REFERENCE`, lifecycle directo y analogías de schema se rechazan.
- `GateRevalidationSignal` se valida completo contra schema y digest, identidad, gate, recomendación y referencias; produce sólo una solicitud nativa explícita y la señal aislada nunca ejecuta el host.
- Los reportes `record-sink-field-mapping.json` y `record-sink-loss-report.json` documentan mapping exacto, contexto externo preservado y cero pérdida material dentro del subconjunto aceptado, sin reclamar submit, E2E bilateral ni certificación.
- GREEN focalizado: 4/4 pruebas; Clippy estricto del crate, formato y `git diff --check` PASS. La depuración del gate corrigió sólo deuda de lint en documentación/nombres contractuales, sin cambiar semántica.

**Siguiente acción**

Escribir el RED de submit/result/status/reconcile y del handler stdio mutable, demostrando replay con un único efecto, version conflict sin overwrite, resultado durable idéntico tras reconcile, `NOT_FOUND` sin retry y separación estricta de `OUTCOME_UNKNOWN`.

**Hito S10.3 — Submit, reconciliación y stdio GREEN**

- RED confirmado en tres fronteras: faltaban `submit/status/reconcile`, el handler JSON-RPC mutable y el harness de conformance S10.
- El test host ejecuta realmente `CommandProcessor<SqliteEventStore, FixedClock>` con `CoreCommandHandler`; `interop-gskills` conserva SQLite sólo como dependencia de desarrollo y la librería sigue storage-independent.
- Submit materializa el `CommandEnvelope` canónico sólo después de validación. Una repetición explícita atraviesa una vez la idempotencia nativa y devuelve `DUPLICATE_REPLAY`; dos submits producen un único evento material.
- Same scope/key con digest distinto devuelve `IDEMPOTENCY_CONFLICT` sin evento. Una carrera entre revalidación y commit devuelve `VERSION_CONFLICT` sin overwrite.
- Una respuesta perdida después del commit queda como error de host, no como `ExternalRecordResult`; `reconcile` recupera el `CommandResult` durable sin segunda ejecución. `NOT_FOUND`, `status` y `reconcile` no llaman a submit.
- `ExternalRecordResult` preserva status, versiones, event refs, checkpoint, rejection, replay bit, timestamp y digest nativos. La referencia de proyección se obtiene mediante lectura confirmada posterior y `post_submit_projection_refs` permanece vacío.
- El handler framed registra sólo manifest, validate, submit, status y reconcile; frames truncados, métodos desconocidos y params inválidos quedan aislados y una llamada posterior válida se recupera.
- El harness local prueba nueve invariantes, incluidos cero duplicate effects, cero blind retries y cero estados de transporte dentro de resultados de dominio. El corpus protocolo/stdio/conformance pasó dos veces consecutivas: 8/8 + 8/8; Clippy estricto, formato y diff check PASS.
- `record-sink-conformance.json` fija esta evidencia local, manteniendo `certified_compatibility=false`; quedan la CLI SQLite real y el E2E bilateral G-Skills.

**Siguiente acción**

Escribir el RED de la composition root CLI `serve sink`, cargar una policy explícita digest-valid, ejecutar validate/submit/replay/status/reconcile sobre SQLite real y comprobar que la CLI no descubre ni concede write trust implícitamente.

### 2026-08-23 — Cierre de Fase 7 / S10: G-Skills RecordSink exacto

**Implementación cerrada**

- La CLI incorpora `serve sink --stdio --profile gskills-context-provider/0.3.0` con base SQLite real, identidades explícitas, policy obligatoria y reloj operativo declarado. La ausencia de policy o cualquier divergencia de identidad falla antes de exponer escritura.
- La frontera `interop-gskills` continúa independiente del storage: valida once contratos exactos S9+S10 y mapea únicamente las cinco capabilities autorizadas a comandos nativos cerrados.
- Write trust queda separado de projection trust. `TRUSTED_FOR_RECORDING`, allowlist, digests vinculados, identidad, material version, expected state, privacidad, idempotency scope y ventana vigente de aprobación son requisitos fail-closed en validate y submit.
- Submit usa el `CommandProcessor` canónico. Replay idéntico produce un solo efecto durable; conflicto de digest o versión no sobrescribe; status/reconcile sólo consultan y jamás reenvían.
- `ExternalRecordResult` contiene sólo el resultado nativo. Timeout, indisponibilidad y outcome desconocido permanecen como estado de transporte del cliente; una respuesta perdida tras commit se resuelve por reconciliación durable.
- Los reportes de mapping, pérdida y conformance quedan fijados por SHA-256 en `upstream-lock.json`. La evidencia local pasa, mientras E2E bilateral y certificación permanecen pendientes del counterpart G-Skills S24.

**Corrección durante el gate**

- La revisión adversarial detectó que `approval_expires_at` estaba comparado con `prepared_at`, pero no con el instante operativo actual. Se añadió `RecordSinkHost::observed_at()` y se exige `prepared_at <= observed_at <= approval_expires_at` tanto en validate como inmediatamente antes de submit.
- Status y reconcile conservan disponibilidad read-only después del vencimiento, pues no crean efectos. El reporte de mapping declara ahora explícitamente `APPROVAL_WINDOW_CURRENT_AT_VALIDATE_AND_SUBMIT` y el lock fue actualizado con su nuevo digest.

**Gate integral definitivo**

| Verificación | Resultado |
|---|---|
| Ciclos RED→GREEN focalizados | PASS — contratos, política/mapping, protocolo/conformance/stdio y CLI fallaron por fronteras ausentes antes de su implementación mínima |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --all-features` | PASS — 176 pruebas, 0 fallos |
| `cargo xtask schema check` | PASS — árbol generado sincronizado |
| `project-ledger specs verify` | PASS — v0.1.0 `772ea517`, v0.2.0 `31cb7cd4` |
| `project-ledger contracts verify` | PASS — 46 schemas, 92 fixtures, 53 filas |
| `project-ledger integration gskills verify-contracts` | PASS — 11 schemas, 22 fixtures, profile exacto v0.3.0 |
| `project-ledger integration compatibility` | PASS — G-Skills `COMPLETE`; GSP/Radar `PENDING`; global no certificado |
| `cargo-deny 0.20.2 --offline check` | PASS — advisories/bans/licenses/sources; sólo duplicados transitivos permitidos conocidos |
| `git diff --check` | PASS |

**Estado de cierre y límites**

- Avance real: 8/12 macrofases y 11/15 slices, según §50/§51.
- S10 demuestra el RecordSink local exacto, un efecto máximo por replay y cero blind retries. No demuestra que el counterpart G-Skills esté implementado, E2E bilateral, certificación global, operación productiva, rentabilidad ni validez científica.
- `certified_compatibility=false` y `bilateral_e2e_status=PENDING_COUNTERPART_GSKILLS_S24` se mantienen deliberadamente.
- El cierre se versiona mediante el commit de fase `feat: add exact G-Skills record sink`.

**Siguiente paso según la especificación**

Iniciar, sólo tras una nueva autorización, Fase 8 / S11: puente GSP exact-contract con corpus minimizado, sidecar de contratos upstream exactos, enlaces explícitos de resultados y aislamiento de autoridad; su gate exige cero degradación semántica y cero elevación de autoridad.

### 2026-08-23 — Inicio de Fase 8 / S11: puente GSP exact-contract

**Estado al inicio**

- HEAD limpio: `1f334e3 feat: add exact G-Skills record sink`.
- Avance conservado hasta superar el gate: 8/12 macrofases y 11/15 slices.
- Plan ejecutable: `docs/superpowers/plans/2026-08-23-project-ledger-phase-8-s11.md`.
- La ejecución continúa inline por autorización expresa del usuario, sin delegación y sin abrir S12.

**Autoridad GSP reconciliada**

- Project Ledger §35, AC-052…060 y Fase 8 exigen corpus minimizado, contratos GSP exactos, resultados enlazados por referencia, contradicción/incertidumbre preservadas, failure isolation y cero authority elevation.
- GSP-Gemma Compiler tag `v0.2.0`, commit `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, fija la especificación técnica SHA-256 `c838bdd8...`, el contrato de no degradación `30d40f79...`, su API `ca5d45e1...` y el lock upstream `c9fb65b3...`.
- La autoridad canónica es `gsp-spec` tag `gsp-exact-2026.04.12`, commit `004b14d7b64837025671970eac2469f092c49475`. El release declara ZIP `59395d1e...` y `HASH_MANIFEST.csv` `c6ca0f6d...`; ambos se descargaron/verificaron y coinciden con el checkout local limpio.
- El registry upstream SHA-256 `523faabc...` enumera 28 contratos. La superficie exacta contiene 31 artefactos JSON y 140 fixtures contractuales YAML: 56 válidos y 84 inválidos.
- No se modificará G-GSP. Los bytes se importarán desde el checkout canónico limpio y cada archivo quedará contrastado con el manifiesto upstream.

**Frontera de implementación**

- `interop-gsp` será Rust y storage-independent. La CLI será la única composition root SQLite; GSP/Python permanecerá sidecar opcional y el golden path seguirá funcionando deshabilitado.
- El corpus exigirá referencias explícitas y budgets duros. No habrá modo export-all, timeline completo, raw evidence output, paths, transcripts, secretos ni razonamiento privado.
- `EvidenceBundle` y los seis artifacts del `CompilationResult` se validarán contra los schemas upstream completos; Ledger no publicará modelos reducidos sustitutos.
- Contradicciones, uncertainty y validación partial/fail se conservarán en un reporte visible. Ningún claim GSP cerrará fases, verificará gates ni cambiará lifecycle.
- Un resultado sólo podrá enlazarse mediante el comando nativo `LINK_EXTERNAL_EVIDENCE`, separado, explícito, con expected version e idempotency key.
- El gate local puede cerrar S11 con degradación semántica y elevación de autoridad en cero, pero `certified_compatibility=false` seguirá vigente mientras no exista evidencia separada de un E2E live-sidecar externo.

**Siguiente acción**

Escribir el RED del crate y lock GSP: exigir 28 contratos, 31 artefactos JSON, 56 fixtures válidos y 84 inválidos, además de comprobar cada byte contra `HASH_MANIFEST.csv`; después importar la superficie upstream exacta sin reespecificarla.

**Hito S11.1 — Lock y replay contractual exactos GREEN**

- RED confirmado: `project-ledger-interop-gsp` compiló el test sin encontrar constantes, catálogo, error de drift ni verificador.
- Se importaron byte-for-byte los 31 artefactos JSON de schemas y los 140 fixtures contractuales YAML desde el checkout limpio `gsp-spec@004b14d`; no se modificó G-GSP.
- `GspContractCatalog` valida el registry contra su propio schema, exige 28 nombres/IDs/paths exactos, resuelve referencias offline mediante alias físicos y `schema://contracts/*`, y compila todos los contratos con formatos Draft 2020-12 activos.
- El verificador fija el `HASH_MANIFEST.csv` upstream y contrasta cada schema/fixture raw antes del replay. Los 56 fixtures válidos pasan y los 84 inválidos se rechazan según su contrato declarado.
- Una prueba adversarial agrega un byte al `EvidenceBundle` schema y obtiene `RawDigestMismatch` antes de validar fixtures.
- Durante GREEN se corrigió una suposición de envoltura: `NOTICE.md` y `LICENSES/MIT.txt` pertenecen al release público pero no a los 283 paths del manifiesto canónico; se conservan para atribución, fuera del conteo/verificador contractual.
- Evidencia focalizada: 2/2 pruebas PASS; Clippy estricto, formato y diff check PASS. La comparación independiente de todos los bytes vendorizados contra el checkout upstream también pasó.
- `upstream-lock.json` contiene ya versión, commits y digests exactos, pero conserva mapping y conformance en `PENDING_S11`; aún no se añadió compatibility range, evitando declarar integración completa antes de tiempo.

**Siguiente acción**

Escribir el RED del corpus minimizado: exigir scope explícito, budgets duros, referencias existentes, summaries de eventos allowlisted, ausencia de raw output/paths y un `EvidenceBundle` completo que valide contra el schema upstream sin degradarlo.

**Hito S11.2 — Corpus minimizado y EvidenceBundle exacto GREEN**

- RED confirmado: el test no compiló porque `GspCorpusRequest`, `GspCorpusBudget`, `GspCorpusError` y `build_minimized_corpus` no existían.
- El request carece deliberadamente de modo discovery/export-all: exige query, proyecto, referencias explícitas de authority/decision/gate/checkpoint/evento material, instante observado y budgets duros.
- La construcción valida identidad de proyecto, unicidad, existencia, allowlist del tipo de evento, máximo de 64 items y 1 MiB de JSON canónico. Scope vacío, referencia ausente/duplicada, evento no autorizado y exceso de items/bytes fallan cerrados.
- El manifest fija las seis clases de directorio, digests de query/estado/corpus, inventario de archivos y exclusiones. Declara falsos timeline completo, raw evidence output y paths de workspace; URIs `file:`/`workspace:` y paths absolutos se reemplazan por scheme y SHA-256.
- Cada selección se transforma en texto RFC 8785 digest-addressed con UUID determinista, source URI sintética segura, routing explícito y sufficiency conservadora `partial`. El valor resultante valida contra el `EvidenceBundle` upstream completo, sin schema sustituto reducido.
- Los eventos seleccionados no exportan el envelope/payload crudo: sólo identidad, tipo, versión, tiempos, digests y summary allowlisted por variante. El estado de conflicto abierto se propaga a `conflicts_present`.
- Se publicaron el schema local del manifest y reportes de mapping/pérdida. Registran cero pérdida material no declarada y cero authority elevation, pero dejan explícitamente pendientes la validación del resultado, aislamiento del sidecar, link explícito, CLI SQLite y E2E live-sidecar.
- GREEN focalizado: 2/2 pruebas de corpus y 2/2 pruebas contractuales PASS; salida repetida idéntica; validación de manifest y `EvidenceBundle` PASS; Clippy estricto, formato y `git diff --check` PASS.

**Siguiente acción**

Escribir el RED del `CompilationResult`: exigir los seis artefactos upstream completos, coherencia de `request_id`, digests canónicos, contradicción/incertidumbre visibles, error aislado del sidecar sin eventos y un único payload posible de link externo sin autoridad de lifecycle.

**Hito S11.3 — Resultado exacto, aislamiento y link explícito GREEN**

- RED confirmado: no existían `GspBridge`, `GspCompilerSidecar`, errores de sidecar/bridge ni los modelos de resultado, receipt y reporte.
- `GspCompilationResult` conserva como `Value` los seis artefactos completos y modela sólo la envoltura pública exacta de siete campos de GSP v0.2; campos extra/reducidos fallan por `deny_unknown_fields` o required fields.
- `GspBridge` valida primero el `EvidenceBundle`, exige que el sidecar lo devuelva idéntico y valida candidate/compilation/validation/uncertainty/task projection contra los seis schemas upstream. También exige `request_id` común, `validation.status=pass`, autoridad no vacía y la lista exacta ordenada de contratos comprobados.
- Los seis outputs permanecen inalterados y quedan referenciados por digests RFC 8785 en un receipt local. Este preserva proyecto, versiones/digest de estado, query/corpus digests, refs de fuentes, autoridad runtime digestada y semantic status.
- `partial`/`fail`, contradiction clusters, `conflicts_present`, uncertainty items y uncertainty notes no se normalizan: producen `GspConflictVisibility` y human review visible. El estado contractual `validation.status=pass` no se confunde con semantic validation `pass|partial|fail`.
- El puerto `GspCompilerSidecar` no posee storage. Un sidecar `Unavailable` contra SQLite real dejó el conteo 0→0 eventos. No se incorporó Python, DB compartida, red ni daemon al crate de producción.
- El receipt sólo puede producir `LinkExternalEvidencePayload` con schema fijo y digest propio. Ejecutado explícitamente por `CommandProcessor<SqliteEventStore, FixedClock>`, añadió exactamente un `EXTERNAL_EVIDENCE_LINKED`; replay idéntico no agregó otro y lifecycle/fases/gates no cambiaron.
- Se publicaron el schema del receipt y `gsp-conformance.json`. El reporte fija degradación semántica 0 y elevación de autoridad 0, pero declara que el sidecar usado por pruebas es un fixture in-process y no un runtime Python externo.
- GREEN focalizado: 9/9 pruebas del crate PASS (5 bridge/conformance, 2 corpus, 2 lock); Clippy estricto, formato y `git diff --check` PASS.

**Siguiente acción**

Escribir el RED de la composition root CLI GSP sobre SQLite real: verificar contratos, exportar corpus sin mutación, validar resultado/emitir receipt sin mutación y enlazarlo sólo con parámetros write explícitos, expected version e idempotencia.

**Hito S11.4 — Workflow CLI SQLite GREEN previo al gate integral**

- RED confirmado: `integration gsp` no existía y las tres pruebas CLI fallaron por subcomando desconocido.
- `verify-contracts` ejecuta el verificador raw/registry/fixture del crate y reporta 28 contratos, 31 schemas y 140 fixtures para el tag exacto.
- `export-corpus` exige DB existente, proyecto y request cerrado; verifica cadena/rebuild y publica JSON canónico con `create_new`, sin append. Scope desconocido, identidad divergente y overwrite fallan antes de publicar.
- `validate-result` carga el corpus completo, valida el resultado exacto y publica result+receipt+report canónicos no-clobber. Resultado reducido o output existente falla sin evento.
- `link-result` exige DB, proyecto, receipt, URI, expected state version, idempotency key, actor y observed time. Verifica identidad/digest del receipt y deriva un `CommandId` UUIDv7 determinista del scope para que replay CLI idéntico conserve el mismo command digest.
- En SQLite real, export y validate conservaron 7→7 eventos; link produjo 7→8, replay permaneció en 8 y lifecycle/fases/gates no cambiaron. Expected version stale y argumentos omitidos fallaron cerrados.
- El lock GSP incorpora range exact-contract, schemas/reportes con SHA-256, mapping/conformance local PASS, degradación 0, authority elevation 0 y CLI real PASS. `live_external_sidecar_e2e_status` permanece pendiente y `certified_compatibility=false`.
- README, changelog, matriz de cobertura y documento de compatibilidad distinguen conformance local, fixture sidecar, link de evidencia y E2E externo no demostrado.
- GREEN focalizado: 3/3 pruebas CLI PASS; Clippy estricto del CLI y `git diff --check` PASS.

**Siguiente acción**

Ejecutar el gate integral S11 completo, revisar hashes/lock/diff/estado Git, actualizar el avance real a 9/12 macrofases y 12/15 slices sólo si todo pasa, y crear el commit de fase `feat: add exact GSP bridge`.

**Hito S11.5 — Corrección adversarial de binding del corpus y receipt**

- La revisión previa al cierre detectó que `validate-result` validaba el `EvidenceBundle` upstream, pero no comprobaba todavía que el manifest Ledger persistido siguiera unido al bundle que alimentaba el receipt.
- RED de regresión confirmado: una alteración de `project_state_version` en el manifest fue aceptada inicialmente por `GspBridge::validate_result`.
- `GspCorpus::verify_integrity` verifica ahora digests canónicos de cada contenido, query normalizada y corpus completo. El `corpus_digest` cubre manifest más `EvidenceBundle`, con sólo su propio campo puesto a cero durante el hash; cambiar cualquiera de ambos invalida el artefacto.
- `compile` y `validate_result` ejecutan esa comprobación antes de invocar el sidecar o interpretar resultados.
- Un segundo RED demostró que un receipt autoconsistente pero con `project_state_digest` falsificado podía llegar al command processor. `link-result` verifica ahora cadena y proyección SQLite y contrasta `material_state_version` y `state_digest` cuando se realiza el primer enlace en la versión capturada.
- El replay idempotente posterior continúa permitido: cuando el stream ya avanzó, el command processor decide entre `DUPLICATE_REPLAY` y rechazo stale sin producir una segunda mutación.
- GREEN focalizado: la regresión de corpus alterado y el workflow CLI con receipt falsificado pasaron; el intento adversarial dejó el conteo de eventos intacto.

**Siguiente acción**

Repetir desde cero el gate integral S11 sobre el árbol final corregido y cerrar bitácora/plan/commit únicamente si todas las verificaciones vuelven a pasar.

### 2026-08-23 — Cierre de Fase 8 / S11: puente GSP exact-contract

**Resultado implementado**

- Se fijó la autoridad GSP exacta en `gsp-exact-2026.04.12` / `004b14d7b64837025671970eac2469f092c49475`, con 28 contratos, 31 artefactos JSON y 140 fixtures YAML (56 válidos, 84 inválidos) verificados byte-for-byte y ejecutados offline.
- `interop-gsp` implementa export de corpus allowlisted/minimizado, `EvidenceBundle` upstream completo, digest que une manifest+contenidos+bundle, validación de seis artefactos exactos, receipts digest-addressed, contradicción/incertidumbre visible y sidecar port sin storage/Python.
- La CLI implementa `verify-contracts`, `export-corpus`, `validate-result` y `link-result`. Export/validate son read-only y no-clobber; el único write es `LINK_EXTERNAL_EVIDENCE` explícito, con receipt unido a la proyección SQLite vigente, expected version e idempotencia.
- La compatibilidad local informa G-Skills `COMPLETE`, GSP `COMPLETE` y Project Radar `PENDING`, manteniendo `certified_compatibility=false` global y por integración.

**Correcciones descubiertas durante el gate**

- Dos tests históricos todavía esperaban GSP `PENDING`; se actualizaron para exigir GSP `COMPLETE`, Radar `PENDING` y conservar toda compatibilidad sin certificar.
- La revisión adversarial añadió y cerró dos RED: alteración corpus↔manifest↔bundle y receipt autoconsistente con state digest falso. Ambos fallan ahora antes de cualquier mutación.
- Los reportes de loss/conformance aún enumeraban pasos locales ya completados como pendientes; se reconciliaron y sus hashes se actualizaron en el lock. Sólo `LIVE_EXTERNAL_SIDECAR_E2E` permanece pendiente.
- La auditoría del índice detectó normalización CRLF→LF sobre bytes upstream. `.gitattributes` marca la superficie vendorizada exacta como `-text -diff`; los 175 blobs del índice coinciden byte-for-byte con el working tree y el manifiesto conserva SHA-256 `c6ca0f6d...`.

**Gate integral final**

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: PASS; 189 atributos de prueba inventariados, cero fallos.
- `cargo xtask schema check`: PASS.
- `project-ledger specs verify`: PASS; v0.1 `772ea517`, v0.2 `31cb7cd4`.
- `project-ledger contracts verify`: PASS; 46 schemas, 92 fixtures, 53 filas de matriz.
- `integration gskills verify-contracts`: PASS; 11 schemas y 22 fixtures.
- `integration gsp verify-contracts`: PASS; 28 contratos, 31 schemas y 140 fixtures exactos.
- `integration compatibility`: PASS; locks locales G-Skills/GSP completos, Radar pendiente, certificación falsa.
- `cargo-deny --offline check`: PASS para advisories, bans, licenses y sources; persisten sólo duplicidades transitivas informativas de `getrandom`, `r-efi` y `syn`.
- JSON/hash lock de los dos schemas locales y tres reportes GSP: PASS.
- `git diff --check`: PASS.

**Estado y límites de evidencia**

- Fase 8 / S11 cerrada.
- Avance real: 9/12 macrofases y 12/15 slices, según §50/§51.
- Degradación semántica no declarada: 0. Elevación de autoridad: 0.
- S11 demuestra conformance contractual local, SQLite real y aislamiento del puerto sidecar. No demuestra un proceso GSP/Python externo vivo, compatibilidad runtime certificada, corrección semántica/científica, operación productiva ni rentabilidad.
- `live_external_sidecar_e2e_status=PENDING_SEPARATE_EVIDENCE` y `certified_compatibility=false` permanecen deliberadamente.
- El cierre se versiona mediante el commit de fase `feat: add exact GSP bridge`.

**Siguiente paso según la especificación**

Iniciar, sólo tras una nueva autorización, Fase 9 / S12: articulación Project Radar mediante schemas/profile, project link, artifact export, agregación de catálogo, fixtures de reference importer, mapping de status/progress, ownership de campos y provenance transitiva. Su gate exige PL-21…32 y AC-065…080 PASS; S12 no modifica el código de Project Radar desde este repositorio.

### 2026-08-23 — Apertura de Fase 9 / S12: perfil de artefacto Project Radar

**Autoridad y alcance reconciliados**

- Se releyeron `00_START_HERE.md`, el lock Project Radar y las secciones normativas §36–§37, PL-21…32, AC-065…080, §50 y §51 antes de editar producción.
- El alcance autorizado es exclusivamente Ledger: contratos/profile `project-ledger-radar-artifact/0.1.1`, export read-only, catálogo explícito, fixtures de importador de referencia, mapping conservador, ownership y provenance.
- No se localizaron ni se autorizan cambios en un checkout de Project Radar. No habrá acceso a IndexedDB, DB compartida, escritura remota ni certificación bilateral.
- El source spec digest permanece fijado en `277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b`; `counterpart_extension_status=PENDING` y `certified_compatibility=false` no se elevarán por tests locales.
- Se creó el plan persistente `docs/superpowers/plans/2026-08-23-project-ledger-phase-9-s12.md` con ejecución TDD inline, gates completos y límite explícito de no iniciar S13.

**Estado de apertura**

- Base Git limpia confirmada en `a46d29d` (`feat: add exact GSP bridge`).
- Avance se mantiene en 9/12 macrofases y 12/15 slices hasta que S12 supere su gate completo y sea versionada.
- Primer paso de implementación: escribir y ejecutar el RED del lock, registry y replay exactos del perfil Radar.

**Hito S12.1 — Profile contracts RED confirmado**

- Se añadió el test de contrato antes de la implementación. Exige el perfil y source digest literales, cinco schemas, cinco fixtures válidos, cinco inválidos, hashes de registry/manifest en el lock y conservación de la certificación falsa.
- La prueba adversarial prepara una copia aislada de toda la superficie y agrega un byte al schema de proyección; el resultado requerido es `RawDigestMismatch` antes del replay.
- RED confirmado con `cargo test -p project-ledger-interop-radar --test profile_contracts`: el crate de prueba no encontró constantes, catálogo, error ni verificador.
- Los manifests del perfil referencian los schemas y fixtures canónicos generados de Ledger, evitando una segunda copia que pudiera divergir. Cada path queda fijado por SHA-256 de bytes crudos.

**Siguiente acción**

Implementar el catálogo offline mínimo, validar shape/identidad/hashes antes de compilar schemas, ejecutar todos los fixtures y volver a GREEN.

**Hito S12.2 — Profile contracts GREEN y export RED**

- `RadarContractCatalog` valida primero los bytes de ambos manifests y de cada artefacto, luego identidad `$id`, compilación de schema y resultados de los diez fixtures. No usa red ni resolución externa.
- GREEN focalizado: 2/2 tests PASS, incluido drift de un byte; Clippy estricto del crate, formato y `git diff --check` PASS.
- El lock fija `profile_schema_sha256`, `schema_registry_sha256` y `fixture_manifest_sha256`, pero todavía no declara mapping/conformance S12 completo.
- Se escribió el RED de exportación antes del builder. Exige la matriz normativa de status, rechazo de estados activos no mapeados, progreso estable o explícitamente unavailable, privacidad mínima, ausencia de Radar identity/`NOT_IMPLEMENTED`, self-digests y envelope sin tombstones.
- RED confirmado: `RadarEnvelopeRequest`, `RadarExportContext`, ambos builders y el mapper de status no existen todavía.

**Siguiente acción**

Implementar la derivación pura desde `ProjectStateProjection` más metadata explícita de fases, con validación fail-closed y digests RFC 8785.

**Hito S12.3 — Export GREEN e importador de referencia RED**

- La proyección valida el estado antes de mapear, exige metadata de fases activa coherente y rechaza combinaciones `ACTIVE + phase kind` no incluidas en la tabla normativa en vez de inventar `NOT_IMPLEMENTED`.
- Status, progress, counts, diez códigos de atención allowlisted, freshness, identidad Ledger, privacidad y provenance se derivan sin incluir un Radar project ref ni campos Radar-owned.
- El porcentaje sólo está disponible con roadmap lineal y denominador de fases completo; roadmap ausente, metadata incompleta, mismatch o denominador cero producen razón explícita y `basis_points=null`.
- El envelope ordena proyectos, rechaza duplicados y child digests alterados, contiene cero tombstones y fija las semánticas no destructivas del profile. Projection y envelope usan self-digest RFC 8785.
- La expectativa de `LEDGER_NO_MATERIAL_ACTIVITY` sobre genesis se corrigió: `PROJECT_CREATED` ya es actividad material canónica y la spec no define un umbral temporal; no se inventó uno.
- GREEN focalizado: 3/3 tests de export y 2/2 de contratos PASS. El primer Clippy detectó sólo una comparación booleana literal en el test, ya corregida.
- RED del importador confirmado: faltan el enum exacto de cinco métodos Radar y todos los modelos/evaluador de importación.

**Siguiente acción**

Implementar la evaluación pura de identity/merge, preservando campos Radar-owned y devolviendo preview/conflicto sin puertos de persistencia.

**Hito S12.4 — Reference importer y CLI SQLite GREEN previo al gate**

- `RadarProjectLink` usa ahora los cinco métodos exactos del contrato Radar: `USER_CONFIRMED`, `EXACT_STABLE_ID`, `EXACT_REPOSITORY_REMOTE_DIGEST`, `SIGNED_MAPPING` e `IMPORTED_MAPPING_REVIEWED`. El enum genérico de identidad Ledger permanece separado.
- El evaluador puro valida envelope, child projections y link por self-digest antes del preview. Link ausente/nombre/alias produce candidato; link no confirmado no aplica; mismatch de Radar/Ledger identity produce conflicto.
- La matriz implementada distingue `DUPLICATE_REPLAY`, `STALE_IGNORED`, `APPLY_OVERLAY_UPDATE`, `SOURCE_DIVERGENCE` y `NO_ACTION`; nunca crea clear/delete/archive ni usa last-write-wins.
- Priority, tags, notes, ideas, chats, tasks, artifacts y presentation se copian sin alteración a cada evaluación. El overlay conserva `provenance_ref` de Ledger y declara `lifecycle_authority=project-ledger`.
- Tras el cambio contractual se regeneró el schema `RadarProjectLink`, se actualizó su digest en el registry del profile y `cargo xtask schema check` volvió a PASS. Un intento inicial con el subcomando inexistente `schema write` se diagnosticó por el usage real y se corrigió a `schema generate`; no hubo cambio de producto asociado al error.
- La CLI añade `integration radar verify-contracts|export|validate-export` y `catalog radar-export --all`. Recibe DBs/proyectos/catálogos de fase explícitos, verifica cadena y proyección, escribe JSON canónico no-clobber y no descubre el filesystem.
- GREEN focalizado: 2/2 tests del importador, 3/3 de CLI Radar, 3/3 de export y 2/2 de contracts PASS; export/validate/catalog conservaron el conteo de eventos en SQLite real.
- Se publicaron mapping, loss y conformance con hashes fijados en el lock. Degradación semántica no declarada 0, elevación de autoridad 0; `counterpart_extension_status=PENDING`, `bilateral_e2e_status=PENDING_SEPARATE_EVIDENCE` y `certified_compatibility=false` siguen explícitos.
- README, changelog, matriz ejecutable y compatibilidad Radar distinguen cierre local S12 de código/IndexedDB/certificación counterpart.

**Siguiente acción**

Ejecutar el gate integral final desde el árbol reconciliado, revisar hashes/lock/diff/estado Git y cerrar Fase 9 / S12 con un solo commit únicamente si todo vuelve a pasar.

**Hito S12.5 — Primera pasada del gate y corrección de expectativa histórica**

- Formato y Clippy workspace pasaron, pero el workspace test detectó una expectativa histórica: `integration-protocol/tests/contracts.rs` aún esperaba Project Radar `PENDING` pese a que S12 ya aporta spec/schema/fixtures/range locales completos.
- La implementación y la CLI coincidieron en `status=COMPLETE`, `missing_evidence=[]` y `certified_compatibility=false`; se actualizó únicamente la expectativa Radar a `COMPLETE`, manteniendo la aserción de no certificación.
- El resto de la primera pasada continuó verde: schema/spec/contract checks, contratos G-Skills/GSP/Radar, compatibility y cargo-deny. Esta pasada no sirve como cierre porque el workspace test global tuvo un fallo.

**Siguiente acción**

Repetir el gate integral completo desde cero después de la corrección y usar sólo esa evidencia fresca para el cierre.

### 2026-08-23 — Cierre de Fase 9 / S12: perfil de artefacto Project Radar

**Resultado implementado**

- Se incorporó `project-ledger-interop-radar` con replay offline y byte-exacto del perfil `project-ledger-radar-artifact/0.1.1`: cinco schemas, cinco fixtures válidos y cinco inválidos, con rechazo adversarial de drift antes de validar contenido.
- La proyección y el envelope Radar se derivan de estado Ledger validado, con status/progress fail-closed, privacidad mínima, provenance y self-digests canónicos. No contienen `radar_project_ref`, tombstones ni campos de autoridad Radar.
- El importador de referencia es una evaluación pura: requiere enlace fuerte confirmado para aplicar, distingue replay/stale/update/divergence/no-action, preserva todos los campos Radar-owned y no dispone de puerto de persistencia.
- La CLI expone verificación, export, validación y catálogo sobre proyectos conocidos explícitamente. Cada operación valida SQLite real, usa salida canónica no-clobber y conserva el conteo de eventos.
- Mapping, loss y conformance quedaron fijados por hash. La integración local figura `COMPLETE`, pero `counterpart_extension_status=PENDING`, `bilateral_e2e_status=PENDING_SEPARATE_EVIDENCE` y `certified_compatibility=false` permanecen deliberadamente.

**Gate integral final**

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: PASS; 199 atributos `#[test]` inventariados, cero fallos.
- `cargo xtask schema check`: PASS.
- `project-ledger specs verify`: PASS; v0.1 `772ea517`, v0.2 `31cb7cd4`.
- `project-ledger contracts verify`: PASS; 46 schemas, 92 fixtures y 53 filas de matriz.
- Contratos G-Skills, GSP y Radar: PASS; Radar verificó cinco schemas y diez fixtures.
- `integration compatibility`: PASS; los tres perfiles locales están completos y ninguno certificado.
- `cargo deny --offline check`: PASS para advisories, bans, licenses y sources; sólo permanecen las duplicidades transitivas informativas ya conocidas de `getrandom`, `r-efi` y `syn`.
- Los SHA-256 del registry, manifest y tres reportes Radar coinciden con el lock; `git diff --check`: PASS.

**Estado y límites de evidencia**

- Fase 9 / S12 cerrada; avance real: 10/12 macrofases y 13/15 slices, según §50/§51.
- Degradación semántica no declarada: 0. Elevación de autoridad: 0.
- S12 no demuestra código Project Radar, acceso IndexedDB, interoperabilidad bilateral viva, compatibilidad certificada, operación productiva ni validez científica.
- El cierre se versiona mediante el commit de fase `feat: add Project Radar artifact profile`.

**Siguiente paso según la especificación**

Iniciar, sólo tras una nueva autorización, Fase 10 / S13 Agent Skill/Codex: manifest versionado, workflow, ejemplos, casos de trigger positivos/negativos, degradación explícita si falta la herramienta y handoff E2E; el gate exige conformance de la skill y ausencia de bypass de autoridad.

### 2026-08-23 — Apertura de Fase 10 / S13: Agent Skill/Codex

**Autoridad y alcance reconciliados**

- Se releyeron el handoff, `INTEGRATION_LOCKS.json` y las secciones normativas §27, §36, §50, §51, AC-061…064 y el E2E de §58 antes de editar implementación.
- La skill será un adaptador procedimental versionado: no contiene estado canónico, no es dependencia del core, no lee SQLite ni edita Markdown generado y no puede completar fases o gates.
- La revisión de la CLI detectó que `resume` y `checkpoint create`, aunque normativos y respaldados por motores S6 existentes, todavía no estaban expuestos. S13 añadirá sólo ese puente mínimo para demostrar el handoff E2E real.
- El plan persistente `docs/superpowers/plans/2026-08-23-project-ledger-phase-10-s13.md` fija TDD, schemas/profile, casos de trigger, degradación explícita, no-authority y gate completo.

**Estado de apertura**

- Base Git limpia confirmada en `d78a454` (`feat: add Project Radar artifact profile`).
- Avance se mantiene en 10/12 macrofases y 13/15 slices hasta que S13 supere su gate integral y sea versionada.
- Primer paso: escribir y observar el RED del contrato exacto del paquete antes de crear la skill.

**Hito S13.1 — Contrato del paquete RED→GREEN**

- RED confirmado con `cargo test -p project-ledger-agent-skill --test package_contract`: el crate todavía no exponía perfil, error ni verificador.
- El paquete sigue la anatomía Agent Skills (`SKILL.md` con frontmatter) y añade el manifest normativo de Ledger, workflow, dos schemas cerrados, dos ejemplos, siete casos de trigger y cinco casos de política.
- El profile lock fija spec `31cb7cd4…`, versión `0.2.0`, rango de herramienta `[0.2.0,0.3.0)`, Windows/Linux/macOS, `state_authority=false`, `core_dependency=false` y certificación falsa.
- El verificador valida primero bytes crudos de registry/fixture manifest y cada artefacto, luego schemas, self-digests, identidad del perfil y ausencia de `project:<uuid>`/`checkpoint:<uuid>` concretos en contenido ejecutable.
- GREEN focalizado: 2/2 tests PASS, incluido drift adversarial de un byte antes del replay semántico.

**Siguiente acción**

Escribir el RED conductual para triggers, missing-tool, version conflict, unknown outcome y no-authority; después implementar el planner puro y la conformance suite.

**Hito S13.2 — Conformance conductual RED→GREEN**

- RED confirmado: faltaban intent/actions/status, runtime conditions, planner, inspección de contenido y conformance runner.
- El planner puro separa `NOT_TRIGGERED`, `READY`, `DEGRADED_MISSING_TOOL`, `REPLAN_REQUIRED` y `RECONCILE_REQUIRED`; toda salida fija `completion_claim_allowed=false`.
- Cinco triggers positivos y dos negativos se ejecutan desde fixtures. Las cinco políticas cubren herramienta ausente, conflicto de versión, resultado incierto, evaluación sin evidencia y handoff gobernado.
- La mutación adversarial `DECLARE_PHASE_COMPLETE` en `ordered_actions` se rechaza como `AuthorityBypass`; current state concreto se rechaza, mientras placeholders no canónicos permanecen válidos.
- GREEN focalizado: 4/4 tests conductuales y 6/6 tests totales del crate PASS.

**Siguiente acción**

Escribir el RED de CLI para `integration agent-skill verify-contracts`, resume read-only y checkpoint→resume real sobre SQLite.

**Hito S13.3 — Handoff CLI SQLite RED→GREEN**

- El primer intento RED se corrigió porque el test local ocultaba la función `checkpoint` con una variable del mismo nombre; tras reparar el test, el RED válido mostró `agent-skill` no reconocido y ausencia de `resume`.
- La CLI añade `integration agent-skill verify-contracts`, `resume --database --project-ref --format codex|json` y `checkpoint create` con version/idempotency/actor/timestamp explícitos.
- Resume verifica cadena, reconstruye el stream y usa el `ResumeBuilder` S6 existente sin añadir eventos. Checkpoint entra por `CommandProcessor` + `CoreCommandHandler`; la skill no abre SQLite directamente.
- El E2E real ejecuta init→resume(1 evento)→checkpoint(2 eventos)→duplicate replay(2)→resume con `CURRENT_FOR_PROJECT_ONLY`; una idempotency nueva con expected version stale devuelve `VERSION_CONFLICT` y conserva 2 eventos.
- GREEN focalizado: 2/2 tests CLI PASS. La compatibilidad incorpora `agent-skills-codex` como perfil local `COMPLETE`, siempre `certified_compatibility=false`.

**Siguiente acción**

Reconciliar README/changelog/matriz/compatibilidad, validar la anatomía de la skill con el validador oficial local y ejecutar el gate integral.

### 2026-08-23 — Cierre de Fase 10 / S13: Agent Skill/Codex

**Resultado implementado**

- Se publicó `project-ledger-continuity` v0.2.0 con frontmatter Agent Skills válido, manifest/profile, workflow cerrado, dos schemas, registry/fixture manifest, dos ejemplos, siete triggers y cinco políticas raw-locked.
- `project-ledger-agent-skill` valida offline bytes, schema, self-digests, ausencia estructural de current state concreto y ejecuta la conformance conductual. Tool ausente, version conflict y unknown outcome fallan cerrados.
- El workflow prohíbe DB directa, edición de Markdown generado, blind retry, `DECLARE_GATE_VERIFIED` y `DECLARE_PHASE_COMPLETE`; toda planificación mantiene `completion_claim_allowed=false`.
- La CLI incorpora verificación del paquete, resume canónico read-only y checkpoint gobernado. El E2E SQLite demuestra checkpoint N/N+1, replay idempotente, freshness vigente y conflicto de versión sin evento adicional.
- `integration compatibility` incluye los cuatro perfiles locales como `COMPLETE`, todos con `certified_compatibility=false`.

**Gate integral final**

- Validador oficial local de `skill-creator`: PASS (`Skill is valid!`).
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: PASS; 207 atributos `#[test]` inventariados, cero fallos.
- `cargo xtask schema check`: PASS.
- `project-ledger specs verify`: PASS; v0.1 `772ea517`, v0.2 `31cb7cd4`.
- `project-ledger contracts verify`: PASS; 46 schemas, 92 fixtures y 53 filas de matriz.
- Contratos G-Skills, GSP, Radar y Agent Skill: PASS; la skill verificó 2 schemas, 7 triggers y 5 políticas.
- `integration compatibility`: PASS; cuatro perfiles locales completos, certificación global y por perfil falsa.
- `cargo deny --offline check`: PASS para advisories, bans, licenses y sources; sólo permanecen duplicidades transitivas informativas de `getrandom`, `r-efi` y `syn`.
- `git diff --check`: PASS.
- Los hashes finales coinciden con el lock: registry `8b543f80…`; fixture manifest `af685beb…`.

**Estado y límites de evidencia**

- Fase 10 / S13 cerrada; avance real: 11/12 macrofases y 14/15 slices, según §50/§51.
- La skill permanece procedimental: `state_authority=false`, `core_dependency=false`, `certified_compatibility=false`.
- S13 no demuestra obediencia universal de agentes/modelos, enforcement fuera del CLI, sandbox universal, integración remota, operación productiva ni readiness de release.
- El cierre se versiona mediante el commit de fase `feat: add governed Agent Skill workflow`.

**Siguiente paso según la especificación**

Iniciar, sólo tras una nueva autorización, Fase 11 / S14 hardening, benchmark y release: seguridad/privacidad, fuzzing, chaos, performance, packaging, SBOM, migraciones y compatibility report. Su gate exige AC-001…090 PASS o waiver no crítico explícito; S14 no se inició en este cierre.

### 2026-08-23 — Apertura de Fase 11 / S14: hardening, benchmark y release

**Autoridad y alcance reconciliados**

- Se releyeron el handoff y las secciones normativas §41–49, §50–51 y §56 antes de editar implementación.
- S14 agregará un gate de release reproducible, no una nueva autoridad ni una integración adicional. Reutilizará las pruebas reales de fases anteriores y añadirá TDD sólo para conducta de seguridad/release aún no cubierta.
- El corpus de privacidad será declarado y fijado por hash; el resultado cero se limitará a esas superficies y patrones soportados, tal como exige §42.4.
- El benchmark publicará mediciones p95 con entorno exacto. Los sistemas/counterparts no ejecutados se representarán como waivers no críticos o condiciones pendientes, nunca como PASS ni certificación implícita.
- El plan persistente `docs/superpowers/plans/2026-08-23-project-ledger-phase-11-s14.md` divide el trabajo en contrato de hardening, evidencia/migración, SBOM/packaging/benchmark, CLI/CI/documentación y gate final.

**Estado de apertura**

- Base Git limpia confirmada en `66b26fb` (`feat: add governed Agent Skill workflow`).
- El avance permanece en 11/12 macrofases y 14/15 slices hasta que S14 supere su gate integral y se versione.
- Primer paso TDD: escribir y observar el RED de AC-081/082/083/085 antes de crear las APIs de release.

**Hito S14.1 — Política y escáner de privacidad RED→GREEN**

- RED confirmado: el nuevo test de hardening no pudo importar ninguna de las APIs requeridas.
- `ReleasePolicy` deriva capacidades sólo desde trust state explícito. `UNTRUSTED`, `LOCAL_USER`, `TRUSTED_PROJECT` y `REVOKED` no obtienen adapter, write, authority, network, credentials ni hooks; recording/projection tampoco adquieren autoridad, red, credenciales o hooks.
- `ExactProcessPlan` conserva executable, cwd y argv literal; no existe command line interpolada ni rama de shell.
- El escáner clasifica secretos, transcripts, private reasoning y raw output sin incluir valores sensibles en el finding. Valores nulos/deshabilitados o `[REDACTED]` permanecen limpios.
- Los paths del corpus deben ser relativos y normalizados; absolute, parent traversal, `.` y vacío fallan cerrados.
- GREEN focalizado: 6/6 pruebas PASS.

**Siguiente acción**

Escribir el RED del gate agregado: inventarios AC-001…090/PL-01…32 exactos, evidencia existente, migración reversible explícita, waivers tipados, compatibility sin overclaim y drift de hashes.

**Hito S14.2 — Gate agregado, migración y threat model RED→GREEN**

- RED confirmado: faltaban `verify_release`, el modelo de error y el resultado agregado.
- El reporte exige exactamente AC-001…090 y PL-01…32 sin duplicados; cada grupo referencia evidencia existente y PASS no puede esconder waiver. Missing evidence y BLOCKED permanecen contables.
- AC-086 queda `WAIVED`: la matrix Windows/Linux/macOS está configurada, pero este checkout local no contiene evidencia de una ejecución hosted Linux/macOS. El waiver tiene owner, razón, scope, revisión y bloquea GA.
- La migración v0.1→v0.2 es `REVIEWED_IMPORT_INTO_NEW_STORE`: no in-place, preserva source, no reinterpreta silenciosamente y revierte descartando el destino nuevo.
- Siete surfaces de privacidad quedan fijados por hash tanto en el corpus como en el release manifest. La mutación de un byte falla por `ArtifactDigestMismatch` antes de aceptar semántica.
- El compatibility report conserva cuatro perfiles `COMPLETE` locales, todos `certified_compatibility=false` y con evidencia externa pendiente.
- Se publicó `docs/THREAT_MODEL.md`, el chaos report y la distinción NO-GO/waiver no crítico.
- GREEN focalizado: 11/11 tests del crate release PASS.

**Hito S14.3 — Benchmark, package, SBOM y checksums**

- RED de comandos confirmado: `xtask` sólo aceptaba `schema`; `benchmark` y `release` fallaron por usage.
- La primera medición real rechazó el reporte: 7/8 targets pasaron, pero se había medido `projection request warm` como rebuild de 10.000 eventos y el p95 fue 3240,112 ms contra 200 ms.
- Se corrigió la metodología, no el límite: §44 reserva 10.000 eventos para render. Projection warm declara 100 eventos y midió p95 37,610 ms; render conserva el stream canónico de 10.000 y midió 0,070 ms.
- Medición final Windows x86_64, Rust 1.97.1, release, 20 muestras: cold CLI 41,355 ms; resume 0,098 ms; append SQLite/WAL 4,318 ms; gate 0,026 ms; render 10k 0,070 ms; projection warm 37,610 ms; idle 0 % sin daemon; network calls default 0. Todos esos targets pasan.
- Peak-RSS p95 no se inventó: queda waiver no crítico AC-089 por falta de instrumentación portable sin dependencias unsafe/runtime-specific y bloquea GA. El acceptance report contabiliza así dos waivers totales, AC-086 y AC-089.
- `cargo xtask release generate|check` materializa/verifica package manifest single-binary, SBOM SPDX 2.3 con 200 paquetes, manifest raw-locked y checksums de 13 artefactos, incluido el antecedente v0.1 con `772ea517…`.
- GREEN focalizado: 2/2 tests de comandos xtask PASS; aggregate release check informa 90 AC, 32 PL y 0 secretos.

**Hito S14.4 — CLI/CI y documentación GREEN previo al gate**

- RED CLI confirmado: `release` era un subcomando desconocido.
- `project-ledger release verify --root` devuelve JSON canónico read-only con conteos AC/PL, secretos, waivers, migración y certificación global falsa.
- El primer intento GREEN detectó drift de `Cargo.lock` porque la nueva dependencia CLI se añadió después del SBOM; se regeneró desde el grafo final sin relajar el gate y la prueba pasó.
- CI ahora construye el binario release y ejecuta los gates xtask/CLI en Windows, Linux y macOS. Esta configuración no se registra como ejecución hosted ya realizada.
- README, changelog, SECURITY, threat model y matriz contractual documentan entorno, límites, rollback, waivers y condición de engineering release candidate.

**Siguiente acción**

Ejecutar formato y Clippy estricto; corregir cualquier hallazgo; regenerar artefactos sólo si cambian bytes inventariados; después ejecutar el gate integral completo desde cero y cerrar S14 con un único commit si toda la evidencia fresca pasa.

### 2026-08-23 — Cierre de Fase 11 / S14: hardening y engineering release candidate

**Resultado implementado**

- `project-ledger-release` agrega políticas auditables de trust y proceso exacto, escáner de privacidad sin eco de valores sensibles y verificación fail-closed de paths, symlinks, hashes, aceptación, migración, benchmark, package, SBOM, checksums y compatibilidad.
- El conjunto de release publica 90 AC y 32 PL exactos, siete superficies de privacidad raw-locked con `secret_persistence_count=0`, threat model, chaos report y migración v0.1→v0.2 revisada/no destructiva/reversible.
- Packaging declara un único binario default sin daemon/red; el SBOM SPDX 2.3 inventaría 200 paquetes y 13 checksums fijan Cargo.lock, benchmark, threat model, reportes, manifest, SBOM, schema registry y antecedente v0.1.
- El runner p95 usa un binario release, transacción SQLite/WAL, PredicateAST y streams canónicos. Ocho targets medidos pasan; no se fabricó peak-RSS.
- La CLI `release verify` expone JSON canónico read-only. `xtask benchmark run|check` y `release generate|check` permiten regeneración y drift check reproducibles.
- CI incorpora build locked release y gate de release/CLI en Windows, Linux y macOS; la configuración no se presenta como una ejecución hosted ya observada.

**Gate integral final fresco**

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: PASS; 221 atributos `#[test]` inventariados, cero fallos.
- `cargo build --locked --release -p project-ledger-cli`: PASS.
- `cargo xtask schema check`: PASS.
- `project-ledger specs verify`: PASS; v0.1 `772ea517`, v0.2 `31cb7cd4`.
- `project-ledger contracts verify`: PASS; 46 schemas, 92 fixtures y 53 filas de matriz.
- G-Skills: 11 schemas/22 fixtures PASS; GSP: registry 28, 31 schemas/140 fixtures PASS; Radar: 5 schemas/10 fixtures PASS; Agent Skill: 2 schemas, 7 triggers, 5 policies PASS.
- `integration compatibility`: PASS local; cuatro perfiles `COMPLETE`, certificación global y por perfil falsa.
- `cargo xtask benchmark check`: PASS; ocho métricas medidas y un waiver explícito.
- `cargo xtask release generate --check`, `cargo xtask release check` y `project-ledger release verify`: PASS; 90 AC, 32 PL, 0 secretos, 0 blockers, 0 evidencia ausente, 2 waivers, 0 certificaciones.
- `cargo deny --offline check`: PASS para advisories, bans, licenses y sources; duplicidades transitivas informativas conocidas de `getrandom`, `r-efi` y `syn`.
- Replay de los 13 SHA-256: PASS. `git diff --check`: PASS.

**Estado y límites de evidencia**

- Fase 11 / S14 cerrada localmente; avance de implementación: 12/12 macrofases y 15/15 slices, según §50/§51.
- El artefacto es `ENGINEERING_RELEASE_CANDIDATE`, no General Availability. AC-086 y AC-089 están `WAIVED` de forma no crítica: falta evidencia hosted Linux/macOS y medición peak-RSS; ambos waivers bloquean GA.
- Los cuatro perfiles permanecen `certified_compatibility=false`; faltan counterpart/bilateral/live-sidecar/external-agent runtime según cada reporte.
- El cierre no prueba operación productiva, sandbox universal, obediencia universal de agentes, rentabilidad ni validez científica.
- El cierre se versiona mediante el commit de fase `feat: harden and prepare v0.2.0 release`.

**Siguiente paso según la especificación**

No queda otro slice de implementación. El siguiente paso es la calificación de release: obtener ejecuciones hosted Linux/macOS, incorporar medición portable de peak-RSS y cerrar ambos waivers; después aportar evidencia counterpart/bilateral/live para cualquier perfil que se quiera certificar, sin cambiar `certified_compatibility=false` antes de esa evidencia.

### 2026-08-26 — Apertura de calificación de release / Gate 1 de 2: peak-RSS

**Reconciliación de entrada**

- Git está limpio en `cd99750` (`feat: harden and prepare v0.2.0 release`); la implementación permanece cerrada en 12/12 macrofases y 15/15 slices.
- El checkout no tiene ningún remote Git configurado. `gh` está autenticado, pero no puede resolver un repositorio ni ejecuciones hosted; por ello AC-086 continúa `WAIVED` y bloquea General Availability.
- AC-089 continúa `WAIVED`: el reporte actual contiene ocho métricas medidas y un waiver para `memory_one_shot_p95`.
- La autoridad §44 fija `memory one-shot p95 < 100 MiB` y aclara que los targets son presupuestos de ingeniería, no garantías universales.

**Decisión de implementación**

- El plan persistente `docs/superpowers/plans/2026-08-26-project-ledger-release-qualification.md` separa dos gates de calificación: peak-RSS portable y ejecución hosted multiplataforma.
- El Gate 1 medirá externamente el RSS del binario release real, con 20 ejecuciones one-shot y muestreo por PID cada 1 ms mediante `sysinfo` 0.39.6 sin features por defecto. La dependencia quedará confinada a `xtask`; no se añadirá red, daemon, shell ni `unsafe` propio al producto.
- El resultado se describirá honestamente como máximo RSS muestreado por ejecución y p95 del conjunto, ligado al entorno declarado. No se presentará como pico kernel exacto ni garantía universal.
- El Gate 2 no puede cerrarse en este estado porque no existe remote ni identificador de ejecución. No se creará ni seleccionará un repositorio remoto por inferencia.

**Estado de apertura**

- Avance de implementación: 12/12 macrofases y 15/15 slices, sin cambio.
- Avance de calificación GA: 0/2 waivers resueltos.
- Estado de release: `ENGINEERING_RELEASE_CANDIDATE`; `certified_compatibility=false` en los cuatro perfiles.

**Siguiente acción**

Aplicar TDD: cambiar la expectativa ejecutable de AC-089 a nueve métricas sin waivers, demostrar el RED contra el reporte vigente y sólo entonces implementar el muestreador externo.

**Hito de calificación RQ1.1 — Peak-RSS RED→GREEN**

- RED confirmado: 2/2 pruebas AC-089 fallaron porque `benchmark check` todavía aceptaba ocho métricas y el waiver `memory_one_shot_p95`.
- El checker ahora exige inventario exacto de nueve métricas, `measurement_status=COMPLETE`, 20 muestras por métrica, cero waivers, métodos declarados y peak-RSS positivo/menor que 100 MiB.
- `sysinfo` 0.39.6 se limita a `xtask`, con features por defecto deshabilitadas y sólo `system` habilitada. El binario `project-ledger` y su perfil offline/single-binary no adquieren esa dependencia.
- La primera sonda sobre `--version` falló cerrada con cero lecturas: el proceso duraba aproximadamente 24–73 ms y Windows no exponía RSS antes de su salida. No se aceptó cero como resultado.
- Se seleccionó la operación one-shot real `project-ledger contracts verify`, que valida 46 schemas/92 fixtures. El muestreador externo observa sólo ese PID cada 1 ms, conserva el máximo RSS de cada ejecución y calcula p95 sobre 20 muestras.
- Medición material Windows x86_64, Rust 1.97.1, release, fechada 2026-08-26: `memory_one_shot_p95=17,855 MiB`, presupuesto `<100 MiB`, PASS. El reporte completo contiene nueve métricas PASS y cero waivers.
- GREEN focalizado: 2/2 pruebas AC-089 PASS; el fixture legacy con waiver es rechazado por el comando real.
- `release/acceptance-report.json` promueve exclusivamente AC-089 a PASS. AC-086 continúa `WAIVED`; el estado sigue `ENGINEERING_RELEASE_CANDIDATE` y los cuatro perfiles conservan `certified_compatibility=false`.

**Estado previo al gate integral**

- Avance de implementación: 12/12 macrofases y 15/15 slices.
- Avance de calificación GA: 1/2 waivers resueltos.
- Falta regenerar SBOM/manifest/checksums desde el grafo y evidencia finales, y ejecutar el gate integral fresco antes del commit de calificación.

**Siguiente acción**

Regenerar la cadena de release, ejecutar el gate integral completo y versionar RQ1 sólo si no aparece drift, fallo de política o regresión.

### 2026-08-26 — Cierre de calificación RQ1 / Gate 1 de 2: peak-RSS

**Resultado materializado**

- El runner externo mide el proceso release real `project-ledger contracts verify` sin shell, con argv literal, muestreo RSS por PID cada 1 ms y 20 máximos one-shot.
- `memory_one_shot_p95=17,855 MiB` frente al presupuesto `<100 MiB` en Windows x86_64, Rust 1.97.1, release, campaña fechada 2026-08-26.
- El benchmark contiene nueve métricas PASS, `measurement_status=COMPLETE` y cero waivers. El fixture anterior de ocho métricas con waiver es rechazado.
- AC-089 cambia de `WAIVED` a `PASS`. AC-086 es el único waiver no crítico restante y sigue bloqueando General Availability.
- `sysinfo` 0.39.6 se incorpora sólo a `xtask`, con default features deshabilitadas y feature `system`; el árbol locked de `project-ledger-cli` no contiene `sysinfo`.
- SBOM SPDX 2.3 regenerado con 217 paquetes; release manifest y 13 checksums se regeneraron desde el lock y los artefactos finales.

**Gate integral final fresco**

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: PASS; 222 atributos `#[test]` inventariados, cero fallos.
- `cargo build --locked --release -p project-ledger-cli`: PASS.
- `cargo xtask schema check`: PASS.
- `project-ledger specs verify`: PASS; v0.1 `772ea517`, v0.2 `31cb7cd4`.
- `project-ledger contracts verify`: PASS; 46 schemas, 92 fixtures y 53 filas de matriz.
- G-Skills: 11 schemas/22 fixtures PASS; GSP: registry 28, 31 schemas/140 fixtures PASS; Radar: 5 schemas/10 fixtures PASS; Agent Skill: 2 schemas, 7 triggers y 5 policies PASS.
- `integration compatibility`: PASS local; cuatro perfiles `COMPLETE`, certificación global y por perfil falsa.
- `cargo xtask benchmark check`: PASS; nueve métricas y cero waivers de benchmark.
- `cargo xtask release generate --check`, `cargo xtask release check` y `project-ledger release verify`: PASS; 90 AC, 32 PL, 0 secretos, 0 blockers, 0 evidencia ausente, 1 waiver y 0 certificaciones.
- `cargo deny --offline check`: PASS para advisories, bans, licenses y sources; duplicidades transitivas informativas conocidas de `getrandom`, `r-efi` y `syn`.
- Replay de 13 SHA-256: PASS. `project-ledger-cli` excluye `sysinfo`. `git diff --check`: PASS.

**Estado y límites de evidencia**

- Avance de implementación: 12/12 macrofases y 15/15 slices, sin cambio.
- Avance de calificación GA: 1/2 waivers resueltos.
- Estado: `ENGINEERING_RELEASE_CANDIDATE`, no General Availability.
- Los cuatro perfiles continúan `certified_compatibility=false`; esta calificación no aporta counterpart, bilateral, live-sidecar ni runtime externo.
- El cierre se versiona mediante el commit de calificación `perf: qualify peak RSS budget`.

**Siguiente paso según el plan**

Iniciar RQ2: configurar o identificar un remote autorizado, ejecutar la matriz GitHub Actions en Windows/Linux/macOS para el commit exacto de calificación y conservar IDs/URLs de jobs. AC-086 sólo podrá pasar si los tres entornos concluyen satisfactoriamente sobre el mismo SHA; hasta entonces el release no es GA.

### 2026-08-26 — Apertura de calificación RQ2 / Gate 2 de 2: CI hosted multiplataforma

**Reconciliación de remote**

- Git inicia limpio en `24e490d` (`perf: qualify peak RSS budget`) y la rama local es `main`.
- `git remote -v` no devuelve ningún remote configurado.
- GitHub CLI está autenticada como `gcatalansc`, con permisos `repo` y `workflow`.
- La cuenta posee 14 repositorios, pero ninguno contiene `ledger` en el nombre.
- Las consultas exactas a `gcatalansc/G-Ledger` y `gcatalansc/project-ledger` fallan porque ambos repositorios son inexistentes o inaccesibles.
- No se observaron organizaciones asociadas ni un repositorio público/accesible exacto que pueda adoptarse sin ambigüedad.

**Bloqueo de autoridad**

- RQ2 exige un remote GitHub inequívoco para publicar el SHA y ejecutar `.github/workflows/ci.yml` en `windows-latest`, `ubuntu-latest` y `macos-latest`.
- Crear un repositorio nuevo requiere una decisión humana sobre nombre/owner y visibilidad pública o privada. No se inventa esa decisión ni se reutiliza uno de los 14 repositorios existentes.
- No se configuró remote, no se realizó push y no se inició ninguna ejecución hosted.
- AC-086 permanece `WAIVED`; el release continúa `ENGINEERING_RELEASE_CANDIDATE`, no GA, con avance de calificación 1/2.

**Entrada humana requerida para continuar**

Autorizar la creación de `gcatalansc/G-Ledger` indicando visibilidad pública o privada, o proporcionar la URL exacta de un repositorio GitHub existente que deba usarse como `origin`. Tras esa decisión, verificar historial/destino, hacer sólo push fast-forward, observar la matriz del SHA publicado y conservar URLs/IDs de jobs antes de modificar AC-086.

**Autorización y materialización del remote**

- El usuario autorizó explícitamente crear el repositorio con visibilidad privada.
- Se creó `gcatalansc/G-Ledger` como `PRIVATE` el 2026-08-27T01:34:38Z: `https://github.com/gcatalansc/G-Ledger`.
- `origin` apunta por HTTPS a `https://github.com/gcatalansc/G-Ledger.git` para fetch y push.
- `defaultBranchRef` y `git ls-remote --heads origin` confirman que el repositorio remoto está vacío; no existe historia que sobrescribir ni reconciliar.
- La creación no incluyó push. Antes de publicar `main` se ejecutará un gate local fresco sobre `24e490d`.

**Primera ejecución hosted y diagnóstico sistemático**

- El gate local fresco sobre `24e490d2ef044a20115ed5db4bb472d569bcaf3c` pasó formato, Clippy estricto, suite workspace, build release locked, reproducción de artefactos, verificación agregada, política de dependencias y replay 13/13 de checksums.
- `main` se publicó por primera vez mediante fast-forward y `origin/main` quedó exactamente en ese SHA. GitHub creó la ejecución `33030810589`: `https://github.com/gcatalansc/G-Ledger/actions/runs/33030810589`.
- La ejecución no es evidencia de aceptación: `dependency-policy` pasó, pero Windows y Ubuntu fallaron en Clippy, y macOS falló durante la suite workspace.
- Causa 1 verificada: aunque la acción instala Rust 1.97.1, `rust-toolchain.toml` declara `stable` y prevalece al ejecutar Cargo; los runners usaron Rust/Clippy 1.98.0. El lint nuevo `chunks_exact_to_as_chunks` detectó `crates/canonical-json/src/lib.rs:65`. El fallo se reprodujo localmente con `cargo +1.98.0 clippy`.
- Causa 2 verificada: el test de release ejecuta `cargo metadata --locked --offline`; en el runner macOS no estaba descargado `anstyle-wincon`, una dependencia target-specific de Windows, y el comando falló cerrado. En un `CARGO_HOME` nuevo se comprobó que `cargo fetch --locked` descarga dependencias de todos los targets y permite inmediatamente el metadata offline.

**Corrección mínima RED→GREEN previa al reintento**

- El parser SHA-256 usa `as_chunks::<2>()`, compatible con el MSRV 1.97.1 y con el lint estricto de Rust 1.98.0; no cambia su contrato ni sus bytes de salida.
- El job de matriz fija `RUSTUP_TOOLCHAIN=1.97.1`, manteniendo `rust-toolchain.toml` en `stable` para no reactivar el conflicto local de instalación nominal ya documentado, y ejecuta `cargo fetch --locked` antes de los gates offline.
- GREEN focalizado: Clippy del crate canónico pasa tanto con Rust 1.98.0 como con el workspace Rust 1.97.1; 5/5 pruebas canónicas y 3/3 pruebas de release `xtask` pasan.
- AC-086 continúa `WAIVED`. La ejecución fallida se conserva como evidencia de diagnóstico, no como prueba de conformidad.

**Siguiente acción**

Ejecutar el gate integral local sobre la remediación, versionarla en un commit propio y publicar ese SHA para obtener una nueva matriz hosted completa.

**Cierre local de la remediación RQ2**

- Gate integral PASS: formato, `cargo fetch --locked`, Clippy estricto, suite workspace completa, schemas deterministas, specs v0.1/v0.2, 46 schemas/92 fixtures/53 filas, cuatro verificadores de integración, build release locked, nueve métricas sin waivers de benchmark, artefactos reproducibles y verificación release CLI.
- Resultado agregado sin cambio de aceptación anticipado: 90 AC, 32 PL, 0 blockers, 0 evidencia ausente, 0 secretos, 1 waiver (AC-086) y 0 certificaciones.
- `cargo deny --offline check` pasa advisories, bans, licenses y sources; replay SHA-256 13/13 y `git diff --check` pasan.
- `actionlint` no está instalado localmente. La aceptación sintáctica y conductual del workflow se comprobará mediante la ejecución GitHub posterior al push del commit de remediación.
- La remediación se versionará con el commit `fix: harden hosted cross-platform gate`; AC-086 no cambia en este commit.

**Siguiente acción**

Publicar el commit de remediación, identificar la nueva ejecución por SHA exacto y exigir PASS de Windows, Ubuntu, macOS y dependency-policy antes de editar la evidencia de aceptación.

**Ejecución hosted aceptada para AC-086**

- El commit de remediación es `5d34294c1f06b59e635c84d734b3156950940ae5` (`fix: harden hosted cross-platform gate`) y `origin/main` quedó exactamente en ese SHA.
- Run agregado: `33031982562`, `success`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33031982562`.
- Windows: job `98386337267`, `success`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33031982562/job/98386337267`.
- Ubuntu: job `98386337436`, `success`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33031982562/job/98386337436`.
- macOS: job `98386337365`, `success`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33031982562/job/98386337365`.
- Dependency policy: job `98386337495`, `success`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33031982562/job/98386337495`.
- Los cuatro jobs comparten el mismo `headSha`; no hubo jobs missing, skipped ni neutral. La ejecución fallida anterior no se reutiliza como evidencia positiva.

**TDD de cierre evidencial**

- RED confirmado: la prueba del crate release y la prueba CLI exigieron `waived_count=0` y ambas observaron el valor vigente `1` antes de modificar el reporte.
- `release/acceptance-report.json` promueve exclusivamente AC-086 de `WAIVED` a `PASS`, elimina su waiver y conserva el run, SHA y cuatro IDs de jobs en el locator hash-locked.
- Los cuatro perfiles de integración permanecen `certified_compatibility=false`; la evidencia hosted no demuestra contraparte, bilateralidad ni runtime live.
- La calificación RQ1/RQ2 alcanza 2/2 waivers de aceptación resueltos, pero el release permanece `ENGINEERING_RELEASE_CANDIDATE`: `release/chaos-report.json` todavía declara el escenario `physical-disk-full` como waiver que bloquea `GENERAL_AVAILABILITY`.

**Siguiente acción**

Regenerar manifest/checksums, demostrar GREEN de las pruebas de cero waivers y ejecutar el gate integral antes de versionar el cierre de RQ2.

### 2026-08-26 — Cierre de calificación RQ2 / Gate 2 de 2: CI hosted multiplataforma

**Resultado materializado**

- AC-086 es `PASS` con evidencia del run `33031982562` sobre el SHA exacto `5d34294c1f06b59e635c84d734b3156950940ae5`; Windows, Ubuntu, macOS y dependency-policy concluyeron `success`.
- La evidencia conserva run URL, SHA y los cuatro IDs de jobs dentro de `release/acceptance-report.json`; el release manifest fija su digest y `release/checksums.sha256` fija tanto el acceptance report como el manifest.
- TDD RED→GREEN: las dos pruebas de release observaron primero `waived_count=1` y después pasan con `waived_count=0`.
- El reporte agregado queda en 90 AC, 32 PL, 0 blockers, 0 evidencia ausente, 0 secretos, 0 waivers de aceptación y 0 certificaciones.

**Gate integral final fresco**

- Formato, fetch locked, Clippy estricto, suite workspace, schemas, specs, contratos, cuatro integraciones y build release: PASS.
- Benchmark: nueve métricas medidas, cero waivers; release generate/check y CLI release verify: PASS y `waived_count=0`.
- `cargo deny --offline check`: PASS para advisories, bans, licenses y sources; las duplicidades transitivas conocidas siguen siendo informativas.
- Replay de 13 checksums y `git diff --check`: PASS.
- Los cuatro perfiles continúan `COMPLETE` localmente y `certified_compatibility=false`; no se añadió evidencia counterpart, bilateral ni live.

**Estado real**

- Avance de implementación: 12/12 macrofases y 15/15 slices.
- Avance de calificación AC-086/AC-089: 2/2 waivers resueltos.
- Estado de release: `ENGINEERING_RELEASE_CANDIDATE`, no General Availability.
- Bloqueo GA restante: `release/chaos-report.json` mantiene `physical-disk-full` como waiver no crítico que bloquea `GENERAL_AVAILABILITY`. El waiver `live-sidecar-process-crash` bloquea específicamente la certificación runtime de GSP.
- El cierre se versionará mediante el commit `chore: qualify hosted cross-platform release`.

**Siguiente paso según la especificación**

Diseñar y ejecutar una prueba destructiva controlada de `physical-disk-full` en un entorno efímero, verificando fallo cerrado, rollback/recovery SQLite y preservación de la cadena antes de retirar ese waiver. Después, aportar evidencia counterpart/bilateral/live para cualquier perfil que se pretenda certificar, sin modificar `certified_compatibility=false` antes de ello.

### 2026-08-26 — Apertura de calificación RQ3 / Bloqueo GA de caos: physical disk-full

**Autoridad y estado reconciliados**

- Git inicia limpio en `9cea832ca7f68032c6bb18c25f800e578fe6d6ac`; `HEAD` y `origin/main` coinciden, y el run hosted `33033255000` terminó `success` sobre ese SHA.
- La especificación normativa §45.7 exige el escenario `disk full`; Fase 11/S14 exige hardening de caos con PASS o waiver no crítico explícito.
- `release/chaos-report.json` conserva dos waivers: `physical-disk-full` bloquea `GENERAL_AVAILABILITY` y `live-sidecar-process-crash` bloquea sólo `GSP_RUNTIME_CERTIFICATION`.
- El verifier de release fija el digest del chaos report, pero todavía no interpreta su inventario ni sus scopes de bloqueo; ese vacío debe fallar cerrado antes de retirar evidencia.
- Docker no está disponible localmente. La prueba física se ejecutará en un `tmpfs` Linux efímero de GitHub Actions; el equipo local nunca será llenado.

**Plan persistente y límites**

- Plan: `docs/superpowers/plans/2026-08-26-project-ledger-physical-disk-full-qualification.md`.
- TDD separa la prueba determinista multiplataforma mediante `PRAGMA max_page_count` de la evidencia física hosted mediante kernel `ENOSPC`/OS error 28.
- El harness físico exigirá marker explícito, rechazará roots inseguros/symlinks, escribirá sólo en un hijo fijo y tendrá un límite absoluto de 64 MiB.
- No se modificará `certified_compatibility=false`, ni se presentará el fault de cuota local como evidencia física, ni se promoverá el `release_status` sin un procedimiento separado y autorizado.

**Estado de apertura**

- Avance de implementación: 12/12 macrofases y 15/15 slices.
- Avance de calificación AC-086/AC-089: 2/2.
- Avance de calificación del bloqueo GA `physical-disk-full`: 0/1.
- Estado de release: `ENGINEERING_RELEASE_CANDIDATE`; cuatro perfiles sin certificación.

**Siguiente acción**

Escribir expectativas RED que obliguen al verifier y al CLI a exponer dos waivers de caos, un bloqueo GA y un bloqueo GSP runtime antes de implementar el parser semántico fail-closed.

**Hito RQ3.1 — Contrato de caos y harness local/hosted**

- RED de release confirmado: la prueba de biblioteca no compiló porque `ReleaseVerification` carecía de los tres contadores de caos; la prueba CLI ejecutó el binario real y falló porque el JSON no contenía esos campos.
- El verifier ahora exige el inventario exacto de 12 identificadores materializados que cubren los escenarios §45.7, evidencia local existente/no symlink para cada escenario cubierto, waivers no críticos explícitos y scopes no conflados.
- Mutaciones hash-locked de escenario desconocido y evidencia cubierta inexistente son rechazadas por el verifier real.
- GREEN vigente: 2 waivers de caos, 1 bloqueo `GENERAL_AVAILABILITY` y 1 bloqueo `GSP_RUNTIME_CERTIFICATION`; estos contadores son independientes de los 0 waivers de aceptación.
- La primera inyección determinista fijó `max_page_count` desde una conexión auxiliar, pero el store reabierto confirmó el append. El resultado se trató como RED de diseño: el límite no estaba instalado en el pager que ejecutaba la transacción.
- La inyección se movió al mismo connection/pager dentro de una prueba unitaria privada del crate, sin añadir API productiva exclusiva de test. Allí el append sobredimensionado devuelve `SQLITE_FULL`; tras restaurar el límite, tanto la conexión vigente como la reapertura reportan `quick_check=ok`, WAL, cadena válida y exactamente versión/evento/comando/resultado `1`.
- El harness físico es un test ignorado: exige variable y marker exactos, rechaza workspace/ancestros/descendientes y symlinks, crea un hijo fijo, limita el filler a 64 MiB y limpia sólo cinco nombres conocidos.
- El workflow incorpora un job Ubuntu dedicado que monta un `tmpfs` de 24 MiB con `nosuid,nodev,noexec`, ejecuta sólo el test físico exacto y desmonta en `always()`.
- Pruebas focalizadas, compilación del test físico ignorado y Clippy estricto del crate SQLite: PASS. El ENOSPC físico aún no se ha ejecutado; el waiver permanece intacto.

**Estado previo al gate del commit de harness**

- Implementación: 12/12 macrofases; 15/15 slices.
- Calificación AC-086/AC-089: 2/2.
- Calificación del bloqueo GA `physical-disk-full`: 0/1.
- Siguiente acción: ejecutar el gate integral local, versionar el harness si queda verde y publicar ese SHA para obtener evidencia física hosted antes de tocar `release/chaos-report.json`.

**Refinamiento del entorno antes de publicación**

- El job físico se ajustó de `tmpfs` a ext4 sobre una imagen loopback de tamaño fijo de 32 MiB. Así el fault ocurre en un filesystem de bloque Linux real, manteniendo el mismo aislamiento efímero y sin llenar el filesystem del host.
- El job registra `findmnt` para probar tipo, dispositivo y capacidad, desmonta siempre el mountpoint exacto y elimina sólo la imagen creada por ese job.
- Clippy señaló `format_collect` en un helper SHA-256 de tests; se corrigió usando el patrón `fold`/`write!` ya empleado por el verifier. El gate integral se reinició y Clippy estricto pasó.

**Gate integral del harness RQ3.1 — PASS**

- Un RED adicional demostró que el chaos report podía retirarse del inventario sin ser rechazado por ese motivo. El verifier ahora exige explícitamente su entrada hash-locked; la prueba de remoción es GREEN.
- La recuperación compara también el digest del génesis capturado antes del fault y exige ausencia del resultado del comando fallido. El filler físico sincroniza cada bloque escrito para observar la asignación real de ext4.
- Formato, fetch locked, Clippy estricto, suite workspace/all-targets/all-features y build release locked: PASS. Hay 229 atributos `#[test]` inventariados; el escenario físico es el único nuevo test ignorado localmente y no se contabiliza como evidencia ejecutada.
- Schemas, specs preservadas v0.1/v0.2, contratos (46/92/53), G-Skills (11/22), GSP (28 registry/31 schemas/140 fixtures), Radar (5/10), Agent Skill (2 schemas/7 triggers/5 policies) e integración compatibility: PASS.
- Benchmark (9 métricas, 0 waivers), release reproducible, release check y CLI release verify: PASS. El resultado conserva 90 AC, 32 PL, 0 secretos, 0 blockers de aceptación, 0 evidencia ausente y 0 waivers de aceptación; caos conserva 2 waivers, 1 bloqueo GA y 1 bloqueo GSP runtime.
- `cargo deny --offline check`: PASS; duplicidades transitivas conocidas informativas. Replay SHA-256 13/13 y `git diff --check`: PASS.
- El remote se revalidó como `https://github.com/gcatalansc/G-Ledger.git`, repositorio `PRIVATE`. No hay cambios ajenos en el worktree.
- El harness se versionará como `test: add bounded disk-full chaos gate`; después se publicará ese SHA exacto. `physical-disk-full` sigue waived hasta observar la ejecución física hosted.

**Publicación del harness y espera evidencial**

- Commit de preparación: `7464e2e3ff17fa3c5bdc58b198ae8b01f8ab4ae8` (`test: add bounded disk-full chaos gate`). Push fast-forward confirmado; `origin/main` coincide con ese SHA.
- Run hosted iniciado: `33035586838`, `https://github.com/gcatalansc/G-Ledger/actions/runs/33035586838`, con `headSha` exacto.
- Job físico: `98397460929`; Windows: `98397460944`; Ubuntu: `98397460955`; macOS: `98397460930`; dependency-policy: `98397460762`.
- La ejecución está en curso. No hay todavía conclusión física positiva ni cambio del waiver; el siguiente paso es inspeccionar su log y los resultados de la matriz.

**Evidencia física observada y RED de cierre**

- El job `98397460929` concluyó `success` en 55 s, incluyendo el desmontaje y cleanup; el SHA fuente es `7464e2e3ff17fa3c5bdc58b198ae8b01f8ab4ae8`.
- Log físico leído mediante el endpoint de logs del job: a las `2026-08-27T03:11:25Z`, `findmnt` mostró `/dev/loop0`, `ext4`, tamaño utilizable `26M` y `25.3M` disponibles sobre la imagen fija de 32 MiB.
- A las `2026-08-27T03:11:54Z`, el filler sincronizado recibió `ENOSPC(28)` después de 26.214.400 bytes. El append real del store falló cerrado con `database or disk is full`.
- Rollback en el mismo proceso: versión/evento/comando/resultado `1`, cadena válida, digest previo preservado y resultado del comando fallido ausente.
- Reapertura: `quick_check=ok`, WAL y cadena versión/conteo `1/1`; el test físico terminó `1 passed; 0 failed; 0 ignored`.
- La evidencia es kernel ENOSPC de Linux/ext4 en loopback efímero, no un mock, no una cuota SQLite y no una certificación de dispositivos/OS universales.
- RED de cierre confirmado antes de editar el chaos report: biblioteca y CLI devuelven exit 101 porque observan 2 waivers de caos frente a la expectativa nueva de 1; el JSON conserva todavía 1 bloqueo GA.
- La matriz general hosted continúa en curso. Se mantiene el reporte sin cambios hasta que el run agregado termine satisfactoriamente.

**Aceptación hosted completa y actualización evidencial**

- Run `33035586838`: `completed/success`, SHA `7464e2e3ff17fa3c5bdc58b198ae8b01f8ab4ae8`.
- Los cinco jobs concluyeron `success`: dependency-policy `98397460762`, físico `98397460929`, macOS `98397460930`, Windows `98397460944` y Ubuntu `98397460955`; no hubo jobs missing/skipped/neutral.
- El job Windows terminó en 22m15s; la demora fue de compilación/gates, no un fallo de aceptación.
- Entorno físico confirmado en log: Ubuntu 24.04.4 LTS, imagen `ubuntu-24.04` versión `20260823.283.1`, Rust 1.97.1, ext4/loopback con imagen de 33.554.432 bytes y filler ENOSPC observado de 26.214.400 bytes.
- Sólo después de ese cierre se movió `physical-disk-full` de `waived` a `covered`, con test path, `ENOSPC(28)` y locator que conserva job/run/SHA/entorno/capacidad.
- `live-sidecar-process-crash` permanece sin cambios y bloquea únicamente `GSP_RUNTIME_CERTIFICATION`. `release_status` y todos los flags de certificación permanecen intactos.
- README y matriz contractual fueron reconciliados: ya no describen AC-086 como waived y distinguen calificación de caos, promoción GA y certificación externa.
- Siguiente acción: regenerar manifest/checksums, demostrar GREEN de los dos consumidores y ejecutar el gate integral de cierre.

**GREEN evidencial RQ3**

- `cargo xtask release generate --root .` regeneró la cadena de artefactos; sólo cambian los digests ligados al chaos report y al manifest.
- Biblioteca release: 9/9 tests PASS, incluyendo rechazo de drift, escenario desconocido, evidencia ausente y remoción del hash lock.
- CLI release: PASS. Resultado observado: 90 AC, 32 PL, 0 waivers de aceptación, 1 waiver de caos, 0 bloqueos GA de caos, 1 bloqueo GSP runtime, 0 secretos y 0 certificaciones.
- El avance evidencial del escenario físico alcanza 1/1; falta el gate integral y el commit de cierre antes de entregar la fase.

### 2026-08-26 — Cierre local de calificación RQ3 / physical disk-full

**Gate integral final fresco — PASS**

- Formato, `cargo fetch --locked`, Clippy workspace/all-targets/all-features con warnings como errores y build release locked: PASS.
- Suite workspace completa: 228 tests PASS, 0 FAIL y 1 test físico ignorado intencionadamente en el equipo local. Ese escenario fue ejecutado realmente en el job hosted `98397460929`, no se contabiliza el skip como evidencia física.
- Schemas, specs preservadas, contratos, los cuatro verificadores de integración, compatibility, benchmark, generación reproducible, release check y release verify: PASS.
- `cargo deny --offline check`: advisories, bans, licenses y sources PASS; duplicidades transitivas conocidas informativas.
- Replay SHA-256: 13/13 PASS. `git diff --check`: PASS. El cierre sólo modifica evidencia, expectativas y documentación; no cambia el motor transaccional ni introduce dependencias.
- Resultado de release: 90 AC, 32 PL, 0 blockers de aceptación, 0 evidencia ausente, 0 waivers de aceptación, 1 waiver de caos, 0 bloqueos GA de caos, 1 bloqueo GSP runtime, 0 secretos y 0 certificaciones.

**Estado y publicación de cierre**

- Avance de implementación: 12/12 macrofases y 15/15 slices.
- Avance de calificación AC-086/AC-089: 2/2.
- Avance evidencial `physical-disk-full`: 1/1, sustentado en el run `33035586838` y SHA exacto `7464e2e3ff17fa3c5bdc58b198ae8b01f8ab4ae8`.
- Estado de release: `ENGINEERING_RELEASE_CANDIDATE`. Cero bloqueos GA de caos no constituye una promoción automática a General Availability; los cuatro perfiles conservan `certified_compatibility=false`.
- Este snapshot queda preparado para el commit `chore: qualify physical disk-full recovery`. La entrega requiere publicar el commit y observar los cinco jobs del workflow final en `success` sobre ese SHA exacto; ese resultado futuro no se da por aprobado en esta bitácora.
- La comprobación post-commit será de sólo lectura y se incluirá en el mensaje de entrega, conservando aquí la evidencia fuente inmutable ya observada.

**Siguiente paso según la especificación**

Calificar `live-sidecar-process-crash` frente a una contraparte GSP identificada por versión/SHA y con evidencia bilateral/runtime live. Conservar el waiver y `certified_compatibility=false` hasta observar crash, recuperación y reconciliación reales; no iniciar esa fase dentro del cierre de RQ3.

### 2026-08-27 — Apertura RQ4 / live-sidecar-process-crash

**Estado reconciliado y autoridad**

- `HEAD=44a0260db3ca3d4af0a7da54278336843f28dc63`, árbol limpio; CI `33037163775` reconsultada en `completed/success` sobre ese SHA.
- Verificación fresca: 28 contratos GSP, 31 schemas y 140 fixtures MATCH; release conserva 90 AC, 32 PL, 0 waivers de aceptación, 1 waiver de caos, 0 bloqueos GA de caos, 1 bloqueo GSP runtime y 0 certificaciones.
- La especificación §35.1–35.7 permite `compile_bundle`, artifact exchange, corpus minimizado y sidecar opcional sin shared DB; §45.7 exige crash del sidecar.
- La contraparte fijada existe en Git: GSP v0.2.0 / `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, contratos `004b14d7b64837025671970eac2469f092c49475`. El repositorio upstream es público. El checkout activo GSP está limpio en `ddf2cfb32b53de0eb0967a57862ff11fdfa65af1` (línea posterior); no se cambiará ni usará silenciosamente en lugar del pin.
- La API real valida y compila seis artefactos sin llamar modelos ni red. Se usará en un checkout aislado con dependencias frozen, no mediante un resultado fixture ni un error simulado in-process.
- Se leyeron las instrucciones operativas y el workflow GSP del commit fijado. Sus referencias a `docs/01_SPEC_MVP.md` y `docs/02_IMPLEMENTATION_PLAN_CODEX_QWEN.md` ya no existen en ese commit; no se reconstruyen ni se reabre la implementación upstream. Su API y contratos exactos son la autoridad de esta integración.

**Plan y límites**

- Plan persistente: `docs/superpowers/plans/2026-08-27-project-ledger-live-gsp-chaos-qualification.md`; ejecución inline con TDD y verificación previa al cierre.
- Adapter Python opcional por artifact exchange, no dependencia del core; comandos/paths explícitos, entrada/salida acotadas y salida create-new.
- Fault real: terminar y reap sólo el proceso propio después de entrar al compilador; comprobar cadena/proyección intactas, reiniciar, validar resultado y enlazar/reconciliar explícitamente en SQLite desechable.
- La sincronización del fault queda en utilidades de test; no se modifica el código ni los contratos GSP, ni se exponen flags destructivos en la API productiva.
- Se mantienen el waiver y todas las certificaciones falsas hasta evidencia; esta calificación de caos no equivale a validez semántica/científica, runtime universal ni promoción GA.

**Avance**

- Implementación: 12/12 macrofases, 15/15 slices. AC-086/089: 2/2. Disk-full: 1/1.
- RQ4: 0/3 hitos — ejecución/crash local real, calificación hosted y cierre evidencial.
- Siguiente acción: provisionar el pin aislado y escribir el test live RED antes del adapter.

**RQ4.1 — Ejecución local real y TDD**

- Se provisionó `.tooling/rq4-gsp-runtime` desde el tag público v0.2.0 y se comprobó el commit exacto `7a43559c...`. `uv 0.11.16 sync --frozen` instaló un entorno separado: Python 3.12.13, GSP 0.2.0 y 27 paquetes fijados por el lock; no cambió el entorno GSP activo ni Python global.
- RED de compilación: el CLI exportó un corpus real desde SQLite desechable y el proceso falló por ausencia del adapter. GREEN posterior: el API GSP real produjo un resultado aceptado por Ledger con seis artefactos y delta de eventos 0.
- Diagnóstico del primer adapter: el manifiesto tiene 31 archivos JSON y un README bajo schemas. Se preservó la verificación de bytes de todos y se corrigió únicamente el conteo JSON, sin tocar upstream.
- RED de crash: el test rechazó la salida prematura del proceso porque aún no existía el helper de sincronización. GREEN posterior: el proceso propio PID 11824 entró a `gspgemma.compiler.pipeline.compile_evidence`, recibió terminación OS, fue reaped y no publicó resultado; cadena y proyección permanecieron idénticas con `quick_check=ok`, WAL y 7 eventos.
- Reinicio en proceso nuevo: seis artefactos exactos aceptados, delta 0 antes del enlace. `link-result` explícito: delta 1; replay misma clave: delta 0; lifecycle/gates/phases sin elevación y `certified=false`.
- Casos de rechazo reales: checkout incorrecto, JSON inválido, entrada mayor a 1 MiB y salida existente; no publican resultado válido ni mutan Ledger. La salida existente se conserva byte a byte.
- El primer fixture de la prueba release carecía de evidencia colateral. Se completaron sus referencias y checksums hasta verificar primero el fixture intacto; sólo entonces el RED probó que el verifier aceptaba una cobertura live sin evidencia. El nuevo guard exige fault observado, contraparte exacta y locator hosted. No se retira todavía el waiver.
- Resultado focalizado: 3/3 tests live realmente ejecutados, 10/10 tests release; Ruff PASS. Las pruebas live permanecen opt-in/ignoradas en el golden path sin Python.
- RQ4 alcanza 1/3 hitos. Siguiente acción: gate integral local y publicación del harness para calificación hosted sobre SHA exacto.

**Gates de contraparte y endurecimiento evidencial**

- Se provisionó además el paquete contractual completo en `.tooling/rq4-gsp-spec`, SHA `004b14d7...`, con digest del manifiesto `c6ca0f6d...`. Los comandos reales GSP `verify-upstream-hashes`, `validate-contracts` y `fixture-replay` pasan. La selección inicial de tests usó el nombre inexistente `test_api.py`; tras localizar el archivo vigente `test_public_api.py`, los 18 tests de API/compilador/contratos pasan. Ambas copias upstream permanecen limpias.
- Se reforzó el fingerprint para invocar `verify_event_chain` explícitamente. Las pruebas de rechazo verifican códigos estructurados sin eco de datos e incluyen claves JSON duplicadas.
- RED adicional: un locator con `head_sha=unverified` era aceptado. GREEN: ahora exige repositorio/run/job exactos en la forma del locator, IDs numéricos y SHA hexadecimal de 40 caracteres, además del fault y counterpart commit fijados.
- Suite release focalizada: 11/11 PASS; suite live real: 3/3 PASS; Clippy estricto y Ruff: PASS. El job hosted incluirá los mismos gates de contraparte antes del fault, con acciones setup-python/setup-uv fijadas por commit y uv 0.11.16.
- Siguiente acción: ejecutar el gate integral fresco; el waiver oficial aún no cambia.

**Gate integral del harness RQ4 — PASS**

- Formato/fetch locked/Clippy estricto/workspace completo/build release locked: PASS; 230 tests aprobados, 0 fallos y 4 tests opt-in ignorados en el golden path (3 GSP live, 1 ENOSPC).
- Schemas, specs v0.1/v0.2, contratos 46/92/53, cuatro integraciones, benchmark de 9 métricas, artefactos reproducibles, release check y CLI release verify: PASS.
- `cargo deny --offline check`: advisories/bans/licenses/sources PASS; duplicidades transitivas conocidas informativas. Replay de checksums 13/13 y diff check: PASS.
- GSP live separado: 3/3 PASS; counterpart upstream: hashes/contratos/fixtures y 18 tests PASS. El `uv.lock` de contraparte tiene SHA-256 `e999e6ad6856e25b76a630312df396e6e74841333b91e0dadb3e507b48555620`.
- El checkout activo GSP sigue limpio en `ddf2cfb32b53de0eb0967a57862ff11fdfa65af1`; las copias aisladas también siguen limpias. No se modificaron Cargo.lock, dependencias Rust, autoridad congelada ni flags de certificación.
- El harness queda preparado para el commit `test: qualify live GSP process crash isolation`. RQ4 permanece en 1/3 hasta observar la ejecución hosted sobre ese SHA; se conserva `live-sidecar-process-crash` como waiver oficial.
- Siguiente acción: publicar el commit y exigir éxito de los seis jobs, incluyendo contraparte real, antes de editar evidencia de release.

**Publicación y corrección de sintaxis CI**

- Harness publicado en `bdd482553806ebbd0139b916b663c4a9908c9483`; `HEAD` y `origin/main` coincidieron con árbol limpio.
- El run `33060429046` terminó `failure` sin jobs: el parser YAML rechazó el `run` no entrecomillado que contiene `live_sidecar::` (línea 152, columna 78). No se reutiliza ese run como evidencia física/runtime.
- Se reprodujo con PyYAML y, antes de editar el workflow, con una regresión Rust que parsea el archivo publicado y comprueba que cada `run` es una cadena ejecutable. RED observado por el error de scanner, no por una comparación textual.
- Corrección mínima: entrecomillar el comando YAML conservando exactamente su argv. Se ejecutará el gate nuevamente antes de publicar `fix: quote live sidecar CI command`.
- El waiver sigue vigente y RQ4 permanece 1/3; no hubo crash hosted ni cambio de calificación.
- La regresión y PyYAML ya pasan con el comando entrecomillado. También se resolvió el tag anotado `setup-uv@v7` hasta su commit real `37802adc94f370d6bfd71619e3f0bf239e1f3b78`, en lugar de fijar el objeto tag `94527f2e...`; `setup-python` ya estaba fijado a un commit.
- Gate de corrección PASS: regresión CLI, parseo YAML, formato, Clippy estricto, suite workspace completa, release check, generación reproducible y diff check. La corrección sólo cambia el workflow, su test consumidor y esta bitácora; los artefactos oficiales conservan el waiver.

**Fuente hosted corregida y RED de cierre**

- Corrección publicada en `30c17ac0253a581db69b7fe38e001da07c9216d8`, push fast-forward y árbol limpio en ese momento.
- Run `33060749387`, URL `https://github.com/gcatalansc/G-Ledger/actions/runs/33060749387`, aceptado por GitHub y en curso sobre ese SHA. Jobs: GSP live `98478615297`, macOS `98478615538`, Windows `98478615568`, Ubuntu `98478615585`, ENOSPC `98478615614`, dependencias `98478615619`.
- El setup de GSP y sus gates upstream ya pasaron; el ensayo live hosted y la matriz general siguen en curso. No se retira todavía el waiver.
- RED evidencial de cierre: biblioteca y CLI se cambiaron primero para exigir cero waivers de caos/cero bloqueos de caos GSP; ambos tests fallaron con exit 101 porque el reporte oficial conserva `chaos_waived_count=1` y `gsp_runtime_certification_blocker_count=1`.
- Punto de continuación: esperar éxito agregado de los seis jobs, leer log GSP, registrar contraparte/entorno/terminación/reinicio/link/replay y sólo entonces mover el escenario a covered, regenerar artefactos y demostrar GREEN.

**Evidencia hosted GSP observada — matriz agregada aún en curso**

- Job `98478615297` concluyó `success` en el run `33060749387`, SHA exacto `30c17ac0253a581db69b7fe38e001da07c9216d8`; todos sus pasos, incluidos los post/cleanup, terminaron success.
- Ventana: `2026-08-27T09:55:26Z`–`09:57:28Z`. Entorno observado: imagen `ubuntu-24.04`, versión `20260816.277.1`, Python 3.12.14, uv 0.11.16 y toolchain Rust 1.97.1 fijado por workflow. La diferencia Python respecto de Windows local (3.12.13) queda explícita.
- Contraparte confirmada en el log: `7a43559c02417641691dbb5f2a6e79c13d74d1a1`; hashes/contratos/fixtures PASS, 18 tests upstream PASS.
- `09:57:17Z`: el proceso propio PID 5115 entró al `compile_evidence` real; terminó por intervención OS, fue reaped, no dejó resultado y conservó exactamente cadena/proyección, delta de eventos 0.
- `09:57:18Z`: proceso nuevo recompiló y Ledger aceptó los seis artefactos; delta 0 antes del enlace. Enlace explícito delta 1, replay delta 0, elevación de autoridad 0 y certificación false. Suite live: 3 PASS, 0 FAIL, 0 IGNORE.
- Fuente inmutable: `https://github.com/gcatalansc/G-Ledger/actions/runs/33060749387/job/98478615297`. No se presenta un provider mock ni un resultado fixture como runtime; tampoco se afirma calidad semántica/científica del contenido.
- La matriz Windows/macOS/Ubuntu aún debe concluir; el chaos report oficial conserva su waiver hasta éxito agregado.

**RQ4.2 — Calificación hosted completa y cierre evidencial en curso**

- El run `33060749387` concluyó `completed/success` sobre `30c17ac0253a581db69b7fe38e001da07c9216d8`. Los seis jobs, incluidos Windows/macOS/Ubuntu, dependencias, ENOSPC y GSP live, terminaron success. La evidencia agregada se reconsultó antes de modificar los reportes.
- RQ4 alcanza 2/3. Sólo ahora se mueve `live-sidecar-process-crash` a covered con fault `PROCESS_TERMINATED_DURING_COMPILE`, contraparte exacta y locator run/job/head SHA/entorno. El chaos report pasa a `PASS` sin waivers; la evidencia física ENOSPC anterior se conserva.
- Se concilian los metadatos propios de Ledger en compatibility report y upstream lock: el E2E acotado por artifact exchange/crash está probado, pero siguen pendientes conformidad runtime completa y revisión de certificación. No se cambian pins/digests upstream, reporte histórico S11, booleanos de certificación ni estado `ENGINEERING_RELEASE_CANDIDATE`.
- README, compatibilidad GSP, changelog, matriz y plan delimitan los resultados reales sobre SQLite desechable frente a operación productiva, daemon, validez científica y compatibilidad universal. Python sigue siendo opt-in fuera del golden path Rust.
- Siguiente acción: regenerar los artefactos, demostrar GREEN del RED de cierre y ejecutar el gate integral fresco antes del commit final y su CI exact-SHA.

**RQ4.3 — Gate integral fresco y snapshot de cierre preparado**

- GREEN del cierre: 11/11 tests de biblioteca release y 2/2 de CLI; se regeneraron manifest/checksums con `cargo xtask release generate --root .`. Los consumidores ahora confirman cero waivers de caos y cero bloqueos de caos GA/GSP, sin habilitar certificación.
- Repetición GSP real después del cambio de metadatos: 3/3 PASS. PID propio 31276 terminó por intervención OS durante la compilación, fue reaped y dejó cadena/proyección idénticas; reinicio válido, seis artefactos, enlace delta 1 y replay delta 0. No se cuenta el skip de la suite estándar como ejecución live.
- Gate final completo: formato, fetch locked, Clippy workspace/all-targets/all-features con warnings como errores, suite completa y build release locked PASS. Conteo fresco agregado: 231 PASS, 0 FAIL, 4 IGNORE opt-in (3 GSP y 1 ENOSPC).
- Schemas, specs preservadas, contratos, cuatro integraciones, compatibility, benchmark, generación reproducible, release check y release verify PASS. `cargo deny --offline check`: advisories/bans/licenses/sources PASS; duplicidades transitivas conocidas informativas. Replay SHA-256: 13/13 PASS; diff check PASS.
- Resultado de release: 90 AC, 32 PL, 0 blockers de aceptación, 0 evidencia ausente, 0 waivers de aceptación, 0 waivers de caos, 0 bloqueos GA de caos, 0 bloqueos GSP de caos, 0 secretos en superficies declaradas y 0 certificaciones. `ENGINEERING_RELEASE_CANDIDATE` se conserva.
- Los pins/digests upstream, reporte histórico S11, contratos congelados, Cargo.lock y dependencias Rust permanecen intactos. El checkout GSP activo sigue limpio en `ddf2cfb32b53de0eb0967a57862ff11fdfa65af1`; las dos copias de tooling también están limpias.
- Se prepara `chore: qualify live GSP crash recovery`. El snapshot conserva la fuente inmutable ya aprobada (`33060749387` / `98478615297` / `30c17ac0253a581db69b7fe38e001da07c9216d8`); la entrega exige publicar este cierre, observar los seis jobs finales en success y comprobar HEAD=origin/main=SHA hosted con árbol limpio. No se da por aprobado aquí un resultado futuro.
- Avance de implementación: 12/12 macrofases y 15/15 slices. AC-086/089: 2/2. Disk-full: 1/1. RQ4 permanece 2/3 hasta esa comprobación post-commit de sólo lectura, que se comunicará en la entrega.

**Siguiente paso según plan y especificación**

Revisar la evidencia bilateral/runtime restante y delimitar el alcance de conformidad GSP más allá del ensayo de crash antes de plantear su certificación (§35, AC-090). Mantener todos los `certified_compatibility=false`; no modificar otro repositorio ni promover GA automáticamente.

## 2026-08-27 — Inicio del cierre pre-piloto autorizado (0/8)

**Solicitud y acuerdos**

- El usuario autoriza implementar el plan de ocho etapas, antes de usar Ledger con un proyecto nuevo o actual real. Windows primero; Ledger + Codex + GSP exacto; G-Skills/Radar deshabilitados para el piloto y conservados como regresión; contratos v0.2 sin ampliación.
- Se aplican planificación, diagnóstico causal, TDD y verificación fresca. Ejecución inline; las subskills de ejecución referidas por la skill de planes no están disponibles. No se crean agentes, proyectos de usuario, instalaciones globales ni cambios upstream.
- Git ya estaba inicializado y limpio en `dea208d294dfddc282b84dcb5a58b9d8d9887b91`. Se conserva el repo privado y se hará commit por etapa. El contador nuevo X/8 no sustituye la historia, pero el antiguo 12/12/15/15 no acredita CLI/paquete operativos completos.

**Etapa 1 — baseline y conciliación**

- Persistido `docs/superpowers/plans/2026-08-27-project-ledger-prepilot-readiness.md`, con los ocho cierres, interfaces, pruebas y límites. `docs/PREPILOT_GAP_MATRIX.md` asigna 14 brechas y cubre AC-001…090 / PL-01…32 distinguiendo biblioteca, CLI, runtime, fixtures, paquete y hosted.
- `release/prepilot-readiness.json` declara `NOT_READY`, 0/8, ninguna prueba real de usuario y cero certificación. El reporte de aceptación histórico no se borra ni se convierte en prueba de preparación operacional.
- Consulta GitHub fresca: run `33062468586`, SHA `dea208d294dfddc282b84dcb5a58b9d8d9887b91`, completed/success y seis jobs success. RQ4 se concilia a 3/3 para crash/recovery; los anteriores 2/3 quedan como checkpoints históricos pre-commit. README y plan RQ4 aclaran el alcance.
- Probes CLI no mutantes reproducidos: `record progress --help` devuelve 2 (comando ausente); resume con DB inexistente devuelve 1 (esperado 4), y no crea el archivo. `integration gsp verify-contracts` desde Temp falla porque busca recursos bajo cwd (exit 1).
- Hallazgos estáticos separados de los probes: siete comandos auxiliares sin handler/reducer, `Command::output()` sin timeout/captura acotada/entorno permitido, sin backup online ni apertura de lectura separada. No se afirma haber reproducido agotamiento de memoria ni pérdida de datos.
- Hashes v0.1/v0.2 coinciden; `specs verify` y `contracts verify` PASS (46 schemas / 92 fixtures / 53 filas); reporte parseado: 90 AC, 32 PL, ocho etapas y 14 brechas iniciales. `git diff --check` PASS. Gate Rust completo en curso, aún no es evidencia de cierre.
- Siguiente acción: terminar la verificación de etapa 1, cerrarla con commit, luego completar el núcleo con pruebas RED de comandos auxiliares.

**Etapa 1 cerrada — 1/8, NOT_READY**

- Formato, Clippy workspace/all-targets/all-features con warnings como errores y suite workspace completa concluyeron con exit 0. Los tres tests GSP opt-in y el ENOSPC físico permanecen ignored en esa suite; no se cuentan como nuevos resultados runtime/físicos.
- Se contrastó el inventario completo de 90 AC / 32 PL con la matriz y se verificaron hashes de autoridad y contratos. PP-01 queda CLOSED; 13 brechas siguen OPEN. No se modificaron contratos, dependencia, runtime o semántica productiva en esta etapa.
- Commit de etapa: `docs: reconcile prepilot baseline and eight-stage gap matrix`. Las pruebas nuevas de etapa 2 se preparan por separado y no forman parte de este commit.
- Siguiente paso: RED/GREEN de los comandos auxiliares del núcleo, incluidos replay, referencias, autorización y reconstrucción.

**Etapa 2 en curso — checkpoint de implementación**

- Etapa 1 quedó en commit `e51ec81`. Las nuevas pruebas de aplicación fallaron primero 6/7 por comandos válidos enviados al motor equivocado; se implementaron las siete variantes existentes y sus índices/reductores auxiliares sin modificar schemas.
- GREEN inicial: 7/7. Notas/artefactos, sesiones con y sin checkpoint, mappings y corrección descriptiva son transacciones reales y replayables. Correcciones restringidas a `text` de nota y `work_record.summary`, con originales intactos y sin autoridad semántica.
- RED adicional observado: un actor `UNTRUSTED` podía editar metadata por `CoreCommandHandler`. Guard de recording explícito agregado; 8/8 pruebas focales PASS. Se amplía la prueba para distinguir `POLICY_DENIED` y evitar que `TRUSTED_FOR_RECORDING` permita lifecycle/authority.
- Se prepara `command_coverage.rs`: recorrido real de los 53 comandos con aceptación, replay, rechazo de versión obsoleta y rebuild después de cada operación. Las fixtures aportan entradas, no resultados simulados ni prueba runtime externa. Etapa 2 sigue abierta hasta superar esa cobertura y regresión completa.
- No se habilitan CLI nueva, runtime o piloto; estado global permanece 1/8, NOT_READY. Cambios de etapa 2 todavía no comprometidos.

**Etapa 2 — segundo checkpoint de continuidad**

- Se reprodujeron rechazos no tipados y ausencia de `resulting_checkpoint_ref`; se corrigieron en el processor sin ampliar `ErrorCode` ni los contratos. La política central diferencia recording de autoridad y devuelve `POLICY_DENIED` sin eventos.
- La prueba de sesiones reprodujo cierre anterior al inicio y ahora compara instantes RFC3339 (incluidos offsets diferentes), no strings. El reloj del resultado no reemplaza `submitted_at` del evento; se corrigió primero ese detalle del test. Los 10 tests auxiliares pasaron después.
- El recorrido observó los 53 comandos y 66 eventos ejecutables, con replay, versiones obsoletas y rebuild. La cobertura ampliada agrega rechazo `UNTRUSTED` a cada comando. `LEGACY_IMPORT_REVIEWED` está prohibido en todas las filas v0.2: se prueba su rechazo transaccional, no se inventa un comando de emisión.
- Dos omisiones detectadas por esa cobertura: decisiones sin revalidación tras perder su autoridad y ausencia de `PHASE_MARKED_READY_TO_CLOSE`. Ahora una solicitud con recording trust sólo deja la fase pendiente de aprobación; completar requiere humano local y todos los predicados. Autoridad no vigente invalida decisiones dependientes, sin afectar documentos ajenos ni reaceptar automáticamente al recuperar un digest.
- Se reprodujo que una decisión en revalidación no tenía ruta de reaceptación. Se permite `ACCEPT_DECISION` explícito por humano local con autoridad vigente, preservando los eventos anteriores.
- El projector interno pasa a versión 5 para no reutilizar caches construidas con semántica anterior; wire/schema permanece 0.2.0. Se reutiliza `time` ya bloqueado en el workspace para comparación temporal.
- Una ejecución concurrente de tests en Windows produjo LNK1104 al intentar reemplazar el mismo ejecutable todavía activo. Se identificó el proceso propio y se dejó finalizar; no se eliminó ni mató un proceso ajeno. La repetición se hace secuencialmente. No es evidencia RED de comportamiento.
- Pendientes antes del commit: regresión workspace/Clippy/build/spec/contratos, cobertura de corrección de work summary, consolidar evidencia del núcleo. Etapas 3–8 todavía pendientes; estado 1/8 y `NOT_READY`.

**Etapa 2 — verificación ampliada y metadatos**

- `command_coverage` pasó con 53 comandos, 66 eventos, replay, versiones obsoletas y rechazo UNTRUSTED por comando. `command_transaction`: 6/6, incluida la prohibición real de LEGACY_IMPORT_REVIEWED. Auxiliares: 11/11, incluido work summary corregido con historia intacta y checkpoint invalidado.
- RED/GREEN adicional: observar una autoridad SUPERSEDED la convertía en UNAVAILABLE, permitiendo que un MATCH posterior la reactivase. Las observaciones ahora preservan SUPERSEDED; el test recorre UNAVAILABLE → MATCH → MISMATCH → MATCH sin devolver autoridad.
- Se reprodujo y corrigió el reconocimiento de la URI plural `project-ledger://projects/<uuid>` que genera el catálogo. Se conserva la forma singular ya admitida y se rechazan identidades ajenas.
- Dependencias (`cargo deny --offline check`): advisories/bans/licenses/sources OK; duplicidades transitivas informativas. Hashes v0.1/v0.2 MATCH; contratos 46/92/53 MATCH. No nuevos pins ni versiones de dependencias externas.
- La primera suite workspace se detuvo en el test de release por hash anterior de Cargo.lock. Se regeneran SBOM/manifiesto/checksums con el generador existente; esto no eleva el estado del producto. Clippy estricto pasó antes de esa ejecución; se repite junto con la suite/build/checks después de las últimas correcciones.
- Verificación final en curso (sesión de comandos local); no commit de etapa 2 todavía. Próximo punto: revisar salida final, actualizar informe/matriz y crear commit sólo si todos los controles pasan.

## 2026-08-27 — Reanudación solicitada: continuar desde el checkpoint

- El usuario pide retomar profesionalmente el plan. HEAD sigue en `e51ec81` (etapa 1); los cambios de etapa 2 y sus metadatos generados están presentes y se preservan. No se reinicia Git ni se rehace la etapa cerrada.
- La sesión de verificación interrumpida ya no permite recuperar una salida final. No se infiere PASS: se repetirá el gate fresco antes del commit. Estado confirmado: 1/8, NOT_READY; implementación de etapa 2 pendiente de cierre verificado.
- Se reutiliza el plan persistente con las skills de planificación, TDD, diagnóstico y verificación. Ejecución inline; no hay autorización nueva sobre proyectos reales ni cambios upstream.

**Etapa 2 cerrada — 2/8, NOT_READY**

- Se repitió la verificación completa tras la interrupción. Un test previo exigía error para toda solicitud de cierre por agente; se reprodujo el fallo y se contrastó con la nueva ruta contractual de preparación. Ahora comprueba READY_TO_CLOSE con aprobación pendiente, ausencia de PHASE_COMPLETED y cierre posterior mediante solicitud humana. No se eliminó la comprobación de autoridad ni se habilitó cierre autónomo.
- Gate local completo con exit 0: formato, Clippy workspace/all-targets/all-features con warnings como errores, tests workspace/all-targets/all-features, build workspace locked, schema check, specs verify y contracts verify. Contratos: 46 schemas / 92 fixtures / 53 filas; hashes de specs preservados.
- `cargo xtask release check` pasó: 90 AC / 32 PL del inventario histórico, 0 secretos; valida el reporte histórico de 9 métricas y 0 waivers, no ejecuta un benchmark nuevo. `cargo deny --offline check`: advisories/bans/licenses/sources OK, con duplicidades transitivas conocidas. SHA-256 13/13 y diff check correctos.
- Se añadieron 15 tests: 11 auxiliares, 3 recorridos/invariantes del núcleo y 1 rechazo transaccional de legacy no emitible. Se observaron los 53 comandos y 66 eventos ejecutables con replay, rechazo UNTRUSTED, conflicto de versión e igualdad incremental/rebuild. El evento restante está prohibido por las 53 filas v0.2; no se simula su implementación.
- Las cuatro pruebas opt-in (tres GSP real y una ENOSPC físico) permanecieron ignored. No hubo nueva CI hosted, ejecución de runtime GSP, prueba de ZIP ni intervención de un proyecto real. Los contratos, schemas, integraciones fijadas y workflow CI no cambiaron.
- Plan, matriz y `release/prepilot-readiness.json` conciliados a 2/8. PP-02 cerrado; PP-03…PP-14 abiertos. La cobertura y sus límites quedan en `docs/PREPILOT_CORE_COVERAGE.md`.
- Commit de cierre preparado: `feat: complete prepilot core command and event behavior`.

**Siguiente paso según el plan**

Ejecutar la etapa 3: CLI cotidiana, errores tipados y códigos normativos, selección explícita de proyecto, consultas sin crear/migrar bases y reconciliación por clave de idempotencia. Mantener `NOT_READY` hasta verificar las ocho etapas; no usar todavía un proyecto real.

## 2026-08-27 — Autorización para continuar hasta el cierre del plan

- El usuario solicita avanzar hasta terminar. Se retoma desde `d024f2a`, árbol limpio, 2/8 verificado. Se ejecutarán las etapas 3–8 en orden con pruebas y commit por cierre, sin pedir una autorización redundante entre etapas.
- Se preservan alcance, contratos y proyectos reales fuera de intervención. `NOT_READY` se mantiene hasta demostrar todos los criterios, incluida CI exact-SHA y ZIP extraído. Una limitación externa real se informará sin simular un cierre.
- Skills aplicadas: planificación para el seguimiento existente, TDD/diagnóstico para reproducciones y correcciones, verificación antes de cada cierre. Ejecución inline.

**Etapa 3 — checkpoint de CLI (continúa abierta, 2/8)**

- RED observado: cuatro pruebas de procesos fallaron por ausencia de record/pause/status y código 1 en lugar de 4. Tras implementar selección explícita, política recording/humano, reconciliación scoped y apertura SQLite sin migraciones, las cuatro pasan.
- `open_read_only` y `open_existing` validan la versión y digests de migraciones; no crean ni migran DB. Reconciliación consulta el scope completo y valida comando/resultado; una clave no encontrada devuelve OUTCOME_UNKNOWN (12), no invita a duplicar.
- Se amplió la CLI por familias con payloads JSON/YAML y argumentos cotidianos. Ocho pruebas de procesos pasaron, incluidas 26 operaciones con replay y consultas, importación rebajada a OBSERVED, rechazo de gate con código 6, notas/progreso y flags cotidianos. Las fixtures son entradas, no prueba de compilador externo.
- Se identificaron y corrigieron dos errores de preparación de tests: el WorkRecord referenciaba un claim no creado y tenía timestamp diferente al envelope; Session también requería el timestamp real del comando. Se conservó el rechazo productivo y se corrigieron los inputs, no los invariantes.
- En curso: pruebas adicionales de acciones posicionales, enlace/desenlace, YAML, repositorio existente y refresh/supersede de autoridad. No commit ni cierre de etapa 3 todavía. Pendientes: completar interfaz/errores restantes, catálogo y regresión integral; después continuar etapas 4–8 sin nueva autorización.

**Etapa 3 — checkpoint ampliado antes de cierre**

- Pasaron 14 pruebas de procesos, más el caso nuevo de `evidence run` con política explícita, parser no cualificado y replay sin nueva ejecución. `docs/CLI_OPERATIONS.md` documenta los comandos reales, selección, scopes y límites. Se conserva `NOT_READY`, 2/8.
- RED/GREEN adicionales: colisión de command_id entre actores con la misma clave corregida incorporando actor/trust; importación YAML de Decision confundía su campo descriptivo `decision` con un payload wrapper, ahora distingue objetos; rutas locales de autoridad se convierten a file URI y el refresh no descarga ni ejecuta nada.
- Supersesión de autoridad/decisión exige que el reemplazo ya esté registrado/aceptado, según el núcleo. La CLI no añade silenciosamente otro comando. El test registra primero el reemplazo y conserva los archivos del repositorio existente intactos.
- Se unificaron las funciones CLI a CliError. Consultas heredadas GSP/Radar/provider abren read-only; sink y enlaces abren existing sin migrar. Contratos y wire schemas no cambiaron. Se regeneraron SBOM/manifiesto/checksums por las nuevas aristas de dependencias internas.
- Clippy estricto pasó. Suite workspace completa en curso, sesión local 91604; todavía no se cuenta como PASS. No lanzar otra compilación de ese ejecutable mientras termine: Windows produjo LNK1104 cuando dos ejecuciones previas coincidieron; no se mataron procesos ajenos.
- Se añadieron dos tests de revisión final todavía pendientes de ejecutar: auditoría de command_results manipulados y aislamiento de scopes entre actores en checkpoint histórico. Confirmar RED y corregir antes del commit si fallan. Después repetir formato/Clippy/tests/build/spec/contratos/release/dependencias/checksums y actualizar matriz/plan a 3/8 sólo cuando proceda.
- Investigación read-only para etapa 4: documentación primaria de process-wrap (sucesor de command-group), Job Objects Windows y grupos POSIX para limpiar descendientes propios. No se incorporó aún esa dependencia ni se implementaron timeout/cancelación/entorno acotado. La ejecución genérica actual sigue bajo las limitaciones pendientes de PP-07/08.

## 2026-08-27 — Etapa 3 verificada: CLI operativa, 3/8

- RED final confirmado: audit aceptaba resultados manipulados (0 en lugar de 15); checkpoints de dos actores con igual clave colisionaban en command_id (11). GREEN: auditoría utiliza recovery_report, IDs nuevos incluyen actor/trust y replay recupera el ID histórico. Pasan las 17 pruebas de procesos CLI.
- Gate fresco completo, sesión 16727, exit 0: formato, Clippy estricto, tests workspace/all-targets/all-features, build workspace locked, schema check (46 schemas, 92 fixtures, 53 filas), specs/contratos, release check y cargo deny offline. SHA-256 13/13 y git diff check correctos. Las duplicidades transitivas siguen siendo warnings permitidos; no hubo fallos.
- Las cuatro pruebas opt-in no se ejecutaron. No se atribuye a esta etapa prueba nueva de runtime GSP, ENOSPC físico, Codex completo, ZIP o CI hosted. Inventario release/benchmark histórico no equivale a readiness.
- Plan/matriz/informe conciliados a 3/8, NOT_READY. PP-03/04 cerrados; componentes CLI de PP-05/06 verificados, resto en sus etapas. Evidencia y límites: `docs/PREPILOT_CLI_COVERAGE.md`.
- Se prepararon aparte pruebas RED de entorno heredado/ruta relativa/importación sensible para etapa 4; no forman parte del commit de etapa 3 ni del PASS anterior.
- Commit de cierre: `feat: complete operational CLI and scoped reconciliation`.

**Siguiente paso:** ejecutar etapa 4 con procesos propios descartables, límites y cancelación, redacción y prevención durable de reejecución incierta; continuar luego etapas 5–8 según autorización vigente.

## 2026-08-27 — Etapa 4 en ejecución, checkpoint de continuidad (3/8)

- Confirmados RED: el runner heredaba una credencial ficticia del padre; aceptaba `rustc` por PATH; el importador admitía `token=...` en un resumen. Tras reemplazar la captura por supervisión, las pruebas de privacidad pasan. No se modifican comandos/eventos/schemas canónicos.
- `evidence::supervisor`: process-wrap 10.0.0 fijado, Job Objects Windows/grupos POSIX, argv literal, cwd/executable absolutos, env_clear y mapa explícito, 60 s / 1 MiB por flujo por defecto, captura concurrente acotada y limpieza con plazo de 2 s. Pruebas reales de hang, flood en ambos flujos, cancelación, hijos y salida normal del padre pasan en Windows.
- Runner: resumen 4 KiB configurable, UTF8 seguro, redacción configurada y de patrones soportados; no raw output persistido; parser genérico OBSERVED incluso con exit 0. Importación sensible se rechaza sin eco del valor. Se reutiliza el escáner de privacidad existente.
- Reserva durable previa a ejecutar: migración SQLite interna 3 (`execution_attempts`), transacción única por scope, sólo digests; no nuevo evento ni autoridad. Pruebas de reapertura, concurrencia, inmutabilidad y tampering pasan. Consultas siguen sin migrar. La operación explícita de migración con backup corresponde a etapa 5 y aún no está expuesta.
- CLI: política `process` explícita, Ctrl-C/cancel-file, códigos de timeout/cancel/overflow, resultado conocido no cero registrado pero exit 3; replay no ejecuta. El test integrado de timeout → no retry → conflicto de solicitud → evidencia fallida → gate no aprobado pasó; cancel-file real también pasó.
- Clippy estricto pasó antes del último caso de revisión. Suite evidencia/storage pasó; CLI completa en curso en sesión 29861. Pendiente un test adicional Windows: matar sólo al supervisor propio debe cerrar sus descendientes. Inspección del wrapper muestra Job Object sin kill-on-close; reproducir con el nuevo test `windows_closes_the_owned_job_when_the_supervisor_is_terminated`, corregir con API segura si falla y repetir gates. `win32job` sólo fue inspeccionado, no agregado aún.
- Estado: 3/8, NOT_READY. Etapa 4 no cerrada ni comprometida. Próximo: terminar revisión de limpieza, regenerar artefactos de dependencias, gate completo y commit de etapa 4; después continuar 5–8 sin pedir autorización redundante. GSP real/ENOSPC físico/ZIP/CI nueva siguen pendientes.

**Etapa 4 — revisión final antes del gate agregado**

- La CLI completa pasó 19 tests. Se reprodujo luego la fuga al matar al supervisor Windows: el heartbeat del descendiente cambió de 5 a 12 después de terminar al padre. Se añadió un job exterior no heredable con kill-on-close usando win32job 2.0.3; se asigna antes de la reanudación y confirma que su lista de procesos queda vacía. El caso de terminación abrupta y los otros cinco casos de supervisión pasan; también argv literal con metacaracteres.
- Se añadió y reprodujo un error de clasificación de privacidad: importación sensible devolvía 2 en vez de 13. Corregido a PRIVACY_REJECTED; el test confirma ausencia del canary en stdout/stderr y cero eventos nuevos.
- La generación offline de SBOM necesitó descargar las nuevas dependencias Unix de desarrollo que aún no estaban en caché. `cargo fetch --locked` completó cfg_aliases/nix; no se descargó runtime GSP ni componente durante uso del núcleo. SBOM/manifiesto/checksums se regeneraron correctamente después.
- Gate completo vigente: sesión 18066. Clippy estricto, test focal de privacidad y generación release pasaron; suite workspace/all-targets/all-features en compilación/ejecución. Quedan build locked, schemas/specs/contratos, release check, cargo deny online, checksums y diff. No contar como PASS antes del exit final. No hay otra compilación activa; la lectura de API backup de SQLite ya terminó.
- Evidencia y límites en `docs/PREPILOT_PROCESS_SAFETY.md`, todavía declara gate pendiente. Una vez completo: actualizar plan/matriz/readiness a 4/8 y commit; continuar etapa 5. Hay 20 tests normales en operations más un helper; los helpers ignored son ejecutados por sus padres y no equivalen a GSP/ENOSPC opt-in.

## 2026-08-27 — Etapa 4 verificada, 4/8

- Sesión 18066 finalizó exit 0: Clippy estricto, tests workspace/all-targets/all-features, build locked, schemas, specs/contratos, release check, cargo deny online, 13/13 SHA-256 y diff. Formato --check fresco también exit 0. Duplicidades transitivas permitidas, sin fallos.
- Plan/matriz/readiness y README conciliados. PP-07 cerrado; PP-08 mantiene pendiente la verificación de los canales de exportación y paquete. Los helpers ignored se ejecutaron como hijos reales; las cuatro pruebas opt-in GSP/ENOSPC no se ejecutaron de nuevo.
- Commit de cierre: `feat: supervise evidence processes and preserve uncertain executions`. No piloto ni certificación, y todavía no nueva CI hosted.

**Siguiente paso:** etapa 5, operaciones CLI de render/portable/importación histórica revisada, backup SQLite/WAL completo, restauración nueva y migración con respaldo previo; luego continuar 6–8.

## 2026-08-27 — Etapa 5 en ejecución, checkpoint (4/8)

- Commit etapa 4: `53f6f33`. No nuevas autorizaciones requeridas; sigue vigente completar 5–8.
- RED reproducidos: CLI sin backup/migrate/export/render/import-markdown; audit aceptaba DELETE de idempotency_keys; portable ignoraba directorio extra vacío; legacy sin límite de candidatos. Implementación mínima y regresión focal: cinco recorridos CLI pasan, metadata recovery pasa y suites portable/legacy pasan.
- Backup online con WAL, resultado/idempotencia/reservas; restore nuevo checksummed; migración transaccional con respaldo previo y exclusión de escritores. Consultas no migran. Metadata operacional se verifica adicionalmente al hash de eventos. Se conservan contratos v0.2.
- Render y sidecar derivado con verificación/re-render; portable new/identical/fast-forward, fork rechazado antes de escribir. Importación histórica exige decisiones individuales de revisor humano y digest del preview; sólo ASSERTED, provenance y contradicciones persistidas descriptivamente, sin convertir lifecycle/gates en hechos verificados. Reporta candidatos no materializados y resultados parciales recuperables por clave.
- Pendiente gate completo. Clippy detectó tres cuestiones de estilo en backup (doc SQLite, unidad Duration, buffer en stack), corregidas; reejecución vigente sesión 92302 ya terminó y el nuevo Clippy está corriendo. Tests adicionales privacidad/divergencia/WAL concreto por ejecutar. Detalles de límites en `docs/PREPILOT_PORTABILITY_RECOVERY.md`, todavía declara no cerrado.
- Próximo: terminar revisión y gates, actualizar a 5/8 sólo si pasan, commit; luego skill/Codex, matriz GSP y ZIP/CI fresca. No piloto, ninguna nueva ejecución GSP/ENOSPC, ningún push aún.

- Revisión de la prueba WAL: abrir una conexión sin leer no retenía una transacción, por lo que SQLite podía checkpointar al cerrar el escritor CLI. El nuevo assert sobre un WAL real expuso esa debilidad del test (archivo ausente). Se mantiene un snapshot lector anterior al append para impedir el checkpoint completo; la prueba deberá recuperar el commit que permanece en WAL. No se presenta la prueba anterior como evidencia de páginas WAL pendientes.

- La prueba WAL reforzada pasó. Revisión adicional con proceso CLI y SQLite descartable confirmó audit=0 tras sustituir sólo idempotency_keys.command_ref por una referencia inexistente: el lookup validaba el result pero no ese índice cruzado. Se añade regresión persistente y validación explícita del join completo. El gate 55727 verificaba la versión previa a esta última corrección; repetir regresión afectada y agregado antes de cerrar. Python se usó sólo como cliente SQLite de la prueba descartable, no como dependencia operativa del núcleo ni modificación del runtime GSP.

## 2026-08-27 — Etapa 5 verificada, 5/8

- Sesión 92829 exit 0: formato, Clippy estricto, regresión de metadata, workspace/all-targets/all-features, build locked, schemas/specs/contratos, release check, cargo deny offline, 13/13 SHA-256 y diff. Seis recorridos CLI de portabilidad/recuperación pasan; la última corrección del índice cruzado sí está incluida en este gate.
- Plan, matriz, README y readiness conciliados. PP-05/09/10 cerrados. Se preserva la diferencia entre exportación de historia y respaldo privado completo; revisión Markdown sólo ASSERTED, fork explícitamente no disponible, no contratos ampliados.
- Commit de cierre: `feat: complete verified portability and SQLite recovery`. Sin nueva CI, ZIP, GSP/ENOSPC opt-in ni proyecto real.

**Siguiente paso:** etapa 6, ejemplos ejecutables de la skill y recorrido Codex descartable; continuar después GSP y paquete Windows conforme a la autorización de terminar el plan.

## 2026-08-27 — Etapa 6 en ejecución, 5/8

- Etapa 5 cerrada en `9a03de0`. Skill actualizada con argv ejecutables, códigos de salida, scope de reconciliación y sin elevación humana. RED inicial por ejemplo inexistente; dos tests CLI y seis del paquete pasan; validador skill-creator PASS. El fixture de gate necesitó su digest real, no el placeholder del catálogo de schemas.
- Recorrido real Codex por CLI documentado en `docs/PREPILOT_CODEX_PROCEDURE.md`: progreso/evidencia/gate incompleto/checkpoint/reconciliación, datos descartables; sin SQLite directo ni Markdown editado. Denegación 9 y missing-result 12 comprobados.
- Hallazgo real: resume JSON usa presupuesto compacto de 220 bytes conservadores y elimina referencias/progreso cuando crece el texto; no pérdida en SQLite. Nueva aserción E2E exige preservar autoridad, gate pendiente y progreso después del checkpoint. Etapa 6 permanece abierta hasta corregir, repetir regresión y demostrar salida completa.
- Gate 12219 se interrumpió por acceso Windows al binario mientras estaba en uso por el recorrido; se evitó repetir el solapamiento. Gate 17205 corresponde al código anterior a la corrección de resume; no puede cerrar ese nuevo hallazgo. No tocar otros procesos ni repositorios.

- Resume corregido: modo Structured para JSON y resume.json portable sin pérdida de referencias por presupuesto textual. Markdown anuncia su acotación y muestra autoridad; skill usa JSON para argumentos exactos. No schemas modificados. RED explícito en copia del binario antiguo; tests focales 44889 PASS. Recorrido Codex repetido completo en v16 conserva autoridad/gate/progreso y audit sin findings. Detalles/digests en el informe de etapa 6.
- Gate fresco 24790: formato y Clippy pasaron; workspace completo en ejecución, luego build locked/specs/contratos/release/deny/checksums/diff. No ejecutar el binario target durante compilación Windows ni solapar pipelines Cargo. Etapas 7–8 sólo preparación/lectura: pin GSP sigue limpio y exacto, `integration gsp compile` sigue ausente (RED observado). No commit de etapa 6 todavía.

## 2026-08-27 — Etapa 6 verificada, 6/8

- Gate 24790 finalizó exit 0 con todas las comprobaciones enumeradas; quick_validate de la skill volvió a pasar. Recorrido Codex real repetido y E2E automatizado demuestran referencias conservadas, gate pendiente, permisos, versión, reconciliación y replay.
- PP-06/11 cerrados; JSON/portable resume completo y Markdown explícitamente acotado sin cambiar contratos. Plan, matriz, README y readiness conciliados. Commit: `fix: verify Codex workflow and preserve structured resume context`.
- Consulta read-only de issues (hasta 30, todos los estados) de gcatalansc/G-Ledger y gcatalansc/gsp-gemma-compiler devolvió [] en ambos. No se interpreta como aceptación bilateral; el informe GSP de etapa 7 distinguirá la evidencia local de certificación externa.

**Siguiente paso:** etapa 7, entrada GSP supervisada, matriz finita runtime/adversarial y revisión de evidencia disponible; luego ZIP/CI de etapa 8. No piloto real ni certificación.

## 2026-08-27 — Etapa 7 en ejecución, 6/8

- Etapa 6 cerrada en `805b783`. RED 99469: compile ausente, export GSP dejaba salir un canary ficticio y validate-result leía >8 MiB sin límite previo. Nueva entrada gsp_compile, policy explícita, supervisor de etapa 4, artefactos acotados y publicación validada sin enlace automático.
- RED adicional 80051: error de validación reproducía token ficticio y segundo actor con la misma clave provocaba UNIQUE/11. Corregidos escaneo previo + errores sin valores y command_id con scope de actor, preservando IDs históricos por reconciliación. Cinco tests nuevos CLI PASS.
- Sesión 15576 exit 0: dos tests GSP supervisado real (normal/contradicción/uncertainty, alteraciones de resultados reales, link/replay, startup timeout, cancelación prearmada y drift) y tres RQ4 reales, incluido crash dentro de compile_evidence, PID propio 6340, reinicio y un solo enlace. Runtime upstream limpio e intacto.
- Se añadió caso real con supervisor compartido para timeout y cancelación después del marker de entrada al compilador; no opción de inyección en producción. Clippy detectó un cast u64 redundante del test y se corrigió. Pipeline 68925: Clippy PASS y tres tests runtime supervisados en ejecución. CI incorpora esos tests para su futura corrida en etapa 8; todavía no se ha hecho push.
- Matriz/alcance y evidencia bilateral disponible en `docs/PREPILOT_GSP_RUNTIME_MATRIX.md`, aún declara gate pendiente. Próximo: completar 68925, full gate fresco, README/plan/readiness 7/8 y commit; después implementación y prueba del ZIP fuera del checkout + CI exact-SHA. No declarar piloto listo antes de 8/8.

## 2026-08-27 — Etapa 7 verificada, 7/8

- 68925 y gate agregado 62580 terminaron exit 0. Seis pruebas GSP opt-in reales ejecutadas, cinco CLI nuevas, full workspace, formato/Clippy, build locked, schemas/specs/contratos/release, dependencias y 13 hashes verificados. Matriz finita y separación de runtime/fixture/adversarial persistidas; ninguna aceptación externa atribuida.
- Plan, README, matriz y readiness conciliados; PP-12 cerrado. Commit de etapa: `feat: supervise and verify the pinned GSP runtime`. Sin nuevas descargas, migración de contratos, habilitación G-Skills/Radar, piloto real ni push todavía.
- Etapa 8 comienza con pruebas RED de resolución de recursos y del directorio instalado; siguen pendientes ZIP Windows y CI final. Se avanzará hasta cerrar el plan conforme a la autorización vigente.

**Siguiente paso:** completar recursos/ZIP, probar la extracción fuera del checkout y ejecutar la CI exact-SHA antes de declarar 8/8.

## 2026-08-27 — Etapa 8 en ejecución, 7/8

- Etapa 7 en `bbd160d`. RED 92171: CLI fuera del checkout tomaba schemas del cwd y faltaba catálogo portable. Implementado catálogo de recursos SHA-256 incorporado al binario, resolución por instalación o `--resources` (prioritario), fallback de desarrollo anclado al exe, sin dependencia de cwd. GREEN 33277: ambos tests reales PASS, incluida instalación con acentos, alteración rechazada y prioridad explícita.
- Recursos incluyen contratos, schemas, skill, sidecar verificado, reports y archivos fuente de evidencia para release verify; no checkout Git, credenciales, DB ni runtime Python. Readiness/bitácora quedan fuera del catálogo para no fabricar un ciclo de hashes con la entrega. Manual Windows persistido.
- Scripts nativos para ZIP nuevo, manifiesto de integridad, build con CRT estático, extracción temporal Unicode, PE imports y procesos con entorno sin PATH. Los mismos argv Codex se ejecutarán contra el binario extraído; nuevo recorrido CLI de portable/backup/restore/idempotencia y privacidad. Prueba local inicial corrigió expectativa del test: destino restore existente es INVALID_INPUT/2, no integridad/15.
- CI incorpora build/ensayo/upload del ZIP Windows, conservando las otras plataformas, ENOSPC y seis casos GSP. Acción upload-artifact v7.0.1 fijada a SHA consultado en su repo oficial; no cambios de permisos ni de visibilidad.
- Aún no hay ZIP verificado, benchmark fresco ni CI nueva. Siguiente: completar focales, compilar ZIP, corregir fallos reproducibles, repetir gate/benchmark/runtime y publicar candidato privado para observar CI exact-SHA. No cerrar etapa 8 por una compilación aislada.

- RED de distribución real 24701: ZIP del build normal válido en hashes, pero el verificador PE detectó dependencia VCRUNTIME140.dll. Por eso la entrega se compila con CRT estático y se exige pasar ese control; build 50196 en curso. Test de privacidad de proceso 50196 PASS antes del build: fallo observado registrado, canary ausente de resultado/SQLite/Markdown, sin promover gates. El canary de nota local se rechaza al exportar y se omite en vista compartible.
- Inspección del benchmark encontró measured_at fijo en 2026-08-26. Se extrajo su constructor sin cambiar comportamiento y se añadió prueba de frescura; debe reproducirse RED y corregirse antes de medir el candidato. No se sustituirá la fecha histórica de una medición anterior por una fecha nueva sin ejecutarla.

- ZIP estático candidato 50196 PASS; QA estricto 31049 repitió ambos harnesses, cada uno con exactamente 1 passed/0 failed, en extracción externa Unicode sin PATH. Se añadió rechazo de cero tests filtrados (un filtro inexistente retorna exit 0, reproducido). SHA candidato b13c67f7f4d8992f613084e366908d836d4522b38a3d6ce0fd33c77ad2c85976, árbol sucio identificado; no es la entrega final.
- RED benchmark 62658, GREEN 73652; Clippy corrigió orden del módulo test. 12018 Clippy + seis GSP reales PASS, incluidos tres supervisados con CLI del ZIP; crash RQ4 PID 5388, sin resultado/eventos antes del reinicio, enlace 1/replay 0. Upstreams siguen limpios.
- Medición fresca lanzada con el mismo binario estático copiado al artefacto de build target/release (no se modificaron datos de usuario); después regenerar reportes y correr gate completo. Siguen 7/8; quedan commit candidato limpio, ZIP actualizado y CI privada exact-SHA. No expandir alcance ni considerar el ZIP candidato sucio como cierre.

- 64399 exit 0: benchmark fresco 2026-08-28T02:39:50.27391Z, nueve targets PASS, startup p95 42,77 ms y RSS p95 19,684 MiB; reports/hashes regenerados. Windows local observado: 11 Home Single Language 10.0.26200 x64. README actualizado con la medición real, no fecha reemplazada sobre datos antiguos.
- Inventario cerrado de recursos: RED 61570, GREEN 23550 (3/3 tests). `crates/cli/resources.txt` autoriza exactamente 448 archivos; build sin enumeración de directorios, rutas relativas seguras, no symlinks y límites por recurso. Se conservó el mismo conjunto de recursos observado; se evita empaquetar extras locales ignorados/accidentales.
- Gate completo 70512 en ejecución: formato/Clippy/release generate PASS; tests workspace en curso. No solapar Cargo ni ejecutar target/debug mientras se reconstruye. Al terminar, commit candidato, ZIP limpio actualizado, push privado y observar seis jobs de CI; después cerrar etapa 8 documentalmente y conservar hashes/artefacto verificables.

## 2026-08-27 — Candidato de etapa 8, verificación externa pendiente, 7/8

- Gate 70512 terminó exit 0: suite workspace completa (53 comandos, CLI y nuevos tests incluidos), fmt, Clippy estricto, build locked, schemas/specs/contratos, release, deny e integridad 13/13. Los seis GSP reales fueron repetidos en 12018; ENOSPC físico será observado en CI.
- Revisión final del ZIP detectó omisión de LICENSE del proyecto: rechazo reproducido contra candidato-1; empaquetado corregido para incluirla y verificar su hash. La comprobación completa del ZIP limpio debe repetir ese caso. SBOM/NOTICE de contratos se preservan; no se declara certificación legal ni GA.
- Se prepara commit de candidato `feat: package Windows candidate with locked installation resources`. Es un checkpoint de implementación, no el cierre 8/8: faltan reconstrucción limpia/QA, push privado y CI exact-SHA. El cierre tendrá su propio commit documental con evidencia observada y artefacto identificado.

**Siguiente paso:** construir/probar el ZIP limpio del candidato y observar CI Windows/Linux/macOS, dependencias, ENOSPC y GSP. Resolver cualquier fallo del alcance antes de entregar.

## 2026-08-27 — Verificación local completa; cierre bloqueado por GitHub, 7/8

- Candidato `f4b34e591b13199d040ecf7d41a071e21bf63d37` comprometido y enviado al repositorio privado. 52201 exit 0: build estático locked, árbol limpio, ZIP con 448 recursos + binario + LICENSE; hashes y PE verificados. SHA ZIP `0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c`. Ambos harnesses ejecutaron exactamente un escenario con cero fallos desde extracción externa Unicode, sin PATH ni herramientas de desarrollo para el núcleo.
- 56539 exit 0: seis GSP reales repetidos usando la CLI de ese ZIP en los casos supervisados; crash RQ4 propio PID 45084 seguido de recuperación, un enlace y replay sin efecto. Los recursos del paquete, no el cwd, gobiernan la entrada de compilación. Prerrequisitos externos GSP siguen explícitos y separados.
- CI `33137113781` sobre f4b34e5 terminó failure sin ejecutar pasos. gh-fix-ci: auth válida, todos los jobs con steps=[] y las seis anotaciones coinciden en bloqueo de cuenta por pagos fallidos o límite de gasto. Windows 98739436754, macOS 98739436756, ENOSPC 98739436757, GSP 98739436787, Ubuntu 98739436805, dependencias 98739436808. No hay logs porque no arrancaron; no se atribuye el fallo al código ni se reutiliza la CI histórica como cierre nuevo.
- PP-08 y PP-13 cerrados con artefacto limpio; sólo PP-14 permanece abierto. Plan, matriz, README y readiness conservan 7/8 y NOT_READY. Informe de verificación local persistido sin rutas personales en `release/prepilot-windows-verification.json`.
- No se efectuaron pagos, cambios de límite/visibilidad, instalación de runners, ampliaciones de contratos, piloto real ni reintentos ciegos de CI. El checkpoint documental queda en commit local; no se fuerza otro push/ejecución conocida como bloqueada. Esto no es commit de cierre de etapa 8.

**Siguiente paso necesario:** el titular habilita GitHub Actions resolviendo la restricción indicada en Billing & plans; reejecutar CI del candidato exacto, verificar seis jobs y cerrar 8/8 con evidencia observada. No intervenir un proyecto real hasta entonces.

## 2026-08-27 — Reanudación solicitada, bloqueo externo confirmado, 7/8

- Nueva instrucción del usuario: «Procede profesionalmente». Se retomó desde el checkpoint documental local `bf8f986`, sin reiniciar etapas ni Git. `gh-fix-ci` guio la inspección de autenticación, run exact-SHA y anotaciones; `verification-before-completion` exige no declarar cierre sin CI ejecutada.
- Se realizó un único reintento de CI `33137113781` sobre `f4b34e591b13199d040ecf7d41a071e21bf63d37`. Attempt 2 terminó failure: ENOSPC 98744058292, GSP 98744058423, dependencias 98744058427, Windows 98744058452, Ubuntu 98744058456 y macOS 98744058466. A las 2026-08-28T03:20:39Z se confirmaron las seis anotaciones de pagos/límite de gasto y cero pasos en todos los jobs. No se observó fallo del código; no se harán más reintentos hasta confirmar resuelta la restricción.
- 58849 exit 0: ensayo fresco del ZIP limpio, SHA `0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c` sin cambios. Extracción externa nueva Unicode; hashes, imports PE y verificadores PASS; exactamente 1 test Codex y 1 portable/backup/restore/replay/privacidad PASS, además de negativos GSP ausente y recursos alterados. Se recompilaron los harnesses con dependencias locked; el núcleo ensayado fue el binario del ZIP, con entorno aislado sin toolchain.
- No hubo cambios de implementación. No se repitieron aquí el gate completo del workspace, benchmark ni seis casos GSP reales; sus resultados previos se conservan, sin fecharlos como nuevos. Plan, matriz, readiness y evidencia Windows registran la revalidación; sigue únicamente PP-14 abierto, 7/8 y NOT_READY.
- Control documental 39191 exit 0: JSON y contador coherentes, evidencia del intento 2, SHA ZIP coincidente, dos escenarios realmente ejecutados, ningún recurso incorporado al binario modificado, formato Rust y diff sin errores.
- No se cambiaron pagos, límites, visibilidad, runners o contratos, ni se intervino un proyecto real. Se conserva un nuevo checkpoint documental local; no se envía otro push que dispare CI bajo la misma restricción conocida. No es un commit de cierre de etapa 8.

**Siguiente paso necesario:** el titular resuelve el bloqueo de GitHub Actions y confirma que está habilitado. Después, repetir CI del candidato, verificar sus seis jobs y el artefacto alojado, resolver fallos reales si aparecen y cerrar 8/8 antes del piloto.

## 2026-08-27 — Consulta de continuidad sin nueva ejecución, 7/8

- Nueva instrucción: «Procede con el siguiente paso profesionalmente». Consulta de sólo lectura a GitHub a las 2026-08-28T03:24:56Z: autenticación válida, ninguna CI más reciente en main y run `33137113781` todavía en attempt 2, SHA `f4b34e591b13199d040ecf7d41a071e21bf63d37`, failure. La anotación Windows conserva el motivo de pagos fallidos/límite de gasto; esto describe aquel intento y no demuestra por sí solo el estado actual de facturación.
- No se ha recibido confirmación de que el titular resolvió la restricción. Conforme al punto de continuación registrado, no se lanza un tercer intento ni un push, no se alteran ajustes de cuenta y no se repiten pruebas locales ya verificadas para presentarlas como avance nuevo. El estado permanece 7/8, NOT_READY, PP-14 abierto.
- Sólo se actualiza esta bitácora en un checkpoint documental local, sin cambios de código, recursos del ZIP o evidencia histórica. La inspección sigue `gh-fix-ci` y el cierre continúa sujeto a `verification-before-completion`.

**Siguiente paso necesario:** confirmar con el titular si ya resolvió la restricción de GitHub Actions en Billing & plans. Si está resuelta, reanudar la CI del candidato y verificar los seis jobs y el artefacto antes del cierre 8/8; de lo contrario, esa gestión de cuenta sigue pendiente y no corresponde iniciar el piloto.

## 2026-08-27 — Tercer intento expresamente autorizado, 7/8

- El usuario indicó «Prueba ahora». Se reintentó una sola vez CI `33137113781`, tras comprobar autenticación y SHA exacto `f4b34e591b13199d040ecf7d41a071e21bf63d37`. Un error tipográfico en el primer comando de consulta se corrigió antes de enviar el reintento; no produjo otra ejecución.
- Attempt 3 terminó failure. A las 2026-08-28T03:45:01Z se consultaron los seis jobs y sus anotaciones: ENOSPC 98747749850, Ubuntu 98747749914, Windows 98747749918, dependencias 98747749919, macOS 98747749948 y GSP 98747750004. Todos con cero pasos y la misma restricción de pagos fallidos/límite de gasto. No es un fallo observado del código ni una CI aprobada; no hay un artefacto alojado nuevo.
- Se actualizan el último intento en readiness/informe Windows y la evidencia de entrega, conservando el historial en esta bitácora y Git. El estado sigue 7/8, NOT_READY y PP-14 abierto. No se repiten pruebas locales ni se atribuye una nueva fecha a resultados anteriores; no se cambian código, recursos, contratos, facturación, visibilidad o runners.
- Registro documental local, sin push que active otra CI bajo la restricción recién observada. Revisión con `gh-fix-ci` y cierre condicionado por `verification-before-completion`; no se inicia el piloto real.
- Control documental b39004 exit 0: JSON, intento 3/seis jobs sin pasos, 7/8, fechas históricas intactas y diff limitado a cuatro documentos. Se corrigió sólo la comprobación ad hoc de fechas: ConvertFrom-Json devuelve DateTime y la comparación inicial con texto fallaba; tipo, bytes y diff confirmaron que ningún timestamp histórico había cambiado. Diagnóstico con `systematic-debugging`, sin parche de código del producto.

**Siguiente paso necesario:** resolver la restricción de cuenta que GitHub sigue notificando y, una vez habilitadas las ejecuciones, verificar CI exact-SHA y artefacto alojado antes de cerrar 8/8. No repetir intentos automáticamente bajo el mismo bloqueo.

## 2026-08-27 — GitHub aplazado; entrega local de etapa 8 en curso, 7/8

- El usuario indicó «Dejemos pendiente github, avanza con lo siguiente del plan». Se aplazan consultas, reintentos y pushes; la evidencia del intento 3 se conserva como histórica y no se vuelve a inspeccionar en este turno. No se dispensan los criterios de cierre 8/8.
- La revisión del plan y la matriz confirma implementación local completa y sólo PP-14 abierto. Se separan en la checklist los controles locales ya verificados de la CI pendiente, sin atribuir al PASS local cobertura Linux/macOS/ENOSPC nueva.
- Continuación dentro de etapa 8: entrega local identificada, guía operativa probada en datos descartables, carpeta con ZIP/evidencia/checksums y registro final. Se usará el candidato limpio existente sin cambiar su binario ni recursos. No se agrega fase 9, se habilitan integraciones excluidas ni se interviene un proyecto real.
- `writing-plans` guía la separación del aplazamiento y la entrega; `verification-before-completion` exige ejecutar la guía/validar copias antes de declarar la entrega local comprobada. Trabajo inline, sin subagentes.

**Siguiente paso local:** preparar y ejecutar la guía desde el ZIP, empaquetar la evidencia local y cerrar este checkpoint documental; GitHub queda fuera de la actividad hasta nueva instrucción del usuario.

- Guía `docs/PREPILOT_LOCAL_HANDOFF.md` creada y ejecutada desde su copia en la carpeta de entrega, con ZIP de hash/fuente conocidos. 67950 exit 0, 2026-08-28T03:57:29Z: cuatro bloques, 19 invocaciones, v3, checkpoint `checkpoint:49cfa512-453c-7aa9-b4f9-6cbc3b70285e`; digest `sha256:ba63c0af8361920f212e6ded58ba19a662e49d1bb7b94be7f9d3277b292de51a` preservado por reconciliación tras restore. Replay DUPLICATE_REPLAY, rechazo 2 al sobrescribir destino, audit sin findings y SHA incorrecto rechazado antes de extraer.
- Evidencia local nueva separada de los gates históricos; no se volvió a compilar Ledger ni a ejecutar GSP/ENOSPC o CI. Procesos de guía sin PATH, sólo SystemRoot/TEMP/TMP, sin SQLite directo o autoridad humana atribuida. Resultado sanitizado en `release/prepilot-local-handoff.json`; pendiente validar el dossier completo y guardar commit local.

## 2026-08-28 — Entrega local preparada; GitHub continúa aplazado, 7/8

- 026894 exit 0, 2026-08-28T04:00:56Z: carpeta nueva `dist/entrega-local-0.2.0-f4b34e5/`, 17 archivos seleccionados explícitamente (ZIP, guía y 15 documentos/informes), todas las copias contrastadas por SHA-256 con sus fuentes. No se empaquetan los proyectos de ensayo ni los respaldos privados generados. El ZIP conserva su hash original; los documentos externos no alteran su catálogo incorporado.
- Guía entregada en LEEME.md, con cuatro bloques ya ejecutados desde esa copia; resultados en `release/prepilot-local-handoff.json`. README, matriz, plan y readiness distinguen la entrega local de GitHub DEFERRED_BY_USER. El aplazamiento no se trata como waiver, CI aprobada ni una novena fase.
- Se prepara commit de entrega local con revisión final de hashes/diff y sin push. No se ha consultado GitHub durante este turno. La evidencia del último intento conserva su fecha histórica. Código, contratos, dependencias y runtime no cambiaron; no corresponde atribuir a este cambio documental una nueva regresión del workspace.

- Integridad final 5f8556 exit 0: 17/17 entradas SHA-256, 18 archivos incluyendo el manifiesto, sin extras ni copias desactualizadas, documentos de readiness coherentes con 7/8 y GitHub aplazado, implementación/recursos del candidato sin cambios y diff sin errores. Entrega local comprobada; este registro no afirma cierre de etapa 8. Se conservará el mismo control después de incorporar esta anotación a las copias finales.

**Siguiente paso del plan:** no quedan tareas locales de implementación; revisar o repetir la guía descartable si se desea. Cuando el usuario retome GitHub, verificar CI exact-SHA/artefacto alojado para cerrar 8/8 y después elegir el proyecto real del piloto. No hay reintentos ni nuevas fases automáticas.

## 2026-08-28 — Diagnóstico de GitHub mediante navegación; cuota agotada, 7/8

- El usuario solicita analizar el bloqueo con navegación y presentar opciones. Se reanudan exclusivamente las consultas de diagnóstico: sesión autenticada del navegador, documentación oficial y lectura del run/anotación. No se autorizaron ni ejecutaron pagos, cambios de presupuesto, suscripciones, visibilidad, runners, push o reintentos de CI.
- Observación del panel de facturación, confirmada a las 11:18 UTC: GitHub Free, Actions 2.000/2.000 minutos consumidos, almacenamiento mostrado 0/0,5 GB y uso facturable USD 0. El panel indica reinicio de la cuota en cuatro días, correspondiente al siguiente período mensual alrededor del 1 de septiembre; no se verificó una hora exacta.
- Budgets and alerts muestra el presupuesto de Actions de alcance cuenta en USD 0, con Stop usage = Yes. Esta combinación de cuota agotada y gasto adicional bloqueado explica que no arranquen jobs. Payment history no muestra pagos realizados; no se observó evidencia de un pago rechazado ni se afirma que exista una deuda. La anotación genérica de CI por sí sola no distinguía esas causas. No se guardan capturas, datos de tarjeta, datos de facturación personales ni consumos de otros repositorios en este registro.
- Opciones documentadas: esperar la renovación gratuita; habilitar uso adicional con método de pago válido y un presupuesto acotado elegido por el titular; evaluar Pro (3.000 minutos mensuales incluidos, no ilimitados); o evaluar runners propios como cambio separado de infraestructura. Una máquina Windows no sustituye la matriz Linux/macOS ni las pruebas físicas requeridas. Hacer público el repositorio no es compatible con el acuerdo de privacidad.
- Precaución operativa: un presupuesto particular de repositorio no anula el presupuesto global de Actions agotado; GitHub bloquea el uso cuando cualquiera de los presupuestos aplicables con parada activada está agotado. Para uso adicional corresponde revisar el presupuesto existente de Actions y conservar parada/alertas, no eliminar límites ni modificar otros productos. La cuota pertenece a la cuenta y no exclusivamente a G-Ledger. Ningún importe propuesto constituye un pago autorizado ni una estimación garantizada del costo de la CI.
- Fuentes observadas: [Billing overview](https://github.com/settings/billing), [Budgets and alerts](https://github.com/settings/billing/budgets), [Payment history](https://github.com/account/billing/history), [facturación de Actions](https://docs.github.com/en/billing/concepts/product-billing/github-actions), [presupuestos superpuestos y parada](https://docs.github.com/en/billing/how-tos/set-up-budgets) y [runners propios](https://docs.github.com/en/actions/concepts/runners/self-hosted-runners). Las páginas de cuenta requieren la sesión del titular.
- `gh-fix-ci` y la skill de navegación guiaron la distinción entre la anotación del job y el estado actual de cuenta; `verification-before-completion` mantiene el cierre condicionado a evidencia ejecutada. Sólo se actualiza esta bitácora; código, paquete, dossier local ya entregado y resultados históricos permanecen intactos. Sin nueva regresión ni nueva CI que atribuir a esta revisión. Estado: 7/8, NOT_READY y PP-14 abierto.

**Siguiente paso:** el usuario elige esperar la renovación o habilitar gasto acotado. Tras comprobar cuota disponible o restricción resuelta y autorizar la ejecución, repetir la CI del candidato exacto, verificar sus seis jobs y el artefacto alojado; sólo entonces evaluar el cierre 8/8. No hay seguimiento ni reintentos automáticos.
