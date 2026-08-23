use project_ledger_domain::{CommandEnvelope, CommandType, EventEnvelope, EventType};

const DIGEST: &str = "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";

fn start_phase_command(command_type: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": "0.2.0",
        "command_id": "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "command_schema_ref": "urn:project-ledger:command:start-phase:0.2.0",
        "project_ref": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "expected_state_version": 4,
        "idempotency_key": "session-01:start-phase:phase-0",
        "actor": {
            "actor_id": "agent:codex",
            "actor_kind": "AGENT",
            "trust_level": "LOCAL_USER",
            "display_name": "Codex"
        },
        "command_type": command_type,
        "payload": {"phase_ref": "phase-0"},
        "submitted_at": "2026-08-22T12:00:00-04:00",
        "payload_digest": DIGEST,
        "command_digest": DIGEST
    })
}

fn phase_started_event(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": "0.2.0",
        "event_id": "event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "project_ref": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "stream_id": "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
        "stream_version": 5,
        "event_type": event_type,
        "event_schema_ref": "urn:project-ledger:event:phase-started:0.2.0",
        "payload": {"phase_ref": "phase-0"},
        "actor": {
            "actor_id": "agent:codex",
            "actor_kind": "AGENT",
            "trust_level": "LOCAL_USER",
            "display_name": "Codex"
        },
        "causation": {
            "command_ref": "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d",
            "command_digest": DIGEST,
            "correlation_ref": "session:01"
        },
        "occurred_at": "2026-08-22T12:00:00-04:00",
        "recorded_at": "2026-08-22T12:00:01-04:00",
        "previous_event_digest": DIGEST,
        "payload_digest": DIGEST,
        "event_digest": DIGEST
    })
}

#[test]
fn catalogs_have_exactly_the_normative_variant_counts() {
    assert_eq!(CommandType::ALL.len(), 53);
    assert_eq!(EventType::ALL.len(), 67);
}

#[test]
fn unknown_or_derived_only_command_variants_are_rejected() {
    assert!(
        serde_json::from_value::<CommandEnvelope>(start_phase_command("COMPLETE_PHASE_DIRECTLY"))
            .is_err()
    );
    assert!(
        serde_json::from_value::<CommandEnvelope>(start_phase_command("PHASE_COMPLETED")).is_err()
    );
}

#[test]
fn command_payload_with_unknown_field_is_rejected() {
    let mut value = start_phase_command("START_PHASE");
    value["payload"]["hidden"] = serde_json::json!(true);

    assert!(serde_json::from_value::<CommandEnvelope>(value).is_err());
}

#[test]
fn command_variant_and_payload_roundtrip_as_one_discriminated_union() {
    let value = start_phase_command("START_PHASE");
    let command: CommandEnvelope = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(command.command_type(), CommandType::StartPhase);
    assert_eq!(serde_json::to_value(command).unwrap(), value);
}

#[test]
fn unknown_event_variant_and_extra_payload_are_rejected() {
    assert!(
        serde_json::from_value::<EventEnvelope>(phase_started_event("PHASE_STARTED_BY_LLM"))
            .is_err()
    );

    let mut value = phase_started_event("PHASE_STARTED");
    value["payload"]["fabricated"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EventEnvelope>(value).is_err());
}

#[test]
fn event_variant_and_payload_roundtrip_as_one_discriminated_union() {
    let value = phase_started_event("PHASE_STARTED");
    let event: EventEnvelope = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(event.event_type(), EventType::PhaseStarted);
    assert_eq!(serde_json::to_value(event).unwrap(), value);
}
