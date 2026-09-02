$ErrorActionPreference = 'Stop'

Describe 'Operational pilot runner Stage 1 preflight' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\bd564cd\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = '19429f1a9cb4431ec6f9bb06d18f5c8b49310666e894763ec45228bd083d0884'
    $expectedSourceCommit = 'bd564cdba07b616337681c15801d22478824108b'

    It 'extracts the exact hosted artifact into a new Unicode root and records its identity' {
        $pilotRoot = Join-Path $TestDrive 'Piloto á con espacios'

        $result = & $runner `
            -Stage 1 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-1-preflight.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        (Test-Path -LiteralPath (Join-Path $pilotRoot 'instalación con espacios\project-ledger.exe')) | Should Be $true

        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 1
        $manifest.status | Should Be 'PREFLIGHT_PASS'
        $manifest.subject.zip_sha256 | Should Be $expectedHash
        $manifest.subject.source_commit | Should Be $expectedSourceCommit
        $manifest.subject.version | Should Be '0.2.0'
        $manifest.capabilities.missing.Count | Should Be 0
        $manifest.project.project_ref | Should Match '^project:[0-9a-f-]{36}$'
        $manifest.project.state_version | Should Be 15
        $manifest.project.lifecycle_status | Should Be 'ACTIVE'
        $manifest.project.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.project.required_gate_count | Should Be 8
        $manifest.project.optional_gate_count | Should Be 1
        $manifest.scenarios.'OPA-S01' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S02' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S03' | Should Be 'PASS'

        $locator = Get-Content -Raw -LiteralPath (Join-Path $pilotRoot 'pilot-locator.json') | ConvertFrom-Json
        (Test-Path -LiteralPath $locator.database -PathType Leaf) | Should Be $true
        $state = & (Join-Path $pilotRoot 'instalación con espacios\project-ledger.exe') `
            status --database $locator.database --project-ref $locator.project_ref --json |
            ConvertFrom-Json
        $state.state_version | Should Be 15
        $state.roadmap.phase_order | Should Be @('phase:operational-acceptance')
        $state.pending_gate_refs.Count | Should Be 9
        $result.stage | Should Be 1
        $result.status | Should Be 'PREFLIGHT_PASS'
    }

    It 'rejects a mismatched artifact hash before creating the pilot root' {
        $pilotRoot = Join-Path $TestDrive 'hash mismatch root'

        {
            & $runner `
                -Stage 1 `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 ('0' * 64) `
                -ExpectedSourceCommit $expectedSourceCommit
        } | Should Throw 'PILOT_ZIP_HASH_MISMATCH'

        (Test-Path -LiteralPath $pilotRoot) | Should Be $false
    }

    It 'rejects an existing pilot root without altering its sentinel' {
        $pilotRoot = Join-Path $TestDrive 'existing root'
        $null = New-Item -ItemType Directory -Path $pilotRoot
        $sentinel = Join-Path $pilotRoot 'preserve.me'
        [System.IO.File]::WriteAllText($sentinel, 'unchanged')

        {
            & $runner `
                -Stage 1 `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        } | Should Throw 'PILOT_DESTINATION_EXISTS'

        (Get-Content -Raw -LiteralPath $sentinel) | Should Be 'unchanged'
        (Test-Path -LiteralPath (Join-Path $pilotRoot 'instalación con espacios')) | Should Be $false
    }
}

Describe 'Operational pilot runner Stage 2 governed model' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\bd564cd\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = '19429f1a9cb4431ec6f9bb06d18f5c8b49310666e894763ec45228bd083d0884'
    $expectedSourceCommit = 'bd564cdba07b616337681c15801d22478824108b'

    It 'proves read-only queries, authority boundaries, supersession history, and fail-closed lifecycle' {
        $pilotRoot = Join-Path $TestDrive 'Stage 2 á governed model'

        $null = & $runner `
            -Stage 1 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit

        $result = & $runner `
            -Stage 2 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-2-governed-model.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json

        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 2
        $manifest.status | Should Be 'GOVERNED_MODEL_PASS'
        $manifest.read_only.before_state_version | Should Be 15
        $manifest.read_only.after_state_version | Should Be 15
        $manifest.read_only.before_state_digest | Should Be $manifest.read_only.after_state_digest
        $manifest.read_only.queries.Count | Should Be 10
        ($manifest.read_only.queries | Where-Object { $_.exit_code -ne 0 }).Count | Should Be 0
        $manifest.agent_authority_attempt.exit_code | Should Be 9
        $manifest.agent_authority_attempt.state_version_delta | Should Be 0
        $manifest.agent_authority_attempt.state_digest_unchanged | Should Be $true
        $manifest.authority_history.original_document_ref | Should Be 'authority:opa-aux-v1'
        $manifest.authority_history.original_status | Should Be 'SUPERSEDED'
        $manifest.authority_history.replacement_document_ref | Should Be 'authority:opa-aux-v2'
        $manifest.authority_history.replacement_status | Should Be 'CURRENT'
        $manifest.authority_history.original_digest_preserved | Should Be $true
        $manifest.authority_history.replacement_supersedes_original | Should Be $true
        $manifest.lifecycle.state_version | Should Be 21
        $manifest.lifecycle.project_status | Should Be 'ACTIVE'
        $manifest.lifecycle.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.lifecycle.completed_phase_count | Should Be 0
        $manifest.lifecycle.completed_project_without_command | Should Be $false
        $manifest.lifecycle.all_gates_unverified | Should Be $true
        $manifest.repository_safety.binding_mode | Should Be 'DECLARATIVE_READ_ONLY'
        $manifest.repository_safety.canary_executed | Should Be $false
        $manifest.repository_safety.binding_present | Should Be $true
        $manifest.artifact_reference.present | Should Be $true
        $manifest.scenarios.'OPA-S04' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S05' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S06' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S07' | Should Be 'PASS'
        $result.stage | Should Be 2
        $result.status | Should Be 'GOVERNED_MODEL_PASS'
    }
}
