# Fuentes y alternativas — investigación de utilidad v0.1

Consulta: 2026-09-07. Revisión dirigida a decisiones del estudio; no revisión sistemática exhaustiva. Se inspeccionaron páginas primarias, abstracts y documentación de autores. Las capacidades comerciales documentadas no se consideran resultados independientes ni pruebas locales de compatibilidad.

## Fuentes y consecuencias para el estudio

| ID | Fuente primaria y alcance leído | Qué aporta | Qué no permite concluir |
|---|---|---|---|
| S01 | Forsgren et al., SPACE, 2021; [publicación](https://www.microsoft.com/en-us/research/publication/the-space-of-developer-productivity-theres-more-to-it-than-you-think/) | La productividad requiere varias dimensiones; medir calidad, esfuerzo y experiencia | Que más eventos o commits signifiquen mayor utilidad |
| S02 | METR, 2025-07-10; [estudio](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/) | Separar productividad percibida de resultados observados | Trasladar el efecto de aquella muestra a G-Ledger o a modelos actuales |
| S03 | METR, 2026-02-24; [actualización](https://metr.org/blog/2026-02-24-uplift-update/) | Registrar selección, abandonos y tiempo humano activo separado de espera | Usar estimaciones afectadas por selección como expectativa de mejora |
| S04 | Wu et al., LongMemEval, ICLR 2025; [abstract y versión v2](https://arxiv.org/abs/2410.10813) | Evaluar actualización de conocimiento, tiempo y abstención, además de recuperar información | Equiparar memoria conversacional con productividad de desarrollo |
| S05 | LongMemEval; [README de autores](https://github.com/xiaowu0162/LongMemEval) | Diferencia QA y retrieval; este último excluye 30 preguntas de abstención | Comparar sus porcentajes sin declarar denominadores, splits y evaluación |
| S06 | Maharana et al., LoCoMo, ACL 2024; [abstract](https://aclanthology.org/2024.acl-long.747/) | Incluye relaciones temporales y causales en diálogos largos | Sustituir proyectos reales por éxito en conversaciones sintéticas |
| S07 | Nygard, ADR, 2011-11-15; [artículo original](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) | Documentos breves con contexto, estado, consecuencias y supersesión conservada | Atribuir a G-Ledger exclusividad sobre conservar decisiones y razones |
| S08 | [AGENTS.md](https://agents.md/), página del proyecto | Convenciones e instrucciones en un archivo que acompaña al repositorio | Tratar instrucciones como prueba de ejecución o estado actualizado automáticamente |
| S09 | Beads; [README canónico](https://github.com/gastownhall/beads) | Memoria de trabajo estructurada, dependencias, trabajo disponible, JSON y sincronización con Dolt según documentación | Ventaja de productividad, ausencia universal de conflictos o equivalencia de sus controles con G-Ledger |
| S10 | Mem0; [Add Memories v3](https://docs.mem0.ai/api-reference/memory/add-memories) | Extracción aditiva asíncrona y seguimiento por event_id según documentación actual | Suponer que toda memoria especializada sobrescribe o elimina historia |
| S11 | Mem0; [Update Memory](https://docs.mem0.ai/core-concepts/memory-operations/update) | Operación explícita de actualización y metadatos | Mezclar la semántica de update con el pipeline add v3 |
| S12 | Letta Agent SDK; [Memory](https://docs.letta.com/agent-sdk/memory) | Memoria persistente con archivos en Git, contexto fijo y lectura bajo demanda según documentación | Equiparar memoria persistente con aprobación humana o evidencia bilateral |

Control de vigencia: Beads redirigió desde `steveyegge/beads` al repositorio canónico indicado. La URL histórica de memoria de Letta redirigió a documentación **v1 legacy**; se consultó después la página vigente de Agent SDK para S12. Mem0 mantiene páginas de varias operaciones y versiones: se conservan separadas S10/S11. No se ejecutaron instaladores ni se fijaron versiones binarias de estas alternativas; una comparación ejecutable futura deberá hacerlo.

## Comparación de mecanismos

Esta matriz sintetiza las fuentes anteriores y la inspección local de G-Ledger. «No evaluado» expresa el alcance de la investigación, no ausencia de una capacidad.

| Alternativa | Necesidad que puede cubrir | Mecanismo relevante | Diferencia que debemos medir |
|---|---|---|---|
| Git + estado actual breve + ADR + seguimiento + memoria habitual del agente | Reanudar y entender razones | Fuentes versionadas, resumen mantenido, enlaces a pruebas; ya funciona en varios casos locales R04/R06/R07/R09/R10 | ¿G-Ledger reduce errores o mantenimiento frente a este flujo bien usado? |
| Beads | Dependencias y continuidad de tareas de agentes | Grafo de trabajo persistente y consultas de tareas disponibles; S09 | Coste de captura y relevo; alcance real de procedencia y autoridad aún no evaluado |
| Mem0 | Recordar y recuperar hechos entre interacciones | Extracción, búsqueda y operaciones de memoria; S10/S11 | Coste de recuperar la versión pertinente y de distinguir hechos, inferencias y decisiones |
| Letta | Continuidad de un agente y contexto bajo demanda | Repositorio de memoria, contexto fijo y archivos consultables; S12 | Beneficio adicional de una bitácora de proyecto compartida con humanos |
| G-Ledger actual | Historia, identidad, checkpoints, referencias y controles de escritura | CLI y stream local verificado; calibración C01 y código fijado en `53dc909` | Comprensión efectiva, acceso al detalle y carga operativa; eficacia comparativa aún no medida |

**Decisión:** usar primero el flujo documental fuerte como condición A. Ya existe en el entorno, admite las mismas fuentes y permite contabilizar su mantenimiento. No se utilizará una conversación sin memoria ni una carpeta deliberadamente desordenada para favorecer al producto. Beads queda como comparador especializado prioritario para una segunda evaluación ejecutable; esta revisión documental no determina un ganador.

**Inferencia de investigación:** la diferenciación a investigar es recuperar una conclusión vigente, con sujeto, evidencia y límites correctos, a un coste sostenible. Persistencia, búsqueda y Markdown por sí solos ya tienen sustitutos. El ahorro y la preferencia de uso siguen siendo preguntas abiertas.

Cierre de esta búsqueda dirigida: hay soporte primario para elegir comparador, dimensiones y tareas; no hay evidencia externa que pruebe la utilidad de G-Ledger. Se reabrirá la búsqueda si aparece una capacidad decisiva no cubierta, una alternativa mejor o una contradicción que cambie el diseño experimental.
