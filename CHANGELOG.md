# Changelog

Todos los cambios relevantes de Project Ledger se documentan en este archivo.

## [Unreleased]

### Added

- Baseline documental v0.2.0 y preservación verificable de v0.1.0.
- Plan de implementación de la primera fase contractual S0 + S1.
- Esqueleto del workspace Rust, CI y políticas del repositorio.
- JSON canónico RFC 8785, SHA-256 tipado, UUIDv7, timestamps RFC 3339 y referencias validadas.
- Contratos durables S1 y uniones cerradas de 53 comandos y 67 eventos.
- Registro offline de 46 schemas con 92 fixtures, semántica de arrays y matriz comando-evento.
- CLI read-only para verificar antecedentes y contratos, además de generación determinista mediante `xtask`.
- Puertos `event-store`/`application` independientes de SQLite y adapter `storage-sqlite` con migraciones v1–v2 verificadas, WAL, claves foráneas, bloqueo acotado y `synchronous=FULL`.
- Append de eventos inmutable, versiones contiguas, cadena SHA-256 obligatoria, lectura verificada y detección fail-closed de corrupción.
- Procesador de comandos con matriz cerrada, expected version, idempotencia por scope, resultados persistentes y handler real `CREATE_PROJECT`.
- Historial append-only de `ExternalSubmissionRecord`, compare-and-set y reconciliación sin reintento ciego ni `OUTCOME_UNKNOWN` en resultados servidor.
- `project-ledger init` offline y reporte de recuperación con `quick_check`, WAL, migración, cadena, conteos y submissions inciertas.
- `projection-engine` storage-independent con digest canónico de `ProjectState`, rebuild desde génesis, aplicación incremental equivalente y rechazo explícito de reducers aún no implementados.
- Caché SQLite descartable ligada a versión/head/projector/digest, con reconstrucción y reparación automática ante ausencia, obsolescencia o corrupción.
- Consulta de estado que verifica primero la cadena canónica y mantiene invariantes de no mutación tras 100 lecturas consecutivas.
- Cursor lifecycle con reducers fail-closed para autoridad, repositorios/snapshots, roadmap secuencial, fases, gates, evidencia, claims, decisiones, blockers, riesgos, próximas acciones y conflictos; `ProjectStateProjector` v3 exige estado derivado vigente al final del stream.
- Motor puro de comandos S4 conectado al stream verificado por `CommandProcessor`, con transiciones humanas explícitas, estado derivado determinista e IDs UUIDv7 separados criptográficamente por comando/ordinal.
- Catálogo local derivado y reconstruible exclusivamente desde proyectos conocidos explícitos, sin adquirir autoridad ni escanear filesystem.
- Crate `gate-engine` con los 17 operadores declarativos cerrados, selección de evidencia por kind/status/snapshot, evaluaciones inmutables, waivers y Phase Engine explícito; evidencia verde aislada nunca cierra gate/fase.
- Crate `evidence` con ejecución exacta programa+argv sin shell, allowlist, salida cruda desactivada, resúmenes acotados/redactados y import unsigned degradado a `OBSERVED`.
- Freshness determinista por `EVIDENCE_INVALIDATED`, `AUTHORITY_DOCUMENT_CHANGED` y `REPOSITORY_DEPENDENCY_CHANGED`, con gate stale/revalidation y reapertura de fase cuando la matriz lo permite.
- Modelo ejecutable de claims: alta únicamente `ASSERTED`; `VERIFIED` requiere actor humano y referencias a evidencia `VERIFIED_SOURCE`.
- `CoreCommandHandler` compone owners S2–S5 sobre el mismo stream verificado y conserva una sola transacción SQLite/matriz/idempotencia.
- `projection-engine` v4 reconstruye work progress, unknowns, checkpoints inmutables y registros append-only de freshness; `CHECKPOINT_CREATED/INVALIDATED` no avanzan `material_state_version`.
- Crate `continuity` con checkpoint N/N+1 determinista, rechazo de selección ambigua entre repositorios, invalidación matrix-compatible y composición atómica en `CoreCommandHandler`/SQLite.
- `ResumeProjection` compacto ≤220 y Markdown para agente ≤450 usando bytes UTF-8 como cota conservadora reproducible ante la ausencia de tokenizer contractual.
- Renderer Markdown puro con warning normativo, trece secciones ordenadas, perfiles `FULL_LOCAL`/`MINIMAL_LOCAL`/`SHAREABLE`, acotación/redacción y `GeneratedArtifactRecord` verificable.
- Auditoría de stream/origen/digest que detecta `GENERATED_ARTIFACT_MODIFIED`, permite regeneración determinista y mantiene visibles checkpoints stale, unknowns materiales y conflictos críticos.
- Crate `portable-bundle` con árbol §31.1 determinista, JSONL canónico, manifest/checksums/schema lock, lectura segura, controles de privacidad y rechazo de overwrite.
- Política portable verificada para cadena idéntica, proyecto nuevo, fast-forward, destino más nuevo y divergencia fail-closed; `FORK_EXPLICIT` expone nueva identidad y provenance sin inventar una mutación durable ausente del contrato.
- Puerto `EventImportStore` y append SQLite transaccional de sufijos ya verificados, con revalidación de versión/identity/digest y rollback íntegro ante conflicto.
- Crate `legacy-import` con extracción conservadora por etiqueta/sección/línea, contradicciones bidireccionales, review individual obligatorio, ausencia de `TRUST_ALL` y propuestas exclusivamente `ASSERTED`.
- Crate `integration-protocol` con builders/verificadores canónicos de manifest, health, projection envelope y loss report, límites, privacidad fail-closed, exact identity y upstream-lock inspection sin overclaim.
- Harness read-only con 100 projections deterministas, current/stale, timeout medido, 7 fixtures hostiles, detección de mutación observable, redacción y failure isolation; sin dependencia de SQLite ni promesa de sandbox universal.
- Crate `transport-stdio` con framing/dispatch JSON-RPC 2.0 `Content-Length` estricto y acotado; crate `artifact-exchange` con JSON RFC 8785, digest, límites y publicación atómica no-clobber.
- CLI genérica `integration manifest`/`integration compatibility`, semánticamente equivalente al manifest directo/stdio y conservando G-Skills, GSP y Radar como no certificados.
- Crate `interop-gskills` con lock/read schemas exactos v0.3.0, mapping completo state/material/fases/gates/acciones/checkpoint/repositorio, freshness, budgets y privacidad sin acceso a SQLite ni métodos mutables.
- Harness G-Skills S9 con 100 lecturas current, stale, nueve requests hostiles, detección de mutación, cero pérdida material, recuperación post-fallo y equivalencia directa/stdio.
- CLI `integration gskills verify-contracts` y `serve provider --stdio --profile gskills-context-provider/0.3.0` sobre stream SQLite verificado y catálogo explícito de fases.
- Cinco contratos write G-Skills exactos, policy explícita con write trust separado y mapping cerrado a `RECORD_WORK_PROGRESS`, `RECORD_VERIFICATION_RESULT`, `REQUEST_GATE_EVALUATION`, `REQUEST_GATE_REVALIDATION` y `CREATE_CONTINUITY_CHECKPOINT`.
- RecordSink storage-independent con validate sin efectos, submit idempotente, mapping exacto de `CommandResult`, status/reconcile durable sin retry y separación estricta de outcome incierto.
- Handler stdio mutable de cinco métodos, conformance local de cero duplicate effects/blind retries y CLI `serve sink` sobre `CommandProcessor`/SQLite real con `--policy` obligatorio.
- Crate `interop-gsp` con 31 schemas y 140 fixtures upstream exactos, corpus minimizado explícito, `EvidenceBundle` completo y validación de los seis artefactos de `CompilationResult` sin modelos sustitutos reducidos.
- Receipts GSP digest-addressed con contradicción/incertidumbre y semantic status visibles, sidecar opcional sin storage/Python en el core y único link posible mediante `LINK_EXTERNAL_EVIDENCE` explícito.
- CLI `integration gsp verify-contracts|export-corpus|validate-result|link-result` con export/validate read-only, no-clobber, binding digest de manifest+`EvidenceBundle`, contraste del receipt contra la proyección SQLite vigente, expected version e idempotencia determinista; compatibilidad local completa pero no certificada sin E2E live-sidecar.
- Crate `interop-radar` con profile `project-ledger-radar-artifact/0.1.1`, cinco schemas/diez fixtures raw-locked, export canónico, mapping status/progress conservador, diez attention signals y privacidad mínima.
- Importador Project Radar de referencia storage-independent: link confirmado fuerte, nombres sólo candidatos, duplicate/stale/update/divergence separados, omission no destructiva, campos Radar-owned preservados y provenance Ledger transitiva.
- CLI `integration radar verify-contracts|export|validate-export` y `catalog radar-export --all` sobre ledgers explícitos; cadena/proyección SQLite verificadas, cero eventos, no overwrite y compatibilidad local completa pero `certified_compatibility=false` hasta counterpart/E2E bilateral.
- Crate `agent-skill` y paquete `project-ledger-agent-skill/0.2.0` con manifest/workflow schemas, raw locks, triggers positivos/negativos y políticas ejecutables para degradación, reconciliación y no-authority.
- CLI `integration agent-skill verify-contracts`, `resume --format codex|json` read-only y `checkpoint create` gobernado; handoff SQLite real, replay idempotente y conflicto de versión fail-closed sin completion bypass.
- Gate local reproducible con formato, Clippy estricto, workspace tests, contratos exactos y política de dependencias.
