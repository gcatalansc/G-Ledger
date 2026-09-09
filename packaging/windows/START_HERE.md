# G-Ledger 0.3.0-rc.1 — empiece aquí

Esta carpeta es una distribución candidata de G-Ledger para Windows x86_64. No requiere instalador ni privilegios administrativos. Compruebe versión y commit en `package.json`.

## Secuencia obligatoria

1. Antes de extraer, compare el SHA-256 del ZIP con el archivo `.sha256` entregado junto a él.
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
7. Para registrar y reconstruir episodios, siga la [guía para LLM](documentation/history/LLM_USAGE.md).

## Qué contiene esta entrega

- `project-ledger.exe`: ejecutable del producto.
- `documentation/`: instalación, tutorial, guía de usuario, operación y procedencia.
- `assurance/`: informes históricos con su alcance original; la clasificación del candidato está en `release/distribution.json`.
- `resources/`: contratos y recursos verificados requeridos por el binario.
- `package.json`, `CHECKSUMS.sha256` y `SBOM.spdx.json`: identidad, integridad y composición.
- `LICENSE`, `SECURITY.md`, `CHANGELOG.md` y `RELEASE_NOTES.md`: condiciones de entrega.

## Estado y límites

G-Ledger 0.3.0-rc.1 es `ENGINEERING_RELEASE_CANDIDATE`, con `operational_acceptance=NOT_GRANTED` y `certified_compatibility=false`. El piloto de rc.1 no concede aceptación a este binario. `release verify` comprueba la evidencia base v0.2 y muestra separadamente la identidad candidata. Este prerelease no es General Availability.

Revise el candidato con datos desechables. No abra con rc.1 una base que contenga episodios v0.3. Consulte [RELEASE_NOTES.md](RELEASE_NOTES.md).

No mueva `project-ledger.exe` fuera de esta carpeta ni elimine `resources/`. No adjunte bases, backups o localizadores privados a un reporte. Para vulnerabilidades, consulte [`SECURITY.md`](SECURITY.md).
