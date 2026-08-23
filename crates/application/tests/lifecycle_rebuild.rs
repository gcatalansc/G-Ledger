use project_ledger_application::{
    CommandDraft, CommandProcessor, CreateProjectHandler, FixedClock, ProjectStateQuery,
    ProjectionSource, prepare_command,
};
use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    AcceptDecisionPayload, Actor, ActorKind, AuthorityDocument, AuthorityDocumentStatus,
    AuthorityPolicy, Blocker, BlockerStatus, Command, CommandEnvelope, CommandId,
    CreateProjectPayload, Decision, DecisionStatus, DefineRoadmapPayload, LedgerTimestamp,
    LikelihoodClass, NextAction, NextActionClassification, OpenBlockerPayload, OpenRiskPayload,
    Project, ProjectId, ProjectLifecycleStatus, ProposeDecisionPayload,
    RegisterAuthorityDocumentPayload, ResolveBlockerPayload, ResourceRef, Risk, RiskStatus,
    Roadmap, RoadmapCompletionPolicy, RoadmapExecutionModel, SchemaVersion, ScopeKind,
    SetNextActionPayload, Severity, StableId, StartPhasePayload, StorageProfile,
    SupersedeDecisionPayload, TrustLevel,
};
use project_ledger_event_store::EventStore;
use project_ledger_lifecycle::LifecycleEngine;
use project_ledger_projection_engine::{ProjectStateProjection, ProjectStateProjector};
use project_ledger_storage_sqlite::SqliteEventStore;
use rusqlite::Connection;
use serde::Serialize;
use tempfile::tempdir;

