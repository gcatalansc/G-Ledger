use serde_json::{Value, json};

use crate::{SchemaError, schema_catalog};

const DIGEST: &str = "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";
const PROJECT_ID: &str = "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const COMMAND_ID: &str = "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const EVENT_ID: &str = "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const TIMESTAMP: &str = "2026-08-22T12:00:00-04:00";

#[derive(Clone, Debug)]
pub struct FixtureEntry {
    pub schema_id: String,
    pub positive: Value,
    pub negative: Value,
}

/// Builds one positive and one closed-world negative fixture per schema.
///
/// # Errors
///
/// Returns [`SchemaError`] when the schema catalog cannot be built, a positive
/// fixture is missing, or a fixture root is not a JSON object.
pub fn fixture_catalog() -> Result<Vec<FixtureEntry>, SchemaError> {
    schema_catalog()?
        .into_iter()
        .map(|schema| {
            let positive = positive_fixture(&schema.relative_path)
                .ok_or_else(|| SchemaError::MissingFixture(schema.relative_path.clone()))?;
            let mut negative = positive.clone();
            negative
                .as_object_mut()
                .ok_or_else(|| SchemaError::NonObjectFixture(schema.relative_path.clone()))?
                .insert("unexpected_contract_field".to_owned(), Value::Bool(true));
            Ok(FixtureEntry {
                schema_id: schema.schema_id,
                positive,
                negative,
            })
        })
        .collect()
}

fn positive_fixture(path: &str) -> Option<Value> {
    Some(match path {
        "event-envelope.schema.json" => event_envelope(),
        "event-payload.schema.json" => event_payload(),
        "command-envelope.schema.json" => command_envelope(),
        "command-payload.schema.json" => command_payload(),
        "command-result.schema.json" => command_result(),
        "external-submission-record.schema.json" => external_submission(),
        "project.schema.json" => project(),
        "project-state.schema.json" => project_state(),
        "authority-document.schema.json" => authority_document(),
        "roadmap.schema.json" => roadmap(),
        "phase.schema.json" => phase(),
        "gate-definition.schema.json" => gate_definition(),
        "gate-evaluation.schema.json" => gate_evaluation(),
        "gate-waiver.schema.json" => gate_waiver(),
        "evidence-record.schema.json" => evidence_record(),
        "evidence-artifact-ref.schema.json" => evidence_artifact(),
        "artifact-reference.schema.json" => artifact_reference(),
        "repository-binding.schema.json" => repository_binding(),
        "unknown-record.schema.json" => unknown_record(),
        "claim-record.schema.json" => claim_record(),
        "work-record.schema.json" => work_record(),
        "decision.schema.json" => decision(),
        "blocker.schema.json" => blocker(),
        "risk.schema.json" => risk(),
        "next-action.schema.json" => next_action(),
        "conflict-record.schema.json" => conflict_record(),
        "session.schema.json" => session(),
        "continuity-checkpoint.schema.json" => checkpoint(),
        "checkpoint-freshness-record.schema.json" => checkpoint_freshness(),
        "resume-projection.schema.json" => resume_projection(),
        "legacy-import-candidate.schema.json" => legacy_candidate(),
        "legacy-import-review.schema.json" => legacy_review(),
        "generated-artifact-record.schema.json" => generated_artifact(),
        "identity-mapping.schema.json" => identity_mapping(),
        "integration-manifest.schema.json" => integration_manifest(),
        "projection-request.schema.json" => projection_request(),
        "projection-envelope.schema.json" => projection_envelope(),
        "provider-health.schema.json" => provider_health(),
        "loss-report.schema.json" => loss_report(),
        "portable-export.schema.json" => portable_export(),
        "project-catalog-entry.schema.json" => project_catalog_entry(),
        "radar/ledger-radar-project-projection.schema.json" => radar_projection(),
        "radar/radar-sync-envelope.schema.json" => radar_sync(),
        "radar/radar-project-link.schema.json" => radar_project_link(),
        "radar/radar-status-mapping-policy.schema.json" => radar_status_policy(),
        "radar/radar-field-ownership-policy.schema.json" => radar_field_policy(),
        _ => return None,
    })
}

