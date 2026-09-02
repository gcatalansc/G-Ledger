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
    $json = $Value | ConvertTo-Json -Depth 20
    [IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}

function Invoke-PilotLedgerJson {
    param([string]$Executable, [string[]]$Arguments, [int[]]$ExpectedExitCodes = @(0))
    $lines = & $Executable @Arguments 2>&1
    $exit = $LASTEXITCODE
    $text = ($lines | Out-String).Trim()
    if ($ExpectedExitCodes -notcontains $exit) {
        throw "PILOT_LEDGER_COMMAND_FAILED: exit=$exit args=$($Arguments -join ' ') output=$text"
    }
    if ([string]::IsNullOrWhiteSpace($text)) { return $null }
    return $text | ConvertFrom-Json
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
