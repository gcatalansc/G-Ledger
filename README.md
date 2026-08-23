# Project Ledger

Project Ledger es un motor local de continuidad verificable para proyectos desarrollados por personas y agentes.

El repositorio ha materializado S0–S14: preservación, contratos cerrados, JSON canónico, SQLite/WAL append-only, cadena SHA-256 obligatoria, ejecución transaccional de comandos, idempotencia, concurrencia optimista, reconciliación de submissions, inicialización offline, proyección determinista de `ProjectState`, lifecycle, gates/evidencia, continuidad, portabilidad, protocolo genérico de integración, provider/RecordSink G-Skills exactos, bridge GSP exact-contract, perfil de artefacto read-only Project Radar, Agent Skill/Codex gobernada y un gate de hardening/release reproducible. La caché de proyección es descartable y se reconstruye desde el stream canónico ante ausencia, obsolescencia o corrupción.

S5 incorpora runner local sin shell y con allowlist, importación de evidencia con degradación de confianza, los 17 operadores cerrados de `PredicateAst`, `GateEvaluation` inmutable, waivers humanos, claims inicialmente `ASSERTED`, cierre explícito por Phase Engine y revalidación por evidencia, autoridad o snapshot de repositorio. El `CommandProcessor` entrega a los motores exactamente el stream cuya cadena verificó y conserva expected version, idempotencia, matriz cerrada y append atómico.

S6 incorpora checkpoints inmutables con captura N/evento N+1, freshness append-only, versiones material/no material separadas, `ResumeProjection` compacto y para agente bajo cotas conservadoras, Markdown derivado de trece secciones con perfiles de privacidad y auditoría de divergencia. `CoreCommandHandler` registra progreso/unknowns/checkpoints e invalidaciones compatibles en una sola transacción SQLite; varios repositorios vigentes se rechazan sin selección implícita.

S7 añade el árbol portable normativo con JSON/JSONL canónico, manifest autocontenido, checksums, schema/upstream locks transportados, lectura sin traversal/symlinks y privacidad fail-closed. El import clasifica cadena idéntica, proyecto nuevo, fast-forward, destino más nuevo y divergencia antes de mutar; SQLite revalida el sufijo y lo anexa atómicamente. El importador Markdown conserva digest/sección/líneas/contradicciones, recomienda siempre `REVIEW` y sólo produce propuestas `ASSERTED` tras una decisión explícita por candidato; no existe `TRUST_ALL`.

S8 añade builders/verificadores canónicos de manifest, health, projection envelope y loss report; identidad exacta sin fallback por nombre; inspección conservadora de upstream locks; framing stdio JSON-RPC 2.0 `Content-Length`; artifact exchange atómico no-clobber; y un harness que ejecuta 100 projections deterministas, current/stale, límites, timeout, privacidad, requests hostiles y failure isolation contra una tupla observable sin mutaciones. `project-ledger integration manifest` y `integration compatibility` exponen las superficies genéricas read-only.

S9 añade los seis contratos read-only exactos de `gskills-context-provider/0.3.0`, mapping de continuidad sin pérdida material para la solicitud normativa completa, freshness separada por proyecto/repositorio, budgets, privacidad minimizada, manifest/health y un handler stdio con sólo tres métodos de lectura. El adapter recibe una `ProjectStateProjection` inmutable y metadata de fases explícita; no abre una DB compartida ni registra métodos mutables. Su harness ejecuta 100 projections current, stale, nueve requests hostiles, recuperación post-fallo y equivalencia directa/stdio con cero mutaciones observables.

S10 añade cinco contratos write exactos, policy externa con `TRUSTED_FOR_RECORDING`, mapping cerrado de cinco comandos, expected state, idempotencia cross-system, `ExternalRecordResult`, lookup durable y JSON-RPC mutable separado. Validate, status y reconcile son read-only; timeout/outcome unknown nunca se convierte en resultado de dominio. Replay produce un solo efecto y los conflictos no sobrescriben estado.

S11 fija GSP-Gemma Compiler v0.2 y `gsp-exact-2026.04.12`: 28 contratos, 31 schemas y 140 fixtures byte-locked. `interop-gsp` construye un corpus explícito/acotado, valida el `EvidenceBundle` y los seis artefactos completos del `CompilationResult`, conserva contradicción/incertidumbre en receipts/reports y aísla un sidecar opcional sin storage. Exportar/validar son read-only; sólo `link-result` crea un `LINK_EXTERNAL_EVIDENCE` nativo y explícito.

S12 añade `project-ledger-radar-artifact/0.1.1`: cinco schemas y diez fixtures raw-locked, proyección/envelope deterministas, identity fuerte consumer-owned, importador de referencia puro, merge idempotente, ownership no destructivo y CLI SQLite read-only para proyecto o catálogo explícito. Nombre/alias nunca autoriza apply y el perfil no contiene tombstones, acceso a IndexedDB ni writes de lifecycle.

