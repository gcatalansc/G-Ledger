# Changelog

Todos los cambios relevantes de Project Ledger se documentan en este archivo.

## [Unreleased]

### Added

- Baseline documental v0.2.0 y preservación verificable de v0.1.0.
- Plan de implementación de la primera fase contractual S0 + S1.
- Esqueleto del workspace Rust, CI y políticas del repositorio.
- JSON canónico RFC 8785, SHA-256 tipado, UUIDv7, timestamps RFC 3339 y referencias validadas.
- Contratos durables S1 y uniones cerradas de 53 comandos y 67 eventos.
- Registro offline de 46 schemas con 92 fixtures, semántica de arrays y matriz comando-evento.
- CLI read-only para verificar antecedentes y contratos, además de generación determinista mediante `xtask`.
- Puertos `event-store`/`application` independientes de SQLite y adapter `storage-sqlite` con migración v1, WAL, claves foráneas, bloqueo acotado y `synchronous=FULL`.
- Append de eventos inmutable, versiones contiguas, cadena SHA-256 obligatoria, lectura verificada y detección fail-closed de corrupción.
- Procesador de comandos con matriz cerrada, expected version, idempotencia por scope, resultados persistentes y handler real `CREATE_PROJECT`.
- Historial append-only de `ExternalSubmissionRecord`, compare-and-set y reconciliación sin reintento ciego ni `OUTCOME_UNKNOWN` en resultados servidor.
- `project-ledger init` offline y reporte de recuperación con `quick_check`, WAL, migración, cadena, conteos y submissions inciertas.
- Gate local reproducible con formato, Clippy pedantic, 61 pruebas y política de dependencias.
