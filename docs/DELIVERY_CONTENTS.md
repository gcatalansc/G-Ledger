# Contenido de la entrega oficial Windows

La entrega portable está diseñada para que una empresa receptora pueda identificar, verificar, ejecutar y auditar el producto sin conocer la estructura del repositorio.

## Estructura del ZIP

```text
G-Ledger 0.2.0/
├── START_HERE.md
├── project-ledger.exe
├── LICENSE
├── CHANGELOG.md
├── SECURITY.md
├── RELEASE_NOTES.md
├── SBOM.spdx.json
├── CHECKSUMS.sha256
├── package.json
├── documentation/
│   ├── README.md
│   ├── SOURCE_AND_BUILD.md
│   ├── getting-started/
│   │   ├── QUICKSTART.md
│   │   └── TUTORIAL.md
│   ├── operations/
│   │   ├── USER_GUIDE.md
│   │   └── CLI_OPERATIONS.md
│   └── WINDOWS_PORTABLE.md
├── assurance/
│   ├── G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md
│   ├── operational-pilot-report.json
│   ├── G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md
│   ├── acceptance-report.json
│   ├── compatibility-report.json
│   └── release-manifest.json
└── resources/
    └── inventario contractual verificado por el ejecutable
```

## Responsabilidad de cada elemento

| Elemento | Propósito |
|---|---|
| `START_HERE.md` | Entrada humana y secuencia de verificación |
| `project-ledger.exe` | Producto ejecutable |
| `documentation/` | Instalación, tutorial, uso, operación y build |
| `assurance/` | Resultado humano y evidencia machine-readable |
| `resources/` | Schemas, contratos, locks y evidencia incorporada requerida por verificadores |
| `package.json` | Inventario exacto, tamaños, hashes, target y source commit |
| `CHECKSUMS.sha256` | Hash legible de todos los archivos inventariados |
| `SBOM.spdx.json` | Composición de software en formato SPDX 2.3 |
| `SECURITY.md` | Límites y canal de reporte |

## Elementos deliberadamente excluidos

- bases SQLite, WAL, backups y localizadores del usuario;
- secretos, credenciales, logs crudos o conversaciones;
- `.git/`, `target/`, `dist/`, `.tooling/` y cachés;
- toolchains de Rust/Python y checkouts externos;
- G-Skills, GSP o Project Radar como productos ejecutables;
- una declaración de GA o compatibilidad externa certificada.

El código fuente completo se obtiene mediante los archivos fuente del mismo tag en GitHub. El ZIP portable contiene el ejecutable y todo lo necesario para operar y evaluar el núcleo sin un checkout.

