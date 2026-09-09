# G-Ledger 0.3.0-rc.1 — plan de distribución y CI

> Ejecución inline autorizada por el propietario. Aplicar TDD y verificación
> proporcional; no delegar ni publicar una release por inferencia.

**Objetivo:** identificar la distribución histórica como 0.3.0-rc.1 y ejecutar
una CI del commit candidato sin transferirle la aceptación de rc.1.

**Arquitectura:** versionar el paquete CLI distribuido; las bibliotecas internas
y los contratos congelados conservan sus versiones. Un manifiesto generado de
distribución separa la identidad candidata de la evidencia histórica v0.2.
El empaquetador y la CLI exponen esa misma identidad. CI conserva la cobertura
existente y evita compilar dos binarios release de Windows.

**Herramientas:** Rust/Cargo 1.97.1, PowerShell 7, Git y GitHub Actions.

## Restricciones

- Base `d916a7c`, rama `codex/history-core`; no tocar `main` ni el tag rc.1.
- Distribución `0.3.0-rc.1`, contratos base `0.2.0`, historia `0.3.0`.
- `operational_acceptance=NOT_GRANTED`, `certified_compatibility=false`.
- No regenerar aceptación, benchmarks ni certificación por cambiar la versión.
- Dos jobs locales, caché existente, sin símbolos/incremental para los tests.
- Un envío a CI del candidato final; no repetir jobs sin causa observada.
- No publicar ni crear tags. Facturación/autenticación bloqueadas se documentan.

## 1. Identidad de distribución

Archivos: `crates/cli/Cargo.toml`, `Cargo.lock`, `xtask/src/release.rs`,
`crates/cli/src/main.rs`, `crates/cli/resources.txt`, `crates/cli/tests/release.rs`,
`release/distribution.json`, SBOM/inventario/checksums generados.

- [x] Añadir prueba CLI: `--version` devuelve `project-ledger 0.3.0-rc.1` y
  `release verify` conserva la evidencia base pero declara distribución sin
  aceptación operativa. Ejecutarla RED sobre el código anterior.
- [x] Fijar la versión del CLI y generar `release/distribution.json` desde
  `cargo metadata`. Validar que la identidad cargada coincide con el ejecutable.
- [x] Regenerar sólo metadatos derivados con `cargo xtask release generate`.
  Verificar CLI/contratos afectados y metadatos con la caché local existente.

## 2. Paquete coherente

Archivos: `scripts/package-windows.ps1`, `scripts/test-windows-package.ps1`,
`scripts/build-windows-package.ps1`, entrada/assurance de Windows y nuevas notas
`docs/RELEASE_NOTES_v0.3.0-rc.1.md`.

- [x] Preparar empaquetado perfil `project-ledger-windows-zip/2`, versión y aceptación
  tomadas del manifiesto; rechazar un binario de otra versión.
- [ ] Verificar versión, hashes, clasificación y recorridos existentes contra
  el ZIP extraído. Las notas del piloto siguen identificadas como históricas.
- [ ] Construir el release portable una vez en CI; no producir otro ZIP local
  duplicado si la evidencia local ya cubre la preparación.

## 3. CI del candidato

Archivos: `.github/workflows/ci.yml`, `BITACORA_IMPLEMENTACION.md` y registro
de calificación `docs/maintenance/DISTRIBUTION_0.3.0-rc.1.md`.

- [x] Habilitar push para `codex/history-core`, conservar los seis jobs y usar
  un nombre de ZIP obtenido de la versión CLI. Evitar el release Windows previo
  al build estático que ya realiza `build-windows-package.ps1`.
- [x] Formato, Clippy acotado, tests afectados y chequeo de metadatos antes de
  guardar un commit local y enviarlo a su rama remota.
- [x] Consultar SHA, conclusión, jobs y artefacto exactos de esa ejecución.
  Ante un fallo, leer su causa y corregir sólo lo que corresponda; una restricción
  de infraestructura no se sustituye por un PASS local.
- [x] Registrar resultado y condición de entrada para decidir la publicación.

Resultado: candidato `973db3e`, CI `34297786152`, **BLOCKED_CI_BILLING**.
Seis trabajos con cero pasos y cero artefactos; el paquete portable y sus
recorridos de CI permanecen pendientes. No repetir hasta regularizar GitHub.