fn actor() -> Value {
    json!({"actor_id":"agent:codex","actor_kind":"AGENT","trust_level":"LOCAL_USER","display_name":"Codex"})
}

fn project() -> Value {
    json!({
        "project_id": PROJECT_ID,
        "title": "Project Ledger",
        "summary": "Continuidad verificable",
        "lifecycle_status": "ACTIVE",
        "created_at": TIMESTAMP,
        "timezone": "America/Santiago",
        "authority_policy": "SPEC_FIRST",
        "storage_profile": "USER_DATA",
        "repository_binding_refs": [],
        "paused_reason": null,
        "archived_at": null
    })
}

fn authority_document() -> Value {
    json!({
        "schema_version":"0.2.0","document_id":"authority:spec","uri":"workspace://specs/PROJECT_LEDGER_SPEC_v0.2.0.md",
        "digest":DIGEST,"version":"0.2.0","status":"CURRENT","authority_rank":1,"effective_from":TIMESTAMP,"supersedes_ref":null
    })
}

fn roadmap() -> Value {
    json!({
        "schema_version":"0.2.0","roadmap_id":"roadmap:main","title":"Implementation roadmap","execution_model":"SEQUENTIAL",
        "phase_order":["phase-0"],"completion_policy":"ALL_REQUIRED_PHASES","version":1,"source_document_ref":"authority:spec"
    })
}

fn phase() -> Value {
    json!({
        "schema_version":"0.2.0","phase_id":"phase-0","title":"Contracts","ordinal":0,"kind":"FORMALIZATION","status":"ACTIVE",
        "required_gate_refs":["gate:contracts"],"optional_gate_refs":[],"started_at":TIMESTAMP,"completed_at":null,"reopened_at":null
    })
}

fn repository_snapshot() -> Value {
    json!({"repository_ref":"repo:ledger","snapshot_ref":"snapshot:01","snapshot_digest":DIGEST})
}

fn project_state() -> Value {
    json!({
        "schema_version":"0.2.0","project_state_id":"state:project-01","project":project(),"state_version":4,"material_state_version":4,
        "authority_documents":[authority_document()],"roadmap":roadmap(),"active_phase_refs":["phase-0"],"primary_active_phase_ref":"phase-0",
        "completed_phase_refs":[],"pending_gate_refs":["gate:contracts"],"verified_gate_refs":[],"revalidation_required_gate_refs":[],
        "current_decision_refs":[],"open_blocker_refs":[],"open_risk_refs":[],"ordered_next_action_refs":[],"preferred_next_action_ref":null,
        "checkpoint_ref":null,"last_material_event_ref":null,"last_material_activity_at":TIMESTAMP,"open_conflict_refs":[],"updated_at":TIMESTAMP,"state_digest":DIGEST
    })
}

fn command_payload() -> Value {
    json!({"command_type":"START_PHASE","payload":{"phase_ref":"phase-0"}})
}

fn command_envelope() -> Value {
    let mut value = json!({
        "schema_version":"0.2.0","command_id":COMMAND_ID,"command_schema_ref":"urn:project-ledger:command:start-phase:0.2.0",
        "project_ref":PROJECT_ID,"expected_state_version":4,"idempotency_key":"session-01:start-phase:phase-0","actor":actor(),
        "submitted_at":TIMESTAMP,"payload_digest":DIGEST,"command_digest":DIGEST
    });
    let body = command_payload();
    value["command_type"] = body["command_type"].clone();
    value["payload"] = body["payload"].clone();
    value
}

fn event_payload() -> Value {
    json!({"event_type":"PHASE_STARTED","payload":{"phase_ref":"phase-0"}})
}

fn event_envelope() -> Value {
    let mut value = json!({
        "schema_version":"0.2.0","event_id":EVENT_ID,"project_ref":PROJECT_ID,"stream_id":PROJECT_ID,"stream_version":5,
        "event_schema_ref":"urn:project-ledger:event:phase-started:0.2.0","actor":actor(),
        "causation":{"command_ref":COMMAND_ID,"command_digest":DIGEST,"correlation_ref":"session:01"},
        "occurred_at":TIMESTAMP,"recorded_at":TIMESTAMP,"previous_event_digest":DIGEST,"payload_digest":DIGEST,"event_digest":DIGEST
    });
    let body = event_payload();
    value["event_type"] = body["event_type"].clone();
    value["payload"] = body["payload"].clone();
    value
}

