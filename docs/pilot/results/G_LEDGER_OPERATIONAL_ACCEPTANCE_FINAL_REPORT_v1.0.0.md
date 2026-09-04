# Informe Final para Decisión Humana

## Piloto de Aceptación Operativa de G-Ledger 0.2.0

| Control | Valor |
|---|---|
| Fecha de corte | 4 de septiembre de 2026 |
| Producto evaluado | G-Ledger 0.2.0 |
| Estado formal | `OPERATIONALLY_ACCEPTED` |
| Avance formal | 8/8 etapas, 100 % |
| Decisión humana vigente | Aprobada por `human:owner` |
| Uso evaluado | Adopción operativa controlada; no General Availability |

---

## 1. Dictamen ejecutivo

G-Ledger demostró que puede cumplir su propósito principal: actuar como una bitácora automatizada de alto nivel que conserva continuidad, evidencia, decisiones, riesgos, autoridad y estado operativo sin permitir que archivos, agentes o integraciones externas se atribuyan facultades que no poseen.

La recalificación técnica concluyó satisfactoriamente para el binario exacto evaluado. Las siete etapas técnicas están acreditadas, los treinta escenarios ejecutables previos a la decisión final pasaron y los cuatro defectos de producto encontrados durante el piloto fueron corregidos y recalificados. No quedan defectos abiertos, riesgos abiertos, conflictos, incógnitas críticas, dispensas críticas ni hallazgos de integridad.

Tras revisar este informe, `human:owner` emitió la aprobación. La decisión se registró como `OPERATIONALLY_ACCEPTED`, quedó ligada al ZIP exacto evaluado y habilitó la transacción gobernada final. Gate Engine verificó los ocho gates requeridos y Phase Engine completó la fase y el proyecto.

**Dictamen final:** G-Ledger queda aceptado para uso operativo controlado, sujeto a los límites y exclusiones indicados en este informe. Esta aceptación no autoriza General Availability, merge ni release.

## 2. Qué se evaluó exactamente

La evaluación se realizó sobre un sujeto inmutable y verificable:

| Elemento | Identidad acreditada |
|---|---|
| Versión | `0.2.0` |
| Commit fuente | `138eabe715d209da5f527340a34492bff3f6fd1a` |
| ZIP alojado SHA-256 | `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162` |
| Ejecución CI | `33838359314` |
| Artefacto alojado | `9924403634` |
| Proyecto descartable | `project:01a06adc-c603-7291-aabe-240d71809978` |
| Plataforma primaria | Windows / PowerShell |
| Compatibilidad certificada | `false` |

El binario se ejecutó desde una carpeta descartable separada del código fuente. El proyecto de prueba, sus bases SQLite y la salida cruda permanecieron bajo `.tooling/` y no fueron incorporados al repositorio.

## 3. Respuesta a la pregunta central

### ¿G-Ledger hace lo que fue creado para hacer?

**Sí, dentro del alcance probado.** La evidencia demuestra que:

- registra progreso material y conserva el historial sin reescritura;
- distingue afirmaciones, observaciones y evidencia verificada;
- reconstruye una continuidad compacta mediante `resume` y checkpoints;
- mantiene decisiones, blockers, riesgos, unknowns y próximos pasos visibles;
- impide que un agente adquiera autoridad humana por conveniencia;
- impide que archivos existentes o comandos exitosos completen por sí solos una fase;
- reserva la verificación de gates y el cierre de fases para sus motores autorizados;
- trata Markdown como una proyección regenerable, no como fuente de verdad;
- preserva idempotencia, concurrencia optimista y reconciliación de resultados inciertos;
- rechaza corrupción, divergencia y sobrescritura no autorizada;
- permite handoff a un agente nuevo sin entregarle la historia completa;
- mantiene las integraciones locales acotadas, sin elevación de autoridad ni mutaciones implícitas.

## 4. Resultado por etapa

