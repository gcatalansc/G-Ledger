# Investigación de utilidad de G-Ledger

Fecha: 2026-09-07. **Investigación en ejecución; utilidad comparativa y adopción aún no demostradas.** Autorización del propietario: «Procede con el plan profesionalmente».

La oportunidad mejor sustentada hasta ahora es **ayudar a retomar y entregar trabajo con el estado, la evidencia y los límites correctos**. Los registros existentes contienen buena parte de esa información. Falta demostrar que G-Ledger permite aprovecharla con menos esfuerzo y errores que una documentación bien mantenida.

## Resultado y decisiones

- Descubrimiento documental: **10 episodios, cuatro proyectos, siete documentos fijados por commit y SHA-256**. [Catálogo](cases.v0.1.json) y [procedencia con rangos](case-source-manifest.v0.1.json).
- Revisión dirigida: **12 fuentes primarias** de metodología y alternativas. [Fuentes, límites y comparación](SOURCES_AND_ALTERNATIVES_v0.1.md).
- Calibración C01: binario local fijado, un Ledger sintético, tres estados y **30 lecturas de continuidad dentro de 63 llamadas CLI**. [Resultados](results/calibration-C01.json).
- Un candidato de presentación P1, aislado del producto, hace visibles conteos y texto recortado. [Ejemplo legible](results/continuity-P1.md), [salida actual](results/resume-codex.md), [comparación de propiedades](results/comparison-P1.json).
- E01 ejecutó **doce sesiones nuevas de subagentes, seis pares A/B**. No demostró ventaja neta frente a documentación mantenida. Once entregas cumplen la rúbrica bajo revisión del investigador; una ambigüedad sigue pendiente de adjudicación humana. [Informe, resultados y decisión](e01/REPORT.md).

E01 tuvo exposición automática a memoria previa en los doce participantes, sin identificador exacto de modelo ni telemetría de tokens. Las duraciones de intento fueron mayores en B, pero no permiten estimar productividad causal. No participaron humanos ni se midieron mantenimiento longitudinal o ahorro neto. La siguiente pregunta es cuánto cuesta conservar el estado correcto durante cambios y relevos reales.

**Situaciones seleccionadas:** U1, retomar después de cambiar de sesión o responsable; U2, cerrar una sesión dejando un avance verificable y útil para el siguiente responsable. Se conserva la prioridad inicial de continuidad, aún sin una preferencia posterior del propietario entre públicos. Humanos y LLM se analizarán por separado.

## Lo que enseñan los casos

| Casos | Observación documentada | Consecuencia para investigar |
|---|---|---|
| R01–R03, G-Ledger | El piloto detectó problemas de identidad temporal, frescura y una operación CLI ausente; fueron corregidos | Importa que una conclusión corresponda al sujeto correcto y que el flujo sea ejecutable |
| R04, G-Ledger | La corrección local posterior tiene un alcance distinto de la release | La continuidad debe separar código, artefacto distribuido y calificación |
| R05, G-Ledger | Fue necesario crear un informe humano sobre el reporte de máquina | La presentación debe servir para comprender y decidir |
| R06, G-Flow | Una capacidad faltante bloquea F10; la bitácora preserva correctamente bloqueo y siguiente condición | Integrar exige evidencia ejecutable, no sólo un nombre en la especificación |
| R07–R08, G-Writer | DB de prueba y DB real difieren; un PASS posterior convive con una causa no reproducida | Mantener entorno, alcance e incertidumbre junto al resultado |
| R09–R10, G-ASRL | La aprobación cambia el estado; aprobar, implementar y probar siguen siendo hechos distintos | Consultar historia por momento y alcance, sin convertir registros antiguos en estado actual |

Es una muestra intencional del ecosistema de un propietario: los episodios se agrupan en siete clusters y no son independientes. No se midieron frecuencia poblacional, tiempo perdido ni daño producido por confundirlos. Los defectos de ingeniería del piloto no se reclasifican como errores de continuidad de usuarios. En todos los casos seleccionados, la documentación ya aporta una parte útil de la respuesta: esto sostiene una alternativa fuerte y limita cualquier afirmación de necesidad.

Las fuentes son snapshots, no un monitor: G-Writer se fijó en `a799d678` y G-ASRL en `1076bc0c`, donde ya hay avances posteriores a la interiorización inicial. Se usa `git show <commit>:<path>`; los proyectos externos sólo se leyeron. El manifiesto permite reconstruir qué se observó sin publicar sus registros brutos.

## Hallazgos medidos en el G-Ledger actual

C01 usa `53dc909`, binario SHA-256 `3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438`. Se verificaron recursos; `verify` y `doctor` terminaron con integridad `ok`. Las lecturas conservaron versión y digest. El fixture deja deliberadamente un checkpoint invalidado y ocho incógnitas abiertas: sus nueve findings son esperados, no un gate de proyecto aprobado. [Verificación completa](results/integrity-C01.json).

| Incógnitas materiales registradas | Referencias en JSON | Referencias en vista Codex | Bytes Codex / JSON |
|---:|---:|---:|---:|
| 0 | 0 | 0 | 364 / 807 |
| 1 | 1 | 1 | 372 / 868 |
| 8 | 8 | 4 | 442 / 1394 |

