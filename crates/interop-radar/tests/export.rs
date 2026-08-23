use std::collections::BTreeMap;

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityDocument, AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus,
    AuthorityPolicy, Command, CommandId, CreateProjectPayload, Event, EventEnvelope, EventId,
    FreshnessStatus, LedgerTimestamp, Phase, PhaseKind, PhaseStartedEvent, PhaseStatus, Project,
    ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus, ProjectStatusDerivedEvent, ResourceRef,
    Roadmap, RoadmapCompletionPolicy, RoadmapDefinedEvent, RoadmapExecutionModel, SchemaVersion,
    StableId, StorageProfile, SuggestedRadarStatus, TrustLevel,
};
use project_ledger_event_store::{CommandDraft, EventDraft, build_event, prepare_command};
use project_ledger_interop_radar::{
    RADAR_PROFILE, RadarEnvelopeRequest, RadarExportContext, build_project_projection,
    build_sync_envelope, suggested_radar_status,
};
use project_ledger_projection_engine::ProjectStateProjection;

#[test]
fn pl_29_status_mapping_is_exact_and_never_invents_not_implemented() {
    for (ledger, phase, ready, expected, rule) in [
        (
            ProjectLifecycleStatus::Initializing,
            None,
            false,
            SuggestedRadarStatus::Formalizing,
            "INITIALIZING_INCOMPLETE",
        ),
        (
            ProjectLifecycleStatus::Initializing,
            None,
            true,
            SuggestedRadarStatus::SpecReady,
            "INITIALIZING_SPEC_READY_NO_PHASE",
        ),
        (
            ProjectLifecycleStatus::Active,
            Some(PhaseKind::Implementation),
            false,
            SuggestedRadarStatus::InDevelopment,
            "ACTIVE_IMPLEMENTATION",
        ),
        (
            ProjectLifecycleStatus::Active,
            Some(PhaseKind::Testing),
            false,
            SuggestedRadarStatus::Testing,
            "ACTIVE_TESTING",
        ),
        (
            ProjectLifecycleStatus::Active,
            Some(PhaseKind::Audit),
            false,
            SuggestedRadarStatus::Auditing,
            "ACTIVE_AUDIT",
        ),
        (
            ProjectLifecycleStatus::Blocked,
            None,
            false,
            SuggestedRadarStatus::Blocked,
            "PROJECT_BLOCKED",
        ),
        (
            ProjectLifecycleStatus::Paused,
            None,
            false,
            SuggestedRadarStatus::Paused,
            "PROJECT_PAUSED",
        ),
        (
            ProjectLifecycleStatus::Completed,
            None,
            false,
            SuggestedRadarStatus::Done,
            "PROJECT_COMPLETED",
        ),
        (
            ProjectLifecycleStatus::Archived,
            None,
            false,
            SuggestedRadarStatus::Archived,
            "PROJECT_ARCHIVED",
        ),
    ] {
        let mapped = suggested_radar_status(ledger, phase, ready).unwrap();
        assert_eq!(mapped.status, expected);
        assert_eq!(mapped.rule_code, rule);
        assert_ne!(mapped.rule_code, "NOT_IMPLEMENTED");
    }
    assert!(
        suggested_radar_status(
            ProjectLifecycleStatus::Active,
            Some(PhaseKind::Formalization),
            false,
        )
        .is_err()
    );
}

#[test]
fn pl_21_pl_30_projection_and_envelope_are_private_read_only_and_deterministic() {
    let projection = active_projection();
    let phase = phase();
    let context = RadarExportContext {
        phase_catalog: BTreeMap::from([(phase.phase_id.clone(), phase)]),
        source_aliases: vec!["MemPilot".to_owned(), "mempilot".to_owned()],
        observed_at: at(8),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/radar/project-state-5")
            .unwrap(),
    };

    let first = build_project_projection(&projection, &context).unwrap();
    let second = build_project_projection(&projection, &context).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.schema_version, SchemaVersion::V0_1_1);
    assert_eq!(
        first.project.suggested_radar_status,
        SuggestedRadarStatus::InDevelopment
    );
    assert_eq!(first.project.status_mapping_rule, "ACTIVE_IMPLEMENTATION");
    assert_eq!(first.continuity.progress.completed_required_phases, 0);
    assert_eq!(first.continuity.progress.total_required_phases, 1);
    assert!(first.continuity.progress.percentage.available);
    assert_eq!(first.continuity.progress.percentage.basis_points, Some(0));
    assert_eq!(first.freshness.status, FreshnessStatus::Current);
    assert!(!first.privacy.event_log_included);
    assert!(!first.privacy.evidence_payloads_included);
    assert!(!first.privacy.repository_paths_included);
    let json = serde_json::to_value(&first).unwrap();
    assert!(json.pointer("/identity/radar_project_ref").is_none());
    assert!(!json.to_string().contains("IndexedDB"));
    assert!(!json.to_string().contains("NOT_IMPLEMENTED"));
    assert_self_digest(&json);

    let envelope_request = RadarEnvelopeRequest {
        sync_envelope_id: id("radar-sync:fixture"),
        source_instance_ref: ResourceRef::parse("project-ledger://instances/local").unwrap(),
        exported_at: at(8),
    };
    let envelope = build_sync_envelope(vec![first], &envelope_request).unwrap();
    assert_eq!(envelope.profile, RADAR_PROFILE);
    assert_eq!(envelope.projects.len(), 1);
    assert!(envelope.tombstones.is_empty());
    assert!(!envelope.semantics.omission_means_delete);
    assert!(envelope.semantics.imports_are_idempotent);
    assert!(!envelope.semantics.name_only_mapping_allowed);
    envelope.validate().unwrap();
    assert_self_digest(&serde_json::to_value(envelope).unwrap());
}

