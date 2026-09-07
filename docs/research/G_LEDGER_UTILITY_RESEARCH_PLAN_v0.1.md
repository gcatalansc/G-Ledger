# G-Ledger: plan de investigación de utilidad v0.1

Fecha: 2026-09-07. Estado: autorizado y en ejecución. [Expediente y resultados actuales](README.md).

Objetivo del propietario: hacer que G-Ledger sea realmente necesario y útil como bitácora de alto nivel para desarrolladores y LLM que trabajan en desarrollo de software.

## 1. Pregunta y resultado buscado

**¿En qué situaciones G-Ledger permite comprender, retomar y conducir un proyecto mejor que las alternativas disponibles, con un beneficio que compense mantenerlo?**

Definición operativa propuesta de «potente»: una persona o agente puede identificar qué ocurrió, qué está vigente, por qué se decidió, qué evidencia lo sostiene y qué puede hacer a continuación, con menos reconstrucción, menos errores y una carga de registro aceptable. Su necesidad se demostrará en situaciones concretas y mediante uso voluntario sostenido.

La hipótesis inicial prioriza continuidad entre sesiones y responsables. También se observará al desarrollador o maintainer que revisa y coordina el proyecto. La prioridad entre ambos usos queda abierta a la preferencia del propietario; los resultados se analizarán por separado.

El resultado de la investigación será una decisión sustentada: conservar capacidades, simplificarlas, mejorar flujos, incorporar capacidades demostrablemente útiles o descartar hipótesis sin ventaja. El número de funcionalidades no constituye una medida de éxito.

## 2. Punto de partida y límites

- Base técnica disponible: commit `53dc909fc34d2ca03f9ca73db3a1af3b54520610`, con [remediación local verificada](../maintenance/2026-09-07-TECHNICAL_REMEDIATION.md). La release publicada continúa en `22b3eca`; cada experimento fijará explícitamente el binario usado.
- La [especificación v0.2](../../specs/PROJECT_LEDGER_SPEC_v0.2.0.md) ya plantea continuidad verificable; la [guía de usuario](../operations/USER_GUIDE.md) define el registro de hechos materiales. Son el contrato y la tesis que se evaluarán, no evidencia de utilidad comparativa.
- La aceptación del piloto y los tests de ingeniería acreditan su alcance declarado. La adopción sobre un proyecto real sigue pendiente; no se infiere productividad a partir de esos resultados.
- La entrega original preparó el plan sin iniciar el estudio. El propietario autorizó después su ejecución con «Procede con el plan profesionalmente». El expediente distingue trabajo ejecutado, candidatos exploratorios y decisiones todavía no sustentadas.
- Las propuestas experimentales conservarán identidad, procedencia, privacidad y autoridad explícita. Una revisión de contratos se planteará como cambio versionado; una inferencia de un LLM no se convierte automáticamente en hecho canónico.

## 3. Etapas y decisiones

| Etapa | Trabajo | Entregable y condición de avance |
|---|---|---|
| 1. Descubrir el trabajo real | Revisar episodios de desarrollo e interrupción; reconstruir qué costó recuperar, qué faltó y qué registro se abandonó. Muestra exploratoria propuesta: 8–12 episodios en al menos tres proyectos, con diversidad de madurez. Entrevistas u observación de usuarios cuando haya participantes disponibles. | Mapa de problemas con ejemplos, frecuencia observada, gravedad y coste. Seleccionar dos situaciones prioritarias; distinguir observaciones del propietario de evidencia sobre otros usuarios. |
| 2. Investigar alternativas y mecanismos | Revisar literatura primaria sobre continuidad, memoria de agentes, recuperación temporal, procedencia y experiencia de desarrollo. Comparar Git + documentación + decisiones + seguimiento de trabajo, capacidades nativas del agente y herramientas especializadas identificadas durante la búsqueda. | Matriz de necesidades, mecanismos, evidencia, límites y sustitutos. Elegir una alternativa fuerte y reproducible para la comparación. La búsqueda termina cuando las afirmaciones decisivas tienen soporte o incertidumbre explícita y las consultas adicionales no cambian la decisión. |
| 3. Medir el G-Ledger actual | Diseñar tareas equivalentes de reanudación, consulta y registro. Ejecutar primero una calibración pequeña para verificar instrumentos, coste y variabilidad. | Línea base de utilidad y fricción; protocolo comparativo congelado antes de medir resultados confirmatorios. Si el producto actual ya aporta valor, identificar qué lo explica y dónde falla. |
| 4. Probar mejoras acotadas | Elegir como máximo dos hipótesis respaldadas por las etapas anteriores. Preparar prototipos reversibles y medir cada cambio por separado. | Comparación con la alternativa fuerte y con G-Ledger actual. Avanzar sólo con una mejora práctica, reproducible y compatible con los controles de calidad; aceptar resultados negativos. |
| 5. Comprobar adopción y decidir | Observar el flujo seleccionado durante varias sesiones y al menos un relevo real de responsable. Incluir un proyecto distinto del usado para ajustar el prototipo. | Beneficio neto, carga de mantenimiento, errores, uso voluntario y límites de generalización. Elaborar después un roadmap de implementación priorizado por evidencia. |

