# G-Ledger Official Enterprise Delivery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convertir el cierre técnico de G-Ledger 0.2.0 en una entrega oficial, navegable, autocontenida y verificable, sin promoverla a GA ni atribuir compatibilidad externa no certificada.

**Architecture:** El repositorio conservará en sus rutas actuales la autoridad normativa y la evidencia hash-locked, pero tendrá un único centro documental en `docs/README.md`. El ZIP Windows expondrá una capa de entrega humana en la raíz, el ejecutable y una selección documental curada, mientras `resources/` seguirá siendo el inventario técnico verificado por el binario. GitHub publicará el resultado como prerelease ligado al SHA exacto que pase CI en `main`.

**Tech Stack:** Rust 1.97.1, Cargo, PowerShell 7, Pester, Git, GitHub CLI y GitHub Actions.

## Global Constraints

- La especificación normativa continúa siendo `specs/PROJECT_LEDGER_SPEC_v0.2.0.md`.
- No cambiar los 53 comandos, 67 eventos, schemas, locks ni contratos públicos v0.2.0 para acomodar la entrega.
- El estado distribuido permanece `ENGINEERING_RELEASE_CANDIDATE`; la publicación será prerelease y no General Availability.
- `certified_compatibility=false` permanece obligatorio.
- G-Skills, GSP y Project Radar no se presentan como productos certificados por esta entrega.
- El ZIP debe funcionar sin Cargo, Git, Python, checkout ni red para el núcleo.
- Ningún archivo de base de datos, secreto, caché, `target/`, `dist/` local o `.tooling/` entra al paquete.
- Toda salida se crea en destinos nuevos y se verifica después de descargarla desde GitHub.

---

### Task 1: Centro documental único y recorrido de lectura

**Files:**
- Create: `docs/README.md`
- Create: `docs/getting-started/QUICKSTART.md`
- Create: `docs/getting-started/TUTORIAL.md`
- Create: `docs/operations/USER_GUIDE.md`
- Create: `docs/assurance/README.md`
- Create: `docs/SOURCE_AND_BUILD.md`
- Modify: `README.md`
- Test: `scripts/tests/OfficialDelivery.Tests.ps1`

**Interfaces:**
- Consumes: especificación v0.2.0, ayuda real del CLI, informe OPA y documentos hash-locked existentes.
- Produces: una ruta de lectura `README.md` → `docs/README.md` → inicio rápido/tutorial/operación/aseguramiento, sin exigir conocer la historia interna.

- [ ] **Step 1: Write the failing documentation-structure test**

```powershell
It 'exposes one documentation home and the complete user path' {
    @(
        'docs/README.md',
        'docs/getting-started/QUICKSTART.md',
        'docs/getting-started/TUTORIAL.md',
        'docs/operations/USER_GUIDE.md',
        'docs/assurance/README.md',
        'docs/SOURCE_AND_BUILD.md'
    ) | ForEach-Object { Test-Path -LiteralPath (Join-Path $repoRoot $_) | Should -BeTrue }
}
```

- [ ] **Step 2: Run the test and verify RED**

Run: `Invoke-Pester scripts/tests/OfficialDelivery.Tests.ps1 -Output Detailed`

Expected: FAIL because `docs/README.md` and the curated guide files do not exist.

- [ ] **Step 3: Write the documentation home, quickstart, tutorial, user guide, assurance map and source/build guide**

The documents must explicitly identify: supported Windows platform, checksum verification, extraction, `resources verify`, first disposable project, golden-path session, continuity/checkpoint/resume, backup versus portable export, privacy, support boundaries, operational acceptance, source provenance and build verification.

- [ ] **Step 4: Replace the historical root README narrative with an official entrypoint**

The root README must contain product purpose, exact status, five-minute start, documentation map, distribution contents, support boundary, source layout and verification commands; implementation chronology remains in `CHANGELOG.md`, the implementation ledger and assurance evidence.

- [ ] **Step 5: Run the documentation test and verify GREEN**

Run: `Invoke-Pester scripts/tests/OfficialDelivery.Tests.ps1 -Output Detailed`

Expected: PASS with all required documents present and every relative Markdown link from the curated document set resolving locally.

- [ ] **Step 6: Commit**

```powershell
git add README.md docs scripts/tests/OfficialDelivery.Tests.ps1
git commit -m "docs: consolidate official G-Ledger documentation"
```

