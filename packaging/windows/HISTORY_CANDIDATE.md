# G-Ledger — candidato local de historia

Esta entrega permite revisar el núcleo histórico y la integración local con
G-Skills. Identidad de producto: `952cf77`; versión declarada del ejecutable:
`0.2.0`; extensión de historia: `0.3.0`. No tiene un nuevo tag ni aprobación de
publicación. `package.json` y el SHA-256 del ZIP identifican esta entrega.

## Uso

1. Compare el SHA-256 del ZIP con su archivo `.sha256`.
2. Extraiga el ZIP completo en una carpeta nueva.
3. Ejecute en PowerShell desde esa carpeta:

   ```powershell
   .\project-ledger.exe --version
   .\project-ledger.exe resources verify --json
   .\project-ledger.exe init --title "Historia de prueba" --data-root .\datos-prueba
   ```

4. Use la base y el identificador devueltos por `init` siguiendo la
   [guía para LLM](resources/docs/history/LLM_USAGE.md).

El núcleo funciona sin Cargo, Python, red ni G-Skills. Conserve el ejecutable
junto a `resources/`.

## Contenido y procedencia

- Episodios, alternativas, intentos, correcciones, reconstrucción por versión,
  diferencias y trazas de decisiones: [contrato de historia](resources/docs/history/HISTORY_EXTENSION_v0.3.md).
- Confirmación histórica, recuperación y conflictos de versión durables:
  [calificación local con G-Skills](docs/maintenance/PL-GSK-004.md).
- Integridad y composición: `package.json`, `CHECKSUMS.sha256`, `SBOM.spdx.json`.

El ejecutable es el mismo que obtuvo 12/12 casos bilaterales en PL-GSK-004.
Los informes anteriores conservan su fecha y alcance. La aceptación operativa de
`v0.2.0-rc.1`, incluida en algunos recursos históricos, no se transfiere a este ZIP.

## Límites de esta entrega

Estado: `LOCAL_REVIEW_CANDIDATE`; `operational_acceptance=NOT_GRANTED`;
`certified_compatibility=false`. La extensión v0.3 no cambia el número de versión
de paquete que muestra este ejecutable.

Use bases desechables para la revisión. Los eventos de episodio requieren este
lector: no abra una base que los contenga con rc.1. Para conservar resultados e
idempotencia utilice backup SQLite; el export portable conserva el historial,
pero minimiza los datos operacionales privados.

La publicación de una nueva versión requiere fijar su versión de distribución,
verificar ese sujeto en CI y decidir su aceptación. La adopción real y la mejora
comparativa de utilidad siguen pendientes.
