use std::{fs, path::Path};

use project_ledger_agent_skill::{
    AGENT_SKILL_PROFILE, AgentSkillContractError, verify_agent_skill_package,
};

#[test]
fn ac_061_manifest_profile_and_raw_package_validate_offline() {
    let report = verify_agent_skill_package(&workspace_root()).unwrap();

    assert_eq!(report.profile, "project-ledger-agent-skill/0.2.0");
    assert_eq!(report.profile, AGENT_SKILL_PROFILE);
    assert_eq!(report.skill_version, "0.2.0");
    assert_eq!(report.required_tool, "project-ledger");
    assert_eq!(report.minimum_tool_version, "0.2.0");
    assert_eq!(report.maximum_tool_version_exclusive, "0.3.0");
    assert_eq!(report.supported_platforms, ["linux", "macos", "windows"]);
    assert_eq!(report.schema_count, 2);
    assert_eq!(report.trigger_case_count, 7);
    assert_eq!(report.policy_case_count, 5);
    assert!(!report.state_authority);
    assert!(!report.core_dependency);
}

#[test]
fn ac_061_raw_drift_is_rejected_before_semantic_validation() {
    let temporary = tempfile::tempdir().unwrap();
    copy_tree(
        &workspace_root().join("integrations/agent-skills"),
        &temporary.path().join("integrations/agent-skills"),
    );
    let workflow = temporary
        .path()
        .join("integrations/agent-skills/project-ledger-continuity/workflow.json");
    fs::OpenOptions::new().append(true).open(&workflow).unwrap();
    fs::write(
        &workflow,
        [fs::read(&workflow).unwrap(), vec![b' ']].concat(),
    )
    .unwrap();

    assert!(matches!(
        verify_agent_skill_package(temporary.path()),
        Err(AgentSkillContractError::RawDigestMismatch { .. })
    ));
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &target_path);
        } else {
            fs::copy(source_path, target_path).unwrap();
        }
    }
}
