# Project Ledger Pre-pilot Readiness Implementation Plan

> **For agentic workers:** execute the eight stages below inline with `test-driven-development`, `systematic-debugging` and `verification-before-completion`. The execution subskills named by `writing-plans` are not available in this session; this does not authorize delegation or modifying another repository. Each stage has its own verification and closing commit.

**Goal:** Cerrar las brechas evitables de Ledger + Codex + GSP antes de una primera prueba en un proyecto real, con evidencia operacional y una entrega Windows portable.

**Architecture:** Mantener SQLite/WAL y los comandos/eventos v0.2 como autoridad. Completar comportamiento e interfaces en sus crates existentes, separar consultas de mutaciones y ejecutar procesos externos bajo una política acotada. La matriz `docs/PREPILOT_GAP_MATRIX.md` y `release/prepilot-readiness.json` distinguen preparación para el piloto de los resultados históricos de release.

**Tech Stack:** Rust 1.97.1 / edition 2024, SQLite/WAL, CLI Clap, JSON canónico, Python 3.12 opcional para el GSP fijado y GitHub Actions.

## Global Constraints

- Autoridad: `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`, SHA-256 `31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9`.
- No cambiar los 53 comandos, 67 eventos ni schemas/locks canónicos congelados para resolver carencias de implementación.
- Windows primero; mantener regresión Linux/macOS. Ledger + Codex + GSP dentro del alcance; G-Skills/Radar deshabilitados para el piloto y conservados como regresión, no certificados.
- GSP runtime `7a43559c02417641691dbb5f2a6e79c13d74d1a1`; contrato `gsp-exact-2026.04.12`, autoridad `004b14d7b64837025671970eac2469f092c49475`. No modificar la contraparte activa.
- Núcleo sin Python, red, daemon o modelos. Runtime GSP opcional, provisionado explícitamente, sin descarga automática durante uso.
- Sólo proyectos descartables. Nunca sobrescribir un proyecto real o restaurar sobre una DB existente.
- Fork materializado y extensiones contractuales quedan no disponibles con rechazo explícito, no éxito simulado. `certified_compatibility=false` y `ENGINEERING_RELEASE_CANDIDATE` se conservan.
- El contador de este cierre es X/8. El historial 12/12 macrofases y 15/15 slices no demuestra disponibilidad operacional completa.

## Stage 1 — Conciliar estado y matriz

**Files:** este plan, `docs/PREPILOT_GAP_MATRIX.md`, `release/prepilot-readiness.json`, `README.md`, `BITACORA_IMPLEMENTACION.md` y anexo del plan RQ4.

**Consumes:** especificación §§15–16, 27–31, 35–42, 45–49, 55–56; contratos, binario y CI observada.
**Produces:** una clasificación por brecha, etapa, interfaz, evidencia existente y prueba de cierre; inventario AC/PL sin confundir mocks con runtime o paquete.

- [x] Persistir el plan autorizado y conciliar la CI final RQ4 `33062468586` / `dea208d294dfddc282b84dcb5a58b9d8d9887b91`, seis jobs success.
- [x] Reproducir CLI ausente, código erróneo de DB ausente y dependencia de cwd; documentar por separado los hallazgos estáticos.
- [x] Verificar cobertura de todos los AC/PL, referencias, hashes de autoridad y diff; registrar NOT_READY y commit.

## Stage 2 — Núcleo completo

**Files:** `crates/application/src/core_handler.rs`, un módulo propio para registros auxiliares, `crates/projection-engine/src/lifecycle.rs` y pruebas de aplicación/proyección.

**Consumes/produces:** `CommandEnvelope` → `CoreCommandHandler` → `EventDraft` → `CommandProcessor`/SQLite → `ProjectStateProjection`. No nuevas variantes públicas.

