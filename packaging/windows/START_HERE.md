# G-Ledger 0.2.0 — empiece aquí

Esta carpeta es una entrega portable oficial de G-Ledger para Windows x86_64. No requiere instalador ni privilegios administrativos.

## Secuencia obligatoria

1. Antes de extraer, compare el SHA-256 del ZIP con el archivo `.sha256` publicado en la misma GitHub Release.
2. Extraiga el ZIP completo en una carpeta nueva; no sobrescriba una versión anterior.
3. Abra PowerShell 7 en esta carpeta.
4. Ejecute:

```powershell
.\project-ledger.exe --version
.\project-ledger.exe resources verify --json
.\project-ledger.exe specs verify --json
.\project-ledger.exe contracts verify --json
.\project-ledger.exe release verify --json
```

5. Lea [`documentation/README.md`](documentation/README.md).
6. Siga [`documentation/getting-started/QUICKSTART.md`](documentation/getting-started/QUICKSTART.md).

## Qué contiene esta entrega

- `project-ledger.exe`: ejecutable del producto.
- `documentation/`: instalación, tutorial, guía de usuario, operación y procedencia.
- `assurance/`: informe humano y evidencia machine-readable de aceptación.
- `resources/`: contratos y recursos verificados requeridos por el binario.
- `package.json`, `CHECKSUMS.sha256` y `SBOM.spdx.json`: identidad, integridad y composición.
- `LICENSE`, `SECURITY.md`, `CHANGELOG.md` y `RELEASE_NOTES.md`: condiciones de entrega.

## Estado y límites

G-Ledger 0.2.0 está `OPERATIONALLY_ACCEPTED` para uso operativo controlado y se distribuye como `ENGINEERING_RELEASE_CANDIDATE`. `certified_compatibility=false` permanece vigente. Este prerelease no es General Availability y no certifica G-Skills, GSP o Project Radar.

No mueva `project-ledger.exe` fuera de esta carpeta ni elimine `resources/`. No adjunte bases, backups o localizadores privados a un reporte. Para vulnerabilidades, consulte [`SECURITY.md`](SECURITY.md).
