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

Describe 'Operational pilot runner Stage 7 blind handoff and bounded integrations' -Tag 'Stage7' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'validates a blind handoff, bounded provider and RecordSink, and the fixed GSP runtime' {
        $pilotRoot = Join-Path $TestDrive 'Stage 7 blind integrations'
        foreach ($stage in 1..6) {
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
        $skill = Join-Path $pilotRoot 'instalación con espacios\resources\integrations\agent-skills\project-ledger-continuity\SKILL.md'
        $reported = & $ledger resume `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --format json | ConvertFrom-Json
        $observedAt = [DateTimeOffset]::UtcNow.ToString(
            "yyyy-MM-dd'T'HH:mm:ss'Z'",
            [Globalization.CultureInfo]::InvariantCulture
        )
        $progress = & $ledger record progress `
            --summary 'A new blind Codex task reconstructed continuity and recorded this material handoff.' `
            --phase 'phase:operational-acceptance' `
            --record-id 'work:opa-stage7-blind-handoff' `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --expected-version ([string]$reported.state_version) `
            --idempotency-key 'opa:stage7:blind-progress' `
            --actor-id 'agent:codex-handoff' `
            --observed-at $observedAt `
            --json | ConvertFrom-Json
        $checkpoint = & $ledger checkpoint create `
            --reason 'Blind Codex Stage 7 handoff' `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --expected-version ([string]$progress.new_state_version) `
            --idempotency-key 'opa:stage7:blind-checkpoint' `
            --actor-id 'agent:codex-handoff' `
            --observed-at $observedAt `
            --json | ConvertFrom-Json
        $handoffEvidence = [ordered]@{
            profile = 'project-ledger-operational-pilot-blind-handoff/1'
            task_kind = 'CODEX_NEW_BLIND'
            task_ref = 'task:test-blind-codex'
            prompt_sha256 = 'sha256:' + ('7' * 64)
            inputs = [ordered]@{
                executable = $ledger
                database = [string]$locator.database
                project_ref = [string]$locator.project_ref
                skill = $skill
            }
            constraints = [ordered]@{
                narrative_context_received = $false
                direct_database_access = $false
                repository_context_access = $false
                git_access = $false
                as_human_used = $false
                stage_8_executed = $false
                lifecycle_command_count = 0
            }
            reported_state = [ordered]@{
                state_version = [int64]$reported.state_version
                state_digest = [string]$reported.state_digest
                phase_ref = [string]$reported.active_phase_refs[0]
                pending_gate_refs = @($reported.pending_gate_refs)
                open_blocker_refs = @($reported.open_blocker_refs)
                current_decision_refs = @($reported.current_decision_refs)
                next_action_ref = [string]$reported.preferred_next_action_ref
                checkpoint_ref = [string]$reported.checkpoint_ref
                checkpoint_freshness = [string]$reported.checkpoint_freshness
            }
            progress = [ordered]@{
                actor_id = 'agent:codex-handoff'
                idempotency_key = 'opa:stage7:blind-progress'
                record_id = 'work:opa-stage7-blind-handoff'
                summary = 'A new blind Codex task reconstructed continuity and recorded this material handoff.'
                status = [string]$progress.status
            }
            checkpoint = [ordered]@{
                actor_id = 'agent:codex-handoff'
                idempotency_key = 'opa:stage7:blind-checkpoint'
                checkpoint_ref = [string]$checkpoint.resulting_checkpoint_ref
                status = [string]$checkpoint.status
            }
        }
        $handoffPath = Join-Path $pilotRoot 'evidence\blind-codex-handoff-receipt.json'
        [IO.File]::WriteAllText(
            $handoffPath,
            ($handoffEvidence | ConvertTo-Json -Depth 20) + [Environment]::NewLine,
            [Text.UTF8Encoding]::new($false)
        )
        $gspPython = (Resolve-Path (Join-Path $repositoryRoot '..\G-GSP\gsp-gemma-compiler\.venv\Scripts\python.exe')).Path
        $gspSource = (Resolve-Path (Join-Path $repositoryRoot '.tooling\g-ledger-operational-pilot\gsp-exact-7a43559')).Path

        $result = & $runner `
            -Stage 7 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit `
            -BlindHandoffEvidence $handoffPath `
            -GspPython $gspPython `
            -GspSourceRoot $gspSource

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-7-blind-handoff-integrations.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
        $manifest.stage | Should Be 7
        $manifest.status | Should Be 'BLIND_HANDOFF_INTEGRATIONS_PASS'
        $manifest.proof_class | Should Be 'LIVE_HANDOFF_AND_LOCAL_INTEGRATION'
        $manifest.blind_handoff.input_count | Should Be 4
        $manifest.blind_handoff.narrative_context_received | Should Be $false
        $manifest.blind_handoff.progress_status | Should Be 'ACCEPTED'
        $manifest.blind_handoff.progress_state_version_delta | Should Be 1
        $manifest.blind_handoff.checkpoint_status | Should Be 'ACCEPTED'
        $manifest.blind_handoff.checkpoint_freshness | Should Be 'CURRENT'
        $manifest.blind_handoff.lifecycle_command_count | Should Be 0
        $manifest.blind_handoff.as_human_used | Should Be $false
        $manifest.provider.runtime_status | Should Be 'AVAILABLE'
        $manifest.provider.mutable_method_error_code | Should Be -32601
        $manifest.provider.record_sink_capability_count | Should Be 0
        $manifest.provider.state_version_delta | Should Be 0
        $manifest.provider.state_digest_unchanged | Should Be $true
        $manifest.record_sink.allowed_command_kinds.Count | Should Be 5
        $manifest.record_sink.claimed_authority_domain_count | Should Be 0
        $manifest.record_sink.validate_status | Should Be 'VALID'
        $manifest.record_sink.submit_status | Should Be 'ACCEPTED'
        $manifest.record_sink.replay_status | Should Be 'DUPLICATE_REPLAY'
        $manifest.record_sink.first_submit_state_version_delta | Should Be 1
        $manifest.record_sink.replay_state_version_delta | Should Be 0
        $manifest.record_sink.lifecycle_authority_count | Should Be 0
        $manifest.gsp.implementation_commit | Should Be '7a43559c02417641691dbb5f2a6e79c13d74d1a1'
        $manifest.gsp.contract_tag | Should Be 'gsp-exact-2026.04.12'
        $manifest.gsp.registry_contract_count | Should Be 28
        $manifest.gsp.schema_artifact_count | Should Be 31
        $manifest.gsp.export_state_version_delta | Should Be 0
        $manifest.gsp.compile_status | Should Be 'COMPILED_UNLINKED'
        $manifest.gsp.compile_state_version_delta | Should Be 0
        $manifest.gsp.validate_status | Should Be 'VALIDATED_UNLINKED'
        $manifest.gsp.validate_state_version_delta | Should Be 0
        $manifest.gsp.link_status | Should Be 'ACCEPTED'
        $manifest.gsp.link_state_version_delta | Should Be 1
        $manifest.gsp.replay_status | Should Be 'DUPLICATE_REPLAY'
        $manifest.gsp.replay_state_version_delta | Should Be 0
        $manifest.gsp.authority_elevation_count | Should Be 0
        $manifest.gsp.lifecycle_state_unchanged | Should Be $true
        $manifest.gsp.phase_state_unchanged | Should Be $true
        $manifest.gsp.certified_compatibility | Should Be $false
        $manifest.radar.status | Should Be 'EXCLUDED_BY_SCOPE'
        $manifest.radar.executed | Should Be $false
        $manifest.radar.pass_claimed | Should Be $false
        $manifest.integrity.doctor_integrity | Should Be 'ok'
        $manifest.integrity.verify_integrity | Should Be 'ok'
        $manifest.integrity.audit_integrity | Should Be 'ok'
        $manifest.integrity.verify_finding_count | Should Be 0
        $manifest.integrity.audit_finding_count | Should Be 0
        $manifest.boundary.project_status | Should Be 'BLOCKED'
        $manifest.boundary.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.boundary.completed_phase_count | Should Be 0
        $manifest.boundary.open_blocker_ref | Should Be 'blocker:opa-stage8-human-decision'
        $manifest.boundary.next_action_ref | Should Be 'action:opa-next-stage8-decision'
        $manifest.boundary.checkpoint_freshness | Should Be 'CURRENT'
        $manifest.boundary.stage_8_executed | Should Be $false
        $manifest.boundary.operational_acceptance_declared | Should Be $false
        $manifest.scenarios.'OPA-S28' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S29' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S30' | Should Be 'PASS'
        $manifest.scenarios.'OPA-S31' | Should Be 'EXCLUDED_BY_SCOPE'
        $result.stage | Should Be 7
        $result.status | Should Be 'BLIND_HANDOFF_INTEGRATIONS_PASS'
    }
}

Describe 'Operational pilot runner Stage 8 owner-decision readiness' -Tag 'Stage8' {
    $repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $runner = Join-Path $repositoryRoot 'scripts\run-operational-pilot.ps1'
    $ledgerZip = Join-Path $repositoryRoot '.tooling\candidates\99fdaab\project-ledger-0.2.0-windows-x86_64.zip'
    $expectedHash = 'b7abca67c630687f6c5a4f9d58c19d0711bebb87a6f6acdbb1487f91a000d9dc'
    $expectedSourceCommit = '99fdaab52bcec7003bab02e4cfee45059a4bf737'

    It 'reconciles all available evidence read-only while preserving the final owner-decision boundary' {
        $pilotRoot = Join-Path $TestDrive 'Stage 8 predecision reconciliation'
        foreach ($stage in 1..6) {
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
        $skill = Join-Path $pilotRoot 'instalación con espacios\resources\integrations\agent-skills\project-ledger-continuity\SKILL.md'
        $reported = & $ledger resume `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --format json | ConvertFrom-Json
        $observedAt = [DateTimeOffset]::UtcNow.ToString(
            "yyyy-MM-dd'T'HH:mm:ss'Z'",
            [Globalization.CultureInfo]::InvariantCulture
        )
        $progress = & $ledger record progress `
            --summary 'A new blind Codex task reconstructed continuity and recorded this material handoff.' `
            --phase 'phase:operational-acceptance' `
            --record-id 'work:opa-stage7-blind-handoff' `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --expected-version ([string]$reported.state_version) `
            --idempotency-key 'opa:stage7:blind-progress' `
            --actor-id 'agent:codex-handoff' `
            --observed-at $observedAt `
            --json | ConvertFrom-Json
        $checkpoint = & $ledger checkpoint create `
            --reason 'Blind Codex Stage 7 handoff' `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --expected-version ([string]$progress.new_state_version) `
            --idempotency-key 'opa:stage7:blind-checkpoint' `
            --actor-id 'agent:codex-handoff' `
            --observed-at $observedAt `
            --json | ConvertFrom-Json
        $handoffEvidence = [ordered]@{
            profile = 'project-ledger-operational-pilot-blind-handoff/1'
            task_kind = 'CODEX_NEW_BLIND'
            task_ref = 'task:test-stage8-blind-codex'
            prompt_sha256 = 'sha256:' + ('8' * 64)
            inputs = [ordered]@{
                executable = $ledger
                database = [string]$locator.database
                project_ref = [string]$locator.project_ref
                skill = $skill
            }
            constraints = [ordered]@{
                narrative_context_received = $false
                direct_database_access = $false
                repository_context_access = $false
                git_access = $false
                as_human_used = $false
                stage_8_executed = $false
                lifecycle_command_count = 0
            }
            reported_state = [ordered]@{
                state_version = [int64]$reported.state_version
                state_digest = [string]$reported.state_digest
                phase_ref = [string]$reported.active_phase_refs[0]
                pending_gate_refs = @($reported.pending_gate_refs)
                open_blocker_refs = @($reported.open_blocker_refs)
                current_decision_refs = @($reported.current_decision_refs)
                next_action_ref = [string]$reported.preferred_next_action_ref
                checkpoint_ref = [string]$reported.checkpoint_ref
                checkpoint_freshness = [string]$reported.checkpoint_freshness
            }
            progress = [ordered]@{
                actor_id = 'agent:codex-handoff'
                idempotency_key = 'opa:stage7:blind-progress'
                record_id = 'work:opa-stage7-blind-handoff'
                summary = 'A new blind Codex task reconstructed continuity and recorded this material handoff.'
                status = [string]$progress.status
            }
            checkpoint = [ordered]@{
                actor_id = 'agent:codex-handoff'
                idempotency_key = 'opa:stage7:blind-checkpoint'
                checkpoint_ref = [string]$checkpoint.resulting_checkpoint_ref
                status = [string]$checkpoint.status
            }
        }
        $handoffPath = Join-Path $pilotRoot 'evidence\blind-codex-handoff-receipt.json'
        [IO.File]::WriteAllText(
            $handoffPath,
            ($handoffEvidence | ConvertTo-Json -Depth 20) + [Environment]::NewLine,
            [Text.UTF8Encoding]::new($false)
        )
        $gspPython = (Resolve-Path (Join-Path $repositoryRoot '..\G-GSP\gsp-gemma-compiler\.venv\Scripts\python.exe')).Path
        $gspSource = (Resolve-Path (Join-Path $repositoryRoot '.tooling\g-ledger-operational-pilot\gsp-exact-7a43559')).Path
        $null = & $runner `
            -Stage 7 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $repositoryRoot `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit `
            -BlindHandoffEvidence $handoffPath `
            -GspPython $gspPython `
            -GspSourceRoot $gspSource

        $stageSeven = Get-Content -Raw -LiteralPath (
            Join-Path $pilotRoot 'evidence\stage-7-blind-handoff-integrations.json'
        ) | ConvertFrom-Json
        $stageEightRepository = Join-Path $TestDrive 'Stage 8 repository inputs'
        $resultsDirectory = Join-Path $stageEightRepository 'docs\pilot\results'
        $scriptsDirectory = Join-Path $stageEightRepository 'scripts'
        $null = New-Item -ItemType Directory -Path $resultsDirectory -Force
        $null = New-Item -ItemType Directory -Path $scriptsDirectory -Force
        Copy-Item -LiteralPath (Join-Path $repositoryRoot 'scripts\invoke-record-sink-rpc.ps1') `
            -Destination (Join-Path $scriptsDirectory 'invoke-record-sink-rpc.ps1')

        $scenarioResults = @()
        foreach ($number in 1..30) {
            $scenarioResults += [ordered]@{
                id = 'OPA-S{0:D2}' -f $number
                stage = if ($number -le 3) { 1 } elseif ($number -le 7) { 2 } elseif ($number -le 10) { 3 } elseif ($number -le 15) { 4 } elseif ($number -le 21) { 5 } elseif ($number -le 27) { 6 } else { 7 }
                critical = $number -ne 15
                status = 'PASS'
                proof_class = 'LIVE_PILOT'
                evidence = @('fixture://stage-evidence')
            }
        }
        $scenarioResults += [ordered]@{
            id = 'OPA-S31'; stage = 7; critical = $false; status = 'EXCLUDED_BY_SCOPE'
            proof_class = 'EXCLUDED_BY_SCOPE'; evidence = @('fixture://stage-7')
        }
        $scenarioResults += [ordered]@{
            id = 'OPA-S32'; stage = 8; critical = $true; status = 'NOT_RUN'
            proof_class = $null; evidence = @()
        }
        $stages = foreach ($number in 1..8) {
            [ordered]@{
                stage = $number
                name = 'STAGE_{0}' -f $number
                status = if ($number -eq 8) { 'NOT_RUN' } else { 'PASS' }
                evidence = if ($number -eq 8) { @() } else { @('fixture://stage-evidence') }
            }
        }
        $report = [ordered]@{
            profile = 'project-ledger-operational-pilot/1'
            status = 'IN_PROGRESS'
            completed_at = $null
            subject = [ordered]@{
                product_version = '0.2.0'
                binary_source_commit = $expectedSourceCommit
                zip_sha256 = $expectedHash
                qualification = 'HOSTED_REQUALIFIED'
            }
            scope = [ordered]@{ certified_compatibility = $false }
            progress = [ordered]@{
                total_stages = 8
                completed_stages = @(1, 2, 3, 4, 5, 6, 7)
                percentage = 87.5
            }
            project_identity = [ordered]@{
                project_ref = [string]$locator.project_ref
                latest_state_version = [int64]$stageSeven.lifecycle.final_state_version
                latest_state_digest = [string]$stageSeven.lifecycle.final_state_digest
                latest_checkpoint_ref = [string]$stageSeven.boundary.checkpoint_ref
                latest_checkpoint_freshness = 'CURRENT'
            }
            stages = @($stages)
            scenario_results = @($scenarioResults)
            defects = @()
            critical_waivers = @()
            owner_decision = $null
            limitations = @('No external certification is claimed')
        }
        $reportPath = Join-Path $resultsDirectory 'operational-pilot-report.json'
        [IO.File]::WriteAllText(
            $reportPath,
            ($report | ConvertTo-Json -Depth 20) + [Environment]::NewLine,
            [Text.UTF8Encoding]::new($false)
        )
        $matrixPath = Join-Path $stageEightRepository 'docs\pilot\G_LEDGER_OPERATIONAL_ACCEPTANCE_MATRIX.md'
        [IO.File]::WriteAllText(
            $matrixPath,
            "# Matrix fixture`n`n7/8 stages reconciled; final owner decision pending.`n",
            [Text.UTF8Encoding]::new($false)
        )

        $result = & $runner `
            -Stage 8 `
            -LedgerZip $ledgerZip `
            -PilotRoot $pilotRoot `
            -RepositoryRoot $stageEightRepository `
            -ExpectedZipSha256 $expectedHash `
            -ExpectedSourceCommit $expectedSourceCommit

        $LASTEXITCODE | Should Be 0
        $manifestPath = Join-Path $pilotRoot 'evidence\stage-8-owner-decision-readiness.json'
        (Test-Path -LiteralPath $manifestPath) | Should Be $true
        $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
        $manifest.stage | Should Be 8
        $manifest.status | Should Be 'READY_FOR_OWNER_DECISION'
        $manifest.proof_class | Should Be 'LIVE_PILOT_PREDECISION'
        $manifest.reconciliation.prior_scenario_pass_count | Should Be 30
        $manifest.reconciliation.excluded_scenario_count | Should Be 1
        $manifest.reconciliation.open_defect_count | Should Be 0
        $manifest.reconciliation.critical_waiver_count | Should Be 0
        $manifest.execution.read_only | Should Be $true
        $manifest.execution.ledger_state_version_delta | Should Be 0
        $manifest.gates.verified_required_gate_count | Should Be 1
        $manifest.gates.pending_required_gate_refs | Should Be @(
            'gate:opa-01',
            'gate:opa-02',
            'gate:opa-03',
            'gate:opa-05',
            'gate:opa-06',
            'gate:opa-07',
            'gate:opa-08'
        )
        $manifest.gates.waived_required_gate_count | Should Be 0
        $manifest.integrity.checkpoint_freshness | Should Be 'CURRENT'
        $manifest.integrity.doctor_finding_count | Should Be 0
        $manifest.integrity.verify_finding_count | Should Be 0
        $manifest.integrity.audit_finding_count | Should Be 0
        $manifest.boundary.owner_decision | Should Be $null
        $manifest.boundary.project_status | Should Be 'BLOCKED'
        $manifest.boundary.active_phase_ref | Should Be 'phase:operational-acceptance'
        $manifest.boundary.completed_phase_count | Should Be 0
        $manifest.boundary.open_blocker_refs | Should Be @('blocker:opa-stage8-human-decision')
        $manifest.boundary.stage_8_completed | Should Be $false
        $manifest.boundary.phase_close_attempted | Should Be $false
        $manifest.boundary.operational_acceptance_declared | Should Be $false
        $manifest.boundary.merge_performed | Should Be $false
        $manifest.boundary.release_performed | Should Be $false
        $manifest.scenarios.'OPA-S32' | Should Be 'AWAITING_HUMAN_DECISION'

        $finalState = & $ledger status `
            --database $locator.database `
            --project-ref $locator.project_ref `
            --json | ConvertFrom-Json
        $finalState.project.lifecycle_status | Should Be 'BLOCKED'
        $finalState.completed_phase_refs.Count | Should Be 0
        $finalState.verified_gate_refs | Should Be @('gate:opa-04')
        $finalState.pending_gate_refs | Should Be @(
            'gate:opa-01',
            'gate:opa-02',
            'gate:opa-03',
            'gate:opa-05',
            'gate:opa-06',
            'gate:opa-07',
            'gate:opa-08'
        )
        (Get-Content -Raw -LiteralPath $reportPath | ConvertFrom-Json).owner_decision | Should Be $null
        $result.stage | Should Be 8
        $result.status | Should Be 'READY_FOR_OWNER_DECISION'
    }
}