El primer entregable será el mapa de problemas y las dos situaciones seleccionadas. La duración y el presupuesto de los experimentos se fijarán al conocer los casos, participantes y coste de la calibración; no se promete una fecha con esos recursos aún indeterminados.

## 4. Hipótesis refutables

| Hipótesis | Señal favorable | Resultado que la debilita |
|---|---|---|
| H1. El estado vigente verificable facilita retomar trabajo | Menor tiempo hasta una reconstrucción correcta y una próxima acción válida | Un resumen manual mantenido obtiene resultados equivalentes con menor coste |
| H2. Decisiones, supersesiones y evidencia evitan errores de continuidad | Menos reaperturas injustificadas, trabajo repetido y afirmaciones sin soporte | Los errores permanecen o sólo se evitan gracias a intervención adicional del operador |
| H3. Registrar hechos materiales es sostenible | El ahorro supera preparación, registro, revisión y corrección de la bitácora | El registro se abandona, se duplica en varias herramientas o consume el beneficio |
| H4. La presentación puede mejorar el uso del conocimiento existente | Una vista breve con detalle y fuentes bajo demanda mejora precisión y coste de contexto | El resumen omite condiciones necesarias o no mejora frente al acceso actual |

H4 es una posibilidad para investigar, no una funcionalidad aprobada. La captura asistida, las alertas de vigencia o el contexto adaptado a una tarea deberán justificar su propio mecanismo y coste si emergen como candidatas.

## 5. Medición y comparación justa

**Casos iniciales:** reanudar tras perder contexto; incorporar otro responsable; explicar una decisión reemplazada; reconciliar documentación desactualizada con evidencia; determinar el estado real de un bloqueo o gate; registrar un avance material con bajo esfuerzo.

**Condiciones:** A, flujo alternativo bien mantenido; B, el mismo flujo más G-Ledger actual; C, B más una mejora concreta, sólo después de seleccionarla. Se usan los mismos hechos disponibles hasta el corte temporal y se imputa el coste de producir y mantener cada representación. La condición A conserva las capacidades habituales del agente, incluida su memoria disponible: no se empobrece artificialmente la alternativa.

| Dimensión | Qué se medirá |
|---|---|
| Continuidad correcta | Tiempo hasta identificar correctamente estado, decisiones relevantes, evidencia y próxima acción; completitud según una rúbrica previa |
| Calidad | Errores materiales, afirmaciones sin soporte, confusión entre vigente e histórico y retrabajo hasta un resultado aceptable |
| Carga humana | Minutos de preparación, registro, consulta, revisión y corrección; esfuerzo percibido registrado por separado |
| Coste del agente | Tokens, llamadas, latencia y coste monetario con modelo y tarifas fechados; tiempo humano activo separado del tiempo de espera |
| Calidad de la bitácora | Proporción de hechos relevantes capturados, ruido, duplicados y actualización oportuna, contrastados con una revisión independiente |
| Adopción | Uso voluntario en sesiones posteriores, continuidad del registro, motivos de abandono y elección informada entre flujos |

Reglas del protocolo:

