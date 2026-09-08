use project_ledger_domain::{
    ArtifactReference, AuthorityDocument, Blocker, CheckpointFreshnessRecord, ClaimRecord, Command,
    CommandEnvelope, CommandResult, ConflictRecord, ContinuityCheckpoint, Decision, Event,
    EventEnvelope, EvidenceArtifactRef, EvidenceRecord, ExternalSubmissionRecord, GateDefinition,
    GateEvaluation, GateWaiver, GeneratedArtifactRecord, IdentityMapping, IntegrationManifest,
    LedgerRadarProjectProjection, LegacyImportCandidate, LegacyImportReview, LossReport,
    NextAction, Phase, PortableExport, Project, ProjectCatalogEntry, ProjectState,
    ProjectionEnvelope, ProjectionRequest, ProviderHealthReport, RadarFieldOwnershipPolicy,
    RadarProjectLink, RadarStatusMappingPolicy, RadarSyncEnvelope, RepositoryBinding,
    ResumeProjection, Risk, Roadmap, Session, UnknownRecord, WorkRecord,
};
use schemars::{JsonSchema, schema_for};
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SchemaEntry {
    pub schema_id: String,
    pub relative_path: String,
    pub schema: Value,
}

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("schema serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("generated schema root is not an object")]
    NonObjectRoot,
    #[error("no positive fixture is defined for {0}")]
    MissingFixture(String),
    #[error("generated fixture root is not an object for {0}")]
    NonObjectFixture(String),
    #[error("contract validation failed for {schema_id}: {detail}")]
    Validation { schema_id: String, detail: String },
}