fn command_result() -> Value {
    json!({
        "schema_version":"0.2.0","command_result_id":"result:01","command_ref":COMMAND_ID,"status":"ACCEPTED",
        "previous_state_version":4,"new_state_version":5,"emitted_event_refs":[EVENT_ID],"resulting_checkpoint_ref":null,
        "rejection":null,"idempotency":{"replayed":false},"processed_at":TIMESTAMP,"digest":DIGEST
    })
}

fn external_submission() -> Value {
    json!({
        "schema_version":"0.2.0","submission_id":"submission:01","command_ref":COMMAND_ID,"idempotency_key":"session-01:start-phase:phase-0",
        "state":"OUTCOME_UNKNOWN","attempt_count":1,"last_transport_error":"TIMEOUT_AFTER_SEND","known_result_ref":null,
        "prepared_at":TIMESTAMP,"submitted_at":TIMESTAMP,"reconciled_at":null,"digest":DIGEST
    })
}

fn gate_definition() -> Value {
    json!({
        "schema_version":"0.2.0","gate_id":"gate:contracts","definition_version":1,"title":"Contracts closed","phase_ref":"phase-0","criticality":"REQUIRED",
        "required_evidence_rules":[{"rule_id":"rule:tests","kind":"TEST_RESULT","predicate":{"operator":"EQUALS","path":"result.exit_code","value":0}}],
        "repository_dependencies":{"paths":["schemas/v0.2/**"],"external_refs":[]},"completion_rule":"ALL","waiver_policy":"HUMAN_APPROVAL",
        "freshness_policy":{"evidence_must_match_same_repository_snapshot":true},"revalidate_on":["AUTHORITY_DOCUMENT_CHANGED"],"created_at":TIMESTAMP,"digest":DIGEST
    })
}

fn gate_evaluation() -> Value {
    json!({
        "schema_version":"0.2.0","gate_evaluation_id":"gate-eval:01","gate_ref":"gate:contracts","gate_definition_version":1,
        "gate_definition_digest":DIGEST,"project_state_version":4,"repository_binding":repository_snapshot(),"selected_evidence_refs":["evidence:01"],
        "rejected_evidence":[],"rule_results":[{"rule_id":"rule:tests","result":"PASS"}],"result":"PASS","proposed_gate_status":"VERIFIED",
        "explanation_codes":[],"evaluated_by":{"engine":"project-ledger-gate-engine","version":"0.2.0"},"evaluated_at":TIMESTAMP,
        "provenance_ref":"project-ledger://provenance/gate-eval-01","digest":DIGEST
    })
}

fn gate_waiver() -> Value {
    json!({
        "schema_version":"0.2.0","waiver_id":"waiver:01","gate_ref":"gate:contracts","gate_definition_version":1,"gate_definition_digest":DIGEST,
        "actor_ref":"human:owner","reason":"Accepted limitation","scope":"phase-0","expires_at":null,"review_condition":"Before release",
        "authority_ref":"authority:spec","created_at":TIMESTAMP,"digest":DIGEST
    })
}

fn evidence_artifact() -> Value {
    json!({"uri":"workspace://target/test-result.json","digest":DIGEST,"persisted":false})
}

fn evidence_record() -> Value {
    json!({
        "schema_version":"0.2.0","evidence_id":"evidence:01","project_ref":PROJECT_ID,"kind":"TEST_RESULT","status":"VERIFIED_SOURCE",
        "source":{"source_type":"COMMAND_EXECUTION","tool":"cargo","command_display":"cargo test","working_directory_ref":"workspace://root","runner_version":"project-ledger/0.2.0"},
        "result":{"exit_code":0,"duration_ms":10,"metrics":{"passed":1,"failed":0,"skipped":0,"errors":null},"summary":"1 passed"},
        "repository_binding":repository_snapshot(),"artifacts":[evidence_artifact()],"privacy":{"raw_output_stored":false,"redaction_applied":true},
        "observed_at":TIMESTAMP,"payload_digest":DIGEST,"provenance_ref":"project-ledger://provenance/evidence-01"
    })
}

