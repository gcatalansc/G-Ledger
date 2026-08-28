# Operación local de la CLI

Estado de preparación: consultar `release/prepilot-readiness.json`. La disponibilidad de estos comandos no certifica todavía procesos supervisados, recuperación, GSP completo ni el ZIP Windows.

## Selección e identidad

Use `--database <ledger.db> --project-ref <project:uuid>`. También se admite `--locator <archivo.json>` con exactamente `database` y `project_ref`; una selección explícita completa prevalece incluso sobre un localizador ilegible. El localizador no ejecuta instrucciones, habilita permisos ni concede autoridad.

Las mutaciones reciben `--expected-version`, `--idempotency-key`, `--actor-id` y `--observed-at` (RFC3339). Conserve esos valores al repetir una solicitud. Los comandos nuevos usan recording trust por defecto; `--as-human` representa una acción humana local explícita, no una facultad que un agente deba atribuirse. La CLI es una frontera del usuario local, no un sandbox entre procesos del mismo usuario.

`--json` deja sólo JSON en stdout y diagnósticos en stderr. Una evaluación de gate puede devolver código 6 con un CommandResult ACCEPTED: se registró la evaluación, pero el gate no pasó. El resultado persistido, no el exit code aislado, describe los eventos emitidos.

## Operaciones disponibles

| Familia | Operaciones | Entrada |
|---|---|---|
| Proyecto | init, status, doctor, audit, verify, pause, resume-project, archive | Flags y selección explícita |
| Catálogo | catalog list, catalog rebuild | `--data-root`; list no escribe |
| Autoridad | authority add, refresh, list, supersede | URI/digest; refresh local no descarga; registrar reemplazo con `--supersedes` antes de sustituir |
| Repositorio | repository bind, refresh, revoke, list | Root existente; remote y snapshot declarados explícitamente, sin ejecutar Git/hooks |
| Roadmap | roadmap import, show | `--file` JSON/YAML contractual |
| Fases | phase list, start, request-close, reopen | ID; reason/authority cuando corresponda |
| Gates | gate list, show, define, evaluate, request-revalidation, waive | Definición JSON/YAML; waiver requiere aprobación humana coincidente y autoridad |
| Registros | record progress, note, notes; artifact add, list | Flags o payload completo |
| Conocimiento | claim record, status; unknown record, resolve | Afirmaciones iniciales ASSERTED, referencias explícitas |
| Evidencia | evidence run, import, link, unlink, show, external-list | Política explícita para run; import nunca conserva VERIFIED_SOURCE del archivo |
| Decisiones | decision propose, accept, supersede, show | JSON/YAML; un reemplazo debe estar propuesto y aceptado antes de sustituir |
| Impedimentos | blocker open, resolve; risk open, mitigate | Flags o payload completo |
| Continuidad | next set; session start/end; checkpoint create; resume; reconcile | Comandos existentes; sesiones admiten payload contractual |

`--file` admite el payload completo del comando o, para registros compuestos, el objeto contractual (por ejemplo, GateDefinition). Se verifican campos cerrados, digests y referencias; no se inventan gates o autoridades a partir de un nombre. Para next action AUTHORIZED use el payload completo con su autoridad; una clasificación no concede autoridad por sí sola.

## Reconciliación

```text
project-ledger reconcile --database <db> --project-ref <project> --command-type RECORD_WORK_PROGRESS --actor-id agent:codex --idempotency-key <key> --json
```

La consulta usa proyecto, tipo de comando, actor, dominio de confianza y clave. Devuelve el resultado original verificado. Ausencia devuelve OUTCOME_UNKNOWN (12), no una orden de reintentar con otra clave. Para comandos históricos ejecutados con LOCAL_USER, como checkpoint create, añada `--trust-domain LOCAL_USER`; esto sólo selecciona el scope de lectura.

## Evidencia ejecutada

```json
{"allowed_programs":["<ruta absoluta al ejecutable autorizado>"],"redactions":[],"max_summary_bytes":4096}
```

`evidence run` requiere `--policy`, `--cwd`, identidad de escritura, `--kind` y `-- <exe> <args...>`. No usa una cadena de shell. El parser genérico no está cualificado y produce OBSERVED; un exit 0 no basta para VERIFIED_SOURCE ni para completar un gate. La misma solicitud aceptada se reconcilia antes de volver a ejecutar el proceso. Las garantías adicionales de timeout, entorno y descendientes se cierran en la etapa 4.

## Evidencia de implementación

`crates/cli/tests/operations.rs` ejecuta procesos reales y transacciones SQLite descartables: familias normativas, replay, scopes de actores, conflictos 5/10, política 9, DB ausente 4, queries sin migraciones, YAML, referencias, gates y catálogo. Las fixtures aportan entradas contractuales; no simulan una ejecución externa de GSP.
