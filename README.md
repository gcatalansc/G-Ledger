# Project Ledger

Project Ledger es un motor local de continuidad verificable para proyectos desarrollados por personas y agentes.

El repositorio ha materializado S0 + S1 + S2 + S3 + S4: preservación, contratos cerrados, JSON canónico, SQLite/WAL append-only, cadena SHA-256 obligatoria, ejecución transaccional de comandos, idempotencia, concurrencia optimista, reconciliación de submissions, inicialización offline, proyección determinista de `ProjectState` y lifecycle S4. La caché de proyección es descartable y se reconstruye desde el stream canónico ante ausencia, obsolescencia o corrupción.

S4 incorpora autoridad versionada, roadmap secuencial por referencias de fase, pausa/reanudación/archivo humanos, decisiones, blockers, riesgos, próximas acciones, conflictos, estado agregado derivado y un catálogo local puro y reconstruible. El `CommandProcessor` entrega a las reglas lifecycle exactamente el stream cuya cadena verificó y conserva expected version, idempotencia, matriz cerrada y append atómico.

La semántica ejecutable sigue acotada por la especificación: gates, evidence/Phase Engine de cierre, checkpoints y resume de continuidad, renderer/import/export, adapters, stdio, UI, daemon, red y AI permanecen pendientes de S5 y slices posteriores.

## Autoridad

- Especificación normativa: [`specs/PROJECT_LEDGER_SPEC_v0.2.0.md`](specs/PROJECT_LEDGER_SPEC_v0.2.0.md)
- Antecedente inmutable: [`specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md`](specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md)
- Orden operativo del handoff: [`referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md`](referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md)
- Bitácora de implementación: [`BITACORA_IMPLEMENTACION.md`](BITACORA_IMPLEMENTACION.md)
- Matriz de cobertura contractual: [`docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`](docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md)

## Comandos de desarrollo

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
```

Inicialización offline contra una ubicación explícita:

```powershell
cargo run -p project-ledger-cli -- init `
  --title "Mi proyecto" `
  --summary "Continuidad verificable" `
  --timezone "America/Santiago" `
  --data-root ".tmp-user-data"
```

Sin `--data-root`, el CLI usa la convención de user-data del sistema operativo. Cada proyecto se almacena en `project-ledger/projects/<uuid>/ledger.db`; el contrato conserva el identificador canónico `project:<uuid>` y el directorio usa sólo el UUID para ser válido en Windows.

La red no forma parte del runtime ni de las pruebas. La descarga inicial de dependencias es una operación de desarrollo de Cargo.
