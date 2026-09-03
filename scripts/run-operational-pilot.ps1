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

function Get-PilotTextDigest {
    param([string]$Text)

    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    return 'sha256:' + [Convert]::ToHexString($hash).ToLowerInvariant()
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
