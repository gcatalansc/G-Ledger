use project_ledger_canonical_json::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    EngineIdentity, GateStatus, LedgerTimestamp, ProjectId, RepositorySnapshot, ResourceRef,
    SchemaVersion, StableId, common::closed_enum,
};

closed_enum! {
    pub enum EvidenceKind {
        TestResult,
        BuildResult,
        LintResult,
        FormatResult,
        TypecheckResult,
        SecurityScanResult,
        PrivacyScanResult,
        BenchmarkResult,
        SmokeResult,
        E2eResult,
        ManualReview,
        ArtifactDigest,
        FileExistence,
        SchemaValidation,
        MigrationResult,
        ExternalVerifiedEvidence,
        CustomDeclarative
    }
}

closed_enum! { pub enum EvidenceStatus { Asserted, Observed, VerifiedSource, Rejected, Stale, Invalidated, Unavailable } }
closed_enum! { pub enum EvidenceSourceType { CommandExecution, ImportedArtifact, ExternalSystem, ManualReview } }
closed_enum! { pub enum GateCriticality { Required, Optional } }
closed_enum! { pub enum GateCompletionOperator { All, Any } }
closed_enum! { pub enum GateWaiverPolicy { Forbidden, HumanApproval, AuthorityApproval } }
closed_enum! { pub enum GateEvaluationResult { Pass, Fail, Incomplete, Stale, Conflicted, Error } }
closed_enum! { pub enum RuleResult { Pass, Fail, Missing, Stale, Conflicted, Error } }
closed_enum! { pub enum RevalidationTrigger { EvidenceInvalidated, RepositoryDependencyChanged, AuthorityDocumentChanged } }

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ScalarValue {
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    String(String),
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(
    tag = "operator",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum PredicateAst {
    All { predicates: Vec<Self> },
    Any { predicates: Vec<Self> },
    Not { predicate: Box<Self> },
    Equals { path: String, value: ScalarValue },
    NotEquals { path: String, value: ScalarValue },
    LessThan { path: String, value: ScalarValue },
    LessOrEqual { path: String, value: ScalarValue },
    GreaterThan { path: String, value: ScalarValue },
    GreaterOrEqual { path: String, value: ScalarValue },
    Exists { path: String },
    Missing { path: String },
    Contains { path: String, value: ScalarValue },
    DigestEquals { path: String, value: Sha256Digest },
    StatusIn { path: String, values: Vec<String> },
    CountEquals { path: String, value: u64 },
    CountAtLeast { path: String, value: u64 },
    SameSnapshotAs { evidence_ref: StableId },
    FreshnessIn { path: String, values: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredEvidenceRule {
    pub rule_id: StableId,
    pub kind: EvidenceKind,
    pub predicate: PredicateAst,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryDependencies {
    pub paths: Vec<String>,
    pub external_refs: Vec<ResourceRef>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshnessPolicy {
    pub evidence_must_match_same_repository_snapshot: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateDefinition {
    pub schema_version: SchemaVersion,
    pub gate_id: StableId,
    pub definition_version: u64,
    pub title: String,
    pub phase_ref: StableId,
    pub criticality: GateCriticality,
    pub required_evidence_rules: Vec<RequiredEvidenceRule>,
    pub repository_dependencies: RepositoryDependencies,
    pub completion_rule: GateCompletionOperator,
    pub waiver_policy: GateWaiverPolicy,
    pub freshness_policy: FreshnessPolicy,
    pub revalidate_on: Vec<RevalidationTrigger>,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedEvidence {
    pub evidence_ref: StableId,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateRuleResult {
    pub rule_id: StableId,
    pub result: RuleResult,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateEvaluation {
    pub schema_version: SchemaVersion,
    pub gate_evaluation_id: StableId,
    pub gate_ref: StableId,
    pub gate_definition_version: u64,
    pub gate_definition_digest: Sha256Digest,
    pub project_state_version: u64,
    pub repository_binding: Option<RepositorySnapshot>,
    pub selected_evidence_refs: Vec<StableId>,
    pub rejected_evidence: Vec<RejectedEvidence>,
    pub rule_results: Vec<GateRuleResult>,
    pub result: GateEvaluationResult,
    pub proposed_gate_status: GateStatus,
    pub explanation_codes: Vec<String>,
    pub evaluated_by: EngineIdentity,
    pub evaluated_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateWaiver {
    pub schema_version: SchemaVersion,
    pub waiver_id: StableId,
    pub gate_ref: StableId,
    pub gate_definition_version: u64,
    pub gate_definition_digest: Sha256Digest,
    pub actor_ref: StableId,
    pub reason: String,
    pub scope: String,
    pub expires_at: Option<LedgerTimestamp>,
    pub review_condition: String,
    pub authority_ref: StableId,
    pub created_at: LedgerTimestamp,
    pub digest: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSource {
    pub source_type: EvidenceSourceType,
    pub tool: String,
    pub command_display: Option<String>,
    pub working_directory_ref: Option<ResourceRef>,
    pub runner_version: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMetrics {
    pub passed: Option<u64>,
    pub failed: Option<u64>,
    pub skipped: Option<u64>,
    pub errors: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceResult {
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub metrics: EvidenceMetrics,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArtifactRef {
    pub uri: ResourceRef,
    pub digest: Sha256Digest,
    pub persisted: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePrivacy {
    pub raw_output_stored: bool,
    pub redaction_applied: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub schema_version: SchemaVersion,
    pub evidence_id: StableId,
    pub project_ref: ProjectId,
    pub kind: EvidenceKind,
    pub status: EvidenceStatus,
    pub source: EvidenceSource,
    pub result: EvidenceResult,
    pub repository_binding: Option<RepositorySnapshot>,
    pub artifacts: Vec<EvidenceArtifactRef>,
    pub privacy: EvidencePrivacy,
    pub observed_at: LedgerTimestamp,
    pub payload_digest: Sha256Digest,
    pub provenance_ref: ResourceRef,
}
