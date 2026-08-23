# Bitácora de implementación de Project Ledger

> Este fichero registra la vida de la implementación. No es el estado canónico del producto ni sustituye la futura proyección generada `BITACORA_PROYECTO.md`.

## Estado actual

| Campo | Valor |
|---|---|
| Fecha de actualización | 2026-08-23 |
| Estado | FASE 2 COMPLETADA — S4 |
| Macrofases completadas (§50) | 3/12 |
| Slices completados (§51) | 5/15 |
| Última macrofase completada | Fase 2 — Proyecciones y lifecycle básico |
| Macrofase actual | Fase 3 — Gates y evidencia (pendiente) |
| Último slice completado | S4 — lifecycle/authority/roadmap/status/catalog |
| Slice actual | S5 — gates/evidence/phase engine (pendiente) |
| Último commit de baseline | `6584c04227356cb156ff2614f8564f2ce2e1a68a` |
| Commit de cierre más reciente | Este commit: `feat: complete deterministic project lifecycle` |

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
