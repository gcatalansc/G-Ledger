# PL-GSK-002: conflicto nativo antes del envío G-Skills

## Autoridad y base

Alfredo aprobó el plan bilateral en G-Ledger Engine II: «Apruebo, pero quiero que verifiques que no quede nada roto.» La autorización cubre implementación, regresiones, commits locales candidatos y recalificación G-Skills con nuevas referencias exactas. No incluye push, merge, tag, publicación, datos operativos ni cambios contractuales congelados.

Base: `e593e8a173d824ba72a4c70a4299a7e3623be8a9`, rama `codex/history-core`, línea histórica v0.3 todavía sin versionar como release. El paquete declara `0.2.0`; la entrega publicada `v0.2.0-rc.1` permanece en `22b3eca46ac6b963ab928e0131b0cdf88cd6627c`.

## Problema y cambio

Después de preparar un comando válido, un checkpoint competidor aceptado avanza la versión. El adaptador anterior compara el estado antes de invocar al procesador y devuelve JSON-RPC `-32602`, `StateMismatch`; no existe un resultado durable para consultar.

`GSkillsRecordSink::submit` conserva el `expected_state_version` aprobado cuando difiere del actual. El procesador nativo produce y persiste `VERSION_CONFLICT` sin eventos de dominio. No se cambia el comando aprobado, no se infiere una aceptación y no hay retry automático. Si la versión coincide, siguen siendo obligatorias las comparaciones de digest y versión material. Identidad, política, caducidad, privacidad y consistencia interna se verifican antes de elegir ambas rutas.

Código de producción: únicamente `crates/interop-gskills/src/sink.rs`. Regresiones nuevas: `crates/interop-gskills/tests/sink_protocol.rs` y `crates/cli/tests/gskills_sink.rs`. Sin dependencias, migraciones, nuevos eventos, capacidades, schemas o locks contractuales.

## Evidencia local

- Reproducción inicial: la prueba de protocolo falla con `StateMismatch { message: "project_state_version is stale" }`. La prueba CLI demuestra primero `VALID`, acepta un checkpoint competidor de versión 1 a 2 y reproduce `-32602` al enviar el comando original.
- Corrección: las once pruebas de ambas suites específicas pasan. El conflicto informa versión 2 a 2 y `emitted_event_refs=[]`; los eventos serializados antes/después son idénticos y la cadena íntegra.
- Cada consulta CLI posterior arranca un proceso nuevo. `status` y `reconcile` recuperan `FOUND`, `VERSION_CONFLICT` y el mismo digest nativo, incluso después de expirar la aprobación de envío. También se lee el resultado persistido mediante la API de almacenamiento.
- Casos negativos: versión coincidente con digest/material incorrectos; versión obsoleta con proyecto, actor, scope, política, ejecutable, caducidad, privacidad o versión interna incorrectos. Conservan rechazo sin resultado persistido ni eventos.
- Primera ejecución de la prueba de identidad: una aserción buscó el detalle en `error.message`; el protocolo lo entrega en `error.data.record_sink_error`. Se corrigió la aserción, sin cambiar producción.

Verificación fresca local, Windows/Rust 1.97.1:

| Comprobación | Resultado |
|---|---|
| `cargo test --locked --workspace --all-targets --all-features --no-fail-fast` | 319 aprobadas, 0 fallos, 14 omitidas; 110 harnesses, exit 0 |
| Repetición de `gskills_sink` y `sink_protocol` con `--locked --all-features` | 11 aprobadas, 0 fallos |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | PASS |
| `xtask schema check` | PASS; schemas generados coinciden |
| `specs verify` / `contracts verify` | v0.1/v0.2 intactas; 46 schemas, 92 fixtures, 53 filas base; extensión 8 schemas/54 filas |
| `resources verify` | 461 recursos verificados |
| `xtask release check` / `release verify` | PASS local; 90 AC, 32 escenarios PL, cero secretos/waivers; no certificación externa |
| Diff de contratos, specs, locks y dependencias frente a la base | Sin cambios |

Las 14 omisiones preexistentes corresponden a helpers invocados por pruebas supervisoras, runtime GSP externo exacto y filesystem efímero para ENOSPC. No se atribuye a este candidato una ejecución nueva de los jobs externos ni de CI Linux/macOS. La suite completa incluye las seis regresiones históricas v0.3 y los recorridos portable, backups, recursos y release.

Clippy solicitó invertir `if != ... else` a `if == ... else` (`if_not_else`), sin cambiar la lógica. La suite completa se compiló antes de esa inversión; las once pruebas específicas se repitieron sobre la forma final y Clippy terminó limpio. Logs locales en `.tooling/pl-gsk-002/` (ignorados por Git); SHA-256 de `workspace.log`: `ec236f13eacc3768e551669942ec8ac2d4792290b09afb7fe0d0a97550251ee2`.

## Entrega candidata

Build: `cargo build --locked --release -p project-ledger-cli`, exit 0; perfil optimizado, 2m 52s. Binario `target/release/project-ledger.exe`, copiado sin cambios a `.tooling/pl-gsk-002/delivery/project-ledger.exe` para la recalificación. SHA-256: `175aa89d01c27107522299814aedb35d6cae9b3ad967d99c1a2b15445d97fd20`. El ejecutable optimizado también pasó `specs verify`, `contracts verify`, `resources verify` y `release verify`.

La tarea G-Skills recibe el commit completo que incorpora esta corrección y el binario exacto; su recalificación y el posible gap adicional se documentan como evidencia posterior, sin trasladar los PASS del antiguo candidato `22b3eca`.

## Límite de concurrencia y aceptación

UB-52 de G-Skills v0.3.0 describe un cambio entre preparación y envío: rechazo nativo, actualización y ausencia de sobrescritura. Este cambio resuelve ese caso. Las rutas ya existentes del procesador que detectan una carrera durante el propio commit pueden retornar un conflicto no persistido; tampoco se acredita recuperación de resultados frente a avances posteriores de la proyección. No se declara durabilidad universal bajo concurrencia continua.

La tarea G-Skills Engine III contrasta esos límites con sus gates y ejecuta la recalificación contra commit/binario exactos, con una tentativa, refresco y obligación de nuevo comando/aprobación. Un criterio obligatorio no cubierto se conserva abierto; el parche no autoriza una ampliación del núcleo ni un resultado sintético. Una calificación local no sustituye CI multiplataforma, certificación externa ni cierre de Fase 6.

## Rollback

Revertir el commit local de corrección y retirar la selección del binario candidato en G-Skills; conservar evidencia y fixtures aislados. No usar rc.1 para abrir bases que ya contengan eventos v0.3.
