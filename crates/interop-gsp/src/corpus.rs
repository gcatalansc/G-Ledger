use std::collections::BTreeSet;

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_bytes, sha256_digest, to_canonical_vec,
};
use project_ledger_domain::{Event, EventEnvelope, EventId, LedgerTimestamp, ProjectId, StableId};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

const PROFILE: &str = "GSP_MINIMIZED_CORPUS_V1";
const MAX_CORPUS_ITEMS: usize = 64;
const MAX_CORPUS_TEXT_BYTES: usize = 1_048_576;
const DIRECTORIES: [&str; 6] = [
    "authority",
    "decisions",
    "gates",
    "checkpoints",
    "selected-material-events",
    "sidecars",
];

/// Explicit caps applied before any Project Ledger record enters a GSP corpus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCorpusBudget {
    pub max_items: usize,
    pub max_text_bytes: usize,
}

/// Read-only selection request. Every source record must be named explicitly.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCorpusRequest {
    pub query: String,
    pub project_ref: ProjectId,
    pub authority_refs: Vec<StableId>,
    pub decision_refs: Vec<StableId>,
    pub gate_refs: Vec<StableId>,
    pub checkpoint_refs: Vec<StableId>,
    pub material_event_refs: Vec<EventId>,
    pub observed_at: LedgerTimestamp,
    pub budget: GspCorpusBudget,
}

/// One minimized, digest-addressed file in the local corpus view.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCorpusEntry {
    pub relative_path: String,
    pub source_ref: String,
    pub source_kind: String,
    pub title: String,
    pub status: String,
    pub content: Value,
    pub content_digest: Sha256Digest,
}

/// Persistent inventory proving scope, omissions and source-state binding.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCorpusManifest {
    pub schema_version: String,
    pub artifact_kind: String,
    pub profile: String,
    pub project_ref: ProjectId,
    pub project_state_version: u64,
    pub material_state_version: u64,
    pub project_state_digest: Sha256Digest,
    pub query_digest: Sha256Digest,
    pub observed_at: LedgerTimestamp,
    pub directories: Vec<String>,
    pub files: Vec<GspCorpusEntry>,
    pub full_timeline_included: bool,
    pub raw_evidence_output_included: bool,
    pub workspace_paths_included: bool,
    pub omitted_classes: Vec<String>,
    pub corpus_digest: Sha256Digest,
}

/// Exact upstream `EvidenceBundle` plus the auditable minimized source corpus.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GspCorpus {
    pub manifest: GspCorpusManifest,
    pub evidence_bundle: Value,
}

