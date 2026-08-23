# Schemas ejecutables

Este directorio contiene la forma ejecutable de los contratos del GSP.

## Reglas

- Todos los schemas usan JSON Schema Draft 2020-12.
- `core/gsp/schemas/common.schema.json` centraliza enums y tipos compartidos.
- `core/gsp/schemas/schema-registry.json` declara el inventario de schemas y su compatibilidad.
- `core/gsp/schemas/schema-registry.json` declara `resolution_base: core/gsp`; todo `path` del registry se resuelve contra esa raíz explícita, no contra el directorio físico del registry.
- `core/gsp/schemas/schema-registry.schema.json` valida la forma del registry GSP.
- `core/gsp/schemas/schema-registry.schema.json` declara `$id: core/gsp/schemas/schema-registry.schema.json`; este ID es único frente al registry raíz y se resuelve por path root-relative.
- Todo cambio de contrato debe pasar por fixtures y tests.

## Inventario

- `core/gsp/schemas/common.schema.json`
- `retrieval.request.schema.json`
- `evidence.bundle.schema.json`
- `semantic-compilation.request.schema.json`
- `ontology.snapshot.schema.json`
- `candidate.bundle.schema.json`
- `compilation.bundle.schema.json`
- `validation.report.schema.json`
- `uncertainty.map.schema.json`
- `compilation.feedback.schema.json`
- `memory.note.schema.json`
- `memory.write.proposal.schema.json`
- `memory.commit.schema.json`
- `session.state.schema.json`
- `context.plan.schema.json`
- `task.projection.request.schema.json`
- `task.projection.bundle.schema.json`
- `episodic.event.schema.json`
- `checkpoint.schema.json`
- `tool.execution.schema.json`
- `action.trace.schema.json`
