use super::*;
use project_ledger_domain::EpisodeUpdate;
use serde_json::json;

fn fixture() -> (tempfile::TempDir, SqliteRecordSinkHost) {
    let root = tempfile::tempdir().unwrap();
    let project_ref = ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap();
    let clock = FixedClock::new(LedgerTimestamp::parse("2026-09-07T12:00:00Z").unwrap());
    let host = SqliteRecordSinkHost {
        store: RefCell::new(SqliteEventStore::open(&root.path().join("ledger.db")).unwrap()),
        project_ref: project_ref.clone(),
        clock,
    };
    let project = Project {
        project_id: project_ref.clone(),
        title: "Historical confirmation".to_owned(),
        summary: "Owned test fixture".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: host.clock.now(),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: vec![],
        paused_reason: None,
        archived_at: None,
    };
    let episode: EpisodeUpdate = serde_json::from_value(json!({
        "episode_ref":"episode:confirmation", "objective":"Atomic historical cut",
        "summary":"Episode and work progress share one command", "entries":[]
    }))
    .unwrap();
    for (uuid, version, schema, command) in [
        (
            "0198f8c8-7417-7fb0-b88f-0fe0c0a7df50",
            0,
            SchemaVersion::V0_2_0,
            Command::CreateProject(CreateProjectPayload { project }),
        ),
        (
            "0198f8c8-7417-7fb0-b88f-0fe0c0a7df51",
            1,
            SchemaVersion::V0_3_0,
            Command::RecordEpisodeUpdate(episode),
        ),
    ] {
        let draft = CommandDraft {
            schema_version: schema,
            command_id: CommandId::parse(&format!("command:{uuid}")).unwrap(),
            project_ref: project_ref.clone(),
            expected_state_version: version,
            idempotency_key: StableId::parse(&format!("test:{uuid}")).unwrap(),
            actor: Actor {
                actor_id: StableId::parse("human:fixture").unwrap(),
                actor_kind: ActorKind::Human,
                trust_level: TrustLevel::LocalUser,
                display_name: None,
            },
            command,
            submitted_at: host.clock.now(),
        };
        assert_eq!(
            host.execute(prepare_command(draft).unwrap())
                .unwrap()
                .status,
            CommandResultStatus::Accepted
        );
    }
    assert_eq!(
        host.store.borrow().current_version(&project_ref).unwrap(),
        3
    );
    (root, host)
}

#[test]
fn confirms_only_existing_atomic_cuts_of_the_selected_project() {
    let (_root, host) = fixture();
    let before = host.store.borrow().load_stream(&host.project_ref).unwrap();
    for version in [1, 3] {
        assert_eq!(
            host.confirmed_projection_ref(&host.project_ref, version)
                .unwrap()
                .as_str(),
            format!("continuity:ledger-{version}")
        );
    }
    for version in [0, 2, 4, u64::MAX] {
        assert!(
            host.confirmed_projection_ref(&host.project_ref, version)
                .is_err(),
            "invalid cut {version}"
        );
    }
    let other = ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5e").unwrap();
    assert!(host.confirmed_projection_ref(&other, 1).is_err());
    assert_eq!(
        serde_json::to_value(host.store.borrow().load_stream(&host.project_ref).unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}

#[test]
fn historical_confirmation_rejects_a_corrupt_current_stream_head() {
    let (root, host) = fixture();
    let connection = rusqlite::Connection::open(root.path().join("ledger.db")).unwrap();
    connection
        .execute(
            "UPDATE event_streams SET head_event_digest=?1",
            [format!("sha256:{}", "0".repeat(64))],
        )
        .unwrap();
    assert!(host.confirmed_projection_ref(&host.project_ref, 1).is_err());
    assert!(host.confirmed_projection_ref(&host.project_ref, 3).is_err());
}
