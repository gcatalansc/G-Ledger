# Operación local de la CLI

Estado vigente: G-Ledger 0.2.0 está `OPERATIONALLY_ACCEPTED` para uso operativo controlado y se distribuye como `ENGINEERING_RELEASE_CANDIDATE`. La disponibilidad de un comando no certifica una integración externa, no promueve General Availability y mantiene `certified_compatibility=false`.

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
| Impedimentos | blocker open, resolve; risk open, mitigate, close | Flags o payload completo |
| Continuidad | next set; session start/end; checkpoint create; resume; reconcile | Comandos existentes; sesiones admiten payload contractual |

`--file` admite el payload completo del comando o, para registros compuestos, el objeto contractual (por ejemplo, GateDefinition). Se verifican campos cerrados, digests y referencias; no se inventan gates o autoridades a partir de un nombre. Para next action AUTHORIZED use el payload completo con su autoridad; una clasificación no concede autoridad por sí sola.

## Reconciliación

```text
project-ledger reconcile --database <db> --project-ref <project> --command-type RECORD_WORK_PROGRESS --actor-id agent:codex --idempotency-key <key> --json
```

La consulta usa proyecto, tipo de comando, actor, dominio de confianza y clave. Devuelve el resultado original verificado. Ausencia devuelve OUTCOME_UNKNOWN (12), no una orden de reintentar con otra clave. Para comandos históricos ejecutados con LOCAL_USER, como checkpoint create, añada `--trust-domain LOCAL_USER`; esto sólo selecciona el scope de lectura.

## Evidencia ejecutada

```json
{"allowed_programs":["<ruta absoluta al ejecutable autorizado>"],"redactions":[],"max_summary_bytes":4096,"process":{"timeout_ms":60000,"max_stdout_bytes":1048576,"max_stderr_bytes":1048576,"environment":{}}}
```

`evidence run` requiere `--policy`, `--cwd`, identidad de escritura y `-- <exe> <args...>`; `--kind` vale TEST_RESULT por defecto. El programa debe ser absoluto y autorizado; en Windows debe ser un ejecutable nativo, no .cmd/.bat. No usa una cadena de shell. `environment` es el entorno completo, no una extensión del padre: PATH y credenciales no se heredan. Amplíe límites y entorno solamente en la política explícita.

El parser genérico no está cualificado y produce OBSERVED; un exit 0 no basta para VERIFIED_SOURCE ni para completar un gate. Los resúmenes se acotan y redactan; los bytes stdout/stderr son efímeros. El cwd y la intención de ejecución se representan por digests en la evidencia, sin persistir argv o valores del entorno. Una evidencia importada con patrones sensibles soportados se rechaza.

Ctrl-C solicita cancelación; también se puede crear el archivo absoluto indicado en `--cancel-file`. Timeout, cancelación o exceso de salida devuelven 12 con un código específico, sin aprobar ni emitir evidencia exitosa. El supervisor termina la familia propia y confirma limpieza dentro de un plazo adicional de 2 segundos. En Windows un Job Object no heredable también termina esa familia si se mata abruptamente al supervisor. No es un sandbox de código hostil ni un cortafuegos.

Antes de ejecutar se reserva durablemente la clave y un digest de la solicitud en metadatos SQLite. Si queda una reserva sin resultado, repetir devuelve OUTCOME_UNKNOWN sin volver a ejecutar; una solicitud diferente devuelve 10. Reconciliar una solicitud ya registrada recupera el resultado y replay no ejecuta. Si el proceso termina con exit distinto de cero, se registra su evidencia OBSERVED y la CLI devuelve 3; el CommandResult ACCEPTED significa registro aceptado, no prueba aprobada.

La migración SQLite interna 3 contiene estas reservas, sin ampliar contratos canónicos. `migrate --backup <directorio nuevo>` respalda antes de cambiar el esquema; las consultas nunca migran por sí solas.

## Incorporación, vistas y recuperación

`render`, `verify --artifact`, `export --format portable`, `import`, `import-markdown`, `backup`, `restore` y `migrate` están descritos con sus límites y revisión explícita en [portabilidad y recuperación](PREPILOT_PORTABILITY_RECOVERY.md). El portable conserva el historial, pero no resultados/idempotencia operacionales; un backup SQLite completo sí los conserva. Restore exige un destino nuevo. Fork materializado no está disponible en v0.2.

## Evidencia de implementación

`crates/cli/tests/operations.rs` ejecuta procesos reales y transacciones SQLite descartables: familias normativas, replay, scopes de actores, conflictos 5/10, política 9, DB ausente 4, queries sin migraciones, YAML, referencias, gates y catálogo. Las fixtures aportan entradas contractuales; no simulan una ejecución externa de GSP.
