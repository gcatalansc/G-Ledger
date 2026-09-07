# E01 — resultado de la calibración comparativa

Fecha: 2026-09-07. **Doce intentos terminados; utilidad comparativa y adopción no demostradas.**

En estos seis pares, añadir el flujo mínimo de G-Ledger a documentación bien mantenida no demostró un beneficio que compense su coste. Las duraciones observadas fueron mayores en B. Esto orienta la investigación hacia el esfuerzo de mantener y recuperar contexto a lo largo del trabajo; no demuestra que G-Ledger sea innecesario o intrínsecamente más lento.

La conclusión se limita a una calibración pequeña con sesiones de LLM. Las respuestas alcanzaron casi siempre el máximo de la rúbrica: los resúmenes mantenidos ya exponían las distinciones esenciales. El instrumento deja poco margen para detectar mejora de calidad y no mide el trabajo necesario para producir esos resúmenes.

## Qué se ejecutó

El propietario autorizó explícitamente subagentes para E01. Se ejecutaron doce sesiones nuevas, una por intento, en el orden aleatorizado del [registro congelado](registration.json), con semilla `20260907` y un máximo de tres participantes concurrentes. Todos terminaron dentro de los 15 minutos asignados. No participaron humanos ni se evaluó una segunda configuración de modelo verificable.

- **A:** tarea, estado breve mantenido y extractos de fuentes fijadas.
- **B:** los mismos tres archivos, idénticos en bytes, más un índice sintético del G-Ledger actual. Usa progreso y checkpoint; no reconstruye decisiones humanas, fases y gates de los proyectos originales.
- **U1:** reconstruir estado, evidencia, incertidumbre y siguiente acción válida. Los cuatro participantes B usaron vistas capturadas; no ejecutaron comandos CLI.
- **U2:** incorporar un resultado nuevo a un `HANDOFF.md`. B debía registrar además progreso y checkpoint: cada uno de sus dos participantes reportó cuatro comandos CLI y dos mutaciones aceptadas. El relevo posterior a otro responsable no se ejecutó.

Se utilizó el binario de `53dc909`, SHA-256 `3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438`. La preparación de B consumió 38 llamadas CLI fuera del reloj de los participantes. El experimento no evalúa toda la configuración posible del producto ni el prototipo P1.

## Resultados observados

Fuente numérica: [resultados y hashes](results.json). Los segundos representan **duración del intento**, incluyendo razonamiento y herramientas, según las marcas de reloj reportadas por cada participante. No son tiempo humano activo ni tiempo hasta una entrega validada independientemente.

| Tarea | Situación | A, segundos | B, segundos | B menos A | Calidad A / B, sobre 10 |
|---|---|---:|---:|---:|---|
| T1: corrección local y release | U1 | 85 | 99 | +14 | 10 / 10 |
| T2: bloqueo F10 y evidencia ejecutable | U1 | 82 | 88 | +6 | 10 / 10 |
| T3: aprobación y secuencia D1/D2 | U1 | 80 | 93 | +13 | 10 / 10 |
| T4: DB aislada y DB real | U2 | 130 | 202 | +72 | 10 / 10 |
| T5: PASS posterior y causa desconocida | U2 | 111 | 173 | +62 | 10 / 10 |
| T6: incógnitas y resumen recortado | U1, diagnóstico | 95 | 129 | +34 | 8–10 pendiente / 10 |

Las medianas descriptivas de los seis intentos por condición son A **90 s** y B **114 s**. No se estima un porcentaje de productividad ni un efecto causal. Las tareas son heterogéneas; T6 entrega también a A un JSON estático de G-Ledger para conservar hechos equivalentes. Por ello T6 diagnostica interpretación de representaciones, no una comparación pura de uso frente a ausencia de G-Ledger.

La revisión del investigador contra fuentes y rúbrica considera claras once entregas y deja una pendiente. En T4/T5 ambas condiciones produjeron un traspaso que conserva el entorno y la incertidumbre; la escritura adicional de B aporta una señal de coste en este flujo configurado. No sabemos si ese coste se recupera en sesiones posteriores, ni si un flujo que reutilice el registro evitaría duplicación.

## Ambigüedad pendiente y sensibilidad

En T6A aparece: «El checkpoint contiene un resumen truncado del último progreso y referencias a las incógnitas». La evidencia proporcionada es la vista actual `resume`, versión 12, con un checkpoint invalidado; no incluye su payload histórico. T6A cuenta correctamente las ocho incógnitas y no infiere permiso para D3. T6B distingue explícitamente vista actual y contenido histórico no suministrado.

Hay dos interpretaciones pendientes de revisión humana: terminología imprecisa sin consecuencia material, o atribución de evidencia al objeto histórico equivocado. La segunda podría ser crítica según la rúbrica. Se conserva la dimensión de temporalidad/sujeto sin puntuar y un intervalo 8–10; no se registra como error crítico confirmado ni como victoria de B. [Revisión completa](researcher-review.json).

