# Guía de usuario de G-Ledger 0.2.0

## Qué gobierna G-Ledger

G-Ledger es la fuente canónica de continuidad operacional de un proyecto. Registra hechos de alto nivel y mantiene trazabilidad append-only. No sustituye Git, un sistema de tareas, un repositorio documental ni la observabilidad técnica detallada.

Debe registrar:

- avance material y artefactos relevantes;
- evidencia y su procedencia;
- gates y fases mediante sus motores propietarios;
- decisiones propuestas, aceptadas y supersedidas;
- blockers, riesgos, unknowns y resoluciones;
- próxima acción autorizada;
- checkpoints de continuidad.

No debe registrar por defecto:

- conversaciones completas o razonamiento privado;
- stdout/stderr crudo;
- secretos, credenciales o bases compartidas;
- cada archivo tocado o cada comando trivial;
- afirmaciones externas como autoridad propia.

## Ciclo operativo recomendado

```text
verificar instalación
    ↓
resume al comenzar
    ↓
leer fase, gates, blockers, riesgos y próxima acción
    ↓
trabajar fuera de G-Ledger
    ↓
registrar sólo cambios materiales y evidencia
    ↓
evaluar gates / solicitar lifecycle cuando corresponda
    ↓
crear checkpoint
    ↓
resume para el siguiente responsable
```

## Identidad, concurrencia e idempotencia

Cada operación se dirige a `database` y `project_ref` explícitos, o a un localizador que contenga exactamente ambos. Un nombre humano nunca sustituye la identidad canónica.

Toda escritura necesita:

- `expected_version`: evita sobrescribir cambios concurrentes;
- `idempotency_key`: permite repetir la misma intención sin duplicarla;
- `actor_id`: identifica al actor;
- `observed_at`: fija el momento observado;
- trust domain apropiado; `--as-human` sólo representa una acción humana local efectiva.

Ante exit `5`, lea de nuevo con `resume` y replantee la operación. Ante outcome incierto, use `reconcile` con el mismo actor, tipo, trust domain y clave; no reenvíe a ciegas.

## Evidencia, gates y fases

Una evidencia `OBSERVED` con exit `0` no verifica un gate. La procedencia y el tipo de evidencia deben cumplir la política declarada. Sólo Gate Engine verifica gates y sólo Phase Engine habilita el cierre de una fase.

Un archivo existente, tests verdes, documentación o un comando aceptado no completan por sí solos un proyecto. El lifecycle deriva de transiciones explícitas y gates requeridos.

## Vistas y privacidad

`render` genera Markdown con perfiles `minimal-local`, `shareable` y `full-local`. La vista es descartable y regenerable. No la use como entrada para mutar el Ledger.

Use `shareable` antes de entregar una vista fuera del entorno local. Aun así, revise el resultado: la ausencia de un canary conocido en superficies soportadas no demuestra ausencia universal en todos los bytes internos.

## Backup, restore y portable

| Necesidad | Operación | Conserva |
|---|---|---|
| Recuperación privada exacta | `backup` / `restore` | SQLite/WAL, resultados, idempotencia, eventos |
| Intercambio minimizado | `export --format portable` / `import` | Historia contractual y locks permitidos |
| Lectura humana | `render` | Proyección, no autoridad |

Restore siempre escribe en un destino nuevo. Un checksum incorrecto, corrupción, divergencia o destino existente se rechazan. El portable no sustituye el backup.

## Integraciones

- Agent Skill/Codex invoca la CLI; no lee SQLite ni completa lifecycle.
- Provider G-Skills es read-only; RecordSink exige una policy externa exacta para escribir.
- GSP intercambia artefactos; sólo el enlace explícito agrega evidencia externa.
- Project Radar se limita a un perfil de artefacto read-only en esta versión.

Estas superficies conservan `certified_compatibility=false` hasta validación bilateral independiente.

## Diagnóstico

Ejecute, en este orden:

```powershell
.\project-ledger.exe resources verify --json
.\project-ledger.exe doctor --locator .\mi-ledger.local.json --json
.\project-ledger.exe verify --locator .\mi-ledger.local.json --json
.\project-ledger.exe audit --locator .\mi-ledger.local.json --json
.\project-ledger.exe resume --locator .\mi-ledger.local.json --format json
```

No edite SQLite, WAL, manifests o Markdown generado para corregir un hallazgo. Conserve el error y consulte la [Referencia operativa de la CLI](../CLI_OPERATIONS.md), el [Threat model](../THREAT_MODEL.md) y la política de reporte en [SECURITY.md](../../SECURITY.md).