impl GspCorpus {
    /// Verifies the canonical corpus digest and every embedded content/query digest.
    ///
    /// The corpus digest covers both the Ledger-owned manifest and the exact GSP
    /// `EvidenceBundle`, with only the digest field itself zeroed while hashing.
    ///
    /// # Errors
    ///
    /// Returns [`GspCorpusError`] when any manifest, source content, query or
    /// `EvidenceBundle` byte has changed since export.
    pub fn verify_integrity(&self) -> Result<(), GspCorpusError> {
        for file in &self.manifest.files {
            let actual = sha256_digest(&file.content)?;
            if actual != file.content_digest {
                return Err(GspCorpusError::ContentDigestMismatch {
                    source_ref: file.source_ref.clone(),
                    expected: file.content_digest,
                    actual,
                });
            }
        }
        let query = self
            .evidence_bundle
            .get("normalized_query")
            .and_then(Value::as_str)
            .filter(|query| !query.trim().is_empty())
            .ok_or(GspCorpusError::MissingNormalizedQuery)?;
        let actual_query_digest = sha256_bytes(query.as_bytes());
        if actual_query_digest != self.manifest.query_digest {
            return Err(GspCorpusError::QueryDigestMismatch {
                expected: self.manifest.query_digest,
                actual: actual_query_digest,
            });
        }
        let actual_corpus_digest = corpus_digest(self)?;
        if actual_corpus_digest != self.manifest.corpus_digest {
            return Err(GspCorpusError::CorpusDigestMismatch {
                expected: self.manifest.corpus_digest,
                actual: actual_corpus_digest,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum GspCorpusError {
    #[error("GSP corpus query must be non-empty")]
    EmptyQuery,
    #[error("GSP corpus request project does not match the projection")]
    ProjectMismatch,
    #[error("GSP corpus requires at least one explicit source selection")]
    EmptySelection,
    #[error("duplicate explicit GSP corpus selection: {reference}")]
    DuplicateSelection { reference: String },
    #[error("invalid GSP corpus budget {field}: {actual} (maximum {maximum})")]
    InvalidBudget {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("GSP corpus budget exceeded for {field}: requested {requested}, limit {limit}")]
    BudgetExceeded {
        field: &'static str,
        requested: usize,
        limit: usize,
    },
    #[error("explicit GSP corpus source is missing: {kind} {reference}")]
    MissingSelection {
        kind: &'static str,
        reference: String,
    },
    #[error("event type is not allowed in the minimized GSP corpus: {event_type}")]
    EventTypeNotAllowed { event_type: String },
    #[error("selected event belongs to a different project: {reference}")]
    EventProjectMismatch { reference: String },
    #[error(
        "GSP corpus content digest mismatch for {source_ref}: expected {expected}, actual {actual}"
    )]
    ContentDigestMismatch {
        source_ref: String,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("GSP corpus EvidenceBundle has no non-empty normalized_query")]
    MissingNormalizedQuery,
    #[error("GSP corpus query digest mismatch: expected {expected}, actual {actual}")]
    QueryDigestMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("GSP corpus digest mismatch: expected {expected}, actual {actual}")]
    CorpusDigestMismatch {
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("cannot serialize minimized GSP content: {0}")]
    Json(#[from] serde_json::Error),
}

/// Builds a deterministic, explicitly scoped corpus without mutating the ledger.
///
/// # Errors
///
/// Fails closed for absent or duplicate selections, disallowed events, unsafe
/// budgets, cross-project data or canonical serialization failure.
#[allow(clippy::too_many_lines)] // One explicit source-class mapping keeps corpus omissions auditable.
pub fn build_minimized_corpus(
    projection: &ProjectStateProjection,
    events: &[EventEnvelope],
    request: &GspCorpusRequest,
) -> Result<GspCorpus, GspCorpusError> {
    validate_request(projection, request)?;

    let mut files = Vec::new();
    for reference in &request.authority_refs {
        let document = projection.authority_document(reference).ok_or_else(|| {
            GspCorpusError::MissingSelection {
                kind: "authority",
                reference: reference.to_string(),
            }
        })?;
        files.push(entry(
            "authority",
            reference.as_str(),
            "AUTHORITY_DOCUMENT",
            &format!("Authority document {}", reference.as_str()),
            &enum_text(document.status)?,
            document,
        )?);
    }
    for reference in &request.decision_refs {
        let decision =
            projection
                .decision(reference)
                .ok_or_else(|| GspCorpusError::MissingSelection {
                    kind: "decision",
                    reference: reference.to_string(),
                })?;
        files.push(entry(
            "decisions",
            reference.as_str(),
            "DECISION",
            &decision.title,
            &enum_text(decision.status)?,
            decision,
        )?);
    }
    for reference in &request.gate_refs {
        let gate = projection
            .gate(reference)
            .ok_or_else(|| GspCorpusError::MissingSelection {
                kind: "gate",
                reference: reference.to_string(),
            })?;
        let status = projection
            .gate_status(reference)
            .map(enum_text)
            .transpose()?
            .unwrap_or_else(|| "UNKNOWN".to_owned());
        let content = json!({
            "definition": gate,
            "current_status": status,
            "current_evaluation": projection.current_gate_evaluation(reference),
        });
        files.push(entry_value(
            "gates",
            reference.as_str(),
            "GATE",
            &gate.title,
            &status,
            content,
        )?);
    }
    for reference in &request.checkpoint_refs {
        let checkpoint =
            projection
                .checkpoint(reference)
                .ok_or_else(|| GspCorpusError::MissingSelection {
                    kind: "checkpoint",
                    reference: reference.to_string(),
                })?;
        let status = projection
            .checkpoint_freshness(reference)
            .map(enum_text)
            .transpose()?
            .unwrap_or_else(|| "UNKNOWN".to_owned());
        let content = json!({"checkpoint": checkpoint, "freshness": status});
        files.push(entry_value(
            "checkpoints",
            reference.as_str(),
            "CONTINUITY_CHECKPOINT",
            &format!("Checkpoint {}", reference.as_str()),
            &status,
            content,
        )?);
    }
    for reference in &request.material_event_refs {
        let event = events
            .iter()
            .find(|event| &event.event_id == reference)
            .ok_or_else(|| GspCorpusError::MissingSelection {
                kind: "material event",
                reference: reference.to_string(),
            })?;
        if event.project_ref != request.project_ref {
            return Err(GspCorpusError::EventProjectMismatch {
                reference: reference.to_string(),
            });
        }
        let summary = minimized_event_summary(event)?;
        let event_type = enum_text(event.event_type())?;
        files.push(entry_value(
            "selected-material-events",
            reference.as_str(),
            "SELECTED_MATERIAL_EVENT",
            &format!("{event_type} at stream version {}", event.stream_version),
            "RECORDED",
            summary,
        )?);
    }

    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let text_bytes = files.iter().try_fold(0_usize, |total, file| {
        to_canonical_vec(&file.content).map(|bytes| total.saturating_add(bytes.len()))
    })?;
    if text_bytes > request.budget.max_text_bytes {
        return Err(GspCorpusError::BudgetExceeded {
            field: "max_text_bytes",
            requested: text_bytes,
            limit: request.budget.max_text_bytes,
        });
    }

    let state = projection.state();
    let query_digest = sha256_bytes(request.query.trim().as_bytes());
    let manifest = GspCorpusManifest {
        schema_version: "1.0.0".to_owned(),
        artifact_kind: "GSP_MINIMIZED_CORPUS_MANIFEST".to_owned(),
        profile: PROFILE.to_owned(),
        project_ref: request.project_ref.clone(),
        project_state_version: state.state_version,
        material_state_version: state.material_state_version,
        project_state_digest: state.state_digest,
        query_digest,
        observed_at: request.observed_at.clone(),
        directories: DIRECTORIES.iter().map(ToString::to_string).collect(),
        files,
        full_timeline_included: false,
        raw_evidence_output_included: false,
        workspace_paths_included: false,
        omitted_classes: vec![
            "FULL_EVENT_TIMELINE".to_owned(),
            "RAW_EVIDENCE_OUTPUT".to_owned(),
            "UNSELECTED_LEDGER_RECORDS".to_owned(),
            "PRIVATE_WORKSPACE_PATHS".to_owned(),
        ],
        corpus_digest: Sha256Digest::from([0; 32]),
    };
    let evidence_bundle = evidence_bundle(
        &manifest,
        request,
        !projection.state().open_conflict_refs.is_empty(),
    )?;
    let mut corpus = GspCorpus {
        manifest,
        evidence_bundle,
    };
    corpus.manifest.corpus_digest = corpus_digest(&corpus)?;
    Ok(corpus)
}

fn validate_request(
    projection: &ProjectStateProjection,
    request: &GspCorpusRequest,
) -> Result<(), GspCorpusError> {
    if request.query.trim().is_empty() {
        return Err(GspCorpusError::EmptyQuery);
    }
    if request.project_ref != projection.state().project.project_id {
        return Err(GspCorpusError::ProjectMismatch);
    }
    if request.budget.max_items == 0 || request.budget.max_items > MAX_CORPUS_ITEMS {
        return Err(GspCorpusError::InvalidBudget {
            field: "max_items",
            actual: request.budget.max_items,
            maximum: MAX_CORPUS_ITEMS,
        });
    }
    if request.budget.max_text_bytes == 0 || request.budget.max_text_bytes > MAX_CORPUS_TEXT_BYTES {
        return Err(GspCorpusError::InvalidBudget {
            field: "max_text_bytes",
            actual: request.budget.max_text_bytes,
            maximum: MAX_CORPUS_TEXT_BYTES,
        });
    }
    let selection_count = request.authority_refs.len()
        + request.decision_refs.len()
        + request.gate_refs.len()
        + request.checkpoint_refs.len()
        + request.material_event_refs.len();
    if selection_count == 0 {
        return Err(GspCorpusError::EmptySelection);
    }
    if selection_count > request.budget.max_items {
        return Err(GspCorpusError::BudgetExceeded {
            field: "max_items",
            requested: selection_count,
            limit: request.budget.max_items,
        });
    }
    let mut selected = BTreeSet::new();
    for reference in request
        .authority_refs
        .iter()
        .chain(&request.decision_refs)
        .chain(&request.gate_refs)
        .chain(&request.checkpoint_refs)
        .map(ToString::to_string)
        .chain(request.material_event_refs.iter().map(ToString::to_string))
    {
        if !selected.insert(reference.clone()) {
            return Err(GspCorpusError::DuplicateSelection { reference });
        }
    }
    Ok(())
}

fn entry<T: Serialize>(
    directory: &str,
    source_ref: &str,
    source_kind: &str,
    title: &str,
    status: &str,
    content: &T,
) -> Result<GspCorpusEntry, GspCorpusError> {
    entry_value(
        directory,
        source_ref,
        source_kind,
        title,
        status,
        serde_json::to_value(content)?,
    )
}

fn entry_value(
    directory: &str,
    source_ref: &str,
    source_kind: &str,
    title: &str,
    status: &str,
    mut content: Value,
) -> Result<GspCorpusEntry, GspCorpusError> {
    minimize_private_refs(&mut content);
    let content_digest = sha256_digest(&content)?;
    let safe_name = source_ref.replace(':', "_");
    Ok(GspCorpusEntry {
        relative_path: format!("{directory}/{safe_name}.json"),
        source_ref: source_ref.to_owned(),
        source_kind: source_kind.to_owned(),
        title: title.to_owned(),
        status: status.to_owned(),
        content,
        content_digest,
    })
}

fn minimize_private_refs(value: &mut Value) {
    match value {
        Value::String(text) if is_private_path(text) => {
            let scheme = text
                .split_once(':')
                .map_or("windows-path", |(scheme, _)| scheme);
            *value = json!({
                "source_ref_scheme": scheme,
                "source_ref_digest": sha256_bytes(text.as_bytes()),
                "redaction": "PRIVATE_PATH_MINIMIZED"
            });
        }
        Value::Array(values) => values.iter_mut().for_each(minimize_private_refs),
        Value::Object(fields) => fields.values_mut().for_each(minimize_private_refs),
        _ => {}
    }
}

fn is_private_path(value: &str) -> bool {
    value.starts_with("file:")
        || value.starts_with("workspace:")
        || value.as_bytes().get(1) == Some(&b':')
            && value
                .as_bytes()
                .get(2)
                .is_some_and(|byte| matches!(byte, b'/' | b'\\'))
}

fn minimized_event_summary(event: &EventEnvelope) -> Result<Value, GspCorpusError> {
    let event_summary = match &event.event {
        Event::AuthorityDocumentRegistered(value) => {
            json!({"document_ref": value.document.document_id, "digest": value.document.digest})
        }
        Event::DecisionProposed(value) => json!({
            "decision_ref": value.decision.decision_id,
            "title": value.decision.title,
            "decision": value.decision.decision,
            "digest": value.decision.digest,
        }),
        Event::DecisionAccepted(value) => json!({
            "decision_ref": value.decision_ref,
            "authority_ref": value.authority_ref,
        }),
        Event::DecisionSuperseded(value) => json!({
            "decision_ref": value.decision_ref,
            "replacement_ref": value.replacement_ref,
        }),
        Event::GateDefined(value) => {
            json!({"gate_ref": value.gate.gate_id, "title": value.gate.title, "digest": value.gate.digest})
        }
        Event::GateEvaluated(value) => json!({
            "gate_ref": value.evaluation.gate_ref,
            "result": value.evaluation.result,
            "digest": value.evaluation.digest,
        }),
        Event::GateVerified(value) => {
            json!({"gate_ref": value.gate_ref, "evaluation_ref": value.evaluation_ref})
        }
        Event::GateRejected(value) => {
            json!({"gate_ref": value.gate_ref, "evaluation_ref": value.evaluation_ref})
        }
        Event::WorkProgressRecorded(value) => json!({
            "work_record_ref": value.work_record.work_record_id,
            "summary": value.work_record.summary,
            "phase_ref": value.work_record.phase_ref,
            "task_ref": value.work_record.task_ref,
            "digest": value.work_record.digest,
        }),
        Event::CheckpointCreated(value) => json!({
            "checkpoint_ref": value.checkpoint.checkpoint_id,
            "captured_state_version": value.checkpoint.captured_state_version,
            "checkpoint_digest": value.checkpoint.checkpoint_digest,
        }),
        Event::ExternalEvidenceLinked(value) => json!({
            "evidence_ref": value.evidence_ref,
            "external_system": value.external_system,
            "artifact_digest": value.artifact_digest,
            "schema_ref": value.schema_ref,
        }),
        Event::ConflictOpened(value) => json!({
            "conflict_ref": value.conflict.conflict_id,
            "kind": value.conflict.kind,
            "status": value.conflict.status,
        }),
        Event::ConflictResolved(value) => json!({
            "conflict_ref": value.conflict_ref,
            "resolution_ref": value.resolution_ref,
            "reason": value.reason,
        }),
        other => {
            return Err(GspCorpusError::EventTypeNotAllowed {
                event_type: enum_text(other.event_type())?,
            });
        }
    };
    Ok(json!({
        "event_id": event.event_id,
        "event_type": event.event_type(),
        "stream_version": event.stream_version,
        "occurred_at": event.occurred_at,
        "recorded_at": event.recorded_at,
        "payload_digest": event.payload_digest,
        "event_digest": event.event_digest,
        "summary": event_summary,
    }))
}

fn evidence_bundle(
    manifest: &GspCorpusManifest,
    request: &GspCorpusRequest,
    conflicts_present: bool,
) -> Result<Value, GspCorpusError> {
    let request_id = deterministic_uuid(&format!(
        "{}:{}:{}",
        request.project_ref, manifest.project_state_digest, manifest.query_digest
    ));
    let items = manifest
        .files
        .iter()
        .map(|file| {
            let text = String::from_utf8(to_canonical_vec(&file.content)?)
                .expect("canonical JSON is valid UTF-8");
            Ok(json!({
                "evidence_id": deterministic_uuid(&format!("{}:{}", request_id, file.content_digest)),
                "doc_id": file.source_ref,
                "chunk_id": file.relative_path,
                "source_uri": format!("project-ledger://{}/{}", request.project_ref.uuid(), file.relative_path),
                "title": file.title,
                "section_path": [file.source_kind],
                "modality": "text",
                "text": text,
                "metadata": {
                    "updated_at": request.observed_at,
                    "trust_tier": "project-ledger-minimized",
                    "version": manifest.project_state_version.to_string()
                },
                "scores": {"ledger": 1.0},
                "retrieval_path": {
                    "channels": ["project-ledger-explicit-scope"],
                    "matched_subqueries": [request.query.trim()]
                },
                "flags": {"low_trust": false, "likely_injected": false}
            }))
        })
        .collect::<Result<Vec<Value>, GspCorpusError>>()?;
    Ok(json!({
        "request_id": request_id,
        "normalized_query": request.query.trim(),
        "rewritten_queries": [request.query.trim()],
        "subqueries": [request.query.trim()],
        "route_taken": ["project-ledger-explicit-scope"],
        "retrieval_confidence": 1.0,
        "evidence_sufficiency": "partial",
        "conflicts_present": conflicts_present,
        "evidence_items": items,
        "debug": {"corrective_rounds_used": 0}
    }))
}

fn deterministic_uuid(seed: &str) -> String {
    let digest = sha256_bytes(seed.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

fn corpus_digest(corpus: &GspCorpus) -> Result<Sha256Digest, GspCorpusError> {
    let mut unsigned = corpus.clone();
    unsigned.manifest.corpus_digest = Sha256Digest::from([0; 32]);
    Ok(sha256_digest(&unsigned)?)
}

fn enum_text<T: Serialize>(value: T) -> Result<String, GspCorpusError> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .expect("closed enums serialize as strings")
        .to_owned())
}
