# Contribuir a Project Ledger

## Autoridad y alcance

1. Leer `SPEC.md`, la especificación normativa y `BITACORA_IMPLEMENTACION.md` antes de modificar contratos.
2. No editar ni reformatear `specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md`.
3. Citar los identificadores `REQ-*`, `AC-*` y el slice afectado en cambios contractuales.
4. Una discrepancia spec↔schema↔fixture bloquea el cambio; no se resuelve por lo que compile.
5. No introducir integraciones, persistencia o comportamiento de slices posteriores antes de superar el gate anterior.

## Calidad

Todo comportamiento se desarrolla mediante RED→GREEN→REFACTOR. Antes de un commit de cierre deben pasar formato, Clippy, tests, schemas, checksums, dependency policy y `git diff --check`.

## Commits

- Cada macrofase completada tiene un commit de cierre identificable.
- No se declara una fase completa si su gate contractual no tiene evidencia fresca.
- Los cambios generados de schemas y fixtures se versionan junto con sus tipos, pruebas y registry.
