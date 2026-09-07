# Protocolo de calibración comparativa E01

Fecha: 2026-09-07. Estado: preparado para asignar participantes; **no ejecutado y no prerregistro confirmatorio**. C01 ya ejecutada mide el instrumento y superficies del CLI; no estima variabilidad entre usuarios o agentes. El investigador actual conoce los casos y no será presentado como participante independiente.

## Dos situaciones prioritarias

**U1 — Retomar después de un relevo de sesión o responsable.** Entregar estado vigente, cambio relevante, evidencia, incertidumbre y próxima acción válida. R02/R04/R06/R09 motivan la selección.

**U2 — Cerrar una sesión dejando un avance útil para otro responsable.** Registrar lo material, su sujeto y alcance, un pendiente o propuesta y las fuentes necesarias, sin volver a redactar todo el historial. R03/R05/R07/R08/R10 y el rechazo de escrituras de agente en C01 motivan observar este flujo. No se presupone que deba relajarse la autoridad para hacerlo cómodo.

## Tareas exploratorias y referencia de evaluación

Los diez casos del catálogo están vistos por quien desarrolla el prototipo: **ninguno es holdout**. E01 usa tareas de calibración; el estudio confirmatorio necesitará casos nuevos, reservados antes de cualquier ajuste.

| Tarea | Situación y caso | Respuesta o comportamiento que permite comprobar la calidad |
|---|---|---|
| T1 | U1, R04 | Separar commit de remediación y release publicada; no trasladar su calificación |
| T2 | U1, R06 | Mantener F10 bloqueada hasta método nativo y nueva prueba; no confundir lectura estática con RPC fresco |
| T3 | U1, R09, dos cortes temporales | Antes de aprobación: pendiente. Después: D1 cerrada y D2 autorizada; aquella aprobación no demuestra D2 implementada |
| T4 | U2, R07/R10 | Registrar DB o entorno probado y pruebas pendientes; conservar la condición de entrada siguiente |
| T5 | U2, R08 | Registrar el PASS posterior sin inventar causa ni corrección del fallo no reproducido |
| T6 | U1/U2, fixture C01 | Recuperar ocho incógnitas, detectar texto recortado y abstenerse de inferir autorización por ausencia de próxima acción |

Los cortes de T3 se preparan como paquetes separados: hechos posteriores y respuestas de esta tabla quedan fuera del paquete del participante. Compartir una carpeta sin aislamiento no constituye una sesión ciega. Para T4/T5 se mide también la preparación del registro y su interpretación por otro responsable.

## Condiciones y recursos

- **A:** fuentes equivalentes al corte + Git + estado actual conciso + decisiones/tareas bien mantenidas + herramientas y memoria habitual del participante.
- **B:** A más G-Ledger `53dc909`, binario SHA-256 `3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438`. Su ingestión y mantenimiento cuentan como coste. La alternativa A también puede producir resúmenes; no se restringe ese comportamiento.
- **C:** B más P1. Sólo se compara después de B; se imputa la lectura del JSON y cualquier recuperación posterior. P1 no aporta hechos que A/B no puedan obtener.
- El paquete bruto debe contener los mismos hechos, momentos, evidencia negativa y límites en todas las condiciones. Registrar diferencias de representación; revisión independiente del equilibrio antes de empezar.
- Primera calibración propuesta: seis pares de tareas, doce intentos A/B, con orden contrabalanceado. Es un presupuesto de descubrimiento, no una muestra con potencia confirmatoria. C se añade sólo si la calibración muestra una pregunta de presentación sin resolver.
- Sesiones de LLM nuevas y sin historial de respuestas: fijar modelo, esfuerzo, configuración, memoria común pertinente, herramientas y límites. Los humanos no repiten el mismo caso inmediatamente; usar variantes equivalentes y registrar experiencia previa.

Recursos aún necesarios: participantes independientes, disponibilidad, modelo/configuración si corresponde y presupuesto de consumo. No se han contratado servicios ni iniciado subagentes. La autorización del plan permite preparar el estudio; la configuración concreta debe reflejar participantes realmente disponibles, no identidades simuladas.

## Registro y juicio

Métrica primaria de calibración: **tiempo hasta una entrega correcta**, con máximo exploratorio de 15 minutos por intento y marca de no completado si vence. Conservar tiempo activo, espera, preparación, correcciones y llamadas por separado. No convertir fallos en observaciones rápidas exitosas. En U2 incluir interpretación por el siguiente responsable y mantenimiento; no comparar sólo latencia de lectura.

Rúbrica por intento: cinco dimensiones de 0–2 puntos — estado, temporalidad/sujeto, evidencia/razón, incertidumbres y próxima acción. 0: incorrecto o ausente; 1: parcial sin error material; 2: correcto y sustentado. Entrega correcta exige 2 en cada dimensión aplicable; justificar cualquier dimensión no aplicable antes de asignar la tarea. Abstenerse con razón y una petición concreta de evidencia puede ser correcto. Una acción no autorizada, una afirmación de ejecución inexistente o evidencia de otro sujeto es error crítico, independientemente de la suma.

Registrar: `participant_id`, `task_id`, `condition`, corte y hashes, orden, configuración, tiempos, tokens reales si se exponen, llamadas, costes con tarifas fechadas, puntuación por dimensión, error crítico, fuente que resuelve cada punto, correcciones, abandono y motivo. Las respuestas se evalúan contra fuentes por un revisor que no haya actuado como participante. Los errores críticos o discrepancias requieren revisión humana; un juez LLM único no basta.

No habrá dictamen de eficacia en E01. Después de la calibración se fijarán efecto mínimo que justifique adoptar, margen de calidad, muestra calculada a partir de variabilidad, presupuesto, exclusiones y parada **antes** de nuevos resultados. Publicar también equivalencias, resultados negativos y costes de preparación. Repeticiones del mismo episodio se agrupan; no se cuentan como proyectos independientes.

## Adopción y salida

Tras evidencia favorable, usar un proyecto nuevo y varias sesiones con al menos un relevo real. Registrar elección voluntaria, mantenimiento y abandonos sin exigir que se siga usando G-Ledger. El criterio de necesidad requiere beneficio neto y preferencia repetida; las pruebas de integridad, conteo o formato de P1 no satisfacen esa condición.

Siguiente acción concreta: asignar participantes y modelo/disponibilidad para E01, producir sus paquetes aislados y revisar el equilibrio de hechos. El mapa, P1 y la calibración actual siguen siendo descubrimiento; no se promueve código a producto ni se modifica un contrato con esta evidencia.
