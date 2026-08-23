# Project Ledger v0.2.0 — Codex Handoff limpio

Este paquete contiene **solo los artefactos activos necesarios para iniciar la implementación de Project Ledger**. Sustituye como handoff operativo al ZIP anterior, que mezclaba spec, auditorías, history, QA, scripts, locks y copias upstream.

## 1. Autoridad

1. `PROJECT_LEDGER_SPEC_v0.2.0.md` — **única especificación normativa de implementación** de este handoff.
2. `INTEGRATION_LOCKS.json` — estado exacto de articulación con G-Skills, GSP, Project Radar y Codex/Agent Skill.
3. `PACKAGE_CHECKSUMS.sha256` — integridad del paquete.
4. Este archivo — orden de lectura y reglas operativas; no sustituye a la spec.

No hay `history/`, auditorías ni QA reports dentro del handoff porque **no son inputs de implementación**. Los antecedentes siguen existiendo fuera de este paquete, pero Codex no debe cargarlos salvo una investigación explícita de provenance.

## 2. Qué cambió respecto del ZIP anterior

- Se conserva Project Ledger **v0.2.0**, con revisión documental `GSKILLS-E2E-CONSOLIDATED-1-ALIGNMENT`.
- Se alinea la integración contra la G-Skills v0.3.0 adjunta, revisión `E2E-CONSOLIDATED-1`, SHA-256 `8ddbb2a9c23bf7355a1e577d93a0c541b0986df7416ab64069aea16295376928`.
- El lock actual distribuido por G-Skills todavía fija el antecedente Ledger SHA-256 `e7acc496...`; por tanto, **el lock recíproco debe refrescarse antes de certificar el adapter**, pero esto no bloquea el core de Ledger.
- Ledger deja de anunciar `RECORD_DECISION_REFERENCE` y `RECORD_BLOCKER_REFERENCE`. No tienen mapping canónico suficiente en v0.2.0 y no se permite inventarlo por analogía.
- El perfil Radar activo permanece `project-ledger-radar-artifact/0.1.1`, read-only Ledger → Radar mediante artifact exchange.
- GSP permanece exact-contract, opcional y read-only; sus schemas/fixtures exactos se bloquean cuando comience su slice.

## 3. Orden de implementación

Codex debe seguir los slices de la spec, en orden:

```text
S0  repository skeleton + preservation
S1  contracts/schema registry/closed unions
S2  event store + commands + submission reconciliation
S3  projection engine
S4  lifecycle/authority/roadmap/status/catalog
S5  gates/evidence/phase engine
S6  checkpoints/resume/Markdown/audit
S7  portable + legacy import
S8  generic integration protocol + SDK
S9  G-Skills provider
S10 G-Skills RecordSink
S11 GSP exact-contract bridge
S12 Project Radar artifact profile + reference importer fixtures
S13 Agent Skill/Codex
S14 hardening/benchmark/release
```

### Gate de integraciones

- **S0–S8:** pueden implementarse sin G-Skills, GSP o Radar.
- **S9–S10:** no declarar compatibilidad runtime hasta refrescar el lock recíproco G-Skills y superar schema/fixture locks, field mapping, loss report, conformance y E2E bilateral.
- **S11:** no certificar GSP hasta fijar sus contratos exactos, registry y fixtures upstream.
- **S12:** Ledger puede implementar schemas/export/fixtures Radar. Los cambios en IndexedDB/UI de Radar pertenecen al repositorio de Project Radar, no al de Ledger.
- **S13:** la Agent Skill enseña el procedimiento; no contiene estado canónico ni puede cerrar gates/fases.

## 4. Golden path obligatorio

La implementación debe seguir funcionando con:

```text
G-Skills      disabled
GSP           disabled
Project Radar disabled
AI            disabled
network       disabled
no daemon
```

Las integraciones enriquecen Project Ledger; no lo convierten en una superapp ni en una base compartida.

## 5. Regla para Codex

En la primera iteración, leer completamente la spec y ejecutar **solo S0 + S1**. No implementar adapters reales, UI, GSP, G-Skills, Radar o Agent Skill antes de sus slices.

Si una integración todavía está marcada `PENDING`, `LOCK_GATED` o `certified_compatibility=false`, Codex debe implementar únicamente los contratos genéricos permitidos por el slice y **no declarar compatibilidad certificada**.
