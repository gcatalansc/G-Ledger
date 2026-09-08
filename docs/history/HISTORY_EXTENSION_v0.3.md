# G-Ledger — extensión de historia v0.3

Estado: implementación autorizada, basada en `5c20e7b`; no es una release publicada.

## Propósito y arquitectura

Un LLM registra una actualización material una sola vez y reconstruye la trayectoria
de una decisión mediante datos explícitos. El stream SQLite/WAL sigue siendo la
autoridad; historia es una proyección determinista. GSP conserva compilación semántica
y RAG recuperación. No se deducen causas ni decisiones desde texto.

Se conserva el catálogo v0.2 byte por byte. La extensión añade un comando
`RECORD_EPISODE_UPDATE` y un evento `EPISODE_UPDATE_RECORDED`, versión 0.3.0,
con catálogos propios. Los eventos anteriores y sus digests no se migran. El mismo
stream admite ambas versiones. Los eventos auxiliares ya existentes conservan v0.2.

## Escritura

`record episode --file update.json` usa selección, expected-version, identidad,
idempotencia y observed-at habituales. Está permitido a `TrustedForRecording`.
El payload contiene `episode_ref`, `objective` (obligatorio al crear), `summary`
y `entries`. Cada entrada tiene `entry_id`, `corrects_ref` opcional y una variante:

- `ALTERNATIVE`: `description`, `rationale`.
- `ATTEMPT`: `alternative_ref`, `description`, `outcome` (`SUCCEEDED`, `FAILED`,
  `INCONCLUSIVE`), `evidence_refs`.
- `DECISION_LINK`: `decision_ref`, `alternative_ref`, `rationale`.

Los IDs son únicos en la historia del proyecto. Referencias nuevas deben existir
en el proyecto o en el mismo lote y tener el tipo correcto. Una corrección conserva
el original, tiene nueva identidad y reemplaza la interpretación actual de una
entrada del mismo tipo/episodio. No se corrige una entrada ya sustituida.
Los vínculos registran contexto; no aceptan decisiones ni verifican evidencia.
Alternativas sin motivo o relaciones nunca registradas permanecen ausentes.

Una escritura emite el evento de episodio y un `WORK_PROGRESS_RECORDED` en una
transacción, con la invalidación de checkpoint que corresponda. No requiere repetir
el avance en otra herramienta. El objetivo inicial es inmutable.

## Consultas

- `history at --version N`: estado y episodios que constaban hasta N.
- `history diff --from N --to M`: cambios estructurados y eventos del intervalo.
- `decision trace ID [--version N] [--cursor C]`: familia de supersesiones,
  episodios vinculados, entradas y estado de la evidencia referenciada.
- `history get REF [--version N]`: entrada, episodio, evento o registro canónico
  completo. Las referencias externas conservan URI/digest, sin lecturas de red.

Las respuestas son JSON v0.3 con project_ref, versión, digest, procedencia y
ausencias explícitas. Orden por stream_version y entry_id. Trace usa páginas de
50 entradas; el cursor liga proyecto, consulta, versión y digest. Nunca omite
texto silenciosamente. Get recupera el detalle completo.

Un corte debe coincidir con el final de un comando. Un corte intermedio se rechaza
con los límites válidos anterior/siguiente. Se conservan occurred_at y recorded_at,
pero no se reordenan eventos por ellos. Correcciones futuras no afectan lecturas
pasadas. Una evidencia invalidada cambia el soporte mostrado, no acepta/rechaza
automáticamente la decisión.

## Compatibilidad, entrega y validación

Nuevo lector admite v0.2 y la extensión. Portable debe transportar todos los eventos
sin pérdida; backups conservan el stream. No se crea otro almacén canónico.
No se alteran contratos GSP ni se reclama nueva certificación externa.

Implementación: contratos → registro/proyección → consultas/CLI → uso y validación.
Seis escenarios: trayectoria, cortes, correcciones/ausencias, escritura canónica,
compatibilidad y uso directo. Pruebas focalizadas durante cambios; una pasada final
de formato, Clippy, contratos y workspace. Evidencia E01 permanece histórica.