fn artifact_reference() -> Value {
    json!({
        "schema_version":"0.2.0","artifact_ref_id":"artifact:spec","name":"Project Ledger Spec","kind":"SPEC","uri":"workspace://specs/PROJECT_LEDGER_SPEC_v0.2.0.md",
        "digest":DIGEST,"version":"0.2.0","sensitivity":"INTERNAL","source_ref":null,"created_at":TIMESTAMP,"provenance_ref":"project-ledger://provenance/artifact-spec"
    })
}

fn repository_binding() -> Value {
    json!({
        "schema_version":"0.2.0","repository_binding_id":"binding:ledger","repository_ref":"repo:ledger","workspace_root_ref":"workspace://root",
        "identity_method":"USER_CONFIRMED","remote_identity_digest":null,"current_snapshot_ref":"snapshot:01","current_snapshot_digest":DIGEST,
        "status":"CURRENT","confirmed_by":"human:owner","created_at":TIMESTAMP,"updated_at":TIMESTAMP,"provenance_ref":"project-ledger://provenance/binding-ledger","digest":DIGEST
    })
}

fn unknown_record() -> Value {
    json!({
        "schema_version":"0.2.0","unknown_id":"unknown:01","subject_kind":"GATE","subject_ref":"gate:contracts","field_path":"evidence.complete",
        "statement":"Coverage not yet known","criticality":"MATERIAL","status":"OPEN","blocks":["phase-0"],"required_evidence":["contract test"],
        "evidence_refs":[],"opened_at":TIMESTAMP,"resolved_at":null,"resolution_ref":null,"provenance_ref":"project-ledger://provenance/unknown-01","digest":DIGEST
    })
}

fn claim_record() -> Value {
    json!({"schema_version":"0.2.0","claim_id":"claim:01","statement":"Schemas generated","classification":"ASSERTED","subject_refs":["phase-0"],"evidence_refs":[],"authority_ref":null})
}

fn work_record() -> Value {
    json!({
        "schema_version":"0.2.0","work_record_id":"work:01","summary":"Implemented contracts","phase_ref":"phase-0","task_ref":null,
        "files_or_artifacts":["workspace://schemas/v0.2"],"commit_refs":[],"claim_refs":["claim:01"],"evidence_refs":[],"actor_ref":"agent:codex",
        "influence":null,"occurred_at":TIMESTAMP,"recorded_at":TIMESTAMP,"provenance_ref":"project-ledger://provenance/work-01","digest":DIGEST
    })
}

fn decision() -> Value {
    json!({
        "schema_version":"0.2.0","decision_id":"decision:rust","title":"Use Rust","domain":"architecture","scope":"core","context":"Single binary",
        "decision":"Stable Rust","rationale":"Portability","alternatives":["Go"],"status":"ACCEPTED","version":1,"authority_ref":"authority:spec",
        "evidence_refs":[],"supersedes_ref":null,"created_at":TIMESTAMP,"accepted_at":TIMESTAMP,"provenance_ref":"project-ledger://provenance/decision-rust","digest":DIGEST
    })
}

fn blocker() -> Value {
    json!({
        "schema_version":"0.2.0","blocker_id":"blocker:01","title":"Missing lock","description":"Upstream lock pending","scope_kind":"EXTERNAL","scope_ref":"gate:integration",
        "severity":"HIGH","status":"OPEN","source_kind":"INTEGRATION","source_ref":"urn:project-ledger:integration:gskills","owner_ref":null,"evidence_refs":[],
        "opened_at":TIMESTAMP,"updated_at":TIMESTAMP,"resolved_at":null,"resolution_summary":null,"provenance_ref":"project-ledger://provenance/blocker-01","digest":DIGEST
    })
}

fn risk() -> Value {
    json!({
        "schema_version":"0.2.0","risk_id":"risk:01","title":"Schema drift","description":"Contracts may diverge","scope_kind":"PROJECT","scope_ref":"phase-0",
        "severity":"HIGH","likelihood_class":"POSSIBLE","status":"MONITORED","owner_ref":"human:owner","trigger_conditions":["digest mismatch"],
        "mitigation_actions":["offline registry"],"evidence_refs":[],"opened_at":TIMESTAMP,"updated_at":TIMESTAMP,"closed_at":null,
        "provenance_ref":"project-ledger://provenance/risk-01","digest":DIGEST
    })
}

