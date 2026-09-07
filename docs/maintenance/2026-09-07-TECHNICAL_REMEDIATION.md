# Remediación técnica de G-Ledger — 2026-09-07

Base revisada: `22b3eca46ac6b963ab928e0131b0cdf88cd6627c`, tag `v0.2.0-rc.1`. Alcance autorizado: remediar los pendientes técnicos antes de preparar, con el propietario, un plan de investigación. Rama de trabajo: `codex/technical-remediation`.

Resultado: correcciones locales verificadas. Firma de código y certificación bilateral permanecen abiertas por dependencias externas. El commit que incorpora este registro identifica el cierre local; no se publicó una nueva release.

## Resultado por pendiente

| Pendiente | Tratamiento y evidencia |
|---|---|
| Fallo de SQLite/WAL con rutas largas | Reproducido con `CannotOpen` y exit 11 antes de `init`; corregido en la apertura central de conexiones y en respaldo/restauración/migración. Pruebas sobre bases de 257 y 1.150 caracteres. |
| Publicación atómica en rutas largas | Reproducido el error de `tempfile` al persistir import/artefactos. Se usan padres canónicos para que origen y destino de la publicación tengan rutas extendidas en Windows. Se conserva `persist_noclobber`. |
| Documentación desactualizada | Resumen actual de bitácora corregido, snapshot de S9 preservado como histórico, `risk close` incluido en CLI y publicación rc.1 enlazada desde aseguramiento. Checklist de entrega reconciliado con GitHub. |
| Firma de código | Dependencia externa abierta: revisión de `CurrentUser/My` y `LocalMachine/My`, cero certificados de firma disponibles con clave privada. Requiere identidad/certificado de editor y servicio de sellado temporal. No se creó confianza local artificial. |
| Certificación bilateral | Dependencia externa abierta. Los cuatro perfiles locales permanecen `COMPLETE`, todos con `certified_compatibility=false`. G-Skills conserva `PENDING_COUNTERPART_GSKILLS_S24`; Radar requiere extensión de contraparte y evidencia bilateral. La conformidad general de GSP y Agent Skill excede sus pruebas acotadas existentes. |
| Fork e idempotencia portable | Límites contractuales de v0.2, no defectos: `FORK_UNAVAILABLE` rechaza antes de escribir; portable omite resultados/idempotencia privados, mientras backup SQLite los conserva. No se modifican esos contratos bajo una corrección de portabilidad. |

## Causa y corrección

El SQLite nativo incluido usa por defecto el VFS Windows de rutas cortas y no añade automáticamente el prefijo extendido. Seleccionar sólo `win32-longpath` tampoco resolvió la regresión: es necesario normalizar la ruta absoluta y usar el espacio de nombres extendido cuando base o sidecars exceden el límite clásico. El VFS conserva los bloqueos; no se usan variantes `none`, APIs inseguras, cambios de esquema ni dependencias nuevas. Fuentes: [VFS de SQLite](https://www.sqlite.org/vfs.html) y [límites de rutas de Windows](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation).

Las operaciones de publicación de `tempfile` usan APIs nativas que tampoco expanden rutas. Canonicalizar el directorio ya existente antes de crear el temporal proporciona rutas compatibles para ambos extremos, conservando publicación atómica, sincronización y rechazo de destinos existentes.

## Verificación de esta corrección

- RED observado antes del arreglo en almacenamiento, CLI, intercambio de artefactos y publicación local del resultado GSP.
- GREEN focalizado: lectura concurrente de páginas WAL, cadena verificada, reapertura, respaldo/restauración, migración explícita, resultados conservados y rechazo de sobrescritura/base ausente.
- Recorrido público: init, registro, replay, checkpoint, render/verify, portable import/export, backup/restore, reconciliación, audit y rechazos de privacidad/fork, con rutas largas de datos y entorno sin PATH.
- La prueba de publicación GSP es local; no se contabiliza como ejecución ni certificación de la contraparte.
- `cargo fmt --all -- --check` y `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets --all-features`: 309 pruebas aprobadas, cero fallos, 109 resultados de suite; 14 entradas marcadas `ignored` en la ejecución normal. Siete son auxiliares invocados por sus pruebas propietarias; seis requieren GSP externo exacto y una requiere un volumen efímero ENOSPC. No se cuentan como siete pruebas externas aprobadas del candidato.
- `cargo xtask schema check`, `specs verify` y `contracts verify`: PASS; 46 schemas, 92 fixtures y 53 filas contractuales. Los hashes de ambas especificaciones se conservan.
- `cargo xtask benchmark check`, `cargo xtask release check` y `release verify --root .`: PASS. Se verificó la evidencia existente de 90 AC, 32 escenarios PL y nueve métricas; no se realizó una medición nueva ni se repitió el piloto histórico.
- `cargo deny check`: advisories, bans, licenses y sources PASS. `git diff --check`: PASS.
- `OfficialDelivery.Tests.ps1`: 9/9 PASS; enlaces locales de los cuatro documentos de entrada modificados, cero roturas; parser del verificador Windows, cero errores.
- Build `--locked --release --target x86_64-pc-windows-msvc` con CRT estático: PASS. Ejecutable SHA-256 `3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438`.
- Copia aislada fuera del checkout: 448 recursos coincidentes; `resources/specs/contracts/release verify` PASS y los tres recorridos de `codex_readiness`/`package_flow` PASS. Las llamadas del recorrido portable carecen de PATH. La copia de prueba se clasifica `LOCAL_TECHNICAL` / `NOT_REQUALIFIED`; no es un nuevo ZIP oficial ni una nueva aceptación humana.

Los logs de esta ejecución y el recibo del binario aislado están en `.tooling/technical-remediation/` (evidencia local ignorada por Git). La reproducción estable está en las pruebas versionadas y los comandos anteriores. No se disparó CI alojada ni se atribuyen a este commit los resultados Windows/Linux/macOS del SHA anterior.

Entorno: Windows x86_64, Rust 1.97.1, SQLite incluido por `rusqlite 0.40.2`; `LongPathsEnabled=1` observado, sin modificarlo. Las pruebas usan directorios y proyectos descartables. No se ejerció una base operativa real ni se alteró otro repositorio.

## Entrega histórica y límites

[PR #5](https://github.com/gcatalansc/G-Ledger/pull/5) fue integrado el 2026-09-04. El [run 33890755947](https://github.com/gcatalansc/G-Ledger/actions/runs/33890755947) terminó 6/6 success sobre el SHA base. La [release rc.1](https://github.com/gcatalansc/G-Ledger/releases/tag/v0.2.0-rc.1) se publicó ese día y su ZIP conserva SHA-256 `5abcff9728a7b7c8a77551636cc54cd3d2a4a1d8be92a337ad7a1d83d7f55391`. Estos datos se reconsultaron en GitHub durante esta revisión.

El informe humano, los JSON y la matriz del piloto se preservan como evidencia del sujeto aceptado. Las frases históricas sobre merge/release no describen la publicación posterior. Esta corrección es trabajo posterior a rc.1: la aceptación del piloto y la CI de aquel SHA no califican automáticamente el nuevo candidato. No se reemplazó la release ni se inició un proyecto de adopción o una investigación de mejoras.

Los directorios de trabajo de procesos, extractores ZIP y sistemas de archivos tienen límites propios. La cobertura nueva corresponde a rutas de datos en disco local; no certifica rutas UNC/red ni una longitud máxima universal.