### Task 2: ZIP oficial autocontenido

**Files:**
- Create: `docs/DELIVERY_CONTENTS.md`
- Modify: `scripts/package-windows.ps1`
- Modify: `scripts/test-windows-package.ps1`
- Modify: `scripts/build-windows-package.ps1`
- Modify: `release/package-manifest.json`
- Test: `scripts/tests/OfficialDelivery.Tests.ps1`

**Interfaces:**
- Consumes: binario Windows static-CRT, catálogo incorporado, documentos curados y reportes de release/aceptación.
- Produces: ZIP con `START_HERE.md`, `project-ledger.exe`, licencia, changelog, seguridad, `documentation/`, `assurance/`, `resources/`, `SBOM.spdx.json`, `CHECKSUMS.sha256` y `package.json`.

- [ ] **Step 1: Extend the package regression and verify RED against the last package**

```powershell
$required = @(
    'START_HERE.md', 'project-ledger.exe', 'LICENSE', 'CHANGELOG.md',
    'SECURITY.md', 'SBOM.spdx.json', 'CHECKSUMS.sha256', 'package.json',
    'documentation/README.md', 'documentation/getting-started/QUICKSTART.md',
    'documentation/getting-started/TUTORIAL.md',
    'documentation/operations/USER_GUIDE.md',
    'assurance/G_LEDGER_OPERATIONAL_ACCEPTANCE_FINAL_REPORT_v1.0.0.md',
    'assurance/operational-pilot-report.json'
)
foreach ($name in $required) { $names.Contains($name) | Should -BeTrue }
```

Expected: the old ZIP fails because it only exposes the binary, license and internal resources at top level.

- [ ] **Step 2: Implement an explicit delivery allowlist**

`package-windows.ps1` must copy only reviewed files, preserve document hierarchy under `documentation/`, place the final human and machine OPA reports under `assurance/`, generate a sorted `CHECKSUMS.sha256`, and include every file except `package.json` itself in the signed-by-manifest inventory.

- [ ] **Step 3: Validate package completeness, integrity and exclusions**

The verifier must reject missing entrypoint/docs/SBOM/checksums, unlisted files, checksum drift, unsafe paths, databases, VCS metadata, `target`, `dist`, `.tooling`, secrets or a falsely certified compatibility marker.

- [ ] **Step 4: Build and exercise a new package from a clean commit**

Run: `pwsh -File scripts/build-windows-package.ps1 -Output <new-path>/project-ledger-0.2.0-windows-x86_64.zip -RequireClean`

Expected: package verification PASS, documented entrypoint present, core scenarios pass from an extracted path with spaces and non-ASCII characters.

- [ ] **Step 5: Commit**

```powershell
git add docs/DELIVERY_CONTENTS.md scripts release/package-manifest.json
git commit -m "build: produce self-contained official Windows delivery"
```

### Task 3: Release metadata and repository readiness

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `release/release-manifest.json`
- Create: `docs/RELEASE_NOTES_v0.2.0-rc.1.md`
- Modify: `.github/workflows/ci.yml`
- Modify: `BITACORA_IMPLEMENTACION.md`
- Modify: `docs/superpowers/plans/2026-09-04-g-ledger-official-delivery.md`

**Interfaces:**
- Consumes: OPA final, package layout and exact CI evidence.
- Produces: release notes legibles, artefacto preservado por CI y manifestación explícita de prerelease operacional.

- [ ] **Step 1: Add a failing policy assertion**

The delivery test must require the release notes to say `OPERATIONALLY_ACCEPTED`, `ENGINEERING_RELEASE_CANDIDATE`, `certified_compatibility=false`, `prerelease`, and to exclude GA/external certification claims.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `Invoke-Pester scripts/tests/OfficialDelivery.Tests.ps1 -Output Detailed`

Expected: FAIL because release notes do not yet exist.

- [ ] **Step 3: Write release notes and metadata**

Document the exact scope, platform, installation, artifact inventory, operational acceptance, limitations, known exclusions, rollback and support/verification route. CI must upload the ZIP and its verification report from Windows for the exact SHA.

- [ ] **Step 4: Update changelog, implementation ledger and executable plan**

Record the official delivery closure without rewriting prior pilot history and mark only actually completed checkboxes.

- [ ] **Step 5: Run the focused test and verify GREEN**