Si se acepta la terminología, ambas condiciones tienen seis respuestas conformes bajo esta revisión. Si se aplica la interpretación estricta, A tiene una entrega cuestionada y B ninguna. Ninguna lectura de ese único caso permite concluir eficacia, con las limitaciones del diseño. No es necesario resolverlo para mantener la decisión de no promover una mejora con E01.

## Límites y desviaciones

1. **Contexto previo:** los doce participantes declararon recibir automáticamente `MEMORY_SUMMARY`, con antecedentes relacionados. Las sesiones no copiaron la conversación del investigador, pero no estuvieron aisladas de memoria. El registro inicial esperaba ausencia de memoria precargada; se preserva ese registro y se corrige mediante las [notas de ejecución](EXECUTION_NOTES.md).
2. **Configuración y medición:** el identificador exacto del modelo y su telemetría no están expuestos. Se heredó la configuración del padre sin overrides. No hay tokens, coste monetario, tiempo humano ni medición independiente de tiempo activo. Orden, concurrencia y variación del servicio pueden afectar duraciones.
3. **Selección y juicio:** una ejecución por condición y tarea, casos seleccionados por el investigador dentro del ecosistema de un propietario, sin casos reservados. El investigador conoce la condición al puntuar. No hubo revisión humana independiente del equilibrio antes de iniciar ni del juicio posterior.
4. **Cobertura parcial del protocolo:** T3 solicitó reconstruir dos momentos dentro de un mismo paquete, no probar recuperación con paquetes temporales separados. T4 utilizó R07; T6 fue U1. U2 produjo entregas, pero no evaluó su interpretación posterior ni mantenimiento. La métrica primaria prevista de tiempo hasta entrega correcta validada queda **no disponible**.
5. **Representación y carga:** B es un índice mínimo añadido a una alternativa que ya contiene el estado correcto. Su coste de preparación no se imputó al reloj y su posible beneficio posterior no se observó. No se puede extrapolar a un despliegue completo, a información desactualizada, a uso longitudinal o a desarrolladores humanos.

## Decisión y siguiente investigación

Se conserva la documentación mantenida como comparador fuerte. **No se promueve P1 ni se construye P2 a partir de E01.** Tampoco se inicia la etapa de adopción: el plan exige primero evidencia favorable, aún ausente. Esto es una decisión sobre suficiencia de evidencia, no un abandono del objetivo.

La pregunta siguiente es: **¿G-Ledger reduce el coste total de conservar un estado correcto cuando cambian decisiones, evidencia y responsables?** La prueba actual regaló a ambas condiciones un buen resumen; la próxima debe observar cuánto cuesta mantenerlo.

Antes de otro ensayo, preparar un protocolo de mantenimiento y relevo con casos nuevos: varias actualizaciones materiales por proyecto, al menos un cambio de responsable y fuentes independientes que permitan comprobar qué quedó vigente. Comparar flujos completos que puedan reutilizar sus registros para producir la entrega; imputar preparación, captura, consulta, reconciliación, revisión y corrección a ambos. Registrar también duplicados, omisiones y elección voluntaria, sin exigir continuidad de uso para declarar éxito. La muestra y el umbral de beneficio se fijarán antes de resultados nuevos; E01 no proporciona una estimación suficiente para dimensionar un ensayo confirmatorio.

Para ejecutarlo faltan episodios y participantes adecuados, control inspeccionable de memoria/modelo y telemetría del trabajo completo. Una observación humana ayudaría a separar carga del operador y espera del agente. La capacidad nativa `ledger.project_summary` señalada en R06 sigue siendo una oportunidad de integración documentada, pero su implementación o disponibilidad no demuestra por sí sola utilidad: requiere su propia decisión y validación bilateral.

**Estado del programa:** descubrimiento documental y calibraciones C01/E01 realizados en sus alcances; adjudicación de T6A, validación comparativa y adopción pendientes. No hay roadmap de nuevas funcionalidades respaldado todavía.

## Evidencia y conservación

El registro se congeló antes de participantes con SHA-256 `a4ae0d3f2fcf44e283738534f37a990db6be674d304cd3de6ce00bb1646c723a`. Los archivos comunes y las vistas iniciales permanecieron intactos. Los seis Ledgers B pasaron `verify`: T1–T3 en v3, T4/T5 en v5 y T6 en v12. T6 conserva los nueve findings esperados de C01; integridad no equivale a aprobación de su checkpoint.

Las respuestas y cuatro traspasos están conservados localmente en `.tooling/research/e01/`, ligados por hashes en `results.json`. Incluyen metadatos de contexto y rutas locales y no se incorporan a documentación distribuida. Sus hashes permiten auditar los archivos conservados; por sí solos no permiten reproducir respuestas estocásticas. [Preparación de paquetes](../tools/prepare_e01.py) y [recolección](../tools/collect_e01.py) conservan el instrumento ejecutado; ambos rechazan reemplazar su resultado principal existente. Para repetir, se debe versionar el estudio y revisar antes las desviaciones aquí documentadas.

Los cambios de esta etapa se limitan al expediente y su estado documental. El código de producto, los contratos, la release y los proyectos fuente permanecen en sus sujetos originales.
