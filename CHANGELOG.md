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
- Puertos `event-store`/`application` independientes de SQLite y adapter `storage-sqlite` con migraciones v1–v2 verificadas, WAL, claves foráneas, bloqueo acotado y `synchronous=FULL`.
- Append de eventos inmutable, versiones contiguas, cadena SHA-256 obligatoria, lectura verificada y detección fail-closed de corrupción.
- Procesador de comandos con matriz cerrada, expected version, idempotencia por scope, resultados persistentes y handler real `CREATE_PROJECT`.
- Historial append-only de `ExternalSubmissionRecord`, compare-and-set y reconciliación sin reintento ciego ni `OUTCOME_UNKNOWN` en resultados servidor.
- `project-ledger init` offline y reporte de recuperación con `quick_check`, WAL, migración, cadena, conteos y submissions inciertas.
- `projection-engine` storage-independent con digest canónico de `ProjectState`, rebuild desde génesis, aplicación incremental equivalente y rechazo explícito de reducers aún no implementados.
- Caché SQLite descartable ligada a versión/head/projector/digest, con reconstrucción y reparación automática ante ausencia, obsolescencia o corrupción.
- Consulta de estado que verifica primero la cadena canónica y mantiene invariantes de no mutación tras 100 lecturas consecutivas.
- Cursor lifecycle con reducers fail-closed para autoridad, roadmap secuencial, fases, decisiones, blockers, riesgos, próximas acciones y conflictos; `ProjectStateProjector` v2 exige estado derivado vigente al final del stream.
- Motor puro de comandos S4 conectado al stream verificado por `CommandProcessor`, con transiciones humanas explícitas, estado derivado determinista e IDs UUIDv7 separados criptográficamente por comando/ordinal.
- Catálogo local derivado y reconstruible exclusivamente desde proyectos conocidos explícitos, sin adquirir autoridad ni escanear filesystem.
- Gate local reproducible con formato, Clippy pedantic, 90 pruebas y política de dependencias.
