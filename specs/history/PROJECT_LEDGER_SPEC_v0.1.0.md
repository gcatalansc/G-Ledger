# Project Ledger — Especificación Técnica Integrada v0.1.0

| Campo | Valor |
|---|---|
| Proyecto | **Project Ledger** |
| Definición | Motor local de continuidad verificable para proyectos desarrollados por personas y agentes |
| Forma principal | Binario único en Rust + CLI + proyecciones de solo lectura + `RecordSink` explícito + exportaciones |
| Estado | `SPECIFICATION BASELINE` / `CODEX IMPLEMENTATION READY` |
| Versión | **0.1.0** |
| Fecha | **2026-08-16** |
| Idioma canónico | Español |
| Plataforma inicial | Windows 11 x86_64; core portable a Linux y macOS |
| Persistencia | SQLite + WAL, event store append-only y proyecciones reconstruibles |
| Interfaz canónica | CLI y JSON |
| Integraciones prioritarias | **G-Skills v0.3**, **GSP Internals v0.2**, Agent Skills/Codex y Project Radar |
| IA obligatoria | No |
| Red obligatoria | No |
| Daemon obligatorio | No |
| Licencia recomendada | Apache-2.0 |
| Archivo recomendado | `PROJECT_LEDGER_SPEC_v0.1.0.md` |

---

## Mapa del documento

