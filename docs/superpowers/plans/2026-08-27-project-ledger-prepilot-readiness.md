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
- [x] Gate local: formato, Clippy estricto, tests workspace, build locked, schemas/specs/integraciones, benchmark, dependencias, checksums y fallos Windows/GSP afectados. Evidencia 70512, 64399, 52201 y 56539; no sustituye ENOSPC físico ni CI del candidato en Linux/macOS.
- [x] CI exact-SHA con seis jobs y artefacto alojado: run `33664393505`, SHA `b171f51cb58c60ed8c8a99b60386117f9fed6822`, 6/6 PASS; artefacto `9860800241` descargado y revalidado.
- [x] Cerrar sólo sin brechas conocidas en alcance. Implementación, artefacto, CI y limitaciones quedan registrados sin prometer certificación o GA.

### Entrega local mientras GitHub queda aplazado

Decisión del usuario del 2026-08-27: «Dejemos pendiente github, avanza con lo siguiente del plan». Se suspende toda consulta, reintento y push a GitHub. El trabajo continúa con la documentación/entrega local prevista en esta misma etapa, no con una fase 9 ni un piloto real. La CI sigue siendo requisito de cierre 8/8; el aplazamiento no elimina PP-14.

**Archivos y responsabilidad:** `docs/PREPILOT_LOCAL_HANDOFF.md` contendrá la guía operativa descartable; `release/prepilot-local-handoff.json`, el resultado verificable de esa guía. La carpeta nueva `dist/entrega-local-0.2.0-f4b34e5/` contendrá el ZIP existente, la guía, los informes y la evidencia documental seleccionada explícitamente, con hashes. `release/prepilot-readiness.json`, README, matriz y bitácora registrarán el aplazamiento sin alterar contratos, binario ni recursos incorporados.

**Interfaces:** consumir el ZIP limpio fuente `f4b34e591b13199d040ecf7d41a071e21bf63d37`, SHA-256 `0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c`. Producir una entrega local para revisión/ensayo descartable, nunca READY_FOR_CONTROLLED_PILOT mientras la CI quede pendiente.

- [x] Preparar guía PowerShell con selección explícita de un ZIP, hash antes de extraer, destino temporal nuevo con acentos y comandos CLI reales de init, resume, nota, checkpoint, reconciliación, render, backup, restore y audit. Sin `--as-human`, lectura directa de SQLite, edición de Markdown generado, instalación global o descargas.
- [x] Ejecutar los bloques exactos de la guía desde fuera del checkout; exigir un checkpoint reconciliado idéntico, restore con estado/idempotencia conservados y replay sin duplicación. Sólo datos descartables nuevos; detenerse ante cualquier código inesperado. 67950 exit 0: 19 invocaciones, v3, resultado preservado, replay sin duplicación y audit sin findings; SHA incorrecto rechazado antes de extraer.
- [x] Preparar la carpeta local mediante copias explícitas sin sobrescribir instalaciones ajenas; verificar hashes de todos sus archivos y el ZIP, y repetir la guía contra esa copia. 026894 exit 0: 17 archivos explícitos, copias íntegras, ZIP original y guía sin cambios; ensayo de la copia en 67950. Resultados históricos y único pendiente alojado separados.
- [x] Conciliar informes/plan/bitácora y preparar el commit de entrega local sujeto a la validación final de checksums/diff. No es commit de cierre 8/8; no hacer push durante el aplazamiento.

## Seguimiento

Estado inicial: **0/8, NOT_READY**. Cada etapa se cierra con bitácora, prueba fresca y commit. Un hallazgo reabre su etapa; no amplía el alcance silenciosamente. Antes de compactar se registra el punto de continuación. La primera prueba con proyecto real exige el cierre 8/8 y la elección posterior de ese proyecto por el usuario.

Estado durante el aplazamiento: **7/8, NOT_READY; GitHub aplazado por el usuario**. Stages 1–7 verificadas; etapa 8 implementada y verificada localmente, incluido ZIP limpio. Sólo PP-14 seguía abierto. La actualización final de este documento registra su resolución posterior; no se reescribe esta evidencia histórica.

Reanudación 2026-08-27 23:21 America/Santiago: el único reintento (attempt 2) confirmó la misma restricción en los seis jobs, todos sin pasos. La verificación local del ZIP se repitió en 58849, exit 0, conservando el SHA del candidato y dos recorridos completos aprobados. No hay cierre adicional: esperar confirmación del titular de que Actions está habilitado antes de otro reintento; no convertir el PASS local en CI alojada aprobada.

Actualización 2026-08-28: entrega local preparada y ensayo de su guía verificado; no quedan pasos locales de implementación en este plan. Se conserva la CI como pendiente aplazado y no se agrega trabajo para sustituirla. El siguiente criterio de cierre 8/8 será verificar GitHub cuando el usuario lo retome; hasta entonces la carpeta local sirve para revisión/ensayo descartable, no piloto real.

Actualización final 2026-09-02: el usuario retomó GitHub después de la renovación de cuota. Un fallo real de macOS reabrió la garantía de limpieza, se corrigió sin tolerar EPERM y la CI `33664393505` terminó 6/6 PASS sobre el SHA `b171f51cb58c60ed8c8a99b60386117f9fed6822`. El ZIP alojado fue descargado y repitió integridad, Codex y portabilidad/recuperación/privacidad con 0 fallos. **Estado final del plan: 8/8, APTO PARA PILOTO CONTROLADO; PP-01…PP-14 cerrados.** El siguiente paso requiere que el usuario elija un proyecto y autorice un piloto acotado; no se habilitan G-Skills/Radar, GA o certificación general.
