//! Behavioral inventory: every canonical command must execute through the real
//! processor/store, survive replay, reject stale writes and rebuild its events.
use std::collections::BTreeSet;

use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::{
    Actor, ActorKind, Command, CommandId, CommandResultStatus, CommandType, Event, EventEnvelope,
    EventType, LedgerTimestamp, ProjectId, SchemaVersion, StableId, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_schemas::fixture_catalog;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

const PROJECT: &str = "project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d";
const AT: &str = "2026-08-22T12:00:00-04:00";
const DIGEST: &str = "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";

#[test]
#[allow(clippy::too_many_lines)] // Explicit ordered scenario, not generated success assertions.
fn all_53_commands_have_real_atomic_replayable_rebuildable_paths() {
    let mut f = Fixture::new();
    f.send(
        "CREATE_PROJECT",
        json!({"project": record("project", json!({"lifecycle_status":"INITIALIZING"}))}),
    );
    f.send(
        "UPDATE_PROJECT_METADATA",
        json!({"title":"Operational coverage","summary":"Discardable ledger"}),
    );
    f.send(
        "REGISTER_AUTHORITY_DOCUMENT",
        json!({"document":record("authority-document",json!({}))}),
    );
    f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:spec","observation":{"outcome":"MATCH","observed_digest":DIGEST}}));
    f.send(
        "DEFINE_ROADMAP",
        json!({"roadmap":record("roadmap",json!({"phase_order":["phase-0","phase-1"]}))}),
    );
    f.send("REVISE_ROADMAP", json!({"roadmap":record("roadmap",json!({"version":2,"phase_order":["phase-0","phase-1"]}))}));
    f.send(
        "BIND_REPOSITORY",
        json!({"binding":record("repository-binding",json!({}))}),
    );
    f.send("REFRESH_REPOSITORY_BINDING", json!({"binding_ref":"binding:ledger","snapshot":{"repository_ref":"repo:ledger","snapshot_ref":"snapshot:01","snapshot_digest":DIGEST}}));
    let gate = record("gate-definition", json!({}));
    f.send("DEFINE_GATE", json!({"gate":gate}));
    f.send("DEFINE_GATE", json!({"gate":record("gate-definition",json!({"gate_id":"gate:negative","criticality":"OPTIONAL","required_evidence_rules":[{"rule_id":"rule:negative","kind":"TEST_RESULT","predicate":{"operator":"EQUALS","path":"result.exit_code","value":1}}]}))}));
    f.send("START_PHASE", json!({"phase_ref":"phase-0"}));
    f.send(
        "RECORD_CLAIM",
        json!({"claim":record("claim-record",json!({}))}),
    );
    f.send(
        "RECORD_UNKNOWN",
        json!({"unknown":record("unknown-record",json!({}))}),
    );
    f.send(
        "RESOLVE_UNKNOWN",
        json!({"unknown_ref":"unknown:01","resolution_ref":"workspace://reports/clarification"}),
    );
    f.send(
        "RECORD_WORK_PROGRESS",
        json!({"work_record":record("work-record",json!({"actor_ref":"human:owner"}))}),
    );
    let note_events = f.send(
        "RECORD_NOTE",
        json!({"kind":"LIMITATION","text":"Descriptive fact","scope_ref":"phase-0"}),
    );
    f.send(
        "ADD_ARTIFACT_REFERENCE",
        json!({"artifact":record("artifact-reference",json!({}))}),
    );
    f.send(
        "START_SESSION",
        json!({"session":record("session",json!({"actor_ref":"human:owner"}))}),
    );
    f.send(
        "CREATE_CONTINUITY_CHECKPOINT",
        json!({"reason":"coverage checkpoint"}),
    );
    f.send(
        "END_SESSION",
        json!({"session_ref":"session:01","create_checkpoint":true}),
    );
    f.send(
        "RECORD_VERIFICATION_RESULT",
        json!({"evidence":record("evidence-record",json!({}))}),
    );
    f.send(
        "UPDATE_CLAIM_STATUS",
        json!({"claim_ref":"claim:01","classification":"OBSERVED","evidence_refs":["evidence:01"]}),
    );
    f.send(
        "REQUEST_GATE_EVALUATION",
        json!({"gate_ref":"gate:negative"}),
    );
    assert_eq!(
        f.projection().gate_status(&id("gate:negative")),
        Some(project_ledger_domain::GateStatus::Rejected)
    );
    f.send(
        "REQUEST_GATE_EVALUATION",
        json!({"gate_ref":"gate:contracts"}),
    );
    f.send_as(
        "REQUEST_PHASE_COMPLETION",
        json!({"phase_ref":"phase-0"}),
        ActorKind::Agent,
        TrustLevel::TrustedForRecording,
    );
    assert_eq!(
        f.projection().phase_status(&id("phase-0")),
        Some(project_ledger_domain::PhaseStatus::ReadyToClose)
    );
    f.send("REQUEST_PHASE_COMPLETION", json!({"phase_ref":"phase-0"}));
    f.send("SKIP_PHASE", json!({"phase_ref":"phase-1","reason":"Explicitly excluded","authority_ref":"authority:spec"}));
    f.send("REOPEN_PHASE", json!({"phase_ref":"phase-0","reason":"explicit follow-up","authority_ref":"authority:spec"}));
    let linked = f.send("LINK_EXTERNAL_EVIDENCE", json!({"external_system":"gsp","artifact_uri":"workspace://gsp/report.json","artifact_digest":DIGEST,"schema_ref":"urn:test:report:1"}));
    let external_ref = linked
        .iter()
        .find_map(|event| {
            if let Event::ExternalEvidenceLinked(link) = &event.event {
                Some(link.evidence_ref.clone())
            } else {
                None
            }
        })
        .unwrap();
    f.send(
        "UNLINK_EXTERNAL_EVIDENCE",
        json!({"evidence_ref":external_ref,"reason":"explicit unlink"}),
    );
    f.send(
        "INVALIDATE_EVIDENCE",
        json!({"evidence_ref":"evidence:01","reason":"changed inputs"}),
    );
    f.send(
        "REQUEST_GATE_REVALIDATION",
        json!({"gate_ref":"gate:contracts","signal_ref":null}),
    );
    f.send(
        "REQUEST_GATE_EVALUATION",
        json!({"gate_ref":"gate:contracts"}),
    );
    let waiver = record(
        "gate-waiver",
        json!({"gate_definition_digest":gate["digest"]}),
    );
    f.send("WAIVE_GATE", json!({"waiver":waiver}));

    let decision = record(
        "decision",
        json!({"status":"PROPOSED","accepted_at":null,"authority_ref":null}),
    );
    f.send("PROPOSE_DECISION", json!({"decision":decision}));
    f.send(
        "ACCEPT_DECISION",
        json!({"decision_ref":"decision:rust","authority_ref":"authority:spec"}),
    );
    let replacement = record(
        "decision",
        json!({"decision_id":"decision:rust-v2","status":"PROPOSED","accepted_at":null,"authority_ref":null,"version":2,"supersedes_ref":"decision:rust"}),
    );
    f.send("PROPOSE_DECISION", json!({"decision":replacement}));
    f.send(
        "ACCEPT_DECISION",
        json!({"decision_ref":"decision:rust-v2","authority_ref":"authority:spec"}),
    );
    let accepted = f
        .projection()
        .decision(&id("decision:rust-v2"))
        .unwrap()
        .clone();
    f.send(
        "SUPERSEDE_DECISION",
        json!({"decision_ref":"decision:rust","replacement":accepted}),
    );
    f.send("PROPOSE_DECISION", json!({"decision":record("decision",json!({"decision_id":"decision:rejected","status":"PROPOSED","accepted_at":null,"authority_ref":null}))}));
    f.send(
        "REJECT_DECISION",
        json!({"decision_ref":"decision:rejected","reason":"Not selected"}),
    );

    f.send("OPEN_BLOCKER", json!({"blocker":record("blocker",json!({"scope_kind":"PHASE","scope_ref":"phase-0","severity":"CRITICAL"}))}));
    f.send("UPDATE_BLOCKER", json!({"blocker":record("blocker",json!({"scope_kind":"PHASE","scope_ref":"phase-0","severity":"CRITICAL","description":"Reproduced dependency problem"}))}));
    f.send("RESOLVE_BLOCKER", json!({"blocker_ref":"blocker:01","evidence_refs":["evidence:01"],"resolution_summary":"Resolved explicitly"}));
    f.send(
        "OPEN_RISK",
        json!({"risk":record("risk",json!({"status":"OPEN"}))}),
    );
    f.send(
        "UPDATE_RISK",
        json!({"risk":record("risk",json!({"status":"OPEN","description":"Reviewed risk"}))}),
    );
    f.send(
        "MITIGATE_RISK",
        json!({"risk_ref":"risk:01","note":"Mitigation applied","evidence_refs":[]}),
    );
    f.send(
        "CLOSE_RISK",
        json!({"risk_ref":"risk:01","note":"Reviewed closure","evidence_refs":[]}),
    );
    f.send(
        "SET_NEXT_ACTION",
        json!({"action":record("next-action",json!({}))}),
    );
    f.send(
        "COMPLETE_NEXT_ACTION",
        json!({"action_ref":"action:schemas","evidence_refs":["evidence:01"]}),
    );
    f.send(
        "SET_NEXT_ACTION",
        json!({"action":record("next-action",json!({"next_action_id":"action:cancelled"}))}),
    );
    f.send(
        "CANCEL_NEXT_ACTION",
        json!({"action_ref":"action:cancelled","reason":"No longer applicable"}),
    );

    f.send("CONFIRM_EXTERNAL_MAPPING", json!({"mapping":record("identity-mapping",json!({"local_ref":format!("project-ledger://project/{}",project().uuid()),"confirmed_by":"human:owner","status":"CONFIRMED"}))}));
    let mapping_id = record("identity-mapping", json!({}))["mapping_id"].clone();
    f.send(
        "REVOKE_EXTERNAL_MAPPING",
        json!({"mapping_ref":mapping_id,"reason":"Revoked explicitly"}),
    );
    f.send("RECORD_EVENT_CORRECTION", json!({"corrects_event_ref":note_events[0].event_id,"reason":"Descriptive correction","replacement_field_values":[{"field_path":"text","replacement":"Corrected description"}]}));
    let replacement = record(
        "authority-document",
        json!({"document_id":"authority:new","version":"0.2.1","supersedes_ref":"authority:spec"}),
    );
    f.send(
        "REGISTER_AUTHORITY_DOCUMENT",
        json!({"document":replacement}),
    );
    f.send(
        "SUPERSEDE_AUTHORITY_DOCUMENT",
        json!({"document_ref":"authority:spec","replacement":replacement}),
    );
    assert_eq!(
        f.projection()
            .decision(&id("decision:rust-v2"))
            .unwrap()
            .status,
        project_ledger_domain::DecisionStatus::RevalidationRequired
    );
    let mismatch = f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:new","observation":{"outcome":"MISMATCH","observed_digest":format!("sha256:{}","ab".repeat(32))}}));
    let conflict = mismatch
        .iter()
        .find_map(|event| {
            if let Event::ConflictOpened(opened) = &event.event {
                Some(opened.conflict.conflict_id.clone())
            } else {
                None
            }
        })
        .unwrap();
    f.send("RESOLVE_CONFLICT", json!({"conflict_ref":conflict,"resolution_ref":"workspace://authority/review","reason":"Reviewed discrepancy"}));
    f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:new","observation":{"outcome":"UNAVAILABLE","reason":"File unavailable in test"}}));
    f.send("REGISTER_AUTHORITY_DOCUMENT", json!({"document":record("authority-document",json!({"document_id":"authority:current","authority_rank":0}))}));
    f.send("PAUSE_PROJECT", json!({"reason":"Human pause"}));
    f.send("RESUME_PROJECT", json!({"reason":"Human resume"}));
    f.send(
        "REVOKE_REPOSITORY_BINDING",
        json!({"binding_ref":"binding:ledger","reason":"End of synthetic workflow"}),
    );
    f.send(
        "ARCHIVE_PROJECT",
        json!({"reason":"Completed synthetic run"}),
    );

    assert_eq!(
        f.commands,
        CommandType::ALL.iter().copied().collect(),
        "each canonical command needs an actually accepted transaction"
    );
    assert!(f.events.contains(&EventType::SessionEnded));
    assert!(f.events.contains(&EventType::EventCorrectionRecorded));
    let missing: BTreeSet<_> = EventType::ALL
        .iter()
        .copied()
        .filter(|kind| !f.events.contains(kind))
        .collect();
    assert_eq!(
        missing,
        BTreeSet::from([EventType::LegacyImportReviewed]),
        "all executable events must have an observed path; legacy review has no admitted v0.2 command"
    );
    assert_eq!(
        f.projection()
            .state()
            .project
            .archived_at
            .as_ref()
            .unwrap()
            .as_str(),
        AT
    );
}

struct Fixture {
    _directory: TempDir,
    store: SqliteEventStore,
    ordinal: u32,
    commands: BTreeSet<CommandType>,
    events: BTreeSet<EventType>,
}

#[test]
fn decision_loses_authority_without_silent_reacceptance() {
    let mut f = Fixture::new();
    f.send(
        "CREATE_PROJECT",
        json!({"project":record("project",json!({"lifecycle_status":"INITIALIZING"}))}),
    );
    f.send(
        "REGISTER_AUTHORITY_DOCUMENT",
        json!({"document":record("authority-document",json!({}))}),
    );
    f.send("PROPOSE_DECISION", json!({"decision":record("decision",json!({"status":"PROPOSED","accepted_at":null,"authority_ref":null}))}));
    f.send(
        "ACCEPT_DECISION",
        json!({"decision_ref":"decision:rust","authority_ref":"authority:spec"}),
    );
    let matched = f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:spec","observation":{"outcome":"MATCH","observed_digest":DIGEST}}));
    assert!(
        !matched
            .iter()
            .any(|event| event.event_type() == EventType::DecisionRevalidationRequired)
    );
    let unrelated = f.send("REGISTER_AUTHORITY_DOCUMENT", json!({"document":record("authority-document",json!({"document_id":"authority:unrelated"}))}));
    assert!(
        !unrelated
            .iter()
            .any(|event| event.event_type() == EventType::DecisionRevalidationRequired)
    );
    f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:spec","observation":{"outcome":"UNAVAILABLE","reason":"Unavailable source"}}));
    assert_eq!(
        f.projection()
            .decision(&id("decision:rust"))
            .unwrap()
            .status,
        project_ledger_domain::DecisionStatus::RevalidationRequired
    );
    f.send("REFRESH_AUTHORITY_DOCUMENT_DIGEST", json!({"document_ref":"authority:spec","observation":{"outcome":"MATCH","observed_digest":DIGEST}}));
    assert_eq!(
        f.projection()
            .decision(&id("decision:rust"))
            .unwrap()
            .status,
        project_ledger_domain::DecisionStatus::RevalidationRequired
    );
    f.send(
        "ACCEPT_DECISION",
        json!({"decision_ref":"decision:rust","authority_ref":"authority:spec"}),
    );
    assert_eq!(
        f.projection()
            .decision(&id("decision:rust"))
            .unwrap()
            .status,
        project_ledger_domain::DecisionStatus::Accepted
    );
}

#[test]
fn observing_superseded_authority_never_restores_its_authority() {
    let mut f = Fixture::new();
    f.send(
        "CREATE_PROJECT",
        json!({"project":record("project",json!({"lifecycle_status":"INITIALIZING"}))}),
    );
    f.send(
        "REGISTER_AUTHORITY_DOCUMENT",
        json!({"document":record("authority-document",json!({}))}),
    );
    let replacement = record(
        "authority-document",
        json!({"document_id":"authority:replacement","supersedes_ref":"authority:spec"}),
    );
    f.send(
        "REGISTER_AUTHORITY_DOCUMENT",
        json!({"document":replacement}),
    );
    f.send(
        "SUPERSEDE_AUTHORITY_DOCUMENT",
        json!({"document_ref":"authority:spec","replacement":replacement}),
    );
    for observation in [
        json!({"outcome":"UNAVAILABLE","reason":"Old source missing"}),
        json!({"outcome":"MATCH","observed_digest":DIGEST}),
        json!({"outcome":"MISMATCH","observed_digest":format!("sha256:{}","ab".repeat(32))}),
        json!({"outcome":"MATCH","observed_digest":DIGEST}),
    ] {
        f.send(
            "REFRESH_AUTHORITY_DOCUMENT_DIGEST",
            json!({"document_ref":"authority:spec","observation":observation}),
        );
        assert_eq!(
            f.projection()
                .authority_document(&id("authority:spec"))
                .unwrap()
                .status,
            project_ledger_domain::AuthorityDocumentStatus::Superseded
        );
    }
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
        Self {
            _directory: directory,
            store,
            ordinal: 0,
            commands: BTreeSet::new(),
            events: BTreeSet::new(),
        }
    }

    fn send(&mut self, name: &str, payload: Value) -> Vec<EventEnvelope> {
        self.send_as(name, payload, ActorKind::Human, TrustLevel::LocalUser)
    }

    fn send_as(
        &mut self,
        name: &str,
        payload: Value,
        actor_kind: ActorKind,
        trust_level: TrustLevel,
    ) -> Vec<EventEnvelope> {
        let command = canonical_command(name, payload);
        self.ordinal += 1;
        let before = self.store.load_stream(&project()).unwrap();
        let envelope = prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: CommandId::parse(&format!(
                "command:0198f8c8-7417-7fb0-b88f-0fea{:08x}",
                self.ordinal
            ))
            .unwrap(),
            project_ref: project(),
            expected_state_version: before.len() as u64,
            idempotency_key: id(&format!("coverage:{}", self.ordinal)),
            actor: Actor {
                actor_id: id("human:owner"),
                actor_kind,
                trust_level,
                display_name: None,
            },
            command,
            submitted_at: LedgerTimestamp::parse(AT).unwrap(),
        })
        .unwrap();
        let clock = FixedClock::new(LedgerTimestamp::parse(AT).unwrap());
        let result = CommandProcessor::new(&mut self.store, &clock)
            .execute(envelope.clone(), &CoreCommandHandler)
            .unwrap();
        assert_eq!(
            result.status,
            CommandResultStatus::Accepted,
            "{name}: {:?}",
            result.rejection
        );
        self.commands.insert(envelope.command_type());
        let stream = self.store.load_stream(&project()).unwrap();
        assert_eq!(&stream[..before.len()], &before);
        let emitted = stream[before.len()..].to_vec();
        self.events
            .extend(emitted.iter().map(EventEnvelope::event_type));
        let replay = CommandProcessor::new(&mut self.store, &clock)
            .execute(envelope.clone(), &CoreCommandHandler)
            .unwrap();
        assert_eq!(
            replay.status,
            CommandResultStatus::DuplicateReplay,
            "{name}"
        );
        assert_eq!(self.store.load_stream(&project()).unwrap(), stream);
        let mut untrusted_actor = envelope.actor.clone();
        untrusted_actor.trust_level = TrustLevel::Untrusted;
        let denied = prepare_command(CommandDraft {
            schema_version: envelope.schema_version,
            command_id: CommandId::parse(&format!(
                "command:0198f8c8-7417-7fb0-b88f-0fec{:08x}",
                self.ordinal
            ))
            .unwrap(),
            project_ref: envelope.project_ref.clone(),
            expected_state_version: stream.len() as u64,
            idempotency_key: id(&format!("coverage:denied:{}", self.ordinal)),
            actor: untrusted_actor,
            command: envelope.command.clone(),
            submitted_at: envelope.submitted_at.clone(),
        })
        .unwrap();
        assert_eq!(
            CommandProcessor::new(&mut self.store, &clock)
                .execute(denied, &CoreCommandHandler)
                .unwrap()
                .status,
            CommandResultStatus::PolicyDenied,
            "{name}"
        );
        assert_eq!(self.store.load_stream(&project()).unwrap(), stream);
        let stale = prepare_command(CommandDraft {
            schema_version: envelope.schema_version,
            command_id: CommandId::parse(&format!(
                "command:0198f8c8-7417-7fb0-b88f-0feb{:08x}",
                self.ordinal
            ))
            .unwrap(),
            project_ref: envelope.project_ref,
            expected_state_version: before.len() as u64 + 1000,
            idempotency_key: id(&format!("coverage:stale:{}", self.ordinal)),
            actor: envelope.actor,
            command: envelope.command,
            submitted_at: envelope.submitted_at,
        })
        .unwrap();
        assert_eq!(
            CommandProcessor::new(&mut self.store, &clock)
                .execute(stale, &CoreCommandHandler)
                .unwrap()
                .status,
            CommandResultStatus::VersionConflict,
            "{name}"
        );
        assert_eq!(self.store.load_stream(&project()).unwrap(), stream);
        self.assert_consistent_rebuild(name, &stream);
        emitted
    }

    fn assert_consistent_rebuild(&self, name: &str, stream: &[EventEnvelope]) {
        self.store.verify_event_chain(&project()).unwrap();
        let rebuilt = ProjectStateProjection::rebuild(stream).unwrap();
        let mut cursor = ProjectStateProjection::rebuild(&stream[..1]).unwrap();
        for event in &stream[1..] {
            cursor = cursor.clone().apply(cursor.head_digest(), event).unwrap();
        }
        assert_eq!(
            rebuilt.state(),
            cursor.state(),
            "{name} incremental versus rebuild"
        );
    }

    fn projection(&self) -> ProjectStateProjection {
        ProjectStateProjection::rebuild(&self.store.load_stream(&project()).unwrap()).unwrap()
    }
}

// Canonical schema examples supply input records only. Acceptance, expected
// status, version preservation and actor identity are asserted independently.
fn canonical_command(name: &str, payload: Value) -> Command {
    let mut value = json!({"command_type":name});
    value["payload"] = payload;
    serde_json::from_value(value).unwrap()
}

fn record(name: &str, changes: Value) -> Value {
    let suffix = format!(":{name}:0.2.0");
    let mut value = fixture_catalog()
        .unwrap()
        .into_iter()
        .find(|entry| entry.schema_id.ends_with(&suffix))
        .unwrap_or_else(|| panic!("missing fixture {name}"))
        .positive;
    let Value::Object(changes) = changes else {
        panic!("record overrides must be an object")
    };
    for (key, item) in changes {
        value[&key] = item;
    }
    let digest_field = if name == "evidence-record" {
        "payload_digest"
    } else {
        "digest"
    };
    if name != "artifact-reference"
        && name != "authority-document"
        && value.get(digest_field).is_some()
    {
        value.as_object_mut().unwrap().remove(digest_field);
        let digest = sha256_digest(&value).unwrap();
        value[digest_field] = json!(digest);
    }
    value
}
fn project() -> ProjectId {
    ProjectId::parse(PROJECT).unwrap()
}
fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