fn next_action() -> Value {
    json!({
        "schema_version":"0.2.0","next_action_id":"action:schemas","title":"Generate schemas","description":"Materialize registry","phase_ref":"phase-0","gate_ref":"gate:contracts",
        "classification":"AUTHORIZED","authority_ref":"authority:spec","precondition_refs":[],"blocked_by_refs":[],"owner_ref":"agent:codex",
        "created_at":TIMESTAMP,"updated_at":TIMESTAMP,"completed_at":null,"provenance_ref":"project-ledger://provenance/action-schemas","digest":DIGEST
    })
}

fn conflict_record() -> Value {
    json!({
        "schema_version":"0.2.0","conflict_id":"conflict:01","kind":"EXTERNAL_VERSION_CONFLICT","severity":"HIGH","domain":"integration",
        "subject_refs":["gate:integration"],"source_refs":["urn:project-ledger:integration:gskills"],"status":"OPEN","required_action":"Refresh lock",
        "opened_at":TIMESTAMP,"resolved_at":null,"resolution_ref":null
    })
}

fn session() -> Value {
    json!({"schema_version":"0.2.0","session_id":"session:01","actor_ref":"agent:codex","host":"windows","started_at":TIMESTAMP,"ended_at":null,"starting_checkpoint_ref":null,"ending_checkpoint_ref":null})
}

fn checkpoint() -> Value {
    json!({
        "schema_version":"0.2.0","checkpoint_id":"checkpoint:01","project_ref":PROJECT_ID,"captured_state_version":4,"captured_material_state_version":4,
        "captured_state_digest":DIGEST,"checkpoint_event_version":5,"checkpoint_event_ref":EVENT_ID,"authority_document_refs":["authority:spec"],
        "active_phase_refs":["phase-0"],"primary_active_phase_ref":"phase-0","verified_completed_phase_refs":[],"pending_gate_refs":["gate:contracts"],
        "current_decision_refs":[],"open_blocker_refs":[],"open_risk_refs":["risk:01"],"next_action_refs":["action:schemas"],"last_material_event_refs":[],
        "repository_binding":repository_snapshot(),"unknown_refs":[],"conflict_refs":[],"created_by":"agent:codex","created_at":TIMESTAMP,"checkpoint_digest":DIGEST
    })
}

fn checkpoint_freshness() -> Value {
    json!({"schema_version":"0.2.0","freshness_record_id":"freshness:01","checkpoint_ref":"checkpoint:01","status":"CURRENT","evaluated_at":TIMESTAMP,"reason_codes":[],"digest":DIGEST})
}

fn resume_projection() -> Value {
    json!({
        "schema_version":"0.2.0","project_ref":PROJECT_ID,"state_version":4,"state_digest":DIGEST,"checkpoint_freshness":"CURRENT",
        "authority_document_refs":["authority:spec"],"active_phase_refs":["phase-0"],"completed_phase_refs":[],"current_decision_refs":[],
        "open_blocker_refs":[],"open_risk_refs":["risk:01"],"pending_gate_refs":["gate:contracts"],"last_material_progress_summary":"Contracts in progress",
        "preferred_next_action_ref":"action:schemas","critical_unknown_refs":[],"conflict_refs":[],"checkpoint_ref":"checkpoint:01","rendered_token_count":120,"digest":DIGEST
    })
}

fn legacy_candidate() -> Value {
    json!({
        "schema_version":"0.2.0","candidate_id":"legacy:phase-2-active","kind":"ACTIVE_PHASE","value":"phase-2",
        "source":{"path":"BITACORA_PROYECTO.md","section":"Estado ejecutivo","line_range":[8,8]},"conflicts_with":[],"recommended_action":"REVIEW"
    })
}

fn legacy_review() -> Value {
    json!({
        "schema_version":"0.2.0","review_id":"legacy-review:01","source_uri":"workspace://BITACORA_PROYECTO.md","source_digest":DIGEST,
        "mode":"PREVIEW","candidates":[legacy_candidate()],"decisions":[],"reviewed_at":TIMESTAMP,"digest":DIGEST
    })
}

