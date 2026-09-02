# Etapa 4 — procesos y privacidad

Estado: etapa 4 verificada en Windows; avance agregado 4/8, NOT_READY. Gate completo local: formato, Clippy estricto, workspace/all-targets/all-features, build locked, schemas, specs, contratos, release, cargo deny online, 13/13 checksums y diff. Las duplicidades transitivas permitidas son warnings, no fallos. CI multiplataforma fresca y paquete quedan para etapa 8.

## Implementación

- Supervisor compartido en `crates/evidence/src/supervisor.rs`: ejecutable nativo absoluto autorizado por el caller, argv literal, cwd explícito, stdin cerrado, entorno completo allowlisted sin herencia y captura concurrente.
- Defaults: 60 segundos, 1 MiB stdout y 1 MiB stderr. Al superar un límite se termina la familia propia. La confirmación de limpieza y drenaje tiene un plazo adicional de 2 segundos; una limpieza no confirmada se informa como error, nunca éxito.
- Windows: grupo Job Object más un job exterior no heredable con KILL_ON_JOB_CLOSE, asignados antes de reanudar el hijo suspendido; la comprobación incluye la lista de procesos del job. Unix: grupo propio y SIGKILL al finalizar/cancelar. No es un sandbox para código hostil ni un cortafuegos.
- Ctrl-C/SIGTERM/SIGHUP donde aplica y archivo explícito de cancelación. Ninguna enumeración/terminación global de procesos por nombre.
- Resumen inicial 4 KiB, truncado en frontera UTF8; redacción configurada y patrones soportados. El entorno explícito también aporta valores a redactar. Los bytes raw no se serializan ni persisten por defecto. El parser CLI no cualificado siempre produce OBSERVED.
- Importación sensible: rechazo tipado 13 sin eco del valor; importaciones aceptadas no conservan VERIFIED_SOURCE. Un proceso con exit distinto de cero puede registrar evidencia OBSERVED, pero la CLI devuelve 3 y no aprueba un gate.
- Antes de iniciar un proceso, SQLite registra una reserva inmutable por scope y digest de solicitud. Resultado canónico reconciliado implica replay sin ejecución; reserva sin resultado implica OUTCOME_UNKNOWN, no retry. No se añaden comandos/eventos v0.2: `execution_attempts` es metadata operacional de la migración interna 3.

## Evidencia focal observada

| Prueba | Observación |
|---|---|
| `process_safety` | RED de credencial ficticia heredada y executable relativo; GREEN tras entorno vacío/validación. Resumen UTF8 acotado, display sensible redactado y status OBSERVED |
| `supervision` | Hang → timeout; token → cancel; flood stdout/stderr/ambos → corte; entorno y argv con metacaracteres literales; exit no cero observable |
| Descendientes | Heartbeat real se detiene tras timeout y salida normal del padre; no sólo se verifica el PID raíz |
| Terminación abrupta Windows | RED: heartbeat siguió cambiando tras matar al supervisor propio. GREEN: job exterior no heredable termina descendientes aun sin Drop del supervisor |
| `execution_reservations` | Reapertura conserva reserva; dos conexiones reservan una sola vez; modificaciones rechazadas y tampering detectado; versión/eventos canónicos no cambian |
| CLI `operations` | Timeout, no retry, solicitud distinta rechazada, fallo conocido registrado, replay sin ejecución y gate no aprobado; cancel-file real; privacidad con exit 13 sin eco ni evento |
| Importación | Token ficticio no entra en el registro; fixtures de confianza y digests siguen verificándose |

Los helpers marcados ignored no se contabilizan como suites externas pendientes: se invocan explícitamente como procesos hijos por sus tests padres. Las tres pruebas opt-in del GSP real y ENOSPC físico sí continúan pendientes de nueva ejecución; no son sustituidas por estos helpers.

## Límites y siguiente verificación

Las garantías demostradas corresponden a procesos propios autorizados, no a contención universal de binarios maliciosos. La terminación abrupta del supervisor se demuestra específicamente en Windows; en Unix se verifica supervisión/cancelación del grupo durante ejecución normal. No se promete reconocer todos los formatos secretos desconocidos.

La migración CLI con backup/restauración de estas reservas se cierra en etapa 5. Los canales Markdown/portable y el ZIP se comprueban al quedar disponibles en etapas 5/8. Ningún proyecto real fue intervenido.

Dependencias de control revisadas desde su código publicado: [process-wrap 10.0.0](https://docs.rs/process-wrap/10.0.0/process_wrap/), [win32job 2.0.3](https://docs.rs/win32job/2.0.3/win32job/) y [ctrlc 3.5.2](https://docs.rs/ctrlc/3.5.2/ctrlc/). Cargo.lock fija los bytes resueltos; no se incorporó unsafe al código propio.

## Revalidación macOS durante el cierre de etapa 8

La primera CI ejecutable posterior a la renovación de cuota detectó que el caso de desbordamiento devolvía `Cleanup` en macOS. La instrumentación temporal demostró `killpg: PermissionDenied (EPERM)`; la [fuente XNU](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/kern_sig.c) explica que Darwin excluye zombis al recorrer el grupo y retorna EPERM en modo POSIX cuando no encuentra un miembro señalable. No se convirtió EPERM en éxito.

La corrección final, en `b171f51cb58c60ed8c8a99b60386117f9fed6822`, intenta cosechar al padre mediante el wrapper de grupo y exige que un segundo intento devuelva ESRCH antes de considerar desaparecido el grupo. Si el padre sigue vivo, el grupo persiste, existe un descendiente no señalable o surge otro error, el resultado continúa siendo `Cleanup`. La CI `33664393505`, job macOS `100362429417`, pasó la suite completa; Windows `100362429340` y Ubuntu `100362429401` confirmaron regresión. Estado agregado posterior: **8/8, APTO PARA PILOTO CONTROLADO**, con los límites de certificación documentados en la entrega Windows.
