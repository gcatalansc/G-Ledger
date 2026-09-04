# Fuente, build y procedencia

## Fuente oficial

El repositorio oficial es [gcatalansc/G-Ledger](https://github.com/gcatalansc/G-Ledger). Cada GitHub Release incluye los archivos fuente generados por GitHub para el tag exacto. El ZIP portable distribuye el ejecutable; no necesita un checkout para operar.

La versión normativa es `0.2.0`. La autoridad de implementación es [PROJECT_LEDGER_SPEC_v0.2.0.md](../specs/PROJECT_LEDGER_SPEC_v0.2.0.md), cuyo digest esperado figura en [SPEC.md](../SPEC.md).

## Entorno reproducible de ingeniería

- Rust estable fijado: `1.97.1`.
- Dependencias: `Cargo.lock`, obligatorio con `--locked` en build de entrega.
- Target portable: `x86_64-pc-windows-msvc`.
- CRT: estático.
- Core: sin Python, Node, JVM, Docker, daemon o red.
- Empaquetado: PowerShell 7 mediante `scripts/build-windows-package.ps1`.

## Build local

```powershell
git status --short
cargo build --locked --release -p project-ledger-cli
```

El binario de desarrollo queda bajo `target/release/`. No lo presente como entrega oficial: el ZIP oficial se crea con target Windows static-CRT, manifiesto, documentación, hashes y verificación desde una revisión limpia.

## Build del ZIP oficial

```powershell
$output = Join-Path $PWD 'dist/project-ledger-0.2.0-windows-x86_64.zip'
pwsh -File scripts/build-windows-package.ps1 -Output $output -RequireClean
```

El builder:

1. compila con dependencias locked y CRT estático;
2. contrasta recursos compilados contra el source tree;
3. copia una allowlist revisada;
4. genera manifiesto y checksums;
5. extrae en una ruta nueva con espacios y caracteres no ASCII;
6. verifica integridad, contratos, release y escenarios de operación sin toolchain.

## Calificación

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
git diff --check
```

La CI repite los gates principales en Windows, Linux y macOS, y ejecuta pruebas hosted separadas para ENOSPC físico y GSP fijado. Un release oficial debe usar el ZIP producido por la corrida exitosa de `main` para el mismo SHA del tag.

## Identidad y confianza

`source_commit` dentro de `package.json` liga el paquete a Git. `CHECKSUMS.sha256` y el manifiesto verifican integridad interna. El SHA-256 publicado junto al ZIP verifica la descarga. No existe firma de código en v0.2.0; esa limitación debe conservarse en cualquier entrega.

