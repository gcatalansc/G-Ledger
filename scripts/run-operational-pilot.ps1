[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateRange(1, 8)]
    [int]$Stage,

    [Parameter(Mandatory)]
    [string]$LedgerZip,

    [Parameter(Mandatory)]
    [string]$PilotRoot,

    [Parameter(Mandatory)]
    [string]$RepositoryRoot,

    [string]$ExpectedZipSha256 = '19429f1a9cb4431ec6f9bb06d18f5c8b49310666e894763ec45228bd083d0884',

    [string]$ExpectedSourceCommit = 'bd564cdba07b616337681c15801d22478824108b'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function ConvertTo-PilotCanonicalJson {
    param([AllowNull()]$Value)

    if ($null -eq $Value) { return 'null' }
    if ($Value -is [string]) { return ($Value | ConvertTo-Json -Compress) }
    if ($Value -is [bool]) { if ($Value) { return 'true' } else { return 'false' } }
    if ($Value -is [System.Collections.IDictionary]) {
        [string[]]$keys = @($Value.Keys | ForEach-Object { [string]$_ })
        [Array]::Sort($keys, [StringComparer]::Ordinal)
        $members = foreach ($key in $keys) {
            (ConvertTo-PilotCanonicalJson $key) + ':' + (ConvertTo-PilotCanonicalJson $Value[$key])
        }
        return '{' + ($members -join ',') + '}'
    }
    if ($Value -is [System.Collections.IEnumerable]) {
        $items = foreach ($item in $Value) { ConvertTo-PilotCanonicalJson $item }
        return '[' + ($items -join ',') + ']'
    }
    if ($Value -is [System.Management.Automation.PSCustomObject]) {
        $dictionary = [ordered]@{}
        foreach ($property in $Value.PSObject.Properties) { $dictionary[$property.Name] = $property.Value }
        return ConvertTo-PilotCanonicalJson $dictionary
    }
    if ($Value -is [byte] -or $Value -is [sbyte] -or $Value -is [int16] -or $Value -is [uint16] -or
        $Value -is [int32] -or $Value -is [uint32] -or $Value -is [int64] -or $Value -is [uint64]) {
        return [Convert]::ToString($Value, [Globalization.CultureInfo]::InvariantCulture)
    }
    throw "PILOT_CANONICAL_JSON_UNSUPPORTED_TYPE: $($Value.GetType().FullName)"
}

function Get-PilotCanonicalDigest {
    param([System.Collections.IDictionary]$Value)

    $canonical = ConvertTo-PilotCanonicalJson $Value
    $bytes = [Text.Encoding]::UTF8.GetBytes($canonical)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    return 'sha256:' + [Convert]::ToHexString($hash).ToLowerInvariant()
}

