use std::{fs, path::Path};

use project_ledger_agent_skill::{
    AgentSkillWorkflowError, EvidenceAvailability, RuntimeConditions, SkillAction, SkillIntent,
    SkillPlanStatus, SubmissionState, ToolAvailability, inspect_instruction_content,
    plan_skill_actions, run_agent_skill_conformance, validate_workflow_policy,
};

#[test]
fn ac_061_trigger_cases_execute_as_positive_and_negative_behavior() {
    let report = run_agent_skill_conformance(&workspace_root()).unwrap();

    assert_eq!(report.positive_trigger_count, 5);
    assert_eq!(report.negative_trigger_count, 2);
    assert_eq!(report.policy_case_count, 5);
    assert_eq!(report.failed_case_count, 0);
}

#[test]
fn ac_062_concrete_current_state_is_rejected_but_placeholders_are_allowed() {
    inspect_instruction_content(
        "example.json",
        r#"{"project_ref":"${PROJECT_REF}","state_version":"${STATE_VERSION}"}"#,
    )
    .unwrap();

    assert!(matches!(
        inspect_instruction_content(
            "SKILL.md",
            "Continue project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d"
        ),
        Err(AgentSkillWorkflowError::CurrentStateEmbedded { .. })
    ));
}

#[test]
fn ac_063_missing_tool_and_uncertain_writes_degrade_without_blind_mutation() {
    let missing = plan_skill_actions(
        SkillIntent::StartOrResume,
        RuntimeConditions {
            tool: ToolAvailability::Missing,
            evidence: EvidenceAvailability::Unavailable,
            submission: SubmissionState::Clean,
        },
    );
    assert_eq!(missing.status, SkillPlanStatus::DegradedMissingTool);
    assert!(missing.actions.is_empty());

    let conflict = plan_skill_actions(
        SkillIntent::MaterialProgress,
        RuntimeConditions {
            tool: ToolAvailability::Available,
            evidence: EvidenceAvailability::Unavailable,
            submission: SubmissionState::VersionConflict,
        },
    );
    assert_eq!(conflict.status, SkillPlanStatus::ReplanRequired);
    assert_eq!(conflict.actions, [SkillAction::Resume]);

    let uncertain = plan_skill_actions(
        SkillIntent::MaterialProgress,
        RuntimeConditions {
            tool: ToolAvailability::Available,
            evidence: EvidenceAvailability::Unavailable,
            submission: SubmissionState::UnknownOutcome,
        },
    );
    assert_eq!(uncertain.status, SkillPlanStatus::ReconcileRequired);
    assert_eq!(uncertain.actions, [SkillAction::ReconcileSubmission]);
}

#[test]
fn ac_064_workflow_rejects_authority_bypass_and_completion_stays_engine_owned() {
    let gate = plan_skill_actions(
        SkillIntent::EvaluateGate,
        RuntimeConditions {
            tool: ToolAvailability::Available,
            evidence: EvidenceAvailability::Unavailable,
            submission: SubmissionState::Clean,
        },
    );
    assert_eq!(gate.status, SkillPlanStatus::Ready);
    assert_eq!(gate.actions, [SkillAction::RequestGateEvaluation]);
    assert!(!gate.completion_claim_allowed);

    let workflow_path =
        workspace_root().join("integrations/agent-skills/project-ledger-continuity/workflow.json");
    let mut workflow: serde_json::Value =
        serde_json::from_slice(&fs::read(workflow_path).unwrap()).unwrap();
    workflow["ordered_actions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::Value::String(
            "DECLARE_PHASE_COMPLETE".to_owned(),
        ));
    assert!(matches!(
        validate_workflow_policy(&workflow),
        Err(AgentSkillWorkflowError::AuthorityBypass { .. })
    ));
}

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