fn generated_artifact() -> Value {
    json!({
        "schema_version":"0.2.0","generated_artifact_id":"generated:01","kind":"MARKDOWN","uri":"workspace://BITACORA_PROYECTO.md",
        "source_state_version":4,"source_state_digest":DIGEST,"renderer_version":"0.2.0","privacy_profile":"FULL_LOCAL","artifact_digest":DIGEST,"created_at":TIMESTAMP
    })
}

fn identity_mapping() -> Value {
    json!({
        "schema_version":"0.2.0","mapping_id":"mapping:01","local_ref":"project-ledger://projects/01","external_system":"project-radar",
        "external_ref":"project-radar://projects/01","method":"USER_CONFIRMED","status":"CONFIRMED","confirmed_by":"human:owner","created_at":TIMESTAMP,"updated_at":TIMESTAMP,"digest":DIGEST
    })
}

fn integration_manifest() -> Value {
    json!({
        "schema_version":"0.2.0","integration_manifest_id":"integration-manifest:project-ledger","provider_id":"project-ledger","provider_version":"0.2.0",
        "protocols":[{"name":"project-ledger-integration","version":"0.2.0"}],"projection_capabilities":["PROJECT_CONTINUITY"],
        "record_capabilities":["RECORD_WORK_PROGRESS"],"side_effect_contract":{"projection":"NONE","health":"NONE","record_submit":"DECLARED_ONLY"},
        "storage":{"shared_database_required":false},"upstream_locks":[],"digest":DIGEST
    })
}

fn projection_request() -> Value {
    json!({
        "schema_version":"0.2.0","request_id":"request:01","protocol":{"name":"project-ledger-integration","version":"0.2.0"},"project_ref":PROJECT_ID,
        "repository_identity_digest":null,"requested_fields":["continuity"],"budget":{"max_bytes":65536,"max_items":100},"privacy_profile":"MINIMAL_LOCAL",
        "trace_ref":"trace:01","expected_source_version":4
    })
}

fn projection_envelope() -> Value {
    json!({
        "schema_version":"0.2.0","request_ref":"request:01","profile":"project-ledger-integration/0.2.0","source_product":"project-ledger","source_version":"0.2.0",
        "manifest_digest":DIGEST,"project_ref":PROJECT_ID,"state_version":4,"state_digest":DIGEST,"payload_schema_ref":"urn:project-ledger:schema:resume-projection:0.2.0",
        "payload_digest":DIGEST,"freshness":"CURRENT","coverage":["continuity"],"omissions":[],"privacy_profile":"MINIMAL_LOCAL",
        "provenance_ref":"project-ledger://provenance/projection-01","payload":{"projection_kind":"PROJECT_CONTINUITY","payload":resume_projection()}
    })
}

fn provider_health() -> Value {
    json!({
        "schema_version":"0.2.0","provider_id":"project-ledger","provider_version":"0.2.0","status":"HEALTHY","checked_at":TIMESTAMP,
        "supported_protocols":[{"name":"project-ledger-integration","version":"0.2.0"}],"explanation_codes":[],"digest":DIGEST
    })
}

fn loss_report() -> Value {
    json!({
        "schema_version":"0.2.0","loss_report_id":"loss:01","source_schema_ref":"urn:project-ledger:schema:project-state:0.2.0",
        "target_schema_ref":"urn:project-ledger:schema:resume-projection:0.2.0","items":[{"field_path":"event_history","severity":"INFORMATIONAL","reason":"Bounded projection"}],
        "material_loss_count":0,"created_at":TIMESTAMP,"digest":DIGEST
    })
}

fn portable_export() -> Value {
    json!({
        "schema_version":"0.2.0","export_id":"export:01","project_ref":PROJECT_ID,"project_state_version":4,"project_state_digest":DIGEST,
        "first_event_ref":EVENT_ID,"last_event_ref":EVENT_ID,"event_count":1,"chain_digest":DIGEST,"import_policy":"FAST_FORWARD",
        "schemas_lock_digest":DIGEST,"checksums_digest":DIGEST,"created_at":TIMESTAMP,"digest":DIGEST
    })
}

