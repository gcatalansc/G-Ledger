# Project Ledger — Windows portable

El núcleo funciona localmente en Windows x86_64, sin instalar Cargo, Git, Python ni un servicio, y no necesita red. Extraer el ZIP completo en una carpeta propia (puede contener espacios/acentos). No separar `project-ledger.exe` de `resources/`. No instalar sobre una carpeta existente: conservar la versión previa para rollback.

Comprobar primero el SHA-256 del ZIP con el publicado en la entrega. `project-ledger.exe resources verify --json` comprueba los recursos contra hashes incorporados al binario. `resources list --json` muestra ese catálogo. Esto detecta corrupción; no es una firma ni autentica un binario reemplazado maliciosamente junto con su manifiesto.

Los recursos se localizan junto al ejecutable; `--resources <directorio>` permite una selección explícita y prioritaria que debe coincidir exactamente con esa versión. El directorio de trabajo no concede autoridad ni selecciona recursos. Los archivos de código bajo resources son evidencia de ingeniería para el verificador, no un checkout ejecutable ni herramientas requeridas.

## Primer uso descartable

Ejecutar `project-ledger.exe init --title "Prueba descartable" --data-root <directorio de datos>`. Conservar project_ref y database devueltos; no inferirlos de nombres. Consultar `resume --database <database> --project-ref <project_ref> --format json` y `--help` de cada familia. Para un repositorio existente, usar `repository bind` con el actor humano explícito y la versión esperada: el vínculo no modifica sus archivos de trabajo ni concede permisos.

La skill Codex y sus argv están en `resources/integrations/agent-skills/project-ledger-continuity/`. No se instala globalmente ni ejecuta sus ejemplos automáticamente. Los registros, evidencia, gates, checkpoints y reconciliación usan los comandos de la CLI. Una evaluación aceptada puede retornar 6 y dejar el gate pendiente; nunca equivale por sí sola a una fase completada.

`render` produce Markdown derivado; no editarlo como fuente de verdad. `export --format portable` exporta historia verificada, no secretos ni todos los resultados privados. `backup` conserva el estado SQLite/WAL completo, resultados e idempotencia: es un respaldo privado, no compartible. `restore` escribe sólo un destino nuevo. `migrate` exige respaldo previo explícito. El fork materializado sigue no disponible en v0.2.

## GSP opcional y fijado

GSP requiere por separado Python 3.12, Git y el entorno congelado del compiler `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, junto al spec `004b14d7b64837025671970eac2469f092c49475`. Contrato `gsp-exact-2026.04.12`. No se descargan ni actualizan automáticamente, no se sustituyen por versiones recientes y su ausencia no impide usar el núcleo.

`integration gsp compile` exige Python, source-root, cwd y policy explícitos. La policy autoriza el ejecutable y cada variable necesaria (p. ej. PATH para Git, SystemRoot y GSPGEMMA_UPSTREAM_ROOT); no copiar todo el entorno. Límites iniciales: 60 segundos y 1 MiB por flujo. Corpus máximo 1 MiB; resultado máximo 8 MiB. El resultado validado es COMPILED_UNLINKED; enlazar requiere `link-result` separado y autorizado. La supervisión limita recursos de procesos propios, no es sandbox para código hostil arbitrario.

G-Skills y Radar no están habilitados para este piloto. Los contratos vendorizados conservan pruebas de regresión, no activación ni certificación. La entrega es para piloto controlado, no GA, precisión semántica universal, aceptación bilateral ni ausencia absoluta de errores. La evidencia y el SHA concreto del ZIP están en el informe externo de entrega, no inferidos de este manual.
