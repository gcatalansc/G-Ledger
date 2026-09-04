# G-Ledger

G-Ledger, cuyo nombre técnico es Project Ledger, es una bitácora local y verificable para gobernar la continuidad de proyectos desarrollados por personas y agentes. Conserva fases, gates, decisiones, evidencia, blockers, riesgos, próximas acciones y checkpoints sin convertir Markdown, una IA o una integración externa en fuente de autoridad.

## Estado

- Producto: `0.2.0`.
- Aceptación: **`OPERATIONALLY_ACCEPTED` para uso operativo controlado**, piloto 8/8.
- Distribución: `ENGINEERING_RELEASE_CANDIDATE`.
- Compatibilidad certificada: `false`.
- Alcance primario: Windows x86_64, PowerShell 7, núcleo offline.
- General Availability: no declarada.
- Primera prueba sobre un proyecto real: pendiente y fuera de la aceptación actual.

La decisión operativa se apoya en el [informe final humano](docs/pilot/results/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md). No certifica G-Skills, GSP, Project Radar ni otra contraparte externa.

## Empiece aquí

Para utilizar la entrega Windows:

1. Descargue el ZIP y el archivo SHA-256 desde el [prerelease oficial `v0.2.0-rc.1`](https://github.com/gcatalansc/G-Ledger/releases/tag/v0.2.0-rc.1).
2. Compare el SHA-256 antes de extraer.
3. Extraiga el ZIP completo en una carpeta nueva.
4. Abra `START_HERE.md` dentro de la entrega.
5. Ejecute `project-ledger.exe resources verify --json`.

El recorrido de cinco minutos está en [Inicio rápido](docs/getting-started/QUICKSTART.md). El recorrido explicativo completo está en el [Tutorial](docs/getting-started/TUTORIAL.md).

## Documentación

El único punto de entrada documental es [docs/README.md](docs/README.md). Desde allí se accede, por audiencia y propósito, a:

- instalación, inicio rápido y tutorial;
- guía de usuario y referencia de la CLI;
- seguridad, privacidad, backup, portabilidad y recuperación;
- especificación normativa y procedencia del código;
- informe de aceptación y evidencia de ingeniería;
- desarrollo, contratos y planes internos.

Los documentos históricos y la evidencia hash-locked conservan sus rutas para no romper trazabilidad. El centro documental los clasifica; no es necesario buscarlos manualmente por el repositorio.

## Contenido de la entrega portable

El ZIP oficial contiene:

- `START_HERE.md`, como entrada inequívoca;
- `project-ledger.exe`, ejecutable Windows x86_64 con CRT estático;
- documentación de instalación, tutorial, uso, operación y aceptación;
- licencia, seguridad, changelog y notas de release;
- SBOM SPDX 2.3, checksums y manifiesto de contenido;
- recursos contractuales verificados que necesita el binario.

El núcleo no necesita Cargo, Git, Python, un checkout, un servicio ni red. GSP es opcional y requiere componentes externos fijados; su ausencia no afecta el uso normal de G-Ledger.

## Primer proyecto descartable

```powershell
$exe = '.\project-ledger.exe'
& $exe resources verify --json
& $exe init `
  --title 'Prueba descartable' `
  --summary 'Primer recorrido controlado' `
  --timezone 'America/Santiago' `
  --data-root '.\datos'
```

El comando devuelve `project_ref`, `database` y `state_version`. Esos identificadores deben conservarse literalmente; nunca se infieren a partir del nombre del proyecto.

## Autoridad y contratos

- Especificación normativa: [specs/PROJECT_LEDGER_SPEC_v0.2.0.md](specs/PROJECT_LEDGER_SPEC_v0.2.0.md).
- Índice de autoridad y digest esperado: [SPEC.md](SPEC.md).
- Handoff operativo para agentes: [referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md](referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md).
- Historial de implementación: [BITACORA_IMPLEMENTACION.md](BITACORA_IMPLEMENTACION.md).
- Cambios de producto: [CHANGELOG.md](CHANGELOG.md).

## Desarrollo y verificación

El build oficial usa Rust `1.97.1` y dependencias locked.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo build --locked --release -p project-ledger-cli
cargo xtask benchmark check
cargo xtask release check
cargo run -p project-ledger-cli -- release verify --root .
cargo deny check
```

Las instrucciones de compilación, estructura y procedencia están en [Fuente y build](docs/SOURCE_AND_BUILD.md). Las reglas de contribución están en [CONTRIBUTING.md](CONTRIBUTING.md) y el reporte de vulnerabilidades en [SECURITY.md](SECURITY.md).
