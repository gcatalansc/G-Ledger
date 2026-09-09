# G-Ledger 0.3.0-rc.1 — historia estructurada para LLM

## Clasificación

Versión de distribución: `0.3.0-rc.1`. Canal previsto: prerelease.
`ENGINEERING_RELEASE_CANDIDATE`, `operational_acceptance=NOT_GRANTED`,
`certified_compatibility=false`. Esta preparación no es una publicación ni GA.
La aceptación operativa de `v0.2.0-rc.1` permanece ligada a su sujeto anterior.

## Cambios desde v0.2.0-rc.1

- Episodios, alternativas, intentos, decisiones vinculadas y correcciones
  append-only. La escritura registra también el avance habitual.
- `history at`, `history diff`, `history get` y `decision trace` con cortes
  atómicos, digests, procedencia, ausencias explícitas y paginación estable.
- Portabilidad de streams mixtos v0.2/v0.3 y conservación del pasado ante
  correcciones posteriores.
- Confirmación histórica read-only para G-Skills; resultados recuperables tras
  reinicios/avances; conflictos de versión durables; recuperación vinculada al
  comando nativo completo.
- Correcciones de rutas largas Windows y de la prueba de supervisión cuyo
  timeout podía vencer antes de crear el marcador del auxiliar.

## Versiones y compatibilidad

El paquete ejecutable CLI declara `0.3.0-rc.1`. Las bibliotecas internas mantienen
su línea `0.2.0`; los contratos base siguen en `0.2.0` y la extensión de historia
en `0.3.0`. La versión de distribución no reescribe eventos ni contratos.

Un stream con episodios requiere el lector nuevo. No lo abra con rc.1 ni retire
eventos para forzar compatibilidad. Backup SQLite conserva resultados e
idempotencia; portable conserva historia y minimiza datos operacionales privados.

## Evidencia e interpretación

`release verify` comprueba el conjunto de evidencia base v0.2 e identifica
separadamente esta distribución. Un resultado correcto no otorga aceptación
operativa al nuevo ejecutable. El manifiesto `release/distribution.json` fija
versión, canal y clasificación; `package.json` añade commit y hashes del ZIP.

La calificación local PL-GSK-004 de 12/12 correspondió al producto `952cf77`
y al hash `5cde5898…`. No se transfiere automáticamente al ejecutable versionado
nuevo. La CI y el paquete extraído aportan evidencia propia, identificada por SHA.

## Uso

Extraiga la entrega completa en una carpeta nueva, compare su checksum y siga
`START_HERE.md`. El núcleo no requiere RAG, GSP, G-Skills, Cargo ni Python.
La guía específica está en [historia para LLM](history/LLM_USAGE.md).

La publicación, aceptación del candidato y adopción real siguen siendo decisiones
separadas de preparar y verificar la distribución.