S13 añade `project-ledger-agent-skill/0.2.0`: `SKILL.md` descubrible, manifest/workflow cerrados, locks de bytes, 7 triggers, 5 políticas y conformance para missing-tool, conflicto de versión, outcome incierto y no-authority. El handoff CLI real expone resume read-only y checkpoint gobernado con expected version/idempotency; la skill no contiene estado vigente ni puede leer DB, editar Markdown generado, verificar gates o completar fases.

S14 añade `project-ledger-release`: políticas fail-closed para trust/argv, escáner de privacidad sin eco de valores, evidencia exacta AC-001…090 y PL-01…32, threat model, caos, migración v0.1→v0.2 no destructiva/reversible, paquete single-binary, SBOM SPDX 2.3, checksums y benchmark p95 con entorno declarado. El estado distribuido es `ENGINEERING_RELEASE_CANDIDATE`: AC-086 y AC-089 conservan waivers no críticos por ejecución hosted Linux/macOS y peak-RSS no medido, que bloquean GA.

La semántica ejecutable sigue acotada por la especificación: el counterpart/IndexedDB de Project Radar, UI, daemon general, AI embebida, el counterpart RecordSink de G-Skills y el E2E live-sidecar GSP permanecen fuera de la evidencia local. `certified_compatibility=false` continúa siendo obligatorio; S9–S14 demuestran fronteras locales/procedimentales y readiness de ingeniería, no obediencia universal del agente, compatibilidad bilateral/live-sidecar, General Availability, operación productiva ni validez científica. `GeneratedArtifactRecord` se devuelve junto con los bytes renderizados, pero no se persiste porque el contrato v0.2.0 no define un command/event para ello. De igual modo, los contratos congelados no definen una mutación durable de fork: la divergencia falla cerrada y `FORK_EXPLICIT` devuelve target nuevo más provenance de origen sin reescribir eventos ni authority.

## Autoridad

- Especificación normativa: [`specs/PROJECT_LEDGER_SPEC_v0.2.0.md`](specs/PROJECT_LEDGER_SPEC_v0.2.0.md)
- Antecedente inmutable: [`specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md`](specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md)
- Orden operativo del handoff: [`referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md`](referencias/PROJECT_LEDGER_v0.2.0_CODEX/00_START_HERE.md)
- Bitácora de implementación: [`BITACORA_IMPLEMENTACION.md`](BITACORA_IMPLEMENTACION.md)
- Matriz de cobertura contractual: [`docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md`](docs/CONTRACT_SCHEMA_FIXTURE_TEST_MATRIX.md)

## Comandos de desarrollo

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo run -p project-ledger-cli -- integration gskills verify-contracts
cargo run -p project-ledger-cli -- integration gsp verify-contracts
cargo run -p project-ledger-cli -- integration radar verify-contracts
cargo run -p project-ledger-cli -- integration agent-skill verify-contracts
cargo build --locked --release -p project-ledger-cli
cargo xtask benchmark check
cargo xtask release check
cargo run -p project-ledger-cli -- release verify --root .
```

Para regenerar SBOM, package manifest y checksums tras un cambio del grafo final:

```powershell
cargo run --locked --release -p xtask -- benchmark run --root .
cargo xtask release generate --root .
cargo xtask release generate --check --root .
```

El benchmark versionado usa 20 muestras en Windows x86_64/Rust 1.97.1 y un stream canónico de 10.000 eventos. Los targets medidos pasan; la memoria one-shot p95 permanece waiver explícito y no se infiere de una aproximación.

Inicialización offline contra una ubicación explícita:

```powershell
cargo run -p project-ledger-cli -- init `
  --title "Mi proyecto" `
  --summary "Continuidad verificable" `
  --timezone "America/Santiago" `
  --data-root ".tmp-user-data"
