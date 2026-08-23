# Compatibilidad G-Skills v0.3.0

- Source revision: `PL-UPSTREAM-REFRESH-1`, con corrección contractual `6d63b60`.
- Source spec SHA-256: `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`.
- Profile exacto: `gskills-context-provider/0.3.0`.
- Estado del provider: `S9_READ_PROFILE_PASS`.
- Estado del RecordSink: `S10_LOCAL_RECORD_SINK_PASS`.
- Compatibilidad certificada: no.
- Lock recíproco de spec: `CURRENT`; G-Skills fija Ledger `31cb7cd4...` y Ledger fija G-Skills `957e62be...`.
- Registry de archivo: `a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9`.
- Schemas/fixtures: once contratos exactos, once positivos y once negativos; cinco corresponden a la superficie write S10.
- Mapping: solicitud normativa completa de 20 campos, metadata de fases reconciliada y cero pérdida material.
- Conformance: 100 lecturas current + stale/post-failure, nueve requests hostiles, cero mutaciones y equivalencia directa/stdio.
- Métodos provider publicados: `ledger.manifest`, `ledger.health`, `ledger.project_continuity`.
- Métodos RecordSink publicados en proceso separado: `ledger.manifest`, `ledger.command_validate`, `ledger.command_submit`, `ledger.command_status`, `ledger.command_reconcile`.
- Authority efectiva fuera de dominios project: vacía.
- RecordSink: policy externa digest-valid con `TRUSTED_FOR_RECORDING`, cinco capabilities cerradas, expected version, scope de idempotencia, resultado nativo separado de incertidumbre de transporte y reconciliación durable sin retry.
- Capabilities S10: `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION` y `CREATE_CONTINUITY_CHECKPOINT`; `RECORD_DECISION_REFERENCE`, `RECORD_BLOCKER_REFERENCE` y transiciones lifecycle directas permanecen denegadas.
- Bloqueo posterior: E2E bilateral con el counterpart G-Skills S24.

Los reportes ejecutables read son `provider-field-mapping.json`, `provider-loss-report.json` y `provider-conformance.json`; los write son `record-sink-field-mapping.json`, `record-sink-loss-report.json` y `record-sink-conformance.json`. Cada familia mantiene gate y proceso separados.

El adapter no comparte la base de datos con G-Skills. La composición provider abre exclusivamente almacenamiento Ledger, verifica su cadena, reconstruye una proyección inmutable e inyecta un catálogo `Phase` cerrado. La composición RecordSink abre el mismo almacenamiento sólo desde la CLI Ledger, carga obligatoriamente una policy explícita y ejecuta mediante `CommandProcessor`; manifest o metadata de repositorio nunca conceden write trust.

Esta evidencia es local de ingeniería y profile conformance read/write. No certifica implementación counterpart de G-Skills, compatibilidad bilateral, operación productiva, rentabilidad ni validez científica; por ello `certified_compatibility=false` permanece obligatorio.
