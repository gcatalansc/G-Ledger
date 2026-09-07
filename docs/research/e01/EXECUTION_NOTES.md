# E01 — condiciones observadas durante ejecución

Autorización posterior del propietario: «Sí, autoriza subagentes para E01». Se prepararon doce paquetes para seis pares A/B y se congeló el registro antes de iniciar participantes: SHA-256 `a4ae0d3f2fcf44e283738534f37a990db6be674d304cd3de6ce00bb1646c723a` de `registration.json`.

Los tres archivos comunes de cada par (`TASK.md`, `CURRENT_STATE.md`, `SOURCES.md`) tienen bytes idénticos. B añade un índice de estudio construido con el CLI actual y fuentes de configuración; no se introducen aprobaciones humanas simuladas. La configuración B es deliberadamente mínima: progreso y checkpoint, más las incógnitas del fixture C01 en T6. No representa un despliegue completo de fases, gates o decisiones canónicas de cada proyecto original.

## Exposición de contexto descubierta

Los primeros participantes informaron que la aplicación suministra automáticamente `MEMORY_SUMMARY`, incluso con `fork_turns=none`. Contiene resúmenes anteriores de proyectos, algunos relacionados con los casos. No habían abierto archivos de memoria. Esto corrige la expectativa inicial del registro sobre ausencia de memoria nativa precargada; el registro congelado se conserva y esta nota documenta lo observado después de comenzar.

Se mantiene el intento como calibración exploratoria, con exposición declarada por participante. No se afirma aislamiento de sistema operativo, prueba ciega, eliminación efectiva del conocimiento previo ni equivalencia exacta de contextos internos. No se modificaron fuentes o tareas después de ver respuestas. En las instrucciones siguientes se hizo explícito que la exposición automática también debía declararse; los primeros participantes ya la habían registrado espontáneamente. La comparación confirmatoria requiere un entorno que permita inspeccionar y fijar ese contexto.

## Medición y límites

Las horas de inicio y fin proceden de consultas al reloj que reporta cada participante; describen la duración de ese intento e incluyen razonamiento y herramientas. No hay telemetría independiente de tokens, precio, esfuerzo interno ni segundos de trabajo humano. El identificador concreto de modelo no está expuesto por la API de subagentes; se hereda la configuración del padre sin overrides. No se lo inferirá de una descripción genérica de la familia.

Se usó el orden aleatorizado registrado, llenando hasta tres slots según se liberaban. Con seis pares y una ejecución por condición, orden, concurrencia, cachés y variación del servicio pueden afectar tiempos. Los datos no permiten atribuir causalmente una diferencia a G-Ledger ni calcular un efecto generalizable. El investigador que preparó las fuentes evalúa contra la rúbrica y conoce la condición; su juicio no reemplaza validación humana independiente de errores críticos.

Para U2, B registra además el progreso y el checkpoint solicitados. Ese coste incremental es parte de este flujo configurado. La preparación automatizada de los índices, el mantenimiento longitudinal y el eventual ahorro de relectura posterior quedan fuera de los tiempos de participante; no se puede calcular beneficio neto de adopción con esos tiempos.

## Corte final y cobertura

Terminaron los doce intentos autorizados. Todos declararon exposición automática de contexto. El registro mantuvo su hash original y las entradas comunes e iniciales conservaron sus bytes. Once entregas son claras bajo la revisión del investigador; T6A requiere adjudicación humana por posible confusión entre el contenido del checkpoint histórico y la proyección actual. Se preserva esa incertidumbre y no se decide a favor de B por ella.

Respecto del diseño inicial, T3 se ejecutó como reconstrucción de dos momentos dentro del mismo paquete; no se evaluó con paquetes temporales separados. T4 usó R07 y T6 fue U1. U2 produjo cuatro traspasos, sin otro participante que los retomara. La revisión previa del equilibrio fue realizada por el investigador mediante igualdad de archivos y fuentes; no hubo un revisor humano independiente. T6 incluye el JSON estático también en A, por lo que es diagnóstico de representación y no comparación pura sin G-Ledger. Estas limitaciones impiden declarar completa la métrica primaria prevista.

El informe final conserva resultados descriptivos y decisiones en [REPORT.md](REPORT.md); el juicio queda ligado por hash a [results.json](results.json). No se ejecutaron participantes de la condición C ni más intentos después de revisar resultados.
