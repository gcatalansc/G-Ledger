# Centro de documentación de G-Ledger 0.2.0

Este es el punto de entrada único para la documentación activa. La ruta recomendada evita mezclar instrucciones de uso con especificaciones, evidencia de aceptación o planes internos.

## Quiero instalarlo y usarlo

Lea en este orden:

1. [Inicio rápido](getting-started/QUICKSTART.md): instalación y primer proyecto en cinco minutos.
2. [Tutorial guiado](getting-started/TUTORIAL.md): recorrido completo de una sesión con progreso, checkpoint y reanudación.
3. [Guía de usuario](operations/USER_GUIDE.md): modelo operativo cotidiano, responsabilidades y recuperación.
4. [Referencia operativa de la CLI](CLI_OPERATIONS.md): familias de comandos, concurrencia, idempotencia y evidencia.
5. [Windows portable](WINDOWS_PORTABLE.md): instalación, verificación, rollback y límites del paquete.

## Quiero evaluar la entrega

- [Estado de la remediación técnica posterior a rc.1](maintenance/2026-09-07-TECHNICAL_REMEDIATION.md): correcciones verificables y dependencias externas pendientes.
- [Contenido oficial de la entrega](DELIVERY_CONTENTS.md).
- [Notas de release `v0.2.0-rc.1`](RELEASE_NOTES_v0.2.0-rc.1.md).
- [Aseguramiento y aceptación](assurance/README.md).
- [Informe final humano del piloto, corte 2026-09-04](pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md).
- [Matriz histórica del piloto](pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md).
- [Threat model](THREAT_MODEL.md).
- [Fuente, build y procedencia](SOURCE_AND_BUILD.md).

## Quiero integrar o desarrollar

- [Recuperación histórica PL-GSK-003](maintenance/PL-GSK-003.md): corrección interna, revisión del contrato y alcance pendiente de autorización.
- [Corrección G-Skills PL-GSK-002](maintenance/PL-GSK-002.md): conflicto nativo recuperable ante un avance anterior al envío, regresiones y límites de recalificación bilateral.
- [Extensión de historia v0.3](history/HISTORY_EXTENSION_v0.3.md) y [uso directo por un LLM](history/LLM_USAGE.md): episodios, intentos, correcciones y reconstrucción por versión; incorporación posterior a rc.1.
- [Entrega del núcleo de historia y verificación](history/IMPLEMENTATION_REPORT.md): capacidades implementadas, seis escenarios y ejercicio directo.

- [Especificación normativa v0.2.0](../specs/PROJECT_LEDGER_SPEC_v0.2.0.md).
- [Handoff Codex](../referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md).
- [Matriz contrato→schema→fixture→test](CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md).
- [Skill de continuidad](../integrations/agent-skills/project-ledger-continuity/SKILL.md).
- [Guía de contribución](../CONTRIBUTING.md).
- [Planes ejecutables](superpowers/plans/).

## Quiero evaluar su utilidad y evolución

- [Investigación de utilidad: resultados y estado](research/README.md): diez episodios en cuatro proyectos, alternativas, calibración mecánica y doce intentos comparativos E01. Utilidad comparativa y adopción aún no demostradas.
- [Informe E01 y siguiente decisión](research/e01/REPORT.md): comparación con documentación mantenida, costes observados, límites y adjudicación pendiente.
- [Plan autorizado v0.1](research/G_LEDGER_UTILITY_RESEARCH_PLAN_v0.1.md) y [protocolo comparativo E01](research/EXPERIMENT_PROTOCOL_v0.1.md).

## Mapa de directorios documentales

| Ubicación | Propósito | Audiencia |
|---|---|---|
| `docs/getting-started/` | Inicio rápido y tutorial | Usuario nuevo |
| `docs/operations/` | Modelo de operación | Operador/maintainer |
| `docs/maintenance/` | Correcciones posteriores a la entrega y evidencia de verificación | Operador/desarrollo |
| `docs/research/` | Investigación de utilidad, protocolos y decisiones basadas en evidencia | Propietario/investigación |
| `docs/pilot/` | Especificación, matriz y evidencia OPA | Auditor/propietario |
| `docs/assurance/` | Índice de aseguramiento y límites | Empresa receptora |
| `docs/PREPILOT_*.md` | Evidencia histórica prepiloto, preservada | Auditor |
| `docs/superpowers/plans/` | Planes de implementación internos | Desarrollo |
| `specs/` | Contrato normativo y compatibilidad | Arquitectura/desarrollo |
| `release/` | Manifiestos, SBOM y evidencia machine-readable | Release/auditoría |

Las rutas históricas no se reorganizan físicamente cuando están referenciadas por hashes, manifests o evidencia. Su clasificación en este índice es deliberada: preserva integridad sin obligar al lector a navegar archivos dispersos.

## Autoridad

Si dos documentos discrepan, rige este orden:

1. [Especificación normativa v0.2.0](../specs/PROJECT_LEDGER_SPEC_v0.2.0.md) y [extensión de historia v0.3](history/HISTORY_EXTENSION_v0.3.md), cada una dentro de su alcance/versionado.
2. Schemas, locks y fixtures cerrados de la versión.
3. Estado derivado del stream canónico de cada Ledger.
4. Guías y tutoriales.
5. Evidencia histórica y planes internos.

Markdown es documentación o proyección; nunca sustituye el stream SQLite/WAL verificado de un proyecto.