- [x] RED: enviar notas, artefactos, sesiones, mappings y correcciones válidos por el procesador real; exigir aceptación, append, replay y reconstrucción equivalente. Añadir rechazos de referencias ajenas, IDs repetidos, sesiones cerradas y campos de corrección no permitidos.
- [x] GREEN: completar handlers e índices internos. Las correcciones sólo alteran vistas descriptivas allowlisted; conservan originales/digests y no cambian identidad, autoridad, evidencia, gates ni lifecycle.
- [x] Auditar los 53 comandos y 67 eventos con casos positivos/negativos aplicables; ejecutar gates y commit. Se observaron 66 eventos ejecutables; LEGACY_IMPORT_REVIEWED se verifica como prohibido por las 53 filas v0.2, no como emisión implementada. Evidencia: `docs/PREPILOT_CORE_COVERAGE.md`.

## Stage 3 — CLI operativa

**Files:** `crates/cli/src/main.rs`, módulos CLI por responsabilidad y tests de procesos CLI.

**Consumes/produces:** CLI normativa §27 → envelopes existentes → resultados canónicos. Añadir reconciliación por clave sin dependencia de G-Skills. Mantener argv existentes compatibles.

- [x] RED: diagnóstico, autoridad/repositorio, roadmap/fases/gates, registros/evidencia, decisiones/blockers/riesgos y continuidad deben ser invocables. DB ausente debe devolver 4, versión obsoleta 5 y conflicto idempotente 10; stdout JSON no contiene diagnóstico.
- [x] GREEN: implementar wrappers del motor, errores tipados y selección explícita prioritaria. El localizador sólo identifica; nunca habilita permisos o hooks.
- [x] Consultas no crean/migran DB. Reconciliación devuelve el resultado original y replay produce cero eventos adicionales. 17 pruebas CLI y regresión completa verificadas; evidencia en `docs/PREPILOT_CLI_COVERAGE.md`.

## Stage 4 — Procesos y privacidad

**Files:** `crates/evidence/src/runner.rs`, soporte de supervisión y tests con procesos propios descartables.

**Consumes/produces:** plan autorizado de argv/cwd/entorno → resultado acotado → `EvidenceRecord` existente; salida cruda efímera por defecto.

- [x] RED: proceso bloqueado, exceso de stdout/stderr, hijo persistente, cancelación, secreto ficticio y salida cero sin parser cualificado.
- [x] GREEN: defaults 60 s, 1 MiB por flujo, resumen 4 KiB; sólo aumentos explícitos. Sin shell, entorno allowlisted, ejecutable autorizado, limpieza de la familia propia y ningún retry ciego.
- [x] Asegurar que fallos no aprueban gates y datos importados no ascienden de confianza por su mera presencia. Gate completo local verificado; evidencia en `docs/PREPILOT_PROCESS_SAFETY.md`.

## Stage 5 — Incorporación y recuperación

**Files:** CLI, `storage-sqlite`, `portable-bundle`, `legacy-import` y `continuity`, con tests de procesos y almacenamiento.

**Consumes/produces:** repositorio nuevo/existente → binding explícito; historial → Markdown derivado/bundle portable; SQLite → snapshot consistente y restauración nueva.

- [x] RED: incorporar ambos tipos de proyecto sin modificar sus archivos, render/divergencia/regeneración, roundtrip portable, importación revisada y fork no disponible.
- [x] GREEN: adaptar las bibliotecas existentes a CLI; exigir revisión individual de candidatos y preservar procedencia/contradicciones. Nunca deducir fases verificadas de Markdown.
- [x] RED/GREEN: backup online WAL, restauración nueva con cadena/resultados/idempotencia íntegros, backup antes de migrar y consulta sin migración. Rechazar corrupción/divergencia/destino existente sin sobrescribir. Gate completo y seis recorridos CLI verificados; `docs/PREPILOT_PORTABILITY_RECOVERY.md`.

## Stage 6 — Codex real

**Files:** paquete `integrations/agent-skills/project-ledger-continuity`, sus locks y pruebas CLI.

**Consumes/produces:** herramienta → resume → progreso/evidencia → evaluación → checkpoint → reconciliación → resume; operaciones exclusivamente por CLI/stdio autorizados.

