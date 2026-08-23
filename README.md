# Project Ledger

Project Ledger es un motor local de continuidad verificable para proyectos desarrollados por personas y agentes.

El repositorio ha materializado S0–S6: preservación, contratos cerrados, JSON canónico, SQLite/WAL append-only, cadena SHA-256 obligatoria, ejecución transaccional de comandos, idempotencia, concurrencia optimista, reconciliación de submissions, inicialización offline, proyección determinista de `ProjectState`, lifecycle, gates/evidencia y continuidad. La caché de proyección es descartable y se reconstruye desde el stream canónico ante ausencia, obsolescencia o corrupción.

S5 incorpora runner local sin shell y con allowlist, importación de evidencia con degradación de confianza, los 17 operadores cerrados de `PredicateAst`, `GateEvaluation` inmutable, waivers humanos, claims inicialmente `ASSERTED`, cierre explícito por Phase Engine y revalidación por evidencia, autoridad o snapshot de repositorio. El `CommandProcessor` entrega a los motores exactamente el stream cuya cadena verificó y conserva expected version, idempotencia, matriz cerrada y append atómico.

S6 incorpora checkpoints inmutables con captura N/evento N+1, freshness append-only, versiones material/no material separadas, `ResumeProjection` compacto y para agente bajo cotas conservadoras, Markdown derivado de trece secciones con perfiles de privacidad y auditoría de divergencia. `CoreCommandHandler` registra progreso/unknowns/checkpoints e invalidaciones compatibles en una sola transacción SQLite; varios repositorios vigentes se rechazan sin selección implícita.

La semántica ejecutable sigue acotada por la especificación: portable bundles, legacy review/import, adapters, stdio, UI, daemon, red y AI permanecen pendientes de S7 y slices posteriores. Los motores de gates/evidencia/continuidad existen como core embebible; sus superficies CLI operativas completas se conectarán junto con las superficies posteriores sin inventar selección implícita de ledger/proyecto. `GeneratedArtifactRecord` se devuelve junto con los bytes renderizados, pero no se persiste porque el contrato v0.2.0 no define un command/event para ello.

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
