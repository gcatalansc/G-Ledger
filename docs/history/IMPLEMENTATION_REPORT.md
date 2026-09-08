# Implementación del núcleo de historia

Fecha local: 2026-09-07. Base: `5c20e7b3817098260312faf5eec30645934b4262`.
Rama: `codex/history-core`. El commit que contiene este informe registra la entrega.

## Resultado

Implementadas las cuatro etapas: contrato y arquitectura, modelo estructurado,
núcleo/CLI y validación acotada. Un LLM puede registrar alternativas, intentos y
vínculos con decisiones; reconstruir cortes completos; comparar estados; seguir
supersesiones y evidencia; recuperar el contenido completo y sus correcciones.

La escritura incorpora el avance habitual en la misma transacción. Las consultas
usan una lectura consistente de SQLite/WAL. Se preservan eventos, digests, spec y
los 142 archivos generados v0.2. La extensión añade ocho schemas y amplía la matriz
a 54 comandos; el catálogo base sigue con 53 comandos y 67 eventos, más el nuevo
evento de episodio. El cache reconstruible cambia de versión interna.

El binario local está en `target/debug/project-ledger.exe`; la guía ejecutable está
en [LLM_USAGE.md](LLM_USAGE.md). Esta implementación y sus recursos son posteriores
a rc.1. No se publicó una nueva release ni se cambió la certificación externa.

## Verificación

| Comprobación | Resultado |
|---|---|
| Seis escenarios CLI sobre SQLite real | 6/6 PASS |
| Escritura | Atomicidad, replay, concurrencia, permisos, campos cerrados, referencias e invalidación de checkpoint |
| Reconstrucción | Cortes indivisibles, correcciones sin alterar el pasado, diferencias y detalle completo |
| Traza | Supersesión, alternativa/intento explícitos, evidencia invalidada, ausencias y paginación fijada al corte |
| Portabilidad | v0.2 y stream mixto v0.2/v0.3, igualdad de bytes de eventos tras importar |
| Regresión consolidada del workspace | 315 casos únicos aprobados; 14 casos especiales permanecen ignored |
| Formato y Clippy estricto | PASS, sin advertencias |
| Schemas/specs/contratos | PASS; v0.2 sin cambios; extensión v0.3 reproducible |
| Recursos del ejecutable | 461 archivos VERIFIED; prueba de instalación aislada PASS |
| Ejercicio directo y backup/restore | 14 llamadas CLI exitosas, corte anterior intacto, restauración exacta |

La regresión se ejecutó completa una vez tras actualizar el primer hash de la guía.
Mostró 302 aprobados y 13 fallos: referencia superior del paquete de instrucciones,
inventario/SBOM por el cambio de dependencias internas y el rechazo de una versión
Radar usada como entrada negativa. Este último detectó que el constructor de
eventos normalizaba también una versión no admitida: se restringió la conversión
a los eventos auxiliares de un comando de episodio v0.3 y se preservó el error
de versión no soportada.

Después se repitieron 56 casos de las superficies afectadas, incluidos los 13
fallidos y los seis de historia: todos pasaron. La cifra 315 consolida esas
ejecuciones; no afirma una segunda pasada completa. Un intento general anterior
se detuvo en el primer desfase del hash de instrucciones. Las pruebas especiales
de entornos/procesos externos no se activaron para esta entrega.

Los únicos metadatos de release regenerados fueron el inventario, checksums y
namespace del SBOM ligado a Cargo.lock; no se reescribió la aceptación histórica.

## Uso directo observado

El LLM implementador registró el problema real encontrado con Serde: una entrada
válida era rechazada con `unknown field description`. Conservó el camino inicial,
la alternativa de deserialización explícita y el resultado de la corrección.
Después recuperó el fallo y la razón de la alternativa mediante `history get`,
comparó v3→v5 y reconstruyó nuevamente v3. Replay dejó v5 intacta; backup/restore
produjo la misma respuesta y digest. Se usó `agent:implementation`, sin `--as-human`.

[Registro del ejercicio](direct-use-report.json). SHA-256 del ejecutable utilizado:
`3914a46ce432afe7134772c69563ff37d159c4bf30ece5a88eb868d0fd2266d2`.

El ejercicio es uso directo de la CLI por el implementador, con observaciones de
esta sesión registradas después de ocurrir. No es un participante independiente,
una prueba longitudinal ni una medición de productividad. Los resultados de
intentos se registraron como declaraciones, sin referencias a evidencia canónica.
La trayectoria formal de decisiones/evidencia se verificó con los escenarios CLI.

## Límites concretos

La reconstrucción recorre el stream; no se ensayó rendimiento a gran escala. Trace
pagina sus entradas, mientras `at`, `diff` y `get` devuelven contenido completo.
Los motivos y relaciones deben registrarse explícitamente. No se infiere historia
faltante, no se aceptan decisiones mediante un vínculo ni se consulta una URI al
leer. Un lector rc.1 no entiende eventos v0.3; el lector nuevo conserva soporte v0.2.

Los resultados C01/E01 permanecen históricos. La mejora funcional está entregada;
la superioridad comparativa y la adopción quedan para una decisión posterior.

Evidencia local de ejecución: `.tooling/history-core/workspace-tests-resumed.log`,
`targeted-recheck.log`, `clippy-final.log` y `direct-use-final/report.json`. Las bases
descartables y los logs con rutas locales quedan fuera de Git.