1. Fijar repositorios, commits, corte temporal, versión de G-Ledger, modelos, configuración, herramientas y presupuesto de contexto. La rúbrica y los resultados posteriores al corte permanecen fuera del alcance del participante evaluado.
2. Usar tareas equivalentes y asignación aleatoria; contrabalancear condiciones humanas para reducir aprendizaje. En LLM, usar sesiones nuevas y repeticiones con configuración registrada.
3. Preparar una referencia verificable antes de evaluar. Admitir varias próximas acciones válidas y abstenciones justificadas. Un LLM evaluador no será el único juez de resultados críticos; revisar discrepancias contra fuentes.
4. Separar calibración, desarrollo y casos reservados. Una muestra pequeña sirve para descubrir problemas y estimar variabilidad, no para afirmar eficacia general.
5. Antes del ensayo confirmatorio, fijar métrica primaria, mejora mínima útil, tolerancia de calidad, muestra, presupuesto y regla de parada. Dimensionar la muestra con la calibración y el efecto que merecería adoptar el producto; no inventar un porcentaje ganador después de ver resultados.
6. Informar denominadores, distribución de resultados, incertidumbre, casos negativos, abandonos y costes de mantenimiento. Analizar por proyecto, tarea y tipo de usuario; no contar repeticiones del mismo caso como proyectos independientes.
7. Para los prototipos, comparar componentes por separado. Si el efecto desaparece al retirar uno, investigar su contribución; si una solución más simple logra lo mismo, considerarla preferente por su coste.

Un incidente crítico de autoridad, privacidad o integridad atribuible al cambio impide promover ese candidato. Un resultado inconcluso se informa como tal. El piloto de adopción complementa el ensayo; su carácter observacional limita las conclusiones causales.

## 6. Evidencia, trazabilidad y salida

Se mantendrá un expediente de investigación dentro de `docs/research/`: este plan, protocolo, matriz de fuentes y afirmaciones, catálogo sanitizado de casos y dictamen. Los datos privados y registros brutos permanecerán fuera de la documentación distribuida. Cada conclusión enlazará caso o fuente, versión evaluada, resultado, incertidumbre y decisión que permite tomar.

Criterio para recomendar una mejora: beneficio práctico frente a la mejor alternativa evaluada, calidad preservada, coste total justificado y señal repetida en casos reservados. La adopción sostenida y la evidencia de otros usuarios serán necesarias para ampliar la afirmación más allá del entorno del propietario.

## 7. Fundamento metodológico inicial

Esta es una lectura inicial para diseñar el estudio, no una revisión exhaustiva ni evidencia de que G-Ledger mejore la productividad:

- **Forsgren et al., SPACE, ACM Queue, febrero de 2021.** Propone medir la productividad de desarrollo mediante varias dimensiones. Motiva combinar resultados, esfuerzo y experiencia, sin reducir éxito a actividad o volumen de registros. [Publicación en Microsoft Research](https://www.microsoft.com/en-us/research/publication/the-space-of-developer-productivity-theres-more-to-it-than-you-think/).
- **METR, estudio del 10 de julio de 2025.** Encontró discrepancias entre productividad percibida y observada en un contexto específico de desarrollo asistido por IA. Motiva medir el resultado y el trabajo completo, manteniendo limitada la generalización. [Estudio original](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/).
- **METR, actualización del 24 de febrero de 2026.** Describe sesgos de selección y dificultades para medir tiempo cuando se utilizan agentes concurrentes. Motiva registrar selección, abandonos y tiempo humano separado de espera. Sus estimaciones no se trasladan como expectativa de efecto de G-Ledger. [Actualización metodológica](https://metr.org/blog/2026-02-24-uplift-update/).

Fuentes consultadas el 2026-09-07. Las hipótesis, etapas y reglas propuestas aquí son decisiones de diseño del estudio, no resultados publicados por esas fuentes.

## 8. Registro de ejecución — 2026-09-07

La revisión documental reunió diez episodios en cuatro proyectos y fijó fuentes por commit/hash; la búsqueda dirigida fundamentó una alternativa fuerte. C01 midió propiedades del CLI en un Ledger sintético, sin participantes independientes. Se preparó P1 para explicitar conteos y recortes, adelantando sólo esta exploración de presentación respecto de E01; no se adelantó su promoción al producto. Comparación con participantes, umbrales confirmatorios y adopción permanecen pendientes. Véanse [resultados y límites](README.md) y [protocolo E01](EXPERIMENT_PROTOCOL_v0.1.md).
