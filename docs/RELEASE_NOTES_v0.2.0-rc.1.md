# G-Ledger 0.2.0-rc.1 — candidato operativo aceptado

## Clasificación

- Decisión de uso: `OPERATIONALLY_ACCEPTED` para uso operativo controlado.
- Estado de distribución: `ENGINEERING_RELEASE_CANDIDATE`.
- Compatibilidad certificada: `certified_compatibility=false`.
- Canal GitHub: **prerelease**.
- Plataforma portable incluida: Windows x86_64.
- General Availability: esta entrega no es General Availability.

## Propósito

Esta es la primera entrega oficial y autocontenida de G-Ledger 0.2.0 para incorporación controlada. Reúne el ejecutable, la documentación de usuario, tutorial, referencia operativa, evidencia de aceptación, SBOM, manifiestos y checksums en un ZIP verificable.

## Capacidades incluidas

- bitácora append-only con cadena SHA-256;
- estado derivado reconstruible y proyecciones Markdown no autoritativas;
- autoridad documental, roadmap, fases, gates y lifecycle gobernado;
- evidencia, claims, decisiones, blockers, riesgos y unknowns;
- concurrencia optimista, idempotencia y reconciliación de resultados inciertos;
- checkpoints y `resume` para continuidad entre personas y agentes;
- export/import portable, backup/restore SQLite y auditoría;
- Agent Skill local y perfiles de integración acotados;
- verificadores offline de schemas, contratos, recursos y release.

## Instalación

1. Descargue el ZIP y su archivo `.sha256` desde esta misma release.
2. Compare el SHA-256.
3. Extraiga todo en una carpeta nueva.
4. Abra `START_HERE.md`.
5. Ejecute `project-ledger.exe resources verify --json`.

No requiere instalador, privilegios administrativos, Cargo, Git, Python, servicio o red para el núcleo.

## Evidencia de aceptación

El Piloto de Aceptación Operativa ejecutó ocho etapas y los escenarios OPA-S01…OPA-S30/S32. OPA-S31, correspondiente a Project Radar, permaneció `EXCLUDED_BY_SCOPE` y no se contabilizó como PASS. La decisión humana final autorizó uso operativo controlado y conservó cero riesgos, unknowns críticos, conflictos, defectos abiertos, waivers críticos o findings sin resolver en el sujeto aceptado.

El ZIP publicado por esta release se construye y verifica nuevamente en CI para el SHA exacto de `main`. La verificación de entrega no reescribe el historial del piloto ni amplía sus límites.

## Límites conocidos

- No se ha ejecutado todavía un proyecto real de adopción.
- No existe firma de código; se publican hashes y manifiestos de integridad.
- No se certifica compatibilidad bilateral con G-Skills, GSP o Project Radar.
- El counterpart de Project Radar, UI, daemon general y AI embebida están fuera de alcance.
- GSP es opcional y requiere un entorno externo fijado; no forma parte del núcleo portable.
- El export portable minimiza datos y no conserva resultados/idempotencia privados; el backup SQLite sí.

## Rollback y recuperación

Instale cada versión en una carpeta nueva y conserve el ZIP anterior. No sobrescriba instalaciones. Antes de migrar datos, cree un backup explícito; `restore` exige una base de destino nueva.

## Contenido y procedencia

Consulte `START_HERE.md`, `CHECKSUMS.sha256`, `package.json` y `SBOM.spdx.json` dentro del ZIP. El `source_commit` del manifiesto debe coincidir con el commit del tag. GitHub ofrece además los archivos fuente del mismo tag.

## Seguridad

No adjunte bases, backups, localizadores privados ni evidencia sin sanitizar a un reporte. Siga `SECURITY.md` para comunicar una vulnerabilidad.