function Write-PilotJson {
    param([string]$Path, $Value)
    $json = ConvertTo-Json -InputObject $Value -Depth 20
    [IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}

function Invoke-PilotLedgerJson {
    param([string]$Executable, [string[]]$Arguments, [int[]]$ExpectedExitCodes = @(0))
    $records = @(& $Executable @Arguments 2>&1)
    $exit = $LASTEXITCODE
    $text = ($records | Out-String).Trim()
    if ($ExpectedExitCodes -notcontains $exit) {
        throw "PILOT_LEDGER_COMMAND_FAILED: exit=$exit args=$($Arguments -join ' ') output=$text"
    }
    $stdoutText = @($records | Where-Object {
        $_ -isnot [System.Management.Automation.ErrorRecord]
    }) | Out-String
    $stdoutText = $stdoutText.Trim()
    if ([string]::IsNullOrWhiteSpace($stdoutText)) { return $null }
    return $stdoutText | ConvertFrom-Json
}

function Get-PilotTextDigest {
    param([string]$Text)

    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    return 'sha256:' + [Convert]::ToHexString($hash).ToLowerInvariant()
}

if ($Stage -eq 6) {
    $resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
    $resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
    $resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)
    if (-not (Test-Path -LiteralPath $resolvedPilotRoot -PathType Container)) {
        throw "PILOT_STAGE_1_REQUIRED: $resolvedPilotRoot"
    }

    $evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
    $fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
    $stageOneManifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
    $stageTwoManifestPath = Join-Path $evidenceDirectory 'stage-2-governed-model.json'
    $stageThreeManifestPath = Join-Path $evidenceDirectory 'stage-3-high-level-continuity.json'
    $stageFourManifestPath = Join-Path $evidenceDirectory 'stage-4-evidence-lifecycle.json'
    $stageFiveManifestPath = Join-Path $evidenceDirectory 'stage-5-transactional-safety.json'
    $stageSixManifestPath = Join-Path $evidenceDirectory 'stage-6-projections-privacy-recovery.json'
    $locatorPath = Join-Path $resolvedPilotRoot 'pilot-locator.json'
    $installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
    $ledger = Join-Path $installation 'project-ledger.exe'
    $packageManifestPath = Join-Path $installation 'package.json'
    foreach ($requiredPath in @(
        $stageOneManifestPath,
        $stageTwoManifestPath,
        $stageThreeManifestPath,
        $stageFourManifestPath,
        $stageFiveManifestPath,
        $locatorPath,
        $ledger,
        $packageManifestPath
    )) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_5_EVIDENCE_MISSING: $requiredPath"
        }
    }
    if (Test-Path -LiteralPath $stageSixManifestPath) {
        throw "PILOT_STAGE_ALREADY_EXECUTED: $stageSixManifestPath"
    }

    $stageSixRoot = Join-Path $resolvedPilotRoot 'stage-6-runtime'
    $stageSixRootExistedBefore = Test-Path -LiteralPath $stageSixRoot

    $actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
    if ($actualZipHash -ne $ExpectedZipSha256.ToLowerInvariant()) {
        throw "PILOT_ZIP_HASH_MISMATCH: expected $($ExpectedZipSha256.ToLowerInvariant()); actual $actualZipHash"
    }
    $stageOneManifest = Get-Content -Raw -LiteralPath $stageOneManifestPath | ConvertFrom-Json
    $stageTwoManifest = Get-Content -Raw -LiteralPath $stageTwoManifestPath | ConvertFrom-Json
    $stageThreeManifest = Get-Content -Raw -LiteralPath $stageThreeManifestPath | ConvertFrom-Json
    $stageFourManifest = Get-Content -Raw -LiteralPath $stageFourManifestPath | ConvertFrom-Json
    $stageFiveManifest = Get-Content -Raw -LiteralPath $stageFiveManifestPath | ConvertFrom-Json
    $packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
    if ($stageOneManifest.status -ne 'PREFLIGHT_PASS' -or
        $stageTwoManifest.status -ne 'GOVERNED_MODEL_PASS' -or
        $stageThreeManifest.status -ne 'HIGH_LEVEL_CONTINUITY_PASS' -or
        $stageFourManifest.status -ne 'EVIDENCE_LIFECYCLE_PASS' -or
        $stageFiveManifest.status -ne 'TRANSACTIONAL_SAFETY_PASS' -or
        $stageOneManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageFiveManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageOneManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $stageFiveManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $packageManifest.source_commit -ne $ExpectedSourceCommit) {
        throw 'PILOT_STAGE_5_IDENTITY_MISMATCH'
    }

    $locator = Get-Content -Raw -LiteralPath $locatorPath | ConvertFrom-Json
    $database = [string]$locator.database
    $projectRef = [string]$locator.project_ref
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'PILOT_DATABASE_MISSING'
    }
    $initialState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $stageFiveFinalVersion = [int64]$stageFiveManifest.lifecycle.final_state_version
    $stageSixResumed = $false
    $stageSixProgressReconciliation = $null
    if ([int64]$initialState.state_version -eq $stageFiveFinalVersion) {
        if ($initialState.state_digest -ne $stageFiveManifest.lifecycle.final_state_digest) {
            throw 'PILOT_STAGE_6_INITIAL_STATE_DIGEST_MISMATCH'
        }
    } elseif ([int64]$initialState.state_version -eq ($stageFiveFinalVersion + 1)) {
        $stageSixProgressReconciliation = Invoke-PilotLedgerJson $ledger @(
            'reconcile', '--database', $database, '--project-ref', $projectRef,
            '--command-type', 'RECORD_WORK_PROGRESS', '--actor-id', 'agent:opa-stage6',
            '--trust-domain', 'TRUSTED_FOR_RECORDING', '--idempotency-key', 'opa:stage6:progress', '--json'
        )
        if ($stageSixProgressReconciliation.outcome -ne 'RECONCILED' -or
            $stageSixProgressReconciliation.result.status -ne 'ACCEPTED' -or
            [int64]$stageSixProgressReconciliation.result.previous_state_version -ne $stageFiveFinalVersion -or
            [int64]$stageSixProgressReconciliation.result.new_state_version -ne [int64]$initialState.state_version -or
            @($stageSixProgressReconciliation.result.emitted_event_refs) -notcontains $initialState.last_material_event_ref) {
            throw 'PILOT_STAGE_6_PARTIAL_PROGRESS_RECONCILIATION_FAILED'
        }
        $stageSixResumed = $true
    } else {
        throw 'PILOT_STAGE_6_INITIAL_STATE_VERSION_MISMATCH'
    }
    if ($initialState.project.lifecycle_status -ne 'BLOCKED' -or
        $initialState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($initialState.completed_phase_refs).Count -ne 0 -or
        $initialState.preferred_next_action_ref -ne 'action:opa-stage6-recovery' -or
        @($initialState.open_blocker_refs) -notcontains 'blocker:opa-stage6-recovery' -or
        $initialState.checkpoint_ref -ne $stageFiveManifest.boundary.checkpoint_ref) {
        throw 'PILOT_STAGE_6_INITIAL_STATE_MISMATCH'
    }

    if ($stageSixRootExistedBefore) {
        if (-not $stageSixResumed) {
            throw "PILOT_STAGE_6_DESTINATION_EXISTS: $stageSixRoot"
        }
        $resumeIndex = 1
        do {
            $stageSixRoot = Join-Path $resolvedPilotRoot ("stage-6-runtime-resume-$resumeIndex")
            $resumeIndex++
        } while (Test-Path -LiteralPath $stageSixRoot)
    }

    $null = New-Item -ItemType Directory -Path $stageSixRoot
    $projectionRoot = Join-Path $stageSixRoot 'projections'
    $portableRoot = Join-Path $stageSixRoot 'portable'
    $backupRoot = Join-Path $stageSixRoot 'backup'
    foreach ($directory in @($projectionRoot, $portableRoot, $backupRoot)) {
        $null = New-Item -ItemType Directory -Path $directory
    }

    function Get-StageSixDirectoryDigest {
        param([string]$Path)

        $entries = foreach ($file in @(Get-ChildItem -LiteralPath $Path -Recurse -File | Sort-Object FullName)) {
            $relative = [IO.Path]::GetRelativePath($Path, $file.FullName).Replace('\', '/')
            $digest = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash.ToLowerInvariant()
            "$relative=$digest"
        }
        return Get-PilotTextDigest ($entries -join "`n")
    }

    $observedAt = [DateTimeOffset]::UtcNow.ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $script:stageSixStateVersion = [int64]$initialState.state_version
    $script:stageSixMutations = @()

    function Invoke-StageSixCommand {
        param(
            [string]$Name,
            [string]$CommandType,
            [string]$ActorId,
            [bool]$AsHuman,
            [string[]]$CommandArguments,
            [int]$ExpectedExitCode,
            [int64]$ExpectedDelta
        )

        $before = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $expectedVersion = [int64]$before.state_version
        $delimiterIndex = [Array]::IndexOf([string[]]$CommandArguments, '--')
        $operationArguments = if ($delimiterIndex -ge 0) {
            @($CommandArguments[0..($delimiterIndex - 1)])
        } else {
            @($CommandArguments)
        }
        $childArguments = if ($delimiterIndex -ge 0 -and $delimiterIndex -lt $CommandArguments.Count - 1) {
            @($CommandArguments[($delimiterIndex + 1)..($CommandArguments.Count - 1)])
        } else {
            @()
        }
        $arguments = @($operationArguments) + @(
            '--database', $database,
            '--project-ref', $projectRef,
            '--expected-version', [string]$expectedVersion,
            '--idempotency-key', ('opa:stage6:' + $Name),
            '--actor-id', $ActorId,
            '--observed-at', $observedAt
        )
        if ($AsHuman) { $arguments += '--as-human' }
        $arguments += '--json'
        if ($delimiterIndex -ge 0) {
            $arguments += '--'
            $arguments += $childArguments
        }
        $result = Invoke-PilotLedgerJson `
            -Executable $ledger `
            -Arguments $arguments `
            -ExpectedExitCodes @($ExpectedExitCode)
        $after = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $delta = [int64]$after.state_version - [int64]$before.state_version
        if ($delta -ne $ExpectedDelta) {
            throw "PILOT_STAGE_6_MUTATION_DELTA_INVALID: $Name expected=$ExpectedDelta actual=$delta"
        }
        if ($ExpectedDelta -eq 0 -and $after.state_digest -ne $before.state_digest) {
            throw "PILOT_STAGE_6_ZERO_DELTA_DIGEST_CHANGED: $Name"
        }
        $script:stageSixStateVersion = [int64]$after.state_version
        $script:stageSixMutations += [ordered]@{
            name = $Name
            command_type = $CommandType
            actor_id = $ActorId
            as_human = $AsHuman
            expected_exit_code = $ExpectedExitCode
            observed_exit_code = $ExpectedExitCode
            expected_state_version = $expectedVersion
            resulting_state_version = [int64]$after.state_version
            state_version_delta = $delta
            idempotency_key = 'opa:stage6:' + $Name
        }
        return [ordered]@{ result = $result; before = $before; state = $after; delta = $delta }
    }

    $canaries = @(
        'token=OPA_STAGE6_PROCESS_CANARY_8173',
        'password=OPA_STAGE6_PROCESS_CANARY_9261'
    )
    $canaryPolicyPath = Join-Path $fixtureDirectory 'stage-6-canary-evidence-policy.json'
    Write-PilotJson $canaryPolicyPath ([ordered]@{
        allowed_programs = @($ledger)
        redactions = @()
        max_summary_bytes = 4096
        process = [ordered]@{
            timeout_ms = 10000
            environment = [ordered]@{}
        }
    })

    $progressExpectedVersion = $stageFiveFinalVersion
    $progressObservedAt = $observedAt
    $progressArguments = @(
        'record', 'progress',
        '--summary', 'Stage 6 validates projections, privacy, portable recovery, and SQLite backup restore.',
        '--phase', 'phase:operational-acceptance',
        '--record-id', 'work:opa-stage6-recovery'
    )
    if ($stageSixResumed) {
        $progressObservedAt = ([DateTimeOffset]$stageSixProgressReconciliation.result.processed_at).ToUniversalTime().ToString(
            "yyyy-MM-dd'T'HH:mm:ss'Z'",
            [Globalization.CultureInfo]::InvariantCulture
        )
        $progress = [ordered]@{
            result = $stageSixProgressReconciliation.result
            before = $null
            state = $initialState
            delta = 1
        }
        $script:stageSixMutations += [ordered]@{
            name = 'progress'
            command_type = 'RECORD_WORK_PROGRESS'
            actor_id = 'agent:opa-stage6'
            as_human = $false
            expected_exit_code = 0
            observed_exit_code = 0
            expected_state_version = $stageFiveFinalVersion
            resulting_state_version = [int64]$initialState.state_version
            state_version_delta = 1
            idempotency_key = 'opa:stage6:progress'
            recovered_by_reconciliation = $true
        }
    } else {
        $progress = Invoke-StageSixCommand `
            'progress' 'RECORD_WORK_PROGRESS' 'agent:opa-stage6' $false `
            $progressArguments 0 1
    }
    if ($progress.result.status -ne 'ACCEPTED') {
        throw 'PILOT_STAGE_6_PROGRESS_NOT_ACCEPTED'
    }

    $sourceState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $sourceResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    $sourceDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseCatalogPath = Join-Path $fixtureDirectory 'stage-6-phase-catalog.json'
    $phaseStatuses = @(Invoke-PilotLedgerJson $ledger @(
        'phase', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $operationalPhaseStatus = @($phaseStatuses | Where-Object {
        $_.phase_ref -eq 'phase:operational-acceptance'
    })
    if ($operationalPhaseStatus.Count -ne 1 -or $operationalPhaseStatus[0].status -ne 'ACTIVE') {
        throw 'PILOT_STAGE_6_PHASE_CATALOG_INVARIANT_FAILED'
    }
    $phaseCatalog = @([ordered]@{
        schema_version = '0.2.0'
        phase_id = 'phase:operational-acceptance'
        title = 'Operational acceptance'
        ordinal = 1
        kind = 'AUDIT'
        status = $operationalPhaseStatus[0].status
        required_gate_refs = @(1..8 | ForEach-Object { 'gate:opa-{0:D2}' -f $_ })
        optional_gate_refs = @('gate:opa-waiver-exercise')
        started_at = $null
        completed_at = $null
        reopened_at = $null
    })
    Write-PilotJson $phaseCatalogPath $phaseCatalog

    $beforeProjection = $sourceState
    $authoritativeRenderPath = Join-Path $projectionRoot 'authoritative-shareable.md'
    $projectionCopyPath = Join-Path $projectionRoot 'divergence-copy.md'
    $null = Invoke-PilotLedgerJson $ledger @(
        'render', '--database', $database, '--project-ref', $projectRef,
        '--output', $authoritativeRenderPath, '--privacy', 'shareable', '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'verify', '--database', $database, '--project-ref', $projectRef,
        '--artifact', $authoritativeRenderPath, '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'render', '--database', $database, '--project-ref', $projectRef,
        '--output', $projectionCopyPath, '--privacy', 'shareable', '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'verify', '--database', $database, '--project-ref', $projectRef,
        '--artifact', $projectionCopyPath, '--json'
    )
    [IO.File]::WriteAllText(
        $projectionCopyPath,
        "# Deliberate divergence`n`nThis copy is not authoritative and must be regenerated.`n",
        [Text.UTF8Encoding]::new($false)
    )
    $divergenceResult = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'verify', '--database', $database, '--project-ref', $projectRef,
            '--artifact', $projectionCopyPath, '--json'
        ) `
        -ExpectedExitCodes @(15)
    $authoritativeRenderDigestBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $authoritativeRenderPath).Hash.ToLowerInvariant()
    $authoritativeRecordPath = $authoritativeRenderPath + '.record.json'
    $authoritativeRecordDigestBefore = (Get-FileHash -Algorithm SHA256 -LiteralPath $authoritativeRecordPath).Hash.ToLowerInvariant()
    $existingRenderResult = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'render', '--database', $database, '--project-ref', $projectRef,
            '--output', $authoritativeRenderPath, '--privacy', 'shareable', '--json'
        ) `
        -ExpectedExitCodes @(2)
    $authoritativeRenderUnchanged =
        $authoritativeRenderDigestBefore -eq (Get-FileHash -Algorithm SHA256 -LiteralPath $authoritativeRenderPath).Hash.ToLowerInvariant() -and
        $authoritativeRecordDigestBefore -eq (Get-FileHash -Algorithm SHA256 -LiteralPath $authoritativeRecordPath).Hash.ToLowerInvariant()
    $null = Invoke-PilotLedgerJson $ledger @(
        'render', '--database', $database, '--project-ref', $projectRef,
        '--output', $projectionCopyPath, '--privacy', 'shareable', '--regenerate', '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'verify', '--database', $database, '--project-ref', $projectRef,
        '--artifact', $projectionCopyPath, '--json'
    )
    $regeneratedMatches =
        [Convert]::ToHexString([IO.File]::ReadAllBytes($authoritativeRenderPath)) -eq
            [Convert]::ToHexString([IO.File]::ReadAllBytes($projectionCopyPath))
    $afterProjection = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $projectionStateDelta = [int64]$afterProjection.state_version - [int64]$beforeProjection.state_version
    if ($divergenceResult.error.code -ne 'GENERATED_ARTIFACT_DIVERGED' -or
        $existingRenderResult.error.code -ne 'INVALID_INPUT' -or
        -not $authoritativeRenderUnchanged -or -not $regeneratedMatches -or
        $projectionStateDelta -ne 0 -or $afterProjection.state_digest -ne $beforeProjection.state_digest) {
        throw 'PILOT_STAGE_6_PROJECTION_INVARIANT_FAILED'
    }

    $portableSourceDataRoot = Join-Path $portableRoot 'source-data'
    $prospectivePortableDatabase = Join-Path $portableSourceDataRoot (
        'project-ledger/projects/00000000-0000-0000-0000-000000000000/ledger.db'
    )
    if ($prospectivePortableDatabase.Length -ge 240) {
        $scratchSuffix = $projectRef.Substring([Math]::Max(0, $projectRef.Length - 8))
        $portableSourceDataRoot = Join-Path $resolvedRepository (
            ".tooling/g-ledger-operational-pilot/s6/$scratchSuffix"
        )
        [System.IO.Directory]::CreateDirectory($portableSourceDataRoot) | Out-Null
    }
    $portableInit = Invoke-PilotLedgerJson $ledger @(
        'init', '--title', 'Stage 6 portable roundtrip fixture',
        '--summary', 'Disposable privacy-clean portable recovery fixture',
        '--timezone', 'UTC', '--data-root', $portableSourceDataRoot, '--json'
    )
    $portableInitLines = $portableInit.output -split "`n"
    $portableProjectRef = ($portableInitLines | Where-Object { $_ -like 'project_ref=*' }) -replace '^project_ref=', ''
    $portableSourceDatabase = ($portableInitLines | Where-Object { $_ -like 'database=*' }) -replace '^database=', ''
    if ($portableProjectRef -notmatch '^project:[0-9a-f-]{36}$' -or
        -not (Test-Path -LiteralPath $portableSourceDatabase -PathType Leaf)) {
        throw 'PILOT_STAGE_6_PORTABLE_FIXTURE_INIT_INVALID'
    }
    $portableProgress = Invoke-PilotLedgerJson $ledger @(
        'record', 'progress', '--summary', 'Portable fixture has a non-trivial replayable history.',
        '--record-id', 'work:stage6-portable-fixture',
        '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef,
        '--expected-version', '1', '--idempotency-key', 'opa:stage6:portable-fixture-progress',
        '--actor-id', 'agent:opa-stage6', '--observed-at', $observedAt, '--json'
    )
    if ($portableProgress.status -ne 'ACCEPTED') {
        throw 'PILOT_STAGE_6_PORTABLE_FIXTURE_PROGRESS_INVALID'
    }
    $portableSourceState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $portableSourceResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef, '--format', 'json'
    )
    $portableSourceDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef, '--json'
    )

    $portableExportPath = Join-Path $portableRoot 'export-original'
    $beforePortableExport = $portableSourceState
    $portableManifest = Invoke-PilotLedgerJson $ledger @(
        'export', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef,
        '--format', 'portable', '--output', $portableExportPath, '--json'
    )
    $afterPortableExport = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $portableExportDelta = [int64]$afterPortableExport.state_version - [int64]$beforePortableExport.state_version
    $portableDatabase = Join-Path $portableRoot 'imported-roundtrip.db'
    $portableImport = Invoke-PilotLedgerJson $ledger @(
        'import', '--file', $portableExportPath, '--database', $portableDatabase,
        '--project-ref', $portableProjectRef, '--json'
    )
    $importedState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $importedResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--format', 'json'
    )
    $importedDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $beforeIdenticalImport = $importedState
    $identicalImport = Invoke-PilotLedgerJson $ledger @(
        'import', '--file', $portableExportPath, '--database', $portableDatabase,
        '--project-ref', $portableProjectRef, '--json'
    )
    $afterIdenticalImport = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $identicalImportDelta = [int64]$afterIdenticalImport.state_version - [int64]$beforeIdenticalImport.state_version

    $portableDigestBeforeExisting = Get-StageSixDirectoryDigest $portableExportPath
    $existingPortableExport = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'export', '--database', $portableSourceDatabase, '--project-ref', $portableProjectRef,
            '--format', 'portable', '--output', $portableExportPath, '--json'
        ) `
        -ExpectedExitCodes @(2)
    $portableExistingUnchanged = $portableDigestBeforeExisting -eq (Get-StageSixDirectoryDigest $portableExportPath)

    $corruptPortablePath = Join-Path $portableRoot 'export-corrupt-checksum'
    Copy-Item -LiteralPath $portableExportPath -Destination $corruptPortablePath -Recurse
    [IO.File]::AppendAllText(
        (Join-Path $corruptPortablePath 'events.jsonl'),
        "{}`n",
        [Text.UTF8Encoding]::new($false)
    )
    $corruptPortableDatabase = Join-Path $portableRoot 'corrupt-must-not-exist.db'
    $corruptPortableImport = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'import', '--file', $corruptPortablePath, '--database', $corruptPortableDatabase,
            '--project-ref', $portableProjectRef, '--json'
        ) `
        -ExpectedExitCodes @(15)

    $branchSourceDatabase = Join-Path $portableRoot 'branch-source.db'
    $null = Invoke-PilotLedgerJson $ledger @(
        'import', '--file', $portableExportPath, '--database', $branchSourceDatabase,
        '--project-ref', $portableProjectRef, '--json'
    )
    $branchVersion = [int64]$portableSourceState.state_version
    $null = Invoke-PilotLedgerJson $ledger @(
        'record', 'note', '--kind', 'LIMITATION', '--text', 'Stage 6 source branch divergence fixture.',
        '--database', $branchSourceDatabase, '--project-ref', $portableProjectRef,
        '--expected-version', [string]$branchVersion, '--idempotency-key', 'opa:stage6:portable-source-branch',
        '--actor-id', 'agent:opa-stage6', '--observed-at', $observedAt, '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'record', 'note', '--kind', 'LIMITATION', '--text', 'Stage 6 target branch divergence fixture.',
        '--database', $portableDatabase, '--project-ref', $portableProjectRef,
        '--expected-version', [string]$branchVersion, '--idempotency-key', 'opa:stage6:portable-target-branch',
        '--actor-id', 'agent:opa-stage6', '--observed-at', $observedAt, '--json'
    )
    $advancedPortablePath = Join-Path $portableRoot 'export-divergent-source'
    $null = Invoke-PilotLedgerJson $ledger @(
        'export', '--database', $branchSourceDatabase, '--project-ref', $portableProjectRef,
        '--format', 'portable', '--output', $advancedPortablePath, '--json'
    )
    $beforeDivergentImport = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $beforeDivergentDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $divergentImport = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'import', '--file', $advancedPortablePath, '--database', $portableDatabase,
            '--project-ref', $portableProjectRef, '--json'
        ) `
        -ExpectedExitCodes @(15)
    $afterDivergentImport = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $afterDivergentDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $portableDatabase, '--project-ref', $portableProjectRef, '--json'
    )
    $divergentTargetUnchanged =
        [int64]$beforeDivergentImport.state_version -eq [int64]$afterDivergentImport.state_version -and
        $beforeDivergentImport.state_digest -eq $afterDivergentImport.state_digest -and
        $beforeDivergentDoctor.head_digest -eq $afterDivergentDoctor.head_digest

    $portableStateMatches =
        [int64]$importedState.state_version -eq [int64]$portableSourceState.state_version -and
        $importedState.state_digest -eq $portableSourceState.state_digest
    $portableHeadMatches = $importedDoctor.head_digest -eq $portableSourceDoctor.head_digest
    $portableResumeMatches = $importedResume.digest -eq $portableSourceResume.digest
    if ($portableExportDelta -ne 0 -or
        $portableManifest.project_ref -ne $portableProjectRef -or
        $portableImport.disposition -ne 'NEW_PROJECT' -or
        $portableImport.restores_command_results -ne $false -or
        -not $portableStateMatches -or -not $portableHeadMatches -or -not $portableResumeMatches -or
        $identicalImport.disposition -ne 'IDENTICAL_CHAIN' -or $identicalImportDelta -ne 0 -or
        $existingPortableExport.error.code -ne 'INVALID_INPUT' -or -not $portableExistingUnchanged -or
        $corruptPortableImport.error.code -ne 'PORTABLE_INTEGRITY' -or
        (Test-Path -LiteralPath $corruptPortableDatabase) -or
        $divergentImport.error.code -ne 'PORTABLE_INTEGRITY' -or -not $divergentTargetUnchanged) {
        throw 'PILOT_STAGE_6_PORTABLE_INVARIANT_FAILED'
    }

    $backupPath = Join-Path $backupRoot 'sqlite-full-fidelity'
    $beforeBackup = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $backupManifest = Invoke-PilotLedgerJson $ledger @(
        'backup', '--database', $database, '--project-ref', $projectRef,
        '--output', $backupPath, '--json'
    )
    $afterBackup = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $backupDelta = [int64]$afterBackup.state_version - [int64]$beforeBackup.state_version
    $backupDigestBeforeExisting = Get-StageSixDirectoryDigest $backupPath
    $existingBackup = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'backup', '--database', $database, '--project-ref', $projectRef,
            '--output', $backupPath, '--json'
        ) `
        -ExpectedExitCodes @(2)
    $backupExistingUnchanged = $backupDigestBeforeExisting -eq (Get-StageSixDirectoryDigest $backupPath)

    $restoredDatabase = Join-Path $backupRoot 'restored-full-fidelity.db'
    $null = Invoke-PilotLedgerJson $ledger @(
        'restore', '--file', $backupPath, '--database', $restoredDatabase, '--json'
    )
    $restoredState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $restoredDatabase, '--project-ref', $projectRef, '--json'
    )
    $restoredDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $restoredDatabase, '--project-ref', $projectRef, '--json'
    )
    $restoredReconcile = Invoke-PilotLedgerJson $ledger @(
        'reconcile', '--database', $restoredDatabase, '--project-ref', $projectRef,
        '--command-type', 'RECORD_WORK_PROGRESS', '--actor-id', 'agent:opa-stage6',
        '--trust-domain', 'TRUSTED_FOR_RECORDING', '--idempotency-key', 'opa:stage6:progress', '--json'
    )
    $restoredBeforeReplay = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $restoredDatabase, '--project-ref', $projectRef, '--json'
    )
    $restoredReplay = Invoke-PilotLedgerJson $ledger @(
        $progressArguments + @(
            '--database', $restoredDatabase, '--project-ref', $projectRef,
            '--expected-version', [string]$progressExpectedVersion,
            '--idempotency-key', 'opa:stage6:progress', '--actor-id', 'agent:opa-stage6',
            '--observed-at', $progressObservedAt, '--json'
        )
    )
    $restoredAfterReplay = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $restoredDatabase, '--project-ref', $projectRef, '--json'
    )
    $restoredReplayDelta = [int64]$restoredAfterReplay.state_version - [int64]$restoredBeforeReplay.state_version

    $restoredDigestBeforeExisting = (Get-FileHash -Algorithm SHA256 -LiteralPath $restoredDatabase).Hash.ToLowerInvariant()
    $existingRestore = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @('restore', '--file', $backupPath, '--database', $restoredDatabase, '--json') `
        -ExpectedExitCodes @(2)
    $restoreExistingUnchanged =
        $restoredDigestBeforeExisting -eq (Get-FileHash -Algorithm SHA256 -LiteralPath $restoredDatabase).Hash.ToLowerInvariant()

    $corruptBackupPath = Join-Path $backupRoot 'sqlite-corrupt-checksum'
    Copy-Item -LiteralPath $backupPath -Destination $corruptBackupPath -Recurse
    $corruptBackupManifestPath = Join-Path $corruptBackupPath 'manifest.json'
    $corruptBackupManifest = Get-Content -Raw -LiteralPath $corruptBackupManifestPath | ConvertFrom-Json
    $corruptBackupManifest.database_digest = 'sha256:' + ('0' * 64)
    Write-PilotJson $corruptBackupManifestPath $corruptBackupManifest
    $corruptRestoreDatabase = Join-Path $backupRoot 'corrupt-must-not-exist.db'
    $corruptRestore = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @('restore', '--file', $corruptBackupPath, '--database', $corruptRestoreDatabase, '--json') `
        -ExpectedExitCodes @(15)

    $restoreStateMatches =
        [int64]$restoredState.state_version -eq [int64]$sourceState.state_version -and
        $restoredState.state_digest -eq $sourceState.state_digest
    $restoreHeadMatches = $restoredDoctor.head_digest -eq $sourceDoctor.head_digest
    if ($backupManifest.format -ne 'project-ledger-sqlite-backup/1' -or
        $backupDelta -ne 0 -or
        $existingBackup.error.code -ne 'INVALID_INPUT' -or -not $backupExistingUnchanged -or
        -not $restoreStateMatches -or -not $restoreHeadMatches -or
        $restoredReconcile.outcome -ne 'RECONCILED' -or $restoredReconcile.result.status -ne 'ACCEPTED' -or
        $restoredReplay.status -ne 'DUPLICATE_REPLAY' -or $restoredReplayDelta -ne 0 -or
        $existingRestore.error.code -ne 'INVALID_INPUT' -or -not $restoreExistingUnchanged -or
        $corruptRestore.error.code -ne 'BACKUP_INTEGRITY' -or
        (Test-Path -LiteralPath $corruptRestoreDatabase)) {
        throw 'PILOT_STAGE_6_BACKUP_INVARIANT_FAILED'
    }

    $canaryEvidence = Invoke-StageSixCommand `
        'canary-evidence' 'RECORD_VERIFICATION_RESULT' 'agent:opa-stage6' $false `
        @(
            'evidence', 'run', '--policy', $canaryPolicyPath, '--cwd', $installation,
            '--kind', 'CUSTOM_DECLARATIVE', '--gate', 'gate:opa-06',
            '--', $ledger, ('--' + $canaries[0]), ('--' + $canaries[1])
        ) `
        3 1
    $canaryIdentity = ConvertTo-PilotCanonicalJson @(
        $projectRef,
        'agent:opa-stage6',
        'TRUSTED_FOR_RECORDING',
        'opa:stage6:canary-evidence'
    )
    $canaryIdentityDigest = Get-PilotTextDigest $canaryIdentity
    $canaryEvidenceRef = 'evidence:' + $canaryIdentityDigest.Substring(7)
    $canaryEvidenceView = Invoke-PilotLedgerJson $ledger @(
        'evidence', 'show', $canaryEvidenceRef,
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($canaryEvidence.result.status -ne 'ACCEPTED' -or
        $canaryEvidenceView.privacy.raw_output_stored -ne $false) {
        throw 'PILOT_STAGE_6_CANARY_EVIDENCE_INVARIANT_FAILED'
    }

    $postCanaryRenderPath = Join-Path $projectionRoot 'post-canary-shareable.md'
    $null = Invoke-PilotLedgerJson $ledger @(
        'render', '--database', $database, '--project-ref', $projectRef,
        '--output', $postCanaryRenderPath, '--privacy', 'shareable', '--json'
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'verify', '--database', $database, '--project-ref', $projectRef,
        '--artifact', $postCanaryRenderPath, '--json'
    )
    $postCanaryRecordPath = $postCanaryRenderPath + '.record.json'
    $postCanaryPortablePath = Join-Path $portableRoot 'post-canary-must-not-exist'
    $postCanaryPortable = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'export', '--database', $database, '--project-ref', $projectRef,
            '--format', 'portable', '--output', $postCanaryPortablePath,
            '--phase-catalog', $phaseCatalogPath, '--json'
        ) `
        -ExpectedExitCodes @(13)
    if ($postCanaryPortable.error.code -ne 'PRIVACY_REJECTED' -or
        (Test-Path -LiteralPath $postCanaryPortablePath)) {
        throw 'PILOT_STAGE_6_PRIVATE_PORTABLE_INVARIANT_FAILED'
    }

    $null = Invoke-StageSixCommand `
        'blocker-stage6-resolve' 'RESOLVE_BLOCKER' 'human:owner' $true `
        @(
            'blocker', 'resolve', 'blocker:opa-stage6-recovery',
            '--summary', 'Projection, privacy, portable, backup, restore, and integrity safeguards passed Stage 6.'
        ) `
        0 3
    $null = Invoke-StageSixCommand `
        'next-stage7-set' 'SET_NEXT_ACTION' 'human:owner' $true `
        @(
            'next', 'set', '--title', 'Request authorization for Stage 7 blind handoff and bounded integrations',
            '--summary', 'Stage 7 remains outside the current authorization.',
            '--phase', 'phase:operational-acceptance', '--classification', 'ELIGIBLE',
            '--record-id', 'action:opa-stage-07-integrations'
        ) `
        0 1
    $null = Invoke-StageSixCommand `
        'blocker-stage7-open' 'OPEN_BLOCKER' 'human:owner' $true `
        @(
            'blocker', 'open', '--title', 'Stage 7 requires separate human authorization',
            '--summary', 'Blind Codex handoff, bounded local integrations, and fixed GSP tests are not authorized by Stage 6.',
            '--severity', 'MEDIUM', '--record-id', 'blocker:opa-stage7-integrations'
        ) `
        0 2
    $null = Invoke-StageSixCommand `
        'checkpoint-final' 'CREATE_CONTINUITY_CHECKPOINT' 'agent:opa-stage6' $false `
        @('checkpoint', 'create', '--reason', 'Stage 6 projections privacy and recovery handoff') `
        0 1

    $finalState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $finalResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    $finalDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $finalVerify = Invoke-PilotLedgerJson $ledger @(
        'verify', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $finalAudit = Invoke-PilotLedgerJson $ledger @(
        'audit', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseList = @(Invoke-PilotLedgerJson $ledger @(
        'phase', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $activePhase = @($phaseList | Where-Object { $_.phase_ref -eq 'phase:operational-acceptance' })
    if ($finalResume.checkpoint_freshness -ne 'CURRENT' -or
        $finalState.project.lifecycle_status -ne 'BLOCKED' -or
        $finalState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($finalState.completed_phase_refs).Count -ne 0 -or
        $finalState.preferred_next_action_ref -ne 'action:opa-stage-07-integrations' -or
        @($finalState.open_blocker_refs) -notcontains 'blocker:opa-stage7-integrations' -or
        @($finalState.open_blocker_refs) -contains 'blocker:opa-stage6-recovery' -or
        $activePhase.Count -ne 1 -or $activePhase[0].status -ne 'ACTIVE' -or
        $finalDoctor.integrity -ne 'ok' -or $finalVerify.integrity -ne 'ok' -or $finalAudit.integrity -ne 'ok' -or
        @($finalVerify.findings).Count -ne 0 -or @($finalAudit.findings).Count -ne 0) {
        $finalInvariantDetails = [ordered]@{
            checkpoint_freshness = $finalResume.checkpoint_freshness
            lifecycle_status = $finalState.project.lifecycle_status
            primary_active_phase_ref = $finalState.primary_active_phase_ref
            completed_phase_count = @($finalState.completed_phase_refs).Count
            preferred_next_action_ref = $finalState.preferred_next_action_ref
            open_blocker_refs = @($finalState.open_blocker_refs)
            active_phase_count = $activePhase.Count
            active_phase_status = if ($activePhase.Count -eq 1) { $activePhase[0].status } else { $null }
            doctor_integrity = $finalDoctor.integrity
            verify_integrity = $finalVerify.integrity
            audit_integrity = $finalAudit.integrity
            verify_finding_count = @($finalVerify.findings).Count
            audit_finding_count = @($finalAudit.findings).Count
        }
        throw ('PILOT_STAGE_6_FINAL_STATE_MISMATCH: ' + (ConvertTo-PilotCanonicalJson $finalInvariantDetails))
    }

    $supportedSurfaceTexts = @(
        ($sourceResume | ConvertTo-Json -Depth 20 -Compress),
        ($finalResume | ConvertTo-Json -Depth 20 -Compress),
        [IO.File]::ReadAllText($authoritativeRenderPath),
        [IO.File]::ReadAllText($authoritativeRecordPath),
        [IO.File]::ReadAllText($postCanaryRenderPath),
        [IO.File]::ReadAllText($postCanaryRecordPath)
    )
    foreach ($file in @(Get-ChildItem -LiteralPath $portableExportPath -Recurse -File)) {
        $supportedSurfaceTexts += [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($file.FullName))
    }
    $canaryMatchCount = 0
    foreach ($canary in $canaries) {
        foreach ($surface in $supportedSurfaceTexts) {
            if ($surface.Contains($canary, [StringComparison]::Ordinal)) {
                $canaryMatchCount++
            }
        }
    }
    if ($canaryMatchCount -ne 0) {
        throw "PILOT_STAGE_6_CANARY_LEAK: matches=$canaryMatchCount"
    }

    $manifest = [ordered]@{
        profile = 'project-ledger-operational-pilot-stage/1'
        stage = 6
        status = 'RECOVERY_PRIVACY_PASS'
        proof_class = 'LIVE_PILOT'
        project_ref = $projectRef
        subject = [ordered]@{
            zip_sha256 = $actualZipHash
            source_commit = $ExpectedSourceCommit
            version = $stageOneManifest.subject.version
        }
        execution = [ordered]@{
            resumed_after_partial_attempt = $stageSixResumed
            prior_runtime_preserved = $stageSixRootExistedBefore
            runtime_root = $stageSixRoot
            auxiliary_data_root = $portableSourceDataRoot
        }
        projection = [ordered]@{
            privacy_profile = 'SHAREABLE'
            initial_render_verified = $true
            divergence_exit_code = 15
            divergence_detected = $divergenceResult.error.code -eq 'GENERATED_ARTIFACT_DIVERGED'
            regenerated_matches_authoritative_render = $regeneratedMatches
            existing_output_exit_code = 2
            existing_output_unchanged = $authoritativeRenderUnchanged
            state_version_delta = $projectionStateDelta
            state_digest_unchanged = $afterProjection.state_digest -eq $beforeProjection.state_digest
            markdown_is_authority = $false
        }
        portable = [ordered]@{
            format = 'project-ledger-portable-bundle/1'
            scope = 'AUXILIARY_DISPOSABLE_PROJECT'
            project_ref = $portableProjectRef
            source_state_version = [int64]$portableSourceState.state_version
            export_state_version_delta = $portableExportDelta
            import_disposition = $portableImport.disposition
            state_version_matches = [int64]$importedState.state_version -eq [int64]$portableSourceState.state_version
            state_digest_matches = $importedState.state_digest -eq $portableSourceState.state_digest
            history_head_digest_matches = $portableHeadMatches
            resume_matches = $portableResumeMatches
            restores_command_results = $portableImport.restores_command_results
            restores_execution_reservations = $portableImport.restores_execution_reservations
            identical_chain_disposition = $identicalImport.disposition
            identical_chain_state_version_delta = $identicalImportDelta
            existing_export_exit_code = 2
            existing_export_unchanged = $portableExistingUnchanged
            checksum_corruption_exit_code = 15
            corrupt_destination_created = Test-Path -LiteralPath $corruptPortableDatabase
            divergence_exit_code = 15
            divergent_target_unchanged = $divergentTargetUnchanged
        }
        backup = [ordered]@{
            format = $backupManifest.format
            database_digest = $backupManifest.database_digest
            source_state_version = [int64]$sourceState.state_version
            backup_state_version_delta = $backupDelta
            restore_state_version_matches = [int64]$restoredState.state_version -eq [int64]$sourceState.state_version
            restore_state_digest_matches = $restoredState.state_digest -eq $sourceState.state_digest
            restore_head_digest_matches = $restoreHeadMatches
            reconcile_outcome = $restoredReconcile.outcome
            reconciled_status = $restoredReconcile.result.status
            replay_status = $restoredReplay.status
            replay_state_version_delta = $restoredReplayDelta
            existing_backup_exit_code = 2
            existing_backup_unchanged = $backupExistingUnchanged
            existing_restore_exit_code = 2
            existing_restore_unchanged = $restoreExistingUnchanged
            checksum_corruption_exit_code = 15
            corrupt_destination_created = Test-Path -LiteralPath $corruptRestoreDatabase
        }
        privacy = [ordered]@{
            canary_count = $canaries.Count
            evidence_ref = $canaryEvidenceRef
            evidence_execution_exit_code = 3
            evidence_record_status = $canaryEvidence.result.status
            raw_output_stored = $canaryEvidenceView.privacy.raw_output_stored
            surfaces_checked = @(
                'resume-json',
                'render-shareable-post-canary',
                'portable-export-clean-roundtrip',
                'portable-export-post-canary-fail-closed',
                'stage-manifest'
            )
            canary_match_count = $canaryMatchCount
            post_canary_render_verified = $true
            portable_export_exit_code = 13
            portable_export_rejected = $postCanaryPortable.error.code -eq 'PRIVACY_REJECTED'
            portable_destination_created = Test-Path -LiteralPath $postCanaryPortablePath
            sanitized_report_check_required = $true
            universal_byte_absence_claimed = $false
        }
        integrity = [ordered]@{
            state_version = [int64]$finalState.state_version
            state_digest = $finalState.state_digest
            head_digest = $finalDoctor.head_digest
            doctor_integrity = $finalDoctor.integrity
            verify_integrity = $finalVerify.integrity
            audit_integrity = $finalAudit.integrity
            verify_finding_count = @($finalVerify.findings).Count
            audit_finding_count = @($finalAudit.findings).Count
            unresolved_submission_count = $finalAudit.unresolved_submission_count
        }
        lifecycle = [ordered]@{
            initial_state_version = [int64]$initialState.state_version
            final_state_version = [int64]$finalState.state_version
            final_state_digest = $finalState.state_digest
            final_head_event_digest = $finalDoctor.head_digest
        }
        boundary = [ordered]@{
            project_status = $finalState.project.lifecycle_status
            active_phase_ref = $finalState.primary_active_phase_ref
            completed_phase_count = @($finalState.completed_phase_refs).Count
            open_blocker_ref = 'blocker:opa-stage7-integrations'
            next_action_ref = $finalState.preferred_next_action_ref
            checkpoint_ref = $finalResume.checkpoint_ref
            checkpoint_freshness = $finalResume.checkpoint_freshness
            stage_7_executed = $false
            merge_performed = $false
            release_performed = $false
            operational_acceptance_declared = $false
        }
        mutations = @($script:stageSixMutations)
        scenarios = [ordered]@{
            'OPA-S22' = 'PASS'
            'OPA-S23' = 'PASS'
            'OPA-S24' = 'PASS'
            'OPA-S25' = 'PASS'
            'OPA-S26' = 'PASS'
            'OPA-S27' = 'PASS'
        }
    }
    $manifestText = $manifest | ConvertTo-Json -Depth 20
    foreach ($canary in $canaries) {
        if ($manifestText.Contains($canary, [StringComparison]::Ordinal)) {
            throw 'PILOT_STAGE_6_CANARY_LEAK_IN_MANIFEST'
        }
    }
    Write-PilotJson $stageSixManifestPath $manifest

    [pscustomobject]@{
        stage = 6
        status = 'RECOVERY_PRIVACY_PASS'
        evidence = 'evidence/stage-6-projections-privacy-recovery.json'
    }
    return
}

if ($Stage -eq 5) {
    $resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
    $resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
    $resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)
    if (-not (Test-Path -LiteralPath $resolvedPilotRoot -PathType Container)) {
        throw "PILOT_STAGE_1_REQUIRED: $resolvedPilotRoot"
    }

    $evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
    $fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
    $stageOneManifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
    $stageTwoManifestPath = Join-Path $evidenceDirectory 'stage-2-governed-model.json'
    $stageThreeManifestPath = Join-Path $evidenceDirectory 'stage-3-high-level-continuity.json'
    $stageFourManifestPath = Join-Path $evidenceDirectory 'stage-4-evidence-lifecycle.json'
    $stageFiveManifestPath = Join-Path $evidenceDirectory 'stage-5-transactional-safety.json'
    $locatorPath = Join-Path $resolvedPilotRoot 'pilot-locator.json'
    $installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
    $ledger = Join-Path $installation 'project-ledger.exe'
    $packageManifestPath = Join-Path $installation 'package.json'
    foreach ($requiredPath in @(
        $stageOneManifestPath,
        $stageTwoManifestPath,
        $stageThreeManifestPath,
        $stageFourManifestPath,
        $locatorPath,
        $ledger,
        $packageManifestPath
    )) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_4_EVIDENCE_MISSING: $requiredPath"
        }
    }
    if (Test-Path -LiteralPath $stageFiveManifestPath) {
        throw "PILOT_STAGE_ALREADY_EXECUTED: $stageFiveManifestPath"
    }

    $actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
    if ($actualZipHash -ne $ExpectedZipSha256.ToLowerInvariant()) {
        throw "PILOT_ZIP_HASH_MISMATCH: expected $($ExpectedZipSha256.ToLowerInvariant()); actual $actualZipHash"
    }
    $stageOneManifest = Get-Content -Raw -LiteralPath $stageOneManifestPath | ConvertFrom-Json
    $stageTwoManifest = Get-Content -Raw -LiteralPath $stageTwoManifestPath | ConvertFrom-Json
    $stageThreeManifest = Get-Content -Raw -LiteralPath $stageThreeManifestPath | ConvertFrom-Json
    $stageFourManifest = Get-Content -Raw -LiteralPath $stageFourManifestPath | ConvertFrom-Json
    $packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
    if ($stageOneManifest.status -ne 'PREFLIGHT_PASS' -or
        $stageTwoManifest.status -ne 'GOVERNED_MODEL_PASS' -or
        $stageThreeManifest.status -ne 'HIGH_LEVEL_CONTINUITY_PASS' -or
        $stageFourManifest.status -ne 'EVIDENCE_LIFECYCLE_PASS' -or
        $stageOneManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageFourManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageOneManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $stageFourManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $packageManifest.source_commit -ne $ExpectedSourceCommit) {
        throw 'PILOT_STAGE_4_IDENTITY_MISMATCH'
    }

    $locator = Get-Content -Raw -LiteralPath $locatorPath | ConvertFrom-Json
    $database = [string]$locator.database
    $projectRef = [string]$locator.project_ref
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'PILOT_DATABASE_MISSING'
    }
    $initialState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $stageFourLastMutation = @($stageFourManifest.mutations)[-1]
    if ([int64]$initialState.state_version -ne [int64]$stageFourLastMutation.resulting_state_version -or
        $initialState.project.lifecycle_status -ne 'BLOCKED' -or
        $initialState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($initialState.completed_phase_refs).Count -ne 0 -or
        $initialState.preferred_next_action_ref -ne 'action:opa-stage5-transaction' -or
        @($initialState.open_blocker_refs) -notcontains 'blocker:opa-stage5-transaction' -or
        $initialState.checkpoint_ref -ne $stageFourManifest.freshness.replacement_checkpoint_ref) {
        throw 'PILOT_STAGE_5_INITIAL_STATE_MISMATCH'
    }

    $observedAt = [DateTimeOffset]::UtcNow.ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $script:stageFiveStateVersion = [int64]$initialState.state_version
    $script:stageFiveMutations = @()

    function Invoke-StageFiveCommand {
        param(
            [string]$Name,
            [string]$CommandType,
            [string]$ActorId,
            [bool]$AsHuman,
            [string[]]$CommandArguments,
            [int64]$ExpectedVersion,
            [string]$IdempotencyKey,
            [int]$ExpectedExitCode,
            [int64]$ExpectedDelta
        )

        $before = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $arguments = @($CommandArguments) + @(
            '--database', $database,
            '--project-ref', $projectRef,
            '--expected-version', [string]$ExpectedVersion,
            '--idempotency-key', $IdempotencyKey,
            '--actor-id', $ActorId,
            '--observed-at', $observedAt
        )
        if ($AsHuman) { $arguments += '--as-human' }
        $arguments += '--json'
        $result = Invoke-PilotLedgerJson `
            -Executable $ledger `
            -Arguments $arguments `
            -ExpectedExitCodes @($ExpectedExitCode)
        $after = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $delta = [int64]$after.state_version - [int64]$before.state_version
        if ($delta -ne $ExpectedDelta) {
            throw "PILOT_STAGE_5_MUTATION_DELTA_INVALID: $Name expected=$ExpectedDelta actual=$delta"
        }
        if ($ExpectedDelta -eq 0 -and $after.state_digest -ne $before.state_digest) {
            throw "PILOT_STAGE_5_ZERO_DELTA_DIGEST_CHANGED: $Name"
        }
        $script:stageFiveStateVersion = [int64]$after.state_version
        $script:stageFiveMutations += [ordered]@{
            name = $Name
            command_type = $CommandType
            actor_id = $ActorId
            as_human = $AsHuman
            expected_exit_code = $ExpectedExitCode
            observed_exit_code = $ExpectedExitCode
            expected_state_version = $ExpectedVersion
            state_version_before = [int64]$before.state_version
            resulting_state_version = [int64]$after.state_version
            state_version_delta = $delta
            idempotency_key = $IdempotencyKey
        }
        return [ordered]@{ result = $result; before = $before; state = $after; delta = $delta }
    }

    $initialVersion = [int64]$initialState.state_version
    $replayArguments = @(
        'record', 'progress', '--summary', 'Stage 5 exact replay probe accepted once.',
        '--phase', 'phase:operational-acceptance',
        '--files', 'workspace://docs/pilot/G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md',
        '--record-id', 'work:opa-stage5-replay'
    )
    $first = Invoke-StageFiveCommand `
        'replay-first' 'RECORD_PROGRESS' 'agent:opa-stage5' $false `
        $replayArguments $initialVersion 'opa:stage5:replay' 0 1
    $afterFirst = $first.state
    $replay = Invoke-StageFiveCommand `
        'replay-identical' 'RECORD_PROGRESS' 'agent:opa-stage5' $false `
        $replayArguments $initialVersion 'opa:stage5:replay' 0 0
    if ($first.result.status -ne 'ACCEPTED' -or
        $replay.result.status -ne 'DUPLICATE_REPLAY' -or
        $replay.result.command_result_id -ne $first.result.command_result_id -or
        $replay.result.idempotency.replayed -ne $true) {
        throw "PILOT_STAGE_5_REPLAY_INVARIANT_FAILED: first_status=$($first.result.status) replay_status=$($replay.result.status) first_result=$($first.result.command_result_id) replay_result=$($replay.result.command_result_id) replayed=$($replay.result.idempotency.replayed)"
    }

    $idempotencyConflict = Invoke-StageFiveCommand `
        'idempotency-conflict' 'RECORD_PROGRESS' 'agent:opa-stage5' $false `
        @(
            'record', 'progress', '--summary', 'Changed intent must be rejected.',
            '--phase', 'phase:operational-acceptance', '--record-id', 'work:opa-stage5-conflict'
        ) `
        $initialVersion 'opa:stage5:replay' 10 0
    if ($idempotencyConflict.result.status -ne 'REJECTED' -or
        $idempotencyConflict.result.rejection.code -ne 'IDEMPOTENCY_CONFLICT') {
        throw 'PILOT_STAGE_5_IDEMPOTENCY_CONFLICT_NOT_TYPED'
    }

    $versionConflict = Invoke-StageFiveCommand `
        'version-conflict' 'RECORD_PROGRESS' 'agent:opa-stage5' $false `
        @(
            'record', 'progress', '--summary', 'Stale expected version must be rejected.',
            '--phase', 'phase:operational-acceptance', '--record-id', 'work:opa-stage5-stale'
        ) `
        $initialVersion 'opa:stage5:version-stale' 5 0
    if ($versionConflict.result.status -ne 'VERSION_CONFLICT') {
        throw 'PILOT_STAGE_5_VERSION_CONFLICT_NOT_TYPED'
    }
    $resumeAfterConflict = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    if ([int64]$resumeAfterConflict.state_version -ne [int64]$versionConflict.state.state_version) {
        throw 'PILOT_STAGE_5_RESUME_AFTER_CONFLICT_MISMATCH'
    }
    $replannedExpectedVersion = [int64]$resumeAfterConflict.state_version
    $replanned = Invoke-StageFiveCommand `
        'version-replanned' 'RECORD_PROGRESS' 'agent:opa-stage5' $false `
        @(
            'record', 'progress', '--summary', 'Stage 5 resumed and replanned from the current version.',
            '--phase', 'phase:operational-acceptance', '--record-id', 'work:opa-stage5-replanned'
        ) `
        $replannedExpectedVersion 'opa:stage5:version-replanned' 0 1
    if ($replanned.result.status -ne 'ACCEPTED') {
        throw 'PILOT_STAGE_5_REPLAN_NOT_ACCEPTED'
    }

    $agentAuthorityFixture = Join-Path $fixtureDirectory 'stage-5-agent-authority-forbidden.md'
    [IO.File]::WriteAllText(
        $agentAuthorityFixture,
        "# Stage 5 authority denial fixture`n`nAn agent must not register this as authority.`n",
        [Text.UTF8Encoding]::new($false)
    )
    $agentAuthorityDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $agentAuthorityFixture).Hash.ToLowerInvariant()
    $authorityDenial = Invoke-StageFiveCommand `
        'authority-denied' 'REGISTER_AUTHORITY_DOCUMENT' 'agent:opa-stage5' $false `
        @(
            'authority', 'add', '--uri', $agentAuthorityFixture, '--digest', $agentAuthorityDigest,
            '--record-id', 'authority:opa-stage5-agent-forbidden'
        ) `
        $script:stageFiveStateVersion 'opa:stage5:authority-denied' 9 0
    if ($authorityDenial.result.status -ne 'POLICY_DENIED') {
        throw 'PILOT_STAGE_5_AUTHORITY_DENIAL_NOT_TYPED'
    }

    $checkpointVersionBefore = [int64]$script:stageFiveStateVersion
    $checkpointKey = 'opa:stage5:checkpoint-outcome'
    $checkpointArguments = @(
        'checkpoint', 'create', '--reason', 'Stage 5 discarded-response reconciliation probe',
        '--database', $database, '--project-ref', $projectRef,
        '--expected-version', [string]$checkpointVersionBefore,
        '--idempotency-key', $checkpointKey,
        '--actor-id', 'agent:opa-stage5', '--observed-at', $observedAt, '--json'
    )
    & $ledger @checkpointArguments 1>$null 2>$null
    $discardedCheckpointExit = $LASTEXITCODE
    if ($discardedCheckpointExit -ne 0) {
        throw "PILOT_STAGE_5_DISCARDED_CHECKPOINT_FAILED: exit=$discardedCheckpointExit"
    }
    $afterDiscardedCheckpoint = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if (([int64]$afterDiscardedCheckpoint.state_version - $checkpointVersionBefore) -ne 1) {
        throw 'PILOT_STAGE_5_DISCARDED_CHECKPOINT_DELTA_INVALID'
    }
    $script:stageFiveStateVersion = [int64]$afterDiscardedCheckpoint.state_version
    $script:stageFiveMutations += [ordered]@{
        name = 'checkpoint-response-discarded'
        command_type = 'CREATE_CONTINUITY_CHECKPOINT'
        actor_id = 'agent:opa-stage5'
        as_human = $false
        expected_exit_code = 0
        observed_exit_code = $discardedCheckpointExit
        expected_state_version = $checkpointVersionBefore
        state_version_before = $checkpointVersionBefore
        resulting_state_version = [int64]$afterDiscardedCheckpoint.state_version
        state_version_delta = 1
        idempotency_key = $checkpointKey
        stdout_discarded = $true
    }
    $reconciledCheckpoint = Invoke-PilotLedgerJson $ledger @(
        'reconcile', '--database', $database, '--project-ref', $projectRef,
        '--command-type', 'CREATE_CONTINUITY_CHECKPOINT',
        '--actor-id', 'agent:opa-stage5', '--trust-domain', 'LOCAL_USER',
        '--idempotency-key', $checkpointKey, '--json'
    )
    $afterReconcile = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $reconcileDelta = [int64]$afterReconcile.state_version - [int64]$afterDiscardedCheckpoint.state_version
    if ($reconciledCheckpoint.outcome -ne 'RECONCILED' -or
        $reconciledCheckpoint.result.status -ne 'ACCEPTED' -or
        $reconcileDelta -ne 0 -or
        $afterReconcile.state_digest -ne $afterDiscardedCheckpoint.state_digest) {
        throw 'PILOT_STAGE_5_RECONCILE_INVARIANT_FAILED'
    }

    $beforeMissingResult = $afterReconcile
    $missingResult = Invoke-PilotLedgerJson `
        -Executable $ledger `
        -Arguments @(
            'reconcile', '--database', $database, '--project-ref', $projectRef,
            '--command-type', 'CREATE_CONTINUITY_CHECKPOINT',
            '--actor-id', 'agent:opa-stage5', '--trust-domain', 'LOCAL_USER',
            '--idempotency-key', 'opa:stage5:checkpoint-missing', '--json'
        ) `
        -ExpectedExitCodes @(12)
    $afterMissingResult = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $missingResultDelta = [int64]$afterMissingResult.state_version - [int64]$beforeMissingResult.state_version
    if ($missingResult.error.code -ne 'OUTCOME_UNKNOWN' -or
        $missingResultDelta -ne 0 -or
        $afterMissingResult.state_digest -ne $beforeMissingResult.state_digest) {
        throw 'PILOT_STAGE_5_MISSING_RESULT_INVARIANT_FAILED'
    }

    $null = Invoke-StageFiveCommand `
        'blocker-stage5-resolve' 'RESOLVE_BLOCKER' 'human:owner' $true `
        @(
            'blocker', 'resolve', 'blocker:opa-stage5-transaction',
            '--summary', 'Replay, conflict, authority, and uncertain-outcome safeguards passed Stage 5.'
        ) `
        $script:stageFiveStateVersion 'opa:stage5:blocker-resolve' 0 3
    $null = Invoke-StageFiveCommand `
        'next-stage6-set' 'SET_NEXT_ACTION' 'human:owner' $true `
        @(
            'next', 'set', '--title', 'Request authorization for Stage 6 recovery and privacy',
            '--summary', 'Stage 6 remains outside the current authorization.',
            '--phase', 'phase:operational-acceptance', '--classification', 'ELIGIBLE',
            '--record-id', 'action:opa-stage6-recovery'
        ) `
        $script:stageFiveStateVersion 'opa:stage5:next-stage6' 0 1
    $null = Invoke-StageFiveCommand `
        'blocker-stage6-open' 'OPEN_BLOCKER' 'human:owner' $true `
        @(
            'blocker', 'open', '--title', 'Stage 6 requires separate human authorization',
            '--summary', 'Projection, privacy, portable, backup, and restore tests are not authorized by Stage 5.',
            '--severity', 'MEDIUM', '--record-id', 'blocker:opa-stage6-recovery'
        ) `
        $script:stageFiveStateVersion 'opa:stage5:blocker-stage6' 0 2
    $finalCheckpoint = Invoke-StageFiveCommand `
        'checkpoint-final' 'CREATE_CONTINUITY_CHECKPOINT' 'agent:opa-stage5' $false `
        @('checkpoint', 'create', '--reason', 'Stage 5 transactional safety handoff') `
        $script:stageFiveStateVersion 'opa:stage5:checkpoint-final' 0 1

    $finalResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    $finalState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $finalDoctor = Invoke-PilotLedgerJson $ledger @(
        'doctor', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseList = @(Invoke-PilotLedgerJson $ledger @(
        'phase', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $activePhase = @($phaseList | Where-Object { $_.phase_ref -eq 'phase:operational-acceptance' })
    if ($finalResume.checkpoint_freshness -ne 'CURRENT' -or
        $finalState.project.lifecycle_status -ne 'BLOCKED' -or
        $finalState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($finalState.completed_phase_refs).Count -ne 0 -or
        $finalState.preferred_next_action_ref -ne 'action:opa-stage6-recovery' -or
        @($finalState.open_blocker_refs) -notcontains 'blocker:opa-stage6-recovery' -or
        @($finalState.open_blocker_refs) -contains 'blocker:opa-stage5-transaction' -or
        $finalDoctor.integrity -ne 'ok' -or
        [int64]$finalDoctor.state_version -ne [int64]$finalState.state_version -or
        $activePhase.Count -ne 1 -or $activePhase[0].status -ne 'ACTIVE') {
        throw 'PILOT_STAGE_5_FINAL_STATE_MISMATCH'
    }

    $manifest = [ordered]@{
        profile = 'project-ledger-operational-pilot-stage/1'
        stage = 5
        status = 'TRANSACTIONAL_SAFETY_PASS'
        proof_class = 'LIVE_PILOT'
        project_ref = $projectRef
        subject = [ordered]@{
            zip_sha256 = $actualZipHash
            source_commit = $ExpectedSourceCommit
            version = $stageOneManifest.subject.version
        }
        replay = [ordered]@{
            command_type = 'RECORD_PROGRESS'
            actor_id = 'agent:opa-stage5'
            trust_domain = 'TRUSTED_FOR_RECORDING'
            idempotency_key = 'opa:stage5:replay'
            first_status = $first.result.status
            replay_status = $replay.result.status
            replay_state_version_delta = $replay.delta
            replay_state_digest_unchanged = $replay.before.state_digest -eq $replay.state.state_digest
            same_command_result_id = $replay.result.command_result_id -eq $first.result.command_result_id
            replayed = $replay.result.idempotency.replayed
        }
        idempotency_conflict = [ordered]@{
            exit_code = 10
            status = $idempotencyConflict.result.status
            error_code = $idempotencyConflict.result.rejection.code
            state_version_delta = $idempotencyConflict.delta
            state_digest_unchanged = $idempotencyConflict.before.state_digest -eq $idempotencyConflict.state.state_digest
        }
        version_conflict = [ordered]@{
            exit_code = 5
            status = $versionConflict.result.status
            state_version_delta = $versionConflict.delta
            state_digest_unchanged = $versionConflict.before.state_digest -eq $versionConflict.state.state_digest
            resume_observed = [int64]$resumeAfterConflict.state_version -eq [int64]$versionConflict.state.state_version
            replanned_expected_version = $replannedExpectedVersion
            replanned_status = $replanned.result.status
            original_key_reused = $false
        }
        authority_denial = [ordered]@{
            exit_code = 9
            status = $authorityDenial.result.status
            state_version_delta = $authorityDenial.delta
            state_digest_unchanged = $authorityDenial.before.state_digest -eq $authorityDenial.state.state_digest
            as_human_added = $false
        }
        uncertain_checkpoint = [ordered]@{
            stdout_discarded = $true
            command_attempt_count = 1
            idempotency_key = $checkpointKey
            actor_id = 'agent:opa-stage5'
            actor_kind = 'AGENT'
            command_type = 'CREATE_CONTINUITY_CHECKPOINT'
            trust_domain = 'LOCAL_USER'
            initial_state_version_delta = 1
            reconcile_outcome = $reconciledCheckpoint.outcome
            reconciled_status = $reconciledCheckpoint.result.status
            reconciled_result_digest = $reconciledCheckpoint.result.digest
            reconcile_state_version_delta = $reconcileDelta
            blind_resubmission_performed = $false
        }
        missing_result = [ordered]@{
            exit_code = 12
            error_code = $missingResult.error.code
            state_version_delta = $missingResultDelta
            state_digest_unchanged = $beforeMissingResult.state_digest -eq $afterMissingResult.state_digest
            blind_retry_performed = $false
        }
        lifecycle = [ordered]@{
            initial_state_version = [int64]$initialState.state_version
            final_state_version = [int64]$finalState.state_version
            final_state_digest = $finalState.state_digest
            final_head_event_digest = $finalDoctor.head_digest
        }
        boundary = [ordered]@{
            project_status = $finalState.project.lifecycle_status
            active_phase_ref = $finalState.primary_active_phase_ref
            completed_phase_count = @($finalState.completed_phase_refs).Count
            open_blocker_ref = 'blocker:opa-stage6-recovery'
            next_action_ref = $finalState.preferred_next_action_ref
            checkpoint_ref = $finalResume.checkpoint_ref
            checkpoint_freshness = $finalResume.checkpoint_freshness
            stage_6_executed = $false
            merge_performed = $false
            release_performed = $false
            operational_acceptance_declared = $false
        }
        mutations = @($script:stageFiveMutations)
        scenarios = [ordered]@{
            'OPA-S16' = 'PASS'
            'OPA-S17' = 'PASS'
            'OPA-S18' = 'PASS'
            'OPA-S19' = 'PASS'
            'OPA-S20' = 'PASS'
            'OPA-S21' = 'PASS'
        }
    }
    Write-PilotJson $stageFiveManifestPath $manifest

    [pscustomobject]@{
        stage = 5
        status = 'TRANSACTIONAL_SAFETY_PASS'
        evidence = 'evidence/stage-5-transactional-safety.json'
    }
    return
}

if ($Stage -eq 4) {
    $resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
    $resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
    $resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)
    if (-not (Test-Path -LiteralPath $resolvedPilotRoot -PathType Container)) {
        throw "PILOT_STAGE_1_REQUIRED: $resolvedPilotRoot"
    }

    $evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
    $fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
    $stageOneManifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
    $stageTwoManifestPath = Join-Path $evidenceDirectory 'stage-2-governed-model.json'
    $stageThreeManifestPath = Join-Path $evidenceDirectory 'stage-3-high-level-continuity.json'
    $stageFourManifestPath = Join-Path $evidenceDirectory 'stage-4-evidence-lifecycle.json'
    $locatorPath = Join-Path $resolvedPilotRoot 'pilot-locator.json'
    $installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
    $ledger = Join-Path $installation 'project-ledger.exe'
    $packageManifestPath = Join-Path $installation 'package.json'
    $recordSinkClient = Join-Path $resolvedRepository 'scripts\invoke-record-sink-rpc.ps1'
    foreach ($requiredPath in @(
        $stageOneManifestPath,
        $stageTwoManifestPath,
        $stageThreeManifestPath,
        $locatorPath,
        $ledger,
        $packageManifestPath,
        $recordSinkClient
    )) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_3_EVIDENCE_MISSING: $requiredPath"
        }
    }
    if (Test-Path -LiteralPath $stageFourManifestPath) {
        throw "PILOT_STAGE_ALREADY_EXECUTED: $stageFourManifestPath"
    }

    $actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
    if ($actualZipHash -ne $ExpectedZipSha256.ToLowerInvariant()) {
        throw "PILOT_ZIP_HASH_MISMATCH: expected $($ExpectedZipSha256.ToLowerInvariant()); actual $actualZipHash"
    }
    $stageOneManifest = Get-Content -Raw -LiteralPath $stageOneManifestPath | ConvertFrom-Json
    $stageTwoManifest = Get-Content -Raw -LiteralPath $stageTwoManifestPath | ConvertFrom-Json
    $stageThreeManifest = Get-Content -Raw -LiteralPath $stageThreeManifestPath | ConvertFrom-Json
    $packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
    if ($stageOneManifest.status -ne 'PREFLIGHT_PASS' -or
        $stageTwoManifest.status -ne 'GOVERNED_MODEL_PASS' -or
        $stageThreeManifest.status -ne 'HIGH_LEVEL_CONTINUITY_PASS' -or
        $stageOneManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageThreeManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageOneManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $stageThreeManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $packageManifest.source_commit -ne $ExpectedSourceCommit) {
        throw 'PILOT_STAGE_3_IDENTITY_MISMATCH'
    }

    $locator = Get-Content -Raw -LiteralPath $locatorPath | ConvertFrom-Json
    $database = [string]$locator.database
    $projectRef = [string]$locator.project_ref
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'PILOT_DATABASE_MISSING'
    }
    $initialState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($initialState.state_version -ne 44 -or
        $initialState.state_digest -ne $stageThreeManifest.lifecycle.final_state_digest -or
        $initialState.project.lifecycle_status -ne 'BLOCKED' -or
        $initialState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($initialState.completed_phase_refs).Count -ne 0 -or
        @($initialState.verified_gate_refs).Count -ne 0 -or
        $initialState.checkpoint_ref -ne $stageThreeManifest.checkpoint.checkpoint_ref) {
        throw 'PILOT_STAGE_4_INITIAL_STATE_MISMATCH'
    }

    $observedAt = [DateTimeOffset]::UtcNow.ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $approvalExpiresAt = [DateTimeOffset]::UtcNow.AddMinutes(15).ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $script:stageFourStateVersion = [int64]$initialState.state_version
    $script:stageFourMutations = @()

    function Invoke-StageFourCommand {
        param(
            [string]$Name,
            [string]$CommandType,
            [string]$ActorId,
            [bool]$AsHuman,
            [string[]]$CommandArguments,
            [int]$ExpectedExitCode,
            [bool]$ExpectMutation
        )

        $expectedVersion = $script:stageFourStateVersion
        $delimiterIndex = [Array]::IndexOf([string[]]$CommandArguments, '--')
        $operationArguments = if ($delimiterIndex -ge 0) {
            @($CommandArguments[0..($delimiterIndex - 1)])
        } else {
            @($CommandArguments)
        }
        $childArguments = if ($delimiterIndex -ge 0 -and $delimiterIndex -lt $CommandArguments.Count - 1) {
            @($CommandArguments[($delimiterIndex + 1)..($CommandArguments.Count - 1)])
        } else {
            @()
        }
        $arguments = @($operationArguments) + @(
            '--database', $database,
            '--project-ref', $projectRef,
            '--expected-version', [string]$expectedVersion,
            '--idempotency-key', ('opa:stage4:' + $Name),
            '--actor-id', $ActorId,
            '--observed-at', $observedAt
        )
        if ($AsHuman) { $arguments += '--as-human' }
        $arguments += '--json'
        if ($delimiterIndex -ge 0) {
            $arguments += '--'
            $arguments += $childArguments
        }
        $result = Invoke-PilotLedgerJson `
            -Executable $ledger `
            -Arguments $arguments `
            -ExpectedExitCodes @($ExpectedExitCode)
        $after = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $delta = [int64]$after.state_version - $expectedVersion
        if (($ExpectMutation -and $delta -le 0) -or ((-not $ExpectMutation) -and $delta -ne 0)) {
            throw "PILOT_STAGE_4_MUTATION_DELTA_INVALID: $Name delta=$delta"
        }
        $script:stageFourStateVersion = [int64]$after.state_version
        $script:stageFourMutations += [ordered]@{
            name = $Name
            command_type = $CommandType
            actor_id = $ActorId
            as_human = $AsHuman
            expected_exit_code = $ExpectedExitCode
            observed_exit_code = $ExpectedExitCode
            expected_state_version = $expectedVersion
            resulting_state_version = $script:stageFourStateVersion
            state_version_delta = $delta
            idempotency_key = 'opa:stage4:' + $Name
        }
        return [ordered]@{ result = $result; state = $after; delta = $delta }
    }

    $executionPolicyPath = Join-Path $fixtureDirectory 'stage-4-evidence-run-policy.json'
    Write-PilotJson $executionPolicyPath ([ordered]@{
        allowed_programs = @($ledger)
        redactions = @()
        max_summary_bytes = 4096
        process = [ordered]@{
            timeout_ms = 10000
            environment = [ordered]@{}
        }
    })
    $observedRun = Invoke-StageFourCommand `
        'observed-evidence' `
        'RECORD_VERIFICATION_RESULT' `
        'agent:opa-stage4' `
        $false `
        @(
            'evidence', 'run', '--policy', $executionPolicyPath, '--cwd', $installation,
            '--kind', 'CUSTOM_DECLARATIVE', '--gate', 'gate:opa-04', '--', $ledger, '--version'
        ) `
        0 `
        $true
    $observedIdentity = ConvertTo-PilotCanonicalJson @(
        $projectRef,
        'agent:opa-stage4',
        'TRUSTED_FOR_RECORDING',
        'opa:stage4:observed-evidence'
    )
    $observedIdentityDigest = Get-PilotTextDigest $observedIdentity
    $observedEvidenceRef = 'evidence:' + $observedIdentityDigest.Substring(7)
    $observedEvidence = Invoke-PilotLedgerJson $ledger @(
        'evidence', 'show', $observedEvidenceRef,
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $observedGate = Invoke-StageFourCommand `
        'gate-evaluate-observed' `
        'REQUEST_GATE_EVALUATION' `
        'agent:opa-stage4' `
        $false `
        @('gate', 'evaluate', 'gate:opa-04') `
        6 `
        $true
    $gateAfterObserved = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-04',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseAfterObserved = Invoke-StageFourCommand `
        'phase-close-observed' `
        'REQUEST_PHASE_COMPLETION' `
        'agent:opa-stage4' `
        $false `
        @('phase', 'request-close', 'phase:operational-acceptance') `
        6 `
        $false

    $gskillsRoot = Join-Path $installation 'resources\integrations\gskills'
    $providerManifestPath = Join-Path $gskillsRoot 'reports\record-sink-conformance.json'
    $compatibilityLockPath = Join-Path $gskillsRoot 'profile-contract-lock.json'
    $identityMapPath = Join-Path $gskillsRoot 'reports\record-sink-field-mapping.json'
    foreach ($requiredPath in @($providerManifestPath, $compatibilityLockPath, $identityMapPath)) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_4_RECORD_SINK_RESOURCE_MISSING: $requiredPath"
        }
    }
    $providerManifestDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $providerManifestPath).Hash.ToLowerInvariant()
    $providerExecutableDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $ledger).Hash.ToLowerInvariant()
    $compatibilityLockDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $compatibilityLockPath).Hash.ToLowerInvariant()
    $identityMapDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $identityMapPath).Hash.ToLowerInvariant()
    $gskillsProjectRef = 'project:g-ledger-operational-pilot'
    $externalProjectRef = 'project-ledger://projects/g-ledger-operational-pilot'
    $actorTrustDomain = 'agent_harness:on_behalf_of_local_user'
    $recordSinkPolicyWithoutDigest = [ordered]@{
        schema_version = '0.2.0'
        policy_id = 'policy:opa-stage4-record-sink'
        sink_id = 'project-ledger'
        profile = 'gskills-context-provider/0.3.0'
        gskills_project_ref = $gskillsProjectRef
        external_project_ref = $externalProjectRef
        native_project_ref = $projectRef
        external_actor = [ordered]@{
            type = 'agent_harness'
            id = 'opa-stage4-client'
            on_behalf_of = 'human:owner'
        }
        native_actor = [ordered]@{
            actor_id = 'integration:gskills-pilot'
            actor_kind = 'INTEGRATION'
            trust_level = 'TRUSTED_FOR_RECORDING'
            display_name = 'G-Ledger OPA local RecordSink client'
        }
        actor_trust_domain = $actorTrustDomain
        allowed_command_kinds = @(
            'RECORD_WORK_PROGRESS',
            'RECORD_VERIFICATION_RESULT',
            'REQUEST_GATE_EVALUATION',
            'REQUEST_GATE_REVALIDATION',
            'CREATE_CONTINUITY_CHECKPOINT'
        )
        approval_requirement = 'USER_OR_POLICY'
        provider_manifest_digest = $providerManifestDigest
        provider_executable_digest = $providerExecutableDigest
        compatibility_lock_digest = $compatibilityLockDigest
        identity_map_digest = $identityMapDigest
        created_at = $observedAt
    }
    $recordSinkPolicy = [ordered]@{}
    foreach ($key in $recordSinkPolicyWithoutDigest.Keys) {
        $recordSinkPolicy[$key] = $recordSinkPolicyWithoutDigest[$key]
    }
    $recordSinkPolicy['digest'] = Get-PilotCanonicalDigest $recordSinkPolicyWithoutDigest
    $recordSinkPolicyPath = Join-Path $fixtureDirectory 'record-sink-policy-stage4.json'
    Write-PilotJson $recordSinkPolicyPath $recordSinkPolicy

    $beforeSink = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $verifiedEvidenceWithoutDigest = [ordered]@{
        schema_version = '0.2.0'
        evidence_id = 'evidence:opa-stage4-verified-source'
        project_ref = $projectRef
        kind = 'CUSTOM_DECLARATIVE'
        status = 'VERIFIED_SOURCE'
        source = [ordered]@{
            source_type = 'EXTERNAL_SYSTEM'
            tool = 'opa-record-sink-harness'
            command_display = $null
            working_directory_ref = $null
            runner_version = 'project-ledger-operational-pilot/1'
        }
        result = [ordered]@{
            exit_code = 0
            duration_ms = 1
            metrics = [ordered]@{ passed = 1; failed = 0; skipped = 0; errors = 0 }
            summary = 'Verificación exacta de Stage 4 mediante RecordSink local.'
        }
        repository_binding = $null
        artifacts = @(
            [ordered]@{
                uri = [Uri]::new($resolvedZip).AbsoluteUri
                digest = 'sha256:' + $actualZipHash
                persisted = $false
            }
        )
        privacy = [ordered]@{ raw_output_stored = $false; redaction_applied = $true }
        observed_at = $observedAt
        provenance_ref = 'urn:g-ledger:pilot:stage4:recordsink'
    }
    $verifiedEvidence = [ordered]@{}
    foreach ($key in $verifiedEvidenceWithoutDigest.Keys) {
        $verifiedEvidence[$key] = $verifiedEvidenceWithoutDigest[$key]
    }
    $verifiedEvidence['payload_digest'] = Get-PilotCanonicalDigest $verifiedEvidenceWithoutDigest

    $externalCommand = [ordered]@{
        schema_version = '0.3.0'
        external_record_command_id = 'external-command:' + $projectRef.Substring('project:'.Length)
        command_kind = 'RECORD_VERIFICATION_RESULT'
        sink_id = 'project-ledger'
        profile = 'gskills-context-provider/0.3.0'
        project_ref = $gskillsProjectRef
        external_project_ref = $externalProjectRef
        actor = [ordered]@{
            type = 'agent_harness'
            id = 'opa-stage4-client'
            on_behalf_of = 'human:owner'
        }
        session_ref = 'session:opa-stage4-record-sink'
        native_command = [ordered]@{
            command_type = 'RECORD_VERIFICATION_RESULT'
            command_schema_ref = 'urn:project-ledger:command:record-verification-result:0.2.0'
            command_schema_ref_status = 'EXACT'
            expected_state_version = [int64]$beforeSink.state_version
        }
        expected_state = [ordered]@{
            project_state_version = [int64]$beforeSink.state_version
            material_state_version = [int64]$beforeSink.material_state_version
            project_state_digest = [string]$beforeSink.state_digest
        }
        payload = [ordered]@{ evidence = $verifiedEvidence }
        idempotency_key = 'opa:stage4:sink:verified-evidence'
        idempotency_scope = [ordered]@{
            sink_id = 'project-ledger'
            external_project_ref = $externalProjectRef
            native_command_type = 'RECORD_VERIFICATION_RESULT'
            actor_trust_domain = $actorTrustDomain
        }
        preparation = [ordered]@{
            prepared_by = 'gskills'
            prepared_at = $observedAt
            prepared_command_digest = 'sha256:' + ('0' * 64)
            preview_digest = 'sha256:' + ('0' * 64)
            provider_manifest_digest = $providerManifestDigest
            provider_executable_digest = $providerExecutableDigest
            compatibility_lock_digest = $compatibilityLockDigest
            policy_digest = $recordSinkPolicy.digest
            identity_map_digest = $identityMapDigest
            expected_state_digest = [string]$beforeSink.state_digest
            approval_requirement = 'USER_OR_POLICY'
            approval_expires_at = $approvalExpiresAt
        }
        privacy = [ordered]@{ raw_output_included = $false; evidence_refs_only = $true }
        provenance_ref = 'gskills://provenance/opa-stage4-record-sink'
        digest = 'sha256:' + ('0' * 64)
    }
    $preparedCore = $externalCommand | ConvertTo-Json -Depth 30 -Compress |
        ConvertFrom-Json -AsHashtable -DateKind String
    $preparedCore.Remove('digest')
    foreach ($field in @(
        'prepared_command_digest',
        'preview_digest',
        'provider_manifest_digest',
        'provider_executable_digest',
        'compatibility_lock_digest',
        'policy_digest',
        'identity_map_digest',
        'expected_state_digest'
    )) {
        $preparedCore.preparation.Remove($field)
    }
    $preparedDigest = Get-PilotCanonicalDigest $preparedCore
    $externalCommand.preparation.prepared_command_digest = $preparedDigest
    $externalCommand.preparation.preview_digest = $preparedDigest
    $commandWithoutDigest = $externalCommand | ConvertTo-Json -Depth 30 -Compress |
        ConvertFrom-Json -AsHashtable -DateKind String
    $commandWithoutDigest.Remove('digest')
    $externalCommand.digest = Get-PilotCanonicalDigest $commandWithoutDigest

    $recordSinkRequests = @(
        [ordered]@{ jsonrpc = '2.0'; id = 1; method = 'ledger.command_validate'; params = [ordered]@{ command = $externalCommand } },
        [ordered]@{ jsonrpc = '2.0'; id = 2; method = 'ledger.command_submit'; params = [ordered]@{ command = $externalCommand } },
        [ordered]@{ jsonrpc = '2.0'; id = 3; method = 'ledger.command_submit'; params = [ordered]@{ command = $externalCommand } },
        [ordered]@{ jsonrpc = '2.0'; id = 4; method = 'ledger.command_status'; params = [ordered]@{ command = $externalCommand } },
        [ordered]@{ jsonrpc = '2.0'; id = 5; method = 'ledger.command_reconcile'; params = [ordered]@{ command = $externalCommand } }
    )
    $recordSinkRequestsPath = Join-Path $fixtureDirectory 'record-sink-requests-stage4.json'
    Write-PilotJson $recordSinkRequestsPath $recordSinkRequests
    $rpcOutput = & $recordSinkClient `
        -Executable $ledger `
        -ProcessArguments @(
            'serve', 'sink', '--stdio', '--profile', 'gskills-context-provider/0.3.0',
            '--database', $database, '--project-ref', $projectRef,
            '--gskills-project-ref', $gskillsProjectRef, '--policy', $recordSinkPolicyPath,
            '--observed-at', $observedAt
        ) `
        -RequestsFile $recordSinkRequestsPath
    if ($LASTEXITCODE -ne 0) {
        throw "PILOT_STAGE_4_RECORD_SINK_CLIENT_FAILED: exit=$LASTEXITCODE"
    }
    $responses = @(($rpcOutput | Out-String).Trim() | ConvertFrom-Json)
    $responseErrors = @($responses | Where-Object {
        $null -ne $_.PSObject.Properties['error'] -and $null -ne $_.error
    })
    if ($responses.Count -ne 5 -or $responseErrors.Count -ne 0) {
        throw 'PILOT_STAGE_4_RECORD_SINK_RESPONSE_INVALID'
    }
    $validateResult = $responses[0].result
    $submitResult = $responses[1].result
    $replayResult = $responses[2].result
    $statusResult = $responses[3].result
    $reconcileResult = $responses[4].result
    $afterSink = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $script:stageFourStateVersion = [int64]$afterSink.state_version
    $verifiedEvidenceView = Invoke-PilotLedgerJson $ledger @(
        'evidence', 'show', 'evidence:opa-stage4-verified-source',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($validateResult.status -ne 'VALID' -or
        $submitResult.status -ne 'ACCEPTED' -or
        $replayResult.status -ne 'DUPLICATE_REPLAY' -or
        $statusResult.status -ne 'FOUND' -or
        $reconcileResult.status -ne 'FOUND' -or
        ([int64]$afterSink.state_version - [int64]$beforeSink.state_version) -ne 1 -or
        $verifiedEvidenceView.status -ne 'VERIFIED_SOURCE' -or
        $verifiedEvidenceView.privacy.raw_output_stored -ne $false -or
        $statusResult.result.native_result_digest -ne $submitResult.native_result_digest -or
        $reconcileResult.result.native_result_digest -ne $submitResult.native_result_digest) {
        throw 'PILOT_STAGE_4_RECORD_SINK_INVARIANT_FAILED'
    }

    $verifiedGate = Invoke-StageFourCommand `
        'gate-evaluate-verified' `
        'REQUEST_GATE_EVALUATION' `
        'agent:opa-stage4' `
        $false `
        @('gate', 'evaluate', 'gate:opa-04') `
        0 `
        $true
    $gateAfterVerified = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-04',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseAfterVerified = Invoke-StageFourCommand `
        'phase-close-after-gate' `
        'REQUEST_PHASE_COMPLETION' `
        'agent:opa-stage4' `
        $false `
        @('phase', 'request-close', 'phase:operational-acceptance') `
        6 `
        $false

    $auxiliaryAuthorityPath = Join-Path $fixtureDirectory 'authority-stage4-material-change.md'
    [IO.File]::WriteAllText(
        $auxiliaryAuthorityPath,
        "# Stage 4 material auxiliary authority`n`nThis disposable record forces explicit gate and checkpoint revalidation.`n",
        [Text.UTF8Encoding]::new($false)
    )
    $auxiliaryAuthorityDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $auxiliaryAuthorityPath).Hash.ToLowerInvariant()
    $authorityChange = Invoke-StageFourCommand `
        'authority-material-change' `
        'REGISTER_AUTHORITY_DOCUMENT' `
        'human:owner' `
        $true `
        @(
            'authority', 'add', '--uri', $auxiliaryAuthorityPath,
            '--digest', $auxiliaryAuthorityDigest,
            '--record-id', 'authority:opa-stage4-material-change'
        ) `
        0 `
        $true
    $gateAfterAuthorityChange = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-04',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $resumeAfterAuthorityChange = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )

    $revalidationSignalPath = Join-Path $fixtureDirectory 'stage-4-revalidation-signal.json'
    Write-PilotJson $revalidationSignalPath ([ordered]@{
        schema_version = '0.3.0'
        gate_ref = 'gate:opa-04'
        reason = 'Stage 4 material auxiliary authority was registered and inspected.'
        evidence_refs = @('evidence:opa-stage4-verified-source')
        observed_at = $observedAt
    })
    $revalidatedGateResult = Invoke-StageFourCommand `
        'gate-revalidate-authority' `
        'REQUEST_GATE_REVALIDATION' `
        'agent:opa-stage4' `
        $false `
        @('gate', 'request-revalidation', 'gate:opa-04', '--signal', $revalidationSignalPath) `
        0 `
        $true
    $gateAfterRevalidation = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-04',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )

    $beforeCriticalWaiver = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $criticalWaiver = Invoke-StageFourCommand `
        'waiver-critical-rejected' `
        'WAIVE_GATE' `
        'human:owner' `
        $true `
        @(
            'gate', 'waive', 'gate:opa-08', '--reason', 'Negative Stage 4 waiver-policy exercise',
            '--approved-by', 'human:owner', '--authority', 'authority:opa-spec'
        ) `
        3 `
        $false
    $criticalGate = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-08',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $optionalWaiver = Invoke-StageFourCommand `
        'waiver-optional' `
        'WAIVE_GATE' `
        'human:owner' `
        $true `
        @(
            'gate', 'waive', 'gate:opa-waiver-exercise',
            '--reason', 'Stage 4 optional waiver exercise only',
            '--approved-by', 'human:owner', '--authority', 'authority:opa-spec'
        ) `
        0 `
        $true
    $optionalGate = Invoke-PilotLedgerJson $ledger @(
        'gate', 'show', 'gate:opa-waiver-exercise',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )

    $null = Invoke-StageFourCommand `
        'unknown-stage4-resolve' `
        'RESOLVE_UNKNOWN' `
        'human:owner' `
        $true `
        @(
            'unknown', 'resolve', 'unknown:opa-stage4-provider-policy',
            '--resolution', 'urn:g-ledger:pilot:stage4:recordsink'
        ) `
        0 `
        $true
    $null = Invoke-StageFourCommand `
        'blocker-stage4-resolve' `
        'RESOLVE_BLOCKER' `
        'human:owner' `
        $true `
        @(
            'blocker', 'resolve', 'blocker:opa-stage4-recordsink',
            '--summary', 'Local RecordSink policy and lifecycle boundaries passed Stage 4.',
            '--evidence', 'evidence:opa-stage4-verified-source'
        ) `
        0 `
        $true
    $null = Invoke-StageFourCommand `
        'next-stage5-set' `
        'SET_NEXT_ACTION' `
        'human:owner' `
        $true `
        @(
            'next', 'set', '--title', 'Request authorization for Stage 5 transactional semantics',
            '--summary', 'Stage 5 remains outside the current authorization.',
            '--phase', 'phase:operational-acceptance', '--classification', 'RECOMMENDED',
            '--record-id', 'action:opa-stage5-transaction'
        ) `
        0 `
        $true
    $null = Invoke-StageFourCommand `
        'blocker-stage5-open' `
        'OPEN_BLOCKER' `
        'human:owner' `
        $true `
        @(
            'blocker', 'open', '--title', 'Stage 5 requires separate human authorization',
            '--summary', 'Idempotency, concurrency, and uncertain-outcome tests are not authorized by Stage 4.',
            '--severity', 'MEDIUM', '--record-id', 'blocker:opa-stage5-transaction'
        ) `
        0 `
        $true
    $replacementCheckpointResult = Invoke-StageFourCommand `
        'checkpoint-replacement' `
        'CREATE_CONTINUITY_CHECKPOINT' `
        'agent:opa-stage4' `
        $false `
        @('checkpoint', 'create', '--reason', 'Stage 4 evidence and lifecycle handoff') `
        0 `
        $true
    $finalResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    $finalState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $phaseList = @(Invoke-PilotLedgerJson $ledger @(
        'phase', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $activePhase = $phaseList | Where-Object { $_.phase_ref -eq 'phase:operational-acceptance' }
    $criticalAttemptDelta = [int64]$criticalWaiver.state.state_version - [int64]$beforeCriticalWaiver.state_version
    if ($observedEvidence.status -ne 'OBSERVED' -or
        $observedEvidence.result.exit_code -ne 0 -or
        $gateAfterObserved.status -eq 'VERIFIED' -or
        $phaseAfterObserved.state.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        $gateAfterVerified.status -ne 'VERIFIED' -or
        $phaseAfterVerified.state.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($phaseAfterVerified.state.completed_phase_refs).Count -ne 0 -or
        $gateAfterAuthorityChange.status -ne 'REVALIDATION_REQUIRED' -or
        $resumeAfterAuthorityChange.checkpoint_freshness -ne 'STALE_FOR_AUTHORITY' -or
        $gateAfterRevalidation.status -ne 'VERIFIED' -or
        $criticalGate.status -ne 'PENDING' -or
        $criticalAttemptDelta -ne 0 -or
        $optionalGate.status -ne 'WAIVED' -or
        $finalResume.checkpoint_freshness -ne 'CURRENT' -or
        $finalState.project.lifecycle_status -ne 'BLOCKED' -or
        $finalState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($finalState.completed_phase_refs).Count -ne 0 -or
        $finalState.preferred_next_action_ref -ne 'action:opa-stage5-transaction' -or
        @($finalState.open_blocker_refs) -notcontains 'blocker:opa-stage5-transaction' -or
        @($activePhase).Count -ne 1 -or $activePhase.status -ne 'ACTIVE') {
        throw 'PILOT_STAGE_4_FINAL_STATE_MISMATCH'
    }

    $manifest = [ordered]@{
        profile = 'project-ledger-operational-pilot-stage/1'
        stage = 4
        status = 'EVIDENCE_LIFECYCLE_PASS'
        proof_class = 'LIVE_PILOT'
        project_ref = $projectRef
        subject = [ordered]@{
            zip_sha256 = $actualZipHash
            source_commit = $ExpectedSourceCommit
            version = $stageOneManifest.subject.version
        }
        observed_evidence = [ordered]@{
            evidence_ref = $observedEvidenceRef
            status = $observedEvidence.status
            execution_exit_code = $observedEvidence.result.exit_code
            gate_evaluation_exit_code = 6
            gate_verified = $gateAfterObserved.status -eq 'VERIFIED'
            phase_close_exit_code = 6
            phase_remained_active = $phaseAfterObserved.state.primary_active_phase_ref -eq 'phase:operational-acceptance'
        }
        record_sink = [ordered]@{
            profile = 'gskills-context-provider/0.3.0'
            policy_digest = $recordSinkPolicy.digest
            validate_status = $validateResult.status
            submit_status = $submitResult.status
            replay_status = $replayResult.status
            status_lookup = $statusResult.status
            reconcile_lookup = $reconcileResult.status
            state_version_delta = [int64]$afterSink.state_version - [int64]$beforeSink.state_version
            replay_state_version_delta = [int64]$afterSink.state_version -
                [int64]$submitResult.state_transition.new_state_version
            evidence_ref = $verifiedEvidenceView.evidence_id
            evidence_status = $verifiedEvidenceView.status
            raw_output_stored = $verifiedEvidenceView.privacy.raw_output_stored
            native_result_digest = $submitResult.native_result_digest
            client = 'scripts/invoke-record-sink-rpc.ps1'
            distributed_client = $false
        }
        lifecycle = [ordered]@{
            gate_opa_04_after_verified_evidence = $gateAfterVerified.status
            phase_close_after_gate_exit_code = 6
            phase_status = $activePhase.status
            completed_phase_count = @($finalState.completed_phase_refs).Count
            external_lifecycle_event_count = 0
        }
        freshness = [ordered]@{
            auxiliary_authority_ref = 'authority:opa-stage4-material-change'
            auxiliary_authority_digest = $auxiliaryAuthorityDigest
            gate_status_after_authority_change = $gateAfterAuthorityChange.status
            checkpoint_after_authority_change = $resumeAfterAuthorityChange.checkpoint_freshness
            gate_status_after_revalidation = $gateAfterRevalidation.status
            replacement_checkpoint_ref = $finalResume.checkpoint_ref
            replacement_checkpoint = $finalResume.checkpoint_freshness
        }
        waiver = [ordered]@{
            optional_gate_ref = 'gate:opa-waiver-exercise'
            optional_gate_status = $optionalGate.status
            critical_gate_ref = 'gate:opa-08'
            critical_waiver_exit_code = 3
            critical_gate_status = $criticalGate.status
            critical_attempt_state_version_delta = $criticalAttemptDelta
        }
        boundary = [ordered]@{
            project_status = $finalState.project.lifecycle_status
            active_phase_ref = $finalState.primary_active_phase_ref
            open_blocker_ref = 'blocker:opa-stage5-transaction'
            next_action_ref = $finalState.preferred_next_action_ref
            stage_5_executed = $false
            merge_performed = $false
            release_performed = $false
            operational_acceptance_declared = $false
        }
        lifecycle_authority = [ordered]@{
            gate_engine_only = $true
            phase_engine_only = $true
            record_sink_claims_lifecycle_authority = $false
        }
        mutations = @($script:stageFourMutations)
        scenarios = [ordered]@{
            'OPA-S11' = 'PASS'
            'OPA-S12' = 'PASS'
            'OPA-S13' = 'PASS'
            'OPA-S14' = 'PASS'
            'OPA-S15' = 'PASS'
        }
    }
    Write-PilotJson $stageFourManifestPath $manifest

    [pscustomobject]@{
        stage = 4
        status = 'EVIDENCE_LIFECYCLE_PASS'
        evidence = 'evidence/stage-4-evidence-lifecycle.json'
    }
    return
}

if ($Stage -eq 3) {
    $resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
    $resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
    $resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)
    if (-not (Test-Path -LiteralPath $resolvedPilotRoot -PathType Container)) {
        throw "PILOT_STAGE_1_REQUIRED: $resolvedPilotRoot"
    }

    $evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
    $fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
    $stageOneManifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
    $stageTwoManifestPath = Join-Path $evidenceDirectory 'stage-2-governed-model.json'
    $stageThreeManifestPath = Join-Path $evidenceDirectory 'stage-3-high-level-continuity.json'
    $locatorPath = Join-Path $resolvedPilotRoot 'pilot-locator.json'
    $installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
    $ledger = Join-Path $installation 'project-ledger.exe'
    $packageManifestPath = Join-Path $installation 'package.json'
    foreach ($requiredPath in @(
        $stageOneManifestPath,
        $stageTwoManifestPath,
        $locatorPath,
        $ledger,
        $packageManifestPath
    )) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_2_EVIDENCE_MISSING: $requiredPath"
        }
    }
    if (Test-Path -LiteralPath $stageThreeManifestPath) {
        throw "PILOT_STAGE_ALREADY_EXECUTED: $stageThreeManifestPath"
    }

    $actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
    if ($actualZipHash -ne $ExpectedZipSha256.ToLowerInvariant()) {
        throw "PILOT_ZIP_HASH_MISMATCH: expected $($ExpectedZipSha256.ToLowerInvariant()); actual $actualZipHash"
    }
    $stageOneManifest = Get-Content -Raw -LiteralPath $stageOneManifestPath | ConvertFrom-Json
    $stageTwoManifest = Get-Content -Raw -LiteralPath $stageTwoManifestPath | ConvertFrom-Json
    $packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
    if ($stageOneManifest.status -ne 'PREFLIGHT_PASS' -or
        $stageTwoManifest.status -ne 'GOVERNED_MODEL_PASS' -or
        $stageOneManifest.subject.zip_sha256 -ne $actualZipHash -or
        $stageOneManifest.subject.source_commit -ne $ExpectedSourceCommit -or
        $packageManifest.source_commit -ne $ExpectedSourceCommit) {
        throw 'PILOT_STAGE_2_IDENTITY_MISMATCH'
    }

    $locator = Get-Content -Raw -LiteralPath $locatorPath | ConvertFrom-Json
    $database = [string]$locator.database
    $projectRef = [string]$locator.project_ref
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'PILOT_DATABASE_MISSING'
    }
    $initialState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($initialState.state_version -ne 21 -or
        $initialState.state_digest -ne $stageTwoManifest.lifecycle.state_digest -or
        $initialState.project.lifecycle_status -ne 'ACTIVE' -or
        $initialState.primary_active_phase_ref -ne 'phase:operational-acceptance') {
        throw 'PILOT_STAGE_3_INITIAL_STATE_MISMATCH'
    }

    $observedAt = [DateTimeOffset]::UtcNow.ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $script:stageThreeStateVersion = 21
    $script:stageThreeMutations = @()
    function Invoke-StageThreeMutation {
        param(
            [string]$Name,
            [string]$CommandType,
            [string]$ActorId,
            [bool]$AsHuman,
            [string[]]$CommandArguments
        )

        $expectedVersion = $script:stageThreeStateVersion
        $arguments = @($CommandArguments) + @(
            '--database', $database,
            '--project-ref', $projectRef,
            '--expected-version', [string]$expectedVersion,
            '--idempotency-key', ('opa:stage3:' + $Name),
            '--actor-id', $ActorId,
            '--observed-at', $observedAt
        )
        if ($AsHuman) { $arguments += '--as-human' }
        $arguments += '--json'
        $result = Invoke-PilotLedgerJson $ledger $arguments
        if ($result.status -ne 'ACCEPTED' -or
            [int64]$result.new_state_version -le $expectedVersion -or
            @($result.emitted_event_refs).Count -eq 0) {
            throw "PILOT_STAGE_3_MUTATION_RESULT_INVALID: $Name"
        }
        $script:stageThreeStateVersion = [int64]$result.new_state_version
        $script:stageThreeMutations += [ordered]@{
            name = $Name
            command_type = $CommandType
            actor_id = $ActorId
            as_human = $AsHuman
            expected_state_version = $expectedVersion
            resulting_state_version = $script:stageThreeStateVersion
            idempotency_key = 'opa:stage3:' + $Name
            emitted_event_refs = @($result.emitted_event_refs)
        }
        return $result
    }

    function New-StageThreeDecision {
        param(
            [string]$DecisionId,
            [string]$Title,
            [string]$DecisionText,
            [int]$Version,
            [AllowNull()][object]$SupersedesRef
        )

        $normalizedSupersedesRef = if ([string]::IsNullOrWhiteSpace([string]$SupersedesRef)) {
            $null
        } else {
            [string]$SupersedesRef
        }
        $withoutDigest = [ordered]@{
            schema_version = '0.2.0'
            decision_id = $DecisionId
            title = $Title
            domain = 'OPERATIONAL_PILOT_CONTINUITY'
            scope = 'stage:3'
            context = 'Validate high-level continuity without granting lifecycle completion.'
            decision = $DecisionText
            rationale = 'Keep the next stage explicit, bounded, and subject to separate human authority.'
            alternatives = @('Advance automatically', 'Stop without a recorded next action')
            status = 'PROPOSED'
            version = $Version
            authority_ref = $null
            evidence_refs = @()
            supersedes_ref = $normalizedSupersedesRef
            created_at = $observedAt
            accepted_at = $null
            provenance_ref = 'urn:project-ledger:pilot:stage3'
        }
        $decision = [ordered]@{}
        foreach ($key in $withoutDigest.Keys) { $decision[$key] = $withoutDigest[$key] }
        $decision['digest'] = Get-PilotCanonicalDigest $withoutDigest
        return $decision
    }

    $sessionPath = Join-Path $fixtureDirectory 'stage-3-session.json'
    Write-PilotJson $sessionPath ([ordered]@{
        schema_version = '0.2.0'
        session_id = 'session:opa-stage3'
        actor_ref = 'agent:opa-stage3'
        host = 'windows-operational-pilot'
        started_at = $observedAt
        ended_at = $null
        starting_checkpoint_ref = $null
        ending_checkpoint_ref = $null
    })
    $null = Invoke-StageThreeMutation 'session:start' 'START_SESSION' 'agent:opa-stage3' $false @(
        'session', 'start', '--file', $sessionPath
    )
    $null = Invoke-StageThreeMutation 'progress' 'RECORD_WORK_PROGRESS' 'agent:opa-stage3' $false @(
        'record', 'progress', '--summary', 'Stage 3 records material continuity semantics against the exact hosted candidate.',
        '--phase', 'phase:operational-acceptance', '--record-id', 'work:opa-stage3-progress'
    )
    $null = Invoke-StageThreeMutation 'note:limitation' 'RECORD_NOTE' 'agent:opa-stage3' $false @(
        'record', 'note', '--kind', 'LIMITATION',
        '--text', 'This pilot proves supported projections; it does not prove absence from every internal byte surface.',
        '--phase', 'phase:operational-acceptance'
    )
    $null = Invoke-StageThreeMutation 'claim:asserted' 'RECORD_CLAIM' 'agent:opa-stage3' $false @(
        'claim', 'record', '--statement', 'The hosted candidate preserves high-level project continuity.',
        '--classification', 'ASSERTED', '--subject', $projectRef, '--record-id', 'claim:opa-stage3-continuity'
    )
    $null = Invoke-StageThreeMutation 'unknown:transition' 'RECORD_UNKNOWN' 'agent:opa-stage3' $false @(
        'unknown', 'record', '--subject', $projectRef,
        '--statement', 'The durable accepted replacement shape needed by decision supersession is not yet observed.',
        '--criticality', 'MATERIAL', '--record-id', 'unknown:opa-stage3-replacement-shape'
    )
    $null = Invoke-StageThreeMutation 'unknown:open' 'RECORD_UNKNOWN' 'agent:opa-stage3' $false @(
        'unknown', 'record', '--subject', $projectRef,
        '--statement', 'Stage 4 RecordSink policy behavior has not yet been observed in LIVE_PILOT.',
        '--criticality', 'MATERIAL', '--record-id', 'unknown:opa-stage4-provider-policy'
    )

    $decisionV1Path = Join-Path $fixtureDirectory 'stage-3-decision-v1.json'
    $decisionV1 = New-StageThreeDecision `
        'decision:opa-stage3-sequence-v1' `
        'Require separate authority for Stage 4' `
        'Do not execute Stage 4 without a separate human authorization.' `
        1 `
        $null
    Write-PilotJson $decisionV1Path $decisionV1
    $null = Invoke-StageThreeMutation 'decision:v1:propose' 'PROPOSE_DECISION' 'human:owner' $true @(
        'decision', 'propose', '--file', $decisionV1Path
    )
    $null = Invoke-StageThreeMutation 'decision:v1:accept' 'ACCEPT_DECISION' 'human:owner' $true @(
        'decision', 'accept', 'decision:opa-stage3-sequence-v1', '--authority', 'authority:opa-spec'
    )

    $decisionV2Path = Join-Path $fixtureDirectory 'stage-3-decision-v2.json'
    $decisionV2 = New-StageThreeDecision `
        'decision:opa-stage3-sequence-v2' `
        'Prepare but do not execute Stage 4' `
        'Prepare Stage 4 inputs, but require separate human authorization and a fresh preflight before execution.' `
        2 `
        'decision:opa-stage3-sequence-v1'
    Write-PilotJson $decisionV2Path $decisionV2
    $null = Invoke-StageThreeMutation 'decision:v2:propose' 'PROPOSE_DECISION' 'human:owner' $true @(
        'decision', 'propose', '--file', $decisionV2Path
    )
    $null = Invoke-StageThreeMutation 'decision:v2:accept' 'ACCEPT_DECISION' 'human:owner' $true @(
        'decision', 'accept', 'decision:opa-stage3-sequence-v2', '--authority', 'authority:opa-spec'
    )
    $acceptedReplacement = Invoke-PilotLedgerJson $ledger @(
        'decision', 'show', 'decision:opa-stage3-sequence-v2',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    Write-PilotJson $decisionV2Path $acceptedReplacement

    $null = Invoke-StageThreeMutation 'blocker:transition' 'OPEN_BLOCKER' 'human:owner' $true @(
        'blocker', 'open', '--title', 'Accepted replacement snapshot not yet persisted',
        '--summary', 'Decision supersession must use the exact accepted durable view.',
        '--severity', 'MEDIUM', '--record-id', 'blocker:opa-stage3-replacement-snapshot'
    )
    $null = Invoke-StageThreeMutation 'blocker:open' 'OPEN_BLOCKER' 'human:owner' $true @(
        'blocker', 'open', '--title', 'Stage 4 requires separate human authorization',
        '--summary', 'The evidence and lifecycle stage remains outside the current authorization.',
        '--severity', 'MEDIUM', '--record-id', 'blocker:opa-stage4-recordsink'
    )
    $null = Invoke-StageThreeMutation 'risk:open' 'OPEN_RISK' 'human:owner' $true @(
        'risk', 'open', '--title', 'Bounded resume may omit required continuity semantics',
        '--summary', 'A compact handoff could omit a phase, gate, decision, blocker, risk, unknown, checkpoint, or next action.',
        '--severity', 'HIGH', '--record-id', 'risk:opa-stage3-continuity'
    )
    $null = Invoke-StageThreeMutation 'next:set' 'SET_NEXT_ACTION' 'human:owner' $true @(
        'next', 'set', '--title', 'Request authorization for Stage 4 evidence and lifecycle validation',
        '--summary', 'After Stage 3 closes, request explicit human authority before executing Stage 4.',
        '--phase', 'phase:operational-acceptance', '--classification', 'PROPOSED',
        '--record-id', 'action:opa-stage4-evidence'
    )
    $null = Invoke-StageThreeMutation 'unknown:resolve' 'RESOLVE_UNKNOWN' 'human:owner' $true @(
        'unknown', 'resolve', 'unknown:opa-stage3-replacement-shape',
        '--resolution', 'urn:project-ledger:pilot:stage3:accepted-replacement-view'
    )
    $null = Invoke-StageThreeMutation 'blocker:resolve' 'RESOLVE_BLOCKER' 'human:owner' $true @(
        'blocker', 'resolve', 'blocker:opa-stage3-replacement-snapshot',
        '--summary', 'The accepted replacement was queried and persisted as the exact supersession payload.'
    )
    $null = Invoke-StageThreeMutation 'risk:mitigate' 'MITIGATE_RISK' 'human:owner' $true @(
        'risk', 'mitigate', 'risk:opa-stage3-continuity',
        '--note', 'Require exact reference assertions in both structured JSON and bounded Codex resume output.'
    )
    $null = Invoke-StageThreeMutation 'decision:supersede' 'SUPERSEDE_DECISION' 'human:owner' $true @(
        'decision', 'supersede', 'decision:opa-stage3-sequence-v1', '--with', $decisionV2Path
    )

    $null = Invoke-StageThreeMutation 'repository:revoke-canary' 'REVOKE_REPOSITORY_BINDING' 'human:owner' $true @(
        'repository', 'revoke', 'binding:opa-stage2-canary',
        '--reason', 'Stage 2 canary completed; retire auxiliary binding before continuity checkpoint.'
    )
    $repositoryHeadOutput = & git -C $resolvedRepository rev-parse HEAD 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "PILOT_REPOSITORY_HEAD_UNAVAILABLE: $(($repositoryHeadOutput | Out-String).Trim())"
    }
    $repositoryHead = ($repositoryHeadOutput | Out-String).Trim()
    if ($repositoryHead -notmatch '^[0-9a-f]{40}$') {
        throw "PILOT_REPOSITORY_HEAD_INVALID: $repositoryHead"
    }
    $repositorySnapshotDigest = Get-PilotTextDigest $repositoryHead
    $null = Invoke-StageThreeMutation 'repository:refresh-main' 'REFRESH_REPOSITORY_BINDING' 'human:owner' $true @(
        'repository', 'refresh', 'binding:opa-repository',
        '--snapshot-ref', ('urn:git:commit:' + $repositoryHead), '--snapshot-digest', $repositorySnapshotDigest
    )

    $checkpointResult = Invoke-StageThreeMutation 'checkpoint:create' 'CREATE_CONTINUITY_CHECKPOINT' 'agent:opa-stage3' $false @(
        'checkpoint', 'create', '--reason', 'Stage 3 high-level continuity handoff'
    )
    $checkpointResume = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    if ($checkpointResume.checkpoint_freshness -ne 'CURRENT') {
        $defectResult = Invoke-StageThreeMutation 'defect:checkpoint-authority-history' 'OPEN_BLOCKER' 'human:owner' $true @(
            'blocker', 'open', '--title', 'Fresh checkpoint is stale because superseded authority history exists',
            '--summary', 'A checkpoint created after the authority replacement is classified stale instead of current.',
            '--severity', 'CRITICAL', '--record-id', 'blocker:opa-defect-checkpoint-authority-history'
        )
        $blockedState = Invoke-PilotLedgerJson $ledger @(
            'status', '--database', $database, '--project-ref', $projectRef, '--json'
        )
        $blockedManifest = [ordered]@{
            profile = 'project-ledger-operational-pilot-stage/1'
            stage = 3
            status = 'BLOCKED'
            proof_class = 'LIVE_PILOT'
            project_ref = $projectRef
            subject = [ordered]@{
                zip_sha256 = $actualZipHash
                source_commit = $ExpectedSourceCommit
                version = $stageOneManifest.subject.version
            }
            lifecycle = [ordered]@{
                initial_state_version = $initialState.state_version
                final_state_version = $blockedState.state_version
                final_state_digest = $blockedState.state_digest
                project_status = $blockedState.project.lifecycle_status
                active_phase_ref = $blockedState.primary_active_phase_ref
            }
            checkpoint = [ordered]@{
                checkpoint_ref = $checkpointResume.checkpoint_ref
                observed_freshness = $checkpointResume.checkpoint_freshness
                expected_freshness = 'CURRENT'
                session_closed = $false
            }
            defect = [ordered]@{
                defect_id = 'OPA-DEFECT-003'
                blocker_ref = 'blocker:opa-defect-checkpoint-authority-history'
                severity = 'CRITICAL'
                interpretation = 'A newly created checkpoint treats preserved SUPERSEDED authority history as stale authority.'
                expected = 'Historical SUPERSEDED authority remains queryable but does not stale a checkpoint created after replacement.'
                observed = $checkpointResume.checkpoint_freshness
                command_result_state_version = $defectResult.new_state_version
            }
            mutations = @($script:stageThreeMutations)
            scenarios = [ordered]@{
                'OPA-S08' = 'PASS'
                'OPA-S09' = 'PASS'
                'OPA-S10' = 'BLOCKED'
            }
        }
        Write-PilotJson $stageThreeManifestPath $blockedManifest
        throw "PILOT_STAGE_3_BLOCKED: OPA-DEFECT-003 checkpoint freshness=$($checkpointResume.checkpoint_freshness)"
    }

    $endSessionPath = Join-Path $fixtureDirectory 'stage-3-end-session.json'
    Write-PilotJson $endSessionPath ([ordered]@{
        session_ref = 'session:opa-stage3'
        create_checkpoint = $false
    })
    $sessionEndResult = Invoke-StageThreeMutation 'session:end' 'END_SESSION' 'agent:opa-stage3' $false @(
        'session', 'end', '--file', $endSessionPath
    )

    $resumeJson = Invoke-PilotLedgerJson $ledger @(
        'resume', '--database', $database, '--project-ref', $projectRef, '--format', 'json'
    )
    $codexLines = & $ledger resume `
        --database $database --project-ref $projectRef --format codex 2>&1
    $codexExit = $LASTEXITCODE
    $codexText = ($codexLines | Out-String).Trim()
    if ($codexExit -ne 0 -or [string]::IsNullOrWhiteSpace($codexText)) {
        throw "PILOT_STAGE_3_CODEX_RESUME_FAILED: exit=$codexExit"
    }

    $originalDecision = Invoke-PilotLedgerJson $ledger @(
        'decision', 'show', 'decision:opa-stage3-sequence-v1',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $replacementDecision = Invoke-PilotLedgerJson $ledger @(
        'decision', 'show', 'decision:opa-stage3-sequence-v2',
        '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $renderPath = Join-Path $evidenceDirectory 'stage-3-continuity-view.md'
    $renderRecord = Invoke-PilotLedgerJson $ledger @(
        'render', '--database', $database, '--project-ref', $projectRef,
        '--output', $renderPath, '--privacy', 'full-local', '--json'
    )
    $renderedText = Get-Content -Raw -LiteralPath $renderPath
    $requiredTimelineEvents = @(
        'UNKNOWN_RESOLVED',
        'BLOCKER_RESOLVED',
        'RISK_MITIGATED',
        'DECISION_SUPERSEDED'
    )
    $timelineContains = @($requiredTimelineEvents | Where-Object { $renderedText.Contains($_) })

    $finalState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $gates = @(Invoke-PilotLedgerJson $ledger @(
        'gate', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $allGatesUnverified = @($gates | Where-Object { $_.status -in @('VERIFIED', 'WAIVED') }).Count -eq 0
    $codexRequiredLabels = @(
        '- Active:',
        '- Decisions:',
        '- Blockers:',
        '- Risks:',
        '- Gates:',
        '- Progress:',
        '- Next:',
        '- Critical unknowns:',
        '- Checkpoint:'
    )
    $codexSectionsPresent = @($codexRequiredLabels | Where-Object { $codexText.Contains($_) }).Count -eq
        $codexRequiredLabels.Count
    $resumeAssertions = [ordered]@{
        active_phase_present = @($resumeJson.active_phase_refs) -contains 'phase:operational-acceptance'
        pending_gates_present = @($resumeJson.pending_gate_refs).Count -eq 9
        current_decision_present = @($resumeJson.current_decision_refs) -contains 'decision:opa-stage3-sequence-v2'
        open_blocker_present = @($resumeJson.open_blocker_refs) -contains 'blocker:opa-stage4-recordsink'
        open_risk_present = @($resumeJson.open_risk_refs) -contains 'risk:opa-stage3-continuity'
        critical_unknown_present = @($resumeJson.critical_unknown_refs) -contains 'unknown:opa-stage4-provider-policy'
        checkpoint_present = -not [string]::IsNullOrWhiteSpace([string]$resumeJson.checkpoint_ref)
        next_action_present = $resumeJson.preferred_next_action_ref -eq 'action:opa-stage4-evidence'
        progress_present = [string]$resumeJson.last_material_progress_summary -like 'Stage 3 records material*'
    }
    $resumePass = @($resumeAssertions.Values | Where-Object { $_ -ne $true }).Count -eq 0
    if ($finalState.state_version -ne 44 -or
        $finalState.project.lifecycle_status -ne 'BLOCKED' -or
        $finalState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($finalState.completed_phase_refs).Count -ne 0 -or
        -not $allGatesUnverified -or
        $resumeJson.state_version -ne $finalState.state_version -or
        $resumeJson.checkpoint_freshness -ne 'CURRENT' -or
        -not $resumePass -or
        -not $codexSectionsPresent -or
        $originalDecision.status -ne 'SUPERSEDED' -or
        $replacementDecision.status -ne 'ACCEPTED' -or
        $replacementDecision.supersedes_ref -ne $originalDecision.decision_id -or
        $timelineContains.Count -ne $requiredTimelineEvents.Count) {
        throw 'PILOT_STAGE_3_FINAL_STATE_MISMATCH'
    }

    $manifest = [ordered]@{
        profile = 'project-ledger-operational-pilot-stage/1'
        stage = 3
        status = 'HIGH_LEVEL_CONTINUITY_PASS'
        proof_class = 'LIVE_PILOT'
        project_ref = $projectRef
        subject = [ordered]@{
            zip_sha256 = $actualZipHash
            source_commit = $ExpectedSourceCommit
            version = $stageOneManifest.subject.version
        }
        records = [ordered]@{
            progress = [ordered]@{ record_ref = 'work:opa-stage3-progress'; material = $true }
            note = [ordered]@{ kind = 'LIMITATION'; scope_ref = 'phase:operational-acceptance' }
            claim = [ordered]@{ record_ref = 'claim:opa-stage3-continuity'; initial_classification = 'ASSERTED' }
            open_unknown = [ordered]@{ record_ref = 'unknown:opa-stage4-provider-policy'; criticality = 'MATERIAL' }
            open_blocker = [ordered]@{ record_ref = 'blocker:opa-stage4-recordsink'; severity = 'MEDIUM' }
            mitigated_risk = [ordered]@{ record_ref = 'risk:opa-stage3-continuity'; status = 'MITIGATED' }
            next_action = [ordered]@{ record_ref = 'action:opa-stage4-evidence'; classification = 'PROPOSED' }
        }
        transitions = [ordered]@{
            unknown = [ordered]@{ record_ref = 'unknown:opa-stage3-replacement-shape'; status = 'RESOLVED' }
            blocker = [ordered]@{ record_ref = 'blocker:opa-stage3-replacement-snapshot'; status = 'RESOLVED' }
            risk = [ordered]@{ record_ref = 'risk:opa-stage3-continuity'; status = 'MITIGATED' }
            decision = [ordered]@{
                original_ref = $originalDecision.decision_id
                original_status = $originalDecision.status
                replacement_ref = $replacementDecision.decision_id
                replacement_status = $replacementDecision.status
                predecessor_preserved = $replacementDecision.supersedes_ref -eq $originalDecision.decision_id
            }
            timeline_contains_all = @($timelineContains)
            rendered_view_ref = $renderRecord.generated_artifact_id
            rendered_view_sha256 = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $renderPath).Hash.ToLowerInvariant()
        }
        repository_checkpoint_precondition = [ordered]@{
            human_revoke_binding_ref = 'binding:opa-stage2-canary'
            human_actor = 'human:owner'
            main_binding_ref = 'binding:opa-repository'
            snapshot_ref = 'urn:git:commit:' + $repositoryHead
            snapshot_digest = $repositorySnapshotDigest
        }
        checkpoint = [ordered]@{
            checkpoint_ref = $resumeJson.checkpoint_ref
            freshness = $resumeJson.checkpoint_freshness
            create_result_ref = $checkpointResult.command_ref
            session_ref = 'session:opa-stage3'
            session_closed = $sessionEndResult.status -eq 'ACCEPTED'
        }
        resume = [ordered]@{
            json = [ordered]@{
                state_version = $resumeJson.state_version
                digest = $resumeJson.digest
                active_phase_present = $resumeAssertions.active_phase_present
                pending_gates_present = $resumeAssertions.pending_gates_present
                current_decision_present = $resumeAssertions.current_decision_present
                open_blocker_present = $resumeAssertions.open_blocker_present
                open_risk_present = $resumeAssertions.open_risk_present
                critical_unknown_present = $resumeAssertions.critical_unknown_present
                checkpoint_present = $resumeAssertions.checkpoint_present
                next_action_present = $resumeAssertions.next_action_present
                progress_present = $resumeAssertions.progress_present
            }
            codex = [ordered]@{
                output_sha256 = Get-PilotTextDigest $codexText
                all_required_sections_present = $codexSectionsPresent
                required_labels = $codexRequiredLabels
            }
        }
        lifecycle = [ordered]@{
            initial_state_version = $initialState.state_version
            final_state_version = $finalState.state_version
            final_state_digest = $finalState.state_digest
            project_status = $finalState.project.lifecycle_status
            active_phase_ref = $finalState.primary_active_phase_ref
            completed_phase_count = @($finalState.completed_phase_refs).Count
            all_gates_unverified = $allGatesUnverified
        }
        mutations = @($script:stageThreeMutations)
        scenarios = [ordered]@{
            'OPA-S08' = 'PASS'
            'OPA-S09' = 'PASS'
            'OPA-S10' = 'PASS'
        }
    }
    Write-PilotJson $stageThreeManifestPath $manifest

    [pscustomobject]@{
        stage = 3
        status = 'HIGH_LEVEL_CONTINUITY_PASS'
        evidence = 'evidence/stage-3-high-level-continuity.json'
    }
    return
}

if ($Stage -eq 2) {
    $resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
    $resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
    $resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)
    if (-not (Test-Path -LiteralPath $resolvedPilotRoot -PathType Container)) {
        throw "PILOT_STAGE_1_REQUIRED: $resolvedPilotRoot"
    }

    $evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
    $fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
    $stageOneManifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
    $stageTwoManifestPath = Join-Path $evidenceDirectory 'stage-2-governed-model.json'
    $locatorPath = Join-Path $resolvedPilotRoot 'pilot-locator.json'
    $installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
    $ledger = Join-Path $installation 'project-ledger.exe'
    $packageManifestPath = Join-Path $installation 'package.json'
    foreach ($requiredPath in @($stageOneManifestPath, $locatorPath, $ledger, $packageManifestPath)) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "PILOT_STAGE_1_EVIDENCE_MISSING: $requiredPath"
        }
    }
    if (Test-Path -LiteralPath $stageTwoManifestPath) {
        throw "PILOT_STAGE_ALREADY_EXECUTED: $stageTwoManifestPath"
    }

    $actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
    if ($actualZipHash -ne $ExpectedZipSha256.ToLowerInvariant()) {
        throw "PILOT_ZIP_HASH_MISMATCH: expected $($ExpectedZipSha256.ToLowerInvariant()); actual $actualZipHash"
    }
    $stageOneManifest = Get-Content -Raw -LiteralPath $stageOneManifestPath | ConvertFrom-Json
    $packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
    if ($stageOneManifest.status -ne 'PREFLIGHT_PASS' -or
        $stageOneManifest.subject.zip_sha256 -ne $actualZipHash -or
        $packageManifest.source_commit -ne $ExpectedSourceCommit -or
        $stageOneManifest.subject.source_commit -ne $ExpectedSourceCommit) {
        throw 'PILOT_STAGE_1_IDENTITY_MISMATCH'
    }

    $locator = Get-Content -Raw -LiteralPath $locatorPath | ConvertFrom-Json
    $database = [string]$locator.database
    $projectRef = [string]$locator.project_ref
    if (-not (Test-Path -LiteralPath $database -PathType Leaf)) {
        throw 'PILOT_DATABASE_MISSING'
    }

    $readQueries = @()
    function Invoke-PilotReadQuery {
        param([string]$Name, [string[]]$CommandArguments)

        $value = Invoke-PilotLedgerJson $ledger $CommandArguments
        $json = $value | ConvertTo-Json -Depth 20 -Compress
        $script:readQueries += [ordered]@{
            name = $Name
            exit_code = 0
            output_sha256 = Get-PilotTextDigest $json
        }
        return $value
    }

    $beforeState = Invoke-PilotReadQuery 'status' @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($beforeState.state_version -ne 15 -or $beforeState.project.lifecycle_status -ne 'ACTIVE') {
        throw 'PILOT_STAGE_2_INITIAL_STATE_MISMATCH'
    }
    $null = Invoke-PilotReadQuery 'doctor' @(
        'doctor', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'authority.list' @(
        'authority', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'repository.list' @(
        'repository', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'roadmap.show' @(
        'roadmap', 'show', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'phase.list' @(
        'phase', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'gate.list' @(
        'gate', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'gate.show' @(
        'gate', 'show', 'gate:opa-01', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'artifact.list' @(
        'artifact', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $null = Invoke-PilotReadQuery 'audit' @(
        'audit', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $afterReadState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($readQueries.Count -ne 10 -or $afterReadState.state_version -ne $beforeState.state_version -or
        $afterReadState.state_digest -ne $beforeState.state_digest) {
        throw 'PILOT_READ_ONLY_MUTATION_DETECTED'
    }

    $observedAt = [DateTimeOffset]::UtcNow.ToString(
        "yyyy-MM-dd'T'HH:mm:ss'Z'",
        [Globalization.CultureInfo]::InvariantCulture
    )
    $agentFixture = Join-Path $fixtureDirectory 'agent-cannot-authorize.md'
    [IO.File]::WriteAllText(
        $agentFixture,
        "# Agent authority attempt`n`nThis closed fixture must not acquire human authority.`n",
        [Text.UTF8Encoding]::new($false)
    )
    $agentDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $agentFixture).Hash.ToLowerInvariant()
    $agentOutput = & $ledger authority add `
        --database $database --project-ref $projectRef --uri $agentFixture --digest $agentDigest `
        --record-id 'authority:opa-agent-forbidden' --expected-version '15' `
        --idempotency-key 'opa:stage2:agent-authority-forbidden' --actor-id 'agent:opa-stage2' `
        --observed-at $observedAt --json 2>&1
    $agentExit = $LASTEXITCODE
    $agentText = ($agentOutput | Out-String).Trim()
    if ($agentExit -ne 9) {
        throw "PILOT_AGENT_AUTHORITY_NOT_REJECTED: exit=$agentExit output=$agentText"
    }
    $agentErrorCode = $null
    foreach ($line in @($agentText -split "`r?`n")) {
        if ($line.TrimStart().StartsWith('{')) {
            try {
                $agentErrorCode = ($line | ConvertFrom-Json).error.code
                break
            } catch {
                continue
            }
        }
    }
    $afterAgentState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    if ($afterAgentState.state_version -ne $beforeState.state_version -or
        $afterAgentState.state_digest -ne $beforeState.state_digest) {
        throw 'PILOT_AGENT_AUTHORITY_MUTATED_STATE'
    }

    $auxV1Path = Join-Path $fixtureDirectory 'authority-aux-v1.md'
    $auxV2Path = Join-Path $fixtureDirectory 'authority-aux-v2.md'
    [IO.File]::WriteAllText(
        $auxV1Path,
        "# Disposable auxiliary authority v1`n`nStage 2 original.`n",
        [Text.UTF8Encoding]::new($false)
    )
    [IO.File]::WriteAllText(
        $auxV2Path,
        "# Disposable auxiliary authority v2`n`nStage 2 replacement.`n",
        [Text.UTF8Encoding]::new($false)
    )
    $auxV1Digest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $auxV1Path).Hash.ToLowerInvariant()
    $auxV2Digest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $auxV2Path).Hash.ToLowerInvariant()
    $auxV2Uri = [Uri]::new($auxV2Path).AbsoluteUri
    $stateVersion = 15
    $null = Invoke-PilotLedgerJson $ledger @(
        'authority', 'add', '--database', $database, '--project-ref', $projectRef,
        '--uri', $auxV1Path, '--digest', $auxV1Digest, '--record-id', 'authority:opa-aux-v1',
        '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:stage2:authority:v1',
        '--actor-id', 'human:owner', '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++
    $null = Invoke-PilotLedgerJson $ledger @(
        'authority', 'refresh', 'authority:opa-aux-v1', '--database', $database, '--project-ref', $projectRef,
        '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:stage2:authority:v1:refresh',
        '--actor-id', 'human:owner', '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++
    $null = Invoke-PilotLedgerJson $ledger @(
        'authority', 'add', '--database', $database, '--project-ref', $projectRef,
        '--uri', $auxV2Path, '--digest', $auxV2Digest, '--record-id', 'authority:opa-aux-v2',
        '--supersedes', 'authority:opa-aux-v1', '--expected-version', [string]$stateVersion,
        '--idempotency-key', 'opa:stage2:authority:v2', '--actor-id', 'human:owner',
        '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++
    $null = Invoke-PilotLedgerJson $ledger @(
        'authority', 'supersede', 'authority:opa-aux-v1', '--database', $database,
        '--project-ref', $projectRef, '--with', $auxV2Path, '--record-id', 'authority:opa-aux-v2',
        '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:stage2:authority:supersede',
        '--actor-id', 'human:owner', '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++

    $closedRepository = Join-Path $fixtureDirectory 'repository-read-only-canary'
    $hookDirectory = Join-Path $closedRepository '.git\hooks'
    [IO.Directory]::CreateDirectory($hookDirectory) | Out-Null
    $hookCanary = Join-Path $closedRepository '.git\hook-executed.txt'
    $contentCanary = Join-Path $closedRepository 'content-executed.txt'
    [IO.File]::WriteAllText(
        (Join-Path $hookDirectory 'post-checkout'),
        "#!/bin/sh`nprintf executed > '../hook-executed.txt'`n",
        [Text.UTF8Encoding]::new($false)
    )
    [IO.File]::WriteAllText(
        (Join-Path $closedRepository 'invoke-me.ps1'),
        "Set-Content -LiteralPath 'content-executed.txt' -Value 'executed'`n",
        [Text.UTF8Encoding]::new($false)
    )
    $null = Invoke-PilotLedgerJson $ledger @(
        'repository', 'bind', '--database', $database, '--project-ref', $projectRef,
        '--root', $closedRepository, '--remote', 'https://invalid.example/closed-stage-2-fixture',
        '--record-id', 'binding:opa-stage2-canary', '--expected-version', [string]$stateVersion,
        '--idempotency-key', 'opa:stage2:repository:canary', '--actor-id', 'human:owner',
        '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++
    if ((Test-Path -LiteralPath $hookCanary -PathType Leaf) -or
        (Test-Path -LiteralPath $contentCanary -PathType Leaf)) {
        throw 'PILOT_REPOSITORY_CONTENT_EXECUTED'
    }

    $null = Invoke-PilotLedgerJson $ledger @(
        'artifact', 'add', '--database', $database, '--project-ref', $projectRef,
        '--title', 'Stage 2 auxiliary authority replacement', '--kind', 'SPEC',
        '--uri', $auxV2Uri, '--digest', $auxV2Digest, '--record-id', 'artifact:opa-stage2-authority',
        '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:stage2:artifact:authority',
        '--actor-id', 'agent:opa-harness', '--observed-at', $observedAt, '--json'
    )
    $stateVersion++

    $finalState = Invoke-PilotLedgerJson $ledger @(
        'status', '--database', $database, '--project-ref', $projectRef, '--json'
    )
    $authorities = @(Invoke-PilotLedgerJson $ledger @(
        'authority', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $repositories = @(Invoke-PilotLedgerJson $ledger @(
        'repository', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $artifacts = @(Invoke-PilotLedgerJson $ledger @(
        'artifact', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $gates = @(Invoke-PilotLedgerJson $ledger @(
        'gate', 'list', '--database', $database, '--project-ref', $projectRef, '--json'
    ))
    $auxV1 = $authorities | Where-Object { $_.document_id -eq 'authority:opa-aux-v1' }
    $auxV2 = $authorities | Where-Object { $_.document_id -eq 'authority:opa-aux-v2' }
    $repositoryBinding = $repositories | Where-Object {
        $_.repository_binding_id -eq 'binding:opa-stage2-canary'
    }
    $artifactReference = $artifacts | Where-Object {
        $_.artifact_ref_id -eq 'artifact:opa-stage2-authority'
    }
    $allGatesUnverified = @($gates | Where-Object {
        $_.status -in @('VERIFIED', 'WAIVED')
    }).Count -eq 0
    if ($finalState.state_version -ne 21 -or $finalState.project.lifecycle_status -ne 'ACTIVE' -or
        $finalState.primary_active_phase_ref -ne 'phase:operational-acceptance' -or
        @($finalState.completed_phase_refs).Count -ne 0 -or -not $allGatesUnverified -or
        @($auxV1).Count -ne 1 -or @($auxV2).Count -ne 1 -or
        $auxV1.status -ne 'SUPERSEDED' -or $auxV2.status -ne 'CURRENT' -or
        $auxV1.digest -ne $auxV1Digest -or $auxV2.supersedes_ref -ne 'authority:opa-aux-v1' -or
        @($repositoryBinding).Count -ne 1 -or @($artifactReference).Count -ne 1) {
        throw 'PILOT_STAGE_2_FINAL_STATE_MISMATCH'
    }

    $manifest = [ordered]@{
        profile = 'project-ledger-operational-pilot-stage/1'
        stage = 2
        status = 'GOVERNED_MODEL_PASS'
        proof_class = 'LIVE_PILOT'
        project_ref = $projectRef
        read_only = [ordered]@{
            before_state_version = $beforeState.state_version
            after_state_version = $afterReadState.state_version
            before_state_digest = $beforeState.state_digest
            after_state_digest = $afterReadState.state_digest
            queries = @($readQueries)
        }
        agent_authority_attempt = [ordered]@{
            actor_id = 'agent:opa-stage2'
            as_human_used = $false
            exit_code = $agentExit
            error_code = $agentErrorCode
            state_version_delta = $afterAgentState.state_version - $beforeState.state_version
            state_digest_unchanged = $afterAgentState.state_digest -eq $beforeState.state_digest
        }
        authority_history = [ordered]@{
            original_document_ref = $auxV1.document_id
            original_status = $auxV1.status
            original_digest = $auxV1.digest
            original_digest_preserved = $auxV1.digest -eq $auxV1Digest
            refresh_performed = $true
            replacement_document_ref = $auxV2.document_id
            replacement_status = $auxV2.status
            replacement_digest = $auxV2.digest
            replacement_supersedes_original = $auxV2.supersedes_ref -eq $auxV1.document_id
        }
        repository_safety = [ordered]@{
            binding_mode = 'DECLARATIVE_READ_ONLY'
            binding_ref = 'binding:opa-stage2-canary'
            binding_present = @($repositoryBinding).Count -eq 1
            git_hook_canary_path = '.git/hook-executed.txt'
            repository_content_canary_path = 'content-executed.txt'
            canary_executed = (Test-Path -LiteralPath $hookCanary -PathType Leaf) -or
                (Test-Path -LiteralPath $contentCanary -PathType Leaf)
        }
        artifact_reference = [ordered]@{
            artifact_ref = 'artifact:opa-stage2-authority'
            present = @($artifactReference).Count -eq 1
            digest = $artifactReference.digest
        }
        lifecycle = [ordered]@{
            state_version = $finalState.state_version
            state_digest = $finalState.state_digest
            project_status = $finalState.project.lifecycle_status
            active_phase_ref = $finalState.primary_active_phase_ref
            completed_phase_count = @($finalState.completed_phase_refs).Count
            completed_project_without_command = $finalState.project.lifecycle_status -eq 'COMPLETED'
            all_gates_unverified = $allGatesUnverified
        }
        human_actions = [ordered]@{
            actor_id = 'human:owner'
            actions = @(
                'REGISTER_AUXILIARY_AUTHORITY',
                'REFRESH_AUXILIARY_AUTHORITY',
                'REGISTER_REPLACEMENT_AUTHORITY',
                'SUPERSEDE_AUXILIARY_AUTHORITY',
                'BIND_CLOSED_REPOSITORY_FIXTURE'
            )
            scope = 'Disposable Stage 2 fixtures only'
        }
        scenarios = [ordered]@{
            'OPA-S04' = 'PASS'
            'OPA-S05' = 'PASS'
            'OPA-S06' = 'PASS'
            'OPA-S07' = 'PASS'
        }
    }
    Write-PilotJson $stageTwoManifestPath $manifest

    [pscustomobject]@{
        stage = 2
        status = 'GOVERNED_MODEL_PASS'
        evidence = 'evidence/stage-2-governed-model.json'
    }
    return
}

if ($Stage -ne 1) {
    throw "PILOT_STAGE_NOT_IMPLEMENTED: stage $Stage"
}

$resolvedRepository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$resolvedZip = (Resolve-Path -LiteralPath $LedgerZip).Path
$resolvedPilotRoot = [System.IO.Path]::GetFullPath($PilotRoot)

if (Test-Path -LiteralPath $resolvedPilotRoot) {
    throw "PILOT_DESTINATION_EXISTS: $resolvedPilotRoot"
}

$actualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedZip).Hash.ToLowerInvariant()
$expectedHash = $ExpectedZipSha256.ToLowerInvariant()
if ($actualZipHash -ne $expectedHash) {
    throw "PILOT_ZIP_HASH_MISMATCH: expected $expectedHash; actual $actualZipHash"
}

$installation = Join-Path $resolvedPilotRoot 'instalación con espacios'
$evidenceDirectory = Join-Path $resolvedPilotRoot 'evidence'
$fixtureDirectory = Join-Path $resolvedPilotRoot 'fixtures'
[System.IO.Directory]::CreateDirectory($resolvedPilotRoot) | Out-Null
[System.IO.Directory]::CreateDirectory($evidenceDirectory) | Out-Null
[System.IO.Directory]::CreateDirectory($fixtureDirectory) | Out-Null
Expand-Archive -LiteralPath $resolvedZip -DestinationPath $installation

$ledger = Join-Path $installation 'project-ledger.exe'
$packageManifestPath = Join-Path $installation 'package.json'
if (-not (Test-Path -LiteralPath $ledger -PathType Leaf)) {
    throw 'PILOT_PACKAGE_BINARY_MISSING: project-ledger.exe'
}
if (-not (Test-Path -LiteralPath $packageManifestPath -PathType Leaf)) {
    throw 'PILOT_PACKAGE_MANIFEST_MISSING: package.json'
}

$packageManifest = Get-Content -Raw -LiteralPath $packageManifestPath | ConvertFrom-Json
if ($packageManifest.source_commit -ne $ExpectedSourceCommit) {
    throw "PILOT_SOURCE_COMMIT_MISMATCH: $($packageManifest.source_commit)"
}
if ($packageManifest.source_tree_dirty -ne $false) {
    throw 'PILOT_SOURCE_TREE_NOT_CLEAN'
}

$versionOutput = (& $ledger --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $versionOutput -ne 'project-ledger 0.2.0') {
    throw "PILOT_VERSION_MISMATCH: exit=$LASTEXITCODE output=$versionOutput"
}

$requiredCapabilities = @(
    @{ name = 'status'; arguments = @('status', '--help') },
    @{ name = 'doctor'; arguments = @('doctor', '--help') },
    @{ name = 'authority.add'; arguments = @('authority', 'add', '--help') },
    @{ name = 'repository.bind'; arguments = @('repository', 'bind', '--help') },
    @{ name = 'roadmap.import'; arguments = @('roadmap', 'import', '--help') },
    @{ name = 'phase.start'; arguments = @('phase', 'start', '--help') },
    @{ name = 'gate.define'; arguments = @('gate', 'define', '--help') },
    @{ name = 'record.progress'; arguments = @('record', 'progress', '--help') },
    @{ name = 'evidence.run'; arguments = @('evidence', 'run', '--help') },
    @{ name = 'checkpoint.create'; arguments = @('checkpoint', 'create', '--help') },
    @{ name = 'reconcile'; arguments = @('reconcile', '--help') },
    @{ name = 'render'; arguments = @('render', '--help') },
    @{ name = 'export'; arguments = @('export', '--help') },
    @{ name = 'backup'; arguments = @('backup', '--help') },
    @{ name = 'serve.provider'; arguments = @('serve', 'provider', '--help') },
    @{ name = 'serve.sink'; arguments = @('serve', 'sink', '--help') },
    @{ name = 'integration.gsp.compile'; arguments = @('integration', 'gsp', 'compile', '--help') }
)

$observedCapabilities = @()
$missingCapabilities = @()
foreach ($capability in $requiredCapabilities) {
    $null = & $ledger @($capability.arguments) 2>&1
    $available = $LASTEXITCODE -eq 0
    $observedCapabilities += [ordered]@{ name = $capability.name; available = $available }
    if (-not $available) {
        $missingCapabilities += $capability.name
    }
}
if ($missingCapabilities.Count -ne 0) {
    throw "PILOT_CAPABILITY_MISSING: $($missingCapabilities -join ',')"
}

$observedAt = [DateTimeOffset]::UtcNow.ToString(
    "yyyy-MM-dd'T'HH:mm:ss'Z'",
    [Globalization.CultureInfo]::InvariantCulture
)
$dataRoot = Join-Path $resolvedPilotRoot 'data'
$init = Invoke-PilotLedgerJson $ledger @(
    'init', '--title', 'G-Ledger Operational Acceptance Pilot',
    '--summary', 'Disposable governed validation of G-Ledger 0.2.0',
    '--timezone', 'America/Santiago', '--data-root', $dataRoot, '--json'
)
$initLines = $init.output -split "`n"
$projectRef = ($initLines | Where-Object { $_ -like 'project_ref=*' }) -replace '^project_ref=', ''
$database = ($initLines | Where-Object { $_ -like 'database=*' }) -replace '^database=', ''
if ($projectRef -notmatch '^project:[0-9a-f-]{36}$' -or -not (Test-Path -LiteralPath $database -PathType Leaf)) {
    throw 'PILOT_INIT_IDENTITY_INVALID'
}

$locator = [ordered]@{ database = $database; project_ref = $projectRef }
Write-PilotJson (Join-Path $resolvedPilotRoot 'pilot-locator.json') $locator

$specPath = Join-Path $resolvedRepository 'docs\pilot\G_LEDGER_OPERATIONAL_ACCEPTANCE_PILOT_SPEC_v1.0.0.md'
$specDigest = 'sha256:' + (Get-FileHash -Algorithm SHA256 -LiteralPath $specPath).Hash.ToLowerInvariant()
$stateVersion = 1
$null = Invoke-PilotLedgerJson $ledger @(
    'authority', 'add', '--database', $database, '--project-ref', $projectRef,
    '--uri', $specPath, '--digest', $specDigest, '--record-id', 'authority:opa-spec',
    '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:bootstrap:authority',
    '--actor-id', 'human:owner', '--observed-at', $observedAt, '--as-human', '--json'
)
$stateVersion++

$null = Invoke-PilotLedgerJson $ledger @(
    'repository', 'bind', '--database', $database, '--project-ref', $projectRef,
    '--root', $resolvedRepository, '--remote', 'https://github.com/gcatalansc/G-Ledger',
    '--record-id', 'binding:opa-repository', '--expected-version', [string]$stateVersion,
    '--idempotency-key', 'opa:bootstrap:repository', '--actor-id', 'human:owner',
    '--observed-at', $observedAt, '--as-human', '--json'
)
$stateVersion++

$roadmap = [ordered]@{
    schema_version = '0.2.0'
    roadmap_id = 'roadmap:opa'
    version = 1
    title = 'Operational acceptance roadmap'
    source_document_ref = 'authority:opa-spec'
    phase_order = @('phase:operational-acceptance')
    execution_model = 'SEQUENTIAL'
    completion_policy = 'ALL_REQUIRED_PHASES'
}
$roadmapPath = Join-Path $fixtureDirectory 'roadmap.json'
Write-PilotJson $roadmapPath $roadmap
$null = Invoke-PilotLedgerJson $ledger @(
    'roadmap', 'import', '--database', $database, '--project-ref', $projectRef,
    '--file', $roadmapPath, '--expected-version', [string]$stateVersion,
    '--idempotency-key', 'opa:bootstrap:roadmap', '--actor-id', 'human:owner',
    '--observed-at', $observedAt, '--as-human', '--json'
)
$stateVersion++

$gateIds = @()
foreach ($number in 1..8) {
    $id = 'gate:opa-{0:D2}' -f $number
    $gateIds += $id
    $gateWithoutDigest = [ordered]@{
        schema_version = '0.2.0'
        gate_id = $id
        definition_version = 1
        title = 'Operational acceptance stage {0}' -f $number
        phase_ref = 'phase:operational-acceptance'
        criticality = 'REQUIRED'
        required_evidence_rules = @([ordered]@{
            rule_id = 'rule:opa-{0:D2}' -f $number
            kind = 'CUSTOM_DECLARATIVE'
            predicate = [ordered]@{ operator = 'EQUALS'; path = 'result.exit_code'; value = 0 }
        })
        repository_dependencies = [ordered]@{ paths = @(); external_refs = @() }
        completion_rule = 'ALL'
        waiver_policy = 'FORBIDDEN'
        freshness_policy = [ordered]@{ evidence_must_match_same_repository_snapshot = $false }
        revalidate_on = @('AUTHORITY_DOCUMENT_CHANGED')
        created_at = $observedAt
    }
    $gate = [ordered]@{}
    foreach ($key in $gateWithoutDigest.Keys) { $gate[$key] = $gateWithoutDigest[$key] }
    $gate['digest'] = Get-PilotCanonicalDigest $gateWithoutDigest
    $gatePath = Join-Path $fixtureDirectory (('gate-opa-{0:D2}.json' -f $number))
    Write-PilotJson $gatePath $gate
    $null = Invoke-PilotLedgerJson $ledger @(
        'gate', 'define', '--database', $database, '--project-ref', $projectRef,
        '--file', $gatePath, '--expected-version', [string]$stateVersion,
        '--idempotency-key', ('opa:bootstrap:gate:{0:D2}' -f $number), '--actor-id', 'human:owner',
        '--observed-at', $observedAt, '--as-human', '--json'
    )
    $stateVersion++
}

$optionalGateWithoutDigest = [ordered]@{
    schema_version = '0.2.0'
    gate_id = 'gate:opa-waiver-exercise'
    definition_version = 1
    title = 'Optional waiver exercise'
    phase_ref = 'phase:operational-acceptance'
    criticality = 'OPTIONAL'
    required_evidence_rules = @([ordered]@{
        rule_id = 'rule:opa-waiver'
        kind = 'MANUAL_REVIEW'
        predicate = [ordered]@{ operator = 'EQUALS'; path = 'result.exit_code'; value = 0 }
    })
    repository_dependencies = [ordered]@{ paths = @(); external_refs = @() }
    completion_rule = 'ALL'
    waiver_policy = 'HUMAN_APPROVAL'
    freshness_policy = [ordered]@{ evidence_must_match_same_repository_snapshot = $false }
    revalidate_on = @('AUTHORITY_DOCUMENT_CHANGED')
    created_at = $observedAt
}
$optionalGate = [ordered]@{}
foreach ($key in $optionalGateWithoutDigest.Keys) { $optionalGate[$key] = $optionalGateWithoutDigest[$key] }
$optionalGate['digest'] = Get-PilotCanonicalDigest $optionalGateWithoutDigest
$optionalGatePath = Join-Path $fixtureDirectory 'gate-opa-waiver-exercise.json'
Write-PilotJson $optionalGatePath $optionalGate
$null = Invoke-PilotLedgerJson $ledger @(
    'gate', 'define', '--database', $database, '--project-ref', $projectRef,
    '--file', $optionalGatePath, '--expected-version', [string]$stateVersion,
    '--idempotency-key', 'opa:bootstrap:gate:waiver', '--actor-id', 'human:owner',
    '--observed-at', $observedAt, '--as-human', '--json'
)
$stateVersion++

$null = Invoke-PilotLedgerJson $ledger @(
    'phase', 'start', 'phase:operational-acceptance', '--database', $database, '--project-ref', $projectRef,
    '--expected-version', [string]$stateVersion, '--idempotency-key', 'opa:bootstrap:phase',
    '--actor-id', 'human:owner', '--observed-at', $observedAt, '--as-human', '--json'
)
$stateVersion++

$state = Invoke-PilotLedgerJson $ledger @('status', '--database', $database, '--project-ref', $projectRef, '--json')
$gates = @(Invoke-PilotLedgerJson $ledger @('gate', 'list', '--database', $database, '--project-ref', $projectRef, '--json'))
if ($state.state_version -ne 15 -or $state.project.lifecycle_status -ne 'ACTIVE' -or
    $state.primary_active_phase_ref -ne 'phase:operational-acceptance' -or $gates.Count -ne 9) {
    throw 'PILOT_BOOTSTRAP_STATE_MISMATCH'
}

$manifest = [ordered]@{
    profile = 'project-ledger-operational-pilot-stage/1'
    stage = 1
    status = 'PREFLIGHT_PASS'
    proof_class = 'LIVE_PILOT'
    subject = [ordered]@{
        zip_sha256 = $actualZipHash
        source_commit = $packageManifest.source_commit
        source_tree_dirty = $packageManifest.source_tree_dirty
        version = '0.2.0'
        certified_compatibility = $packageManifest.certified_compatibility
    }
    environment = [ordered]@{
        platform = 'windows'
        extraction_uses_spaces_and_non_ascii = $installation.Contains(' ') -and $installation.Contains('ó')
        repository_root_supplied = -not [string]::IsNullOrWhiteSpace($resolvedRepository)
        target_binary_used = $false
    }
    capabilities = [ordered]@{
        checked = $observedCapabilities
        missing = @($missingCapabilities)
    }
    project = [ordered]@{
        project_ref = $projectRef
        state_version = $state.state_version
        state_digest = $state.state_digest
        lifecycle_status = $state.project.lifecycle_status
        active_phase_ref = $state.primary_active_phase_ref
        required_gate_count = 8
        optional_gate_count = 1
        authority_document_ref = 'authority:opa-spec'
        specification_digest = $specDigest
    }
    human_bootstrap = [ordered]@{
        actor_id = 'human:owner'
        actions = @('REGISTER_AUTHORITY_DOCUMENT', 'BIND_REPOSITORY', 'DEFINE_ROADMAP', 'DEFINE_GATE', 'START_PHASE')
        scope = 'Disposable pilot project only'
    }
    scenarios = [ordered]@{
        'OPA-S01' = 'PASS'
        'OPA-S02' = 'PASS'
        'OPA-S03' = 'PASS'
    }
}

$manifestPath = Join-Path $evidenceDirectory 'stage-1-preflight.json'
Write-PilotJson $manifestPath $manifest

[pscustomobject]@{
    stage = 1
    status = 'PREFLIGHT_PASS'
    evidence = 'evidence/stage-1-preflight.json'
}
