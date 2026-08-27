# Preparación del núcleo — etapa 2

Alcance: biblioteca real + SQLite real, contratos 0.2.0. No acredita todavía CLI cotidiana, procedencia de herramientas externas, ZIP distribuible ni operación con un proyecto real.

## Inventario comprobable

`crates/application/tests/command_coverage.rs` ejecuta un historial descartable por `CommandProcessor → CoreCommandHandler → motores → EventStore`. Compara el conjunto de comandos realmente aceptados con `CommandType::ALL` (53), y los eventos realmente persistidos con `EventType::ALL` salvo la exclusión contractual siguiente (66 de 67).

Después de cada operación comprueba: prefijo del historial intacto, replay sin eventos duplicados, rechazo de actor UNTRUSTED sin eventos, conflicto de versión sin eventos, integridad de cadena y equivalencia de reconstrucción completa/procesamiento incremental. Los fixtures aportan objetos de entrada; no reemplazan motores ni almacenamiento y no demuestran un compilador o runner externo.

| Comportamiento | Implementación | Prueba y resultado observable |
|---|---|---|
| Familias canónicas existentes | `application/core_handler`, `lifecycle`, `gate-engine`, `continuity` | `all_53_commands_have_real_atomic_replayable_rebuildable_paths` compara conjuntos exhaustivos, no conteos de archivos |
| Notas y artefactos | `continuity/checkpoint`, `projection-engine/lifecycle/auxiliary` | `auxiliary_records`: append/replay, identidad duplicada, referencia ajena, vista reconstruida |
| Sesiones | mismos módulos | Fin con/sin checkpoint; dos eventos atómicos; actor propio; rechazo de cierre repetido, inexistente o anterior al inicio; offsets RFC3339 |
| Mappings | mismos módulos | Confirmación humana, identidad local existente, ambigüedad rechazada, revocación conserva historia |
| Correcciones | índice descriptivo privado del projector | Sólo `text` de nota y `work_record.summary`; no IDs, trust, estados, refs, hashes o timestamps; originales intactos; work actualizado invalida checkpoint |
| Separación de permisos | `application/processor` | `POLICY_DENIED` sin eventos para actores sin permiso; recording no habilita metadata/autoridad/lifecycle humano |
| Preparación de cierre de fase | `gate-engine` | Recording puede solicitar READY_TO_CLOSE tras predicados; no produce PHASE_COMPLETED. Completar requiere humano local explícito |
| Revalidación de decisiones | `lifecycle/engine`, `projection-engine/lifecycle` | Supersesión/unavailability de autoridad dependiente marca REVALIDATION_REQUIRED; autoridad ajena/MATCH no invalida; recuperar fuente no reacepta; humano puede reaceptar explícitamente |
| Gate negativo | `gate-engine` | Predicado incumplido produce GATE_REJECTED, separado de INCOMPLETE y revalidation |
| Idempotencia y resultado | `application/processor` | Resultado incluye checkpoint creado; replay conserva referencia; `command_transaction` cubre mismo key/digest distinto y combinación comando/evento prohibida |
| Cache | projector interno versión 5 | Los tests de queries/rebuild deben reconstruir cache anterior o corrupta; no cambia schema ni contrato durable |
| Autoridad sustituida | `projection-engine/lifecycle` | UNAVAILABLE/MISMATCH/MATCH sobre una versión SUPERSEDED nunca la vuelve CURRENT; el evento conserva la observación |

## Límite explícito: el evento 67

`LEGACY_IMPORT_REVIEWED` pertenece al vocabulario, pero todas las filas de `schemas/v0.2/command-event-matrix.json` lo prohíben. No existe un comando canónico para emitirlo en v0.2.

`legacy_review_cannot_be_smuggled_through_a_v02_canonical_command` comprueba las 53 prohibiciones y un intento real de añadirlo a un CREATE_PROJECT válido: el processor devuelve INCOMPATIBLE_CONTRACT / COMMAND_EVENT_CONTRACT_VIOLATION y persiste cero eventos. No se amplía el contrato ni se considera implementada su emisión. Etapa 5 conservará la revisión como artefacto y traducirá candidatos representables a comandos explícitos permitidos, con procedencia; los restantes se informan como no representables.

## Verificación de cierre

Etapa 2 verificada localmente el 2026-08-27. Pasaron formato, Clippy estricto, suite workspace completa, build locked, schema check, specs/contratos (46 schemas / 92 fixtures / 53 filas), release check, cargo-deny offline e integridad SHA-256 (13/13). Las 15 pruebas nuevas cubren comandos auxiliares, inventario operacional, revalidación, autoridad sustituida y rechazo de legacy no representable. La prueba previa de cierre de fase ahora demuestra READY_TO_CLOSE sin PHASE_COMPLETED para el agente y cierre posterior por humano.

La primera regresión detectó metadatos de release desactualizados tras reutilizar `time` del workspace: el hash de Cargo.lock no coincidía. Se regeneraron con `cargo xtask release generate`, sin modificar criterios de aceptación ni certificación, y la repetición completa pasó. Las cuatro pruebas opt-in permanecieron ignored; no se ejecutó un compilador GSP real, ENOSPC físico, benchmark nuevo, CI hosted ni ZIP en este cierre. El verificador de release comprobó el reporte de benchmark histórico, no nuevas mediciones.

Estado global: **2/8, NOT_READY**. CLI operativa, supervisión/privacidad, incorporación/recuperación, Codex, conformidad GSP y paquete Windows siguen pendientes según el plan.

Los cambios de esta etapa no autorizan elevar confianza de evidencia importada: esa comprobación de procedencia corresponde a etapa 4. Tampoco habilitan G-Skills/Radar para el piloto.
