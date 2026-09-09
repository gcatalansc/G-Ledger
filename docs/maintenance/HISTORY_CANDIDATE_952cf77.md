# Candidato local de historia — 2026-09-08

## Resultado y alcance

Se corrige la fragilidad de la prueba CLI de 700 ms y se prepara una entrega
local revisable. El producto sigue siendo `952cf77c7f92fc3c3ceedb3b01383b8c40ffaf57`;
la preparación parte de `ac2dce0fd4a9234b5bd951caf493fc199e647c96` y sólo modifica
pruebas y documentación. No cambia código productivo, recursos compilados,
dependencias, versión de paquete, bases operativas ni la entrega publicada rc.1.

El ejecutable optimizado de PL-GSK-004 se reutiliza byte por byte:
`5cde5898ba6e3bc6decb3245295b808cc0813904585b26b63dd2fce14b64775b`.
Así se conserva la identidad de la calificación bilateral 12/12 existente; no se
presenta la preparación como otra calificación bilateral ni como aceptación operativa.

## Causa y corrección de la prueba

`supervise` inicia su reloj antes de crear el proceso. El timeout puede cumplirse
antes de la primera escritura del auxiliar. La prueba abría incondicionalmente
un marcador que sólo creaba ese auxiliar; `NotFound` era una suposición incorrecta
del arnés, no evidencia de un fallo al detener el proceso.

Se añade al auxiliar un modo controlado que espera antes de escribir el marcador.
La prueba con la aserción anterior falla por el mismo `NotFound` (RED, exit 101).
La corrección crea el archivo de observación vacío desde el padre y admite cero
o una escritura en el intento con timeout. Se cubren ambos modos, con claves
distintas, sin aumentar los 700 ms ni repetir automáticamente una prueba fallida.

Ambos intentos deben dar `EXECUTION_TIMEOUT`, después `OUTCOME_UNKNOWN`, conservar
el marcador y no avanzar el estado. La ejecución con fallo conocido debe escribir
exactamente una vez y su replay no debe repetirla. Se conservan los controles de
idempotencia, gate fallido y ausencia del canario privado en archivos persistidos.
La prueba independiente de cancelación conserva la exigencia de arranque real.

El incidente de PL-GSK-003-CONFIRMATION y su repetición aislada permanecen como
evidencia histórica. Esta corrección posterior cierra esa causa identificada;
no afirma ausencia universal de sensibilidad temporal en cualquier otra prueba.

## Verificación de esta preparación

- Operaciones CLI completas: **22 PASS, 0 FAIL, 1 ignored**, 10,70 s.
  La omisión es el auxiliar `execution_fixture`, ejecutado expresamente como hijo
  por las pruebas de supervisión/cancelación.
- Historia: **6 PASS, 0 FAIL**, 3,86 s, incluidos replay, correcciones, conflictos,
  trazas, cursor fijo y portable mixto v0.2/v0.3.
- Ambos harnesses se ejecutan con `CARGO_BIN_EXE_project-ledger` fijado al
  ejecutable extraído del ZIP; los datos SQLite son temporales. Se utilizan dos
  hilos de prueba y no se repite el workspace ni la campaña bilateral.
- Integridad: 493 payloads por longitud/SHA-256 y 495 entradas totales coinciden.
  `resources verify --json` desde la instalación extraída verifica 484 recursos.
- Formato y revisión estática: resultados fijados en el informe JSON adyacente.

Logs de reproducción, compilación y verificación en `.tooling/history-candidate/`.
El resumen durable y sus hashes están en
[HISTORY_CANDIDATE_952cf77.json](HISTORY_CANDIDATE_952cf77.json).

## Entrega preparada

Archivo local: `dist/g-ledger-history-952cf77-windows-x86_64.zip`, 12.754.442 bytes.
SHA-256: `a41882bf50d0ea8dd82f6d5965c215364a1f1e8e746336b5a96444c01f2df66a`.
Se acompaña de `.zip.sha256`; contiene `START_HERE.md`, el ejecutable, sus recursos,
documentación de alcance, SBOM y manifiesto/inventario de hashes.

Perfil: `g-ledger-local-review-candidate/1`, `LOCAL_REVIEW_CANDIDATE`,
`operational_acceptance=NOT_GRANTED`, `certified_compatibility=false`,
`published=false`. El ejecutable declara 0.2.0 y admite la extensión histórica
0.3.0; `952cf77` identifica el candidato y no es una nueva versión SemVer.

La creación comprime directamente desde los archivos originales, sin staging ni
copias del repositorio. Se conserva una sola copia comprimida; la instalación
desechable de verificación se retira después de comprobarla. La caché de Cargo
no existía: sólo se compilaron los objetivos de prueba necesarios, offline,
con dos jobs, sin símbolos de depuración ni compilación incremental. No hubo
build release nuevo ni descarga de dependencias.

## Condición para la siguiente etapa

Para publicar hay que fijar la versión de distribución, ejecutar CI para el
sujeto final y decidir su aceptación. El empaquetador de rc.1 conserva etiquetas
de aceptación de esa entrega; este ZIP usa un manifiesto local independiente.
La calificación 12/12 corresponde al binario actual y no se transfiere por
inferencia a uno que se reconstruya después con otra versión.

No se avanza G-Skills Fase 6 por este trabajo. Adopción real, superioridad de
utilidad y CI de una futura release quedan fuera de la preparación completada.
