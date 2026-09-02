# Etapa 8 — entrega Windows y preparación para piloto

Estado: **8/8, APTO PARA PILOTO CONTROLADO**. El ZIP de Windows del SHA `b171f51cb58c60ed8c8a99b60386117f9fed6822` pasó verificación local y alojada; la CI privada completó sus seis jobs y el artefacto descargado se revalidó fuera del runner. La [entrega local y guía descartable](PREPILOT_LOCAL_HANDOFF.md) se conserva como evidencia histórica del candidato anterior. No se ha ejecutado todavía un proyecto real.

## Distribución y recursos

`scripts/build-windows-package.ps1 -Output <ZIP nuevo> -RequireClean` compila x86_64-pc-windows-msvc con CRT estático y dependencias locked. `package-windows.ps1` copia exclusivamente el catálogo de 448 recursos incorporado al binario, autorizado por `crates/cli/resources.txt`, y produce package.json con hashes, tamaños y SHA Git. No se descubren archivos locales ajenos o ignorados al compilar. Se identifica cualquier árbol sucio: no equivale a una release reproducida desde un commit limpio. ZIP y extracción siempre usan destinos nuevos; no se sobrescribe una instalación previa.

`resources list` no necesita filesystem; `resources verify`, schemas/specs/contratos/release e integraciones verifican recursos por hashes compilados. Selección explícita `--resources` > instalación junto al exe > fallback de desarrollo anclado a target/debug o target/release. Nunca se usa el cwd accidental como fuente de recursos. Datos de proyectos, bitácora/readiness y credenciales no forman parte del catálogo. Los archivos fuente de evidencia incluidos son datos leídos por release verify, no un checkout ni herramientas requeridas.

La integridad protege contra corrupción o recursos mezclados entre versiones. Un SHA externo de confianza sigue siendo necesario para autenticar el ZIP: no se afirma firma de código ni resistencia al reemplazo malicioso simultáneo de binario/manifiesto. El núcleo no requiere red ni ejecuta descargas; esto no equivale a una sandbox de red para un ejecutable externo autorizado.

## Matriz del artefacto

| Caso | Prueba ejecutable / aceptación |
|---|---|
| ZIP real, no sólo manifiesto | Preflight de nombres, duplicados y presupuesto; extracción fuera del repo, rutas con espacios/acentos; todos los hashes/tamaños y catálogo compilado |
| Windows sin redistribuible MSVC | Inspección de imports PE rechaza VCRUNTIME/MSVCP/MSVCR/ucrtbase; binario estático probado mediante procesos |
| Núcleo sin herramientas de desarrollo | Ledger recibe sólo SystemRoot/TEMP/TMP, sin PATH/Cargo/Git/Python; init, status, resume, escrituras, evidencia y consultas |
| Codex | Mismos argv de la skill incluida en el ZIP: progress, evidence, gate pendiente, checkpoint, reconcile, resume, autorización/conflicto/replay |
| Continuidad y recuperación | Portable export/import conserva resume; backup/restore privado conserva resultado e idempotencia; replay no duplica; audit sin findings |
| Incorporación existente | Bind humano a carpeta de trabajo descartable con acentos; contenido/archivos preservados; no ejecución Git/hooks |
| Errores sin escrituras impropias | Base ausente 4; restore existente 2 sin overwrite; fork 2 antes de escribir; recursos alterados 15 sin fallback |
| Privacidad real | Canary en stderr de proceso fallido ausente de resultado/DB/Markdown; nota local privada rechaza export 13 y no aparece en vista compartible |
| GSP opcional | Ausente 12 sin descarga/publicación; matriz supervisada del runtime fijado repetida contra el ejecutable extraído |
| Regresión agregada | Formato, Clippy, workspace, locked build, schemas/specs/contratos, release, benchmark, dependencias, hashes, seis GSP opt-in y CI exact-SHA con ENOSPC físico |

Los harnesses de ensayo se compilan fuera del paquete; invocan el ejecutable extraído, no bibliotecas de sustitución. Los procesos Ledger del ensayo no reciben rutas ni herramientas del checkout. Las entradas de setup humano son fixtures descartables y no conceden autoridad universal a Codex. Se mantienen límites de 60 s / 1 MiB por flujo / 4 KiB de resumen salvo configuración explícita.

## Evidencia en curso

