use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    LedgerRadarProjectProjection, RadarLinkStatus, RadarProjectLink, RadarSyncEnvelope,
    ResourceRef, Severity,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RadarOwnedFields {
    pub priority: Value,
    pub tags: Value,
    pub notes: Value,
    pub ideas: Value,
    pub chats: Value,
    pub tasks: Value,
    pub artifacts: Value,
    pub presentation: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarImportSnapshot {
    pub radar_project_ref: ResourceRef,
    pub radar_title: String,
    pub source_project_ref: Option<ResourceRef>,
    pub source_state_version: Option<u64>,
    pub source_state_digest: Option<Sha256Digest>,
    pub external_continuity: Option<LedgerRadarProjectProjection>,
    pub radar_owned_fields: RadarOwnedFields,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RadarMergeDecision {
    CandidateReviewRequired,
    LinkRequired,
    IdentityConflict,
    DuplicateReplay,
    StaleIgnored,
    ApplyOverlayUpdate,
    SourceDivergence,
    NoAction,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RadarImportConflict {
    pub code: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RadarImportEvaluation {
    pub decision: RadarMergeDecision,
    pub overlay: Option<LedgerRadarProjectProjection>,
    pub preserved_radar_fields: RadarOwnedFields,
    pub candidate_ledger_refs: Vec<ResourceRef>,
    pub conflicts: Vec<RadarImportConflict>,
    pub provenance_chain: Vec<ResourceRef>,
    pub lifecycle_authority: String,
}

#[derive(Debug, Error)]
pub enum RadarImportError {
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("Radar envelope violates profile 0.1.1")]
    InvalidEnvelope,
    #[error("Radar envelope digest does not cover its complete contents")]
    EnvelopeDigestMismatch,
    #[error("Radar projection digest differs for {project_ref}")]
    ProjectionDigestMismatch { project_ref: String },
    #[error("Radar project link digest differs")]
    LinkDigestMismatch,
}

/// Evaluates a Project Radar import without persistence or product side effects.
///
/// # Errors
///
/// Rejects altered envelopes, projections, or links before returning a preview.
pub fn evaluate_import(
    snapshot: &RadarImportSnapshot,
    envelope: &RadarSyncEnvelope,
    link: Option<&RadarProjectLink>,
) -> Result<RadarImportEvaluation, RadarImportError> {
    validate_sync_envelope(envelope)?;
    let mut evaluation = base_evaluation(snapshot);
    let Some(link) = link else {
        evaluation.candidate_ledger_refs = name_candidates(snapshot, envelope);
        evaluation.decision = if evaluation.candidate_ledger_refs.is_empty() {
            RadarMergeDecision::LinkRequired
        } else {
            RadarMergeDecision::CandidateReviewRequired
        };
        return Ok(evaluation);
    };
    if digest_without_field(link)? != link.digest {
        return Err(RadarImportError::LinkDigestMismatch);
    }
    Ok(evaluate_linked(snapshot, envelope, link, evaluation))
}

/// Validates profile semantics and all self-digests in a Radar sync envelope.
///
/// # Errors
///
/// Returns [`RadarImportError`] for invalid semantics or any altered digest.
pub fn validate_sync_envelope(envelope: &RadarSyncEnvelope) -> Result<(), RadarImportError> {
    envelope
        .validate()
        .map_err(|_| RadarImportError::InvalidEnvelope)?;
    if digest_without_field(envelope)? != envelope.digest {
        return Err(RadarImportError::EnvelopeDigestMismatch);
    }
    for project in &envelope.projects {
        if digest_without_field(project)? != project.digest {
            return Err(RadarImportError::ProjectionDigestMismatch {
                project_ref: project.source.project_ref.to_string(),
            });
        }
    }
    Ok(())
}

fn evaluate_linked(
    snapshot: &RadarImportSnapshot,
    envelope: &RadarSyncEnvelope,
    link: &RadarProjectLink,
    mut evaluation: RadarImportEvaluation,
) -> RadarImportEvaluation {
    if link.status != RadarLinkStatus::Confirmed {
        evaluation.decision = if link.status == RadarLinkStatus::Conflicted {
            evaluation.conflicts.push(identity_conflict());
            RadarMergeDecision::IdentityConflict
        } else {
            RadarMergeDecision::LinkRequired
        };
        return evaluation;
    }
    if link.radar_project_ref != snapshot.radar_project_ref {
        evaluation.decision = RadarMergeDecision::IdentityConflict;
        evaluation.conflicts.push(identity_conflict());
        return evaluation;
    }
    if snapshot
        .source_project_ref
        .as_ref()
        .is_some_and(|reference| reference != &link.ledger_project_ref)
    {
        evaluation.decision = RadarMergeDecision::IdentityConflict;
        evaluation.conflicts.push(identity_conflict());
        return evaluation;
    }

    let Some(project) = envelope.projects.iter().find(|project| {
        project.source.project_ref == link.ledger_project_ref
            && project.identity.ledger_project_ref == link.ledger_project_ref
    }) else {
        evaluation.decision = RadarMergeDecision::NoAction;
        return evaluation;
    };
    evaluation.provenance_chain = vec![project.provenance_ref.clone()];
    evaluation.decision = merge_decision(snapshot, project);
    match evaluation.decision {
        RadarMergeDecision::ApplyOverlayUpdate => evaluation.overlay = Some(project.clone()),
        RadarMergeDecision::SourceDivergence => {
            evaluation.conflicts.push(RadarImportConflict {
                code: "SOURCE_DIVERGENCE".to_owned(),
                severity: Severity::Critical,
                message: "same Ledger project/version arrived with a different source digest"
                    .to_owned(),
            });
        }
        _ => {}
    }
    evaluation
}

fn base_evaluation(snapshot: &RadarImportSnapshot) -> RadarImportEvaluation {
    RadarImportEvaluation {
        decision: RadarMergeDecision::LinkRequired,
        overlay: None,
        preserved_radar_fields: snapshot.radar_owned_fields.clone(),
        candidate_ledger_refs: Vec::new(),
        conflicts: Vec::new(),
        provenance_chain: Vec::new(),
        lifecycle_authority: "project-ledger".to_owned(),
    }
}

fn name_candidates(
    snapshot: &RadarImportSnapshot,
    envelope: &RadarSyncEnvelope,
) -> Vec<ResourceRef> {
    let title = snapshot.radar_title.trim().to_lowercase();
    let mut result = envelope
        .projects
        .iter()
        .filter(|project| {
            project.project.canonical_title.trim().to_lowercase() == title
                || project
                    .identity
                    .source_aliases
                    .iter()
                    .any(|alias| alias.trim().to_lowercase() == title)
        })
        .map(|project| project.source.project_ref.clone())
        .collect::<Vec<_>>();
    result.sort();
    result.dedup();
    result
}

fn merge_decision(
    snapshot: &RadarImportSnapshot,
    project: &LedgerRadarProjectProjection,
) -> RadarMergeDecision {
    let Some(current_version) = snapshot.source_state_version else {
        return RadarMergeDecision::ApplyOverlayUpdate;
    };
    if project.source.state_version < current_version {
        RadarMergeDecision::StaleIgnored
    } else if project.source.state_version > current_version {
        RadarMergeDecision::ApplyOverlayUpdate
    } else if snapshot.source_state_digest == Some(project.source.state_digest) {
        RadarMergeDecision::DuplicateReplay
    } else {
        RadarMergeDecision::SourceDivergence
    }
}

fn identity_conflict() -> RadarImportConflict {
    RadarImportConflict {
        code: "IDENTITY_CONFLICT".to_owned(),
        severity: Severity::Critical,
        message: "confirmed RadarProjectLink does not match the consumer-owned project identity"
            .to_owned(),
    }
}

fn digest_without_field<T: Serialize>(value: &T) -> Result<Sha256Digest, CanonicalJsonError> {
    let mut json = serde_json::to_value(value)?;
    json.as_object_mut()
        .expect("Radar records are objects")
        .remove("digest");
    sha256_digest(&json)
}