| Etapa | Objetivo | Resultado | Conclusión principal |
|---:|---|---|---|
| 1 | Identidad y preflight | `PASS` | El ZIP alojado, sus recursos, versión y commit fuente fueron verificados exactamente. |
| 2 | Modelo gobernado | `PASS` | Proyecto, autoridad, repositorio, roadmap, fases, gates y consultas respetaron sus fronteras. |
| 3 | Bitácora y continuidad | `PASS` | El historial se preservó y `resume` recuperó el estado esencial desde un checkpoint vigente. |
| 4 | Evidencia y lifecycle | `PASS` | Una observación no verificó gates; sólo evidencia admisible y los motores autorizados cambiaron estados. |
| 5 | Seguridad transaccional | `PASS` | Replay, conflictos, versión obsoleta, autoridad insuficiente y resultado incierto fallaron o reconciliaron correctamente. |
| 6 | Proyecciones y recuperación | `PASS` | Markdown, portabilidad, backup/restore, corrupción y privacidad se comportaron de forma fail-closed. |
| 7 | Handoff e integraciones | `PASS` | Un agente ciego reconstruyó continuidad; provider, RecordSink y GSP mantuvieron límites y deltas esperados. |
| 8 | Conciliación y decisión | `PASS` | La decisión humana fue aceptada, los gates se verificaron y la fase se cerró mediante los motores autorizados. |

### Escenarios

- OPA-S01 a OPA-S30: `PASS`.
- OPA-S31, Project Radar: `EXCLUDED_BY_SCOPE`; no se presenta como aprobado.
- OPA-S32, decisión de adopción: `PASS` con evidencia `LIVE_PILOT_HUMAN_DECISION`.

## 5. Estado de integridad y gobierno

La conciliación final fue completamente de lectura y conservó el estado exacto del Ledger:

| Control | Resultado |
|---|---|
| Versión antes/después de la decisión | 97 → 135 |
| Digest de estado final | `sha256:3ecb9ec015c64b0256a18959a35c11d050e0067be865548eb9b55e56da457022` |
| Digest del último evento | `sha256:a0f76396b258546f2d91ccd6373ed6a5ec5136ffec48f7e086af2a68c2b1431f` |
| Checkpoint | `checkpoint:606c14cc-8f7c-709e-9ced-ff6bc0152434` |
| Frescura del checkpoint | `CURRENT` |
| Estado del proyecto | `COMPLETED` |
| Fase | `phase:operational-acceptance`, `COMPLETED` |
| Fases completadas | 1 |
| Riesgos abiertos | 0 |
| Unknowns críticos | 0 |
| Conflictos | 0 |
| Submissions sin resolver | 0 |
| `doctor` / `verify` / `audit` | `ok`, sin findings |

No quedan blockers abiertos. La siguiente acción registrada es `action:opa-first-controlled-project`.

## 6. Gates y cierre 8/8

Los ocho gates requeridos están `VERIFIED`; ninguno fue dispensado. El único waiver pertenece a un gate opcional creado expresamente para probar esa funcionalidad.

La secuencia de cierre respetó la frontera de autoridad:

- `human:owner` propuso y aceptó la decisión;
- RecordSink registró evidencia verificada con replay idempotente;
- el agente solicitó, pero no decidió, la evaluación de gates;
- Gate Engine verificó los gates;
- el agente preparó la fase para cierre;
- `human:owner` confirmó y Phase Engine completó la fase.

## 7. Defectos encontrados y resolución

El piloto encontró cuatro defectos relevantes. Encontrarlos no invalida el valor del piloto; demuestra que el proceso fue capaz de detectar desviaciones reales, invalidar candidatos anteriores y exigir recalificación.

| Defecto | Hallazgo | Tratamiento | Estado |
|---|---|---|---|
| OPA-DEFECT-001 | La normalización temporal podía producir un digest distinto al esperado. | Corrección del cálculo y nueva calificación del candidato. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-002 | La CI de pull request empaquetaba el merge sintético en vez del commit autorizado. | Checkout exacto y comprobación explícita de identidad/limpieza. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-003 | Un checkpoint nuevo podía quedar stale por conservar historia de autoridad supersedida. | Corrección de la evaluación de frescura y repetición desde Stage 3. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-004 | El contrato soportaba cerrar riesgos, pero el CLI público no exponía `risk close`. | Se añadió la superficie CLI, se probó cierre/replay y se repitieron Stage 1–8. | `FIXED_AND_REQUALIFIED` |