- RED recursos: 92171, fuera de cwd / comando inexistente; GREEN 33277, dos tests PASS.
- RED distribución: 24701, ZIP con hashes válidos rechazado por VCRUNTIME140.dll. No se entrega ese ZIP negativo.
- Codex aislado: 12632, dos tests PASS; fixture de versión histórica sigue etiquetado como fixture.
- Portable/privacidad CLI: 8047 y 50196 PASS; ampliación bind incluida y verificada en el ensayo del ZIP.
- ZIP candidato de árbol sucio `b13c67f7f4d8992f613084e366908d836d4522b38a3d6ce0fd33c77ad2c85976`: build estático 50196; ensayo estricto 31049 PASS, exactamente un test Codex y un test portable ejecutados (no se acepta cero tests filtrados). Dos extracciones nuevas externas; PE sin redistribuible MSVC; 448 recursos y binario íntegros. No es el artefacto final de commit limpio.
- GSP 12018: Clippy y seis tests reales PASS, tres supervisados usando la CLI extraída; RQ4 conserva probe test-only externo y crash real PID 5388, reinicio/link/replay sin elevación. Runtime y spec upstream limpios.
- Benchmark: RED 62658 confirmó fecha histórica fija; GREEN 73652 prueba reloj real. Clippy requirió ordenar el módulo de tests y después pasó en 12018. Medición 64399 PASS, 2026-08-28T02:39:50.27391Z: nueve targets, startup p95 42,77 ms, RSS p95 19,684 MiB, 20 muestras/10.000 eventos; reports y hashes regenerados. La medición usa el binario estático candidato, no afirma rendimiento universal.
- Allowlist de distribución: RED 61570 por ausencia de inventario revisado; sustituida enumeración de directorios por 448 rutas explícitas. Test exige igualdad exacta entre inventario y catálogo; ningún archivo local adicional entra por descubrimiento.
- Gate completo 70512 exit 0: fmt, Clippy, workspace/all-targets/all-features, build locked, schemas/specs/contratos/release, dependencias y 13 hashes. ZIP actualizado incluirá además LICENSE del proyecto (omisión detectada por prueba negativa del candidato-1); no cambia el catálogo de recursos. Falta observar la reconstrucción limpia y CI.
- Candidato limpio 52201 PASS: fuente `f4b34e591b13199d040ecf7d41a071e21bf63d37`, ZIP `dist/project-ledger-0.2.0-windows-x86_64-clean.zip`, SHA-256 `0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c`. 448 recursos, binario y LICENSE íntegros; exactamente 1 test Codex y 1 portable/bind/privacidad PASS desde extracción externa con acentos. Se conserva reporte local sin rutas personales en `release/prepilot-windows-verification.json`.
- GSP repetido sobre ese ZIP limpio: 56539 exit 0, seis tests reales PASS. Tres de supervisión con CLI extraída; RQ4 usa su probe test-only y observó crash propio PID 45084, reinicio y link/replay sin cambios de autoridad. No se cambió ningún upstream.
- [CI 33137113781](https://github.com/gcatalansc/G-Ledger/actions/runs/33137113781), SHA exacto del candidato: reported failure, seis jobs con cero pasos. Las seis anotaciones indican que GitHub no inició el trabajo por pagos recientes fallidos o límite de gasto insuficiente. No hay logs de tests ni fallo observado del código. Jobs: Windows 98739436754, macOS 98739436756, ENOSPC 98739436757, GSP 98739436787, Ubuntu 98739436805 y dependencias 98739436808.
- Diagnóstico con gh-fix-ci: autenticación repo/workflow válida, push privado confirmado y anotaciones de todos los jobs revisadas. No se altera workflow para eludir el bloqueo, no se cambia visibilidad ni facturación y no se repiten ejecuciones mientras siga la misma restricción. No se ha ejecutado un proyecto real.

## Reanudación del 2026-08-27, 23:21 America/Santiago

- Ante la nueva instrucción de continuar se realizó un único reintento del candidato exacto, para comprobar si la restricción externa se había resuelto. [Intento 2 de CI 33137113781](https://github.com/gcatalansc/G-Ledger/actions/runs/33137113781/attempts/2): failure, los seis jobs con cero pasos y la misma anotación de pagos/límite de gasto. Observado a las 2026-08-28T03:20:39Z. Jobs ENOSPC 98744058292, GSP 98744058423, dependencias 98744058427, Windows 98744058452, Ubuntu 98744058456 y macOS 98744058466. No se observó un fallo de código; no se harán más reintentos hasta que el titular confirme haber resuelto la restricción.
- Verificación fresca del mismo ZIP limpio: 58849 exit 0, SHA sin cambios, extracción externa nueva con espacios y acentos. Se repitieron integridad de recursos/specs/contratos/release, imports PE, ambos recorridos completos (exactamente 1/1 Codex y 1/1 portable/recuperación/privacidad), GSP ausente y alteración de recursos rechazada. El reporte local nuevo es `dist/project-ledger-0.2.0-windows-x86_64-clean.zip.recheck-20260828T0320Z.json`; resumen sin rutas personales actualizado en `release/prepilot-windows-verification.json`.
- En esta reanudación no se modificó código ni se repitieron el gate completo del workspace, el benchmark o el runtime GSP real: su evidencia anterior se conserva con sus fechas y límites. El contador sigue en 7/8; este registro no es un cierre de etapa ni una aprobación de piloto.

## Límites y siguiente paso

Reintento expresamente solicitado por el usuario («Prueba ahora»): [intento 3](https://github.com/gcatalansc/G-Ledger/actions/runs/33137113781/attempts/3) del mismo candidato, observado a las 2026-08-28T03:45:01Z. Volvió a terminar failure antes de ejecutar pasos; las seis anotaciones coinciden en la restricción de pagos/límite de gasto. Jobs ENOSPC 98747749850, Ubuntu 98747749914, Windows 98747749918, dependencias 98747749919, macOS 98747749948 y GSP 98747750004. Sigue 7/8, NOT_READY; no hubo pruebas del código ni artefacto alojado nuevos. Las verificaciones locales anteriores conservan sus fechas, no se repitieron en este intento.

Históricamente, el usuario decidió dejar GitHub pendiente y se suspendieron consultas, reintentos y pushes hasta una nueva instrucción. Ese aplazamiento terminó el 2026-09-02, cuando el usuario solicitó proceder tras la renovación de cuota. El estado de release de ingeniería y `certified_compatibility=false` permanecen separados. G-Skills y Radar no se habilitan/certifican; no se amplían contratos v0.2 ni se materializa fork. La elección de un proyecto real sigue siendo posterior.

## Entrega local con CI aplazada

Guía ejecutada en 67950 exit 0, 2026-08-28T03:57:29Z, a partir de la copia del ZIP en `dist/entrega-local-0.2.0-f4b34e5/`. Se ejecutaron sus cuatro bloques exactos: 19 invocaciones CLI, recursos/specs/contratos/release, init/nota/checkpoint/reconcile/resume, render/verify, backup/restore/replay y audit. Estado final v3, resultado durable idéntico al reconciliar la restauración, DUPLICATE_REPLAY, destino restore existente rechazado 2, cero findings. Hash incorrecto rechazado antes de extraer. Procesos sin PATH, extracción temporal externa con espacios/acentos; no SQLite directo, `--as-human` ni proyecto real.

Se conserva el ZIP original byte por byte; los informes de entrega son externos al catálogo incorporado al binario. La carpeta local reúne ZIP, guía, informes, plan y matrices identificados mediante checksums. El registro sanitizado está en `release/prepilot-local-handoff.json`. Este ensayo nuevo no sustituye el gate completo anterior ni acredita GSP/ENOSPC o CI nuevos. No añade una etapa al plan ni convierte 7/8 en 8/8.

## Cierre alojado del 2026-09-02

- La cuota de Actions se observó renovada para el período de septiembre y los jobs volvieron a ejecutar pasos reales. El primer gate reveló, no ocultó, un fallo macOS: `killpg` devolvía EPERM cuando el proceso inundador ya era zombi. La corrección no tolera EPERM; cosecha el padre y exige un segundo ESRCH para confirmar que el grupo desapareció. Timeout, cancelación, límite de salida y descendientes conservaron sus pruebas negativas.
- [CI 33664393505](https://github.com/gcatalansc/G-Ledger/actions/runs/33664393505), SHA exacto `b171f51cb58c60ed8c8a99b60386117f9fed6822`, attempt 1: seis jobs PASS. ENOSPC `100362429074`, Windows `100362429340`, GSP real/supervisado `100362429361`, Ubuntu `100362429401`, macOS `100362429417` y dependencias `100362429451`.
- Artefacto alojado `project-ledger-windows-b171f51cb58c60ed8c8a99b60386117f9fed6822`, ID `9860800241`, no expirado al verificarlo y con retención informada hasta 2026-10-02. Contiene exactamente el ZIP y su informe. ZIP SHA-256 `a83624f2e795c8b8d3d9e15a316be38cce8999e437731b73f5f8c8964963ad80`; informe SHA-256 `91a7b1da712c664ba89ec40a3617494ee84e40f300ceff8be7e7cbe6c6e3770c`.
- Tras descargarlo a un destino temporal nuevo, el verificador local repitió hashes/recursos, specs/contratos/release, imports PE, entorno sin PATH/toolchain, un recorrido Codex y uno portable/backup/restore/replay/privacidad: 2/2 harnesses, 0 fallos. La copia descargada es ahora `dist/project-ledger-0.2.0-windows-x86_64.zip`; el build local del mismo SHA se conserva aparte y no se confunde con el artefacto alojado.
- PP-14 queda CLOSED y la etapa 8 queda verificada. No quedan defectos conocidos pendientes dentro del alcance pre-piloto acordado. Esto no certifica compatibilidad general, G-Skills/Radar, firma de código, GA, operación hostil aislada ni ausencia absoluta de errores.

**Siguiente paso:** el usuario elige un proyecto descartable o una copia de uno actual y autoriza un piloto acotado `init/bind → resume → progress/evidence → gate → checkpoint → reconcile → resume`. No intervenir un proyecto real ni ampliar integraciones sin esa elección.
