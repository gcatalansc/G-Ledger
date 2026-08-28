# Etapa 5 — incorporación, portabilidad y recuperación

Estado: etapa 5 verificada localmente, 5/8. Seis recorridos CLI, regresión de metadata y gate agregado completos; CI y ZIP siguen pendientes de etapa 8.

## Interfaces y límites

- Proyecto nuevo: `init --data-root <directorio de datos>` crea una identidad nueva. Repositorio existente: `repository bind --root <repo>` registra un binding explícito; no instala hooks ni modifica archivos de trabajo. La prueba de operaciones conserva el único archivo original del repositorio descartable.
- `render --output <vista.md> [--privacy minimal-local|shareable|full-local]` produce Markdown y `<vista.md>.record.json`, ambos derivados. `verify --artifact <vista.md>` y `audit --artifact` contrastan bytes, contrato, fuente y una nueva renderización. Editar ambos archivos no los convierte en autoridad. `render --regenerate` sólo reemplaza una pareja de generación comprobada, nunca un archivo de trabajo arbitrario.
- `export --format portable --output <directorio nuevo> [--phase-catalog <json>]`: árbol v0.2, cadena, estado, resume, Markdown compartible, referencias y locks. El catálogo explícito aporta metadata de fases cuando el roadmap la necesita; el contrato de eventos conserva IDs, no ese catálogo completo. El archivo de proyección Radar es parte del árbol normativo; no habilita ni ejecuta una integración Radar.
- `import --file <directorio portable> --database <destino> [--project-ref <id>]`: nuevo proyecto, cadena idéntica o fast-forward atómico. Verifica antes de crear destino; divergencia, destino más nuevo e identidad distinta se rechazan. `--mode fork` devuelve FORK_UNAVAILABLE antes de escribir. No se amplían los contratos.
- Lectura portable: 64 MiB y 256 archivos por defecto, árbol fijo sin directorios extra ni symlinks. Biblioteca permite presupuesto explícito. No hay descompresión ni descarga. Material privado en eventos inmutables provoca rechazo 13: no se redacta y re-sella silenciosamente la historia. Para continuidad local completa se usa backup privado.
- `backup --output <directorio nuevo>` usa SQLite online backup, incluyendo WAL y todas las tablas. Snapshot validado, cerrado y sincronizado; manifest SHA-256 se escribe al final. Una carpeta interrumpida sin manifest válido no es un respaldo utilizable.
- `restore --file <backup> --database <archivo nuevo>` verifica bytes y metadata en una copia temporal propia, sin tocar el respaldo original; publica sin sobrescritura. Conserva resultados, idempotencia y reservas de procesos inciertos. No migra al restaurar un esquema histórico soportado.
- `migrate --backup <directorio nuevo>` excluye escritores, valida el esquema histórico y respalda antes de modificarlo. Migraciones en una sola transacción; fallo de respaldo impide migración. Las consultas siguen rechazando versiones antiguas sin migrarlas.

Un backup es una copia privada completa: puede contener información local ya registrada y requiere la misma protección que la base. No es el mecanismo de exportación compartible. Los hashes detectan corrupción/divergencia, no prueban autoría frente a alguien que pueda sustituir todo el respaldo y su manifest.

## Markdown histórico revisado

`import-markdown --file <historia.md> --review-id <id> --observed-at <timestamp> --preview [--output <nuevo.json>]` no cambia el proyecto. Tiene límite 1 MiB y 1024 candidatos antes de vincular contradicciones.

Aplicación: sustituir `--preview` por `--apply-reviewed --review <decisiones.json> --preview-digest <digest> --as-human --actor-id <revisor> --expected-version <N> --idempotency-key <clave>` y selección de proyecto. Cada decisión contiene `candidate_ref`, `action`, `actor_ref` y `reason`. Acciones: IMPORT_AS_ASSERTED, ARCHIVE_ONLY o IGNORE; REVIEW no se aplica. Requiere una decisión por candidato y el mismo revisor explícito.

Sólo las decisiones IMPORT_AS_ASSERTED producen claims ASSERTED. El texto conserva tipo original, digest/URI de fuente, sección/líneas, candidato, links de contradicción, digest de revisión y revisor. No crea objetos operativos actuales, evidencia verificada, fases completadas ni gates aprobados; la salida enumera estas materializaciones no representables. LEGACY_IMPORT_REVIEWED sigue prohibido por la matriz v0.2.

Cada claim usa su comando, versión y clave derivada estable. No se promete atomicidad entre varios comandos: si una carrera interrumpe el lote, se informan resultados parciales. Reconciliar o repetir exactamente la revisión recupera los resultados ya aceptados sin duplicarlos; cambiar revisión bajo las mismas claves se rechaza. No reintentar con claves nuevas.

## Evidencia y verificación

RED observado: comandos CLI ausentes; auditoría aceptaba eliminar idempotency_keys o alterar su command_ref; lector portable ignoraba directorio extra vacío; Markdown admitía más de 1024 candidatos. GREEN: seis recorridos CLI, metadata de recuperación, portable y legacy. La prueba WAL mantiene un lector anterior al append y comprueba páginas WAL pendientes antes de respaldar. Privacidad, divergencia y referencias cruzadas también se comprobaron.

Gate fresco (sesión 92829, exit 0): formato, Clippy estricto, tests workspace/all-targets/all-features, build locked, schemas, specs, contratos, release check, cargo deny offline, 13/13 SHA-256 y diff. No nueva ejecución opt-in GSP/ENOSPC, CI alojada o ZIP; estas evidencias no se atribuyen a este cierre.

El destino nuevo se publica sólo después de verificar. El backup/migración verifica cadena, reconstrucción, estructura SQLite, foreign keys, metadata de migraciones, comandos/resultados/idempotencia y reservas. La cache de proyección es reemplazable; restaurarla no la convierte en autoridad.
