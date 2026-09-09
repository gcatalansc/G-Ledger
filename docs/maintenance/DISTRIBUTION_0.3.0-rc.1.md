# Distribución 0.3.0-rc.1 — preparación y CI

Fecha: 2026-09-08. Base: `d916a7c`, rama `codex/history-core`.
Estado al emitir la preparación: **LOCAL_VERIFIED_PENDING_CI**.
Estado posterior al envío: **BLOCKED_CI_BILLING**, candidato `973db3e479c06a354f672d38a96fa0fd147a7dc3`.

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

## CI observada — cierre de esta intervención

El candidato se envió a `origin/codex/history-core`, repositorio privado
`gcatalansc/G-Ledger` (id 1347951316), sin modificar main ni crear tags.
La revisión automática inicialmente rechazó el push por falta de verificación
del destino/payload. Tras comprobar mediante la API la propiedad, privacidad y
permisos, y delimitar los 18 commits/155 archivos pendientes, aprobó el mismo push.
El bloqueo de transferencia quedó resuelto.

La ejecución [34297786152](https://github.com/gcatalansc/G-Ledger/actions/runs/34297786152)
corresponde exactamente a `973db3e479c06a354f672d38a96fa0fd147a7dc3`, evento push,
intento 1. GitHub la cerró con `failure`: sus seis trabajos tienen cero pasos
y no produjeron artefactos. Las seis anotaciones coinciden: trabajos no iniciados
por pagos recientes fallidos o por necesidad de aumentar el límite de gasto;
GitHub remite a **Billing & plans**.

Esto no es un FAIL de las pruebas del candidato ni una CI aprobada. La compilación
portable 0.3.0-rc.1 y su validación de instalación siguen pendientes. No se generan
reintentos automáticos ni builds locales duplicados para sustituir esa evidencia.

La captura de ejecución, trabajos, anotaciones, artefactos y hashes de evidencia
local queda en [DISTRIBUTION_0.3.0-rc.1.json](DISTRIBUTION_0.3.0-rc.1.json).
El commit documental posterior usa `[skip ci]` porque sólo registra el bloqueo;
la referencia de calificación sigue siendo el candidato `973db3e`.

**Condición de reanudación:** regularizar pagos/límite de GitHub y reejecutar esta
CI sobre el mismo sujeto; confirmar seis trabajos y verificar el ZIP de Windows
por SHA antes de proponer publicación/aceptación. No se cambió la facturación.