La vista Codex ya advierte que es acotada y remite a JSON; no oculta completamente la existencia de una vista más amplia. El problema de presentación a explorar es que no informa cuántas referencias quedaron fuera. El JSON conserva todas las referencias, pero el texto del último avance también se normaliza a 96 caracteres. En el fixture, la condición final de autorización quedó fuera de ambos resúmenes. Esto **no borra el evento**, ni demuestra por sí solo incumplimiento del contrato: obliga a distinguir referencias completas de contenido completo. Mecanismo inspeccionado en `crates/continuity/src/resume.rs`, commit fijado.

El agente pudo registrar progreso, incógnitas y checkpoint. `next set` —incluso con clasificación `PROPOSED`— y `blocker open` devolvieron `POLICY_DENIED`, con cero eventos. Es coherente con `recording_authorized` y el CLI actual; no se eludió con `--as-human`. U2 debe evaluar cómo registrar propuestas y coordinar la decisión humana sin duplicar trabajo. No hay evidencia suficiente para recomendar relajar esa política.

Las medianas de lectura observadas estuvieron entre 36,843 y 44,268 ms, con cinco lecturas por formato y estado. Incluyen arranque y cachés en una sola máquina. No miden comprensión, tokens, ahorro humano o rendimiento general. Los 63 comandos incluyen instrumentación y comprobaciones; no se imputan como pasos obligatorios del usuario.

Tres intentos de preparación se conservaron antes de C01: expectativa incorrecta de permiso para `next set` (`215658Z`), enum de fixture inválido `CRITICAL` en vez de `MATERIAL` (`215905Z`) y aserción del harness de un único evento que omitía la invalidación derivada (`215938Z`). Se corrigió el instrumento, no el producto. Los argv y salidas permanecen en `.tooling/research/utility-v01/` bajo cada run completo.

Control de bytes: Git normalizó inicialmente los saltos CRLF del reporte capturado C01. Se añadió un atributo limitado a ese archivo para conservar los bytes ligados por `comparison-P1.json`; el control final compara los hashes con los blobs Git. Los valores y mediciones no cambiaron.

## P1: qué cambia y qué falta

P1 deriva una ficha del JSON capturado: cuenta las ocho incógnitas, enlaza sus referencias completas, señala texto recortado y evita interpretar una próxima acción ausente como autorización. Conserva proyecto, versión, digest y el alcance de frescura. Funciona fuera del motor y no escribe en el Ledger.

**Resultado de propiedades:** seis pruebas pasan; la entrada queda intacta. La ficha ocupa **987 bytes**, frente a 442 de Codex y 1394 del JSON fuente. Si hace falta abrir el detalle, hay que sumar esa lectura: no se afirma ahorro de contexto. P1 no recupera por sí solo el contenido recortado y todavía no resuelve acceso al significado de las referencias.

Se adelantó este explorador respecto de la comparación E01 para probar una propiedad observada en C01. Es una variación acotada del orden del plan, no una promoción ni evidencia de utilidad del prototipo. No se creó P2: la carga de captura aún no está medida y construir otra capa ahora tendría justificación insuficiente.

## Estado de las etapas

| Etapa | Estado verificable |
|---|---|
| 1. Descubrimiento | Revisión documental realizada; entrevistas, costes y diversidad de propietarios pendientes |
| 2. Alternativas | Revisión dirigida realizada; comparador A elegido; alternativas especializadas no ejecutadas |
| 3. Medición | C01 y doce intentos E01 realizados; una adjudicación pendiente; no ensayo confirmatorio |
| 4. Mejoras | P1 preparado y comprobado en propiedades; comparación y promoción pendientes |
| 5. Adopción | No iniciada; exige sesiones reales, relevo y proyecto nuevo |

**Siguiente paso sustentado:** diseñar la observación del coste total de mantener contexto durante varias actualizaciones y un relevo, con casos nuevos y configuración inspeccionable. E01 no proporciona evidencia suficiente para promover P1, construir P2 o iniciar adopción. El [dictamen](e01/REPORT.md) especifica las condiciones aún necesarias. Se preservan binario, contratos, release y bases reales; sólo se agregan investigación y su estado documental.

## Reproducción

La calibración mecánica C01 y P1 requieren Python 3.11+, Git y el binario exacto de la remediación con sus recursos verificados. Esas herramientas no requieren dependencias Python externas, red ni servicios LLM. E01 sí utilizó participantes LLM de esta aplicación. Ejecutar las herramientas mecánicas desde la raíz de G-Ledger:

```powershell
python docs/research/tools/snapshot_cases.py
python docs/research/tools/calibrate_current.py --binary '<ruta al project-ledger.exe calificado>'
python -m unittest discover -s docs/research/tools -p test_continuity_brief.py
python docs/research/tools/continuity_brief.py docs/research/results/resume-input.json
```

Si `python` no está en PATH, usar la ruta del intérprete disponible. El recolector `publish_calibration.py <run_id>` escribe sólo resultados sintéticos en este repositorio y rechaza reemplazos con bytes diferentes. Para una nueva revisión, conservar el directorio de evidencia anterior y elegir una salida versionada. No hay publicación de red.