Run: `Invoke-Pester scripts/tests/OfficialDelivery.Tests.ps1 -Output Detailed`

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add CHANGELOG.md release docs .github/workflows/ci.yml BITACORA_IMPLEMENTACION.md
git commit -m "docs: prepare operational candidate release"
```

### Task 4: Full local qualification

**Files:**
- Modify only if a test exposes a defect, using a separate RED→GREEN commit.

**Interfaces:**
- Consumes: final clean candidate commit.
- Produces: fresh local evidence for formatting, lint, tests, contracts, release, dependency policy and Windows package.

- [ ] **Step 1: Run documentation and package-focused tests**

Run: `Invoke-Pester scripts/tests/OfficialDelivery.Tests.ps1,scripts/tests/OperationalPilotRunner.Tests.ps1 -Output Detailed`

- [ ] **Step 2: Run the full Rust/spec/release gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo xtask schema check
cargo run -p project-ledger-cli -- specs verify
cargo run -p project-ledger-cli -- contracts verify
cargo build --locked --release -p project-ledger-cli
cargo xtask benchmark check
cargo xtask release check
cargo run -p project-ledger-cli -- release verify --root .
cargo deny check
git diff --check
```

- [ ] **Step 3: Build and verify the official ZIP from the clean candidate**

Run the Windows package builder into a new `dist/` destination and retain the reported SHA-256 and verification JSON.

- [ ] **Step 4: Commit only evidence/document corrections, then repeat the full gate**

No completion statement is permitted until the last full run exits zero on a clean tree.

### Task 5: GitHub integration, merge and exact prerelease

**Files:**
- GitHub PR, Actions run, tag `v0.2.0-rc.1` and release assets.

**Interfaces:**
- Consumes: published final branch and clean local qualification.
- Produces: merged `main`, hosted CI artifact for the exact merge SHA, GitHub prerelease and independently redownloaded/verified package.

- [ ] **Step 1: Push `codex/g-ledger-operational-pilot` and open the final non-draft PR**

The PR body must identify operational acceptance, documentation consolidation, package inventory, unchanged compatibility boundary and exact local commands.

- [ ] **Step 2: Wait for every required PR check and inspect failures before merge**

Run: `gh pr checks <number> --watch`

Expected: all matrix, dependency, disk-full and live-GSP jobs pass for the exact PR SHA.

- [ ] **Step 3: Merge with history preserved and update local `main`**

Use a merge commit only after green hosted evidence. Confirm `origin/main` contains the delivery commits and the working tree is clean.

- [ ] **Step 4: Wait for the `main` CI run and download its Windows artifact**

The release subject is the ZIP generated by the successful `main` run for the exact merge SHA, not a pre-merge or local substitute.

- [ ] **Step 5: Create the prerelease and upload the official evidence set**

Publish `v0.2.0-rc.1` as a GitHub prerelease with the hosted ZIP, verification JSON, SBOM, release manifest, SHA-256 file and release notes. GitHub source archives provide the corresponding source; the portable includes the executable and user documentation.

- [ ] **Step 6: Redownload from the GitHub release into a new directory and verify**

Compare the published hash, run `test-windows-package.ps1`, inspect the extracted entrypoint and execute `resources verify`, `specs verify`, `contracts verify` and `release verify` from the downloaded artifact.

- [ ] **Step 7: Close obsolete qualification PRs and remove their remote branches**

Close PRs 1–4 with a note pointing to the merged final PR and prerelease. Delete only the exact obsolete remote branches after confirming they are not the release tag or `main`.

- [ ] **Step 8: Record final links and immutable identifiers**

Update the release notes through a release-edit operation if necessary with merge SHA, CI run, release URL, ZIP SHA-256 and verification status; do not amend the already merged source merely to write self-referential hosted IDs.

## Self-review

- Spec coverage: the plan preserves the normative spec, exact identity, privacy, offline core, release labels, checksums, SBOM and hosted multi-platform gates.
- Placeholder scan: no deferred implementation marker is authorized; angle-bracket values are command-time unique destinations or GitHub identifiers resolved during execution.
- Type consistency: package profile remains `project-ledger-windows-zip/1`; `certified_compatibility` remains boolean false; release status remains `ENGINEERING_RELEASE_CANDIDATE`; operational decision remains `OPERATIONALLY_ACCEPTED`.
