# Distribución 0.3.0-rc.1 — preparación y CI

Fecha: 2026-09-08. Base: `d916a7c`, rama `codex/history-core`.
Estado al emitir la preparación: **LOCAL_VERIFIED_PENDING_CI**.

## Resultado implementado

- `project-ledger-cli` declara `0.3.0-rc.1`. Las bibliotecas y sus dependencias
  internas permanecen en 0.2.0; Cargo.lock sólo cambia la versión del paquete CLI.
- `release/distribution.json` se genera desde Cargo metadata. El SBOM identifica
  la distribución nueva. No se regeneran aceptación, benchmarks ni contratos.
- `release verify` muestra `verified_evidence_subject=v0.2.0-baseline` y la
  distribución actual con `operational_acceptance=NOT_GRANTED`. Sus 90 criterios
  históricos no conceden aceptación operativa al nuevo binario.
- El ZIP perfil 2 exige que binario, manifiesto y notas coincidan en versión.
  Rechaza versiones mezcladas y aceptación heredada antes de crear un archivo.
- El verificador de instalación añade la versión y un recorrido histórico real;
  fija ambos selectores de ejecutable de los harnesses al binario extraído.
- CI admite un push a esta rama, mantiene Windows/Linux/macOS, dependencias,
  ENOSPC físico y GSP live (seis trabajos). Windows construye un solo release
  estático, y el ZIP toma el nombre de la versión CLI. Debug sin símbolos ni
  incremental reduce disco; la concurrencia retira ejecuciones obsoletas de la
  misma referencia.

## Evidencia local

- RED CLI: la respuesta anterior no distinguía distribución y aceptación;
  `distribution.operational_acceptance` era null en la prueba nueva.
- RED empaquetador: la implementación anterior no comprobaba la versión en
  preflight. Se preserva aparte el fallo inicial de preparación de la fixture.
- Regresión afectada: CLI sink 8, confirmación 8, historia 6, operaciones 22 y
  release 3, todos PASS; una omisión corresponde al auxiliar de proceso que
  las pruebas invocan expresamente.
- Metadatos: 3 pruebas xtask PASS. Total consolidado: **50 casos Rust PASS**,
  sin contar de nuevo las repeticiones de los tres casos release tras finalizar
  el SBOM. No se declara una pasada local de todo el workspace.
- Preflight real: **2 casos PASS**, rechazo de versión incompatible y aceptación
  indebida sin producir ZIP. Formato y sintaxis PowerShell PASS; Clippy de CLI y
  xtask, todos sus targets y sin analizar dependencias, PASS con `-D warnings`.
  La primera cualificación redundante de `Value` fue corregida antes del cierre.
- Schemas reproducibles, contratos base e historia y 485 recursos verificados.

Los logs se conservan en `.tooling/distribution-0.3.0-rc.1/`. Se reutilizó la
caché local, offline y con dos jobs; la construcción portable y sus recorridos
completos se reservan a la CI del commit exacto. No se duplica un ZIP local de
0.3.0-rc.1 ni se presenta el ZIP anterior 952cf77 como la distribución nueva.

## Estado de publicación y condiciones pendientes

La CI remota debe identificarse por SHA, conclusión, jobs y artefacto verificable.
La última CI observada antes del envío pertenece a rc.1, `22b3eca`, run
`33890755947`; no es evidencia para este candidato.

No se ha creado tag, merge ni GitHub Release. El piloto rc.1 y la calificación
bilateral 12/12 de PL-GSK-004 conservan sus sujetos anteriores. Publicación,
aceptación del nuevo candidato y adopción real siguen siendo decisiones separadas.