fn project_catalog_entry() -> Value {
    json!({
        "schema_version":"0.2.0","project_ref":"project-ledger://projects/01","canonical_title":"Project Ledger","ledger_location_ref":"workspace://.project-ledger",
        "last_seen_state_version":4,"last_seen_state_digest":DIGEST
    })
}

fn radar_projection() -> Value {
    json!({
        "schema_version":"0.1.1","projection_id":"radar-projection:01","projection_kind":"LEDGER_PROJECT_SUMMARY",
        "source":{"system":"project-ledger","product_version":"0.2.0","project_ref":"project-ledger://projects/01","state_version":4,"state_digest":DIGEST,"checkpoint_ref":null,"checkpoint_digest":null},
        "identity":{"ledger_project_ref":"project-ledger://projects/01","stable_project_id":PROJECT_ID,"repository_identity_digests":[],"source_aliases":["Project Ledger"]},
        "project":{"canonical_title":"Project Ledger","bounded_summary":"Continuidad verificable","lifecycle_status":"ACTIVE","suggested_radar_status":"FORMALIZING","status_mapping_rule":"ACTIVE_FORMALIZATION"},
        "continuity":{"primary_phase":{"phase_ref":"phase-0","title":"Contracts","kind":"FORMALIZATION","status":"ACTIVE"},"active_phases":[],
            "progress":{"method":"LINEAR_REQUIRED_PHASES","completed_required_phases":0,"total_required_phases":12,"verified_required_gates":0,"total_required_gates":1,
                "percentage":{"available":true,"basis_points":0,"unavailable_reason":null},"status":"PARTIAL"},
            "blockers":{"open_count":0,"critical_count":0},"risks":{"open_count":1,"critical_count":0},"gates":{"pending_count":1,"revalidation_required_count":0},
            "conflicts":{"critical_open_count":0},"next_action_refs":["action:schemas"],"preferred_next_action_ref":"action:schemas","last_material_activity_at":TIMESTAMP,
            "checkpoint_freshness":"CURRENT","authority_health":"CURRENT"},
        "attention_signals":[],"freshness":{"status":"CURRENT","evaluated_at":TIMESTAMP},
        "privacy":{"event_log_included":false,"evidence_payloads_included":false,"repository_paths_included":false},
        "created_at":TIMESTAMP,"provenance_ref":"project-ledger://provenance/radar-export-01","digest":DIGEST
    })
}

fn radar_sync() -> Value {
    json!({
        "schema_version":"0.1.1","sync_envelope_id":"radar-sync:01","profile":"project-ledger-radar-artifact/0.1.1","source_instance_ref":"project-ledger://instances/local-user",
        "exported_at":TIMESTAMP,"projects":[radar_projection()],"tombstones":[],"contract":{"schema_id":"urn:project-ledger:radar-sync-envelope:0.1.1",
            "profile_schema_digest":DIGEST,"source_spec_digest":DIGEST,"canonicalization":"JCS_RFC8785"},
        "semantics":{"omission_means_delete":false,"imports_are_idempotent":true,"name_only_mapping_allowed":false},"digest":DIGEST
    })
}

fn radar_project_link() -> Value {
    json!({
        "schema_version":"0.1.1","link_id":"radar-link:01","ledger_project_ref":"project-ledger://projects/01","radar_project_ref":"project-radar://projects/01",
        "status":"CONFIRMED","method":"USER_CONFIRMED","actor_ref":"human:owner","created_at":TIMESTAMP,"digest":DIGEST
    })
}

fn radar_status_policy() -> Value {
    json!({
        "schema_version":"0.1.1","policy_id":"radar-status-policy:01","rules":[{"ledger_status":"ACTIVE","phase_kind":"FORMALIZATION","radar_status":"FORMALIZING","rule_code":"ACTIVE_FORMALIZATION"}],"digest":DIGEST
    })
}

fn radar_field_policy() -> Value {
    json!({
        "schema_version":"0.1.1","policy_id":"radar-field-policy:01","rules":[{"field_path":"priority","owner":"RADAR","omission_clears":false},{"field_path":"continuity.phase","owner":"LEDGER","omission_clears":false}],"digest":DIGEST
    })
}
