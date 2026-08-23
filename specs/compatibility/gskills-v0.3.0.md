# Compatibilidad G-Skills v0.3.0

- Source revision: `PL-UPSTREAM-REFRESH-1`, con corrección contractual `6d63b60`.
- Source spec SHA-256: `957e62be56d7aa86865da5ce4da7605e7bfd5ee220aac37e031a53b9e0a9950e`.
- Profile exacto: `gskills-context-provider/0.3.0`.
- Estado del provider: `S9_READ_PROFILE_PASS`.
- Compatibilidad certificada: no.
- Lock recíproco de spec: `CURRENT`; G-Skills fija Ledger `31cb7cd4...` y Ledger fija G-Skills `957e62be...`.
- Registry de archivo: `a6c3a12c01258878a924e915cd8053b0ae05cf1da9b871e4a0efd86b8628c4e9`.
- Schemas/fixtures read-only: seis contratos exactos, seis positivos y seis negativos.
- Mapping: solicitud normativa completa de 20 campos, metadata de fases reconciliada y cero pérdida material.
- Conformance: 100 lecturas current + stale/post-failure, nueve requests hostiles, cero mutaciones y equivalencia directa/stdio.
- Métodos publicados: `ledger.manifest`, `ledger.health`, `ledger.project_continuity`.
- Authority efectiva fuera de dominios project: vacía.
- Bloqueos posteriores: RecordSink S10 y E2E bilateral.
- Capabilities mutables anunciadas en S9: ninguna.

Los reportes ejecutables son `integrations/gskills/reports/provider-field-mapping.json`, `provider-loss-report.json` y `provider-conformance.json`. El profile read-only PASS no habilita `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION`, `CREATE_CONTINUITY_CHECKPOINT`, `RECORD_DECISION_REFERENCE` ni `RECORD_BLOCKER_REFERENCE`; esas capacidades requieren el RecordSink S10 y su gate propio.

El adapter no comparte la base de datos con G-Skills. La composición CLI abre exclusivamente almacenamiento Ledger, verifica su cadena, reconstruye una proyección inmutable e inyecta un catálogo `Phase` cerrado. Falta de metadata, status contradictorio, identidad ambigua, campos/budgets/privacy inválidos o un método mutable fallan cerrados.

Esta evidencia es local de ingeniería y profile conformance. No certifica implementación counterpart de G-Skills, compatibilidad bilateral, operación productiva, rentabilidad ni validez científica.
