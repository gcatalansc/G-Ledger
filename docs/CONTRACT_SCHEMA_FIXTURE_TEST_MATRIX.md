# Matriz contrato → schema → fixture → prueba

Esta matriz resume la cobertura contractual de S1 y la ejecución transaccional incorporada en S2. La relación individual y canónica de los 46 schemas con sus 92 fixtures está en `schemas/v0.2/schema-registry.json` y `schemas/v0.2/fixtures/manifest.json`; ambos ficheros son generados de forma determinista y se verifican byte a byte.

| Familia contractual | Contratos tipados | Schemas | Fixtures | Pruebas principales |
|---|---:|---:|---:|---|
| Protocolo de comandos/eventos | 53 comandos, 67 eventos y 4 envelopes/resultados auxiliares | 6 | 12 | `closed_unions.rs`, `command_event_matrix.rs`, `fixtures.rs` |
| Proyecto, lifecycle y evidencia | 20 raíces durables | 20 | 40 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Continuidad, import y export | 9 raíces durables | 9 | 18 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Integración genérica | 6 raíces durables | 6 | 12 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| Perfil Project Radar 0.1.1 | 5 raíces durables | 5 | 10 | `contracts.rs`, `registry.rs`, `fixtures.rs` |
| **Total de raíces registradas** | **46** | **46** | **92** | `generation.rs`, `contract_commands.rs` |

## Invariantes automatizados

- Cada raíz tipada produce exactamente una entrada de registro con `$id` único.
- Cada entrada del registro tiene exactamente un fixture positivo y uno negativo.
- Todos los `$ref` son fragmentos internos resolubles sin red.
- Los positivos validan y los negativos fallan por semántica closed-world.
- La matriz contiene exactamente una fila por cada uno de los 53 comandos y asigna los 67 eventos a un origen de comando o a un owner derivado.
- La regeneración produce 142 ficheros idénticos, JSON con LF final y digests canónicos estables.

## Cobertura ejecutable S2

| Requisito | Evidencia principal | Frontera demostrada |
|---|---|---|
| I-01, I-27, AC-006, AC-008 | `event_chain.rs`, `recovery.rs` | Génesis, append-only, versiones contiguas, previous digest, roundtrip y corrupción fail-closed |
| I-04 y §15–16 | `integrity.rs`, `command_transaction.rs` | Toda escritura canónica nace de `CommandEnvelope`; payload/envelope/result usan digest canónico |
| I-06, AC-038 | `command_transaction.rs` | Expected version stale produce resultado persistente y cero append |
| I-07, AC-036–037 | `command_transaction.rs` | Mismo scope/key/digest: un efecto; digest distinto: rechazo cerrado |
| Matriz comando-evento, AC-039 | `command_transaction.rs` | Un handler no puede emitir un evento fuera del outcome/possible set cerrado |
| I-26, PL-08, PL-20, AC-040–041 | `submission_reconciliation.rs` | `OUTCOME_UNKNOWN` sólo local; historial CAS; accepted/rejected/not-found sin reintento ciego |
| AC-001 y §39–40 | `init.rs`, `bootstrap.rs`, `recovery.rs` | Init offline, SQLite/WAL real, migración idempotente, rollback y recovery report |

Las 61 pruebas del workspace incluyen 23 pruebas nuevas de S2. No certifican operación remota ni semántica de projection/lifecycle/gates, que comienza en S3.
