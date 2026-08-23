use project_ledger_domain::{
    ContinuityCheckpoint, EvidenceRecord, GateEvaluation, IntegrationManifest, ProjectState,
    RadarSyncEnvelope,
};

const DIGEST: &str = "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";

fn project_state_json() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "0.2.0",
        "project_state_id": "state:project-01",
        "project": {
            "project_id": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
            "title": "Project Ledger",
            "summary": "Continuidad verificable",
            "lifecycle_status": "ACTIVE",
            "created_at": "2026-08-22T12:00:00-04:00",
            "timezone": "America/Santiago",
            "authority_policy": "SPEC_FIRST",
            "storage_profile": "USER_DATA",
            "repository_binding_refs": [],
            "paused_reason": null,
            "archived_at": null
        },
        "state_version": 4,
        "material_state_version": 4,
        "authority_documents": [],
        "roadmap": null,
        "active_phase_refs": ["phase-0"],
        "primary_active_phase_ref": "phase-0",
        "completed_phase_refs": [],
        "pending_gate_refs": ["gate:contracts"],
        "verified_gate_refs": [],
        "revalidation_required_gate_refs": [],
        "current_decision_refs": [],
        "open_blocker_refs": [],
        "open_risk_refs": [],
        "ordered_next_action_refs": [],
        "preferred_next_action_ref": null,
        "checkpoint_ref": null,
        "last_material_event_ref": null,
        "last_material_activity_at": "2026-08-22T12:00:00-04:00",
        "open_conflict_refs": [],
        "updated_at": "2026-08-22T12:00:00-04:00",
        "state_digest": DIGEST
    })
}

#[test]
fn project_state_roundtrips_and_rejects_unknown_fields() {
    let valid = project_state_json();
    let state: ProjectState = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(state).unwrap(), valid);

    let mut invalid = valid;
    invalid["fabricated_current"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProjectState>(invalid).is_err());
}

#[test]
fn checkpoint_requires_event_version_immediately_after_captured_state() {
    let valid = serde_json::json!({
        "schema_version": "0.2.0",
        "checkpoint_id": "checkpoint:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "project_ref": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "captured_state_version": 10,
        "captured_material_state_version": 9,
        "captured_state_digest": DIGEST,
        "checkpoint_event_version": 11,
        "checkpoint_event_ref": "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "authority_document_refs": [],
        "active_phase_refs": [],
        "primary_active_phase_ref": null,
        "verified_completed_phase_refs": [],
        "pending_gate_refs": [],
        "current_decision_refs": [],
        "open_blocker_refs": [],
        "open_risk_refs": [],
        "next_action_refs": [],
        "last_material_event_refs": [],
        "repository_binding": null,
        "unknown_refs": [],
        "conflict_refs": [],
        "created_by": "agent:codex",
        "created_at": "2026-08-22T12:00:00-04:00",
        "checkpoint_digest": DIGEST
    });
    let checkpoint: ContinuityCheckpoint = serde_json::from_value(valid.clone()).unwrap();
    assert!(checkpoint.validate().is_ok());

    let mut invalid = valid;
    invalid["checkpoint_event_version"] = serde_json::json!(10);
    let checkpoint: ContinuityCheckpoint = serde_json::from_value(invalid).unwrap();
    assert!(checkpoint.validate().is_err());
}

#[test]
fn representative_contracts_reject_unknown_discriminator_values() {
    let gate = serde_json::json!({
        "schema_version": "0.2.0",
        "gate_evaluation_id": "gate-eval:01",
        "gate_ref": "gate:contracts",
        "gate_definition_version": 1,
        "gate_definition_digest": DIGEST,
        "project_state_version": 4,
        "repository_binding": null,
        "selected_evidence_refs": [],
        "rejected_evidence": [],
        "rule_results": [],
        "result": "SUCCESS",
        "proposed_gate_status": "PENDING",
        "explanation_codes": [],
        "evaluated_by": {"engine": "project-ledger-gate-engine", "version": "0.2.0"},
        "evaluated_at": "2026-08-22T12:00:00-04:00",
        "provenance_ref": "project-ledger://provenance/gate-eval-01",
        "digest": DIGEST
    });
    assert!(serde_json::from_value::<GateEvaluation>(gate).is_err());

    let evidence = serde_json::json!({
        "schema_version": "0.2.0",
        "evidence_id": "evidence:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "project_ref": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "kind": "TEST_RESULT",
        "status": "CERTAIN",
        "source": {"source_type": "COMMAND_EXECUTION", "tool": "cargo", "command_display": "cargo test", "working_directory_ref": "workspace://root", "runner_version": "project-ledger/0.2.0"},
        "result": {"exit_code": 0, "duration_ms": 1, "metrics": {"passed": 1, "failed": 0, "skipped": 0, "errors": null}, "summary": "1 passed"},
        "repository_binding": null,
        "artifacts": [],
        "privacy": {"raw_output_stored": false, "redaction_applied": true},
        "observed_at": "2026-08-22T12:00:00-04:00",
        "payload_digest": DIGEST,
        "provenance_ref": "project-ledger://provenance/evidence-01"
    });
    assert!(serde_json::from_value::<EvidenceRecord>(evidence).is_err());
}

#[test]
fn integration_and_radar_contracts_enforce_fixed_semantics() {
    let manifest = serde_json::json!({
        "schema_version": "0.2.0",
        "integration_manifest_id": "integration-manifest:project-ledger",
        "provider_id": "project-ledger",
        "provider_version": "0.2.0",
        "protocols": [{"name": "project-ledger-integration", "version": "0.2.0"}],
        "projection_capabilities": ["PROJECT_CONTINUITY"],
        "record_capabilities": ["RECORD_WORK_PROGRESS"],
        "side_effect_contract": {"projection": "NONE", "health": "NONE", "record_submit": "DECLARED_ONLY"},
        "storage": {"shared_database_required": false},
        "upstream_locks": [],
        "digest": DIGEST
    });
    let parsed: IntegrationManifest = serde_json::from_value(manifest).unwrap();
    assert!(parsed.validate().is_ok());

    let radar = serde_json::json!({
        "schema_version": "0.1.1",
        "sync_envelope_id": "radar-sync:01",
        "profile": "project-ledger-radar-artifact/0.1.1",
        "source_instance_ref": "project-ledger://instances/local-user",
        "exported_at": "2026-08-22T12:00:00-04:00",
        "projects": [],
        "tombstones": [],
        "contract": {"schema_id": "urn:project-ledger:radar-sync-envelope:0.1.1", "profile_schema_digest": DIGEST, "source_spec_digest": DIGEST, "canonicalization": "JCS_RFC8785"},
        "semantics": {"omission_means_delete": true, "imports_are_idempotent": true, "name_only_mapping_allowed": false},
        "digest": DIGEST
    });
    let parsed: RadarSyncEnvelope = serde_json::from_value(radar).unwrap();
    assert!(parsed.validate().is_err());
}