/// Builds the complete normative schema catalog for the S1 contract surface.
///
/// # Errors
///
/// Returns [`SchemaError`] when a generated root cannot be serialized or is not
/// a JSON object.
pub fn schema_catalog() -> Result<Vec<SchemaEntry>, SchemaError> {
    let mut entries = vec![
        entry::<EventEnvelope>("event-envelope.schema.json", "event-envelope")?,
        entry::<Event>("event-payload.schema.json", "event-payload")?,
        entry::<CommandEnvelope>("command-envelope.schema.json", "command-envelope")?,
        entry::<Command>("command-payload.schema.json", "command-payload")?,
        entry::<CommandResult>("command-result.schema.json", "command-result")?,
        entry::<ExternalSubmissionRecord>(
            "external-submission-record.schema.json",
            "external-submission-record",
        )?,
        entry::<Project>("project.schema.json", "project")?,
        entry::<ProjectState>("project-state.schema.json", "project-state")?,
        entry::<AuthorityDocument>("authority-document.schema.json", "authority-document")?,
        entry::<Roadmap>("roadmap.schema.json", "roadmap")?,
        entry::<Phase>("phase.schema.json", "phase")?,
        entry::<GateDefinition>("gate-definition.schema.json", "gate-definition")?,
        entry::<GateEvaluation>("gate-evaluation.schema.json", "gate-evaluation")?,
        entry::<GateWaiver>("gate-waiver.schema.json", "gate-waiver")?,
        entry::<EvidenceRecord>("evidence-record.schema.json", "evidence-record")?,
        entry::<EvidenceArtifactRef>("evidence-artifact-ref.schema.json", "evidence-artifact-ref")?,
        entry::<ArtifactReference>("artifact-reference.schema.json", "artifact-reference")?,
        entry::<RepositoryBinding>("repository-binding.schema.json", "repository-binding")?,
        entry::<UnknownRecord>("unknown-record.schema.json", "unknown-record")?,
        entry::<ClaimRecord>("claim-record.schema.json", "claim-record")?,
        entry::<WorkRecord>("work-record.schema.json", "work-record")?,
        entry::<Decision>("decision.schema.json", "decision")?,
        entry::<Blocker>("blocker.schema.json", "blocker")?,
        entry::<Risk>("risk.schema.json", "risk")?,
        entry::<NextAction>("next-action.schema.json", "next-action")?,
        entry::<ConflictRecord>("conflict-record.schema.json", "conflict-record")?,
        entry::<Session>("session.schema.json", "session")?,
        entry::<ContinuityCheckpoint>(
            "continuity-checkpoint.schema.json",
            "continuity-checkpoint",
        )?,
        entry::<CheckpointFreshnessRecord>(
            "checkpoint-freshness-record.schema.json",
            "checkpoint-freshness-record",
        )?,
        entry::<ResumeProjection>("resume-projection.schema.json", "resume-projection")?,
        entry::<LegacyImportCandidate>(
            "legacy-import-candidate.schema.json",
            "legacy-import-candidate",
        )?,
        entry::<LegacyImportReview>("legacy-import-review.schema.json", "legacy-import-review")?,
        entry::<GeneratedArtifactRecord>(
            "generated-artifact-record.schema.json",
            "generated-artifact-record",
        )?,
        entry::<IdentityMapping>("identity-mapping.schema.json", "identity-mapping")?,
        entry::<IntegrationManifest>("integration-manifest.schema.json", "integration-manifest")?,
        entry::<ProjectionRequest>("projection-request.schema.json", "projection-request")?,
        entry::<ProjectionEnvelope>("projection-envelope.schema.json", "projection-envelope")?,
        entry::<ProviderHealthReport>("provider-health.schema.json", "provider-health")?,
        entry::<LossReport>("loss-report.schema.json", "loss-report")?,
        entry::<PortableExport>("portable-export.schema.json", "portable-export")?,
        entry::<ProjectCatalogEntry>("project-catalog-entry.schema.json", "project-catalog-entry")?,
        entry::<LedgerRadarProjectProjection>(
            "radar/ledger-radar-project-projection.schema.json",
            "radar:ledger-radar-project-projection",
        )?,
        entry::<RadarSyncEnvelope>(
            "radar/radar-sync-envelope.schema.json",
            "radar:radar-sync-envelope",
        )?,
        entry::<RadarProjectLink>(
            "radar/radar-project-link.schema.json",
            "radar:radar-project-link",
        )?,
        entry::<RadarStatusMappingPolicy>(
            "radar/radar-status-mapping-policy.schema.json",
            "radar:radar-status-mapping-policy",
        )?,
        entry::<RadarFieldOwnershipPolicy>(
            "radar/radar-field-ownership-policy.schema.json",
            "radar:radar-field-ownership-policy",
        )?,
    ];
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn entry<T>(relative_path: &str, schema_name: &str) -> Result<SchemaEntry, SchemaError>
where
    T: JsonSchema,
{
    let contract_version = if schema_name.starts_with("radar:") {
        "0.1.1"
    } else {
        "0.2.0"
    };
    let schema_id = format!("urn:project-ledger:schema:{schema_name}:{contract_version}");
    let mut schema = serde_json::to_value(schema_for!(T))?;
    let object = schema.as_object_mut().ok_or(SchemaError::NonObjectRoot)?;
    object.insert("$id".to_owned(), Value::String(schema_id.clone()));
    apply_primitive_constraints(object);
    Ok(SchemaEntry {
        schema_id,
        relative_path: relative_path.to_owned(),
        schema,
    })
}

pub(crate) fn apply_primitive_constraints(root: &mut Map<String, Value>) {
    let Some(definitions) = root.get_mut("$defs").and_then(Value::as_object_mut) else {
        return;
    };
    set_pattern(definitions, "Sha256Digest", r"^sha256:[0-9a-f]{64}$");
    set_pattern(
        definitions,
        "EventId",
        r"^event:[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
    );
    set_pattern(
        definitions,
        "CommandId",
        r"^command:[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
    );
    set_pattern(
        definitions,
        "ProjectId",
        r"^project:[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
    );
    set_pattern(
        definitions,
        "StableId",
        r"^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$",
    );
    set_pattern(definitions, "ResourceRef", r"^[A-Za-z][A-Za-z0-9+.-]*:.+$");
    if let Some(timestamp) = definitions
        .get_mut("LedgerTimestamp")
        .and_then(Value::as_object_mut)
    {
        timestamp.insert("format".to_owned(), Value::String("date-time".to_owned()));
    }
}

fn set_pattern(definitions: &mut Map<String, Value>, name: &str, pattern: &str) {
    if let Some(schema) = definitions.get_mut(name).and_then(Value::as_object_mut) {
        schema.insert("pattern".to_owned(), Value::String(pattern.to_owned()));
    }
}