1. [Estado, autoridad y convenciones](#0-estado-autoridad-y-convenciones)
2. [Resumen, problema y tesis](#1-resumen-ejecutivo)
3. [Objetivos, usuarios y alcance](#4-objetivos)
4. [Arquitectura y modelo de autoridad](#9-arquitectura-de-referencia)
5. [Modelo de dominio, eventos y comandos](#14-modelo-de-dominio)
6. [Gates, evidencia, decisiones y checkpoints](#21-motor-de-gates)
7. [CLI, renderizado y migración de bitácoras](#27-cli-canónica)
8. [Protocolos e integraciones](#32-plano-de-integración)
9. [Persistencia, seguridad y operación](#39-persistencia)
10. [Testing, benchmark y aceptación](#45-estrategia-de-testing)
11. [Roadmap, slices y estructura del repositorio](#50-roadmap-de-implementación)
12. [Definición de terminado y prompt para Codex](#56-definición-de-terminado)

---

# 0. Estado, autoridad y convenciones

## 0.1 Propósito normativo

Esta especificación define el baseline v0.1.0 de Project Ledger. Gobierna:

- el modelo de continuidad del proyecto;
- la autoridad sobre fases, gates, decisiones, blockers, riesgos y checkpoints;
- el event store y las proyecciones;
- la generación de bitácoras Markdown;
- la interfaz CLI;
- los contratos de integración;
- la compatibilidad con G-Skills y GSP;
- las restricciones de seguridad, privacidad y operación;
- el roadmap de implementación para Codex.

La implementación NO DEBE improvisar semánticas incompatibles con este documento. Cuando spec, schema, ejemplo y código entren en conflicto, el defecto debe registrarse y resolverse antes de declarar el contrato estable.

## 0.2 Fuentes de alineación

Esta spec se ha diseñado para resolver el problema observado en una bitácora real que registra autoridad técnica, fases, gates, evidencia, decisiones y próximos pasos, pero que puede conservar estados antiguos o contradictorios al crecer.

También adopta como frontera de interoperabilidad:

- G-Skills v0.3: `ProjectContinuityProjection`, separación absoluta entre proyección y mutación, autoridad por dominio, `ExternalRecordCommand`, idempotencia, concurrencia optimista y `GateRevalidationSignal`;
- GSP Internals v0.2: frontera post-retrieval `GSPCompiler.compile_bundle(query, evidence_bundle)`, preservación del contrato upstream exacto, provenance, contradicciones e incertidumbre;
- Project Radar: consumo de resúmenes de portfolio de solo lectura, sin asumir autoridad sobre la continuidad interna.

## 0.3 Orden de autoridad

1. Esta spec v0.1.0.
2. Los JSON Schemas materializados para v0.1.0.
3. Los ADR aceptados y versionados.
4. Las políticas locales del proyecto.
5. Los eventos canónicos persistidos.
6. Las proyecciones reconstruidas a partir de esos eventos.
7. Las bitácoras Markdown generadas.
8. Las salidas de agentes, LLM o importadores, siempre como propuestas hasta validación.

## 0.4 Palabras normativas

- **DEBE / MUST:** requisito obligatorio.
- **NO DEBE / MUST NOT:** prohibición obligatoria.
- **DEBERÍA / SHOULD:** recomendación fuerte.
- **PUEDE / MAY:** capacidad opcional compatible.
- **CANÓNICO:** fuente autoritativa dentro del dominio declarado.
- **DERIVADO:** reconstruible desde fuentes canónicas.
- **VERIFICADO:** sustentado por evidencia aceptada según un gate.
- **ASEVERADO:** declarado sin suficiente evidencia para considerarlo verificado.

## 0.5 IDs de requisitos

```text
PRD  producto y alcance
DOM  dominio y autoridad
EVT  eventos
CMD  comandos
GAT  gates
EVD  evidencia
DEC  decisiones
CHK  checkpoints y reanudación
RND  renderizado
CLI  interfaz CLI
INT  interoperabilidad genérica
GSK  integración G-Skills
GSP  integración GSP
AGT  Agent Skills/Codex
RAD  Project Radar
PST  persistencia
CON  concurrencia e idempotencia
SEC  seguridad
PRV  privacidad
OPS  operación
TST  pruebas
PER  rendimiento
MIG  migración
REL  release y compatibilidad
```

---

# 1. Resumen ejecutivo

**Project Ledger** es un motor local que convierte la continuidad de un proyecto en estado estructurado, verificable, auditable y resistente a la pérdida de contexto.

Su pregunta central es:

> **¿Qué ocurrió realmente en el proyecto, qué está vigente, qué está verificado y desde qué punto exacto debe continuar la siguiente persona o agente?**

No es un editor de diarios. Tampoco es una memoria general de LLM.

Project Ledger mantiene de forma canónica:

```text
autoridad documental
+ fases y roadmap
+ gates y evaluaciones
+ avances materiales
+ verificaciones
+ decisiones y supersesiones
+ blockers y riesgos
+ próximas acciones
+ checkpoints de continuidad
+ referencias de evidencia
```

El Markdown deja de ser el almacén primario. `BITACORA_PROYECTO.md` pasa a ser una proyección determinista y regenerable.

```text
comando explícito
      ↓
validación + expected version + idempotencia
      ↓
eventos append-only
      ↓
estado proyectado
      ↓
gate engine
      ↓
checkpoint / resume / bitácora Markdown
      ↓
G-Skills · GSP · Codex · Project Radar
```

---

# 2. Problema

En proyectos largos asistidos por IA, la continuidad queda distribuida entre:

- conversaciones;
- sesiones de Codex;
- specs;
- planes;
- commits;
- logs;
- tests;
- decisiones;
- archivos Markdown;
- memoria humana.

Una bitácora manual ayuda, pero presenta riesgos estructurales:

1. una sección ejecutiva puede indicar Fase 2 mientras otra sección todavía dice que Fase 0 no ha comenzado;
2. el avance `X/Y` puede quedar desactualizado;
3. un blocker resuelto puede seguir apareciendo como abierto;
4. una decisión superseded puede seguir leyéndose como vigente;
5. un test exitoso puede confundirse con cierre de gate;
6. una nueva sesión puede reanudar desde una instrucción histórica;
7. distintos agentes pueden editar varias secciones sin consistencia transaccional;
8. una afirmación puede aparecer como “verificada” sin evidencia reproducible.

El problema no es la falta de texto. Es la falta de un **modelo de estado autoritativo y reconstruible**.

---

# 3. Tesis y principios

## 3.1 Tesis principal

> La continuidad de un proyecto debe modelarse como estado derivado de eventos y reglas, no reconstruirse desde prosa mutable.

## 3.2 Principios

### P-01 — El Markdown es una vista

`BITACORA_PROYECTO.md` es una proyección humana. No es el event store.

### P-02 — Los eventos no se reescriben

La corrección se representa mediante nuevos eventos: supersesión, resolución, reapertura, invalidación o compensación.

### P-03 — Una fase no se completa porque exista código

La fase se completa únicamente cuando su política de cierre y sus gates lo permiten.

### P-04 — Evidencia y afirmación son distintas

“Se ejecutaron 24 tests” no es verificable si no existe comando, resultado, timestamp, exit code o referencia equivalente.

### P-05 — La autoridad es por dominio

Project Ledger gobierna continuidad. No gobierna seguridad del código, estructura del repositorio, evidencia documental de GSP ni portfolio de Project Radar.

### P-06 — Lectura y escritura están separadas

Una proyección de contexto no puede producir mutaciones. Toda escritura requiere un comando explícito.

### P-07 — Idempotencia y concurrencia optimista

Los agentes pueden reintentar, pero nunca duplicar eventos ni sobrescribir silenciosamente estado concurrente.

### P-08 — Integración por contratos, no por base compartida

G-Skills, GSP, Project Radar y Project Ledger conservan sus stores y migraciones.

### P-09 — Continuidad compacta

Una nueva sesión debe obtener un resumen mínimo, vigente y verificable sin cargar toda la historia.

### P-10 — IA opcional y no autoritativa

Una IA puede proponer extracción, clasificación o resumen. No puede cerrar gates, completar fases ni inventar evidencia.

---

# 4. Objetivos

## 4.1 Objetivo principal

Construir un producto local y ligero que permita:

```text
registrar trabajo material
→ asociar evidencia verificable
→ derivar estado vigente
→ evaluar gates
→ mantener decisiones y blockers
→ generar checkpoints compactos
→ renderizar bitácoras consistentes
→ integrarse con agentes y servicios sin acoplamiento
```

## 4.2 Objetivos funcionales

1. Inicializar un ledger desde cero.
2. Registrar autoridad documental y roadmap.
3. Definir fases y gates.
4. Registrar avances materiales.
5. Ejecutar o importar verificaciones.
6. Evaluar gates de forma determinista.
7. Cerrar o reabrir fases bajo reglas explícitas.
8. Registrar decisiones y supersesiones.
9. Gestionar blockers, riesgos y próximas acciones.
10. Crear checkpoints y resúmenes de reanudación.
11. Detectar contradicciones e información obsoleta.
12. Regenerar `BITACORA_PROYECTO.md`.
13. Exportar/importar un bundle portable.
14. Exponer una proyección de continuidad de solo lectura.
15. Aceptar comandos externos explícitos como `RecordSink`.
16. Integrarse con G-Skills sin compartir base de datos.
17. Articularse con GSP conservando sus contratos exactos.
18. Distribuir una Agent Skill para Codex y otros agentes.
19. Funcionar sin IA, red, cuenta ni daemon.
20. Mantener auditabilidad completa de transiciones.

## 4.3 Objetivos no funcionales

- single-binary-first;
- local-first;
- deterministic-first;
- cross-platform core;
- append-only;
- schema-first;
- auditable;
- privacy-first;
- bounded storage;
- no hidden side effects;
- no shared database;
- no silent overwrite;
- no fabricated current state;
- graceful degradation.

---

# 5. No objetivos del MVP

El MVP NO DEBE:

- convertirse en Jira, Scrum, Kanban o Gantt;
- gestionar equipos o permisos empresariales complejos;
- ejecutar agentes autónomos;
- analizar el código fuente;
- determinar impacto estructural;
- convertirse en memoria semántica general;
- sustituir G-Skills;
- sustituir GSP;
- sustituir Project Radar;
- cerrar un gate por juicio de LLM;
- inferir causalidad desde logs;
- compartir SQLite con otros productos;
- exigir nube;
- exigir una UI;
- sincronizar múltiples dispositivos;
- implementar un workflow engine arbitrario;
- almacenar transcripts o razonamiento privado;
- ejecutar comandos sin consentimiento o policy local explícita.

---

# 6. Usuarios

## 6.1 Desarrollador individual

Necesita retomar proyectos sin releer conversaciones.

## 6.2 Usuario de Codex

Necesita que cada sesión conozca fase, gates, decisiones, blocker y próximo paso.

## 6.3 Maintainer

Necesita comprobar por qué una fase se considera cerrada.

## 6.4 Arquitecto

Necesita conservar decisiones, supersesiones, desviaciones y autoridad documental.

## 6.5 Herramienta externa

Necesita consultar continuidad o registrar un resultado sin conocer el almacenamiento interno.

---

# 7. Casos de uso

### UC-01 — Inicio desde una spec

```text
project-ledger init
→ registra proyecto y autoridad
→ define roadmap/fases/gates
→ crea estado inicial
→ genera BITACORA_PROYECTO.md
```

### UC-02 — Avance material

Un agente implementa un repositorio SQLite y registra archivos afectados, resumen y referencias de commit.

### UC-03 — Verificación ejecutada

Project Ledger ejecuta `cargo test`, captura exit code, duración, hashes permitidos y crea evidencia.

### UC-04 — Gate incompleto

Build y tests pasan, pero falta E2E. El gate permanece `PENDING`.

### UC-05 — Fase cerrable

Todos los gates requeridos están verificados y no existen blockers incompatibles. El comando de cierre produce `PhaseCompleted`.

### UC-06 — Reapertura

Un cambio posterior invalida la evidencia de un gate. Project Ledger registra revalidación y puede reabrir la fase según policy.

### UC-07 — Reanudación

Una nueva sesión ejecuta `project-ledger resume --format codex` y recibe contexto compacto.

### UC-08 — Contradicción heredada

Se importa una bitácora Markdown antigua. El importador produce candidatos y conflictos, nunca estado verificado automático.

### UC-09 — G-Skills consulta continuidad

G-Skills solicita `ProjectContinuityProjection`; la consulta no cambia el ledger.

### UC-10 — G-Skills registra resultado

G-Skills prepara y envía `RECORD_VERIFICATION_RESULT` con expected version e idempotency key. Project Ledger decide si lo acepta y qué eventos deriva.

### UC-11 — GSP aporta evidencia

Un bridge genera un `EvidenceBundle` exacto para GSP. Project Ledger enlaza el resultado compilado como evidencia externa, pero no lo convierte automáticamente en cierre de gate.

### UC-12 — Project Radar consume resumen

Radar recibe un resumen de portfolio de solo lectura: estado, fase, blockers, actividad y próxima acción.

---

# 8. Forma de producto

## 8.1 Recomendación

La forma principal DEBE ser:

> **Un único binario local `project-ledger`, escrito en Rust, con CLI canónica, core embebible, servidor stdio opcional y exportaciones portables.**

Superficies:

| Superficie | Responsabilidad | Prioridad |
|---|---|---:|
| CLI | Operación humana, scripting, Codex y CI | Canónica |
| JSON stdout | Contrato automatizable | Canónica |
| Provider stdio JSON-RPC | Proyecciones read-only | MVP |
| RecordSink stdio JSON-RPC | Comandos externos explícitos | MVP |
| Agent Skill | Procedimiento para agentes | MVP |
| Export/import bundle | Portabilidad y backup | MVP |
| MCP server | Acceso directo de agentes | Post-MVP |
| Tauri UI | Timeline y revisión visual | Post-MVP |
| Daemon | Optimización opcional | No requerido |

## 8.2 Perfiles

```text
MINIMAL
CLI + SQLite + bitácora + checkpoints

AGENT
MINIMAL + Agent Skill + JSON outputs

ORCHESTRATED
AGENT + G-Skills provider/RecordSink

EVIDENCE
ORCHESTRATED + bridge GSP

EXTENDED
MCP + UI + Project Radar adapter
```

## 8.3 Golden path

El producto completo debe seguir siendo útil con:

```text
gskills.enabled = false
gsp.enabled = false
project_radar.enabled = false
ai.enabled = false
network = disabled
daemon = disabled
```

---

# 9. Arquitectura de referencia

```text
┌──────────────────────────────────────────────────────────────┐
│ CLI · JSON · Agent Skill · stdio Provider · stdio RecordSink │
└──────────────────────────────┬───────────────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────┐
│ APPLICATION LAYER                                            │
│ CommandBus · QueryBus · UnitOfWork · Authorization           │
└───────────────┬────────────────────────────┬─────────────────┘
                ▼                            ▼
┌────────────────────────────┐   ┌─────────────────────────────┐
│ DOMAIN CORE                │   │ INTEGRATION PLANE           │
│ Project aggregate          │   │ G-Skills profile            │
│ Phase/Gate engine          │   │ GSP bridge contracts        │
│ Decisions/Blockers/Risks   │   │ Radar summary export        │
│ Evidence rules             │   │ Agent/Codex adapters        │
│ Checkpoint builder         │   │ Generic provider SDK        │
└───────────────┬────────────┘   └──────────────┬──────────────┘
                ▼                               │
┌───────────────────────────────────────────────┴──────────────┐
│ EVENT STORE + PROJECTIONS                                    │
│ SQLite/WAL · append-only events · command dedup · snapshots  │
└──────────────────────────────┬───────────────────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────┐
│ DERIVED OUTPUTS                                              │
│ ProjectState · ResumeProjection · BITACORA_PROYECTO.md       │
│ AuditReport · PortableExport                                 │
└──────────────────────────────────────────────────────────────┘
```

## 9.1 Separación de crates

- `domain` no depende de SQLite, CLI, G-Skills, GSP ni Markdown.
- `application` depende del dominio y puertos.
- `storage-sqlite` implementa persistencia.
- `renderer-markdown` consume proyecciones.
- `interop-gskills` mapea contratos externos.
- `interop-gsp` contiene solo bridge/adaptador opcional.
- `cli` compone el binario.
- ningún adapter externo puede modificar el core mediante tipos privados.

---

# 10. Modelo de autoridad

## 10.1 Dominios propiedad de Project Ledger

```text
project.identity
project.authority_documents
project.lifecycle
project.roadmap
project.phases
project.gates
project.decisions
project.blockers
project.risks
project.next_actions
project.checkpoints
project.continuity
project.evidence_references
```

## 10.2 Dominios no propios

```text
repository.structure       → G-Skills Repository State
repository.impact          → G-Skills
code.guard                 → G-Skills policies/constraints
evidence.compilation       → GSP
portfolio.presentation     → Project Radar
organization.policy        → propietario humano/organizacional
agent.assertion            → agente, no canónico
```

## 10.3 Regla de autoridad externa

Una fuente externa puede aportar:

- evidencia;
- señales;
- referencias;
- comandos;
- propuestas.

No puede declarar por sí sola que un gate o fase quedó cerrado. Project Ledger ejecuta su propia validación de dominio.

---

# 11. Invariantes del corazón

### I-01 — Event store append-only

Los eventos canónicos no se actualizan ni eliminan in-place.

### I-02 — Estado derivado

`ProjectState`, `ResumeProjection` y Markdown se reconstruyen desde eventos.

### I-03 — Markdown no canónico

Editar manualmente `BITACORA_PROYECTO.md` no cambia el estado.

### I-04 — Comando explícito

Toda mutación atraviesa un `CommandEnvelope`.

### I-05 — Proyección sin efectos laterales

Una consulta read-only no cambia versión, eventos, evidencia ni checkpoints.

### I-06 — Expected version

Toda escritura externa o concurrente declara `expected_state_version`.

### I-07 — Idempotencia

La misma clave y payload producen como máximo un efecto.

### I-08 — Gate owner

Solo el Gate Engine puede producir `GateVerified`, `GateRejected` o `GateRevalidationRequired`.

### I-09 — Phase owner

Solo el Phase Engine puede producir `PhaseCompleted` o `PhaseReopened`.

### I-10 — Tests no equivalen a fase

Un resultado verde es evidencia; no es una transición de lifecycle.

### I-11 — Evidence provenance

Toda evidencia aceptada tiene fuente, timestamp, digest y estado de validación.

### I-12 — Aseverado no es verificado

Datos manuales sin evidencia se etiquetan `ASSERTED`.

### I-13 — Supersesión explícita

Una decisión antigua permanece en el historial y se marca superseded.

### I-14 — Blocker resuelto no desaparece

Cambia de estado mediante evento y conserva timeline.

### I-15 — Freshness visible

La continuidad ligada a un snapshot o autoridad obsoleta no se presenta como current.

### I-16 — Identidad no nominal

La igualdad de nombres no basta para enlazar proyectos o repositorios externos.

### I-17 — No base compartida

Ninguna integración requiere abrir la SQLite privada de otro producto.

### I-18 — No autoridad de LLM

Una salida de IA nunca produce por sí sola transición verificada.

### I-19 — No raw por defecto

Outputs completos de terminal, transcripts y secretos no se persisten por defecto.

### I-20 — Rebuild equivalence

Reconstruir proyecciones desde el event store produce estado semánticamente equivalente.

---

# 12. Identificadores, tiempo y digests

## 12.1 IDs

- UUIDv7 para eventos, comandos, proyectos, checkpoints y evidencias.
- IDs legibles opcionales para fases/gates/decisiones.
- URIs canónicas:

```text
project-ledger://projects/<project-id>
project-ledger://events/<event-id>
project-ledger://phases/<phase-id>
project-ledger://gates/<gate-id>
project-ledger://decisions/<decision-id>
project-ledger://evidence/<evidence-id>
project-ledger://checkpoints/<checkpoint-id>
```

## 12.2 Tiempo

- RFC 3339 con offset.
- `occurred_at`: cuándo ocurrió el hecho.
- `recorded_at`: cuándo fue persistido.
- `observed_at`: cuándo se observó evidencia.
- el orden canónico usa `stream_version`, no solo timestamps.

## 12.3 JSON canónico

Los digests se calculan sobre JSON canónico y SHA-256.

```text
command_digest
event_digest
payload_digest
state_digest
checkpoint_digest
projection_digest
```

Arrays sin orden semántico se ordenan por una clave estable antes de digerir.

---

# 13. Estado agregado del proyecto

```yaml
schema_version: 0.1.0
project_state_id: state:project-01H...
project:
  project_id: project:01H...
  title: MemPilot
  status: ACTIVE
  state_version: 184

authority_documents:
  - document_id: authority:spec
    uri: workspace://referencias/MEMPILOT_SPEC_V0_1_0.md
    digest: sha256:...
    status: CURRENT
    authority_rank: 1

roadmap:
  roadmap_id: roadmap:main
  phase_order: [phase-0, phase-1, phase-2, phase-3]

active_phase_ref: phase-2
completed_phase_refs: [phase-0, phase-1]

gates:
  pending: [gate:sqlite-roundtrip]
  verified: [gate:bootstrap, gate:collector]

decisions:
  current: [decision:sqlite, decision:retention-seven-days]

blockers:
  open: []

risks:
  open: [risk:sqlite-locking]

next_actions:
  - action:implement-sqlite-repository

checkpoint_ref: checkpoint:184
last_material_event_ref: event:702
updated_at: "2026-08-16T16:05:00-04:00"
state_digest: sha256:...
```

El estado es derivado y no se edita directamente.

---

# 14. Modelo de dominio

## 14.1 `Project`

Campos mínimos:

```text
project_id
title
summary
status
created_at
timezone
authority_policy
storage_profile
repository_bindings[]
```

Estados:

```text
INITIALIZING
ACTIVE
PAUSED
BLOCKED
COMPLETED
ARCHIVED
```

## 14.2 `AuthorityDocument`

```text
document_id
uri
digest
version
status
authority_rank
effective_from
supersedes_ref
```

Estados:

```text
CURRENT
DRAFT
LEGACY
DEPRECATED
SUPERSEDED
UNAVAILABLE
DIGEST_MISMATCH
```

## 14.3 `Roadmap`

```text
roadmap_id
title
phase_order[]
completion_policy
version
source_document_ref
```

## 14.4 `Phase`

```text
phase_id
title
ordinal
status
required_gate_refs[]
optional_gate_refs[]
started_at
completed_at
reopened_at
```

Estados:

```text
NOT_STARTED
ACTIVE
BLOCKED
READY_TO_CLOSE
COMPLETED
REVALIDATION_REQUIRED
REOPENED
SKIPPED
```

## 14.5 `Gate`

```text
gate_id
title
scope
status
required_evidence_rules[]
completion_rule
repository_dependency_refs[]
last_evaluation_ref
```

Estados:

```text
DEFINED
PENDING
EVALUATING
VERIFIED
REJECTED
WAIVED
STALE
REVALIDATION_REQUIRED
```

## 14.6 `Decision`

```text
decision_id
title
context
decision
rationale
alternatives[]
status
version
authority_ref
evidence_refs[]
supersedes_ref
```

Estados:

```text
PROPOSED
ACCEPTED
SUPERSEDED
REJECTED
DEPRECATED
REVALIDATION_REQUIRED
```

## 14.7 `WorkRecord`

Representa avance material, no cada edición.

```text
work_record_id
summary
phase_ref
task_ref
files_or_artifacts[]
commit_refs[]
claims[]
evidence_refs[]
actor
```

## 14.8 `Blocker`

Estados:

```text
OPEN
MITIGATED
RESOLVED
ACCEPTED
CANCELLED
```

## 14.9 `Risk`

Estados:

```text
OPEN
MONITORED
MITIGATED
REALIZED
CLOSED
```

## 14.10 `NextAction`

Clasificación:

```text
AUTHORIZED
ELIGIBLE
RECOMMENDED
PROPOSED
BLOCKED
COMPLETED
CANCELLED
```

Una acción `AUTHORIZED` DEBE enlazar una autoridad verificable.

## 14.11 `Checkpoint`

Contiene una vista compacta, refs y digests; no copia todo el event log.

## 14.12 `Session`

Opcional en MVP, pero el modelo reserva:

```text
session_id
actor
host
started_at
ended_at
starting_checkpoint_ref
ending_checkpoint_ref
```

---

# 15. Event sourcing

## 15.1 Flujo

```text
CommandEnvelope
→ schema validation
→ trust/authorization
→ idempotency lookup
→ expected version check
→ domain validation
→ append events in transaction
→ update projections
→ optional render
→ CommandResult
```

## 15.2 `EventEnvelope`

```yaml
schema_version: 0.1.0
event_id: event:0198...
project_ref: project:01H...
stream_version: 185
event_type: VERIFICATION_RECORDED
payload:
  evidence_ref: evidence:221
  phase_ref: phase-2
  gate_ref: gate:sqlite-roundtrip

actor:
  actor_type: AGENT_HARNESS
  actor_id: codex-session-44
  on_behalf_of: human:local-user

causation:
  command_ref: command:0198
  command_digest: sha256:...
  correlation_ref: session:44

occurred_at: "2026-08-16T16:05:01-04:00"
recorded_at: "2026-08-16T16:05:01-04:00"
previous_event_digest: sha256:...
payload_digest: sha256:...
event_digest: sha256:...
```

## 15.3 Catálogo inicial de eventos

### Proyecto y autoridad

```text
PROJECT_CREATED
PROJECT_METADATA_UPDATED
PROJECT_STATUS_CHANGED
AUTHORITY_DOCUMENT_REGISTERED
AUTHORITY_DOCUMENT_DIGEST_UPDATED
AUTHORITY_DOCUMENT_SUPERSEDED
AUTHORITY_DOCUMENT_MARKED_UNAVAILABLE
ROADMAP_DEFINED
ROADMAP_REVISED
```

### Fases

```text
PHASE_STARTED
PHASE_BLOCKED
PHASE_MARKED_READY_TO_CLOSE
PHASE_COMPLETED
PHASE_REOPENED
PHASE_SKIPPED
```

### Gates

```text
GATE_DEFINED
GATE_EVALUATION_REQUESTED
GATE_EVALUATED
GATE_VERIFIED
GATE_REJECTED
GATE_WAIVED
GATE_MARKED_STALE
GATE_REVALIDATION_REQUESTED
GATE_REVALIDATION_REQUIRED
```

### Trabajo y evidencia

```text
WORK_PROGRESS_RECORDED
ARTIFACT_REFERENCE_ADDED
VERIFICATION_RECORDED
EXTERNAL_EVIDENCE_LINKED
EXTERNAL_EVIDENCE_UNLINKED
EVIDENCE_INVALIDATED
```

### Decisiones

```text
DECISION_PROPOSED
DECISION_ACCEPTED
DECISION_SUPERSEDED
DECISION_REJECTED
DECISION_REVALIDATION_REQUIRED
```

### Blockers, riesgos y acciones

```text
BLOCKER_OPENED
BLOCKER_UPDATED
BLOCKER_RESOLVED
RISK_OPENED
RISK_UPDATED
RISK_MITIGATED
RISK_CLOSED
NEXT_ACTION_SET
NEXT_ACTION_COMPLETED
NEXT_ACTION_CANCELLED
```

### Continuidad

```text
CHECKPOINT_CREATED
CHECKPOINT_INVALIDATED
SESSION_STARTED
SESSION_ENDED
LEGACY_IMPORT_REVIEWED
CONFLICT_OPENED
CONFLICT_RESOLVED
```

## 15.4 Correcciones

No existe `EVENT_UPDATED`. Una corrección genera un evento explícito con:

```text
corrects_event_ref
reason
new_value
actor
```

---

# 16. Comandos

## 16.1 `CommandEnvelope`

```yaml
schema_version: 0.1.0
command_id: command:0198
command_type: RECORD_VERIFICATION_RESULT
project_ref: project:01H...
expected_state_version: 184
idempotency_key: project-ledger:outcome:982

actor:
  actor_type: AGENT_HARNESS
  actor_id: codex-session-44
  on_behalf_of: human:local-user

payload:
  gate_ref: gate:sqlite-roundtrip
  evidence:
    kind: TEST_RESULT
    exit_code: 0
    passed: 24
    failed: 0

submitted_at: "2026-08-16T16:05:00-04:00"
payload_digest: sha256:...
command_digest: sha256:...
```

## 16.2 Comandos canónicos

```text
CREATE_PROJECT
UPDATE_PROJECT_METADATA
REGISTER_AUTHORITY_DOCUMENT
SUPERSEDE_AUTHORITY_DOCUMENT
DEFINE_ROADMAP
REVISE_ROADMAP

START_PHASE
REQUEST_PHASE_COMPLETION
REOPEN_PHASE
SKIP_PHASE

DEFINE_GATE
REQUEST_GATE_EVALUATION
REQUEST_GATE_REVALIDATION
WAIVE_GATE

RECORD_WORK_PROGRESS
RECORD_VERIFICATION_RESULT
LINK_EXTERNAL_EVIDENCE
INVALIDATE_EVIDENCE

PROPOSE_DECISION
ACCEPT_DECISION
SUPERSEDE_DECISION
REJECT_DECISION

OPEN_BLOCKER
UPDATE_BLOCKER
RESOLVE_BLOCKER
OPEN_RISK
UPDATE_RISK
MITIGATE_RISK
CLOSE_RISK

SET_NEXT_ACTION
COMPLETE_NEXT_ACTION
CANCEL_NEXT_ACTION

CREATE_CONTINUITY_CHECKPOINT
START_SESSION
END_SESSION
```

No existe un comando externo directo `PHASE_COMPLETED` ni `GATE_VERIFIED`. Se solicitan evaluaciones; el dominio decide los eventos derivados.

## 16.3 Resultado de comando

```yaml
schema_version: 0.1.0
command_result_id: result:0198
command_ref: command:0198
status: ACCEPTED
previous_state_version: 184
new_state_version: 185
emitted_event_refs:
  - event:702
resulting_checkpoint_ref: null
rejection: null
idempotency:
  replayed: false
processed_at: "2026-08-16T16:05:01-04:00"
digest: sha256:...
```

Estados:

```text
ACCEPTED
REJECTED
VERSION_CONFLICT
VALIDATION_FAILED
POLICY_DENIED
DUPLICATE_REPLAY
OUTCOME_UNKNOWN
```

---

# 17. Máquina de estados de fase

## 17.1 Inicio

Una fase puede iniciarse si:

- existe en el roadmap vigente;
- su predecesora está completa o existe una excepción explícita;
- no hay otra fase activa, salvo roadmap paralelo declarado;
- la autoridad documental no está en conflicto crítico.

## 17.2 Cierre

`REQUEST_PHASE_COMPLETION` evalúa:

```text
all required gates ∈ {VERIFIED, WAIVED}
AND no incompatible open blocker
AND authority documents current
AND no critical continuity conflict
AND phase completion policy passes
```

Resultado:

- `PHASE_COMPLETED`;
- `PHASE_MARKED_READY_TO_CLOSE`;
- rechazo con razones estructuradas.

## 17.3 Reapertura

Una fase completada puede reabrirse por:

- gate revalidation required;
- autoridad documental superseded;
- evidencia invalidada;
- decisión material superseded;
- acción humana explícita con razón.

---

# 18. Máquina de estados de gate

```text
DEFINED
  ↓
PENDING
  ↓ request
EVALUATING
  ├── VERIFIED
  ├── REJECTED
  ├── WAIVED
  └── REVALIDATION_REQUIRED
```

`VERIFIED` no es permanente. Un gate puede pasar a:

```text
VERIFIED → STALE → REVALIDATION_REQUIRED → VERIFIED/REJECTED
```

El historial conserva todas las evaluaciones.

---

# 19. Conflictos de continuidad

## 19.1 Tipos

```text
AUTHORITY_DOCUMENT_DIVERGENCE
ACTIVE_PHASE_DIVERGENCE
GATE_STATE_DIVERGENCE
DECISION_DIVERGENCE
CHECKPOINT_STALE
REPOSITORY_BINDING_STALE
LEGACY_IMPORT_CONTRADICTION
EXTERNAL_VERSION_CONFLICT
```

## 19.2 Regla

Un conflicto crítico abierto:

- aparece en status, resume y bitácora;
- puede impedir cierre de gate/fase;
- no se resuelve por ranking o LLM;
- requiere evento de resolución o nueva evidencia.

---

# 20. Modelado de claims

Toda afirmación material se clasifica:

```text
ASSERTED
OBSERVED
VERIFIED
REJECTED
STALE
UNKNOWN
```

Ejemplos:

- “Se implementó SQLite”: `ASSERTED` si solo se registra manualmente.
- “El archivo existe y su digest coincide”: `OBSERVED`.
- “El gate SQLite roundtrip pasó”: `VERIFIED` tras Gate Engine.
- “La Fase 2 está completa”: solo `VERIFIED` si existe `PHASE_COMPLETED`.

---

# 21. Motor de gates

## 21.1 `GateDefinition`

```yaml
schema_version: 0.1.0
gate_id: gate:sqlite-roundtrip
title: SQLite roundtrip real
phase_ref: phase-2
criticality: REQUIRED

required_evidence_rules:
  - rule_id: tests-pass
    kind: TEST_RESULT
    predicate:
      exit_code_equals: 0
      failed_equals: 0
  - rule_id: build-pass
    kind: BUILD_RESULT
    predicate:
      exit_code_equals: 0
      errors_equals: 0

repository_dependencies:
  paths:
    - tests/persistence/**
  external_refs: []

completion_rule:
  operator: ALL
waiver_policy: HUMAN_APPROVAL
revalidate_on:
  - EVIDENCE_INVALIDATED
  - REPOSITORY_DEPENDENCY_CHANGED
  - AUTHORITY_DOCUMENT_CHANGED
```

## 21.2 Evidencia soportada

```text
TEST_RESULT
BUILD_RESULT
LINT_RESULT
FORMAT_RESULT
TYPECHECK_RESULT
SECURITY_SCAN_RESULT
PRIVACY_SCAN_RESULT
BENCHMARK_RESULT
SMOKE_RESULT
E2E_RESULT
MANUAL_REVIEW
ARTIFACT_DIGEST
FILE_EXISTENCE
SCHEMA_VALIDATION
MIGRATION_RESULT
EXTERNAL_VERIFIED_EVIDENCE
CUSTOM_DECLARATIVE
```

## 21.3 Predicados

El MVP usa un AST declarativo, no scripts:

```text
all / any / not
equals / not_equals
less_than / greater_than
exists / missing
contains
digest_equals
status_in
count_equals / count_at_least
```

No se ejecuta código arbitrario desde un gate.

## 21.4 Evaluación

El motor produce:

```text
PASS
FAIL
INCOMPLETE
STALE
CONFLICTED
ERROR
```

y una explicación con:

- reglas satisfechas;
- reglas faltantes;
- evidencia usada;
- evidencia rechazada;
- freshness;
- conflictos;
- versión de la definición.

## 21.5 Waivers

Un waiver requiere:

```text
actor
reason
scope
expiration o review condition
authority
gate version
```

No elimina la evidencia faltante; cambia el estado a `WAIVED`.

---

# 22. Evidencia

## 22.1 `EvidenceRecord`

```yaml
schema_version: 0.1.0
evidence_id: evidence:221
project_ref: project:01H...
kind: TEST_RESULT
status: VERIFIED_SOURCE

source:
  source_type: COMMAND_EXECUTION
  tool: dotnet
  command_display: "dotnet test MemPilot.slnx --no-build --no-restore"
  working_directory_ref: workspace://
  runner_version: project-ledger/0.1.0

result:
  exit_code: 0
  duration_ms: 48120
  metrics:
    passed: 24
    failed: 0
    skipped: 0
  summary: "24 superados, 0 fallidos"

repository_binding:
  repository_ref: repo:7d24
  snapshot_ref: snapshot:0198
  snapshot_digest: sha256:...

artifacts:
  - uri: workspace://TestResults/results.trx
    digest: sha256:...
    persisted: false

privacy:
  raw_output_stored: false
  redaction_applied: true

observed_at: "2026-08-16T16:04:00-04:00"
payload_digest: sha256:...
provenance_ref: provenance:evidence-221
```

## 22.2 Estados de evidencia

```text
ASSERTED
OBSERVED
VERIFIED_SOURCE
REJECTED
STALE
INVALIDATED
UNAVAILABLE
```

`VERIFIED_SOURCE` significa que el registro y su origen son verificables; no implica que toda interpretación del contenido sea verdadera.

## 22.3 Ejecución de comandos

`project-ledger evidence run`:

1. muestra o recibe el comando;
2. aplica allowlist/policy;
3. ejecuta sin shell interpolation por defecto;
4. captura exit code y métricas;
5. redacta secretos;
6. persiste resumen y digests;
7. no persiste output raw salvo opt-in;
8. crea un `EvidenceRecord`.

## 22.4 Evidencia externa

Se almacena como referencia:

```text
external_system
artifact_uri
artifact_digest
schema_ref
validation_state
provenance
summary
```

Project Ledger no copia por defecto el bundle completo de GSP ni el `DecisionEnvelope` completo de G-Skills.

---

# 23. Decisiones

## 23.1 Regla

Una decisión aceptada es canónica para el dominio del proyecto hasta ser superseded, rechazada o marcada para revalidación.

## 23.2 Supersesión

```text
decision:v1 ACCEPTED
       ↓
decision:v2 ACCEPTED, supersedes=v1
       ↓
v1 permanece consultable con status SUPERSEDED
```

## 23.3 Conflictos

Dos decisiones vigentes incompatibles producen `DECISION_DIVERGENCE`; no se elige silenciosamente la más reciente si su autoridad o scope difieren.

## 23.4 Renderizado ADR

Project Ledger PUEDE renderizar decisiones como:

```text
docs/adr/ADR-0042-sqlite-persistence.md
```

pero el registro canónico permanece en el ledger.

---

# 24. Blockers, riesgos y próximas acciones

## 24.1 Blocker

Un blocker puede bloquear:

- fase;
- gate;
- acción;
- release.

El scope es explícito.

## 24.2 Riesgo

Un riesgo no bloquea automáticamente. La policy puede convertir severidad crítica + condición en blocker.

## 24.3 Próxima acción

La proyección de resume ordena:

1. `AUTHORIZED`;
2. `ELIGIBLE`;
3. `RECOMMENDED`;
4. `PROPOSED`.

Project Ledger no debe presentar una acción propuesta por un agente como autorizada sin `authority_ref`.

---

# 25. Checkpoints

## 25.1 Propósito

Un checkpoint es una captura compacta e inmutable de continuidad para:

- cierre de sesión;
- handoff;
- compactación de contexto;
- cambio de agente;
- auditoría;
- recuperación ante fallo.

## 25.2 `ContinuityCheckpoint`

```yaml
schema_version: 0.1.0
checkpoint_id: checkpoint:184
project_ref: project:01H...
project_state_version: 184
project_state_digest: sha256:...

authority_documents:
  - ref: authority:spec
    digest: sha256:...

active_phase_ref: phase-2
verified_completed_phase_refs: [phase-0, phase-1]
pending_gate_refs: [gate:sqlite-roundtrip]
current_decision_refs: [decision:sqlite, decision:retention-seven-days]
open_blocker_refs: []
open_risk_refs: [risk:sqlite-locking]
next_action_refs: [action:implement-sqlite-repository]

last_material_events:
  - event:701
  - event:702

repository_binding:
  repository_ref: repo:7d24
  snapshot_ref: snapshot:0198
  snapshot_digest: sha256:...

unknowns: []
conflict_refs: []
created_by: agent:codex-session-44
created_at: "2026-08-16T16:05:00-04:00"
checkpoint_digest: sha256:...
```

## 25.3 Freshness

Estados:

```text
CURRENT
CURRENT_FOR_PROJECT_ONLY
STALE_FOR_REPOSITORY
STALE_FOR_AUTHORITY
CONFLICTED
INVALIDATED
UNKNOWN
```

## 25.4 Regla

Un checkpoint stale puede seguir siendo útil como historia, pero no debe presentarse como contexto vigente sin advertencia.

---

# 26. Resume projection

## 26.1 Perfil compacto

```text
PROJECT
AUTHORITY
STATE VERSION / FRESHNESS
ACTIVE PHASE
VERIFIED COMPLETED PHASES
CURRENT DECISIONS
OPEN BLOCKERS / RISKS
PENDING GATES
LAST MATERIAL PROGRESS
NEXT AUTHORIZED OR ELIGIBLE ACTION
CRITICAL UNKNOWNS / CONFLICTS
CHECKPOINT
```

## 26.2 Presupuesto

Targets:

```text
compact text      ≤ 220 tokens
agent markdown    ≤ 450 tokens
full human view   sin límite estricto, pero resumida
```

## 26.3 Ejemplo

```text
Proyecto: MemPilot
Estado: v184, CURRENT
Fase activa: 2 — Persistencia
Completadas: Fases 0 y 1, verificadas
Gate pendiente: SQLite roundtrip real
Decisiones vigentes: SQLite; retención 7 días
Blockers: ninguno
Riesgo: locking SQLite, MEDIUM
Último avance: contratos de historial implementados
Siguiente acción elegible: implementar repository SQLite mediante RED→GREEN
Checkpoint: project-ledger://checkpoints/184
```

---

# 27. CLI canónica

## 27.1 Comandos de proyecto

```bash
project-ledger init
project-ledger status
project-ledger doctor
project-ledger audit
project-ledger verify
```

## 27.2 Autoridad y roadmap

```bash
project-ledger authority add --uri <path-or-uri> --digest <sha256>
project-ledger authority list
project-ledger authority supersede <id> --with <uri>
project-ledger roadmap import --file roadmap.yaml
project-ledger roadmap show
```

## 27.3 Fases y gates

```bash
project-ledger phase list
project-ledger phase start <phase-id>
project-ledger phase request-close <phase-id>
project-ledger phase reopen <phase-id> --reason <text>

project-ledger gate list [--phase <id>]
project-ledger gate show <gate-id>
project-ledger gate define --file gate.yaml
project-ledger gate evaluate <gate-id>
project-ledger gate request-revalidation <gate-id> --signal signal.json
project-ledger gate waive <gate-id> --reason <text> --approved-by <actor>
```

## 27.4 Trabajo y evidencia

```bash
project-ledger record progress \
  --phase phase-2 \
  --summary "Implementados contratos de historial" \
  --files src/... tests/...

project-ledger evidence run \
  --kind TEST_RESULT \
  --gate gate:sqlite-roundtrip \
  -- dotnet test MemPilot.slnx

project-ledger evidence import --file result.json
project-ledger evidence link --system gsp --artifact-ref <uri> --digest <sha256>
project-ledger evidence show <id>
```

## 27.5 Decisiones, blockers y riesgos

```bash
project-ledger decision propose --file decision.yaml
project-ledger decision accept <id>
project-ledger decision supersede <id> --with decision.yaml

project-ledger blocker open --title "..."
project-ledger blocker resolve <id> --evidence <ref>

project-ledger risk open --title "..." --severity MEDIUM
project-ledger risk mitigate <id> --note "..."
```

## 27.6 Continuidad

```bash
project-ledger next set --title "..." --classification ELIGIBLE
project-ledger checkpoint create --reason session-end
project-ledger resume
project-ledger resume --format codex
project-ledger resume --format json
project-ledger render
```

## 27.7 Import/export

```bash
project-ledger export --format portable --output ledger.bundle
project-ledger import --file ledger.bundle --mode merge
project-ledger import-markdown BITACORA_PROYECTO.md --preview
project-ledger import-markdown BITACORA_PROYECTO.md --apply-reviewed review.json
```

## 27.8 Integraciones

```bash
project-ledger serve provider --stdio --profile gskills-0.3
project-ledger serve sink --stdio --profile gskills-0.3
project-ledger integration manifest
project-ledger gsp export-corpus --output ./ledger-corpus
project-ledger radar export --output portfolio-summary.json
```

## 27.9 Output

- datos JSON a stdout con `--json`;
- diagnostics a stderr;
- ninguna barra de progreso en JSON;
- exit codes estables.

## 27.10 Exit codes

```text
0  success
2  invalid input/schema
3  domain validation failed
4  project not initialized
5  version conflict
6  gate incomplete/rejected
7  identity unresolved
8  conflict open
9  policy denied
10 idempotency conflict
11 storage/migration error
12 timeout/cancelled
13 privacy/security rejection
14 integration incompatible
```

---

# 28. Renderizado de `BITACORA_PROYECTO.md`

## 28.1 Regla

El archivo generado DEBE comenzar con:

```markdown
<!-- GENERATED BY PROJECT LEDGER. DO NOT EDIT AS CANONICAL STATE. -->
```

## 28.2 Secciones mínimas

```text
1. Estado ejecutivo
2. Autoridad y documentos vigentes
3. Fase activa y roadmap
4. Gates
5. Últimos avances materiales
6. Evidencia verificada
7. Decisiones vigentes
8. Blockers y riesgos
9. Próximas acciones
10. Checkpoint de continuidad
11. Timeline reciente
12. Conflictos y unknowns
13. Apéndice histórico resumido
```

## 28.3 Determinismo

Mismo `ProjectState` + misma versión de renderer produce el mismo contenido, salvo campos explícitamente no deterministas excluidos del digest.

## 28.4 Edición humana

Notas humanas se incorporan mediante:

```bash
project-ledger record note ...
```

No se importan automáticamente desde modificaciones manuales del archivo generado.

## 28.5 Divergencia

Si el archivo generado fue editado:

- `verify` detecta digest distinto;
- informa `GENERATED_ARTIFACT_MODIFIED`;
- permite regenerar;
- nunca importa cambios silenciosamente.

---

# 29. Importación de bitácoras existentes

## 29.1 Objetivo

Permitir migrar bitácoras manuales sin convertir contradicciones históricas en estado canónico.

## 29.2 Pipeline

```text
Markdown
→ parse sections
→ extract candidates
→ classify claim status
→ detect contradictions
→ produce LegacyImportReview
→ human/agent review
→ explicit commands
→ events
```

## 29.3 Regla de seguridad

La importación inicial:

- no marca fases como verificadas;
- no acepta gates cerrados sin evidence refs;
- no usa el último texto cronológico como autoridad automática;
- no elimina información contradictoria;
- conserva el archivo y su digest como provenance.

## 29.4 `LegacyImportCandidate`

```yaml
candidate_id: legacy:phase-2-active
kind: ACTIVE_PHASE
value: phase-2
source:
  path: BITACORA_PROYECTO.md
  section: Estado ejecutivo actual
  line_range: [8, 8]
conflicts_with:
  - legacy:phase-0-not-started
recommended_action: REVIEW
```

## 29.5 Modos

```text
PREVIEW
REVIEWED_IMPORT
ARCHIVE_ONLY
```

No existe `TRUST_ALL`.

---

# 30. Auditoría y verificación interna

## 30.1 `project-ledger verify`

Comprueba:

- cadena de digests;
- stream versions;
- schemas;
- proyecciones vs rebuild;
- idempotency index;
- checkpoints;
- generated artifact digest;
- authority document digests;
- relaciones huérfanas;
- estados incompatibles.

## 30.2 `project-ledger audit`

Detecta:

```text
claim verificado sin evidencia
fase completada con gate incompleto
gate vigente con evidencia invalidada
decisión actual y superseded simultáneamente
blocker abierto y resuelto simultáneamente
checkpoint stale presentado como current
próxima acción sin authority
Markdown modificado
mapping externo ambiguo
comando duplicado con payload distinto
```

## 30.3 Severidad

```text
CRITICAL
HIGH
MEDIUM
LOW
INFO
```

Hallazgos críticos bloquean release y pueden bloquear nuevas transiciones de lifecycle.

---

# 31. Portable export

## 31.1 Contenido

```text
project-ledger-export/
├── manifest.json
├── project.json
├── events.jsonl
├── schemas-lock.json
├── projections/
│   ├── state.json
│   └── resume.json
├── evidence/
│   └── references.json
├── generated/
│   └── BITACORA_PROYECTO.md
└── checksums.sha256
```

## 31.2 Exclusiones por defecto

- raw terminal output;
- secrets;
- transcripts;
- absolute paths;
- private user IDs;
- encrypted blobs sin opt-in;
- caches.

## 31.3 Import

- valida checksums;
- valida schemas;
- detecta project identity;
- aplica merge por event ID;
- rechaza misma ID con digest distinto;
- no sobrescribe una versión más nueva.

---

# 32. Plano de integración

## 32.1 Principio

Project Ledger expone:

```text
READ PATH
manifest → health → projection

WRITE PATH
validate/prepare → submit → result → reconcile
```

Las rutas son independientes.

## 32.2 Transportes

```text
LOCAL_STDIO_JSON_RPC      MVP preferido
LOCAL_ARTIFACT_EXCHANGE   MVP
LOCAL_SOCKET              post-MVP
MCP                       post-MVP
REMOTE_HTTP               fuera del MVP
```

## 32.3 Sin plugins in-process

Adaptadores de terceros no se cargan como librerías dinámicas dentro del core. Se usan sidecars o artifact exchange.

## 32.4 Manifest

```yaml
provider_id: project-ledger
provider_version: 0.1.0
protocol:
  name: project-ledger-integration
  version: 0.1.0

projection_capabilities:
  - PROJECT_CONTINUITY
  - PORTFOLIO_SUMMARY

record_capabilities:
  - RECORD_WORK_PROGRESS
  - RECORD_VERIFICATION_RESULT
  - REQUEST_GATE_EVALUATION
  - REQUEST_GATE_REVALIDATION
  - RECORD_DECISION_REFERENCE
  - RECORD_BLOCKER_REFERENCE
  - CREATE_CONTINUITY_CHECKPOINT

side_effect_contract:
  projection: NONE
  health: NONE
  record_submit: DECLARED_ONLY

storage:
  shared_database_required: false
```

---

# 33. Protocolo stdio JSON-RPC

## 33.1 Métodos read-only

```text
ledger.manifest
ledger.health
ledger.project_continuity
ledger.portfolio_summary
ledger.command_status
```

## 33.2 Métodos mutables

```text
ledger.command_validate
ledger.command_submit
ledger.command_reconcile
```

## 33.3 Request autocontenido

Toda solicitud incluye:

```text
protocol version
request ID
project identity
repository identity opcional
expected state version
budget
requested fields
privacy policy
trace/correlation IDs
```

## 33.4 Timeouts y cancelación

- timeout por request;
- cancel notification;
- output size limitado;
- un crash no corrompe el event store;
- reintentos mutables requieren idempotency key.

## 33.5 Framing

El perfil de referencia usa JSON-RPC 2.0 sobre stdio con framing `Content-Length`.

---

# 34. Integración con G-Skills v0.3

## 34.1 Rol

Project Ledger implementa dos perfiles:

```text
ProjectContinuityProvider
RecordSink
```

G-Skills sigue siendo propietario de:

- Repository State;
- impacto;
- skills/constraints;
- policies;
- `DecisionEnvelope`;
- `ContextCapsule`;
- guardas.

Project Ledger sigue siendo propietario de:

- lifecycle;
- fases;
- gates;
- decisiones;
- blockers;
- riesgos;
- checkpoints.

## 34.2 Perfil de compatibilidad

Nombre:

```text
gskills-context-provider/0.3
```

Comando:

```bash
project-ledger serve provider --stdio --profile gskills-0.3
project-ledger serve sink --stdio --profile gskills-0.3
```

La implementación DEBE mapear sin pérdida material a:

```text
ProviderManifest
ProviderHealthReport
ContextProjectionRequest
ContextProjectionEnvelope
ProviderFreshnessRecord
ProjectContinuityProjection
ExternalRecordCommand
ExternalRecordResult
GateRevalidationSignal
```

## 34.3 `ProjectContinuityProjection`

Project Ledger debe poder producir:

```yaml
project:
  project_ref: project:mempilot
  external_project_ref: project-ledger://projects/01H...
  title: MemPilot
  state_version: 184

authority_documents: []
active_phase: {}
verified_completed_phases: []
current_decisions: []
open_blockers: []
open_risks: []
pending_gates: []
next_actions: []
checkpoint: {}
repository_binding: {}
recent_material_events: []
unknowns: []
conflict_refs: []
digest: sha256:...
```

La projection:

- no incluye el event log completo;
- no muta el ledger;
- respeta campos solicitados y budget;
- declara omissions;
- declara freshness;
- nunca fabrica `no blockers` si el campo no pudo evaluarse.

## 34.4 Authority domains

Project Ledger puede reclamar capacidades, pero G-Skills decide la autoridad efectiva. Dominios compatibles:

```text
project.lifecycle
project.phases
project.gates
project.decisions
project.blockers
project.risks
project.checkpoints
```

Debe reclamar `NONE` para:

```text
repository.structure
repository.impact
code.guard
organization.policy
evidence.compilation
```

## 34.5 Freshness

La projection expone:

```text
project_state_version
project_state_digest
checkpoint_ref/digest
authority_document_digests
repository_binding
```

G-Skills puede clasificar:

```text
CURRENT
CURRENT_FOR_PROJECT_ONLY
STALE_FOR_REPOSITORY_STATE
STALE_FOR_AUTHORITY_DOCUMENTS
IDENTITY_UNRESOLVED
UNAVAILABLE
```

## 34.6 RecordSink

Commands compatibles:

```text
RECORD_WORK_PROGRESS
RECORD_VERIFICATION_RESULT
REQUEST_GATE_EVALUATION
REQUEST_GATE_REVALIDATION
RECORD_DECISION_REFERENCE
RECORD_BLOCKER_REFERENCE
CREATE_CONTINUITY_CHECKPOINT
```

Project Ledger valida y traduce a comandos internos. No acepta:

```text
PHASE_COMPLETED
GATE_VERIFIED
```

## 34.7 Idempotencia y optimistic concurrency

Cada command requiere:

```text
expected_state_version
idempotency_key
actor
payload_digest
```

- versión distinta → `VERSION_CONFLICT`;
- misma clave + mismo payload → resultado anterior;
- misma clave + payload distinto → rechazo;
- timeout incierto → consulta de reconciliación antes de reintentar.

## 34.8 Gate revalidation

`GateRevalidationSignal` de G-Skills es una señal, no una mutación.

Flujo:

```text
G-Skills detecta impacto
→ prepara REQUEST_GATE_REVALIDATION
→ Project Ledger valida gate y versión
→ registra GATE_REVALIDATION_REQUESTED
→ Gate Engine decide STALE/REVALIDATION_REQUIRED/no-op
```

## 34.9 Identidad

El mapping repositorio↔proyecto debe ser explícito:

```text
USER_CONFIRMED
EXACT_STABLE_ID
EXACT_REPOSITORY_REMOTE_DIGEST
SIGNED_MAPPING
IMPORTED_MAPPING
```

Name-only nunca habilita lectura canónica ni escritura.

## 34.10 Conformance

Project Ledger debe incluir fixtures que cubran:

1. projection sin efectos laterales;
2. continuidad current;
3. continuidad stale;
4. identidad ambigua;
5. provider unavailable;
6. command accepted;
7. phase remains open after evidence;
8. duplicate replay;
9. version conflict;
10. unknown outcome reconciliation;
11. gate revalidation;
12. G-Skills never gains direct DB access.

---


## 34.11 Estado de compatibilidad

Esta spec elimina la ausencia de contrato upstream de Project Ledger, pero **no convierte por sí sola la integración en certificada**.

La etiqueta inicial es:

```text
PROJECT LEDGER UPSTREAM BASELINE: FROZEN v0.1.0
G-SKILLS v0.3 PROFILE: CONTRACT CANDIDATE
CERTIFIED COMPATIBILITY: PENDING CONFORMANCE
```

La compatibilidad solo puede declararse certificada cuando:

1. los schemas exactos de ambos lados estén fijados por digest;
2. el adapter pase las suites de projection y RecordSink;
3. los escenarios de idempotencia, freshness, identidad y revalidación pasen E2E;
4. el informe de pérdidas sea vacío o no material;
5. la matriz de versiones se publique.


# 35. Integración con GSP

## 35.1 Frontera

Project Ledger NO DEBE redefinir GSP ni crear un schema reducido de sus objetos.

La integración respeta:

```text
GSPCompiler.compile_bundle(query, evidence_bundle) -> CompilationResult
```

y preserva los contratos upstream exactos:

```text
EvidenceBundle
CompilationBundle
ValidationReport
UncertaintyMap
TaskProjectionBundle
```

## 35.2 Responsabilidades

### Project Ledger aporta

- documentos de autoridad;
- eventos seleccionados;
- decisiones;
- gates;
- referencias de evidencia;
- versionado;
- URIs y digests;
- estado current/legacy/superseded.

### GSP aporta

- extracción proposition-first;
- compilación de claims;
- provenance;
- contradicciones;
- incertidumbre;
- suficiencia y abstención;
- `TaskProjectionBundle`.

### Ninguno aporta automáticamente

- cierre de gate;
- cierre de fase;
- policy de código;
- causalidad operacional.

## 35.3 Dos modos de integración

### Modo A — Corpus de solo lectura

```bash
project-ledger gsp export-corpus --output ./ledger-corpus
```

Produce Markdown/TXT y sidecars minimizados:

```text
ledger-corpus/
├── authority/
├── decisions/
├── gates/
├── checkpoints/
├── events/
└── manifest.json
```

GSP puede usar su ruta `run_corpus`.

### Modo B — Adapter exact-contract

Un sidecar `project-ledger-gsp-adapter`:

1. recibe query y scope;
2. consulta Project Ledger read-only;
3. recupera fuentes autorizadas;
4. construye un `EvidenceBundle` exacto usando schemas upstream GSP;
5. invoca `compile_bundle`;
6. devuelve refs/digests del `CompilationResult`;
7. no escribe en Project Ledger.

## 35.4 Link de resultado GSP

El resultado se enlaza mediante comando explícito:

```bash
project-ledger evidence link \
  --system gsp \
  --artifact-ref gsp://compilations/01H... \
  --digest sha256:... \
  --validation-state PASS
```

El ledger registra:

```text
schema_ref
artifact_ref
artifact_digest
validation_report_ref
uncertainty_ref
contradiction_refs
query_digest
provenance
```

## 35.5 Authority

Un claim de GSP conserva autoridad `evidence.compilation`. No pasa automáticamente a:

```text
project.gates
project.lifecycle
organization.policy
code.guard
```

Un gate puede requerir `EXTERNAL_VERIFIED_EVIDENCE`, pero su definición debe especificar qué validación y predicates acepta.

## 35.6 Conflictos

Si GSP indica un requisito current que contradice una decisión del Ledger:

- ambos artefactos permanecen;
- se abre conflicto cross-domain;
- no se supersede la decisión automáticamente;
- puede requerirse revisión humana o actualización de autoridad documental.

## 35.7 Dependencias

- el core Rust no depende de Python;
- el bridge GSP es opcional;
- un fallo GSP no afecta al golden path;
- no se comparte SQLite;
- no se persisten bundles completos por defecto.

---

# 36. Agent Skill y Codex

## 36.1 Skill de referencia

Paquete:

```text
integrations/agent-skills/project-ledger-continuity/
├── SKILL.md
├── examples/
└── schemas/
```

## 36.2 Comportamiento instruccional

La skill enseña al agente:

1. ejecutar `project-ledger resume --format codex` al comenzar;
2. respetar fase, gates, decisiones y blockers vigentes;
3. registrar solo avances materiales;
4. ejecutar evidencia mediante comandos verificables;
5. no editar el Markdown generado;
6. no declarar gates o fases completadas;
7. crear checkpoint al cerrar o transferir sesión;
8. manejar `VERSION_CONFLICT`;
9. no reintentar una escritura con outcome desconocido;
10. registrar limitaciones y unknowns.

## 36.3 La skill no es autoridad

La skill no sustituye:

- Gate Engine;
- expected version;
- idempotencia;
- policy;
- schemas;
- tests;
- verificación de evidencia.

## 36.4 Flujo Codex recomendado

```text
SESSION START
project-ledger resume --format codex

AFTER MATERIAL WORK
project-ledger record progress ...

AFTER TEST/BUILD
project-ledger evidence run ...

BEFORE CLAIMING COMPLETION
project-ledger gate evaluate ...
project-ledger phase request-close ...

SESSION END
project-ledger checkpoint create --reason session-end
project-ledger render
```

## 36.5 AGENTS.md

El repositorio PUEDE incluir un bloque mínimo que remita a la CLI y a la skill, sin copiar el estado vigente.

---

# 37. Integración con Project Radar

## 37.1 Rol

Project Radar consume una proyección de portfolio de solo lectura.

## 37.2 Campos

```text
project_ref
title
status
active_phase
verified_progress
open_blocker_count
critical_risk_count
last_material_activity_at
next_action
checkpoint_freshness
attention_signals
```

## 37.3 Restricciones

- Radar no escribe en Project Ledger en v0.1;
- Radar no es fuente canónica de lifecycle;
- la exportación no incluye eventos completos ni evidencia;
- un fallo de Radar no afecta Ledger.

---

# 38. Extensibilidad

## 38.1 Adapter manifest

Toda integración declara:

```text
adapter_id
version
protocol range
projection kinds
command kinds
read/write capabilities
network behavior
data fields
runtime dependency
schemas
loss report
```

## 38.2 Reemplazabilidad

Un adapter puede desaparecer sin invalidar el event store.

## 38.3 No marketplace automático

El MVP no instala ejecutables ni concede trust desde metadatos de repositorio.

## 38.4 Future integrations

- GitHub Issues;
- Jira/Linear;
- CI systems;
- IDEs;
- document stores;
- artifact registries.

Deben mapearse a proyecciones y comandos tipados, no a acceso directo al store.

---


## 38.5 Integration SDK y kit de conformidad

Para facilitar nuevas articulaciones, el repositorio DEBE publicar un SDK declarativo y un runner de conformidad.

Superficies mínimas:

```text
IntegrationManifest builder
typed request/response envelopes
URI and digest helpers
canonical JSON helpers
stdio JSON-RPC framing
test fixtures
malicious/negative fixtures
loss-report utilities
```

CLI:

```bash
project-ledger integration validate-manifest manifest.json
project-ledger integration conformance --command <adapter-command>
project-ledger integration explain-loss loss-report.json
project-ledger integration compatibility
```

La conformidad verifica:

```text
schema negotiation
projection side-effect count = 0
bounded output
timeout/cancellation
identity ambiguity
freshness semantics
authority non-escalation
redaction
same request + same state determinism
failure isolation
```

Para adapters mutables añade:

```text
write trust separate from read trust
prepare/submit separation
idempotency replay
same key + different payload rejection
expected-version conflict
unknown-outcome reconciliation
no direct lifecycle transition invention
```

Superar el kit demuestra compatibilidad de protocolo, no seguridad total ni corrección semántica del producto externo.


# 39. Persistencia

## 39.1 Canonical store

SQLite + WAL, con event store append-only.

Tablas conceptuales:

```text
projects
event_streams
events
commands
command_results
idempotency_keys
project_state_snapshots
evidence_records
evidence_artifact_refs
checkpoints
external_references
generated_artifacts
schema_migrations
audit_records
```

## 39.2 Derived projections

```text
project_state
phase_view
gate_view
decision_view
blocker_view
risk_view
timeline_view
resume_view
portfolio_view
```

Pueden eliminarse y reconstruirse.

## 39.3 Ubicación

Por defecto:

```text
<user-data>/project-ledger/projects/<project-id>/ledger.db
```

El repositorio contiene:

```text
.project-ledger/
├── project.toml
├── gates/
├── policies/
├── schemas.lock
└── .gitignore

BITACORA_PROYECTO.md
```

`project.toml` contiene el stable project ID y configuración, no el historial privado.

## 39.4 Modo portable

`storage.mode = repository-local` PUEDE habilitarse explícitamente en el futuro. No es default del MVP.

## 39.5 Backups

- snapshot consistente;
- export portable;
- verificación de checksums;
- backup antes de migraciones;
- restore no destructive por defecto.

---

# 40. Concurrencia, idempotencia y recuperación

## 40.1 Transacción de escritura

```text
BEGIN IMMEDIATE
→ comprobar expected version
→ comprobar idempotency
→ validar dominio
→ append events
→ update projection/snapshot
→ store result
COMMIT
```

## 40.2 Lock

Un solo writer por proyecto; múltiples readers.

## 40.3 Replay

- misma key + mismo digest → devuelve resultado previo;
- misma key + digest distinto → `IDEMPOTENCY_CONFLICT`;
- sin key en write externo → rechazo.

## 40.4 Unknown outcome

Si el cliente pierde la respuesta:

```text
submit
→ timeout
→ command status by idempotency key
→ accepted/rejected/unknown
```

No hay reintento ciego.

## 40.5 Crash recovery

Al arrancar:

- integrity check;
- migrations;
- WAL recovery;
- projection version check;
- rebuild si es necesario;
- generated artifact stale check.

---

# 41. Seguridad

## 41.1 Trust levels

```text
UNTRUSTED
LOCAL_USER
TRUSTED_PROJECT
TRUSTED_FOR_PROJECTION
TRUSTED_FOR_RECORDING
REVOKED
```

Read trust y write trust son distintos.

## 41.2 Ejecución de comandos

- arguments como array;
- no shell por defecto;
- working directory explícito;
- timeout;
- límites de output;
- environment allowlist;
- secret redaction;
- sin network automática.

## 41.3 Untrusted repository

No puede:

- habilitar RecordSink;
- conceder authority;
- ejecutar adapters;
- modificar configuración global;
- activar network;
- proporcionar credenciales.

## 41.4 Integridad

- digest chain opcional/activada por defecto;
- schema validation;
- checksum de authority docs;
- audit de cambios de trust;
- protección contra path traversal y symlink escape.

---

# 42. Privacidad

## 42.1 Defaults

```text
network                  off
telemetry                off
raw terminal output      off
transcripts              never
reasoning privado        never
absolute paths in export minimized
external payload storage references-only
```

## 42.2 Redacción

Patrones:

- API keys;
- tokens;
- passwords;
- connection strings;
- emails opcional;
- rutas de usuario;
- secrets configurables.

## 42.3 Cifrado

Payloads sensibles persistidos por opt-in usan envelope encryption y key provider del SO. La falta de keyring seguro no habilita plaintext silencioso.

## 42.4 Privacy gate

```text
secret_persistence_count == 0
```

sobre:

```text
DB
WAL/SHM
logs
exports
generated Markdown
crash artifacts
integration payloads
```

---

# 43. Observabilidad

## 43.1 Logs

Structured metadata only:

```text
request_id
command_id
project_id
event_type
state_version
status
latency
digest prefixes
```

No payloads completos por defecto.

## 43.2 Métricas

```text
commands_total{type,status}
events_appended_total{type}
gate_evaluations_total{result}
phase_transitions_total{type}
idempotency_replays_total
version_conflicts_total
checkpoint_build_latency_ms
resume_render_tokens
projection_requests_total{profile,status}
record_commands_total{profile,status}
markdown_divergence_total
legacy_import_conflicts_total
```

## 43.3 Auditoría

Trust changes, waivers, external writes y lifecycle transitions producen audit records.

---

# 44. Rendimiento

Targets de referencia:

```text
cold CLI startup p95                    < 250 ms
status/resume warm p95                  < 100 ms
append command p95                      < 150 ms
gate evaluation simple p95              < 100 ms
render 10k events p95                   < 500 ms
projection request warm p95             < 200 ms
memory one-shot p95                     < 100 MiB
idle CPU                                0 % sin daemon
network calls default                   0
```

Son presupuestos de ingeniería, no garantías universales.

---

# 45. Estrategia de testing

## 45.1 Unit tests

- canonical JSON/digests;
- event application;
- command validation;
- phase state machine;
- gate predicates;
- decision supersession;
- blocker/risk transitions;
- checkpoint builder;
- resume budgeter;
- Markdown renderer;
- redactor;
- identity mapping.

## 45.2 Contract tests

Cada schema requiere:

- fixture positivo;
- fixture negativo;
- `$id`;
- `additionalProperties: false` cuando corresponda;
- resolución offline;
- digest fixture;
- compatibility declaration.

## 45.3 Property tests

```text
same events → same state digest
rebuild projection → semantic equivalence
same key + same payload → at most one effect
same key + different payload → reject
version mismatch → no append
projection request → state version unchanged
verified gate → evidence rules passed or waiver valid
completed phase → required gates satisfied
superseded decision → never current
stale checkpoint → never rendered CURRENT
```

## 45.4 Integration tests

- SQLite append/rebuild;
- WAL recovery;
- migration rollback;
- command transaction;
- evidence command execution;
- generated Markdown divergence;
- portable export/import;
- legacy Markdown preview/review;
- stdio JSON-RPC;
- G-Skills provider profile;
- G-Skills RecordSink profile;
- GSP corpus export;
- Radar summary export.

## 45.5 E2E

```text
init
→ authority
→ roadmap
→ phase start
→ gate define
→ progress
→ test evidence
→ gate incomplete
→ additional evidence
→ gate verified
→ phase close
→ checkpoint
→ resume
→ render
→ export
→ reset/import
→ equivalent state
```

## 45.6 Chaos

- crash during append;
- disk full;
- malformed event;
- corrupted projection;
- sidecar crash;
- timeout after command acceptance;
- concurrent writers;
- authority document changed mid-session;
- Markdown manually edited;
- adapter version changed.

## 45.7 Security/privacy

- path traversal;
- symlink escape;
- malicious command args;
- prompt injection in imported text;
- oversized JSON;
- secret in stdout/stderr;
- untrusted repo enabling sink;
- name-only identity collision;
- adapter authority overclaim.

---

# 46. Escenarios obligatorios del benchmark

### PL-01 — Bitácora contradictoria

Una fuente dice Fase 2 activa y otra “Fase 0 no iniciada”. Import produce conflicto, no estado current automático.

### PL-02 — Tests verdes, gate incompleto

Tests pasan, pero falta build. Gate sigue pendiente.

### PL-03 — Fase no cerrable

Todos los archivos existen, pero un blocker crítico está abierto.

### PL-04 — Supersesión

Una decisión nueva reemplaza SQLite por otra tecnología; solo la nueva aparece como current.

### PL-05 — Revalidación

Un gate verificado se marca stale por cambio de autoridad o snapshot.

### PL-06 — Replay idempotente

El mismo comando externo se envía dos veces; existe un efecto.

### PL-07 — Version conflict

Dos agentes escriben sobre v184; uno gana y el otro recibe conflicto.

### PL-08 — Unknown outcome

Se pierde la respuesta después del commit; reconcile devuelve el resultado original.

### PL-09 — Projection side-effect-free

100 proyecciones consecutivas no cambian state version ni event count.

### PL-10 — G-Skills current projection

Fase, gates, decisiones, blockers y checkpoint mapean sin pérdida.

### PL-11 — G-Skills revalidation signal

La señal no modifica gate hasta command explícito.

### PL-12 — GSP separation

Un claim GSP no cierra gate ni supersede decisión por sí solo.

### PL-13 — GSP conflict

Requisito current de GSP contradice decisión Ledger; se abre conflicto cross-domain.

### PL-14 — Radar read-only

Exportar portfolio no cambia Ledger.

### PL-15 — Generated Markdown

Editar manualmente la bitácora produce divergencia y regeneración, no nuevos eventos.

### PL-16 — Portable roundtrip

Export→reset→import reproduce state digest y timeline semántica.

### PL-17 — Provider unavailable

Ledger standalone sigue funcionando.

### PL-18 — Identity ambiguity

Dos proyectos con mismo nombre no se fusionan.

---

# 47. Gates de calidad

```text
event_chain_integrity                         = 1.00
projection_rebuild_equivalence                = 1.00
duplicate_external_effects                    = 0
blind_retries_after_unknown                   = 0
stale_checkpoint_rendered_current             = 0
verified_gate_without_valid_evidence           = 0
completed_phase_with_unsatisfied_required_gate = 0
projection_hidden_side_effects                = 0
cross_system_false_identity_merge             = 0
secret_persistence_count                      = 0
GSP_semantic_authority_elevation              = 0
GSkills_direct_database_access                = 0
Markdown_manual_edit_becomes_state            = 0
```

---

# 48. Criterios de aceptación del MVP

## Producto y autonomía

- **AC-001:** `project-ledger init` crea un proyecto funcional sin red.
- **AC-002:** el core funciona sin G-Skills, GSP, Radar, IA o daemon.
- **AC-003:** CLI y JSON producen semántica equivalente.
- **AC-004:** el binario es único para el perfil default.
- **AC-005:** el proyecto puede exportarse e importarse.

## Event store y proyecciones

- **AC-006:** los eventos son append-only.
- **AC-007:** el estado se reconstruye desde cero.
- **AC-008:** la cadena de versiones no presenta gaps.
- **AC-009:** una proyección no muta el ledger.
- **AC-010:** una proyección corrupta se reconstruye.

## Fases y gates

- **AC-011:** una fase no se completa con gates requeridos pendientes.
- **AC-012:** un gate verificado referencia evidencia aceptada.
- **AC-013:** un waiver requiere actor, razón y scope.
- **AC-014:** evidencia invalidada reevalúa gates dependientes.
- **AC-015:** una fase puede reabrirse con provenance.

## Evidencia

- **AC-016:** `evidence run` registra exit code, duración y digest.
- **AC-017:** raw output no se almacena por defecto.
- **AC-018:** una afirmación manual no aparece como verificada.
- **AC-019:** evidencia externa conserva sistema, URI, digest y schema.
- **AC-020:** un resultado verde no cierra automáticamente fase/gate.

## Decisiones y continuidad

- **AC-021:** una decisión superseded no aparece como current.
- **AC-022:** blockers y riesgos conservan historial.
- **AC-023:** el resume cabe en el budget configurado.
- **AC-024:** checkpoint stale se etiqueta.
- **AC-025:** próxima acción autorizada tiene authority ref.

## Markdown

- **AC-026:** la bitácora se genera determinísticamente.
- **AC-027:** editarla no modifica estado.
- **AC-028:** `verify` detecta divergencia.
- **AC-029:** regenerar corrige la vista.
- **AC-030:** el Markdown muestra conflictos y unknowns críticos.

## Idempotencia y concurrencia

- **AC-031:** misma key/payload produce un efecto.
- **AC-032:** misma key/payload distinto se rechaza.
- **AC-033:** version conflict no sobrescribe.
- **AC-034:** unknown outcome se reconcilia.
- **AC-035:** crash recovery conserva verdad de auditoría.

## G-Skills

- **AC-036:** `ProjectContinuityProjection` valida contra el perfil acordado.
- **AC-037:** projection no produce eventos.
- **AC-038:** authority overclaim no tiene efecto.
- **AC-039:** identidad ambigua deshabilita writes.
- **AC-040:** `GateRevalidationSignal` no muta.
- **AC-041:** RecordSink requiere write trust.
- **AC-042:** G-Skills no puede enviar `PHASE_COMPLETED`.
- **AC-043:** expected version e idempotency son obligatorios.
- **AC-044:** provider failure no afecta Ledger.
- **AC-045:** no existe acceso directo a DB.

## GSP

- **AC-046:** el core no define un schema GSP reducido.
- **AC-047:** el bridge usa contratos upstream exactos.
- **AC-048:** un claim GSP no adquiere lifecycle authority.
- **AC-049:** contradictions/uncertainty se preservan por referencia.
- **AC-050:** GSP failure no afecta Ledger.
- **AC-051:** el corpus exportado conserva URIs, digests y status.
- **AC-052:** el link de resultado requiere comando explícito.
- **AC-053:** no se comparte DB.
- **AC-054:** no hay dependencia Python en el core.
- **AC-055:** conflicto GSP↔Ledger queda visible.

## Seguridad y privacidad

- **AC-056:** un repo no confiable no habilita writes/adapters.
- **AC-057:** no se ejecuta shell interpolation por defecto.
- **AC-058:** secret persistence es cero.
- **AC-059:** export minimiza datos sensibles.
- **AC-060:** no se almacenan transcripts ni reasoning privado.

## Release

- **AC-061:** tests unit/integration/E2E pasan en Windows y Linux.
- **AC-062:** schemas resuelven offline.
- **AC-063:** benchmarks PL-01…PL-18 pasan.
- **AC-064:** performance budgets se miden y publican.
- **AC-065:** la documentación describe límites sin sobredeclarar compatibilidad.

---

# 49. Matriz de trazabilidad crítica

| Requisito | Componente | Test mínimo | Slice |
|---|---|---|---:|
| I-01 / AC-006 | event-store | `events_are_append_only` | S2 |
| I-02 / AC-007 | projection-engine | `rebuild_equivalence` | S3 |
| I-04 | command-bus | `query_cannot_append` | S2 |
| I-06 / AC-033 | concurrency | `version_conflict_no_overwrite` | S2 |
| I-07 / AC-031 | idempotency | `same_key_single_effect` | S2 |
| I-08 / AC-012 | gate-engine | `gate_verified_only_by_engine` | S4 |
| I-10 / AC-020 | phase-engine | `green_tests_do_not_close_phase` | S4 |
| I-15 / AC-024 | checkpoint | `stale_never_current` | S5 |
| AC-026 | renderer | `markdown_golden_deterministic` | S5 |
| AC-036 | interop-gskills | `continuity_projection_contract` | S6 |
| AC-041 | record-sink | `read_trust_no_write` | S7 |
| AC-047 | interop-gsp | `exact_contract_bridge` | S8 |
| AC-058 | privacy | `secret_scan_zero` | S9 |

---

# 50. Roadmap de implementación

## Fase 0 — Contratos y esqueleto

Entregables:

- workspace Rust;
- CI;
- schemas v0.1;
- canonical JSON/digests;
- error model;
- fixture registry;
- docs y ADR iniciales.

Gate:

```text
fmt + clippy + tests + schemas offline = PASS
```

## Fase 1 — Event store y comandos

- SQLite/WAL;
- event envelope;
- command envelope;
- stream version;
- idempotency;
- transactions;
- audit.

Gate:

```text
append/replay/version conflict/crash tests = PASS
```

## Fase 2 — Proyecciones y dominio básico

- Project;
- authority docs;
- roadmap;
- phases;
- decisions;
- blockers;
- risks;
- next actions;
- state rebuild.

Gate:

```text
clean rebuild equivalence = PASS
```

## Fase 3 — Gates y evidencia

- GateDefinition;
- predicate AST;
- evidence records;
- command runner;
- gate evaluation;
- phase completion/reopen.

Gate:

```text
no verified gate without evidence
no completed phase with pending required gate
```

## Fase 4 — Continuidad y Markdown

- checkpoints;
- freshness;
- resume;
- renderer;
- audit/verify;
- legacy import preview/review;
- portable export/import.

Gate:

```text
Markdown deterministic
portable roundtrip equivalent
legacy contradiction preserved
```

## Fase 5 — Provider read-only para G-Skills

- stdio JSON-RPC;
- manifest/health;
- ProjectContinuityProjection;
- budget;
- freshness;
- identity mapping;
- conformance fixtures.

Gate:

```text
projection side effects = 0
```

## Fase 6 — RecordSink para G-Skills

- validate/submit/reconcile;
- write trust;
- external command mapping;
- unknown outcome;
- gate revalidation;
- E2E.

Gate:

```text
duplicate effects = 0
blind retries = 0
```

## Fase 7 — GSP bridge y Agent Skill

- corpus export;
- exact-contract adapter boundary;
- external evidence linking;
- Agent Skill;
- Codex workflow;
- optional Radar summary.

Gate:

```text
GSP authority elevation = 0
Codex handoff E2E = PASS
```

## Fase 8 — Hardening y release

- security/privacy;
- fuzzing;
- chaos;
- performance;
- packaging;
- SBOM;
- migration docs;
- release report.

Gate:

```text
AC-001…AC-065 = PASS o waiver no crítico explícito
```

---

# 51. Orden de implementación para Codex

## S0 — Repository skeleton

Workspace, CI, docs, license, rust-toolchain, deny config.

## S1 — Contracts

IDs, timestamps, canonical JSON, schemas, fixtures, digests.

## S2 — Event store

SQLite, commands, events, idempotency, expected version.

## S3 — Projection engine

ProjectState y rebuild.

## S4 — Lifecycle core

Authority, roadmap, phases, decisions, blockers, risks, actions.

## S5 — Gates/evidence

Predicate AST, evidence runner, Gate Engine, Phase Engine.

## S6 — Continuity outputs

Checkpoint, resume, Markdown, audit, verify.

## S7 — Import/export

Portable bundle y legacy Markdown review.

## S8 — Integration protocol

stdio JSON-RPC, manifest, health, identities.

## S9 — G-Skills provider

Projection exacta y conformance.

## S10 — G-Skills RecordSink

Commands, reconciliation, gate revalidation.

## S11 — GSP bridge

Corpus/read-only adapter y external evidence refs.

## S12 — Agent Skill/Codex

Skill, examples, handoff workflow.

## S13 — Hardening

Privacy, security, chaos, performance, release.

Codex NO DEBE implementar un slice posterior si el gate contractual del anterior está rojo.

---

# 52. Estructura del repositorio

```text
project-ledger/
├── README.md
├── SPEC.md
├── CHANGELOG.md
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
├── rust-toolchain.toml
├── Cargo.toml
├── deny.toml
│
├── specs/
│   ├── PROJECT_LEDGER_SPEC_v0.1.0.md
│   ├── checksums.txt
│   └── compatibility/
│       ├── gskills-v0.3.md
│       ├── gsp-v0.2.md
│       └── agent-skills.md
│
├── crates/
│   ├── domain/
│   ├── application/
│   ├── canonical-json/
│   ├── schemas/
│   ├── event-store/
│   ├── storage-sqlite/
│   ├── projection-engine/
│   ├── lifecycle/
│   ├── gate-engine/
│   ├── evidence/
│   ├── checkpoint/
│   ├── renderer-markdown/
│   ├── portable-bundle/
│   ├── legacy-import/
│   ├── integration-protocol/
│   ├── transport-stdio/
│   ├── interop-gskills/
│   ├── interop-gsp/
│   ├── interop-radar/
│   ├── cli/
│   └── test-support/
│
├── schemas/
│   └── v0.1/
│       ├── schema-registry.json
│       ├── event-envelope.schema.json
│       ├── command-envelope.schema.json
│       ├── command-result.schema.json
│       ├── project-state.schema.json
│       ├── authority-document.schema.json
│       ├── roadmap.schema.json
│       ├── phase.schema.json
│       ├── gate-definition.schema.json
│       ├── gate-evaluation.schema.json
│       ├── evidence-record.schema.json
│       ├── decision.schema.json
│       ├── blocker.schema.json
│       ├── risk.schema.json
│       ├── next-action.schema.json
│       ├── continuity-checkpoint.schema.json
│       ├── resume-projection.schema.json
│       ├── integration-manifest.schema.json
│       └── portable-export.schema.json
│
├── integrations/
│   ├── sdk/
│   │   ├── schemas/
│   │   ├── examples/
│   │   └── conformance-runner/
│   ├── gskills/
│   ├── gsp/
│   ├── project-radar/
│   └── agent-skills/
│       └── project-ledger-continuity/
│
├── examples/
│   ├── mempilot/
│   ├── gates/
│   ├── projections/
│   ├── legacy-import/
│   └── external-commands/
│
├── benchmark/
│   ├── scenarios/
│   ├── fixtures/
│   └── reports/
│
├── docs/
│   ├── architecture/
│   ├── protocol/
│   ├── privacy/
│   ├── threat-model/
│   ├── ADR/
│   └── release/
│
└── tests/
    ├── contract/
    ├── integration/
    ├── e2e/
    ├── property/
    ├── privacy/
    ├── security/
    ├── chaos/
    └── fuzz/
```

---

# 53. ADR aceptados

| ADR | Decisión | Consecuencia |
|---|---|---|
| ADR-001 | Event sourcing append-only | Estado reconstruible y correcciones explícitas |
| ADR-002 | SQLite/WAL como store canónico | Durabilidad local y transacciones |
| ADR-003 | Markdown derivado | Elimina contradicciones por edición distribuida |
| ADR-004 | CLI/JSON canónicos | Automatización y neutralidad de host |
| ADR-005 | No daemon obligatorio | Cero overhead inactivo |
| ADR-006 | Rust single-binary-first | Distribución local y portabilidad |
| ADR-007 | Projection/RecordSink separados | Sin mutaciones ocultas |
| ADR-008 | Expected version + idempotencia | Concurrencia segura entre agentes |
| ADR-009 | Project Ledger posee lifecycle | Integraciones no cierran fases directamente |
| ADR-010 | G-Skills por provider/sink | Sin DB compartida |
| ADR-011 | GSP exact-contract y read-only | Sin degradación ni autoridad indebida |
| ADR-012 | Agent Skill no autoritativa | Procedimiento separado del motor |
| ADR-013 | Raw evidence off | Privacidad y almacenamiento acotado |
| ADR-014 | Legacy import review-first | No convertir contradicciones en hechos |
| ADR-015 | Project Radar read-only | Portfolio separado de continuidad |

---

# 54. Riesgos y mitigaciones

| ID | Riesgo | Impacto | Mitigación |
|---|---|---:|---|
| R-01 | Event model demasiado complejo | Alto | catálogo mínimo y slices |
| R-02 | Bitácora pierde legibilidad | Medio | renderer y perfiles humanos |
| R-03 | Exceso de registros | Medio | “avance material”, compaction derivada |
| R-04 | Gates demasiado rígidos | Alto | waivers, policies y revisión |
| R-05 | Agentes omiten registrar | Alto | skill, hooks opcionales y audit |
| R-06 | Agentes sobre-registran | Medio | rate/semantic guidance |
| R-07 | Concurrencia entre agentes | Alto | expected version |
| R-08 | Duplicación por timeout | Alto | idempotencia/reconcile |
| R-09 | G-Skills contract drift | Alto | compatibility matrix y conformance |
| R-10 | GSP semánticamente confundido | Crítico | exact-contract, authority isolation |
| R-11 | Import Markdown inventa estado | Crítico | preview/review |
| R-12 | Secretos en evidencia | Crítico | raw-off, redaction, scans |
| R-13 | SQLite corrupta | Alto | backups, integrity check, export |
| R-14 | UI consume esfuerzo prematuro | Medio | CLI-first |
| R-15 | Usuario considera Ledger otra carga manual | Alto | comandos breves y automatización verificable |

## 54.1 Criterios de pivot

Pivotar si:

1. mantener el ledger cuesta más que reanudar manualmente;
2. los gates requieren scripting arbitrario para la mayoría de casos;
3. G-Skills no puede conservar version/digest/idempotencia estable;
4. el Markdown generado resulta menos útil que un checkpoint compacto;
5. los agentes no adoptan la skill ni la CLI;
6. la arquitectura event-sourced no aporta reducción medible de contradicciones.

Pivots permitidos:

```text
full event sourcing → append-only command journal simplificado
mutable RecordSink  → export-only commands
full Markdown       → resume/checkpoint-first
many integrations   → G-Skills + GSP únicamente
```

No pivotar hacia una base compartida ni estado generado por LLM.

---

# 55. Resultado esperado del MVP

Al terminar el MVP, el usuario podrá:

1. inicializar Project Ledger en un repositorio;
2. registrar una spec como autoridad;
3. definir/importar roadmap y gates;
4. registrar progreso y verificaciones;
5. comprobar exactamente por qué una fase sigue abierta;
6. cerrar una fase solo cuando corresponde;
7. reabrirla ante evidencia stale;
8. registrar y superseder decisiones;
9. mantener blockers, riesgos y próxima acción;
10. obtener un resume compacto para Codex;
11. regenerar una bitácora consistente;
12. auditar contradicciones;
13. exportar/importar el ledger;
14. conectar G-Skills en modo read-only;
15. habilitar writes de G-Skills por separado;
16. enlazar evidencia compilada por GSP;
17. usar todo el core sin integraciones externas.

---

# 56. Definición de terminado

Project Ledger v0.1.0 está terminado cuando:

- AC-001…AC-065 pasan;
- los schemas resuelven offline;
- el event store se reconstruye;
- no existen efectos duplicados;
- no se completa una fase indebidamente;
- la bitácora Markdown es determinista;
- el resume es compacto y vigente;
- export/import conserva equivalencia;
- legacy import no eleva claims;
- la projection G-Skills es side-effect-free;
- el RecordSink es idempotente y optimista;
- GSP permanece exact-contract y sin autoridad de lifecycle;
- secret persistence es cero;
- Windows y Linux pasan el core;
- se publican limits, threat model, checksums, SBOM y benchmark report.

## 56.1 Condiciones NO-GO

Bloquean release:

- evento mutable;
- fase completa con gate requerido pendiente;
- gate verificado sin evidencia/waiver válido;
- proyección read-only que muta;
- duplicate external effect;
- blind retry tras unknown outcome;
- stale checkpoint presentado como current;
- identity merge por nombre;
- GSP claim convertido en lifecycle;
- G-Skills con acceso directo a DB;
- secret persistido;
- Markdown manual convertido en estado;
- incompatibilidad spec↔schema no resuelta.

---

# 57. Prompt inicial recomendado para Codex

```text
Implementa Project Ledger a partir de `PROJECT_LEDGER_SPEC_v0.1.0.md`.

REGLAS DE AUTORIDAD

1. Lee completamente la spec antes de editar código.
2. La spec gobierna la implementación.
3. Si spec, schema, fixture y código discrepan, registra el defecto y detén
   esa parte; no inventes una resolución silenciosa.
4. Mantén Project Ledger independiente de G-Skills, GSP y Project Radar.
5. No compartas bases de datos entre productos.
6. No conviertas Markdown en el almacén canónico.
7. No permitas que un comando externo cierre directamente una fase o gate.

ALCANCE DE LA PRIMERA ITERACIÓN

Implementa únicamente Fase 0 / slices S0 y S1:

S0 — Repository skeleton
- Rust workspace;
- crates mínimos necesarios para contracts;
- CI Windows/Linux/macOS;
- fmt, clippy -D warnings, tests, cargo-deny;
- README, LICENSE, CONTRIBUTING, SECURITY;
- docs/ADR;
- estructura schemas/v0.1 y fixtures.

S1 — Contracts
- IDs UUIDv7;
- timestamps RFC3339;
- canonical JSON;
- SHA-256 digests;
- ErrorCode estable;
- EventEnvelope;
- CommandEnvelope;
- CommandResult;
- ProjectState;
- AuthorityDocument;
- Roadmap;
- Phase;
- GateDefinition;
- GateEvaluation;
- EvidenceRecord;
- Decision;
- Blocker;
- Risk;
- NextAction;
- ContinuityCheckpoint;
- ResumeProjection;
- IntegrationManifest;
- PortableExport manifest;
- schema registry offline;
- fixtures positivos y negativos;
- tests de resolución de $ref y digests.

NO IMPLEMENTAR TODAVÍA

- SQLite;
- EventStore;
- projection rebuild;
- Gate Engine;
- command execution;
- Markdown renderer;
- legacy import;
- stdio provider;
- RecordSink;
- G-Skills adapter;
- GSP adapter;
- Agent Skill;
- MCP/UI/daemon;
- red, telemetría, IA o embeddings.

RESTRICCIONES

- Rust estable y portable.
- Sin Python/Node/JVM/Docker en el core.
- Sin `unsafe` salvo ADR y necesidad demostrada.
- Sin crypto propia.
- Sin llamadas de red en tests.
- Schemas cerrados donde corresponda.
- `$ref` resolubles offline.
- APIs públicas documentan invariantes y errores.
- Toda PR cita requirement, AC y slice.
- Mantén una `BITACORA_PROYECTO.md` temporal para el avance de esta
  implementación, pero no la confundas con el producto terminado.

ENTREGABLES

1. Árbol del repositorio.
2. Archivos creados/modificados.
3. Matriz contrato → schema → fixture → test.
4. Comandos ejecutados y resultados.
5. CI verde.
6. Ambigüedades encontradas.
7. TODOs explícitos para S2, sin implementarlos.

DEFINICIÓN DE TERMINADO

- todos los schemas validan contra Draft 2020-12;
- `$id` y `$ref` resuelven offline;
- fixtures positivos pasan;
- fixtures negativos fallan por la razón prevista;
- canonical JSON y digests son deterministas;
- no existe persistencia o integración prematura;
- fmt, clippy, tests y deny están verdes;
- el repositorio queda listo para S2 sin deuda contractual oculta.
```

---

# 58. Ejemplo E2E de uso

```bash
# Inicializar
project-ledger init --title "MemPilot"

# Registrar autoridad
project-ledger authority add \
  --uri referencias/MEMPILOT_SPEC_V0_1_0.md \
  --digest sha256:...

# Importar roadmap
project-ledger roadmap import --file roadmap.yaml

# Iniciar fase
project-ledger phase start phase-2

# Registrar avance
project-ledger record progress \
  --phase phase-2 \
  --summary "Implementados contratos de historial"

# Ejecutar evidencia
project-ledger evidence run \
  --kind TEST_RESULT \
  --gate gate:sqlite-roundtrip \
  -- dotnet test MemPilot.slnx

# Evaluar gate
project-ledger gate evaluate gate:sqlite-roundtrip

# Solicitar cierre; puede rechazar si falta evidencia
project-ledger phase request-close phase-2

# Crear checkpoint
project-ledger checkpoint create --reason session-end

# Entregar contexto a Codex
project-ledger resume --format codex

# Regenerar vista humana
project-ledger render

# Auditar
project-ledger audit
project-ledger verify
```

Resultado conceptual:

```text
Project state: v185 CURRENT
Active phase: phase-2 — Persistencia
Gate sqlite-roundtrip: PENDING
Reason: build evidence missing
Tests: 24 passed, 0 failed
Next action: run build and record BUILD_RESULT
Checkpoint: project-ledger://checkpoints/185
```

---

# 59. Declaración final

Project Ledger v0.1.0 se define como:

> **La fuente canónica y verificable de continuidad operacional de un proyecto, diseñada para sobrevivir cambios de sesión, agentes, herramientas y formatos humanos sin perder autoridad, provenance ni coherencia.**

La arquitectura final es:

```text
Project Ledger
  owns:
    lifecycle · phases · gates · decisions · blockers · risks · checkpoints

G-Skills
  owns:
    repository state · impact · operational memory · guard · context orchestration

GSP
  owns:
    evidence compilation · provenance · contradiction · uncertainty

Project Radar
  owns:
    portfolio presentation for the human user
```

La articulación ocurre mediante:

```text
typed projections
+ explicit commands
+ stable identities
+ digests
+ freshness
+ idempotency
+ optimistic concurrency
+ no shared databases
```

Estado de esta spec:

```text
PROJECT LEDGER v0.1.0 BASELINE DEFINED
CODEX IMPLEMENTATION READY
G-SKILLS v0.3 COMPATIBILITY PROFILE DEFINED
GSP v0.2 EXACT-CONTRACT BOUNDARY PRESERVED
PROJECT RADAR READ-ONLY BOUNDARY DEFINED
PROVIDER-FREE GOLDEN PATH REQUIRED
```
