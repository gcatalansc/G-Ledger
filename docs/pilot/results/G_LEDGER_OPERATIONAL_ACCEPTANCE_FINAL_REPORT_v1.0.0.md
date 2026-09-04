# Informe Final para Decisión Humana

## Piloto de Aceptación Operativa de G-Ledger 0.2.0

**Fecha de corte:** 4 de septiembre de 2026  
**Producto evaluado:** G-Ledger 0.2.0  
**Estado formal:** `READY_FOR_OWNER_DECISION`  
**Avance formal:** 7/8 etapas, 87,5 %  
**Decisión humana vigente:** pendiente  
**Uso evaluado:** adopción operativa controlada; no General Availability

---

## 1. Dictamen ejecutivo

G-Ledger demostró que puede cumplir su propósito principal: actuar como una bitácora automatizada de alto nivel que conserva continuidad, evidencia, decisiones, riesgos, autoridad y estado operativo sin permitir que archivos, agentes o integraciones externas se atribuyan facultades que no poseen.

La recalificación técnica concluyó satisfactoriamente para el binario exacto evaluado. Las siete etapas técnicas están acreditadas, los treinta escenarios ejecutables previos a la decisión final pasaron y los cuatro defectos de producto encontrados durante el piloto fueron corregidos y recalificados. No quedan defectos abiertos, riesgos abiertos, conflictos, incógnitas críticas, dispensas críticas ni hallazgos de integridad.

El resultado todavía **no es `OPERATIONALLY_ACCEPTED`**. La etapa 8 comprende también la decisión soberana del propietario y la transacción gobernada que, si se autoriza, evalúa los gates finales y cierra la fase. Como esa decisión no ha sido emitida, el estado correcto es `READY_FOR_OWNER_DECISION`, con 7/8 etapas y 87,5 %.

**Recomendación técnica:** aprobar G-Ledger para uso operativo controlado, sujeto a los límites y exclusiones indicados en este informe. Esta recomendación no sustituye la decisión de `human:owner`.

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
| 8 | Conciliación y decisión | `AWAITING_OWNER_DECISION` | La conciliación técnica pasó sin mutaciones; falta la decisión humana separada. |

### Escenarios

- OPA-S01 a OPA-S30: `PASS`.
- OPA-S31, Project Radar: `EXCLUDED_BY_SCOPE`; no se presenta como aprobado.
- OPA-S32, decisión de adopción: `AWAITING_OWNER_DECISION`.

## 5. Estado de integridad y gobierno

La conciliación final fue completamente de lectura y conservó el estado exacto del Ledger:

| Control | Resultado |
|---|---|
| Versión antes/después | 97 → 97 |
| Digest de estado | `sha256:044fa21419743f9b81305b734192304c47733467a30cc33b516176f7219eb350` |
| Digest del último evento | `sha256:71b74a1d305dc7aa725c5662ca1b38b8a4d38aaf3fd0c501218860576e500769` |
| Checkpoint | `checkpoint:c85fb171-02ec-76d4-9990-366c3ac6f21d` |
| Frescura del checkpoint | `CURRENT` |
| Estado del proyecto | `BLOCKED` a la espera de decisión humana |
| Fase | `phase:operational-acceptance`, activa |
| Fases completadas | 0 |
| Riesgos abiertos | 0 |
| Unknowns críticos | 0 |
| Conflictos | 0 |
| Submissions sin resolver | 0 |
| `doctor` / `verify` / `audit` | `ok`, sin findings |

El blocker vigente es `blocker:opa-stage8-human-decision`. La siguiente acción registrada es `action:opa-next-stage8-decision`.

## 6. Gates y motivo del 7/8

`gate:opa-04` está verificado. Los otros siete gates requeridos permanecen pendientes y ninguno fue dispensado. El único waiver pertenece a un gate opcional creado expresamente para probar esa funcionalidad.

Este estado no representa siete fallos técnicos. Es una frontera deliberada: la transacción final sólo puede iniciarse después de una decisión humana explícita. Hasta entonces:

- Stage 8 no está completado;
- OPA-S32 no puede pasar;
- la fase no puede cerrarse;
- el proyecto debe permanecer `BLOCKED`;
- el máximo estado permitido es `READY_FOR_OWNER_DECISION`.

## 7. Defectos encontrados y resolución

El piloto encontró cuatro defectos relevantes. Encontrarlos no invalida el valor del piloto; demuestra que el proceso fue capaz de detectar desviaciones reales, invalidar candidatos anteriores y exigir recalificación.

| Defecto | Hallazgo | Tratamiento | Estado |
|---|---|---|---|
| OPA-DEFECT-001 | La normalización temporal podía producir un digest distinto al esperado. | Corrección del cálculo y nueva calificación del candidato. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-002 | La CI de pull request empaquetaba el merge sintético en vez del commit autorizado. | Checkout exacto y comprobación explícita de identidad/limpieza. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-003 | Un checkpoint nuevo podía quedar stale por conservar historia de autoridad supersedida. | Corrección de la evaluación de frescura y repetición desde Stage 3. | `FIXED_AND_REQUALIFIED` |
| OPA-DEFECT-004 | El contrato soportaba cerrar riesgos, pero el CLI público no exponía `risk close`. | Se añadió la superficie CLI, se probó cierre/replay y se repitieron Stage 1–8. | `FIXED_AND_REQUALIFIED` |

Durante Stage 8 se detectó además una incidencia del **harness**, no del producto: PowerShell strict mode no toleraba que el reporte omitiera la propiedad opcional `owner_decision`. Se corrigió con una prueba de regresión en el commit `0758f7c03a5a7735bd5c3167d6a816601531005b`. El fallo ocurrió antes de leer o mutar el Ledger.

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

## 10. Decisión solicitada al propietario

Con la evidencia disponible, el propietario puede adoptar una de estas decisiones:

| Decisión | Significado |
|---|---|
| `OPERATIONALLY_ACCEPTED` | Autoriza el uso operativo controlado de este sujeto exacto y la transacción final prevista por Stage 8. |
| `BLOCKED` | Mantiene el piloto abierto e identifica una condición adicional que debe satisfacerse. |
| `REJECTED` | Determina que el sujeto no debe darse de alta para uso operativo. |

La decisión histórica emitida para un candidato anterior fue preservada como historia, pero quedó invalidada por OPA-DEFECT-004 y no se transfirió a esta recalificación.

### Registro de decisión

**Actor autorizado:** `human:owner`  
**Decisión:** pendiente  
**Fecha:** pendiente  
**Observaciones del propietario:** pendiente

## 11. Documentos de respaldo

- [Especificación normativa](../G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md)
- [Matriz de cobertura](../G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md)
- [Reporte machine-readable](operational-pilot-report.json)
- [Evidencia sanitizada de Stage 8](stage-8-risk-close-owner-decision-readiness.json)
- [Plan ejecutable](../../superpowers/plans/2026-09-02-g-ledger-operational-acceptance-pilot.md)
- [Bitácora de implementación](../../../BITACORA_IMPLEMENTACION.md)

---

## Conclusión

La evidencia técnica respalda que G-Ledger 0.2.0, en el sujeto exacto identificado, está preparado para una decisión favorable de adopción operativa controlada. El sistema demostró su filosofía de bitácora automatizada de alto nivel y mantuvo una separación efectiva entre evidencia, autoridad, proyecciones, integraciones y lifecycle.

El estado seguirá siendo `READY_FOR_OWNER_DECISION`, 7/8 y 87,5 % hasta que `human:owner` emita una decisión nueva y separada. Este informe no prellena ni ejecuta esa decisión.
