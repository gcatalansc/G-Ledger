# Bitácora de implementación de Project Ledger

> Este fichero registra la vida de la implementación. No es el estado canónico del producto ni sustituye la futura proyección generada `BITACORA_PROYECTO.md`.

## Estado actual

| Campo | Valor |
|---|---|
| Fecha de actualización | 2026-08-23 |
| Estado | FASE 1 COMPLETADA |
| Macrofases completadas (§50) | 2/12 |
| Slices completados (§51) | 3/15 |
| Última macrofase completada | Fase 1 — Event store y comandos |
| Macrofase siguiente | Fase 2 — Proyección y lifecycle |
| Slice siguiente | S3 — projection engine |
| Último commit de baseline | `6584c04227356cb156ff2614f8564f2ce2e1a68a` |
| Commit de cierre de fase | Este commit: `feat: complete transactional event store` |

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
