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

Describe 'Operational pilot runner Stage 3 high-level continuity' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'records material semantics, preserves transitions, and resumes from a current checkpoint' {
        $pilotRoot = Join-Path $TestDrive 'Stage 3 á continuity'

        foreach ($stage in 1..3) {
            $result = & $runner `
                -Stage $stage `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        }

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-3-high-level-continuity.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json

        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 3
        $manifest.status | Should Be 'HIGH_LEVEL_CONTINUITY_PASS'
        $manifest.proof_class | Should Be 'LIVE_PILOT'
        $manifest.lifecycle.initial_state_version | Should Be 21
        $manifest.lifecycle.final_state_version | Should Be 44
        $manifest.lifecycle.project_status | Should Be 'BLOCKED'
        $manifest.lifecycle.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.lifecycle.completed_phase_count | Should Be 0
        $manifest.lifecycle.all_gates_unverified | Should Be $true
        $manifest.records.progress.record_ref | Should Be 'work:opa-stage3-progress'
        $manifest.records.note.kind | Should Be 'LIMITATION'
        $manifest.records.claim.initial_classification | Should Be 'ASSERTED'
        $manifest.records.open_unknown.record_ref | Should Be 'unknown:opa-stage4-provider-policy'
        $manifest.records.open_blocker.record_ref | Should Be 'blocker:opa-stage4-recordsink'
        $manifest.records.mitigated_risk.record_ref | Should Be 'risk:opa-stage3-continuity'
        $manifest.records.next_action.record_ref | Should Be 'action:opa-stage4-evidence'
        $manifest.transitions.unknown.status | Should Be 'RESOLVED'
        $manifest.transitions.blocker.status | Should Be 'RESOLVED'
        $manifest.transitions.risk.status | Should Be 'MITIGATED'
        $manifest.transitions.decision.original_status | Should Be 'SUPERSEDED'
        $manifest.transitions.decision.replacement_status | Should Be 'ACCEPTED'
        $manifest.transitions.decision.predecessor_preserved | Should Be $true
        $manifest.transitions.timeline_contains_all.Count | Should Be 4
        $manifest.checkpoint.freshness | Should Match '^CURRENT'
        $manifest.checkpoint.session_closed | Should Be $true
        $manifest.resume.json.state_version | Should Be 44
        $manifest.resume.json.active_phase_present | Should Be $true
        $manifest.resume.json.pending_gates_present | Should Be $true
        $manifest.resume.json.current_decision_present | Should Be $true
        $manifest.resume.json.open_blocker_present | Should Be $true
        $manifest.resume.json.open_risk_present | Should Be $true
        $manifest.resume.json.critical_unknown_present | Should Be $true
        $manifest.resume.json.checkpoint_present | Should Be $true
        $manifest.resume.json.next_action_present | Should Be $true
        $manifest.resume.codex.all_required_sections_present | Should Be $true
        $manifest.scenarios.'OPA-S08' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S09' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S10' | Should Be 'PASS'
        $result.stage | Should Be 3
        $result.status | Should Be 'HIGH_LEVEL_CONTINUITY_PASS'
    }
}

