# Project Ledger

Project Ledger es un motor local de continuidad verificable para proyectos desarrollados por personas y agentes.

## Estado operativo actual

**APTO PARA PILOTO CONTROLADO; avance verificado 8/8.** Las ocho etapas pre-piloto y PP-01…PP-14 están cerradas. La CI privada [33664393505](https://github.com/gcatalansc/G-Ledger/actions/runs/33664393505) pasó sus seis jobs sobre `b171f51cb58c60ed8c8a99b60386117f9fed6822`; el ZIP alojado fue descargado y revalidado fuera del runner. Evidencia y límites en [entrega Windows](docs/PREPILOT_WINDOWS_DELIVERY.md), [plan pre-piloto](docs/superpowers/plans/2026-08-27-project-ledger-prepilot-readiness.md), [matriz de brechas](docs/PREPILOT_GAP_MATRIX.md) e [informe de preparación](release/prepilot-readiness.json). No se ha iniciado un proyecto real: este estado no significa GA, certificación general ni ausencia absoluta de defectos.

El [piloto de aceptación operativa](docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md) está en **0/8, BLOCKED**. Stage 1 descubrió `OPA-DEFECT-001`, corregido en `bd564cdba07b616337681c15801d22478824108b`; su ZIP limpio `19429f1a…083d0884` pasó gate integral, empaquetado y preflight local, pero todavía no tiene procedencia hosted. No se iniciará Stage 2 hasta recalificar y descargar el artefacto alojado del mismo SHA. La [matriz](docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md) y el [reporte](docs/pilot/results/operational-pilot-report.json) separan evidencia viva, regresión, contexto histórico y exclusiones; esto no equivale todavía a adopción operativa.

RQ4 sí quedó cerrado: la CI `33062468586` sobre `dea208d294dfddc282b84dcb5a58b9d8d9887b91` terminó con seis jobs success. Esa evidencia acredita el ensayo de crash/recovery y su regresión, no elimina las brechas operativas siguientes ni certifica GSP.

## Historial de componentes implementados

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

S14 añade `project-ledger-release`: políticas fail-closed para trust/argv, escáner de privacidad sin eco de valores, evidencia exacta AC-001…090 y PL-01…32, threat model, caos, migración v0.1→v0.2 no destructiva/reversible, paquete single-binary, SBOM SPDX 2.3, checksums y benchmark p95 con entorno declarado. Las calificaciones posteriores cierran AC-089 con peak-RSS del proceso release y AC-086 con una matriz hosted Windows/Linux/macOS sobre un SHA exacto. El estado distribuido continúa `ENGINEERING_RELEASE_CANDIDATE`; la evidencia de ingeniería no promueve automáticamente General Availability ni certifica integraciones externas.

La calificación de caos separa el fault determinista SQLite por límite de páginas del `ENOSPC(28)` real observado en ext4 sobre una imagen loopback efímera de 32 MiB. El append falla cerrado y la recuperación conserva el digest previo, versión, comandos, resultados y eventos; el volumen de prueba se desmonta y elimina. `release verify` interpreta el inventario hash-locked y expone por separado los waivers de caos, sus bloqueos GA y los bloqueos de caos para certificación runtime GSP. `physical-disk-full` queda cubierto por el run hosted `33035586838`; `live-sidecar-process-crash`, por el run `33060749387` sobre GSP v0.2.0 real y fijado por SHA. La terminación del proceso propio conserva cadena/proyección, el reinicio produce seis artefactos válidos y sólo el enlace explícito añade un evento, con replay sin duplicados. Quedan cero waivers de caos; esto no certifica toda la integración.

La semántica ejecutable sigue acotada por la especificación: el counterpart/IndexedDB de Project Radar, UI, daemon general, AI embebida, el counterpart RecordSink de G-Skills y la conformidad runtime GSP completa permanecen pendientes. `certified_compatibility=false` continúa siendo obligatorio; S9–S14 y las calificaciones posteriores demuestran fronteras locales/procedimentales y el ensayo GSP externo acotado, no obediencia universal del agente, compatibilidad bilateral universal, General Availability, operación productiva ni validez científica. `GeneratedArtifactRecord` se devuelve junto con los bytes renderizados, pero no se persiste porque el contrato v0.2.0 no define un command/event para ello. De igual modo, los contratos congelados no definen una mutación durable de fork: la divergencia falla cerrada y `FORK_EXPLICIT` devuelve target nuevo más provenance de origen sin reescribir eventos ni authority.

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

El benchmark versionado usa 20 muestras en Windows x86_64/Rust 1.97.1 y un stream canónico de 10.000 eventos. La medición fresca del 2026-08-28 UTC pasa los nueve targets; `memory_one_shot_p95` es 19,684 MiB frente al presupuesto de 100 MiB, calculado como p95 del máximo RSS muestreado cada 1 ms durante `project-ledger contracts verify`. Es una medición de ingeniería ligada al entorno, no un pico kernel exacto ni una garantía universal.

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

Compilación externa opcional entre `export-corpus` y `validate-result` (rutas absolutas explícitas al intérprete y checkout GSP):

```powershell
& $env:PROJECT_LEDGER_GSP_PYTHON -I -B integrations/gsp/sidecar/compile_bundle.py --source-root $env:PROJECT_LEDGER_GSP_SOURCE_ROOT --input <corpus.json> --output <compilation-result.json>
```

El checkout debe estar limpio en `7a43559c02417641691dbb5f2a6e79c13d74d1a1`, con entorno Python 3.12 provisionado mediante su `uv.lock` frozen. El adapter verifica el pin y los bytes contractuales originales, no recibe acceso a SQLite, no llama modelos y rechaza entrada mayor a 1 MiB, resultado mayor a 8 MiB o salida existente. Los bytes generados requieren `validate-result` antes de cualquier enlace. No es un daemon ni una dependencia del golden path Rust. El ensayo opt-in de compilación/crash/reinicio se ejecuta con `cargo test -p project-ledger-cli --test gsp_bridge live_sidecar:: -- --ignored --nocapture --test-threads=1` y esas dos variables explícitas.

Perfil Project Radar por artifact exchange local:

```powershell
cargo run -p project-ledger-cli -- integration radar verify-contracts
cargo run -p project-ledger-cli -- integration radar export --database <ledger.db> --project-ref project:<uuid-v7> --phase-catalog <phases.json> --source-instance-ref project-ledger://instances/local --observed-at <timestamp-rfc3339> --output <radar.json>
cargo run -p project-ledger-cli -- integration radar validate-export --input <radar.json>
cargo run -p project-ledger-cli -- catalog radar-export --all --catalog <known-ledgers.json> --source-instance-ref project-ledger://instances/local --observed-at <timestamp-rfc3339> --output <radar-catalog.json>
```

`known-ledgers.json` es un array explícito de `database`, `project_ref`, `phase_catalog` y `source_aliases`; no existe discovery del filesystem. Los exports verifican cadena/proyección, son canónicos, read-only y no-clobber. El importador incluido es sólo una referencia pura/fixture: no abre Project Radar ni su IndexedDB.

La red no forma parte del runtime ni de la compilación ensayada. La descarga inicial de dependencias Cargo y la provisión opcional del checkout/entorno GSP son operaciones de desarrollo.
