# Matriz contrato → schema → fixture → prueba

Esta matriz resume la cobertura ejecutable de S1. La relación individual y canónica de los 46 schemas con sus 92 fixtures está en `schemas/v0.2/schema-registry.json` y `schemas/v0.2/fixtures/manifest.json`; ambos ficheros son generados de forma determinista y se verifican byte a byte.

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