Durante Stage 8 se detectaron dos incidencias del **harness**, no del producto. PowerShell strict mode no toleraba que el reporte omitiera la propiedad opcional `owner_decision`; se corrigió en `0758f7c03a5a7735bd5c3167d6a816601531005b`. Antes de ejecutar la aprobación también se detectó que el fixture conservaba el hash del candidato histórico; una regresión RED/GREEN obligó a ligarlo al hash exacto suministrado al runner en `5554152bee7e0510c5ef881e11b01ffa1a36c5e2`. Ambas desviaciones se detuvieron antes de mutar el proyecto vigente.

## 8. Evidencia de calidad

La conclusión no se basa únicamente en tests unitarios ni en documentación existente. Combina:

- un artefacto Windows alojado y ligado a un SHA exacto;
- ejecución viva sobre un Ledger descartable nuevo;
- escenarios negativos con códigos de salida y delta cero;
- evidencia verificada mediante RecordSink con efecto durable único;
- reconciliación de resultados inciertos sin reenvío ciego;
- export/import y backup/restore en destinos nuevos;
- un handoff real a una tarea Codex nueva sin relato previo;
- comprobaciones finales de `doctor`, `verify`, `audit` y release;
- hashes independientes para los siete manifiestos de etapa.

La puerta de repositorio informó 90 criterios de aceptación, 32 límites de privacidad, cero secretos, nueve métricas de benchmark y cero waivers explícitos.

## 9. Límites y defectos residuales

### Defectos residuales abiertos

**Ninguno conocido dentro del alcance ejecutado.** Esto no constituye una afirmación de ausencia universal de defectos.

### Límites que deben conservarse

1. G-Skills sigue en desarrollo y no fue validado como producto. Sus superficies locales sólo sirvieron para probar las fronteras de G-Ledger.
2. Project Radar fue excluido y no puede inferirse compatible ni aprobado.
3. La búsqueda de canaries cubrió las superficies soportadas; no demuestra ausencia universal en todos los bytes internos.
4. Un destino no normativo con sidecars SQLite de 261 caracteres falló por compatibilidad de rutas Windows. El destino normativo con espacios y Unicode pasó con 222 caracteres.
5. `certified_compatibility=false` permanece obligatorio.
6. Una aceptación operativa no equivaldrá a General Availability, release, certificación universal ni autorización para cualquier integración futura.
7. El PR privado continúa draft; este piloto no autoriza merge ni release.

## 10. Decisión del propietario

| Control | Resultado |
|---|---|
| Actor | `human:owner` |
| Decisión | `OPERATIONALLY_ACCEPTED` |
| Estado del registro | `ACCEPTED` |
| Autoridad | `authority:opa-spec` |
| Fecha de aceptación | `2026-09-04T13:42:09Z` |
| Alcance | Uso operativo controlado de G-Ledger 0.2.0 |
| ZIP ligado a la decisión | `7160d31d831b0e85d792f77f04f8f56290256e73d695746545fcfa67b13f0162` |

La decisión histórica emitida para un candidato anterior fue preservada como historia, pero quedó invalidada por OPA-DEFECT-004 y no se transfirió a esta recalificación.

La aprobación no se transfiere a builds distintos ni amplía por inferencia el alcance hacia G-Skills, Project Radar, GA, merge o release.

## 11. Documentos de respaldo

- [Especificación normativa](../G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md)
- [Matriz de cobertura](../G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md)
- [Reporte machine-readable](operational-pilot-report.json)
- [Evidencia sanitizada de Stage 8](stage-8-risk-close-owner-decision-readiness.json)
- [Acta sanitizada de aceptación](stage-8-risk-close-operational-acceptance.json)
- [Plan ejecutable](../../superpowers/plans/2026-09-02-g-ledger-operational-acceptance-pilot.md)
- [Bitácora de implementación](../../../BITACORA_IMPLEMENTACION.md)

---

## Conclusión

La evidencia técnica y la decisión de `human:owner` establecen que G-Ledger 0.2.0, en el sujeto exacto identificado, está **`OPERATIONALLY_ACCEPTED` para uso operativo controlado**. El sistema demostró su filosofía de bitácora automatizada de alto nivel y mantuvo una separación efectiva entre evidencia, autoridad, proyecciones, integraciones y lifecycle.

El cierre alcanza 8/8 y 100 %. Se conservan `certified_compatibility=false`, las exclusiones y los límites residuales; no hubo merge, release ni declaración de General Availability.
