use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::verify_agent_skill_package;

const PACKAGE_ROOT: &str = "integrations/agent-skills/project-ledger-continuity";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillIntent {
    StartOrResume,
    MaterialProgress,
    EvidenceResult,
    EvaluateGate,
    Handoff,
    Unrelated,
    OrdinaryDiary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillAction {
    ProbeTool,
    Resume,
    RecordProgress,
    RecordEvidence,
    RequestGateEvaluation,
    CreateCheckpoint,
    ReconcileSubmission,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillPlanStatus {
    NotTriggered,
    Ready,
    DegradedMissingTool,
    ReplanRequired,
    ReconcileRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConditions {
    pub tool: ToolAvailability,
    pub evidence: EvidenceAvailability,
    pub submission: SubmissionState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolAvailability {
    Available,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionState {
    Clean,
    VersionConflict,
    UnknownOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillPlan {
    pub status: SkillPlanStatus,
    pub actions: Vec<SkillAction>,
    pub completion_claim_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillConformanceReport {
    pub positive_trigger_count: usize,
    pub negative_trigger_count: usize,
    pub policy_case_count: usize,
    pub failed_case_count: usize,
}

#[derive(Debug, Error)]
pub enum AgentSkillWorkflowError {
    #[error("cannot read Agent Skill workflow artifact {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Agent Skill workflow JSON is invalid at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Agent Skill executable content contains concrete current state at {path}")]
    CurrentStateEmbedded { path: String },
    #[error("Agent Skill workflow can bypass engine authority through {action}")]
    AuthorityBypass { action: String },
    #[error("Agent Skill workflow policy differs at {field}")]
    PolicyMismatch { field: &'static str },
    #[error("Agent Skill conformance case {case_id} produced an unexpected plan")]
    CaseMismatch { case_id: String },
    #[error("Agent Skill package validation failed: {message}")]
    Package { message: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowContract {
    schema_version: String,
    workflow_id: String,
    profile: String,
    ordered_actions: Vec<SkillAction>,
    mutating_actions: Vec<SkillAction>,
    forbidden_actions: Vec<String>,
    failure_policies: FailurePolicies,
    completion_authority: String,
    storage_access: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FailurePolicies {
    missing_tool: SkillPlanStatus,
    unknown_outcome: SkillPlanStatus,
    version_conflict: SkillPlanStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the closed external conformance fixture vector.
struct ConformanceCase {
    case_id: String,
    intent: SkillIntent,
    tool_available: bool,
    evidence_available: bool,
    version_conflict: bool,
    unknown_outcome: bool,
    expected_status: SkillPlanStatus,
    expected_actions: Vec<SkillAction>,
}

pub fn plan_skill_actions(intent: SkillIntent, conditions: RuntimeConditions) -> SkillPlan {
    if matches!(intent, SkillIntent::Unrelated | SkillIntent::OrdinaryDiary) {
        return plan(SkillPlanStatus::NotTriggered, Vec::new());
    }
    if conditions.tool == ToolAvailability::Missing {
        return plan(SkillPlanStatus::DegradedMissingTool, Vec::new());
    }
    if conditions.submission == SubmissionState::UnknownOutcome {
        return plan(
            SkillPlanStatus::ReconcileRequired,
            vec![SkillAction::ReconcileSubmission],
        );
    }
    if conditions.submission == SubmissionState::VersionConflict {
        return plan(SkillPlanStatus::ReplanRequired, vec![SkillAction::Resume]);
    }

    let actions = match intent {
        SkillIntent::StartOrResume => vec![SkillAction::ProbeTool, SkillAction::Resume],
        SkillIntent::MaterialProgress => vec![SkillAction::RecordProgress],
        SkillIntent::EvidenceResult if conditions.evidence == EvidenceAvailability::Available => {
            vec![
                SkillAction::RecordEvidence,
                SkillAction::RequestGateEvaluation,
            ]
        }
        SkillIntent::EvidenceResult => vec![SkillAction::RecordEvidence],
        SkillIntent::EvaluateGate => vec![SkillAction::RequestGateEvaluation],
        SkillIntent::Handoff => vec![SkillAction::CreateCheckpoint, SkillAction::Resume],
        SkillIntent::Unrelated | SkillIntent::OrdinaryDiary => Vec::new(),
    };
    plan(SkillPlanStatus::Ready, actions)
}

/// Rejects concrete project/checkpoint UUIDs while allowing symbolic placeholders.
///
/// # Errors
///
/// Returns [`AgentSkillWorkflowError::CurrentStateEmbedded`] when executable
/// instruction content contains a concrete Ledger state identity.
pub fn inspect_instruction_content(
    path: &str,
    content: &str,
) -> Result<(), AgentSkillWorkflowError> {
    for prefix in ["project:", "checkpoint:"] {
        for suffix in content.split(prefix).skip(1) {
            let candidate = suffix.chars().take(36).collect::<String>();
            if uuid::Uuid::parse_str(&candidate).is_ok() {
                return Err(AgentSkillWorkflowError::CurrentStateEmbedded {
                    path: path.to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// Validates that the workflow action surface cannot bypass engine authority.
///
/// # Errors
///
/// Returns [`AgentSkillWorkflowError`] for malformed policy, forbidden actions,
/// direct storage access or completion authority drift.
pub fn validate_workflow_policy(value: &Value) -> Result<(), AgentSkillWorkflowError> {
    for field in ["ordered_actions", "mutating_actions"] {
        let actions = value
            .get(field)
            .and_then(Value::as_array)
            .ok_or(AgentSkillWorkflowError::PolicyMismatch { field })?;
        for action in actions {
            let action = action
                .as_str()
                .ok_or(AgentSkillWorkflowError::PolicyMismatch { field })?;
            if [
                "BLIND_RETRY",
                "DECLARE_GATE_VERIFIED",
                "DECLARE_PHASE_COMPLETE",
                "DIRECT_DATABASE_WRITE",
                "EDIT_GENERATED_MARKDOWN",
            ]
            .contains(&action)
            {
                return Err(AgentSkillWorkflowError::AuthorityBypass {
                    action: action.to_owned(),
                });
            }
        }
    }

    let workflow: WorkflowContract =
        serde_json::from_value(value.clone()).map_err(|source| AgentSkillWorkflowError::Json {
            path: PathBuf::from("workflow.json"),
            source,
        })?;
    let expected_order = [
        SkillAction::ProbeTool,
        SkillAction::Resume,
        SkillAction::RecordProgress,
        SkillAction::RecordEvidence,
        SkillAction::RequestGateEvaluation,
        SkillAction::CreateCheckpoint,
        SkillAction::ReconcileSubmission,
    ];
    let expected_mutating = [
        SkillAction::RecordProgress,
        SkillAction::RecordEvidence,
        SkillAction::RequestGateEvaluation,
        SkillAction::CreateCheckpoint,
    ];
    let expected_forbidden = [
        "BLIND_RETRY",
        "DECLARE_GATE_VERIFIED",
        "DECLARE_PHASE_COMPLETE",
        "DIRECT_DATABASE_WRITE",
        "EDIT_GENERATED_MARKDOWN",
    ];
    if workflow.schema_version != "1.0.0"
        || workflow.workflow_id != "project-ledger-continuity-v1"
        || workflow.profile != crate::AGENT_SKILL_PROFILE
        || workflow.ordered_actions != expected_order
        || workflow.mutating_actions != expected_mutating
        || workflow
            .forbidden_actions
            .iter()
            .map(String::as_str)
            .ne(expected_forbidden)
        || workflow.failure_policies.missing_tool != SkillPlanStatus::DegradedMissingTool
        || workflow.failure_policies.unknown_outcome != SkillPlanStatus::ReconcileRequired
        || workflow.failure_policies.version_conflict != SkillPlanStatus::ReplanRequired
        || workflow.completion_authority != "PROJECT_LEDGER_ENGINE_ONLY"
        || workflow.storage_access != "PROJECT_LEDGER_CLI_ONLY"
        || workflow.digest.is_empty()
    {
        return Err(AgentSkillWorkflowError::PolicyMismatch { field: "workflow" });
    }
    let unique = workflow
        .ordered_actions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if unique.len() != workflow.ordered_actions.len() {
        return Err(AgentSkillWorkflowError::PolicyMismatch {
            field: "ordered_actions",
        });
    }
    Ok(())
}

/// Executes every locked trigger and policy case against the pure planner.
///
/// # Errors
///
/// Returns [`AgentSkillWorkflowError`] when the package is invalid, a workflow
/// policy drifts, a fixture cannot be decoded or any expected plan differs.
pub fn run_agent_skill_conformance(
    repository_root: &Path,
) -> Result<AgentSkillConformanceReport, AgentSkillWorkflowError> {
    verify_agent_skill_package(repository_root).map_err(|error| {
        AgentSkillWorkflowError::Package {
            message: error.to_string(),
        }
    })?;
    let package_root = repository_root.join(PACKAGE_ROOT);
    let workflow_path = package_root.join("workflow.json");
    let workflow: Value = read_json(&workflow_path)?;
    validate_workflow_policy(&workflow)?;

    let trigger_cases: Vec<ConformanceCase> =
        read_json(&package_root.join("tests/trigger-cases.json"))?;
    let policy_cases: Vec<ConformanceCase> =
        read_json(&package_root.join("tests/policy-cases.json"))?;
    let mut failed_case_count = 0;
    for case in trigger_cases.iter().chain(&policy_cases) {
        let actual = plan_skill_actions(
            case.intent,
            RuntimeConditions {
                tool: if case.tool_available {
                    ToolAvailability::Available
                } else {
                    ToolAvailability::Missing
                },
                evidence: if case.evidence_available {
                    EvidenceAvailability::Available
                } else {
                    EvidenceAvailability::Unavailable
                },
                submission: submission_state(case),
            },
        );
        if actual.status != case.expected_status
            || actual.actions != case.expected_actions
            || actual.completion_claim_allowed
        {
            failed_case_count += 1;
        }
    }
    if failed_case_count != 0 {
        let case_id = trigger_cases
            .iter()
            .chain(&policy_cases)
            .find(|case| {
                let actual = plan_skill_actions(
                    case.intent,
                    RuntimeConditions {
                        tool: if case.tool_available {
                            ToolAvailability::Available
                        } else {
                            ToolAvailability::Missing
                        },
                        evidence: if case.evidence_available {
                            EvidenceAvailability::Available
                        } else {
                            EvidenceAvailability::Unavailable
                        },
                        submission: submission_state(case),
                    },
                );
                actual.status != case.expected_status
                    || actual.actions != case.expected_actions
                    || actual.completion_claim_allowed
            })
            .map_or_else(|| "unknown".to_owned(), |case| case.case_id.clone());
        return Err(AgentSkillWorkflowError::CaseMismatch { case_id });
    }
    let positive_trigger_count = trigger_cases
        .iter()
        .filter(|case| case.expected_status != SkillPlanStatus::NotTriggered)
        .count();
    Ok(AgentSkillConformanceReport {
        positive_trigger_count,
        negative_trigger_count: trigger_cases.len() - positive_trigger_count,
        policy_case_count: policy_cases.len(),
        failed_case_count,
    })
}

fn plan(status: SkillPlanStatus, actions: Vec<SkillAction>) -> SkillPlan {
    SkillPlan {
        status,
        actions,
        completion_claim_allowed: false,
    }
}

const fn submission_state(case: &ConformanceCase) -> SubmissionState {
    if case.unknown_outcome {
        SubmissionState::UnknownOutcome
    } else if case.version_conflict {
        SubmissionState::VersionConflict
    } else {
        SubmissionState::Clean
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AgentSkillWorkflowError> {
    let bytes = fs::read(path).map_err(|source| AgentSkillWorkflowError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| AgentSkillWorkflowError::Json {
        path: path.to_path_buf(),
        source,
    })
}