```

Sin `--data-root`, el CLI usa la convención de user-data del sistema operativo. Cada proyecto se almacena en `project-ledger/projects/<uuid>/ledger.db`; el contrato conserva el identificador canónico `project:<uuid>` y el directorio usa sólo el UUID para ser válido en Windows.

Superficies genéricas de integración:

```powershell
cargo run -p project-ledger-cli -- integration manifest
cargo run -p project-ledger-cli -- integration compatibility
```

El primer comando emite JSON canónico. El segundo inspecciona evidencia exacta de locks y conserva cualquier ausencia como `PENDING`; ninguno habilita adapters, network, authority o writes.

Agent Skill/Codex y handoff gobernado:

```powershell
cargo run -p project-ledger-cli -- integration agent-skill verify-contracts
cargo run -p project-ledger-cli -- resume --database <ledger.db> --project-ref project:<uuid-v7> --format codex
cargo run -p project-ledger-cli -- checkpoint create --database <ledger.db> --project-ref project:<uuid-v7> --reason session-end --expected-version <N> --idempotency-key <key> --actor-id agent:codex --observed-at <timestamp-rfc3339>
cargo run -p project-ledger-cli -- resume --database <ledger.db> --project-ref project:<uuid-v7> --format json
```

La skill vive en `integrations/agent-skills/project-ledger-continuity/`. Resume sólo verifica/reconstruye y no añade eventos. Checkpoint usa el command boundary canónico; replay exacto es idempotente y un version conflict no se reintenta a ciegas. Si falta tool/subcommand, la skill reporta degradación explícita y no sustituye el motor con acceso directo a DB o parsing de Markdown.

Provider G-Skills read-only sobre un ledger y catálogo de fases explícitos:

```powershell
cargo run -p project-ledger-cli -- serve provider `
  --stdio `
  --profile gskills-context-provider/0.3.0 `
  --database <ruta-al-ledger.db> `
  --project-ref project:<uuid-v7> `
  --gskills-project-ref project:<ref-confirmada> `
  --phase-catalog <ruta-a-phases.json> `
  --observed-at <timestamp-rfc3339>
```

`phases.json` es un array cerrado de contratos `Phase` v0.2.0. El proceso verifica la cadena SQLite, reconstruye la proyección completa y rechaza metadata de fase faltante/contradictoria, más de un binding `CURRENT`, aliases de profile, write capability, raw events y budgets inválidos. Stdio admite frames sucesivos y registra únicamente `ledger.manifest`, `ledger.health` y `ledger.project_continuity`.

RecordSink G-Skills sobre un ledger y una policy explícita:

```powershell
cargo run -p project-ledger-cli -- serve sink `
  --stdio `
  --profile gskills-context-provider/0.3.0 `
  --database <ruta-al-ledger.db> `
  --project-ref project:<uuid-v7> `
  --gskills-project-ref project:<ref-confirmada> `
  --policy <ruta-a-record-sink-policy.json> `
  --observed-at <timestamp-rfc3339>
```

La policy es obligatoria, cerrada y digest-valid; fija identidades, actor, `TRUSTED_FOR_RECORDING`, cinco command kinds y los digests de manifest/ejecutable/lock/identity. No se descubre desde metadata del repositorio. El proceso registra sólo `ledger.manifest`, `ledger.command_validate`, `ledger.command_submit`, `ledger.command_status` y `ledger.command_reconcile`.

Bridge GSP por artifact exchange local:

```powershell
cargo run -p project-ledger-cli -- integration gsp verify-contracts
cargo run -p project-ledger-cli -- integration gsp export-corpus --database <ledger.db> --project-ref project:<uuid-v7> --request <scope.json> --output <corpus.json>
cargo run -p project-ledger-cli -- integration gsp validate-result --evidence-bundle <corpus.json> --result <compilation-result.json> --output <validated.json>
cargo run -p project-ledger-cli -- integration gsp link-result --database <ledger.db> --project-ref project:<uuid-v7> --receipt <receipt.json> --receipt-uri artifact://gsp/receipt.json --expected-state-version <N> --idempotency-key <key> --actor-id <actor> --observed-at <timestamp-rfc3339>
```

`scope.json` es un `GspCorpusRequest` cerrado, con query, referencias seleccionadas y budgets. El corpus queda unido por digest canónico a su manifest, contenidos y `EvidenceBundle`. Los tres primeros comandos no mutan el stream. `link-result` verifica identidad/digest del receipt y su binding con la proyección SQLite vigente, y exige concurrencia, idempotencia, actor, URI y tiempo explícitos; no puede cerrar fases, verificar gates ni cambiar lifecycle.

Perfil Project Radar por artifact exchange local:

```powershell
cargo run -p project-ledger-cli -- integration radar verify-contracts
cargo run -p project-ledger-cli -- integration radar export --database <ledger.db> --project-ref project:<uuid-v7> --phase-catalog <phases.json> --source-instance-ref project-ledger://instances/local --observed-at <timestamp-rfc3339> --output <radar.json>
cargo run -p project-ledger-cli -- integration radar validate-export --input <radar.json>
cargo run -p project-ledger-cli -- catalog radar-export --all --catalog <known-ledgers.json> --source-instance-ref project-ledger://instances/local --observed-at <timestamp-rfc3339> --output <radar-catalog.json>
```

`known-ledgers.json` es un array explícito de `database`, `project_ref`, `phase_catalog` y `source_aliases`; no existe discovery del filesystem. Los exports verifican cadena/proyección, son canónicos, read-only y no-clobber. El importador incluido es sólo una referencia pura/fixture: no abre Project Radar ni su IndexedDB.

La red no forma parte del runtime ni de las pruebas. La descarga inicial de dependencias es una operación de desarrollo de Cargo.
