# Aseguramiento y aceptación

Este índice separa el resultado humano, la evidencia machine-readable y los antecedentes de ingeniería. El lector ejecutivo puede detenerse en el primer bloque; auditoría y desarrollo pueden profundizar sin mezclar niveles.

## Aceptación del piloto y entrega publicada

El informe y la matriz conservan el corte del piloto del 2026-09-04. Las frases sobre ausencia de merge o release describen ese momento. La entrega posterior quedó integrada en [PR #5](https://github.com/gcatalansc/G-Ledger/pull/5) y publicada como [v0.2.0-rc.1](https://github.com/gcatalansc/G-Ledger/releases/tag/v0.2.0-rc.1), commit `22b3eca46ac6b963ab928e0131b0cdf88cd6627c`, con [CI 6/6 success](https://github.com/gcatalansc/G-Ledger/actions/runs/33890755947).

Las correcciones posteriores y sus límites se consultan en el [registro de mantenimiento](../maintenance/2026-09-07-TECHNICAL_REMEDIATION.md). Esa verificación técnica no cambia el sujeto ni la decisión humana del piloto.

- [Informe final humano del Piloto de Aceptación Operativa](../pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md).
- [Acta de decisión humana exacta](../pilot/results/stage-8-risk-close-operational-acceptance.json).
- [Reporte machine-readable consolidado](../pilot/results/operational-pilot-report.json).
- [Matriz de escenarios](../pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md).
- [Especificación normativa del piloto](../pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md).

Resultado: 8/8, `OPERATIONALLY_ACCEPTED` para uso operativo controlado. El resultado no equivale a GA, certificación universal, prueba sobre un proyecto real ni certificación de productos externos.

## Evidencia por etapa

Los JSON bajo [`docs/pilot/results/`](../pilot/results/) preservan identidad del sujeto, escenarios, hashes, findings, gates y decisiones de cada etapa. Son evidencia para herramientas y auditoría; no sustituyen el informe humano.

## Preparación prepiloto

- [Cobertura del núcleo](../PREPILOT_CORE_COVERAGE.md).
- [Cobertura de la CLI](../PREPILOT_CLI_COVERAGE.md).
- [Seguridad de procesos](../PREPILOT_PROCESS_SAFETY.md).
- [Portabilidad y recuperación](../PREPILOT_PORTABILITY_RECOVERY.md).
- [Procedimiento Codex](../PREPILOT_CODEX_PROCEDURE.md).
- [Entrega Windows](../PREPILOT_WINDOWS_DELIVERY.md).
- [Matriz GSP acotada](../PREPILOT_GSP_RUNTIME_MATRIX.md).
- [Matriz de brechas](../PREPILOT_GAP_MATRIX.md).

## Release y seguridad

- [Release manifest](../../release/release-manifest.json).
- [Acceptance report](../../release/acceptance-report.json).
- [Compatibility report](../../release/compatibility-report.json).
- [SBOM SPDX](../../release/sbom.spdx.json).
- [Checksums de evidencia](../../release/checksums.sha256).
- [Threat model](../THREAT_MODEL.md).

## Interpretación correcta

`PASS` significa que el escenario identificado pasó sobre el sujeto y entorno declarados. No amplía el alcance a sistemas no ejecutados. `EXCLUDED_BY_SCOPE` nunca se interpreta como PASS. La evidencia prepiloto aporta contexto, pero no sustituye evidencia `LIVE_PILOT`.
