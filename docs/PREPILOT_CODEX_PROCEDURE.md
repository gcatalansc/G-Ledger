# Etapa 6 — procedimiento Codex verificable

Estado: 6/8; etapa 6 verificada localmente. No certifica obediencia universal de modelos.

La skill propia conserva manifest y workflow v0.2 y añade `examples/operational-session.json`: argv con placeholders, versionado y comprobación de capacidades. Los ejemplos se ejecutan mediante procesos CLI reales en `crates/cli/tests/codex_readiness.rs`; no se limitan a comparar texto. No se instala una skill global ni se altera la contraparte G-Skills.

## Recorrido y límites

Detección/versionado/capacidades → resume → progreso → ejecución de evidencia con policy explícita → evaluación de gate → checkpoint → reconciliación por clave/actor/tipo/trust → resume. La autorización humana del fixture se prepara por separado; el agente no añade `--as-human` ante una denegación.

La evidencia del ejemplo ejecuta el binario con `--version`, usa parser genérico y permanece OBSERVED. La evaluación se acepta como comando pero devuelve exit 6 y deja el gate incompleto. El checkpoint no equivale a completar una fase.

Casos negativos: ejecutable inexistente (fallo real de creación de proceso), versión 0.1.0 emitida por un proceso fixture propio (no una release antigua real), capacidad ausente, denegación 9, concurrencia 5 y respuesta descartada por el cliente después de checkpoint. La reconciliación recupera el resultado durable sin repetir la escritura. Un resultado inexistente da 12, no permiso para reenviar con otra clave. La reserva de ejecución incierta de etapa 4 conserva su bloqueo.

## Evidencia de esta sesión Codex

Proyecto descartable `project:01a04602-a78b-7061-8260-1ccf072ceddf`, dentro de `.tooling/prepilot-codex-demo`, no proyecto real. Exclusivamente CLI para acceder a Ledger; no SQLite directo ni edición de Markdown generado en este recorrido. Fixture humano de autoridad/roadmap/gate/fase preparado también mediante CLI.

- Detección 0.2.0 y cinco subcomandos disponibles; contratos de skill MATCH.
- Resume inicial v6; progreso v7; evidencia v8; evaluación registrada v10 con exit 6. Gate sigue pendiente y fase activa.
- Pause como agente rechazado 9, delta 0. Checkpoint v11; se descartó intencionadamente su stdout en el cliente. Reconciliación recuperó `checkpoint:76745e67-1db9-7f41-870e-d57a9967e3fd`, digest del resultado `sha256:9ee1e1b5d3c67705cf5ae7248c6bf28f78449cc10b548286bb7b4e5aa3a97b69`.
- Clave inexistente: exit 12. Audit CLI: integrity ok, 10 comandos/resultados, una ejecución, cero findings, v11.
- Binario observado antes de la corrección de resume: SHA-256 `0bd00163ee07bfdb2e1c3b8a6f949ba92e47b45eb3dc6deeb04407a99ac26bc4`.

## Hallazgo reproducido y corrección

La prueba real detectó que resume JSON reutilizaba el presupuesto de texto compacto (220 unidades conservadoras), eliminando autoridad, gates y progreso para encajar el checkpoint. La proyección y el historial sí conservaban esos datos. RED confirmado con una copia del ejecutable previo y aserción explícita; E2E reforzado y corregido. ResumeBudget::Structured conserva las listas completas en JSON y en resume.json portable; el resumen de trabajo sigue acotado/redactado. Los budgets de texto de §26.2 no se aplican a eliminar datos de JSON. El Markdown anuncia que es una vista acotada y dirige a JSON para IDs completos; se agregó también la línea de autoridad. Sin cambios de schemas o eventos.

Repetición desde Codex, después de la corrección: v11 → nuevo progreso/evidencia/evaluación incompleta → checkpoint v16. Reconciliación y resume conservan authority:spec, gate:contracts, phase-0 y el resumen «Resume JSON preserva referencias; gate pendiente»; completed_phase_refs sigue vacío. Checkpoint `checkpoint:8bc87ad5-82e7-743f-a91a-cab906dd76c6`, state_digest `sha256:7dd62879939bab493a880147165e45b5dc967db24dac2d5e11275591dda5f012`. Audit: integrity ok, 14 comandos/resultados, dos ejecuciones, cero findings, v16. Binario SHA-256 `1347e3e0d623fc2f7c71f0e5bd63862c04b5199cb63bac7b7433a21a4988317b`.

Tests focales de procedimiento y continuidad PASS (sesión 44889). Gate completo fresco 24790 exit 0: formato, Clippy estricto, workspace/all-targets/all-features, build locked, schemas/specs/contratos, release check, cargo deny offline, checksums y diff. Incluye la corrección y las aserciones nuevas; los tests anteriores no sustituyen esta evidencia.

`skill-creator` guio la separación entre instrucciones breves, ejemplos ejecutables y estado fuera del paquete; su validador pasó después del último cambio. TDD/diagnóstico permitieron reproducir y resolver el hallazgo antes del commit. No nueva CI alojada, ZIP, piloto o certificación en esta etapa.