#[test]
fn pl_30_incomplete_project_exports_unavailable_progress_not_zero_facts() {
    let projection = ProjectStateProjection::rebuild(&Stream::genesis().events).unwrap();
    let context = RadarExportContext {
        phase_catalog: BTreeMap::new(),
        source_aliases: Vec::new(),
        observed_at: at(2),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/radar/genesis").unwrap(),
    };
    let result = build_project_projection(&projection, &context).unwrap();

    assert_eq!(
        result.project.suggested_radar_status,
        SuggestedRadarStatus::Formalizing
    );
    assert!(!result.continuity.progress.percentage.available);
    assert_eq!(result.continuity.progress.percentage.basis_points, None);
    assert_eq!(
        result
            .continuity
            .progress
            .percentage
            .unavailable_reason
            .as_deref(),
        Some("ROADMAP_UNAVAILABLE")
    );
    let signals = result
        .attention_signals
        .iter()
        .map(|signal| signal.code.as_str())
        .collect::<Vec<_>>();
    assert!(signals.contains(&"LEDGER_NO_ELIGIBLE_NEXT_ACTION"));
    assert!(!signals.contains(&"LEDGER_NO_MATERIAL_ACTIVITY"));
}

fn assert_self_digest(value: &serde_json::Value) {
    let expected = value["digest"].as_str().unwrap();
    let mut unsigned = value.clone();
    unsigned.as_object_mut().unwrap().remove("digest");
    assert_eq!(sha256_digest(&unsigned).unwrap().to_string(), expected);
}

fn active_projection() -> ProjectStateProjection {
    let mut stream = Stream::genesis();
    stream.push(Event::AuthorityDocumentRegistered(
        AuthorityDocumentRegisteredEvent {
            document: AuthorityDocument {
                schema_version: SchemaVersion::V0_2_0,
                document_id: id("authority:spec"),
                uri: ResourceRef::parse("workspace://MEMPILOT_SPEC.md").unwrap(),
                digest: Sha256Digest::from([1; 32]),
                version: "1.0.0".to_owned(),
                status: AuthorityDocumentStatus::Current,
                authority_rank: 1,
                effective_from: at(2),
                supersedes_ref: None,
            },
        },
    ));
    stream.push(Event::RoadmapDefined(RoadmapDefinedEvent {
        roadmap: Roadmap {
            schema_version: SchemaVersion::V0_2_0,
            roadmap_id: id("roadmap:main"),
            title: "Main".to_owned(),
            execution_model: RoadmapExecutionModel::Sequential,
            phase_order: vec![id("phase:implementation")],
            completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
            version: 1,
            source_document_ref: id("authority:spec"),
        },
    }));
    stream.push(Event::PhaseStarted(PhaseStartedEvent {
        phase_ref: id("phase:implementation"),
    }));
    stream.push(Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
        previous_status: ProjectLifecycleStatus::Initializing,
        new_status: ProjectLifecycleStatus::Active,
        reason_codes: vec!["PHASE_STARTED".to_owned()],
    }));
    ProjectStateProjection::rebuild(&stream.events).unwrap()
}

fn phase() -> Phase {
    Phase {
        schema_version: SchemaVersion::V0_2_0,
        phase_id: id("phase:implementation"),
        title: "Implementation".to_owned(),
        ordinal: 1,
        kind: PhaseKind::Implementation,
        status: PhaseStatus::Active,
        required_gate_refs: Vec::new(),
        optional_gate_refs: Vec::new(),
        started_at: Some(at(4)),
        completed_at: None,
        reopened_at: None,
    }
}

struct Stream {
    events: Vec<EventEnvelope>,
}

impl Stream {
    fn genesis() -> Self {
        Self {
            events: vec![event(
                1,
                None,
                Event::ProjectCreated(ProjectCreatedEvent { project: project() }),
            )],
        }
    }

    fn push(&mut self, payload: Event) {
        self.events.push(event(
            self.events.len() as u64 + 1,
            self.events.last().map(|item| item.event_digest),
            payload,
        ));
    }
}

fn event(version: u64, previous: Option<Sha256Digest>, payload: Event) -> EventEnvelope {
    let command = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project_id(),
        expected_state_version: version.saturating_sub(1),
        idempotency_key: id(&format!("radar-export:{version}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: Command::CreateProject(CreateProjectPayload { project: project() }),
        submitted_at: at(version),
    })
    .unwrap();
    build_event(
        &command,
        version,
        previous,
        EventDraft {
            event_id: EventId::new(),
            event: payload,
            correlation_ref: id("correlation:radar-export"),
            occurred_at: at(version),
            recorded_at: at(version),
        },
    )
    .unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "MemPilot".to_owned(),
        summary: "Local continuity fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(1),
        timezone: "America/Santiago".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn at(minute: u64) -> LedgerTimestamp {
    LedgerTimestamp::parse(&format!("2026-08-23T10:{minute:02}:00-04:00")).unwrap()
}