- [x] RED: recorrido completo y condiciones missing-tool, versión desactualizada, autorización insuficiente y resultado incierto por procesos reales. La versión antigua es un proceso fixture etiquetado, no una release histórica ejecutada.
- [x] GREEN: conciliar instrucciones/ejemplos con interfaces implementadas y actualizar hashes del paquete propio, nunca contratos upstream. Corregir pérdida de referencias en resume JSON detectada en el recorrido real.
- [x] Recorrido desde Codex descartable completo, sin DB directa, Markdown como autoridad o cierre autónomo de gate/fase. Gate completo verificado; `docs/PREPILOT_CODEX_PROCEDURE.md`.

## Stage 7 — Runtime GSP finito

**Files:** CLI GSP, supervisor de procesos, bridge/sidecar y pruebas runtime opt-in.

**Consumes/produces:** corpus minimizado → compiler real fijado → seis artefactos validados → receipt; enlace posterior separado y explícito.

- [x] RED/GREEN: entrada supervisada de compilación reutilizando contratos fijados y límites existentes de entrada 1 MiB / resultado 8 MiB.
- [x] Cubrir normal, contradicciones, incertidumbre, missing-runtime, drift de versiones, timeout/cancel/crash, truncamiento/manipulación, referencias/estado incorrectos y replay.
- [x] Etiquetar origen de cada evidencia (runtime, alteración adversarial, fixture); conciliar evidencia bilateral disponible, sin simular aceptación externa. Antes del link: delta 0; link: un efecto; replay: delta 0; elevación de autoridad: 0. Gate completo verificado; `docs/PREPILOT_GSP_RUNTIME_MATRIX.md`.

## Stage 8 — Paquete y cierre

**Files:** metadatos `xtask`, empaquetado/verificación PowerShell, recursos CLI, verificadores de release/readiness, CI y documentación de entrega.

**Consumes/produces:** release build y recursos hash-locked → ZIP Windows → pruebas desde extracción limpia → informe READY_FOR_CONTROLLED_PILOT cuando proceda.

- [x] RED/GREEN: recursos resueltos desde instalación o ruta explícita, no cwd; núcleo ejecutable sin toolchain/checkout/Python/red y requisitos GSP separados.
- [x] Probar el ZIP fuera del repo, rutas con espacios/no ASCII, permisos, núcleo offline y recorridos completos. ZIP limpio 52201, SHA y evidencia en `docs/PREPILOT_WINDOWS_DELIVERY.md`.
- [ ] Ejecutar formato, Clippy estricto, tests workspace, build locked, schemas/specs/integraciones, benchmark, dependencias, checksums, pruebas de fallos afectadas y CI exact-SHA.
- [ ] Cerrar sólo sin brechas conocidas en alcance. Registrar implementación verificada, artefacto, CI y limitaciones; commit final documental conserva evidencia ya observada y no promete CI futura.

## Seguimiento

Estado inicial: **0/8, NOT_READY**. Cada etapa se cierra con bitácora, prueba fresca y commit. Un hallazgo reabre su etapa; no amplía el alcance silenciosamente. Antes de compactar se registra el punto de continuación. La primera prueba con proyecto real exige el cierre 8/8 y la elección posterior de ese proyecto por el usuario.

Estado de ejecución: **7/8, NOT_READY**. Stages 1–7 verificadas; etapa 8 implementada y verificada localmente, incluido ZIP limpio. Sólo PP-14 sigue abierto: GitHub bloqueó los seis jobs de CI 33137113781 antes de ejecutar pasos por pagos fallidos o límite de gasto. Siguiente paso: el titular habilita Actions; reejecutar la CI del candidato f4b34e591b13199d040ecf7d41a071e21bf63d37, resolver cualquier fallo real y cerrar etapa 8 con commit documental. Sin cambios de facturación, visibilidad, contratos o proyecto real. No se declara apto para piloto todavía.

Reanudación 2026-08-27 23:21 America/Santiago: el único reintento (attempt 2) confirmó la misma restricción en los seis jobs, todos sin pasos. La verificación local del ZIP se repitió en 58849, exit 0, conservando el SHA del candidato y dos recorridos completos aprobados. No hay cierre adicional: esperar confirmación del titular de que Actions está habilitado antes de otro reintento; no convertir el PASS local en CI alojada aprobada.
