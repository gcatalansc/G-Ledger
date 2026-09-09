# Changelog

Todos los cambios relevantes de Project Ledger se documentan en este archivo.

## [Unreleased]

### Distribution

- Candidato `0.3.0-rc.1`: versión propia del ejecutable CLI, manifiesto de distribución generado y SBOM coherente. `release verify` distingue evidencia base v0.2 de la distribución candidata sin aceptación operativa concedida.
- ZIP Windows perfil 2: verificación de versión y clasificación antes de empaquetar, notas propias y recorrido histórico sobre el ejecutable extraído. CI de la rama histórica conserva sus seis trabajos y evita el segundo build release de Windows.

### Added

- G-Skills: perfil read-only `gskills-record-confirmation/0.1.0` y `ledger.command_confirmation`, con policy independiente, contratos candidatos separados y prueba minimizada de resultado/cortes/eventos sobre un único snapshot verificado. Adenda PL-GSK-003 autorizada el 2026-09-08; se conserva la línea publicada rc.1.

- Historia v0.3: episodios, alternativas, intentos, vínculos explícitos con decisiones y correcciones append-only; registro atómico junto al avance habitual.
- CLI `record episode`, `history at`, `history diff`, `history get`, `decision trace` paginado y exposición de `evidence invalidate` bajo la política existente.
- Catálogo adicional v0.3, lectura de cortes completos y portable mixto v0.2/v0.3. Eventos, digests y schemas v0.2 preservados.
- Guía de uso directo por LLM y seis escenarios de integración sobre procesos y SQLite reales.

### Fixed

- Prueba CLI de supervisión: el marcador de ejecución pertenece al arnés y admite cero escrituras cuando el timeout incluye un arranque lento. Un caso controlado reproduce el timeout antes del marcador; se mantienen el plazo de 700 ms, la prohibición de reintentos inciertos y los controles de privacidad/replay. No cambia el ejecutable del producto.
- G-Skills RecordSink (PL-GSK-004): segunda validación compatible con rechazo nativo por versión obsoleta; finalización transaccional durable de `VERSION_CONFLICT` ante carreras de commit; recuperación ligada al comando nativo completo para impedir atribuciones con clave, scope o payload alterados. Calificación bilateral local 12/12, sin nueva publicación.
- G-Skills RecordSink (PL-GSK-003, parte interna): `status` y `reconcile` reconstruyen el corte atómico del resultado persistido aunque el proyecto haya avanzado. Conservan resultado/digest y rechazan identidad, cortes o cadena inválidos. La confirmación histórica adicional se incorpora mediante la adenda separada autorizada.
- G-Skills RecordSink (PL-GSK-002): una versión que cambia antes del envío conserva la versión aprobada y alcanza el rechazo nativo `VERSION_CONFLICT`, recuperable por `status` y `reconcile` sin eventos adicionales. Las validaciones de identidad y aprobación permanecen activas; el alcance no incluye carreras durante el commit.
- SQLite/WAL en Windows: selección del VFS `win32-longpath` y rutas extendidas para inicialización, lectura, escritura, respaldo, restauración y migración.
- Publicación atómica de backups, imports portable y artefactos locales en rutas largas, manteniendo el rechazo de sobrescrituras.
- Regresión del recorrido portable completo con rutas de datos de más de 1.100 caracteres, espacios y Unicode, incluida en el verificador Windows.
- Resumen vigente de implementación, referencia de `risk close` y trazabilidad documental de la publicación rc.1, con límites externos explícitos.

Registro y verificación: `docs/maintenance/2026-09-07-TECHNICAL_REMEDIATION.md`, accesible desde el centro documental. Estos cambios son posteriores al ZIP rc.1 publicado.

## [0.2.0-rc.1] - 2026-09-04

### Added

- Centro documental único con inicio rápido, tutorial ejecutado, guía de usuario, mapa de aseguramiento, procedencia de fuente/build y notas de release.
- Entrega Windows autocontenida con `START_HERE.md`, documentación completa, evidencia de aceptación, SBOM SPDX 2.3, checksums internos y manifiesto de contenido.
- Verificación fail-closed de estructura documental, enlaces locales, clasificación de release, contenido del ZIP, exclusiones privadas y checksums de cada archivo.
- Publicación como prerelease operativo: `OPERATIONALLY_ACCEPTED`, `ENGINEERING_RELEASE_CANDIDATE`, no GA y `certified_compatibility=false`.
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
- Crate `release` con policy de trust, exact argv/no-shell, escáner de secretos/transcripts/private reasoning, paths normalizados y gate hash-locked sobre 90 AC, 32 PL y siete superficies declaradas.
- Threat model, chaos report, migración v0.1→v0.2 revisada/no destructiva/reversible y compatibility report con cuatro perfiles locales completos pero nunca certificados.
- Package manifest single-binary, SBOM SPDX 2.3, checksums reproducibles y benchmark release p95 de 20 muestras/10.000 eventos; la calificación posterior mide `memory_one_shot_p95` en 17,855 MiB sobre Windows x86_64/Rust 1.97.1 y cierra AC-089.
- CLI `release verify` y `cargo xtask release generate|check`/`benchmark run|check`; CI construye el binario release y ejecuta el gate en Windows, Linux y macOS.
- Gate local reproducible con formato, Clippy estricto, workspace tests, contratos exactos y política de dependencias.
- Verificación semántica hash-locked del inventario de caos, con contadores separados para waivers, bloqueos General Availability y bloqueos de certificación runtime GSP.
- Prueba SQLite `SQLITE_FULL` en la conexión real y harness hosted de kernel `ENOSPC(28)` sobre ext4/loopback efímero de 32 MiB; rollback y reapertura preservan cadena/digest/versionado sin llenar el disco del host.
- Calificación RQ3 `physical-disk-full` con run/job/SHA hosted exactos: se retira su waiver GA; en ese cierre permanecía `live-sidecar-process-crash` para certificación runtime GSP.
- Adapter GSP opcional one-shot por archivos: ejecuta el compilador upstream real fijado, verifica checkout y schemas, acota entrada/salida y no recibe una DB ni sobrescribe artefactos existentes.
- RQ4: tres tests live de compilación/rechazo/crash con proceso propio, terminación OS y reap, cadena/proyección intactas, reinicio, seis artefactos exactos, enlace explícito y replay sin duplicados. Job hosted separado con entorno frozen, 18 tests upstream y hashes/contratos/fixtures.
- Regresión de parseo del workflow publicado y guard fail-closed para evidencia live con fault, contraparte exacta y locator hosted. Cierre del último waiver de caos tras el run `33060749387` en `30c17ac0253a581db69b7fe38e001da07c9216d8`; todas las certificaciones permanecen falsas y no hay promoción GA automática.