Describe 'Operational pilot runner Stage 4 evidence and lifecycle' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'keeps observed evidence incomplete, records one verified sink effect, and preserves lifecycle authority' {
        $pilotRoot = Join-Path $TestDrive 'Stage 4 á evidence lifecycle'

        foreach ($stage in 1..4) {
            $result = & $runner `
                -Stage $stage `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        }

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-4-evidence-lifecycle.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json

        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 4
        $manifest.status | Should Be 'EVIDENCE_LIFECYCLE_PASS'
        $manifest.proof_class | Should Be 'LIVE_PILOT'
        $manifest.observed_evidence.status | Should Be 'OBSERVED'
        $manifest.observed_evidence.execution_exit_code | Should Be 0
        $manifest.observed_evidence.gate_evaluation_exit_code | Should Be 6
        $manifest.observed_evidence.gate_verified | Should Be $false
        $manifest.observed_evidence.phase_close_exit_code | Should Be 6
        $manifest.observed_evidence.phase_remained_active | Should Be $true
        $manifest.record_sink.validate_status | Should Be 'VALID'
        $manifest.record_sink.submit_status | Should Be 'ACCEPTED'
        $manifest.record_sink.replay_status | Should Be 'DUPLICATE_REPLAY'
        $manifest.record_sink.status_lookup | Should Be 'FOUND'
        $manifest.record_sink.reconcile_lookup | Should Be 'FOUND'
        $manifest.record_sink.state_version_delta | Should Be 1
        $manifest.record_sink.replay_state_version_delta | Should Be 0
        $manifest.record_sink.evidence_status | Should Be 'VERIFIED_SOURCE'
        $manifest.record_sink.raw_output_stored | Should Be $false
        $manifest.lifecycle.gate_opa_04_after_verified_evidence | Should Be 'VERIFIED'
        $manifest.lifecycle.phase_close_after_gate_exit_code | Should Be 6
        $manifest.lifecycle.phase_status | Should Be 'ACTIVE'
        $manifest.lifecycle.completed_phase_count | Should Be 0
        $manifest.lifecycle.external_lifecycle_event_count | Should Be 0
        $manifest.freshness.gate_status_after_authority_change | Should Be 'REVALIDATION_REQUIRED'
        $manifest.freshness.checkpoint_after_authority_change | Should Be 'STALE_FOR_AUTHORITY'
        $manifest.freshness.gate_status_after_revalidation | Should Be 'VERIFIED'
        $manifest.freshness.replacement_checkpoint | Should Be 'CURRENT'
        $manifest.waiver.optional_gate_status | Should Be 'WAIVED'
        $manifest.waiver.critical_waiver_exit_code | Should Be 3
        $manifest.waiver.critical_gate_status | Should Be 'PENDING'
        $manifest.waiver.critical_attempt_state_version_delta | Should Be 0
        $manifest.boundary.project_status | Should Be 'BLOCKED'
        $manifest.boundary.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.boundary.open_blocker_ref | Should Be 'blocker:opa-stage5-transaction'
        $manifest.boundary.next_action_ref | Should Be 'action:opa-stage5-transaction'
        $manifest.boundary.stage_5_executed | Should Be $false
        $manifest.boundary.merge_performed | Should Be $false
        $manifest.boundary.release_performed | Should Be $false
        $manifest.boundary.operational_acceptance_declared | Should Be $false
        $manifest.lifecycle_authority.gate_engine_only | Should Be $true
        $manifest.lifecycle_authority.phase_engine_only | Should Be $true
        $manifest.lifecycle_authority.record_sink_claims_lifecycle_authority | Should Be $false
        $manifest.scenarios.'OPA-S11' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S12' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S13' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S14' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S15' | Should Be 'PASS'
        $result.stage | Should Be 4
        $result.status | Should Be 'EVIDENCE_LIFECYCLE_PASS'
    }
}

Describe 'Operational pilot runner Stage 5 transactional safety' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'proves replay, conflicts, authority denial, and unknown-outcome reconciliation without blind retry' {
        $pilotRoot = Join-Path $TestDrive 'Stage 5 á transactional safety'

        foreach ($stage in 1..5) {
            $result = & $runner `
                -Stage $stage `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        }

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-5-transactional-safety.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json

        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 5
        $manifest.status | Should Be 'TRANSACTIONAL_SAFETY_PASS'
        $manifest.proof_class | Should Be 'LIVE_PILOT'
        $manifest.subject.zip_sha256 | Should Be $expectedHash
        $manifest.subject.source_commit | Should Be $expectedSourceCommit
        $manifest.replay.first_status | Should Be 'ACCEPTED'
        $manifest.replay.replay_status | Should Be 'DUPLICATE_REPLAY'
        $manifest.replay.replay_state_version_delta | Should Be 0
        $manifest.replay.replay_state_digest_unchanged | Should Be $true
        $manifest.replay.same_command_result_id | Should Be $true
        $manifest.replay.replayed | Should Be $true
        $manifest.idempotency_conflict.exit_code | Should Be 10
        $manifest.idempotency_conflict.state_version_delta | Should Be 0
        $manifest.idempotency_conflict.state_digest_unchanged | Should Be $true
        $manifest.version_conflict.exit_code | Should Be 5
        $manifest.version_conflict.state_version_delta | Should Be 0
        $manifest.version_conflict.state_digest_unchanged | Should Be $true
        $manifest.version_conflict.resume_observed | Should Be $true
        $manifest.version_conflict.replanned_status | Should Be 'ACCEPTED'
        $manifest.version_conflict.original_key_reused | Should Be $false
        $manifest.authority_denial.exit_code | Should Be 9
        $manifest.authority_denial.state_version_delta | Should Be 0
        $manifest.authority_denial.state_digest_unchanged | Should Be $true
        $manifest.authority_denial.as_human_added | Should Be $false
        $manifest.uncertain_checkpoint.stdout_discarded | Should Be $true
        $manifest.uncertain_checkpoint.command_attempt_count | Should Be 1
        $manifest.uncertain_checkpoint.reconcile_outcome | Should Be 'RECONCILED'
        $manifest.uncertain_checkpoint.reconciled_status | Should Be 'ACCEPTED'
        $manifest.uncertain_checkpoint.reconcile_state_version_delta | Should Be 0
        $manifest.uncertain_checkpoint.actor_id | Should Be 'agent:opa-stage5'
        $manifest.uncertain_checkpoint.actor_kind | Should Be 'AGENT'
        $manifest.uncertain_checkpoint.command_type | Should Be 'CREATE_CONTINUITY_CHECKPOINT'
        $manifest.uncertain_checkpoint.trust_domain | Should Be 'LOCAL_USER'
        $manifest.missing_result.exit_code | Should Be 12
        $manifest.missing_result.state_version_delta | Should Be 0
        $manifest.missing_result.state_digest_unchanged | Should Be $true
        $manifest.missing_result.blind_retry_performed | Should Be $false
        $manifest.boundary.project_status | Should Be 'BLOCKED'
        $manifest.boundary.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.boundary.completed_phase_count | Should Be 0
        $manifest.boundary.open_blocker_ref | Should Be 'blocker:opa-stage6-recovery'
        $manifest.boundary.next_action_ref | Should Be 'action:opa-stage6-recovery'
        $manifest.boundary.checkpoint_freshness | Should Be 'CURRENT'
        $manifest.boundary.stage_6_executed | Should Be $false
        $manifest.boundary.merge_performed | Should Be $false
        $manifest.boundary.release_performed | Should Be $false
        $manifest.boundary.operational_acceptance_declared | Should Be $false
        $manifest.scenarios.'OPA-S16' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S17' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S18' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S19' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S20' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S21' | Should Be 'PASS'
        $result.stage | Should Be 5
        $result.status | Should Be 'TRANSACTIONAL_SAFETY_PASS'
    }
}

