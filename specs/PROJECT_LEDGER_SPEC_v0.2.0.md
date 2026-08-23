# Project Ledger — Especificación Técnica Integrada v0.2.0

| Campo | Valor |
|---|---|
| Proyecto | **Project Ledger** |
| Definición | Motor local de continuidad verificable para proyectos desarrollados por personas y agentes |
| Forma principal | Binario único en Rust + CLI + proyecciones read-only + `RecordSink` explícito + exportaciones portables |
| Estado | `IMPLEMENTATION BASELINE` / `CODEX IMPLEMENTATION READY` / `EXTERNAL ADAPTERS LOCK-GATED` |
| Versión | **0.2.0** |
| Revisión documental | **GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT** — 2026-08-21 |
| Baseline previamente congelada por G-Skills | SHA-256 `e7acc4964089824d1a19d32752bc935f805e3632d46d2ef4efa8edcec845954b` — antecedente de esta revisión documental |
| Fecha | **2026-08-16** |
| Idioma canónico | Español |
| Plataforma inicial | Windows 11 x86_64; core portable a Linux y macOS |
| Persistencia | SQLite + WAL; event store append-only; proyecciones reconstruibles |
| Interfaz canónica | CLI y JSON |
| Integraciones de primera clase | **G-Skills v0.3**, **GSP Internals v0.2**, **Agent Skills/Codex**, **Project Radar v0.1+** |
| Perfil Radar | `project-ledger-radar-artifact/0.1.1` |
| IA obligatoria | No |
| Red obligatoria | No |
| Daemon obligatorio | No |
| Licencia recomendada | Apache-2.0 |
| Antecedente preservado | `PROJECT_LEDGER_SPEC_v0.1.0.md` |
| SHA-256 del antecedente | `772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865` |
| Archivo recomendado | `PROJECT_LEDGER_SPEC_v0.2.0.md` |

## Mapa del documento

