# Etapa 7 — conformidad local finita del GSP fijado

Estado: etapa 7 verificada, avance 7/8. No es certificación general ni aceptación bilateral externa.

## Conjunto e interfaz

- Runtime sin cambios: `7a43559c02417641691dbb5f2a6e79c13d74d1a1` (v0.2.0).
- Contrato: `gsp-exact-2026.04.12`, commit `004b14d7b64837025671970eac2469f092c49475`. Se conservan locks, schemas y contratos v0.2.
- `integration gsp compile --python <exe absoluto> --source-root <checkout absoluto> --input <corpus> --output <archivo nuevo> --policy <json> --cwd <directorio absoluto> [--cancel-file <ruta absoluta>] --json`.
- La policy contiene `allowed_programs` y opcionalmente `process` (timeout_ms, max_stdout_bytes, max_stderr_bytes, environment). Defaults: 60 s, 1 MiB/stdout, 1 MiB/stderr; entorno vacío. PATH para Git, SystemRoot en Windows y cualquier variable GSP necesaria se autorizan individualmente en la policy. Nunca se heredan automáticamente credenciales.
- Entrada máxima 1 MiB; resultado/publicación máximo 8 MiB, con newline incluido. Policy máxima 64 KiB. Lectura acotada, sin symlinks de artefacto; el adapter debe coincidir con los bytes incorporados al binario. Python se invoca -I -B, sin shell, stdin cerrado, con limpieza de la familia propia.
- Se copia el corpus verificado a un temporal propio. Sólo se publica un `GspBridgeOutput` validado (result, receipt, report) y sin sobrescritura, después de terminar el proceso y verificar sus seis artefactos. stdout/stderr crudos no se persisten. Fallo, timeout o cancelación no publican ni enlazan.
- El éxito devuelve COMPILED_UNLINKED. Para enlazar se requiere otro comando explícito: `integration gsp link-result --receipt <salida compile> ...`, con actor, proyecto, URI, versión esperada, clave y tiempo. No existe enlace automático ni acceso SQLite desde el compilador.
- `export-corpus`, `validate-result` y `link-result` también leen/publican con límites y escaneo de privacidad. Los errores de artefactos no reproducen valores privados. Exportar historia seleccionada con material sensible se rechaza (13), no se re-sella una historia redactada.

Python/Git y el entorno congelado son prerrequisitos opcionales externos; no hay descargas durante uso. La supervisión no es una sandbox para código hostil arbitrario. La autoridad del actor sigue siendo la del comando nativo local; un receipt externo no autentica a una persona ni concede autoridad sobre lifecycle/gates.

## Matriz acotada y origen de prueba

| Caso | Evidencia / origen | Resultado exigido |
|---|---|---|
| Compilación normal | CLI + runtime real, texto seleccionado explícito | 6 artefactos; export/compile delta 0; receipt sin certificación |
| Contradicción de polaridad | Runtime real: must / must not | Conflicto y uncertainty_map visibles; requires_human_review=true |
| Incertidumbre | uncertainty_map producido por el compilador real en el caso contradictorio | Conservada, no sustituida por confianza Ledger; no se afirma detectar cualquier incertidumbre en lenguaje natural |
| Runtime ausente | CLI real, ruta inexistente | 12, sin publicación/descarga |
| Ejecutable no autorizado | CLI real, allowlist vacía | 9, sin ejecución/publicación |
| Versión/checkout incompatible | Python real frente a otro checkout; RQ4 wrong-source | Rechazo del pin; no eventos ni resultado publicado |
| Timeout de startup | CLI supervisada, límite explícito 1 ms | 12 EXECUTION_TIMEOUT; familia terminada; no resultado |
| Cancelación prearmada | CLI supervisada, archivo explícito | 12 EXECUTION_CANCELLED; no resultado |
| Timeout/cancelación dentro del compilador | Supervisor compartido + Python real + probe RQ4 de entrada, sólo test | Marker de compile_evidence previo al fallo; proceso terminado; no resultado/eventos |
| Crash dentro del compilador | RQ4 repetido: proceso propio terminado tras marker | Cadena/proyección intactas; reinicio fresco compila; link 1 / replay 0 |
| Resultado truncado / campo eliminado | Alteraciones adversariales de resultados reales | Validación rechazada sin publicar/enlazar |
| request_id incorrecto | Alteración adversarial de resultado real | Rechazo cross-artifact; delta 0 |
| Digest/estado falsificado | Fixture exacto y receipt re-sellado adversarial, tests previos | Rechazo del binding con el estado vigente |
| Estado desactualizado / scopes de actor | CLI + SQLite | Conflicto 5, no UNIQUE/11; IDs de nuevos comandos incluyen actor; replay histórico conserva ID |
| Replay del enlace | CLI con resultado real, y fixtures | Un solo efecto; mismo actor/tipo/trust/clave; ninguna elevación de autoridad |
| Privacidad y salida excesiva | Canary ficticio / archivo >8 MiB, CLI | 13 sin eco o 15 por presupuesto; no publicación |
| Autoridad insuficiente del enlace nativo | CommandProcessor real, matriz de 53 comandos | LINK_EXTERNAL_EVIDENCE con actor UNTRUSTED devuelve POLICY_DENIED y delta 0 |

Pruebas: `crates/cli/tests/gsp_live/supervised.rs`, `gsp_live/mod.rs`, `gsp_bridge.rs`, `crates/interop-gsp/tests/{conformance,bridge,corpus}.rs` y `application/tests/command_coverage.rs`. Los fixtures de contrato siguen etiquetados como fixtures. La instrumentación de entrada sólo vive en `crash_probe.py`; no existe una opción de producción que la habilite ni se modifica el runtime upstream.

## Evidencia disponible y límites

Sesión 15576: cinco pruebas CLI nuevas PASS; dos pruebas de runtime supervisado PASS (normal/contradicción, alteraciones, link/replay, drift, timeout/cancelación); tres pruebas RQ4 reales PASS. El crash observó compile_evidence en PID propio 6340; no publicó resultado ni eventos y recuperó link/replay. Sesión 68925: Clippy y las tres pruebas supervisadas reales PASS, incluido timeout/cancelación tras entrada. Gate 62580 exit 0: formato, Clippy, workspace completo, build locked, schemas/specs/contratos, release check, dependencias, 13 SHA-256 y diff. Los seis tests GSP opt-in fueron ejecutados explícitamente; ENOSPC físico y nueva CI/ZIP quedan para etapa 8.

Revisados locks/reportes locales y evidencia alojada RQ4 histórica, incluida la CI final `33062468586` / `dea208d294dfddc282b84dcb5a58b9d8d9887b91`. Listados GitHub de issues de Ledger y del compiler (todos los estados, hasta 30) devolvieron [] el 2026-08-27. Esto no demuestra aceptación por terceros: no se recibió ni se representa una certificación bilateral. G-Skills y Radar no se habilitan para el piloto.

La matriz sólo cubre este conjunto y estas condiciones reproducibles. No certifica otros runtimes/versiones, precisión semántica universal, modelos, servicios de red, GSP más reciente ni ausencia absoluta de errores. La preparación del ZIP/CI nueva sigue siendo etapa 8.