#[test]
#[allow(clippy::too_many_lines)] // The linear E2E command sequence is intentionally audit-readable.
fn full_lifecycle_rebuild_matches_and_repairs_a_corrupt_discardable_cache() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("ledger.db");
    let mut store = SqliteEventStore::open(&path).unwrap();
    let clock = FixedClock::new(at());

    CommandProcessor::new(&mut store, &clock)
        .execute(create_command(), &CreateProjectHandler)
        .unwrap();
    for (expected, command_id, payload) in [
        (
            1,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df81",
            Command::RegisterAuthorityDocument(RegisterAuthorityDocumentPayload {
                document: authority(),
            }),
        ),
        (
            2,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df82",
            Command::DefineRoadmap(DefineRoadmapPayload { roadmap: roadmap() }),
        ),
        (
            3,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df83",
            Command::StartPhase(StartPhasePayload {
                phase_ref: id("phase:implementation"),
            }),
        ),
    ] {
        execute(&mut store, &clock, command(payload, expected, command_id));
    }
    execute(
        &mut store,
        &clock,
        command(
            Command::OpenRisk(OpenRiskPayload { risk: risk() }),
            5,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df84",
        ),
    );
    execute(
        &mut store,
        &clock,
        command(
            Command::OpenBlocker(OpenBlockerPayload { blocker: blocker() }),
            6,
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df85",
        ),
    );
    execute_next(
        &mut store,
        &clock,
        Command::ResolveBlocker(ResolveBlockerPayload {
            blocker_ref: id("blocker:dependency"),
            evidence_refs: vec![id("evidence:dependency-restored")],
            resolution_summary: "dependency restored".to_owned(),
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df86",
    );

    let first = decision("decision:storage-v1", 1, None);
    execute_next(
        &mut store,
        &clock,
        Command::ProposeDecision(ProposeDecisionPayload {
            decision: first.clone(),
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df87",
    );
    execute_next(
        &mut store,
        &clock,
        Command::AcceptDecision(AcceptDecisionPayload {
            decision_ref: first.decision_id,
            authority_ref: id("authority:canonical"),
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df88",
    );
    let replacement = decision("decision:storage-v2", 2, Some(id("decision:storage-v1")));
    execute_next(
        &mut store,
        &clock,
        Command::ProposeDecision(ProposeDecisionPayload {
            decision: replacement.clone(),
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df89",
    );
    execute_next(
        &mut store,
        &clock,
        Command::AcceptDecision(AcceptDecisionPayload {
            decision_ref: replacement.decision_id.clone(),
            authority_ref: id("authority:canonical"),
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df8a",
    );
    let projection =
        ProjectStateProjection::rebuild(&store.load_stream(&project_id()).unwrap()).unwrap();
    let accepted_replacement = projection
        .decision(&replacement.decision_id)
        .unwrap()
        .clone();
    execute_next(
        &mut store,
        &clock,
        Command::SupersedeDecision(SupersedeDecisionPayload {
            decision_ref: id("decision:storage-v1"),
            replacement: accepted_replacement,
        }),
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df8b",
    );

    for (command_id, action) in [
        (
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df8c",
            action("action:proposed", NextActionClassification::Proposed, None),
        ),
        (
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df8d",
            action(
                "action:authorized",
                NextActionClassification::Authorized,
                Some(id("authority:canonical")),
            ),
        ),
        (
            "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df8e",
            action("action:eligible", NextActionClassification::Eligible, None),
        ),
    ] {
        execute_next(
            &mut store,
            &clock,
            Command::SetNextAction(SetNextActionPayload { action }),
            command_id,
        );
    }

    let stream = store.load_stream(&project_id()).unwrap();
    let exact_rebuild = ProjectStateProjector::rebuild(&stream).unwrap();
    let event_count = store.event_count(&project_id()).unwrap();
    let state_version = store.current_version(&project_id()).unwrap();
    let first_read = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();

    assert_eq!(first_read.source, ProjectionSource::Rebuilt);
    assert_eq!(first_read.state, exact_rebuild);
    assert_eq!(
        first_read.state.project.lifecycle_status,
        ProjectLifecycleStatus::Active
    );
    assert_eq!(first_read.state.open_risk_refs, vec![id("risk:locking")]);
    assert!(first_read.state.open_blocker_refs.is_empty());
    assert_eq!(
        first_read.state.current_decision_refs,
        vec![id("decision:storage-v2")]
    );
    assert_eq!(
        first_read.state.ordered_next_action_refs,
        vec![
            id("action:authorized"),
            id("action:eligible"),
            id("action:proposed")
        ]
    );
    assert_eq!(
        first_read.state.preferred_next_action_ref,
        Some(id("action:authorized"))
    );
    assert_eq!(
        ProjectStateQuery::new(&mut store)
            .load(&project_id())
            .unwrap()
            .source,
        ProjectionSource::Snapshot
    );

    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE project_state_snapshots SET state_json = X'7B7D' WHERE project_ref = ?1",
        [project_id().as_str()],
    )
    .unwrap();
    drop(raw);

    let recovered = ProjectStateQuery::new(&mut store)
        .load(&project_id())
        .unwrap();
    assert_eq!(recovered.source, ProjectionSource::Rebuilt);
    assert_eq!(recovered.state, exact_rebuild);
    assert_eq!(store.event_count(&project_id()).unwrap(), event_count);
    assert_eq!(store.current_version(&project_id()).unwrap(), state_version);
    assert_eq!(
        ProjectStateQuery::new(&mut store)
            .load(&project_id())
            .unwrap()
            .source,
        ProjectionSource::Snapshot
    );
}

fn execute(store: &mut SqliteEventStore, clock: &FixedClock, command: CommandEnvelope) {
    let result = CommandProcessor::new(store, clock)
        .execute(command, &LifecycleEngine)
        .unwrap();
    assert_eq!(
        result.status,
        project_ledger_domain::CommandResultStatus::Accepted
    );
}

fn execute_next(
    store: &mut SqliteEventStore,
    clock: &FixedClock,
    payload: Command,
    command_id: &str,
) {
    let expected = store.current_version(&project_id()).unwrap();
    execute(store, clock, command(payload, expected, command_id));
}

fn create_command() -> CommandEnvelope {
    command(
        Command::CreateProject(CreateProjectPayload { project: project() }),
        0,
        "command:0198f8c8-7417-7fb0-b88f-0fe0c0a7df80",
    )
}

fn command(payload: Command, expected: u64, command_id: &str) -> CommandEnvelope {
    prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::parse(command_id).unwrap(),
        project_ref: project_id(),
        expected_state_version: expected,
        idempotency_key: id(&format!("lifecycle:rebuild:{expected}")),
        actor: Actor {
            actor_id: id("actor:test"),
            actor_kind: ActorKind::Human,
            trust_level: TrustLevel::LocalUser,
            display_name: None,
        },
        command: payload,
        submitted_at: at(),
    })
    .unwrap()
}

fn authority() -> AuthorityDocument {
    AuthorityDocument {
        schema_version: SchemaVersion::V0_2_0,
        document_id: id("authority:canonical"),
        uri: ResourceRef::parse("file:///authority/spec.md").unwrap(),
        digest: Sha256Digest::from([1_u8; 32]),
        version: "1.0.0".to_owned(),
        status: AuthorityDocumentStatus::Current,
        authority_rank: 1,
        effective_from: at(),
        supersedes_ref: None,
    }
}

fn roadmap() -> Roadmap {
    Roadmap {
        schema_version: SchemaVersion::V0_2_0,
        roadmap_id: id("roadmap:main"),
        title: "Implementation".to_owned(),
        execution_model: RoadmapExecutionModel::Sequential,
        phase_order: vec![id("phase:implementation")],
        completion_policy: RoadmapCompletionPolicy::AllRequiredPhases,
        version: 1,
        source_document_ref: id("authority:canonical"),
    }
}

fn blocker() -> Blocker {
    let mut blocker = Blocker {
        schema_version: SchemaVersion::V0_2_0,
        blocker_id: id("blocker:dependency"),
        title: "External dependency".to_owned(),
        description: "required service unavailable".to_owned(),
        scope_kind: ScopeKind::Project,
        scope_ref: id("scope:project"),
        severity: Severity::High,
        status: BlockerStatus::Open,
        source_kind: "OBSERVED".to_owned(),
        source_ref: None,
        owner_ref: None,
        evidence_refs: Vec::new(),
        opened_at: at(),
        updated_at: at(),
        resolved_at: None,
        resolution_summary: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/blocker").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    blocker.digest = record_digest(&blocker);
    blocker
}

fn risk() -> Risk {
    let mut risk = Risk {
        schema_version: SchemaVersion::V0_2_0,
        risk_id: id("risk:locking"),
        title: "SQLite locking".to_owned(),
        description: "concurrent writers may contend".to_owned(),
        scope_kind: ScopeKind::Project,
        scope_ref: id("scope:project"),
        severity: Severity::Critical,
        likelihood_class: LikelihoodClass::Possible,
        status: RiskStatus::Open,
        owner_ref: None,
        trigger_conditions: vec!["concurrent writer".to_owned()],
        mitigation_actions: vec!["busy timeout".to_owned()],
        evidence_refs: Vec::new(),
        opened_at: at(),
        updated_at: at(),
        closed_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/risk").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    risk.digest = record_digest(&risk);
    risk
}

fn decision(decision_id: &str, version: u64, supersedes_ref: Option<StableId>) -> Decision {
    let mut decision = Decision {
        schema_version: SchemaVersion::V0_2_0,
        decision_id: id(decision_id),
        title: "Storage authority".to_owned(),
        domain: "storage".to_owned(),
        scope: "project".to_owned(),
        context: "durable state".to_owned(),
        decision: format!("storage decision version {version}"),
        rationale: "deterministic recovery".to_owned(),
        alternatives: vec!["memory".to_owned()],
        status: DecisionStatus::Proposed,
        version,
        authority_ref: None,
        evidence_refs: Vec::new(),
        supersedes_ref,
        created_at: at(),
        accepted_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/decision").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    decision.digest = record_digest(&decision);
    decision
}

fn action(
    action_id: &str,
    classification: NextActionClassification,
    authority_ref: Option<StableId>,
) -> NextAction {
    let mut action = NextAction {
        schema_version: SchemaVersion::V0_2_0,
        next_action_id: id(action_id),
        title: action_id.to_owned(),
        description: "deterministic next work".to_owned(),
        phase_ref: Some(id("phase:implementation")),
        gate_ref: None,
        classification,
        authority_ref,
        precondition_refs: Vec::new(),
        blocked_by_refs: Vec::new(),
        owner_ref: None,
        created_at: at(),
        updated_at: at(),
        completed_at: None,
        provenance_ref: ResourceRef::parse("project-ledger://provenance/action").unwrap(),
        digest: Sha256Digest::from([0_u8; 32]),
    };
    action.digest = record_digest(&action);
    action
}

fn record_digest<T: Serialize>(record: &T) -> Sha256Digest {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    sha256_digest(&value).unwrap()
}

fn project() -> Project {
    Project {
        project_id: project_id(),
        title: "Lifecycle rebuild".to_owned(),
        summary: "Canonical S4 recovery".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: at(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T16:00:00-04:00").unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
