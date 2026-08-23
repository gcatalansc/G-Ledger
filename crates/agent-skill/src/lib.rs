//! Offline contract and behavioral conformance for the Project Ledger Agent Skill.

mod contracts;
mod workflow;

pub use contracts::{
    AGENT_SKILL_PROFILE, AgentSkillContractError, AgentSkillContractReport,
    verify_agent_skill_package,
};
pub use workflow::{
    AgentSkillConformanceReport, AgentSkillWorkflowError, EvidenceAvailability, RuntimeConditions,
    SkillAction, SkillIntent, SkillPlan, SkillPlanStatus, SubmissionState, ToolAvailability,
    inspect_instruction_content, plan_skill_actions, run_agent_skill_conformance,
    validate_workflow_policy,
};