1. [Estado, preservación, autoridad y requisitos](#0-estado-preservación-autoridad-y-requisitos)
2. [Resumen, problema y tesis](#1-resumen-ejecutivo)
3. [Objetivos, usuarios y alcance](#4-objetivos)
4. [Arquitectura y modelo de autoridad](#9-arquitectura-de-referencia)
5. [Modelo de dominio, eventos y comandos](#14-modelo-de-dominio)
6. [Gates, evidencia, decisiones y checkpoints](#21-motor-de-gates)
7. [CLI, renderizado y migración de bitácoras](#27-cli-canónica)
8. [Protocolos e integraciones](#32-plano-de-integración)
9. [Articulación Project Radar](#37-integración-con-project-radar)
10. [Persistencia, seguridad y operación](#39-persistencia)
11. [Testing, benchmark y aceptación](#45-estrategia-de-testing)
12. [Roadmap, slices y estructura del repositorio](#50-roadmap-de-implementación)
13. [Definición de terminado y prompt para Codex](#56-definición-de-terminado)
14. [Changelog v0.1.0 a v0.2.0](#60-changelog-normativo-v010-a-v020)

# 0. Estado, preservación, autoridad y requisitos

## 0.1 Regla de evolución

Esta v0.2.0 sustituye normativamente a v0.1.0 para toda implementación futura. El antecedente permanece inmutable:

```text
PROJECT_LEDGER_SPEC_v0.1.0.md
SHA-256 772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865
```

**REQ-REL-001:** la implementación v0.2 NO DEBE editar, reformatear, renombrar ni sobrescribir el antecedente.

**REQ-REL-002:** toda migración de contratos v0.1→v0.2 DEBE ser explícita, versionada, testeada y reversible; no se reinterpretan silenciosamente objetos v0.1 como v0.2.

## 0.2 Propósito normativo

Esta especificación gobierna:

- continuidad del proyecto;
- autoridad documental;
- lifecycle, roadmap, fases y gates;
- avances, evidencia, decisiones, blockers, riesgos y próximas acciones;
- event store, comandos, proyecciones y checkpoints;
- bitácora Markdown derivada;
- CLI y JSON;
- protocolos con G-Skills, GSP, Agent Skills/Codex y Project Radar;
- schemas, seguridad, privacidad, operación, benchmark y release.

La implementación NO DEBE improvisar semánticas. Si spec, schema, fixture y código discrepan, el defecto bloquea esa parte hasta resolución.

## 0.3 Fuentes de alineación y locks

| Fuente | Rol | Estado |
|---|---|---|
| Project Ledger v0.1.0 | antecedente | SHA-256 fijado |
| G-Skills v0.3.0 | provider/sink profile | revisión `E2E-CONSOLIDATED-1`, SHA-256 `8ddbb2a9c23bf7355a1e577d93a0c541b0986df7416ab64069aea16295376928`; su lock actual apunta al antecedente Ledger `e7acc496...`, por lo que el lock recíproco DEBE refrescarse antes de certificar esta revisión |
| GSP Internals v0.2 | frontera exact-contract | lock de schemas/fixtures obligatorio antes de bridge certificado |
| Project Radar v0.1.0 | producto humano local-first | SHA-256 de la copia auditada: `277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b` |
| Perfil Radar-Ledger v0.1.1 | contrato de artifact exchange | parte normativa de esta release; supersede v0.1.0 por normalización terminológica |

**REQ-INT-001:** cada adapter distribuido DEBE incluir `upstream-lock.json` con versión, digest de spec/schema registry, fixtures y compatibility range.

Los locks cruzados se mantienen como artefactos externos para evitar dependencias criptográficas recursivas. Esta revisión fija el source digest de G-Skills, pero **no** declara actualizado el lock recíproco distribuido por el proyecto G-Skills: ese refresh es un gate de integración posterior, no un requisito para implementar el core de Ledger.

**REQ-INT-002:** superar conformance demuestra compatibilidad de protocolo, no autoridad, seguridad total ni corrección científica del producto externo.

## 0.4 Orden de autoridad

1. Esta spec v0.2.0.
2. JSON Schemas v0.2 materializados y schema registry.
3. ADR aceptados y migrations.
4. Políticas locales del proyecto.
5. Eventos canónicos persistidos.
6. Proyecciones reconstruidas.
7. Checkpoints y artefactos generados.
8. Salidas de adapters/agentes/LLM/importadores como datos tipados con autoridad limitada.

Contradicción spec↔schema bloquea release; no se resuelve por “lo que compile”.

## 0.5 Palabras normativas

- **DEBE / MUST:** requisito obligatorio.
- **NO DEBE / MUST NOT:** prohibición obligatoria.
- **DEBERÍA / SHOULD:** recomendación fuerte.
- **PUEDE / MAY:** capacidad opcional compatible.
- **CANÓNICO:** fuente autoritativa dentro del dominio declarado.
- **DERIVADO:** reconstruible desde fuentes canónicas.
- **VERIFICADO:** sustentado por una evaluación válida.
- **ASEVERADO:** declarado sin evidencia suficiente.
- **CURRENT:** vigente según versión, digest, identity y freshness explícitos.

## 0.6 Familias de requisitos

```text
PRD  producto y alcance
REL  preservación, compatibilidad y release
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
CON  concurrencia, idempotencia y reconciliación
SEC  seguridad
PRV  privacidad
OPS  operación
TST  pruebas
PER  rendimiento
MIG  migración/importación
```

## 0.7 Requisitos críticos transversales

**REQ-PRD-001:** el golden path DEBE funcionar sin G-Skills, GSP, Radar, IA, red ni daemon.

**REQ-EVT-001:** los eventos canónicos son append-only y encadenados por digest de forma obligatoria.

**REQ-EVT-002:** `event_type + payload` DEBE validar contra una unión discriminada cerrada; no existen payloads libres.

**REQ-CMD-001:** toda mutación usa `CommandEnvelope`; una query no puede producir eventos.

**REQ-CMD-002:** `command_type + payload` DEBE validar contra una unión discriminada cerrada.

**REQ-CON-001:** toda escritura externa requiere expected version e idempotency key.

**REQ-CON-002:** unknown outcome pertenece a un registro de submission/reconciliation, no al resultado de dominio del servidor.

**REQ-GAT-001:** solo Gate Engine puede verificar/rechazar/revalidar un gate.

**REQ-GAT-002:** solo Phase Engine puede completar o reabrir una fase.

**REQ-CHK-001:** un checkpoint separa versión capturada de versión del evento que lo registra.

**REQ-INT-003:** productos comparten IDs, digests, schemas, projections y commands; no comparten tablas, WAL, locks ni migraciones privadas.

**REQ-RAD-001:** la integración Radar es read-only desde Ledger en el MVP.

**REQ-RAD-002:** Radar consume un overlay o import con preview; Ledger nunca escribe directamente en IndexedDB.

**REQ-RAD-003:** campos Radar-owned no se sobrescriben silenciosamente.

**REQ-RAD-004:** omisión en un export no significa delete/archive.

**REQ-RAD-005:** misma versión con digest distinto produce conflicto, no last-write-wins.

**REQ-PRV-001:** raw terminal output, transcripts, reasoning privado y secretos no se persisten por defecto.

**REQ-SEC-001:** configuración de un repositorio no confiable no puede habilitar adapters, network, authority o writes.

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
3. Definir fases, kinds y gates.
4. Registrar avances materiales y notas tipadas.
5. Ejecutar o importar verificaciones.
6. Evaluar gates determinísticamente mediante `GateEvaluation`.
7. Completar/reabrir fases bajo reglas explícitas.
8. Registrar decisiones y supersesiones.
9. Gestionar blockers, riesgos y próximas acciones.
10. Crear checkpoints con semántica de versión inequívoca.
11. Detectar contradicciones, staleness y divergencia.
12. Regenerar `BITACORA_PROYECTO.md`.
13. Exportar/importar un bundle portable fail-closed ante ramas divergentes.
14. Exponer continuidad read-only para G-Skills.
15. Aceptar comandos G-Skills explícitos como `RecordSink`.
16. Articularse con GSP conservando contratos exactos.
17. Distribuir una Agent Skill versionada para Codex y otros agentes.
18. Exportar una proyección Project Radar schema-first, idempotente y sin overwrite silencioso.
19. Permitir que múltiples ledgers se agreguen en un `RadarSyncEnvelope` sin convertir Ledger en portfolio manager.
20. Preservar provenance upstream cuando Radar reexpone un dato Ledger hacia G-Skills.
21. Funcionar sin IA, red, cuenta, backend ni daemon.
22. Mantener auditabilidad completa de transiciones e integraciones.

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

Un agente implementa un repositorio SQLite y registra resumen, fase, artefactos, commits y claims.

### UC-03 — Verificación ejecutada

Project Ledger ejecuta `cargo test`, captura exit code, duración, métricas y digests permitidos, y crea evidencia.

### UC-04 — Gate incompleto

Build y tests pasan, pero falta E2E. `GateEvaluation.result=INCOMPLETE`; el gate permanece pendiente.

### UC-05 — Fase cerrable

Todos los gates requeridos están verificados/waived y no hay conflictos incompatibles. El Phase Engine produce `PHASE_COMPLETED`.

### UC-06 — Reapertura

Un cambio invalida evidencia de un gate. El Gate Engine marca revalidación y el Phase Engine puede reabrir la fase.

### UC-07 — Reanudación

Una nueva sesión ejecuta `project-ledger resume --format codex` y recibe contexto compacto, vigente y con provenance.

### UC-08 — Contradicción heredada

Se importa una bitácora antigua. El importador produce candidatos/conflictos; nunca estado verificado automático.

### UC-09 — G-Skills consulta continuidad

G-Skills solicita `ProjectContinuityProjection`; event count y state version no cambian.

### UC-10 — G-Skills registra resultado

G-Skills envía `RECORD_VERIFICATION_RESULT` con expected version e idempotency key. Ledger decide eventos y gates.

### UC-11 — GSP aporta evidencia

Un bridge construye `EvidenceBundle` exacto. Ledger enlaza el resultado por referencia, sin convertirlo en lifecycle.

### UC-12 — Project Radar importa un overlay

Ledger genera `RadarSyncEnvelope`; Radar valida, previsualiza, confirma identidad y aplica un overlay sin tocar prioridad, tags, notas, ideas, chats ni tareas.

### UC-13 — Radar recibe un export stale

El export tiene una state version inferior. Radar lo ignora como stale y conserva el overlay current.

### UC-14 — Divergencia Radar

Misma state version con digest distinto abre `SOURCE_DIVERGENCE`; no se elige el último archivo por timestamp.

### UC-15 — Radar reexpone portfolio a G-Skills

Radar conserva `upstream_ledger_ref/state_version/digest`; G-Skills no lo considera una segunda fuente canónica independiente.

### UC-16 — Checkpoint sin ambigüedad

El checkpoint captura state N, se registra mediante evento N+1 y declara ambos valores.

### UC-17 — Unknown outcome

El cliente pierde la respuesta tras submit. `ExternalSubmissionRecord=OUTCOME_UNKNOWN`; consulta por idempotency key y reconcilia, sin reintento ciego.

# 8. Forma de producto

## 8.1 Recomendación

La forma principal DEBE ser:

> **Un único binario local `project-ledger`, escrito en Rust, con CLI canónica, core embebible, servidor stdio opcional y artifact exchange portable.**

| Superficie | Responsabilidad | Prioridad |
|---|---|---:|
| CLI | Operación humana, scripting, Codex y CI | Canónica |
| JSON stdout | Contrato automatizable | Canónica |
| Provider stdio JSON-RPC | Proyecciones read-only | MVP |
| RecordSink stdio JSON-RPC | Comandos externos explícitos | MVP |
| Radar artifact exchange | Overlay humano de portfolio | MVP opcional de primera clase |
| Agent Skill | Procedimiento para agentes | MVP |
| Export/import bundle | Portabilidad y backup | MVP |
| GSP sidecar | EvidenceBundle/CompilationResult exact-contract | MVP opcional |
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

PORTFOLIO
MINIMAL + Radar artifact exchange

EVIDENCE
MINIMAL + GSP exact-contract bridge

COMPLETE_MVP
AGENT + ORCHESTRATED + PORTFOLIO + EVIDENCE
```

Todos los perfiles externos son opcionales. `PORTFOLIO` no requiere backend, daemon ni acceso al store de Radar.

## 8.3 Golden path

El core debe completar init→gates→checkpoint→render→export/import con:

```text
gskills.enabled = false
gsp.enabled = false
project_radar.enabled = false
ai.enabled = false
network = disabled
daemon = disabled
```

## 8.4 Golden path modular

```text
Ledger current state
→ G-Skills continuity projection
→ optional GSP evidence reference
→ explicit G-Skills write-back
→ Radar project overlay export
→ independent failure isolation
```

# 9. Arquitectura de referencia

```text
┌─────────────────────────────────────────────────────────────────────┐
│ CLI · JSON · Agent Skill · stdio Provider · stdio RecordSink       │
│ Radar Artifact Export · Portable Bundle                            │
└──────────────────────────────┬──────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ APPLICATION LAYER                                                   │
│ CommandBus · QueryBus · UnitOfWork · Authorization · Submission    │
└───────────────┬─────────────────────────────┬───────────────────────┘
                ▼                             ▼
┌──────────────────────────────┐  ┌───────────────────────────────────┐
│ DOMAIN CORE                  │  │ INTEGRATION PLANE                 │
│ Project aggregate            │  │ G-Skills profile + upstream lock │
│ Phase/Gate engines           │  │ GSP exact-contract bridge        │
│ Decisions/Blockers/Risks     │  │ Radar overlay/export profile     │
│ Evidence rules               │  │ Agent Skill/Codex adapter        │
│ Checkpoint builder           │  │ Generic SDK + conformance        │
└───────────────┬──────────────┘  └────────────────┬──────────────────┘
                ▼                                  │
┌──────────────────────────────────────────────────┴──────────────────┐
│ EVENT STORE + PROJECTIONS                                           │
│ SQLite/WAL · mandatory hash chain · dedup · derived snapshots       │
└──────────────────────────────┬───────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│ DERIVED OUTPUTS                                                     │
│ ProjectState · Resume · Markdown · Audit · PortableExport           │
│ LedgerRadarProjectProjection · RadarSyncEnvelope                    │
└─────────────────────────────────────────────────────────────────────┘
```

## 9.1 Separación de crates

- `domain` no depende de SQLite, CLI, G-Skills, GSP, Radar ni Markdown.
- `application` depende del dominio y puertos.
- `storage-sqlite` implementa persistencia y constraints de append-only.
- `renderer-markdown` consume proyecciones.
- `integration-protocol` contiene envelopes genéricos.
- `interop-gskills`, `interop-gsp` e `interop-radar` dependen hacia adentro del protocolo, nunca al revés.
- `radar-export` solo consume `ProjectState`/projection ports; no abre IndexedDB.
- adapters externos no modifican tipos privados del core.

## 9.2 Catálogo local derivado

Un índice de usuario opcional descubre ledgers conocidos para exportar varios projects a Radar:

```text
<user-data>/project-ledger/catalog.json
```

El catálogo contiene refs/rutas minimizadas y digests; es reconstruible y no es autoridad de continuidad.

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

## 10.2 Dominios externos

```text
repository.structure       → G-Skills Repository State
repository.impact          → G-Skills
code.guard                 → G-Skills policies/constraints
operational.knowledge      → G-Skills
evidence.compilation      → GSP
portfolio.presentation     → Project Radar
portfolio.priority/tags    → Project Radar
portfolio.ideas/chats/tasks/artifacts → Project Radar
organization.policy        → owner humano/organizacional
agent.assertion            → agente, no canónico
```

## 10.3 Regla de autoridad externa

Una fuente externa aporta evidencia, señales, referencias, projections o commands. No puede declarar por sí sola cierre de gate/fase.

## 10.4 Ownership Ledger↔Radar

| Dato | Ledger | Radar | Regla |
|---|---:|---:|---|
| Fase, gates, decisions, checkpoint | Canónico | No | Overlay Ledger |
| Blockers/riesgos técnicos | Canónico en continuidad | Puede tener registros humanos | No fusionar por título |
| Priority, tags, notes | No | Canónico | Nunca overwrite |
| Ideas, chats, tasks, artifacts | No | Canónico | Nunca enviados a Ledger |
| Title | Canonical técnico | Alias/display | Conservar ambos |
| Status | Lifecycle canónico | Presentación canónica | Ledger emite suggestion |
| Next action | Continuidad | Tareas humanas | Mostrar separadas |
| Last activity | Material Ledger | Actividad local Radar | Dos timestamps o policy explícita |

## 10.5 Transitive provenance

Si Radar reexpone a G-Skills un item cuyo estado proviene de Ledger, debe conservar:

```text
upstream_system = project-ledger
upstream_project_ref
upstream_state_version
upstream_state_digest
upstream_projection_digest
```

Esto evita doble conteo epistemológico.

# 11. Invariantes del corazón

### I-01 — Event store append-only

Los eventos canónicos no se actualizan ni eliminan in-place.

### I-02 — Estado derivado

`ProjectState`, `ResumeProjection`, Radar projections y Markdown se reconstruyen desde eventos.

### I-03 — Markdown no canónico

Editar `BITACORA_PROYECTO.md` no cambia estado.

### I-04 — Comando explícito

Toda mutación atraviesa `CommandEnvelope`.

### I-05 — Proyección sin efectos laterales

Una query no cambia versión, eventos, evidencia, checkpoints ni mappings.

### I-06 — Expected version

Toda escritura externa/concurrente declara expected version.

### I-07 — Idempotencia

Misma key + mismo payload produce como máximo un efecto.

### I-08 — Gate owner

Solo Gate Engine produce estados evaluados de gate.

### I-09 — Phase owner

Solo Phase Engine completa/reabre fases.

### I-10 — Tests no equivalen a fase

Resultado verde es evidencia, no lifecycle.

### I-11 — Evidence provenance

Toda evidencia aceptada tiene fuente, tiempo, digest, scope y validation state.

### I-12 — Aseverado no es verificado

Claims manuales sin evidencia son `ASSERTED`.

### I-13 — Supersesión explícita

Decisiones/documentos antiguos permanecen y se marcan superseded.

### I-14 — Blocker resuelto no desaparece

Cambia de estado por evento y conserva timeline.

### I-15 — Freshness visible

Stale/conflicted nunca se renderiza current.

### I-16 — Identidad no nominal

Nombre igual no confirma project/repository mapping.

### I-17 — No base compartida

Ninguna integración abre SQLite/IndexedDB privados ajenos.

### I-18 — No autoridad de LLM

IA nunca produce transición verificada por sí sola.

### I-19 — No raw por defecto

Outputs completos, transcripts, reasoning y secretos no persisten por defecto.

### I-20 — Rebuild equivalence

Reconstrucción produce estado semánticamente equivalente.

### I-21 — Schema closure

Todo payload durable y externo participa en un schema cerrado y registry offline.

### I-22 — Checkpoint no circular

`captured_state_version` y `checkpoint_event_version` son distintos y explícitos.

### I-23 — Project status disciplinado

`ACTIVE/BLOCKED/COMPLETED` se derivan; humanos solo pausan, reanudan o archivan mediante commands definidos.

### I-24 — Radar overlay

Import Radar no sobrescribe fields Radar-owned ni se convierte en store canónico.

### I-25 — Omission is not deletion

Ausencia de un project/field en un export no elimina ni limpia estado downstream.

### I-26 — Unknown outcome separado

La incertidumbre de transporte se registra en `ExternalSubmissionRecord`, no como resultado servidor fabricado.

### I-27 — Mandatory event chain

Todos los eventos canónicos incluyen `previous_event_digest`; solo genesis usa `null` con regla explícita.

### I-28 — Upstream contracts locked

Un adapter no se declara certified sin digests y conformance de contratos upstream exactos.

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

## 12.4 Registro de semántica de arrays

El schema registry declara para cada array una de estas clases:

```text
SEQUENCE          el orden es semántico y se preserva
STABLE_SET        se ordena por una key canónica declarada
MULTISET          se ordena por digest de elemento y conserva multiplicidad
```

Ejemplos normativos:

| Campo | Clase | Clave/razón |
|---|---|---|
| `roadmap.phase_order` | `SEQUENCE` | orden de ejecución |
| `next_actions.ordered` | `SEQUENCE` | prioridad derivada |
| `active_phase_refs` | `STABLE_SET` | `phase_id` |
| `evidence_refs` | `STABLE_SET` | URI/ID canónico |
| `authority_documents` | `STABLE_SET` | `authority_rank`, luego `document_id` |
| `attention_signals` | `STABLE_SET` | `code`, luego `subject_ref` |

Un array sin clase declarada bloquea el digest fixture y el release del schema afectado.

---

# 13. Estado agregado del proyecto

```yaml
schema_version: 0.2.0
project_state_id: state:project-01H...
project:
  project_id: project:01H...
  title: MemPilot
  lifecycle_status: ACTIVE
  state_version: 184
  material_state_version: 183

authority_documents:
  - document_id: authority:spec
    uri: workspace://referencias/MEMPILOT_SPEC_V0_1_0.md
    digest: sha256:...
    status: CURRENT
    authority_rank: 1

roadmap:
  roadmap_id: roadmap:main
  execution_model: SEQUENTIAL
  phase_order: [phase-0, phase-1, phase-2, phase-3]

active_phase_refs: [phase-2]
primary_active_phase_ref: phase-2
completed_phase_refs: [phase-0, phase-1]

gates:
  pending: [gate:sqlite-roundtrip]
  verified: [gate:bootstrap, gate:collector]
  revalidation_required: []

decisions:
  current: [decision:sqlite, decision:retention-seven-days]

blockers:
  open: []

risks:
  open: [risk:sqlite-locking]

next_actions:
  ordered: [action:implement-sqlite-repository]
  preferred_ref: action:implement-sqlite-repository

checkpoint_ref: checkpoint:184
last_material_event_ref: event:701
last_material_activity_at: "2026-08-16T16:05:00-04:00"
open_conflict_refs: []
updated_at: "2026-08-16T16:05:01-04:00"
state_digest: sha256:...
```

El estado es derivado. `state_version` incluye eventos administrativos como checkpoint; `material_state_version` apunta al último evento que cambió continuidad material.

En el MVP:

```text
len(active_phase_refs) <= 1
```

El shape de array evita una migración destructiva futura, pero parallel roadmaps permanecen fuera del MVP.

# 14. Modelo de dominio

## 14.1 `Project`

Campos:

```text
project_id
title
summary
lifecycle_status
created_at
timezone
authority_policy
storage_profile
repository_bindings[]
paused_reason
archived_at
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

Reglas:

- `INITIALIZING` tras creación y antes de autoridad/roadmap suficientes;
- `ACTIVE` se deriva al iniciar una fase válida;
- `BLOCKED` se deriva de blocker/phase con scope bloqueante;
- `COMPLETED` se deriva de roadmap completion policy;
- `PAUSED` y `ARCHIVED` requieren commands humanos explícitos;
- resolver el último blocker restaura el estado derivado previo, salvo PAUSED/ARCHIVED.

No existe `SET_PROJECT_STATUS` genérico.

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

`REFRESH_AUTHORITY_DOCUMENT_DIGEST` vuelve a observar el artefacto. Si el digest coincide, solo registra la verificación. Si difiere, NO reemplaza el digest canónico: emite `AUTHORITY_DOCUMENT_DIGEST_MISMATCH` y abre conflicto. Un cambio material se registra como una nueva `AuthorityDocument.version` y se enlaza mediante `supersedes_ref`.

## 14.3 `Roadmap`

```text
roadmap_id
title
execution_model          # SEQUENTIAL en MVP
phase_order[]
completion_policy
version
source_document_ref
```

`PARALLEL_DAG` queda reservado para post-MVP y no valida en el perfil default v0.2.

## 14.4 `Phase`

```text
phase_id
title
ordinal
kind
status
required_gate_refs[]
optional_gate_refs[]
started_at
completed_at
reopened_at
```

`PhaseKind`:

```text
DISCOVERY
FORMALIZATION
IMPLEMENTATION
TESTING
AUDIT
RELEASE
MAINTENANCE
OTHER
```

Estados:

```text
NOT_STARTED
ACTIVE
BLOCKED
READY_TO_CLOSE
COMPLETED
REVALIDATION_REQUIRED
SKIPPED
```

`PHASE_REOPENED` es un evento de transición, no un estado durable. Su resultado es `ACTIVE` cuando la reapertura es explícita y admisible, o `REVALIDATION_REQUIRED` cuando deriva de evidencia/gates/autoridad obsoletos.

## 14.5 `Gate`

```text
gate_id
title
scope
status
definition_version
definition_digest
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
domain
scope
context
decision
rationale
alternatives[]
status
version
authority_ref
evidence_refs[]
supersedes_ref
created_at
accepted_at
provenance_ref
digest
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

Representa avance material, no cada edición:

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
influence
occurred_at
recorded_at
provenance_ref
digest
```

## 14.8 `Blocker`

```text
blocker_id
title
description
scope_kind             # PROJECT | PHASE | GATE | DECISION | REPOSITORY | EXTERNAL
scope_ref
severity               # LOW | MEDIUM | HIGH | CRITICAL
status
source_kind
source_ref
owner_ref
evidence_refs[]
opened_at
updated_at
resolved_at
resolution_summary
provenance_ref
digest
```

Estados:

```text
OPEN
MITIGATED
RESOLVED
ACCEPTED
CANCELLED
```

`MITIGATED` no equivale a resuelto. `ACCEPTED` requiere actor, razón, review condition y authority/policy.

## 14.9 `Risk`

```text
risk_id
title
description
scope_kind
scope_ref
severity               # LOW | MEDIUM | HIGH | CRITICAL
likelihood_class        # RARE | POSSIBLE | LIKELY | UNKNOWN
status
owner_ref
trigger_conditions[]
mitigation_actions[]
evidence_refs[]
opened_at
updated_at
closed_at
provenance_ref
digest
```

Estados:

```text
OPEN
MONITORED
MITIGATED
REALIZED
CLOSED
```

Un riesgo no bloquea por sí solo. Solo una policy explícita puede derivar un blocker a partir de severidad y condición.

## 14.10 `NextAction`

```text
next_action_id
title
description
phase_ref
gate_ref
classification
status
authority_ref
precondition_refs[]
blocked_by_refs[]
owner_ref
created_at
updated_at
completed_at
provenance_ref
digest
```

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

Una acción `AUTHORIZED` enlaza autoridad verificable. `preferred_ref` se deriva por policy, no por orden accidental de inserción.

## 14.11 `Checkpoint`

Captura refs/digests compactos; no copia el event log.

## 14.12 `Session`

```text
session_id
actor
host
started_at
ended_at
starting_checkpoint_ref
ending_checkpoint_ref
```

## 14.13 `ConflictRecord`

```text
conflict_id
kind
severity
domain
subject_refs[]
source_refs[]
status
required_action
opened_at
resolved_at
resolution_ref
```

## 14.14 `ClaimRecord`

```text
claim_id
statement
classification      # ASSERTED | OBSERVED | VERIFIED | REJECTED | STALE | UNKNOWN
subject_refs[]
evidence_refs[]
authority_ref
```

## 14.15 `ArtifactReference`

```text
artifact_ref_id
name
kind                    # SPEC | AUDIT | BUILD | REPORT | PACKAGE | REPOSITORY | OTHER
uri
digest
version
sensitivity
source_ref
created_at
provenance_ref
```

Es una referencia; Ledger no copia binarios pesados por defecto. URI y path se normalizan y minimizan antes de exportar.

## 14.16 `RepositoryBinding`

```text
repository_binding_id
repository_ref
workspace_root_ref
identity_method         # EXPLICIT_ID | REMOTE_DIGEST | SIGNED_BINDING | USER_CONFIRMED
remote_identity_digest
current_snapshot_ref
current_snapshot_digest
status                  # CURRENT | STALE | UNAVAILABLE | REVOKED | CONFLICTED
confirmed_by
created_at
updated_at
provenance_ref
digest
```

Nombre de carpeta o basename no confirma identidad. El algoritmo de `remote_identity_digest` normaliza provider/host/owner/repository sin credenciales y se versiona en el schema.

## 14.17 `UnknownRecord`

```text
unknown_id
subject_kind
subject_ref
field_path
statement
criticality             # LOW | MEDIUM | HIGH | MATERIAL
status                  # OPEN | RESOLVED | ACCEPTED_UNKNOWN | SUPERSEDED
blocks[]
required_evidence[]
evidence_refs[]
opened_at
resolved_at
resolution_ref
provenance_ref
digest
```

Un unknown material no se convierte en `false`, `no blocker` ni `gate satisfied`.

## 14.18 `ProjectCatalogEntry` derivado

Solo para discovery local y Radar multi-project export:

```text
project_ref
canonical_title
ledger_location_ref
last_seen_state_version
last_seen_state_digest
```

No es parte del aggregate ni autoridad de lifecycle.

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
→ update derived projections
→ store CommandResult
→ optional render/export
```

## 15.2 `EventEnvelope`

```yaml
schema_version: 0.2.0
event_id: event:0198...
project_ref: project:01H...
stream_id: project:01H...
stream_version: 185
event_type: VERIFICATION_RECORDED
event_schema_ref: urn:project-ledger:event:verification-recorded:0.2.0
payload: {}
actor: {}
causation:
  command_ref: command:0198
  command_digest: sha256:...
  correlation_ref: session:44
occurred_at: "2026-08-16T16:05:01-04:00"
recorded_at: "2026-08-16T16:05:01-04:00"
previous_event_digest: sha256:...   # null solo en genesis
payload_digest: sha256:...
event_digest: sha256:...
```

`event_type` discrimina una unión cerrada en `event-payload.schema.json`. Todo tipo tiene `$defs`/schema propio y `additionalProperties: false`.

## 15.3 Catálogo inicial de eventos

### Proyecto/autoridad/roadmap

```text
PROJECT_CREATED
PROJECT_METADATA_UPDATED
PROJECT_PAUSED
PROJECT_RESUMED
PROJECT_ARCHIVED
PROJECT_STATUS_DERIVED
AUTHORITY_DOCUMENT_REGISTERED
AUTHORITY_DOCUMENT_DIGEST_REFRESHED
AUTHORITY_DOCUMENT_SUPERSEDED
AUTHORITY_DOCUMENT_MARKED_UNAVAILABLE
AUTHORITY_DOCUMENT_DIGEST_MISMATCH
REPOSITORY_BOUND
REPOSITORY_BINDING_REFRESHED
REPOSITORY_BINDING_REVOKED
ROADMAP_DEFINED
ROADMAP_REVISED
```

### Fases/gates

```text
PHASE_STARTED
PHASE_BLOCKED
PHASE_MARKED_READY_TO_CLOSE
PHASE_COMPLETED
PHASE_REOPENED
PHASE_SKIPPED
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

### Trabajo/evidencia/notas

```text
WORK_PROGRESS_RECORDED
NOTE_RECORDED
ARTIFACT_REFERENCE_ADDED
CLAIM_RECORDED
CLAIM_STATUS_CHANGED
UNKNOWN_RECORDED
UNKNOWN_RESOLVED
VERIFICATION_RECORDED
EXTERNAL_EVIDENCE_LINKED
EXTERNAL_EVIDENCE_UNLINKED
EVIDENCE_INVALIDATED
```

### Decisiones/blockers/risks/actions

```text
DECISION_PROPOSED
DECISION_ACCEPTED
DECISION_SUPERSEDED
DECISION_REJECTED
DECISION_REVALIDATION_REQUIRED
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

### Continuidad/import/conflictos

```text
CHECKPOINT_CREATED
CHECKPOINT_INVALIDATED
SESSION_STARTED
SESSION_ENDED
LEGACY_IMPORT_REVIEWED
CONFLICT_OPENED
CONFLICT_RESOLVED
EXTERNAL_MAPPING_CONFIRMED
EXTERNAL_MAPPING_REVOKED
EVENT_CORRECTION_RECORDED
```

## 15.4 Matriz command→event cerrada

La release materializa `schemas/v0.2/command-event-matrix.json`. Cada comando declara:

```text
required_events[]       # al menos estos eventos si status=ACCEPTED
possible_events[]       # únicamente estos eventos adicionales
forbidden_events[]      # transiciones que el command handler no puede emitir
engine_owner            # command handler | Gate Engine | Phase Engine | Project Status Engine
atomicity_group         # eventos que deben persistirse en la misma transacción
```

La matriz normativa inicial es:

| Command | Eventos requeridos | Eventos derivados posibles | Owner de la derivación |
|---|---|---|---|
| `CREATE_PROJECT` | `PROJECT_CREATED` | `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `UPDATE_PROJECT_METADATA` | `PROJECT_METADATA_UPDATED` | — | command handler |
| `PAUSE_PROJECT` | `PROJECT_PAUSED` | `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `RESUME_PROJECT` | `PROJECT_RESUMED` | `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `ARCHIVE_PROJECT` | `PROJECT_ARCHIVED` | `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `REGISTER_AUTHORITY_DOCUMENT` | `AUTHORITY_DOCUMENT_REGISTERED` | `DECISION_REVALIDATION_REQUIRED`, `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | dependency/freshness engines |
| `REFRESH_AUTHORITY_DOCUMENT_DIGEST` | `AUTHORITY_DOCUMENT_DIGEST_REFRESHED`, `AUTHORITY_DOCUMENT_MARKED_UNAVAILABLE` **o** `AUTHORITY_DOCUMENT_DIGEST_MISMATCH` | `CONFLICT_OPENED`, `DECISION_REVALIDATION_REQUIRED`, `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | authority/dependency/freshness engines |
| `SUPERSEDE_AUTHORITY_DOCUMENT` | `AUTHORITY_DOCUMENT_SUPERSEDED` | `DECISION_REVALIDATION_REQUIRED`, `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | dependency/freshness engines |
| `BIND_REPOSITORY` | `REPOSITORY_BOUND` | `CONFLICT_OPENED`, `CHECKPOINT_INVALIDATED` | repository identity/freshness engines |
| `REFRESH_REPOSITORY_BINDING` | `REPOSITORY_BINDING_REFRESHED` | `CONFLICT_OPENED`, `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | repository identity/dependency/freshness engines |
| `REVOKE_REPOSITORY_BINDING` | `REPOSITORY_BINDING_REVOKED` | `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | repository identity/dependency/freshness engines |
| `DEFINE_ROADMAP` | `ROADMAP_DEFINED` | `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `REVISE_ROADMAP` | `ROADMAP_REVISED` | `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED`, `PROJECT_STATUS_DERIVED` | dependency/freshness + Project Status engines |
| `START_PHASE` | `PHASE_STARTED` | `PROJECT_STATUS_DERIVED` | Phase + Project Status engines |
| `REQUEST_PHASE_COMPLETION` | `PHASE_MARKED_READY_TO_CLOSE` | `GATE_EVALUATION_REQUESTED`, `GATE_EVALUATED`, `GATE_VERIFIED`, `GATE_REJECTED`, `PHASE_COMPLETED`, `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED` | Gate Engine → Phase Engine → Project Status Engine |
| `REOPEN_PHASE` | `PHASE_REOPENED` | `CHECKPOINT_INVALIDATED`, `PROJECT_STATUS_DERIVED` | Phase + freshness + Project Status engines |
| `SKIP_PHASE` | `PHASE_SKIPPED` | `PROJECT_STATUS_DERIVED` | Phase + Project Status engines |
| `DEFINE_GATE` | `GATE_DEFINED` | `CHECKPOINT_INVALIDATED` | Gate/freshness engines |
| `REQUEST_GATE_EVALUATION` | `GATE_EVALUATION_REQUESTED`, `GATE_EVALUATED` | `GATE_VERIFIED`, `GATE_REJECTED`, `GATE_MARKED_STALE`, `PHASE_COMPLETED`, `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED` | Gate Engine → Phase Engine → Project Status Engine |
| `REQUEST_GATE_REVALIDATION` | `GATE_REVALIDATION_REQUESTED`, `GATE_EVALUATED` | `GATE_VERIFIED`, `GATE_REJECTED`, `GATE_REVALIDATION_REQUIRED`, `PHASE_REOPENED`, `PROJECT_STATUS_DERIVED` | Gate Engine → Phase Engine → Project Status Engine |
| `WAIVE_GATE` | `GATE_WAIVED` | `GATE_EVALUATION_REQUESTED`, `GATE_EVALUATED`, `GATE_VERIFIED`, `PHASE_COMPLETED`, `PROJECT_STATUS_DERIVED` | Gate Engine → Phase Engine → Project Status Engine |
| `RECORD_WORK_PROGRESS` | `WORK_PROGRESS_RECORDED` | — | command handler |
| `RECORD_NOTE` | `NOTE_RECORDED` | — | command handler |
| `ADD_ARTIFACT_REFERENCE` | `ARTIFACT_REFERENCE_ADDED` | — | command handler |
| `RECORD_CLAIM` | `CLAIM_RECORDED` | `CHECKPOINT_INVALIDATED` | claim/freshness engines |
| `UPDATE_CLAIM_STATUS` | `CLAIM_STATUS_CHANGED` | `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` cuando el claim participa en una evaluación | claim/gate/freshness engines |
| `RECORD_UNKNOWN` | `UNKNOWN_RECORDED` | `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED`, `CHECKPOINT_INVALIDATED` cuando el unknown es material y bloqueante | unknown/Phase/Project Status/freshness engines |
| `RESOLVE_UNKNOWN` | `UNKNOWN_RESOLVED` | `PROJECT_STATUS_DERIVED`, `CHECKPOINT_INVALIDATED` | unknown/Project Status/freshness engines |
| `RECORD_VERIFICATION_RESULT` | `VERIFICATION_RECORDED` | `GATE_EVALUATION_REQUESTED`, `GATE_EVALUATED`, `GATE_VERIFIED`, `GATE_REJECTED`, `GATE_MARKED_STALE`, `PHASE_COMPLETED`, `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED` | Gate Engine → Phase Engine → Project Status Engine |
| `LINK_EXTERNAL_EVIDENCE` | `EXTERNAL_EVIDENCE_LINKED` | `GATE_EVALUATION_REQUESTED`, `GATE_EVALUATED`, `GATE_VERIFIED`, `GATE_REJECTED` | Gate Engine |
| `UNLINK_EXTERNAL_EVIDENCE` | `EXTERNAL_EVIDENCE_UNLINKED` | `EVIDENCE_INVALIDATED`, `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `CHECKPOINT_INVALIDATED` | evidence/freshness engines |
| `INVALIDATE_EVIDENCE` | `EVIDENCE_INVALIDATED` | `GATE_MARKED_STALE`, `GATE_REVALIDATION_REQUIRED`, `PHASE_REOPENED`, `CHECKPOINT_INVALIDATED`, `PROJECT_STATUS_DERIVED` | Gate → Phase → freshness → Project Status engines |
| `PROPOSE_DECISION` | `DECISION_PROPOSED` | — | command handler |
| `ACCEPT_DECISION` | `DECISION_ACCEPTED` | `CHECKPOINT_INVALIDATED` | decision/freshness engines |
| `SUPERSEDE_DECISION` | `DECISION_SUPERSEDED` | `CHECKPOINT_INVALIDATED` | decision/freshness engines |
| `REJECT_DECISION` | `DECISION_REJECTED` | — | command handler |
| `OPEN_BLOCKER` | `BLOCKER_OPENED` | `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED`, `CHECKPOINT_INVALIDATED` | Phase + Project Status + freshness engines |
| `UPDATE_BLOCKER` | `BLOCKER_UPDATED` | `PHASE_BLOCKED`, `PROJECT_STATUS_DERIVED`, `CHECKPOINT_INVALIDATED` | Phase + Project Status + freshness engines |
| `RESOLVE_BLOCKER` | `BLOCKER_RESOLVED` | `PROJECT_STATUS_DERIVED`, `CHECKPOINT_INVALIDATED` | Project Status + freshness engines |
| `OPEN_RISK` | `RISK_OPENED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `UPDATE_RISK` | `RISK_UPDATED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `MITIGATE_RISK` | `RISK_MITIGATED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `CLOSE_RISK` | `RISK_CLOSED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `SET_NEXT_ACTION` | `NEXT_ACTION_SET` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `COMPLETE_NEXT_ACTION` | `NEXT_ACTION_COMPLETED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `CANCEL_NEXT_ACTION` | `NEXT_ACTION_CANCELLED` | `CHECKPOINT_INVALIDATED` | freshness engine |
| `CREATE_CONTINUITY_CHECKPOINT` | `CHECKPOINT_CREATED` | — | Checkpoint Engine |
| `START_SESSION` | `SESSION_STARTED` | — | command handler |
| `END_SESSION` | `SESSION_ENDED` | `CHECKPOINT_CREATED` únicamente si la policy de cierre lo solicita explícitamente | Checkpoint Engine |
| `CONFIRM_EXTERNAL_MAPPING` | `EXTERNAL_MAPPING_CONFIRMED` | `CHECKPOINT_INVALIDATED` | identity/freshness engines |
| `REVOKE_EXTERNAL_MAPPING` | `EXTERNAL_MAPPING_REVOKED` | `CHECKPOINT_INVALIDATED` | identity/freshness engines |
| `RESOLVE_CONFLICT` | `CONFLICT_RESOLVED` | `CHECKPOINT_INVALIDATED` | conflict/freshness engines |
| `RECORD_EVENT_CORRECTION` | `EVENT_CORRECTION_RECORDED` | todos los derivados permitidos por el campo corregido, recalculados determinísticamente | projection/dependency engines |

### 15.4.1 Eventos exclusivamente derivados o internos

Los siguientes eventos **NO TIENEN command type externo homónimo**:

| Evento | Único productor autorizado |
|---|---|
| `PROJECT_STATUS_DERIVED` | Project Status Engine |
| `PHASE_BLOCKED` | Phase Engine a partir de blockers/gates/policy |
| `PHASE_COMPLETED` | Phase Engine después de `GateEvaluation` válida |
| `GATE_EVALUATED` | Gate Engine |
| `GATE_VERIFIED` | Gate Engine |
| `GATE_REJECTED` | Gate Engine |
| `GATE_MARKED_STALE` | Gate/freshness engine |
| `GATE_REVALIDATION_REQUIRED` | Gate/freshness engine |
| `DECISION_REVALIDATION_REQUIRED` | decision/freshness engine |
| `CHECKPOINT_INVALIDATED` | freshness engine |
| `CONFLICT_OPENED` | import, identity, digest, projection o domain conflict detector |
| `LEGACY_IMPORT_REVIEWED` | legacy import workflow tras una decisión explícita de review |

En las filas donde `PHASE_COMPLETED` aparece como evento derivado posible, su emisión queda condicionada por la política de cierre descrita en §17.2; la mera verificación de un gate no autoriza un cierre cuando la fase exige aprobación explícita.

Un handler que emita un evento fuera de `required_events ∪ possible_events` falla la transacción con `COMMAND_EVENT_CONTRACT_VIOLATION`. CI valida cobertura bidireccional:

```text
every command variant → one matrix row
every event variant   → command origin or derived-only owner
no external command   → PHASE_COMPLETED | GATE_VERIFIED | PROJECT_STATUS_DERIVED
```

## 15.5 Correcciones

`EVENT_CORRECTION_RECORDED` contiene:

```text
corrects_event_ref
reason
replacement_field_values allowlisted
actor
```

No cambia el evento previo. El projector aplica la corrección como nueva información y conserva ambos digests.

# 16. Comandos y submissions

## 16.1 `CommandEnvelope`

```yaml
schema_version: 0.2.0
command_id: command:0198
command_type: RECORD_VERIFICATION_RESULT
command_schema_ref: urn:project-ledger:command:record-verification-result:0.2.0
project_ref: project:01H...
expected_state_version: 184
idempotency_key: project-ledger:outcome:982
actor: {}
payload: {}
submitted_at: "2026-08-16T16:05:00-04:00"
payload_digest: sha256:...
command_digest: sha256:...
```

Local commands MAY omit expected version only when the CLI obtains it immediately before submit inside the same process; the persisted envelope always contains the resolved value.

## 16.2 Comandos canónicos

```text
CREATE_PROJECT
UPDATE_PROJECT_METADATA
PAUSE_PROJECT
RESUME_PROJECT
ARCHIVE_PROJECT
REGISTER_AUTHORITY_DOCUMENT
REFRESH_AUTHORITY_DOCUMENT_DIGEST
SUPERSEDE_AUTHORITY_DOCUMENT
BIND_REPOSITORY
REFRESH_REPOSITORY_BINDING
REVOKE_REPOSITORY_BINDING
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
RECORD_NOTE
ADD_ARTIFACT_REFERENCE
RECORD_CLAIM
UPDATE_CLAIM_STATUS
RECORD_UNKNOWN
RESOLVE_UNKNOWN
RECORD_VERIFICATION_RESULT
LINK_EXTERNAL_EVIDENCE
UNLINK_EXTERNAL_EVIDENCE
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
CONFIRM_EXTERNAL_MAPPING
REVOKE_EXTERNAL_MAPPING
RESOLVE_CONFLICT
RECORD_EVENT_CORRECTION
```

No existen external command types `PHASE_COMPLETED`, `GATE_VERIFIED`, `PROJECT_STATUS_CHANGED` ni arbitrary event append.

## 16.3 `CommandResult`

```yaml
schema_version: 0.2.0
command_result_id: result:0198
command_ref: command:0198
status: ACCEPTED
previous_state_version: 184
new_state_version: 185
emitted_event_refs: [event:702]
resulting_checkpoint_ref: null
rejection: null
idempotency:
  replayed: false
processed_at: "2026-08-16T16:05:01-04:00"
digest: sha256:...
```

Estados servidor:

```text
ACCEPTED
REJECTED
VERSION_CONFLICT
VALIDATION_FAILED
POLICY_DENIED
DUPLICATE_REPLAY
INCOMPATIBLE_CONTRACT
```

## 16.4 `ExternalSubmissionRecord`

Estado local del cliente/adaptador:

```yaml
schema_version: 0.2.0
submission_id: submission:0198
command_ref: command:0198
idempotency_key: project-ledger:outcome:982
state: OUTCOME_UNKNOWN
attempt_count: 1
last_transport_error: TIMEOUT_AFTER_SEND
known_result_ref: null
prepared_at: "..."
submitted_at: "..."
reconciled_at: null
digest: sha256:...
```

Estados:

```text
PREPARED
APPROVAL_REQUIRED
APPROVED
SUBMITTED
OUTCOME_UNKNOWN
RECONCILED_ACCEPTED
RECONCILED_REJECTED
CANCELLED
```

`OUTCOME_UNKNOWN` nunca se proyecta como accepted/rejected hasta reconciliar por command/idempotency key.

# 17. Máquina de estados de fase

## 17.1 Inicio

Una fase puede iniciarse si:

- existe en roadmap vigente;
- su predecesora está complete/skipped o existe waiver explícito;
- `execution_model=SEQUENTIAL`;
- no existe otra fase activa en el perfil MVP;
- authority docs no tienen conflicto crítico.

La estructura usa `active_phase_refs[]`, pero el gate MVP exige cardinalidad ≤1.

## 17.2 Cierre

`REQUEST_PHASE_COMPLETION` evalúa:

```text
all required gates ∈ {VERIFIED, WAIVED}
AND no incompatible blocker
AND authority documents current
AND no critical continuity conflict
AND phase completion policy passes
```

Salida:

- `PHASE_COMPLETED`;
- `PHASE_MARKED_READY_TO_CLOSE` cuando se requiere aprobación humana adicional;
- rechazo estructurado.

La emisión de `PHASE_COMPLETED` como derivación de `REQUEST_GATE_EVALUATION`, `WAIVE_GATE` o `RECORD_VERIFICATION_RESULT` solo está permitida cuando `completion_policy=AUTO_ON_ALL_REQUIRED_GATES`, todos los predicados de cierre pasan y no existe aprobación humana pendiente. En cualquier otro caso, el motor emite como máximo `PHASE_MARKED_READY_TO_CLOSE` y exige `REQUEST_PHASE_COMPLETION`.

## 17.3 Reapertura

Causas:

- gate revalidation required;
- authority superseded/digest mismatch;
- evidence invalidated;
- decisión material superseded;
- explicit human command con reason/authority.

Semántica de transición:

```text
COMPLETED + explicit admissible reopen → PHASE_REOPENED → ACTIVE
COMPLETED + stale dependency           → PHASE_REOPENED → REVALIDATION_REQUIRED
REVALIDATION_REQUIRED + gates repaired → ACTIVE o REQUEST_PHASE_COMPLETION
```

No existe un status durable `REOPENED`.

## 17.4 Phase kind

`Phase.kind` no cambia lifecycle. Se usa para UX, Radar status suggestion y policies declarativas.

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
schema_version: 0.2.0
gate_id: gate:sqlite-roundtrip
definition_version: 1
title: SQLite roundtrip real
phase_ref: phase-2
criticality: REQUIRED
required_evidence_rules:
  - rule_id: tests-pass
    kind: TEST_RESULT
    predicate:
      all:
        - equals: {path: result.exit_code, value: 0}
        - equals: {path: result.metrics.failed, value: 0}
  - rule_id: build-pass
    kind: BUILD_RESULT
    predicate:
      all:
        - equals: {path: result.exit_code, value: 0}
        - equals: {path: result.metrics.errors, value: 0}
repository_dependencies:
  paths: [tests/persistence/**]
  external_refs: []
completion_rule: {operator: ALL}
waiver_policy: HUMAN_APPROVAL
freshness_policy:
  evidence_must_match_same_repository_snapshot: true
revalidate_on:
  - EVIDENCE_INVALIDATED
  - REPOSITORY_DEPENDENCY_CHANGED
  - AUTHORITY_DOCUMENT_CHANGED
created_at: "..."
digest: sha256:...
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

`CUSTOM_DECLARATIVE` usa `PredicateAST`; nunca scripts.

## 21.3 Predicados

```text
all / any / not
equals / not_equals
less_than / less_or_equal / greater_than / greater_or_equal
exists / missing
contains
digest_equals
status_in
count_equals / count_at_least
same_snapshot_as
freshness_in
```

Unknown critical field produce `INCOMPLETE` o `CONFLICTED`, nunca true.

## 21.4 `GateEvaluation`

```yaml
schema_version: 0.2.0
gate_evaluation_id: gate-eval:0198
gate_ref: gate:sqlite-roundtrip
gate_definition_version: 1
gate_definition_digest: sha256:...
project_state_version: 184
repository_binding:
  repository_ref: repo:7d24
  snapshot_ref: snapshot:0198
  snapshot_digest: sha256:...
selected_evidence_refs:
  - evidence:test-221
rejected_evidence:
  - evidence_ref: evidence:build-old
    reason: STALE_REPOSITORY_SNAPSHOT
rule_results:
  - rule_id: tests-pass
    result: PASS
  - rule_id: build-pass
    result: MISSING
result: INCOMPLETE
proposed_gate_status: PENDING
explanation_codes: [REQUIRED_EVIDENCE_MISSING]
evaluated_by:
  engine: project-ledger-gate-engine
  version: 0.2.0
evaluated_at: "..."
provenance_ref: provenance:gate-eval-0198
digest: sha256:...
```

Resultados:

```text
PASS
FAIL
INCOMPLETE
STALE
CONFLICTED
ERROR
```

La evaluación es inmutable. `GATE_EVALUATED` enlaza su digest; Gate Engine deriva el estado.

## 21.5 Waivers

`GateWaiver` requiere actor, reason, scope, expiration/review condition, authority, gate version y digest. No fabrica evidencia faltante.

## 21.6 Revalidación

Un gate `VERIFIED` puede pasar a `STALE`/`REVALIDATION_REQUIRED` si cambia una dependency, authority document o evidence. La señal externa por sí sola no cambia estado.

# 22. Evidencia

## 22.1 `EvidenceRecord`

```yaml
schema_version: 0.2.0
evidence_id: evidence:221
project_ref: project:01H...
kind: TEST_RESULT
status: VERIFIED_SOURCE

source:
  source_type: COMMAND_EXECUTION
  tool: dotnet
  command_display: "dotnet test MemPilot.slnx --no-build --no-restore"
  working_directory_ref: workspace://
  runner_version: project-ledger/0.2.0

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

## 22.5 Importación y nivel de confianza

`evidence import --file` valida schema/digest y registra provenance, pero **no** convierte automáticamente el contenido en `VERIFIED_SOURCE`.

```text
archivo local sin firma/runner verificable  → ASSERTED
artefacto observado con hash/origen         → OBSERVED
resultado producido por runner confiable,
parser calificado y binding reproducible    → VERIFIED_SOURCE
contrato inválido o firma/digest fallido    → REJECTED
```

Cada parser de toolchain declara versión, campos soportados, fixtures y fallos. Un parser desconocido conserva el output como referencia y no inventa métricas.

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

Captura compacta e inmutable para cierre de sesión, handoff, compactación, cambio de agente y recuperación.

## 25.2 `ContinuityCheckpoint`

```yaml
schema_version: 0.2.0
checkpoint_id: checkpoint:184
project_ref: project:01H...
captured_state_version: 184
captured_material_state_version: 183
captured_state_digest: sha256:...
checkpoint_event_version: 185
checkpoint_event_ref: event:checkpoint-created-185

authority_documents: []
active_phase_refs: [phase-2]
primary_active_phase_ref: phase-2
verified_completed_phase_refs: [phase-0, phase-1]
pending_gate_refs: [gate:sqlite-roundtrip]
current_decision_refs: [decision:sqlite, decision:retention-seven-days]
open_blocker_refs: []
open_risk_refs: [risk:sqlite-locking]
next_action_refs: [action:implement-sqlite-repository]
last_material_event_refs: [event:701]
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

Semántica:

```text
state N
→ build checkpoint over N
→ append CHECKPOINT_CREATED at N+1
→ ProjectState N+1 references checkpoint
```

## 25.3 `CheckpointFreshnessRecord`

```text
CURRENT
CURRENT_FOR_PROJECT_ONLY
STALE_FOR_REPOSITORY
STALE_FOR_AUTHORITY
SUPERSEDED_BY_NEWER_CHECKPOINT
CONFLICTED
INVALIDATED
UNKNOWN
```

Freshness se registra append-only; no muta el checkpoint.

## 25.4 Regla de materialidad

Crear/renderizar/exportar un checkpoint no cambia `material_state_version`. Trabajo, decisión, gate, phase, blocker, risk, action, authority o mapping sí pueden cambiarla.

## 25.5 Current

Un checkpoint puede ser latest pero stale. `CURRENT` requiere:

- no material event posterior incompatible;
- authority digests current;
- repository binding compatible cuando se afirma current-for-repository;
- no conflict crítico.

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

## 27.1 Proyecto y catálogo

```bash
project-ledger init
project-ledger status
project-ledger doctor
project-ledger audit
project-ledger verify
project-ledger pause --reason <text>
project-ledger resume-project
project-ledger archive --reason <text>
project-ledger catalog list
project-ledger catalog rebuild
```

## 27.2 Autoridad y roadmap

```bash
project-ledger authority add --uri <path-or-uri> --digest <sha256>
project-ledger authority refresh <id>
project-ledger authority list
project-ledger authority supersede <id> --with <uri>
project-ledger repository bind --root <path> [--remote <uri>]
project-ledger repository refresh <binding-id> [--snapshot-ref <ref> --snapshot-digest <sha256>]
project-ledger repository revoke <binding-id> --reason <text>
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

## 27.4 Trabajo, notas, artefactos y evidencia

```bash
project-ledger record progress --phase phase-2 --summary "..." --files ...
project-ledger record note --kind LIMITATION --text "..."
project-ledger artifact add --uri <uri> --digest <sha256>
project-ledger claim record --statement "..." --classification ASSERTED
project-ledger claim status <claim-id> --set OBSERVED --evidence <ref>
project-ledger unknown record --subject <ref> --statement "..." --criticality MATERIAL
project-ledger unknown resolve <unknown-id> --resolution <ref>
project-ledger evidence run --kind TEST_RESULT --gate <id> -- <exe> <args...>
project-ledger evidence import --file result.json
project-ledger evidence link --system gsp --artifact-ref <uri> --digest <sha256>
project-ledger evidence unlink <id> --reason <text>
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
project-ledger import --file ledger.bundle --mode verify-fast-forward
project-ledger import --file ledger.bundle --mode fork
project-ledger import-markdown BITACORA_PROYECTO.md --preview
project-ledger import-markdown BITACORA_PROYECTO.md --apply-reviewed review.json
```

## 27.8 Integraciones

```bash
project-ledger serve provider --stdio --profile gskills-context-provider/0.3.0
project-ledger serve sink --stdio --profile gskills-context-provider/0.3.0
project-ledger integration manifest
project-ledger integration compatibility
project-ledger gsp export-corpus --output ./ledger-corpus
project-ledger radar export --output ledger-radar-sync.json
project-ledger radar validate-export ledger-radar-sync.json
project-ledger radar conformance
project-ledger catalog radar-export --all --output ledger-radar-sync.json
```

## 27.9 Output

- `--json` envía datos a stdout;
- diagnostics/progress a stderr;
- JSON nunca mezcla barra de progreso;
- commands de write aceptan `--expected-version` e `--idempotency-key` cuando aplica;
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
12 timeout/cancelled or outcome unknown
13 privacy/security rejection
14 integration incompatible
15 stream divergence
16 stale import ignored
```

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

## 28.6 Perfiles de privacidad del renderer

```text
FULL_LOCAL       títulos/resúmenes autorizados; sin raw output/secrets
MINIMAL_LOCAL    IDs, estados, conteos y summaries acotados
SHAREABLE        paths/identidades sensibles minimizados; provenance por digest
```

El perfil forma parte del digest del artefacto. Cambiarlo regenera una vista distinta, nunca el estado canónico.

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

# 31. Portable export e import

## 31.1 Contenido

```text
project-ledger-export/
├── manifest.json
├── project.json
├── events.jsonl
├── schemas-lock.json
├── upstream-locks/
├── projections/
│   ├── state.json
│   └── resume.json
├── evidence/references.json
├── integrations/
│   └── radar-project-projection.json
├── generated/BITACORA_PROYECTO.md
└── checksums.sha256
```

## 31.2 Exclusiones por defecto

Raw output, secrets, transcripts, absolute paths, private IDs, caches y encrypted blobs sin opt-in.

## 31.3 Import chain policy

Un import solo puede:

```text
IDENTICAL_CHAIN
FAST_FORWARD
NEW_PROJECT
FORK_EXPLICIT
```

Dos cadenas del mismo `project_id` que comparten ancestor pero divergen producen `STREAM_DIVERGENCE`. No existe merge automático por timestamp o solo por event ID.

## 31.4 `FORK_EXPLICIT`

Crea nuevo `project_id`, conserva `forked_from_project_ref/event_ref/digest` y no mezcla authority.

## 31.5 Verificación

- checksums y schemas;
- event hash chain;
- identity;
- stream version continuity;
- same ID/same digest;
- no overwrite de cadena más nueva;
- migrations/loss report.

# 32. Plano de integración

## 32.1 Principio

```text
READ PATH
manifest → health → projection/export

WRITE PATH
prepare/validate → approve/policy → submit → result → reconcile
```

Las rutas son independientes. Radar y GSP son read-only respecto del ledger en el MVP; G-Skills puede usar RecordSink explícito.

## 32.2 Transportes

```text
LOCAL_ARTIFACT_EXCHANGE   obligatorio para Radar y portable integrations
LOCAL_STDIO_JSON_RPC      preferido para G-Skills/GSP sidecars
LOCAL_SOCKET              post-MVP
MCP                       post-MVP
REMOTE_HTTP               fuera del MVP
```

## 32.3 Sin plugins in-process

Adapters de terceros se ejecutan out-of-process o por artifact exchange.

## 32.4 `IntegrationManifest`

```yaml
schema_version: 0.2.0
integration_manifest_id: integration-manifest:project-ledger
provider_id: project-ledger
provider_version: 0.2.0
protocols:
  - name: project-ledger-integration
    version: 0.2.0
  - name: gskills-context-provider
    version: 0.3.0
projection_capabilities:
  - PROJECT_CONTINUITY
  - LEDGER_PROJECT_SUMMARY
record_capabilities:
  - RECORD_WORK_PROGRESS
  - RECORD_VERIFICATION_RESULT
  - REQUEST_GATE_EVALUATION
  - REQUEST_GATE_REVALIDATION
  - CREATE_CONTINUITY_CHECKPOINT
side_effect_contract:
  projection: NONE
  health: NONE
  record_submit: DECLARED_ONLY
storage:
  shared_database_required: false
upstream_locks: []
digest: sha256:...
```

## 32.5 `ProjectionEnvelope`

Toda projection Ledger viaja con:

```text
request/profile ref
source product/version/manifest digest
project identity
state version/digest
payload schema/ref/digest
freshness
coverage/omissions
privacy profile
provenance
```

---

# 33. Protocolo stdio JSON-RPC

## 33.1 Métodos read-only

```text
ledger.manifest
ledger.health
ledger.project_continuity
ledger.project_summary
ledger.command_status
```

## 33.2 Métodos mutables

```text
ledger.command_validate
ledger.command_submit
ledger.command_reconcile
```

## 33.3 Request autocontenido

Protocol version, request ID, exact project/repository identity, requested fields, budget, privacy, trace IDs y expected source version cuando aplica.

## 33.4 Timeouts/cancellation

Timeout, cancellation, output limit, no corruption on crash. Mutable retry requires idempotency and reconciliation.

## 33.5 Framing

JSON-RPC 2.0 sobre stdio `Content-Length`. Mismo request + mismo state produce mismo payload digest en providers deterministas.

---

# 34. Integración con G-Skills v0.3

## 34.1 Rol

```text
ProjectContinuityProvider
RecordSink
```

Ledger conserva lifecycle; G-Skills conserva Repository State, impact, operational knowledge, policies, DecisionEnvelope, ContextCapsule y guards.

## 34.2 Lock y profile

```text
protocol.name = gskills-context-provider
protocol.version = 0.3.0
compatible_range = >=0.3.0 <0.4.0
```

`interop-gskills/upstream-lock.json` fija spec/schema registry/fixtures de G-Skills. `0.3` no se usa como versión canónica.

## 34.3 Contratos exactos

El adapter mapea mediante tabla campo-a-campo a:

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
CrossSystemReference
```

Los payloads G-Skills no se redefinen en schemas reducidos de Ledger. Se vendorizan/lockean o se validan contra el registry upstream exacto.

## 34.4 Continuity mapping

ProjectState → ProjectContinuityProjection preserva:

- state version/digest;
- authority docs;
- active phases/primary phase;
- completed verified phases;
- current decisions;
- blockers/risks;
- pending/revalidation gates;
- next actions con classification;
- checkpoint y repository binding;
- unknowns/conflicts;
- recent material events resumidos.

## 34.5 Authority/freshness

G-Skills concede authority localmente. Ledger reclama NONE fuera de project domains. A projection puede ser current-for-project y stale-for-repository.

## 34.6 RecordSink mapping

External commands permitidos se traducen a internal commands; no se expone arbitrary append ni direct lifecycle transition.

`ExternalRecordResult` y `CommandResult` permanecen distintos. `SINK_UNAVAILABLE/UNKNOWN_OUTCOME` son estados de adapter/submission, no del aggregate.

Esta revisión **NO anuncia** `RECORD_DECISION_REFERENCE` ni `RECORD_BLOCKER_REFERENCE` como capacidades de `RecordSink`. La revisión G-Skills `E2E-CONSOLIDATED-1` las mantiene en cuarentena porque el baseline Ledger previamente congelado las declaraba en el manifest sin variantes canónicas equivalentes en `command-payload` ni filas de `command-event-matrix`. Si se incorporan en el futuro, requieren semántica upstream explícita, schemas, fixtures y mapping bilateral; no se permite mapping analógico.

## 34.7 Gate revalidation

Signal → explicit external command → internal `REQUEST_GATE_REVALIDATION` → Gate Engine. La señal sola no muta.

## 34.8 Conformance

Projection side effects 0; current/stale; identity ambiguity; command accepted; phase stays open; duplicate replay; version conflict; outcome unknown; revalidation; no DB access; exact result mapping.

## 34.9 Compatibility label

```text
PROJECT LEDGER v0.2.0 DOCUMENT REVISION: GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT
G-SKILLS v0.3.0 SOURCE: E2E-CONSOLIDATED-1 / SHA-256 8ddbb2a9c23bf7355a1e577d93a0c541b0986df7416ab64069aea16295376928
RECIPROCAL G-SKILLS LOCK: REFRESH REQUIRED FOR THIS LEDGER REVISION
RECORD CAPABILITIES: 5 SUPPORTED / 2 FORMER *_REFERENCE CAPABILITIES NOT ADVERTISED
CERTIFIED COMPATIBILITY: PENDING RECIPROCAL LOCK + EXACT SCHEMA/FIXTURE LOCK + PROVIDER/RECORDSINK CONFORMANCE + BILATERAL E2E
```

---

# 35. Integración con GSP

## 35.1 Frontera exacta

```text
GSPCompiler.compile_bundle(query, evidence_bundle) -> CompilationResult
GSPCompiler.run_corpus(query, corpus_dir, output_dir, provider, ...) -> GSPRunResult
```

Ledger no redefine `EvidenceBundle`, `CandidateBundle`, `CompilationBundle`, `ValidationReport`, `UncertaintyMap` ni `TaskProjectionBundle`.

## 35.2 Upstream lock

Antes de certificar, el bridge fija:

```text
GSP internals document digest
upstream schema registry digest
fixture manifest digest
adapter normalization version
```

## 35.3 Corpus minimizado

```text
ledger-corpus/
├── authority/
├── decisions/
├── gates/
├── checkpoints/
├── selected-material-events/
├── sidecars/
└── manifest.json
```

Eventos se incluyen solo por allowlist, con bounded summaries; no se exporta timeline completo por defecto.

## 35.4 Adapter exact-contract

Query/scope → Ledger read-only → authorized sources → exact EvidenceBundle → compile_bundle → refs/digests/loss report. No writes.

## 35.5 Link de resultado

Solo mediante `LINK_EXTERNAL_EVIDENCE`; conserva validation, contradictions, uncertainty, query digest y provenance.

## 35.6 Authority

GSP gobierna la integridad de sus artefactos. No adquiere lifecycle, gate, policy ni code-guard authority.

## 35.7 Failure isolation

Core sin Python; sidecar opcional; no shared DB; failure no afecta Ledger.

---

# 36. Agent Skill y Codex

## 36.1 Skill de referencia

```text
integrations/agent-skills/project-ledger-continuity/
├── SKILL.md
├── manifest.json
├── examples/
├── tests/
└── schemas/
```

El manifest fija versión del perfil Agent Skills/Codex utilizado, required tools (`project-ledger`), supported platforms y compatibility range.

## 36.2 Procedimiento

- resume al comenzar;
- registrar avances materiales, no cada edición;
- evidence run/import;
- gate evaluate antes de claims;
- no editar Markdown generado;
- no completar fases/gates;
- checkpoint al terminar/handoff;
- version conflict refresh/replan;
- unknown outcome reconcile;
- registrar unknowns/limitations.

## 36.3 No autoridad

La skill no sustituye schemas, policy, expected version, idempotency, Gate Engine ni tests.

## 36.4 Conformance

- frontmatter/manifest valid;
- positive trigger cases;
- negative trigger cases;
- tool absent degradation;
- no state copied into SKILL.md;
- no write during resume;
- no completion claim without result.

---

# 37. Integración con Project Radar

## 37.1 Objetivo y dirección

Project Radar recibe continuidad Ledger como **overlay read-only** para enriquecer su observatorio humano.

```text
Project Ledger ── LedgerRadarProjectProjection ──► Project Radar
```

Radar NO escribe en Ledger en el MVP. La dirección evita ciclos de autoridad y mantiene la app ultraligera.

## 37.2 Compatibilidad con la arquitectura Radar

Project Radar v0.1 es Svelte/TypeScript/Vite + IndexedDB, sin backend obligatorio. Por ello:

- `stdio` no es transporte directo para el navegador;
- Ledger no abre IndexedDB;
- el perfil MVP obligatorio es artifact exchange;
- import requiere file selection/preview por el usuario;
- live watch queda para File System Access/Tauri/bridge post-MVP.

## 37.3 `LedgerRadarProjectProjection`

```yaml
schema_version: 0.1.1
projection_id: ledger-radar-projection:0198
projection_kind: LEDGER_PROJECT_SUMMARY
source:
  system: project-ledger
  product_version: 0.2.0
  project_ref: project-ledger://projects/01H...
  state_version: 184
  state_digest: sha256:...
  checkpoint_ref: project-ledger://checkpoints/184
  checkpoint_digest: sha256:...
identity:
  ledger_project_ref: project-ledger://projects/01H...
  stable_project_id: 01H...
  repository_identity_digests: [sha256:...]
  source_aliases: [MemPilot]
project:
  canonical_title: MemPilot
  bounded_summary: "..."
  lifecycle_status: ACTIVE
  suggested_radar_status: IN_DEVELOPMENT
  status_mapping_rule: ACTIVE_IMPLEMENTATION
continuity:
  primary_phase:
    ref: phase-2
    title: Persistencia
    kind: IMPLEMENTATION
    status: ACTIVE
  active_phases: []
  progress:
    method: LINEAR_REQUIRED_PHASES
    completed_required_phases: 2
    total_required_phases: 9
    verified_required_gates: 5
    total_required_gates: 8
    percentage:
      available: true
      basis_points: 2222
      unavailable_reason: null
    status: PARTIAL
  blockers: {open_count: 0, critical_count: 0}
  risks: {open_count: 1, critical_count: 0}
  gates: {pending_count: 1, revalidation_required_count: 0}
  conflicts: {critical_open_count: 0}
  next_actions: []
  preferred_next_action_ref: action:implement-sqlite-repository
  last_material_activity_at: "..."
  checkpoint_freshness: CURRENT
  authority_health: CURRENT
attention_signals: []
freshness:
  status: CURRENT
  evaluated_at: "..."
privacy:
  event_log_included: false
  evidence_payloads_included: false
  repository_paths_included: false
created_at: "..."
provenance_ref: project-ledger://provenance/radar-export-0198
digest: sha256:...
```

La proyección **no contiene `radar_project_ref`**. El ID de destino y la decisión de enlace pertenecen al consumidor Radar, lo que permite reutilizar el mismo feed en distintas instalaciones sin acoplar Ledger a una base IndexedDB concreta. Los alias son pistas de UI, nunca evidencia suficiente de identidad.

## 37.4 `RadarSyncEnvelope`

Agrega una o más projections:

```yaml
schema_version: 0.1.1
sync_envelope_id: radar-sync:0198
profile: project-ledger-radar-artifact/0.1.1
source_instance_ref: project-ledger://instances/local-user
exported_at: "..."
projects: []
tombstones: []
contract:
  schema_id: urn:project-ledger:radar-sync-envelope:0.1.1
  profile_schema_digest: sha256:...
  source_spec_digest: sha256:...
  canonicalization: JCS_RFC8785
semantics:
  omission_means_delete: false
  imports_are_idempotent: true
  name_only_mapping_allowed: false
digest: sha256:...
```

El current-project command produce un envelope de un item. `catalog radar-export --all` agrega ledgers conocidos mediante el catálogo derivado. El JSON es autocontenido: Radar valida contra el schema del profile incluido en su build o extensión y comprueba `profile_schema_digest`; no necesita leer archivos hermanos ni acceder a red. `digest` cubre el envelope completo salvo el propio campo `digest`.

## 37.5 Identity

`RadarProjectLink` es un contrato **propiedad de Project Radar**. Contiene Ledger ref, Radar ref, status, basis, actor y digest; no se persiste en el aggregate Ledger ni se exige para generar el feed. Métodos confirmables:

```text
USER_CONFIRMED
EXACT_STABLE_ID
EXACT_REPOSITORY_REMOTE_DIGEST
SIGNED_MAPPING
IMPORTED_MAPPING_REVIEWED
```

Ledger solo exporta anclas de identidad propias y digests de bindings permitidos. Nombre o alias igual es `CANDIDATE` y no autoriza apply automático.

## 37.6 Field ownership

La matriz de §10.4 es normativa. Radar v0.1 compatibility mode siempre muestra preview. Radar v0.2+ DEBERÍA persistir `ExternalProjectContinuity` separado de `Project`.

Un field ausente es `NOT_PROVIDED`, no `CLEAR`. El perfil v0.1.1 no contiene delete commands.

## 37.7 Status mapping

```text
INITIALIZING + authority/roadmap incomplete        → FORMALIZING
INITIALIZING + spec/roadmap ready + no phase       → SPEC_READY
ACTIVE + IMPLEMENTATION                            → IN_DEVELOPMENT
ACTIVE + TESTING                                   → TESTING
ACTIVE + AUDIT                                     → AUDITING
BLOCKED                                            → BLOCKED
PAUSED                                             → PAUSED
COMPLETED                                          → DONE
ARCHIVED                                           → ARCHIVED
```

`NOT_IMPLEMENTED` queda Radar-owned; no se infiere solo por ausencia de commits.

## 37.8 Progress semantics

No se exporta un porcentaje obligatorio. `percentage.available=true` y `basis_points` solo se emiten si:

- roadmap lineal;
- total de fases requeridas conocido y estable;
- no existe branch/denominador desconocido;
- el método está versionado.

En otro caso `percentage.available=false`, `basis_points=null` y `unavailable_reason` explícito. Las métricas estructuradas permanecen disponibles.

## 37.9 Merge/idempotencia

```text
same project + same version + same digest → DUPLICATE_REPLAY
lower version                             → STALE_IGNORED
higher version                            → APPLY_OVERLAY_UPDATE
same version + different digest           → SOURCE_DIVERGENCE
project omitted                           → NO_ACTION
field omitted                             → NO_CLEAR
```

## 37.10 Attention signals

```text
LEDGER_PROJECT_BLOCKED
LEDGER_CRITICAL_BLOCKER
LEDGER_CRITICAL_RISK
LEDGER_GATE_PENDING
LEDGER_GATE_REVALIDATION_REQUIRED
LEDGER_AUTHORITY_MISMATCH
LEDGER_CHECKPOINT_STALE
LEDGER_CONTINUITY_CONFLICT
LEDGER_NO_ELIGIBLE_NEXT_ACTION
LEDGER_NO_MATERIAL_ACTIVITY
```

Radar combina señales con su Motor Radar, conservando source/provenance.

## 37.11 Counterpart extension required

Project Radar v0.1 requiere:

```text
externalProjectContinuity
externalProjectLinks
externalImportReceipts
externalIntegrationConflicts
IndexedDB migration + backup extension
file import preview pipeline
source/freshness UI badges
```

`ExternalProjectContinuity` es infraestructura derivada y no una séptima entidad de producto. El perfil v0.1.1 normaliza estos nombres y supersede las etiquetas provisionales `ExternalProjectOverlay`, `externalSourceMappings` y `externalConflicts` del borrador anterior.

La extensión no añade backend ni IA y no cambia sus seis entidades core.

## 37.12 CLI

```bash
project-ledger radar export --output ledger-radar-sync.json
project-ledger radar export --privacy-profile minimal --output ...
project-ledger radar inspect-profile
project-ledger radar validate-export <file>
project-ledger radar conformance
project-ledger catalog radar-export --all --output <file>
```

## 37.13 Relación con G-Skills

Si Radar publica PortfolioProjection con datos Ledger, conserva upstream refs/digests. G-Skills aplica:

```text
Radar: portfolio.presentation CANONICAL/ADVISORY según policy
Ledger: project.lifecycle CANONICAL_IF_FRESH
```

No se cuenta Radar como confirmación independiente de Ledger.

## 37.14 Seguridad/privacidad

Default export excluye event log, evidence payloads, decision rationale completo, repository paths, commands y user paths. Imported JSON es untrusted input.

## 37.15 Conformance

Escenarios mínimos:

- export side-effect-free;
- explicit mapping;
- name-only candidate;
- replay exacto;
- stale ignored;
- same-version divergence;
- omission no delete;
- field absence no clear;
- Radar fields survive;
- status mapping;
- checkpoint stale;
- offline/no backend;
- no raw data;
- failure isolation;
- provenance preserved to G-Skills.

## 37.16 Compatibility label

```text
LEDGER RADAR PROFILE: DEFINED v0.1.1
PROJECT RADAR v0.1 NATIVE SUPPORT: REQUIRES EXTENSION
ARTIFACT EXCHANGE CONTRACT v0.1.1: SPEC-LEVEL DEFINED
LIVE SYNC: POST-MVP
CERTIFIED COMPATIBILITY: PENDING E2E CONFORMANCE
```

---

# 38. Extensibilidad

## 38.1 Adapter manifest

Declara adapter ID/version, protocol range, projection/command kinds, read/write, network, fields, runtime, schemas, upstream locks y loss report.

## 38.2 Reemplazabilidad

Quitar cualquier adapter no invalida event store ni core.

## 38.3 No marketplace automático

No auto-install ni trust desde metadata de repo.

## 38.4 Future integrations

GitHub Issues, Jira/Linear, CI, IDEs, document stores y artifact registries mediante typed projections/commands.

## 38.5 Integration SDK/conformance

Incluye manifest builder, envelopes, URI/digest/canonical JSON helpers, stdio framing, artifact exchange, malicious fixtures, loss-report utilities y Radar fixtures.

Conformance read-only: negotiation, no side effects, bounds, timeout, identity, freshness, no authority escalation, redaction, determinism, failure isolation.

Conformance mutable: write trust, prepare/submit, idempotency, expected version, unknown outcome, no invented lifecycle.

La detección de side effects se demuestra únicamente dentro del harness y los fixtures de conformidad observables. Este mecanismo **no promete ni constituye un sandbox universal** frente a un proceso local con privilegios equivalentes; la seguridad depende además de trust explícito, ejecución out-of-process, permisos mínimos, límites y cuarentena.

# 39. Persistencia

## 39.1 Canonical store

SQLite + WAL con event store append-only y mandatory hash chain.

Tablas conceptuales:

```text
projects
event_streams
events
commands
command_results
idempotency_keys
external_submissions
project_state_snapshots
evidence_records
evidence_artifact_refs
gate_evaluations
gate_waivers
checkpoints
checkpoint_freshness_records
external_references
external_mappings
conflicts
generated_artifacts
integration_locks
radar_export_receipts
schema_migrations
audit_records
```

La capa storage NO expone UPDATE/DELETE de events a application code. Tests y database constraints/triggers defensivos verifican append-only. `project_state_snapshots` es una cache derivada y descartable para acelerar replay; no sustituye eventos ni puede ser fuente de verdad tras un mismatch.

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
radar_project_view
project_catalog
```

Son rebuildable.

## 39.3 Ubicación

```text
<user-data>/project-ledger/projects/<project-id>/ledger.db
<user-data>/project-ledger/catalog.json
```

Repositorio:

```text
.project-ledger/
├── project.toml
├── gates/
├── policies/
├── integrations/
│   ├── gskills.toml
│   └── project-radar.toml
├── schemas.lock
└── .gitignore

BITACORA_PROYECTO.md
```

## 39.4 Mode portable

Repository-local store no es default MVP. Artifact exchange no implica mover DB al repo.

## 39.5 Backups

SQLite online backup/snapshot consistente, portable export, checksums, backup antes de migrations, restore no destructive.

## 39.6 Project catalog

Derived index; paths minimizadas/cifradas si sensibles; rebuild by explicit scan/known projects; no authority.

# 40. Concurrencia, idempotencia y recuperación

## 40.1 Transacción de escritura

```text
BEGIN IMMEDIATE
→ expected version
→ idempotency key/payload digest
→ domain validation
→ append events
→ update projections/snapshot
→ store CommandResult
COMMIT
```

## 40.2 Writer model

Un writer por project DB; múltiples readers. Multi-process lock/SQLite busy policy es bounded y no oculta conflictos de versión.

## 40.3 Replay

- same key/same digest → previous result;
- same key/different digest → `IDEMPOTENCY_CONFLICT`;
- external write without key → reject.

### 40.3.1 Scope y retención de idempotencia

La unicidad se evalúa sobre:

```text
(project_id, command_type, actor/trust-domain, idempotency_key)
```

La fila conserva `command_digest`, `CommandResult` y external operation refs. No se elimina mientras exista un resultado, submission incierto, checkpoint, audit record o referencia externa dependiente. La compactación solo puede moverla a almacenamiento histórico manteniendo lookup determinista.

## 40.4 Unknown outcome

```text
client submit
→ response lost
→ ExternalSubmissionRecord OUTCOME_UNKNOWN
→ ledger.command_status(command/idempotency)
→ reconcile accepted/rejected/not-found
→ retry only if safe and not-found under contract
```

No blind retry.

## 40.5 Crash recovery

Integrity check, WAL recovery, migrations, event chain verification, projection version/rebuild, submission reconciliation, generated artifact stale check.

## 40.6 Radar artifact idempotency

Export receipt records projection digest but export does not append project events. Radar import idempotency belongs to Radar counterpart; Ledger conformance provides fixtures.

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

Read/write trust separated.

## 41.2 Command execution

Arguments array, no shell default, explicit cwd, timeout/output limits, environment allowlist, redaction, no automatic network.

## 41.3 Untrusted repository

Cannot enable RecordSink/adapters/network/authority/credentials/global mappings or executable hooks.

## 41.4 Integrity

Mandatory event digest chain; schema validation; authority checksums; trust audit; path/symlink defense; integration artifact checksum/schema validation.

Genesis rule:

```text
stream_version = 1
previous_event_digest = null
```

Every later event must reference exact prior digest.

## 41.5 Radar imports

Radar JSON is untrusted. Ledger only produces artifacts; counterpart importer must reject path/URI injection, oversized payloads, unknown critical fields and schema mismatch.

# 42. Privacidad

## 42.1 Defaults

```text
network                  off
telemetry                off
raw terminal output      off
transcripts              never
reasoning privado        never
absolute paths export    minimized
external payload storage references-only
Radar export             continuity summary only
```

## 42.2 Redacción

API keys, tokens, passwords, connection strings, user paths, configurable secrets; email redaction optional by profile.

`command_display` is generated after redaction and cannot be used to reconstruct secret args.

## 42.3 Encryption

Sensitive opt-in payloads use envelope encryption + OS key provider. No silent plaintext fallback.

## 42.4 Privacy gate

```text
secret_persistence_count == 0
```

is a release gate over the declared corpus, scanners and supported channels—not a universal proof against every unknown secret format.

Scanned surfaces:

```text
DB/WAL/SHM
logs
exports
Markdown
crash artifacts
integration payloads
Radar artifacts
```

## 42.5 Radar minimal profile

Only IDs, bounded title/summary, lifecycle facts, counts, next action summary, freshness and provenance. No event log, evidence bodies, full decision rationale or repository paths by default.

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

Canonical JSON/digests, event application, command validation, project/phase/gate state machines, evidence predicates, supersession, blockers/risks, checkpoint version semantics, resume budget, renderer, redactor, repository binding, unknown lifecycle, imported-evidence trust, status mapping and Radar merge reference model.

## 45.2 Contract tests

Every schema requires positive/negative fixtures, `$id`, closed properties, offline refs, digest fixture, compatibility declaration and discriminator coverage.

Critical schemas include event/command payload unions and all Radar contracts.

## 45.3 Property tests

```text
same events → same state digest
rebuild → semantic equivalence
same key/same payload → at most one effect
same key/different payload → reject
version mismatch → no append
projection → state version unchanged
verified gate → valid evaluation/evidence or waiver
completed phase → required gates satisfied
superseded decision → never current
stale checkpoint → never CURRENT
checkpoint captured N → event version N+1
Radar same version/same digest → no duplicate
Radar lower version → no overwrite
Radar omission → no delete/clear
Radar-owned fields → preserved
```

## 45.4 Integration tests

SQLite append/rebuild, WAL, migrations, command transaction, evidence execution, Markdown divergence, portable chain import, legacy review, stdio, G-Skills provider/sink, GSP corpus/adapter, Radar export and reference importer fixtures.

## 45.5 E2E core

```text
init → authority → roadmap → phase → gate → progress → evidence
→ incomplete → complete evidence → gate verified → phase close
→ checkpoint N/N+1 → resume → render → export → reset/import
→ equivalent state
```

## 45.6 E2E Radar

```text
Ledger project → explicit Radar link → export artifact
→ Radar preview → overlay apply → refresh newer state
→ stale ignore → divergence conflict → offline reopen
```

## 45.7 Chaos

Crash append, disk full, malformed event, corrupted projection, sidecar crash, timeout after accept, concurrent writers, authority change, Markdown edit, adapter version change, truncated Radar file, duplicate artifact, stale file.

## 45.8 Security/privacy

Path traversal, symlink escape, malicious args, prompt injection, oversized JSON, secrets across stdout/stderr, untrusted repo enabling sink, name collision, authority overclaim, malicious Radar URI/content.

## 45.9 Differential/equivalence

- CLI vs stdio results;
- incremental vs rebuild;
- G-Skills native fixture vs Ledger mapping;
- Radar reference importer TypeScript vs Rust merge model;
- v0.1 migration fixtures vs v0.2 canonical state.

# 46. Escenarios obligatorios del benchmark

### PL-01 — Bitácora contradictoria
Fase 2 activa vs Fase 0 no iniciada → conflict, no automatic current.

### PL-02 — Tests verdes, gate incompleto
Tests pass, build missing → INCOMPLETE.

### PL-03 — Fase no cerrable
Critical blocker open.

### PL-04 — Supersesión
Only new decision current.

### PL-05 — Revalidación
Verified gate stale by authority/snapshot.

### PL-06 — Replay idempotente
One effect.

### PL-07 — Version conflict
Concurrent v184 writer loses safely.

### PL-08 — Unknown outcome
Submission reconciles original result.

### PL-09 — Projection side-effect-free
100 projections, state/event count unchanged.

### PL-10 — G-Skills current projection
Fields map with no material loss.

### PL-11 — G-Skills revalidation signal
No mutation without command.

### PL-12 — GSP separation
Claim does not close lifecycle.

### PL-13 — GSP conflict
Cross-domain conflict visible.

### PL-14 — Generated Markdown
Manual edit is divergence, not state.

### PL-15 — Portable roundtrip
Equivalent chain/state.

### PL-16 — Provider unavailable
Standalone works.

### PL-17 — Identity ambiguity
Same name not merged.

### PL-18 — Checkpoint version semantics
Capture N/event N+1/digest reproducible.

### PL-19 — Closed event payload
Unknown payload field/type fails schema.

### PL-20 — Command result vs submission
Timeout creates submission unknown, not server result unknown.

### PL-21 — Radar export side-effect-free
No state/version/event changes.

### PL-22 — Radar explicit mapping
User-confirmed mapping applies overlay.

### PL-23 — Radar name-only
Candidate only; no auto-link.

### PL-24 — Radar replay
Same version/digest no duplicate.

### PL-25 — Radar stale
Lower version ignored.

### PL-26 — Radar divergence
Same version/different digest critical conflict.

### PL-27 — Radar omission
Project/field omission does not delete/clear.

### PL-28 — Radar field ownership
Priority/tags/notes/ideas/chats/tasks survive.

### PL-29 — Radar status mapping
PhaseKind TESTING/AUDIT maps deterministically; NOT_IMPLEMENTED not invented.

### PL-30 — Radar unavailable
Last overlay marked stale/unavailable; no zero facts fabricated.

### PL-31 — Radar privacy
No raw events/evidence/paths in minimal export.

### PL-32 — Transitive provenance
Radar→G-Skills preserves upstream Ledger refs and is not counted as independent lifecycle source.

# 47. Gates de calidad

```text
event_chain_integrity                          = 1.00
closed_event_payload_validation                = 1.00
closed_command_payload_validation              = 1.00
projection_rebuild_equivalence                 = 1.00
duplicate_external_effects                     = 0
blind_retries_after_unknown                    = 0
unknown_outcome_misclassified_as_result        = 0
stale_checkpoint_rendered_current              = 0
checkpoint_version_ambiguity                   = 0
verified_gate_without_valid_evaluation         = 0
completed_phase_with_unsatisfied_required_gate = 0
projection_hidden_side_effects                 = 0
cross_system_false_identity_merge              = 0
secret_persistence_count                       = 0
GSP_semantic_authority_elevation               = 0
GSkills_direct_database_access                 = 0
Markdown_manual_edit_becomes_state             = 0
Radar_export_mutations                         = 0
Radar_silent_field_overwrites                  = 0
Radar_omission_deletes                         = 0
Radar_stale_overwrites_current                 = 0
Radar_same_version_digest_conflict_missed      = 0
Radar_raw_private_payload_count                = 0
Radar_lifecycle_authority_elevation            = 0
```

# 48. Criterios de aceptación del MVP

## Producto/autonomía

- **AC-001:** init funciona sin red.
- **AC-002:** core funciona sin G-Skills, GSP, Radar, IA o daemon.
- **AC-003:** CLI/JSON/stdio son semánticamente equivalentes para la misma operación.
- **AC-004:** default profile se distribuye como binario único.
- **AC-005:** export/import portable funciona.

## Event store/schemas

- **AC-006:** events append-only y hash-chain mandatory.
- **AC-007:** state rebuild from zero.
- **AC-008:** no stream gaps.
- **AC-009:** query no muta.
- **AC-010:** corrupted projection rebuilds.
- **AC-011:** event/command payload unions are closed.
- **AC-012:** all durable contracts are in offline registry.

## Lifecycle/gates/evidence

- **AC-013:** phase cannot complete with pending required gate.
- **AC-014:** verified gate references GateEvaluation and accepted evidence/waiver.
- **AC-015:** waiver has actor/reason/scope/review.
- **AC-016:** invalidated evidence reevaluates dependents.
- **AC-017:** phase reopens with provenance.
- **AC-018:** evidence run captures exit/duration/digest.
- **AC-019:** raw output off default.
- **AC-020:** asserted claim is not verified.
- **AC-021:** green result does not close gate/phase directly.

## Decisions/continuity

- **AC-022:** superseded decision not current.
- **AC-023:** blockers/risks preserve history.
- **AC-024:** resume fits budget.
- **AC-025:** stale checkpoint labeled.
- **AC-026:** authorized next action has authority ref.
- **AC-027:** checkpoint captures N and event N+1 explicitly.
- **AC-028:** project status derivation is deterministic.

## Markdown/import

- **AC-029:** Markdown deterministic.
- **AC-030:** editing Markdown does not mutate state.
- **AC-031:** verify detects divergence.
- **AC-032:** regenerate fixes view.
- **AC-033:** critical conflicts/unknowns visible.
- **AC-034:** legacy import never trusts all.
- **AC-035:** divergent event chains fail closed or explicit fork.

## Concurrency

- **AC-036:** same key/payload one effect.
- **AC-037:** same key/different payload reject.
- **AC-038:** version conflict no overwrite.
- **AC-039:** unknown outcome reconciles.
- **AC-040:** server CommandResult does not use transport unknown.
- **AC-041:** crash recovery preserves audit truth.

## G-Skills

- **AC-042:** exact v0.3.0 profile validates against upstream lock.
- **AC-043:** projection side effects zero.
- **AC-044:** authority overclaim ignored.
- **AC-045:** ambiguous identity disables canonical read/write.
- **AC-046:** revalidation signal no mutation.
- **AC-047:** RecordSink needs write trust.
- **AC-048:** G-Skills cannot submit phase/gate completion.
- **AC-049:** expected version/idempotency mandatory.
- **AC-050:** provider failure isolated.
- **AC-051:** no DB access.

## GSP

- **AC-052:** no reduced public GSP schema.
- **AC-053:** exact upstream contracts and locks.
- **AC-054:** GSP claim no lifecycle authority.
- **AC-055:** contradictions/uncertainty preserved.
- **AC-056:** failure isolated.
- **AC-057:** corpus export is allowlisted/minimized.
- **AC-058:** result linking explicit.
- **AC-059:** no shared DB/Python core dependency.
- **AC-060:** GSP↔Ledger conflict visible.

## Agent Skill

- **AC-061:** skill manifest/profile validates.
- **AC-062:** skill does not contain current state.
- **AC-063:** missing tool degrades explicitly.
- **AC-064:** skill cannot bypass engine/policy.

## Project Radar

- **AC-065:** `LedgerRadarProjectProjection` and `RadarSyncEnvelope` validate offline.
- **AC-066:** Radar export is side-effect-free.
- **AC-067:** explicit mapping required; name-only candidate.
- **AC-068:** same version/digest import is idempotent.
- **AC-069:** stale import cannot overwrite current.
- **AC-070:** same version/different digest opens critical conflict.
- **AC-071:** omitted project/field does not delete/clear.
- **AC-072:** Radar-owned fields never overwritten silently.
- **AC-073:** status mapping is deterministic and provenance-visible.
- **AC-074:** progress does not invent percentage.
- **AC-075:** stale/unavailable does not become zero blockers/gates.
- **AC-076:** minimal export excludes event log/evidence/paths.
- **AC-077:** no direct IndexedDB access/backend requirement.
- **AC-078:** Radar/Ledger each work independently.
- **AC-079:** transitive provenance preserved to G-Skills.
- **AC-080:** Radar never becomes canonical lifecycle owner by import.

## Security/privacy/release

- **AC-081:** untrusted repo cannot enable adapters/writes/authority/network.
- **AC-082:** no shell interpolation default.
- **AC-083:** secret scan zero on declared corpus/surfaces.
- **AC-084:** export minimizes sensitive data.
- **AC-085:** no transcripts/reasoning private.
- **AC-086:** unit/integration/E2E pass Windows/Linux; core contract tests also macOS.
- **AC-087:** schemas resolve offline.
- **AC-088:** PL-01…PL-32 pass.
- **AC-089:** performance budgets measured with declared environment.
- **AC-090:** compatibility labels do not overclaim certification.

# 49. Matriz de trazabilidad crítica

La matriz cubre **todos los criterios AC-001…AC-090**. Los requisitos `REQ-*` se añaden en las filas donde existe una obligación normativa directa; los demás criterios siguen siendo contractuales por su propia identificación AC.

| AC / requisito asociado | Criterio contractual | Componente propietario | Test mínimo | Slice |
|---|---|---|---|---:|
| AC-001 / REQ-PRD-001 | init funciona sin red. | `product-bootstrap` | `ac_001_init_funciona_sin_red` | S0 |
| AC-002 / REQ-PRD-001 | core funciona sin G-Skills, GSP, Radar, IA o daemon. | `core-runtime` | `ac_002_core_funciona_sin_g_skills_gsp_radar_ia_o` | S0/S14 |
| AC-003 | CLI/JSON/stdio son semánticamente equivalentes para la misma operación. | `cli-json-stdio` | `ac_003_cli_json_stdio_son_semanticamente_equivalentes_para_la_misma` | S2/S8/S14 |
| AC-004 | default profile se distribuye como binario único. | `release-packaging` | `ac_004_default_profile_se_distribuye_como_binario_unico` | S0/S14 |
| AC-005 / REQ-REL-002 | export/import portable funciona. | `portable-bundle` | `ac_005_export_import_portable_funciona` | S7 |
| AC-006 / REQ-EVT-001 | events append-only y hash-chain mandatory. | `event-store` | `ac_006_events_append_only_y_hash_chain_mandatory` | S1/S2/S3 |
| AC-007 | state rebuild from zero. | `projection-engine` | `ac_007_state_rebuild_from_zero` | S1/S2/S3 |
| AC-008 | no stream gaps. | `event-store` | `ac_008_no_stream_gaps` | S1/S2/S3 |
| AC-009 / REQ-CMD-001 | query no muta. | `query-bus` | `ac_009_query_no_muta` | S1/S2/S3 |
| AC-010 | corrupted projection rebuilds. | `projection-engine` | `ac_010_corrupted_projection_rebuilds` | S1/S2/S3 |
| AC-011 / REQ-EVT-002 / REQ-CMD-002 | event/command payload unions are closed. | `schemas` | `ac_011_event_command_payload_unions_are_closed` | S1/S2/S3 |
| AC-012 | all durable contracts are in offline registry. | `schema-registry` | `ac_012_all_durable_contracts_are_in_offline_registry` | S1/S2/S3 |
| AC-013 / REQ-GAT-002 | phase cannot complete with pending required gate. | `phase-engine` | `ac_013_phase_cannot_complete_with_pending_required_gate` | S5 |
| AC-014 / REQ-GAT-001 | verified gate references GateEvaluation and accepted evidence/waiver. | `gate-engine` | `ac_014_verified_gate_references_gateevaluation_and_accepted_evidence_waiver` | S5 |
| AC-015 | waiver has actor/reason/scope/review. | `gate-waiver` | `ac_015_waiver_has_actor_reason_scope_review` | S5 |
| AC-016 | invalidated evidence reevaluates dependents. | `evidence-invalidation` | `ac_016_invalidated_evidence_reevaluates_dependents` | S5 |
| AC-017 | phase reopens with provenance. | `phase-engine` | `ac_017_phase_reopens_with_provenance` | S5 |
| AC-018 | evidence run captures exit/duration/digest. | `evidence-runner` | `ac_018_evidence_run_captures_exit_duration_digest` | S5 |
| AC-019 | raw output off default. | `evidence-store` | `ac_019_raw_output_off_default` | S5 |
| AC-020 | asserted claim is not verified. | `claim-model` | `ac_020_asserted_claim_is_not_verified` | S5 |
| AC-021 | green result does not close gate/phase directly. | `gate-engine` | `ac_021_green_result_does_not_close_gate_phase_directly` | S5 |
| AC-022 | superseded decision not current. | `decision-model` | `ac_022_superseded_decision_not_current` | S4/S6 |
| AC-023 | blockers/risks preserve history. | `blocker-risk-model` | `ac_023_blockers_risks_preserve_history` | S4/S6 |
| AC-024 | resume fits budget. | `resume-renderer` | `ac_024_resume_fits_budget` | S4/S6 |
| AC-025 | stale checkpoint labeled. | `checkpoint-freshness` | `ac_025_stale_checkpoint_labeled` | S4/S6 |
| AC-026 | authorized next action has authority ref. | `next-action-model` | `ac_026_authorized_next_action_has_authority_ref` | S4/S6 |
| AC-027 / REQ-CHK-001 | checkpoint captures N and event N+1 explicitly. | `checkpoint-engine` | `ac_027_checkpoint_captures_n_and_event_n_1_explicitly` | S4/S6 |
| AC-028 | project status derivation is deterministic. | `lifecycle-projection` | `ac_028_project_status_derivation_is_deterministic` | S4/S6 |
| AC-029 | Markdown deterministic. | `markdown-renderer` | `ac_029_markdown_deterministic` | S6/S7 |
| AC-030 | editing Markdown does not mutate state. | `markdown-boundary` | `ac_030_editing_markdown_does_not_mutate_state` | S6/S7 |
| AC-031 | verify detects divergence. | `audit-verifier` | `ac_031_verify_detects_divergence` | S6/S7 |
| AC-032 | regenerate fixes view. | `markdown-renderer` | `ac_032_regenerate_fixes_view` | S6/S7 |
| AC-033 | critical conflicts/unknowns visible. | `conflict-renderer` | `ac_033_critical_conflicts_unknowns_visible` | S6/S7 |
| AC-034 | legacy import never trusts all. | `legacy-import` | `ac_034_legacy_import_never_trusts_all` | S6/S7 |
| AC-035 | divergent event chains fail closed or explicit fork. | `portable-bundle` | `ac_035_divergent_event_chains_fail_closed_or_explicit_fork` | S6/S7 |
| AC-036 / REQ-CON-001 | same key/payload one effect. | `command-idempotency` | `ac_036_same_key_payload_one_effect` | S2/S14 |
| AC-037 / REQ-CON-001 | same key/different payload reject. | `command-idempotency` | `ac_037_same_key_different_payload_reject` | S2/S14 |
| AC-038 / REQ-CON-001 | version conflict no overwrite. | `optimistic-concurrency` | `ac_038_version_conflict_no_overwrite` | S2/S14 |
| AC-039 / REQ-CON-001 | unknown outcome reconciles. | `submission-reconciliation` | `ac_039_unknown_outcome_reconciles` | S2/S14 |
| AC-040 / REQ-CON-002 | server CommandResult does not use transport unknown. | `command-result-model` | `ac_040_server_commandresult_does_not_use_transport_unknown` | S2/S14 |
| AC-041 | crash recovery preserves audit truth. | `recovery-audit` | `ac_041_crash_recovery_preserves_audit_truth` | S2/S14 |
| AC-042 / REQ-INT-001 | exact v0.3.0 profile validates against upstream lock. | `interop-gskills-contract` | `ac_042_exact_v0_3_0_profile_validates_against_upstream_lock` | S8/S9/S10 |
| AC-043 | projection side effects zero. | `interop-gskills-provider` | `ac_043_projection_side_effects_zero` | S8/S9/S10 |
| AC-044 | authority overclaim ignored. | `authority-resolver` | `ac_044_authority_overclaim_ignored` | S8/S9/S10 |
| AC-045 | ambiguous identity disables canonical read/write. | `identity-resolver` | `ac_045_ambiguous_identity_disables_canonical_read_write` | S8/S9/S10 |
| AC-046 | revalidation signal no mutation. | `gate-revalidation` | `ac_046_revalidation_signal_no_mutation` | S8/S9/S10 |
| AC-047 / REQ-CON-001 | RecordSink needs write trust. | `record-sink-trust` | `ac_047_recordsink_needs_write_trust` | S8/S9/S10 |
| AC-048 | G-Skills cannot submit phase/gate completion. | `interop-gskills-command-map` | `ac_048_g_skills_cannot_submit_phase_gate_completion` | S8/S9/S10 |
| AC-049 / REQ-CON-001 | expected version/idempotency mandatory. | `record-sink-protocol` | `ac_049_expected_version_idempotency_mandatory` | S8/S9/S10 |
| AC-050 | provider failure isolated. | `provider-runtime` | `ac_050_provider_failure_isolated` | S8/S9/S10 |
| AC-051 / REQ-INT-003 | no DB access. | `integration-boundaries` | `ac_051_no_db_access` | S8/S9/S10 |
| AC-052 | no reduced public GSP schema. | `interop-gsp-schema` | `ac_052_no_reduced_public_gsp_schema` | S8/S11 |
| AC-053 / REQ-INT-001 | exact upstream contracts and locks. | `interop-gsp-locks` | `ac_053_exact_upstream_contracts_and_locks` | S8/S11 |
| AC-054 | GSP claim no lifecycle authority. | `authority-resolver` | `ac_054_gsp_claim_no_lifecycle_authority` | S8/S11 |
| AC-055 | contradictions/uncertainty preserved. | `interop-gsp-normalizer` | `ac_055_contradictions_uncertainty_preserved` | S8/S11 |
| AC-056 | failure isolated. | `interop-gsp-runtime` | `ac_056_failure_isolated` | S8/S11 |
| AC-057 | corpus export is allowlisted/minimized. | `gsp-corpus-export` | `ac_057_corpus_export_is_allowlisted_minimized` | S8/S11 |
| AC-058 | result linking explicit. | `external-evidence-links` | `ac_058_result_linking_explicit` | S8/S11 |
| AC-059 / REQ-INT-003 | no shared DB/Python core dependency. | `integration-boundaries` | `ac_059_no_shared_db_python_core_dependency` | S8/S11 |
| AC-060 | GSP↔Ledger conflict visible. | `cross-domain-conflicts` | `ac_060_gspledger_conflict_visible` | S8/S11 |
| AC-061 | skill manifest/profile validates. | `agent-skill-contract` | `ac_061_skill_manifest_profile_validates` | S13 |
| AC-062 | skill does not contain current state. | `agent-skill-content` | `ac_062_skill_does_not_contain_current_state` | S13 |
| AC-063 | missing tool degrades explicitly. | `agent-skill-degradation` | `ac_063_missing_tool_degrades_explicitly` | S13 |
| AC-064 | skill cannot bypass engine/policy. | `agent-skill-policy` | `ac_064_skill_cannot_bypass_engine_policy` | S13 |
| AC-065 | `LedgerRadarProjectProjection` and `RadarSyncEnvelope` validate offline. | `interop-radar-schemas` | `ac_065_ledgerradarprojectprojection_and_radarsyncenvelope_validate_offline` | S12 |
| AC-066 / REQ-RAD-001 | Radar export is side-effect-free. | `radar-export` | `ac_066_radar_export_is_side_effect_free` | S12 |
| AC-067 | explicit mapping required; name-only candidate. | `radar-identity` | `ac_067_explicit_mapping_required_name_only_candidate` | S12 |
| AC-068 | same version/digest import is idempotent. | `radar-merge` | `ac_068_same_version_digest_import_is_idempotent` | S12 |
| AC-069 | stale import cannot overwrite current. | `radar-freshness` | `ac_069_stale_import_cannot_overwrite_current` | S12 |
| AC-070 / REQ-RAD-005 | same version/different digest opens critical conflict. | `radar-conflicts` | `ac_070_same_version_different_digest_opens_critical_conflict` | S12 |
| AC-071 / REQ-RAD-004 | omitted project/field does not delete/clear. | `radar-merge` | `ac_071_omitted_project_field_does_not_delete_clear` | S12 |
| AC-072 / REQ-RAD-003 | Radar-owned fields never overwritten silently. | `radar-field-ownership` | `ac_072_radar_owned_fields_never_overwritten_silently` | S12 |
| AC-073 | status mapping is deterministic and provenance-visible. | `radar-status-mapping` | `ac_073_status_mapping_is_deterministic_and_provenance_visible` | S12 |
| AC-074 | progress does not invent percentage. | `radar-progress` | `ac_074_progress_does_not_invent_percentage` | S12 |
| AC-075 | stale/unavailable does not become zero blockers/gates. | `radar-freshness` | `ac_075_stale_unavailable_does_not_become_zero_blockers_gates` | S12 |
| AC-076 | minimal export excludes event log/evidence/paths. | `radar-privacy` | `ac_076_minimal_export_excludes_event_log_evidence_paths` | S12 |
| AC-077 / REQ-RAD-002 / REQ-INT-003 | no direct IndexedDB access/backend requirement. | `radar-boundaries` | `ac_077_no_direct_indexeddb_access_backend_requirement` | S12 |
| AC-078 / REQ-PRD-001 | Radar/Ledger each work independently. | `radar-isolation` | `ac_078_radar_ledger_each_work_independently` | S12 |
| AC-079 | transitive provenance preserved to G-Skills. | `radar-provenance` | `ac_079_transitive_provenance_preserved_to_g_skills` | S12 |
| AC-080 | Radar never becomes canonical lifecycle owner by import. | `radar-authority` | `ac_080_radar_never_becomes_canonical_lifecycle_owner_by_import` | S12 |
| AC-081 / REQ-SEC-001 | untrusted repo cannot enable adapters/writes/authority/network. | `repository-trust` | `ac_081_untrusted_repo_cannot_enable_adapters_writes_authority_network` | S0/S14 |
| AC-082 | no shell interpolation default. | `process-execution` | `ac_082_no_shell_interpolation_default` | S0/S14 |
| AC-083 / REQ-PRV-001 | secret scan zero on declared corpus/surfaces. | `privacy-scanner` | `ac_083_secret_scan_zero_on_declared_corpus_surfaces` | S0/S14 |
| AC-084 | export minimizes sensitive data. | `export-privacy` | `ac_084_export_minimizes_sensitive_data` | S0/S14 |
| AC-085 / REQ-PRV-001 | no transcripts/reasoning private. | `privacy-model` | `ac_085_no_transcripts_reasoning_private` | S0/S14 |
| AC-086 | unit/integration/E2E pass Windows/Linux; core contract tests also macOS. | `cross-platform-ci` | `ac_086_unit_integration_e2e_pass_windows_linux_core_contract_tests` | S0/S14 |
| AC-087 | schemas resolve offline. | `schema-registry` | `ac_087_schemas_resolve_offline` | S0/S14 |
| AC-088 | PL-01…PL-32 pass. | `benchmark-release` | `ac_088_pl_01_pl_32_pass` | S0/S14 |
| AC-089 | performance budgets measured with declared environment. | `performance-benchmark` | `ac_089_performance_budgets_measured_with_declared_environment` | S0/S14 |
| AC-090 / REQ-INT-002 / REQ-REL-001 | compatibility labels do not overclaim certification. | `compatibility-release` | `ac_090_compatibility_labels_do_not_overclaim_certification` | S0/S14 |

### 49.1 Gate automático de cobertura

CI DEBE extraer IDs `REQ-*`, `AC-*` y escenarios `PL-*` desde spec, schemas, fixtures y tests. El release falla cuando:

- un AC no tiene al menos una fila y un test referenciado;
- un REQ no aparece en spec, schema o test;
- un escenario `PL-*` no está materializado en el benchmark;
- una fila apunta a un slice inexistente;
- un adapter cambia su upstream lock sin actualizar conformance y compatibility report.

Toda PR que implemente o modifique una fila debe citar IDs, actualizar schema/fixtures/tests y declarar impacto sobre compatibility, privacy, seguridad y benchmark.
# 50. Roadmap de implementación

## Fase 0 — Contratos, preservación y esqueleto

Workspace, CI, v0.1 checksum guard, schemas v0.2, canonical JSON, error model, registry/fixtures, upstream lock format.

Gate: schemas/refs/digests/checksum PASS.

## Fase 1 — Event store y comandos

SQLite/WAL, mandatory hash chain, closed command/event unions, idempotency, expected version, CommandResult, ExternalSubmissionRecord.

Gate: append/rebuild/replay/version/unknown-layer PASS.

## Fase 2 — Proyecciones y lifecycle básico

ProjectState, authority, sequential roadmap, phases/kinds, decisions, blockers, risks, actions, project status derivation, catalog.

Gate: rebuild equivalence and deterministic lifecycle PASS.

## Fase 3 — Gates y evidencia

Evidence runner/import, PredicateAST, GateDefinition/GateEvaluation, waivers, Phase Engine, freshness/revalidation.

Gate: no verified gate without valid evaluation; no complete phase with missing gate.

## Fase 4 — Continuidad, Markdown e import/export

Checkpoint N/N+1, freshness records, resume, renderer, audit/verify, legacy review, portable chain/fork.

Gate: deterministic Markdown, checkpoint semantics, roundtrip/divergence PASS.

## Fase 5 — Generic integration protocol

Manifest, health, projection envelope, stdio, artifact exchange, identity, upstream locks, SDK/conformance.

Gate: read side effects zero; malicious fixtures PASS.

## Fase 6 — G-Skills provider

Exact v0.3.0 lock/mapping, continuity, budget, freshness, conformance.

Gate: projection/profile E2E PASS.

## Fase 7 — G-Skills RecordSink

Prepare/submit/reconcile, write trust, command mapping, result mapping, gate revalidation, unknown outcome.

Gate: duplicate effects/blind retries zero.

## Fase 8 — GSP bridge

Minimized corpus, exact-contract sidecar, result links, authority isolation.

Gate: semantic degradation/authority elevation zero.

## Fase 9 — Project Radar articulation

Radar schemas/profile, project link, artifact export, project catalog aggregation, reference importer fixtures, status/progress mapping, field ownership, transitive provenance.

Gate: PL-21…32 and AC-065…080 PASS.

## Fase 10 — Agent Skill/Codex

Skill manifest, workflow, examples, triggers, handoff E2E.

Gate: skill conformance and no authority bypass.

## Fase 11 — Hardening/release

Security/privacy, fuzzing, chaos, performance, packaging, SBOM, migrations, compatibility report.

Gate: AC-001…090 PASS or explicit non-critical waiver.

# 51. Orden de implementación para Codex

```text
S0  repository skeleton + preservation
S1  contracts/schema registry/closed unions
S2  event store + commands + submission reconciliation
S3  projection engine
S4  lifecycle/authority/roadmap/status/catalog
S5  gates/evidence/phase engine
S6  checkpoints/resume/Markdown/audit
S7  portable + legacy import
S8  generic integration protocol + SDK
S9  G-Skills provider
S10 G-Skills RecordSink
S11 GSP exact-contract bridge
S12 Project Radar artifact profile + reference importer fixtures
S13 Agent Skill/Codex
S14 hardening/benchmark/release
```

Codex NO DEBE iniciar un slice posterior si el gate contractual anterior está rojo. S12 no modifica Project Radar product code salvo que el usuario ejecute el companion extension en su repositorio correspondiente; en Ledger se implementan schemas/export/reference fixtures.

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
├── specs/
│   ├── PROJECT_LEDGER_SPEC_v0.2.0.md
│   ├── history/PROJECT_LEDGER_SPEC_v0.1.0.md
│   ├── checksums.txt
│   └── compatibility/
│       ├── gskills-v0.3.0.md
│       ├── gsp-v0.2.md
│       ├── project-radar-v0.1.md
│       └── agent-skills.md
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
│   ├── project-catalog/
│   ├── integration-protocol/
│   ├── transport-stdio/
│   ├── artifact-exchange/
│   ├── interop-gskills/
│   ├── interop-gsp/
│   ├── interop-radar/
│   ├── cli/
│   └── test-support/
├── schemas/v0.2/
│   ├── schema-registry.json
│   ├── event-envelope.schema.json
│   ├── event-payload.schema.json
│   ├── command-envelope.schema.json
│   ├── command-payload.schema.json
│   ├── command-result.schema.json
│   ├── external-submission-record.schema.json
│   ├── project.schema.json
│   ├── project-state.schema.json
│   ├── authority-document.schema.json
│   ├── roadmap.schema.json
│   ├── phase.schema.json
│   ├── gate-definition.schema.json
│   ├── gate-evaluation.schema.json
│   ├── gate-waiver.schema.json
│   ├── evidence-record.schema.json
│   ├── evidence-artifact-ref.schema.json
│   ├── artifact-reference.schema.json
│   ├── repository-binding.schema.json
│   ├── unknown-record.schema.json
│   ├── claim-record.schema.json
│   ├── work-record.schema.json
│   ├── decision.schema.json
│   ├── blocker.schema.json
│   ├── risk.schema.json
│   ├── next-action.schema.json
│   ├── conflict-record.schema.json
│   ├── session.schema.json
│   ├── continuity-checkpoint.schema.json
│   ├── checkpoint-freshness-record.schema.json
│   ├── resume-projection.schema.json
│   ├── legacy-import-candidate.schema.json
│   ├── legacy-import-review.schema.json
│   ├── generated-artifact-record.schema.json
│   ├── identity-mapping.schema.json
│   ├── integration-manifest.schema.json
│   ├── projection-request.schema.json
│   ├── projection-envelope.schema.json
│   ├── provider-health.schema.json
│   ├── loss-report.schema.json
│   ├── portable-export.schema.json
│   ├── project-catalog-entry.schema.json
│   └── radar/
│       ├── ledger-radar-project-projection.schema.json
│       ├── radar-sync-envelope.schema.json
│       ├── radar-project-link.schema.json        # consumer-owned contract
│       ├── radar-status-mapping-policy.schema.json
│       └── radar-field-ownership-policy.schema.json
├── integrations/
│   ├── sdk/
│   ├── gskills/upstream-lock.json
│   ├── gsp/upstream-lock.json
│   ├── project-radar/
│   │   ├── PROFILE.md
│   │   ├── reference-importer/
│   │   └── fixtures/
│   └── agent-skills/project-ledger-continuity/
├── examples/
├── benchmark/
├── docs/
└── tests/
    ├── contract/
    ├── integration/
    ├── e2e/
    ├── property/
    ├── differential/
    ├── privacy/
    ├── security/
    ├── chaos/
    └── fuzz/
```

## 52.1 Schema closure

`event-payload` y `command-payload` contienen discriminated unions cerradas de todas las variantes. Un nuevo event/command requiere schema, fixtures, matrix, migration note y benchmark impact.

## 52.2 Adapter boundaries

Generic crates do not depend on product-specific adapters. Radar reference importer is test/support or companion code; Ledger core never imports Svelte/IndexedDB types.

# 53. ADR aceptados

| ADR | Decisión | Consecuencia |
|---|---|---|
| ADR-001 | Event sourcing append-only | Correcciones explícitas y rebuild |
| ADR-002 | SQLite/WAL | Durabilidad/transacciones locales |
| ADR-003 | Markdown derivado | No estado por edición manual |
| ADR-004 | CLI/JSON canónicos | Neutralidad de host |
| ADR-005 | No daemon obligatorio | 0% idle default |
| ADR-006 | Rust single-binary-first | Portabilidad/distribución |
| ADR-007 | Projection/RecordSink separados | No hidden writes |
| ADR-008 | Expected version + idempotencia | Concurrencia segura |
| ADR-009 | Ledger posee lifecycle | External systems request, not close |
| ADR-010 | G-Skills provider/sink | No shared DB |
| ADR-011 | GSP exact-contract | No reduced semantics |
| ADR-012 | Agent Skill non-authoritative | Procedure ≠ engine |
| ADR-013 | Raw evidence off | Privacy/bounded storage |
| ADR-014 | Legacy review-first | No contradiction elevation |
| ADR-015 | Radar read-only | Portfolio remains independent |
| ADR-016 | Closed event/command unions | No implementation-specific payloads |
| ADR-017 | Checkpoint capture N/event N+1 | No self-reference ambiguity |
| ADR-018 | ExternalSubmissionRecord | Transport uncertainty separated |
| ADR-019 | Sequential phases MVP | Model/state consistency |
| ADR-020 | Radar overlay, not overwrite | Human fields preserved |
| ADR-021 | Artifact exchange mandatory Radar MVP | Compatible with static IndexedDB app |
| ADR-022 | Omission is not delete | Safe partial exports |
| ADR-023 | Same version/different digest conflicts | No silent split-brain |
| ADR-024 | Transitive provenance | No duplicate authority through Radar |
| ADR-025 | Upstream locks | Verifiable interoperability claims |

# 54. Riesgos y mitigaciones

| ID | Riesgo | Impacto | Mitigación |
|---|---|---:|---|
| R-01 | Event model complejo | Alto | closed catalog + slices |
| R-02 | Bitácora pierde legibilidad | Medio | render profiles |
| R-03 | Exceso de registros | Medio | materiality policy |
| R-04 | Gates rígidos | Alto | waivers/review |
| R-05 | Agentes omiten registrar | Alto | skill/hooks/audit |
| R-06 | Sobre-registro | Medio | guidance/metrics |
| R-07 | Concurrencia | Alto | expected version |
| R-08 | Timeout duplica | Alto | idempotency/reconcile |
| R-09 | G-Skills drift | Alto | locks/conformance |
| R-10 | GSP conflation | Crítico | exact-contract/authority |
| R-11 | Legacy inventa estado | Crítico | preview/review |
| R-12 | Secrets evidence/export | Crítico | raw-off/redaction/scan |
| R-13 | SQLite corruption | Alto | backup/integrity/export |
| R-14 | UI premature | Medio | CLI-first |
| R-15 | Ledger manual burden | Alto | short commands/automation |
| R-16 | Radar schema unsupported | Alto | counterpart extension + incompatibility explícita / certificación pendiente |
| R-17 | Radar overwrites human fields | Crítico | overlay/ownership/preview |
| R-18 | Status mapping false precision | Alto | suggestion + rule/provenance |
| R-19 | Stale Radar appears current | Alto | version/digest/freshness |
| R-20 | Circular Ledger→Radar→G-Skills authority | Crítico | transitive provenance/advisory Radar lifecycle |
| R-21 | Browser cannot live-read file | Medio | artifact MVP; Tauri/File API optional |
| R-22 | Portfolio export turns Ledger into manager | Medio | derived catalog only |
| R-23 | Event schema bloat | Medio | `$defs` union and generated types |
| R-24 | Checkpoint off-by-one bugs | Alto | explicit fields/property tests |

## 54.1 Criterios de pivot

Pivot/reduce if maintenance cost exceeds resume benefit, most gates need arbitrary scripts, adapters cannot stabilize version/digest/idempotency, generated Markdown is less useful than resume, or Radar native overlay is not adopted.

Preferred pivots:

```text
full event sourcing → append-only journal simplified
mutable RecordSink  → export-only commands
full Markdown       → checkpoint-first
live Radar sync     → manual artifact exchange
many integrations   → G-Skills + Radar + GSP reference set
```

Never pivot to shared DB or LLM-generated canonical state.

# 55. Resultado esperado del MVP

El usuario podrá:

1. init Ledger;
2. registrar authority/roadmap/gates;
3. registrar progreso/evidence;
4. explicar por qué gate/phase sigue open;
5. completar/reabrir correctamente;
6. supersede decisions;
7. gestionar blockers/risks/next action;
8. resume Codex con checkpoint inequívoco;
9. render bitácora consistente;
10. audit contradictions;
11. export/import sin aceptar split-brain;
12. conectar G-Skills read/write separado;
13. enlazar GSP exact-contract;
14. exportar overlays Radar offline y seguros;
15. visualizar en Radar fase, gates, blockers y next action sin perder prioridad/tags/notas;
16. mantener todos los productos reemplazables e independientes.

# 56. Definición de terminado

Project Ledger v0.2.0 está terminado cuando:

- AC-001…090 pasan;
- v0.1 checksum se preserva;
- schemas/refs/unions resuelven offline;
- event chain/rebuild equivalence pasan;
- no duplicate effects/blind retries;
- checkpoint N/N+1 es reproducible;
- no phase/gate indebidamente complete/verified;
- Markdown/resume/export/import son coherentes;
- G-Skills profile locked/conformant;
- GSP exact-contract lock/conformance sin authority elevation;
- Agent Skill conformant;
- Radar artifact profile + reference importer fixtures + counterpart extension pasan AC-065…080;
- secret persistence gate pasa en declared corpus;
- Windows/Linux core y macOS contract profile pasan;
- limits, threat model, checksums, SBOM, benchmark y compatibility report se publican.

## 56.1 Condiciones NO-GO

- evento mutable o cadena rota;
- payload durable sin schema cerrado;
- phase complete con required gate pendiente;
- gate verified sin GateEvaluation válida;
- projection que muta;
- duplicate effect/blind retry;
- transport unknown tratado como accepted;
- stale checkpoint/current;
- checkpoint version ambiguity;
- identity merge por nombre;
- GSP lifecycle elevation;
- G-Skills DB access;
- Radar silent overwrite/delete/stale overwrite;
- Radar same-version divergence no detectada;
- Radar raw private payload;
- Radar lifecycle authority elevation;
- secret persistence;
- Markdown edit becomes state;
- unresolved spec↔schema mismatch;
- overclaim de certified compatibility.

# 57. Prompt inicial recomendado para Codex

```text
Implementa Project Ledger a partir de `PROJECT_LEDGER_SPEC_v0.2.0.md`.

AUTORIDAD Y PRESERVACIÓN

1. Lee la spec completa.
2. Verifica que el antecedente v0.1.0 conserve SHA-256:
   772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865
3. No edites el antecedente.
4. Si spec/schema/fixture discrepan, registra el defecto y detén esa parte.
5. No compartas DB con G-Skills, GSP o Project Radar.
6. Markdown no es store.
7. External systems cannot directly complete phase/gate.

PRIMERA ITERACIÓN: SOLO S0 + S1

S0
- Rust workspace;
- CI Windows/Linux/macOS;
- fmt/clippy/tests/deny;
- docs/licenses/security;
- checksum guards;
- schema/upstream-lock skeleton.

S1
- IDs/timestamps/canonical JSON/SHA-256/ErrorCode;
- schema registry offline;
- EventEnvelope + closed EventPayload union;
- CommandEnvelope + closed CommandPayload union;
- CommandResult;
- ExternalSubmissionRecord;
- Project/ProjectState;
- AuthorityDocument/Roadmap/Phase;
- GateDefinition/GateEvaluation/GateWaiver;
- Evidence/Claim/WorkRecord;
- Decision/Blocker/Risk/NextAction/Conflict/Session;
- ContinuityCheckpoint/CheckpointFreshness/Resume;
- LegacyImportCandidate/Review;
- GeneratedArtifact/IdentityMapping;
- IntegrationManifest/ProjectionRequest/Envelope/Health/LossReport;
- PortableExport/ProjectCatalogEntry;
- Radar schemas:
  LedgerRadarProjectProjection,
  RadarSyncEnvelope,
  RadarProjectLink,
  RadarStatusMappingPolicy,
  RadarFieldOwnershipPolicy;
- positive/negative fixtures;
- offline refs/digest tests;
- command-event matrix as data.

NO IMPLEMENTAR AÚN

- SQLite/EventStore;
- projection/gate engines;
- command execution;
- renderer/import;
- stdio/provider/sink;
- real G-Skills/GSP/Radar adapters;
- Agent Skill;
- MCP/UI/daemon/network/AI.

RESTRICCIONES

- stable Rust, cross-platform;
- no Python/Node/JVM/Docker core;
- no unsafe without ADR;
- no custom crypto;
- no network in tests;
- all durable schemas closed;
- all refs offline;
- PRs cite REQ/AC/slice;
- temporary implementation bitácora allowed, not product state.

DELIVERABLES

1. repository tree;
2. files changed;
3. contract→schema→fixture→test matrix;
4. checksum report;
5. commands/results;
6. green CI;
7. ambiguities;
8. Lista explícita de pendientes de S2, sin implementarlos.

DONE

- antecedent checksum matches;
- schema registry complete;
- event/command variants closed;
- positive/negative fixtures pass/fail as expected;
- deterministic canonical JSON/digests;
- no premature persistence/integration;
- fmt/clippy/test/deny green.
```

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

Project Ledger v0.2.0 es:

> **La fuente canónica, verificable y portable de continuidad operacional de un proyecto, resistente a pérdida de contexto y articulable con agentes, evidencia y portfolio sin fusionar autoridad ni almacenamiento.**

```text
Project Ledger
  lifecycle · phases · gates · decisions · blockers · risks · checkpoints

G-Skills
  repository state · impact · operational memory · guard · orchestration

GSP
  evidence compilation · provenance · contradiction · uncertainty

Project Radar
  human portfolio · priority · ideas · chats · tasks · artifacts · presentation
```

Articulación:

```text
typed projections
+ explicit commands
+ stable identity
+ version/digest/freshness
+ field ownership
+ idempotency/optimistic concurrency
+ transitive provenance
+ no shared databases
```

Estado:

```text
PROJECT LEDGER v0.2.0 IMPLEMENTATION BASELINE
DOCUMENT REVISION GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT
CODEX IMPLEMENTATION READY
G-SKILLS v0.3.0 E2E-CONSOLIDATED-1 SOURCE PINNED / RECIPROCAL LOCK REFRESH + CONFORMANCE PENDING
GSP v0.2 EXACT-CONTRACT BOUNDARY PRESERVED / EXACT SCHEMA+FIXTURE LOCK PENDING
PROJECT RADAR v0.1 ARTIFACT PROFILE 0.1.1 DEFINED / COUNTERPART EXTENSION REQUIRED
PROVIDER-FREE GOLDEN PATH REQUIRED
```

---

# 60. Changelog normativo v0.1.0 a v0.2.0

## Revisión de alineación G-Skills — 2026-08-21

- se fija como fuente de integración la revisión G-Skills v0.3.0 `E2E-CONSOLIDATED-1`, SHA-256 `8ddbb2a9c23bf7355a1e577d93a0c541b0986df7416ab64069aea16295376928`;
- se reconoce que el lock distribuido por G-Skills todavía congela el antecedente Ledger SHA-256 `e7acc496...`; por tanto, la certificación bilateral de esta revisión exige refresh recíproco del lock;
- `IntegrationManifest.record_capabilities` deja de anunciar `RECORD_DECISION_REFERENCE` y `RECORD_BLOCKER_REFERENCE`, cerrando la ambigüedad `PL-GSK-001` del lado Ledger sin inventar mappings analógicos;
- permanecen soportados exactamente cinco command kinds G-Skills: `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION` y `CREATE_CONTINUITY_CHECKPOINT`;
- S9/S10 quedan contractualmente lock-gated hasta refrescar el lock recíproco y superar schemas/fixtures, mapping/loss reports, conformance y E2E bilateral;
- no se altera el core de Ledger, el contrato GSP exact-contract ni el perfil Radar `project-ledger-radar-artifact/0.1.1`.

## Revisión final de cierre — 2026-08-20

- se preserva el borrador v0.2.0 auditado por hash antes de esta corrección final;
- se normaliza el contrato Radar a `project-ledger-radar-artifact/0.1.1`;
- la terminología **consumer-owned y de persistencia Radar** queda fijada en `ExternalProjectContinuity`, `ExternalProjectLink`, `ExternalImportReceipt` y `ExternalIntegrationConflict`; los contratos wire permanecen `LedgerRadarProjectProjection`, `RadarSyncEnvelope` y `RadarProjectLink`;
- se elimina el lock hash recursivo entre specs y se introduce un lock cruzado externo;
- se alinea G-Skills v0.3.0 con la baseline Project Ledger v0.2.0 sin declarar certificación runtime;
- se aclara que una evaluación de gate solo puede cerrar automáticamente una fase bajo `AUTO_ON_ALL_REQUIRED_GATES`;
- se regeneran manifiesto, checksums, QA y matriz de cierre contra los bytes finales.


## Añadido

- preservación formal del antecedente;
- requisitos `REQ-*` reales;
- closed event/command payload unions;
- `GateEvaluation` formal;
- `ExternalSubmissionRecord`;
- checkpoint capture N/event N+1;
- deterministic project status rules;
- `PhaseKind`;
- project catalog derived;
- upstream contract locks;
- full Project Radar artifact profile;
- `LedgerRadarProjectProjection`;
- `RadarSyncEnvelope`;
- `RadarProjectLink` consumer-owned;
- field ownership/status mapping/progress/freshness/merge semantics;
- Radar acceptance, benchmark, traceability and dedicated phase/slice;
- transitive Ledger→Radar→G-Skills provenance;
- fail-closed portable stream divergence.

## Cambiado

- Radar deja de ser una mención opcional y pasa a integración MVP opcional de primera clase;
- `PORTFOLIO_SUMMARY` se reemplaza por `LEDGER_PROJECT_SUMMARY` + envelope agregador;
- phases use array shape with MVP cardinality ≤1;
- event digest chain becomes mandatory;
- G-Skills canonical protocol version is `0.3.0`;
- GSP corpus is allowlisted/minimized;
- schema inventory expands materially;
- roadmap grows to dedicated integration phases.

## Rechazado explícitamente

- direct IndexedDB/SQLite access;
- Radar writes to Ledger in MVP;
- silent overwrite of human fields;
- omission-as-delete;
- name-only mapping;
- last-write-wins on same-version divergence;
- mandatory percentage progress;
- unknown transport outcome as server domain result;
- parallel phases in default MVP;
- certified compatibility without locked upstream contracts and E2E conformance.