Describe 'Operational pilot runner Stage 6 projections privacy and recovery' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'regenerates projections, roundtrips portable and backup state, rejects corruption, and excludes canaries' {
        $pilotRoot = Join-Path $TestDrive 'Stage 6 á recovery privacy'

        foreach ($stage in 1..6) {
            $result = & $runner `
                -Stage $stage `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        }

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-6-projections-privacy-recovery.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json

        $manifest.profile | Should Be 'project-ledger-operational-pilot-stage/1'
        $manifest.stage | Should Be 6
        $manifest.status | Should Be 'RECOVERY_PRIVACY_PASS'
        $manifest.proof_class | Should Be 'LIVE_PILOT'
        $manifest.subject.zip_sha256 | Should Be $expectedHash
        $manifest.subject.source_commit | Should Be $expectedSourceCommit
        $manifest.execution.resumed_after_partial_attempt | Should Be $false
        $manifest.execution.prior_runtime_preserved | Should Be $false
        $manifest.projection.initial_render_verified | Should Be $true
        $manifest.projection.divergence_exit_code | Should Be 15
        $manifest.projection.divergence_detected | Should Be $true
        $manifest.projection.regenerated_matches_authoritative_render | Should Be $true
        $manifest.projection.existing_output_exit_code | Should Be 2
        $manifest.projection.existing_output_unchanged | Should Be $true
        $manifest.projection.state_version_delta | Should Be 0
        $manifest.projection.state_digest_unchanged | Should Be $true
        $manifest.portable.export_state_version_delta | Should Be 0
        $manifest.portable.scope | Should Be 'AUXILIARY_DISPOSABLE_PROJECT'
        $manifest.portable.import_disposition | Should Be 'NEW_PROJECT'
        $manifest.portable.state_version_matches | Should Be $true
        $manifest.portable.state_digest_matches | Should Be $true
        $manifest.portable.history_head_digest_matches | Should Be $true
        $manifest.portable.resume_matches | Should Be $true
        $manifest.portable.restores_command_results | Should Be $false
        $manifest.portable.identical_chain_disposition | Should Be 'IDENTICAL_CHAIN'
        $manifest.portable.identical_chain_state_version_delta | Should Be 0
        $manifest.portable.existing_export_exit_code | Should Be 2
        $manifest.portable.existing_export_unchanged | Should Be $true
        $manifest.portable.checksum_corruption_exit_code | Should Be 15
        $manifest.portable.corrupt_destination_created | Should Be $false
        $manifest.portable.divergence_exit_code | Should Be 15
        $manifest.portable.divergent_target_unchanged | Should Be $true
        $manifest.backup.format | Should Be 'project-ledger-sqlite-backup/1'
        $manifest.backup.backup_state_version_delta | Should Be 0
        $manifest.backup.restore_state_version_matches | Should Be $true
        $manifest.backup.restore_state_digest_matches | Should Be $true
        $manifest.backup.restore_head_digest_matches | Should Be $true
        $manifest.backup.reconcile_outcome | Should Be 'RECONCILED'
        $manifest.backup.reconciled_status | Should Be 'ACCEPTED'
        $manifest.backup.replay_status | Should Be 'DUPLICATE_REPLAY'
        $manifest.backup.replay_state_version_delta | Should Be 0
        $manifest.backup.existing_backup_exit_code | Should Be 2
        $manifest.backup.existing_backup_unchanged | Should Be $true
        $manifest.backup.existing_restore_exit_code | Should Be 2
        $manifest.backup.existing_restore_unchanged | Should Be $true
        $manifest.backup.checksum_corruption_exit_code | Should Be 15
        $manifest.backup.corrupt_destination_created | Should Be $false
        $manifest.privacy.canary_count | Should Be 2
        $manifest.privacy.evidence_execution_exit_code | Should Be 3
        $manifest.privacy.evidence_record_status | Should Be 'ACCEPTED'
        $manifest.privacy.raw_output_stored | Should Be $false
        $manifest.privacy.canary_match_count | Should Be 0
        $manifest.privacy.post_canary_render_verified | Should Be $true
        $manifest.privacy.portable_export_exit_code | Should Be 13
        $manifest.privacy.portable_export_rejected | Should Be $true
        $manifest.privacy.portable_destination_created | Should Be $false
        $manifest.privacy.sanitized_report_check_required | Should Be $true
        $manifest.privacy.universal_byte_absence_claimed | Should Be $false
        $manifest.integrity.doctor_integrity | Should Be 'ok'
        $manifest.integrity.verify_integrity | Should Be 'ok'
        $manifest.integrity.audit_integrity | Should Be 'ok'
        $manifest.integrity.verify_finding_count | Should Be 0
        $manifest.integrity.audit_finding_count | Should Be 0
        $manifest.boundary.project_status | Should Be 'BLOCKED'
        $manifest.boundary.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.boundary.completed_phase_count | Should Be 0
        $manifest.boundary.open_blocker_ref | Should Be 'blocker:opa-stage7-integrations'
        $manifest.boundary.next_action_ref | Should Be 'action:opa-stage-07-integrations'
        $manifest.boundary.checkpoint_freshness | Should Be 'CURRENT'
        $manifest.boundary.stage_7_executed | Should Be $false
        $manifest.boundary.merge_performed | Should Be $false
        $manifest.boundary.release_performed | Should Be $false
        $manifest.boundary.operational_acceptance_declared | Should Be $false
        $manifest.scenarios.'OPA-S22' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S23' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S24' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S25' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S26' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S27' | Should Be 'PASS'
        $result.stage | Should Be 6
        $result.status | Should Be 'RECOVERY_PRIVACY_PASS'
    }

    It 'reconciles an accepted progress record and preserves a failed runtime before resuming' {
        $pilotRoot = Join-Path $TestDrive 'Stage 6 resumed after interruption'

        foreach ($stage in 1..5) {
            $null = & $runner `
                -Stage $stage `
                -LedgerZip $ledgerZip `
                -PilotRoot $pilotRoot `
                -RepositoryRoot $repositoryRoot `
                -ExpectedZipSha256 $expectedHash `
                -ExpectedSourceCommit $expectedSourceCommit
        }

        $locator = Get-Content -Raw -LiteralPath (Join-Path $pilotRoot 'pilot-locator.json') | ConvertFrom-Json
        $ledger = Join-Path $pilotRoot 'instalación con espacios\project-ledger.exe'
        $state = & $ledger status `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --json | ConvertFrom-Json
        $observedAt = [DateTimeOffset]::UtcNow.ToString(
            "yyyy-MM-dd'T'HH:mm:ss'Z'",
            [Globalization.CultureInfo]::InvariantCulture
        )
        $progress = & $ledger record progress `
            --summary 'Stage 6 validates projections, privacy, portable recovery, and SQLite backup restore.' `
            --phase 'phase:operational-acceptance' `
            --record-id 'work:opa-stage6-recovery' `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --expected-version ([string]$state.state_version) `
            --idempotency-key 'opa:stage6:progress' `
            --actor-id 'agent:opa-stage6' `
            --observed-at $observedAt `
            --json | ConvertFrom-Json
        $progress.status | Should Be 'ACCEPTED'

        $failedRuntime = Join-Path $pilotRoot 'stage-6-runtime'
        $null = New-Item -ItemType Directory -Path $failedRuntime
        $sentinel = Join-Path $failedRuntime 'failed-attempt-preserved.txt'
        [IO.File]::WriteAllText($sentinel, 'preserve')

        $result = & $runner `
            -Stage 6 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit

        $LASTEXITCODE | Should Be 0
        $manifest = Get-Content -Raw -LiteralPath (
            Join-Path $pilotRoot 'evidence\stage-6-projections-privacy-recovery.json'
        ) | ConvertFrom-Json
        $manifest.status | Should Be 'RECOVERY_PRIVACY_PASS'
        $manifest.execution.resumed_after_partial_attempt | Should Be $true
        $manifest.execution.prior_runtime_preserved | Should Be $true
        $manifest.mutations[0].recovered_by_reconciliation | Should Be $true
        (Test-Path -LiteralPath $sentinel) | Should Be $true
        $manifest.boundary.stage_7_executed | Should Be $false
    }
}
