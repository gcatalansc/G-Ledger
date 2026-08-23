# Project Ledger

Project Ledger es un motor local de continuidad verificable para proyectos desarrollados por personas y agentes.

El repositorio ha materializado la primera fase contractual (S0 + S1): preservación, workspace, contratos tipados, JSON canónico, schemas cerrados, fixtures y validación offline. Todavía no incluye event store, ejecución de comandos, proyecciones, gates, adapters, UI, daemon ni red.

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

La red no forma parte del runtime ni de las pruebas. La descarga inicial de dependencias es una operación de desarrollo de Cargo.
