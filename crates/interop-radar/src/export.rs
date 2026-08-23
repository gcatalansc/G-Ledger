use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    AuthorityDocumentStatus, CheckpointFreshnessStatus, FreshnessStatus,
    LedgerRadarProjectProjection, LedgerTimestamp, NextActionClassification, Phase, PhaseKind,
    ProjectLifecycleStatus, RadarAttentionSignal, RadarCanonicalization, RadarConflictCounts,
    RadarContinuity, RadarCounts, RadarFreshness, RadarGateCounts, RadarIdentity, RadarPercentage,
    RadarPhaseSummary, RadarPrivacy, RadarProgress, RadarProgressMethod, RadarProgressStatus,
    RadarProjectSummary, RadarSource, RadarSyncContract, RadarSyncEnvelope, RadarSyncSemantics,
    RepositoryBindingStatus, ResourceRef, SchemaVersion, Severity, StableId, SuggestedRadarStatus,
};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectionError, validate_state_integrity,
};
use serde::Serialize;
use thiserror::Error;

use crate::{RADAR_PROFILE, RADAR_SOURCE_SPEC_SHA256};

const ENVELOPE_SCHEMA_ID: &str = "urn:project-ledger:schema:radar:radar-sync-envelope:0.1.1";
const ENVELOPE_SCHEMA_SHA256: &str =
    "2df971abbd18221b66f0e0ff3c0b328d0488255ceea107f12064510b30154991";

#[derive(Clone, Debug)]
pub struct RadarExportContext {
    pub phase_catalog: BTreeMap<StableId, Phase>,
    pub source_aliases: Vec<String>,
    pub observed_at: LedgerTimestamp,
    pub provenance_ref: ResourceRef,
}

#[derive(Clone, Debug)]
pub struct RadarEnvelopeRequest {
    pub sync_envelope_id: StableId,
    pub source_instance_ref: ResourceRef,
    pub exported_at: LedgerTimestamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RadarStatusSuggestion {
    pub status: SuggestedRadarStatus,
    pub rule_code: &'static str,
}

#[derive(Debug, Error)]
pub enum RadarExportError {
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("Radar export requires phase metadata for {phase_ref}")]
    PhaseMetadataMissing { phase_ref: String },
    #[error("Radar phase metadata status differs from the projection for {phase_ref}")]
    PhaseMetadataMismatch { phase_ref: String },
    #[error("no normative Radar status mapping for {lifecycle:?} and {phase_kind:?}")]
    UnsupportedStatusMapping {
        lifecycle: ProjectLifecycleStatus,
        phase_kind: Option<PhaseKind>,
    },
    #[error("cannot derive a Project Radar resource reference")]
    InvalidResourceReference,
    #[error("duplicate project {project_ref} in one Radar envelope")]
    DuplicateProject { project_ref: String },
    #[error("Radar projection digest differs for {project_ref}")]
    ProjectionDigestMismatch { project_ref: String },
    #[error("Project Radar profile invariant failed: {0}")]
    ProfileInvariant(&'static str),
}

/// Applies only the normative Ledger-to-Radar lifecycle mapping table.
///
/// # Errors
///
/// Rejects an unlisted active phase kind instead of inventing a Radar-owned status.
pub fn suggested_radar_status(
    lifecycle: ProjectLifecycleStatus,
    phase_kind: Option<PhaseKind>,
    spec_ready_without_phase: bool,
) -> Result<RadarStatusSuggestion, RadarExportError> {
    let mapped = match lifecycle {
        ProjectLifecycleStatus::Initializing if spec_ready_without_phase => RadarStatusSuggestion {
            status: SuggestedRadarStatus::SpecReady,
            rule_code: "INITIALIZING_SPEC_READY_NO_PHASE",
        },
        ProjectLifecycleStatus::Initializing => RadarStatusSuggestion {
            status: SuggestedRadarStatus::Formalizing,
            rule_code: "INITIALIZING_INCOMPLETE",
        },
        ProjectLifecycleStatus::Active if phase_kind == Some(PhaseKind::Implementation) => {
            RadarStatusSuggestion {
                status: SuggestedRadarStatus::InDevelopment,
                rule_code: "ACTIVE_IMPLEMENTATION",
            }
        }
        ProjectLifecycleStatus::Active if phase_kind == Some(PhaseKind::Testing) => {
            RadarStatusSuggestion {
                status: SuggestedRadarStatus::Testing,
                rule_code: "ACTIVE_TESTING",
            }
        }
        ProjectLifecycleStatus::Active if phase_kind == Some(PhaseKind::Audit) => {
            RadarStatusSuggestion {
                status: SuggestedRadarStatus::Auditing,
                rule_code: "ACTIVE_AUDIT",
            }
        }
        ProjectLifecycleStatus::Blocked => RadarStatusSuggestion {
            status: SuggestedRadarStatus::Blocked,
            rule_code: "PROJECT_BLOCKED",
        },
        ProjectLifecycleStatus::Paused => RadarStatusSuggestion {
            status: SuggestedRadarStatus::Paused,
            rule_code: "PROJECT_PAUSED",
        },
        ProjectLifecycleStatus::Completed => RadarStatusSuggestion {
            status: SuggestedRadarStatus::Done,
            rule_code: "PROJECT_COMPLETED",
        },
        ProjectLifecycleStatus::Archived => RadarStatusSuggestion {
            status: SuggestedRadarStatus::Archived,
            rule_code: "PROJECT_ARCHIVED",
        },
        ProjectLifecycleStatus::Active => {
            return Err(RadarExportError::UnsupportedStatusMapping {
                lifecycle,
                phase_kind,
            });
        }
    };
    Ok(mapped)
}

/// Builds one deterministic, privacy-minimized Project Radar projection.
///
/// # Errors
///
/// Fails closed on invalid state, missing/mismatched metadata or unmapped status.
#[allow(clippy::too_many_lines)] // The literal profile mapping remains auditable in one builder.
pub fn build_project_projection(
    projection: &ProjectStateProjection,
    context: &RadarExportContext,
) -> Result<LedgerRadarProjectProjection, RadarExportError> {
    let state = projection.state();
    validate_state_integrity(state)?;
    for phase_ref in &state.active_phase_refs {
        exact_phase(projection, context, phase_ref)?;
    }
    let authority_health = authority_health(state);
    let primary_phase = state
        .primary_active_phase_ref
        .as_ref()
        .map(|reference| phase_summary(projection, context, reference))
        .transpose()?;
    let phase_kind = primary_phase.as_ref().map(|phase| phase.kind);
    let spec_ready = authority_health == FreshnessStatus::Current
        && state.roadmap.is_some()
        && state.active_phase_refs.is_empty();
    let suggested = suggested_radar_status(state.project.lifecycle_status, phase_kind, spec_ready)?;
    let mut active_phases = state
        .active_phase_refs
        .iter()
        .map(|reference| phase_summary(projection, context, reference))
        .collect::<Result<Vec<_>, _>>()?;
    active_phases.sort_by(|left, right| left.phase_ref.cmp(&right.phase_ref));

    let checkpoint = state
        .checkpoint_ref
        .as_ref()
        .and_then(|reference| projection.checkpoint(reference));
    let checkpoint_freshness = state
        .checkpoint_ref
        .as_ref()
        .and_then(|reference| projection.checkpoint_freshness(reference))
        .unwrap_or(CheckpointFreshnessStatus::Unknown);
    let project_ref = project_resource(&state.project.project_id)?;
    let mut repository_identity_digests = projection
        .repository_bindings()
        .into_iter()
        .filter(|binding| binding.status == RepositoryBindingStatus::Current)
        .filter_map(|binding| binding.remote_identity_digest)
        .collect::<Vec<_>>();
    repository_identity_digests.sort_unstable();
    repository_identity_digests.dedup();
    let mut aliases = context
        .source_aliases
        .iter()
        .map(|alias| alias.trim().to_owned())
        .filter(|alias| !alias.is_empty())
        .collect::<Vec<_>>();
    aliases.sort();
    aliases.dedup();

    let mut result = LedgerRadarProjectProjection {
        schema_version: SchemaVersion::V0_1_1,
        projection_id: StableId::parse(&format!(
            "radar-projection:{}:{}",
            state.project.project_id.uuid(),
            state.state_version
        ))
        .map_err(|_| RadarExportError::ProfileInvariant("invalid projection identity"))?,
        projection_kind: "LEDGER_PROJECT_SUMMARY".to_owned(),
        source: RadarSource {
            system: "project-ledger".to_owned(),
            product_version: "0.2.0".to_owned(),
            project_ref: project_ref.clone(),
            state_version: state.state_version,
            state_digest: state.state_digest,
            checkpoint_ref: checkpoint
                .map(|value| {
                    ResourceRef::parse(&format!(
                        "project-ledger://checkpoints/{}",
                        value.checkpoint_id
                    ))
                    .map_err(|_| RadarExportError::InvalidResourceReference)
                })
                .transpose()?,
            checkpoint_digest: checkpoint.map(|value| value.checkpoint_digest),
        },
        identity: RadarIdentity {
            ledger_project_ref: project_ref,
            stable_project_id: state.project.project_id.clone(),
            repository_identity_digests,
            source_aliases: aliases,
        },
        project: RadarProjectSummary {
            canonical_title: state.project.title.clone(),
            bounded_summary: state.project.summary.chars().take(512).collect(),
            lifecycle_status: state.project.lifecycle_status,
            suggested_radar_status: suggested.status,
            status_mapping_rule: suggested.rule_code.to_owned(),
        },
        continuity: RadarContinuity {
            primary_phase,
            active_phases,
            progress: derive_progress(projection, context),
            blockers: count_blockers(projection),
            risks: count_risks(projection),
            gates: RadarGateCounts {
                pending_count: count(state.pending_gate_refs.len()),
                revalidation_required_count: count(state.revalidation_required_gate_refs.len()),
            },
            conflicts: RadarConflictCounts {
                critical_open_count: count(
                    state
                        .open_conflict_refs
                        .iter()
                        .filter(|reference| {
                            projection
                                .conflict(reference)
                                .is_some_and(|item| item.severity == Severity::Critical)
                        })
                        .count(),
                ),
            },
            next_action_refs: state.ordered_next_action_refs.clone(),
            preferred_next_action_ref: state.preferred_next_action_ref.clone(),
            last_material_activity_at: state.last_material_activity_at.clone(),
            checkpoint_freshness,
            authority_health,
        },
        attention_signals: attention_signals(projection),
        freshness: RadarFreshness {
            status: overall_freshness(authority_health, checkpoint_freshness),
            evaluated_at: context.observed_at.clone(),
        },
        privacy: RadarPrivacy {
            event_log_included: false,
            evidence_payloads_included: false,
            repository_paths_included: false,
        },
        created_at: context.observed_at.clone(),
        provenance_ref: context.provenance_ref.clone(),
        digest: Sha256Digest::from([0; 32]),
    };
    result.digest = digest_without_field(&result)?;
    Ok(result)
}

/// Builds a sorted, duplicate-free and tombstone-free sync envelope.
///
/// # Errors
///
/// Rejects duplicate identities, altered child digests or invalid locked constants.
pub fn build_sync_envelope(
    mut projects: Vec<LedgerRadarProjectProjection>,
    request: &RadarEnvelopeRequest,
) -> Result<RadarSyncEnvelope, RadarExportError> {
    projects.sort_by(|left, right| left.source.project_ref.cmp(&right.source.project_ref));
    let mut seen = BTreeSet::new();
    for project in &projects {
        let reference = project.source.project_ref.as_str().to_owned();
        if !seen.insert(reference.clone()) {
            return Err(RadarExportError::DuplicateProject {
                project_ref: reference,
            });
        }
        if digest_without_field(project)? != project.digest {
            return Err(RadarExportError::ProjectionDigestMismatch {
                project_ref: reference,
            });
        }
    }
    let mut result = RadarSyncEnvelope {
        schema_version: SchemaVersion::V0_1_1,
        sync_envelope_id: request.sync_envelope_id.clone(),
        profile: RADAR_PROFILE.to_owned(),
        source_instance_ref: request.source_instance_ref.clone(),
        exported_at: request.exported_at.clone(),
        projects,
        tombstones: Vec::new(),
        contract: RadarSyncContract {
            schema_id: ENVELOPE_SCHEMA_ID.to_owned(),
            profile_schema_digest: locked_digest(ENVELOPE_SCHEMA_SHA256)?,
            source_spec_digest: locked_digest(RADAR_SOURCE_SPEC_SHA256)?,
            canonicalization: RadarCanonicalization::JcsRfc8785,
        },
        semantics: RadarSyncSemantics {
            omission_means_delete: false,
            imports_are_idempotent: true,
            name_only_mapping_allowed: false,
        },
        digest: Sha256Digest::from([0; 32]),
    };
    result
        .validate()
        .map_err(|_| RadarExportError::ProfileInvariant("invalid sync semantics"))?;
    result.digest = digest_without_field(&result)?;
    Ok(result)
}

fn exact_phase<'a>(
    projection: &ProjectStateProjection,
    context: &'a RadarExportContext,
    reference: &StableId,
) -> Result<&'a Phase, RadarExportError> {
    let phase = context.phase_catalog.get(reference).ok_or_else(|| {
        RadarExportError::PhaseMetadataMissing {
            phase_ref: reference.to_string(),
        }
    })?;
    if projection.phase_status(reference) != Some(phase.status) {
        return Err(RadarExportError::PhaseMetadataMismatch {
            phase_ref: reference.to_string(),
        });
    }
    Ok(phase)
}

fn phase_summary(
    projection: &ProjectStateProjection,
    context: &RadarExportContext,
    reference: &StableId,
) -> Result<RadarPhaseSummary, RadarExportError> {
    let phase = exact_phase(projection, context, reference)?;
    Ok(RadarPhaseSummary {
        phase_ref: phase.phase_id.clone(),
        title: phase.title.clone(),
        kind: phase.kind,
        status: phase.status,
    })
}

fn derive_progress(
    projection: &ProjectStateProjection,
    context: &RadarExportContext,
) -> RadarProgress {
    let state = projection.state();
    let Some(roadmap) = &state.roadmap else {
        return unavailable_progress("ROADMAP_UNAVAILABLE", 0, 0, 0, 0);
    };
    let total_phases = count(roadmap.phase_order.len());
    let completed = count(
        roadmap
            .phase_order
            .iter()
            .filter(|reference| state.completed_phase_refs.contains(reference))
            .count(),
    );
    let Some(phases) = roadmap
        .phase_order
        .iter()
        .map(|reference| context.phase_catalog.get(reference))
        .collect::<Option<Vec<_>>>()
    else {
        return unavailable_progress(
            "PHASE_DENOMINATOR_INCOMPLETE",
            completed,
            total_phases,
            0,
            0,
        );
    };
    if phases
        .iter()
        .any(|phase| projection.phase_status(&phase.phase_id) != Some(phase.status))
    {
        return unavailable_progress(
            "PHASE_STATUS_METADATA_MISMATCH",
            completed,
            total_phases,
            0,
            0,
        );
    }
    let required_gates = phases
        .iter()
        .flat_map(|phase| phase.required_gate_refs.iter())
        .collect::<BTreeSet<_>>();
    let total_gates = count(required_gates.len());
    let verified = count(
        required_gates
            .iter()
            .filter(|reference| state.verified_gate_refs.contains(reference))
            .count(),
    );
    if total_phases == 0 {
        return unavailable_progress(
            "REQUIRED_PHASE_DENOMINATOR_ZERO",
            completed,
            total_phases,
            verified,
            total_gates,
        );
    }
    let basis_points =
        u16::try_from(u64::from(completed) * 10_000 / u64::from(total_phases)).unwrap_or(10_000);
    RadarProgress {
        method: RadarProgressMethod::LinearRequiredPhases,
        completed_required_phases: completed,
        total_required_phases: total_phases,
        verified_required_gates: verified,
        total_required_gates: total_gates,
        percentage: RadarPercentage {
            available: true,
            basis_points: Some(basis_points),
            unavailable_reason: None,
        },
        status: if completed == total_phases {
            RadarProgressStatus::Complete
        } else {
            RadarProgressStatus::Partial
        },
    }
}

fn unavailable_progress(
    reason: &str,
    completed: u32,
    total: u32,
    verified_gates: u32,
    total_gates: u32,
) -> RadarProgress {
    RadarProgress {
        method: RadarProgressMethod::LinearRequiredPhases,
        completed_required_phases: completed,
        total_required_phases: total,
        verified_required_gates: verified_gates,
        total_required_gates: total_gates,
        percentage: RadarPercentage {
            available: false,
            basis_points: None,
            unavailable_reason: Some(reason.to_owned()),
        },
        status: RadarProgressStatus::Unavailable,
    }
}

fn count_blockers(projection: &ProjectStateProjection) -> RadarCounts {
    let references = &projection.state().open_blocker_refs;
    RadarCounts {
        open_count: count(references.len()),
        critical_count: count(
            references
                .iter()
                .filter(|reference| {
                    projection
                        .blocker(reference)
                        .is_some_and(|item| item.severity == Severity::Critical)
                })
                .count(),
        ),
    }
}

fn count_risks(projection: &ProjectStateProjection) -> RadarCounts {
    let references = &projection.state().open_risk_refs;
    RadarCounts {
        open_count: count(references.len()),
        critical_count: count(
            references
                .iter()
                .filter(|reference| {
                    projection
                        .risk(reference)
                        .is_some_and(|item| item.severity == Severity::Critical)
                })
                .count(),
        ),
    }
}

#[allow(clippy::too_many_lines)] // This is the normative ten-code attention allowlist.
fn attention_signals(projection: &ProjectStateProjection) -> Vec<RadarAttentionSignal> {
    let state = projection.state();
    let mut result = Vec::new();
    if state.project.lifecycle_status == ProjectLifecycleStatus::Blocked {
        signal(
            &mut result,
            "LEDGER_PROJECT_BLOCKED",
            state.project_state_id.clone(),
            Severity::High,
        );
    }
    for reference in &state.open_blocker_refs {
        if projection
            .blocker(reference)
            .is_some_and(|item| item.severity == Severity::Critical)
        {
            signal(
                &mut result,
                "LEDGER_CRITICAL_BLOCKER",
                reference.clone(),
                Severity::Critical,
            );
        }
    }
    for reference in &state.open_risk_refs {
        if projection
            .risk(reference)
            .is_some_and(|item| item.severity == Severity::Critical)
        {
            signal(
                &mut result,
                "LEDGER_CRITICAL_RISK",
                reference.clone(),
                Severity::Critical,
            );
        }
    }
    for reference in &state.pending_gate_refs {
        signal(
            &mut result,
            "LEDGER_GATE_PENDING",
            reference.clone(),
            Severity::High,
        );
    }
    for reference in &state.revalidation_required_gate_refs {
        signal(
            &mut result,
            "LEDGER_GATE_REVALIDATION_REQUIRED",
            reference.clone(),
            Severity::High,
        );
    }
    for document in &state.authority_documents {
        if document.status == AuthorityDocumentStatus::DigestMismatch {
            signal(
                &mut result,
                "LEDGER_AUTHORITY_MISMATCH",
                document.document_id.clone(),
                Severity::Critical,
            );
        }
    }
    if let Some(reference) = &state.checkpoint_ref {
        let freshness = projection
            .checkpoint_freshness(reference)
            .unwrap_or(CheckpointFreshnessStatus::Unknown);
        if !matches!(
            freshness,
            CheckpointFreshnessStatus::Current | CheckpointFreshnessStatus::CurrentForProjectOnly
        ) {
            signal(
                &mut result,
                "LEDGER_CHECKPOINT_STALE",
                reference.clone(),
                Severity::High,
            );
        }
    }
    for reference in &state.open_conflict_refs {
        signal(
            &mut result,
            "LEDGER_CONTINUITY_CONFLICT",
            reference.clone(),
            projection
                .conflict(reference)
                .map_or(Severity::High, |item| item.severity),
        );
    }
    let eligible = state.ordered_next_action_refs.iter().any(|reference| {
        projection.next_action(reference).is_some_and(|item| {
            matches!(
                item.classification,
                NextActionClassification::Eligible | NextActionClassification::Authorized
            )
        })
    });
    if !eligible {
        signal(
            &mut result,
            "LEDGER_NO_ELIGIBLE_NEXT_ACTION",
            state.project_state_id.clone(),
            Severity::Medium,
        );
    }
    if state.last_material_activity_at.is_none() {
        signal(
            &mut result,
            "LEDGER_NO_MATERIAL_ACTIVITY",
            state.project_state_id.clone(),
            Severity::Medium,
        );
    }
    result.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ref.cmp(&right.subject_ref))
    });
    result
}

fn signal(
    target: &mut Vec<RadarAttentionSignal>,
    code: &str,
    subject_ref: StableId,
    severity: Severity,
) {
    target.push(RadarAttentionSignal {
        code: code.to_owned(),
        subject_ref,
        severity,
    });
}

fn authority_health(state: &project_ledger_domain::ProjectState) -> FreshnessStatus {
    if state
        .authority_documents
        .iter()
        .any(|item| item.status == AuthorityDocumentStatus::DigestMismatch)
    {
        FreshnessStatus::Conflicted
    } else if state.authority_documents.is_empty()
        || state
            .authority_documents
            .iter()
            .any(|item| !matches!(item.status, AuthorityDocumentStatus::Current))
    {
        FreshnessStatus::Unavailable
    } else {
        FreshnessStatus::Current
    }
}

fn overall_freshness(
    authority: FreshnessStatus,
    checkpoint: CheckpointFreshnessStatus,
) -> FreshnessStatus {
    match authority {
        FreshnessStatus::Conflicted => FreshnessStatus::Conflicted,
        FreshnessStatus::Unavailable => FreshnessStatus::Unavailable,
        _ if checkpoint == CheckpointFreshnessStatus::Conflicted => FreshnessStatus::Conflicted,
        _ if matches!(
            checkpoint,
            CheckpointFreshnessStatus::StaleForRepository
                | CheckpointFreshnessStatus::StaleForAuthority
                | CheckpointFreshnessStatus::SupersededByNewerCheckpoint
                | CheckpointFreshnessStatus::Invalidated
        ) =>
        {
            FreshnessStatus::Stale
        }
        _ => authority,
    }
}

fn project_resource(
    project: &project_ledger_domain::ProjectId,
) -> Result<ResourceRef, RadarExportError> {
    ResourceRef::parse(&format!("project-ledger://projects/{}", project.uuid()))
        .map_err(|_| RadarExportError::InvalidResourceReference)
}

fn digest_without_field<T: Serialize>(value: &T) -> Result<Sha256Digest, CanonicalJsonError> {
    let mut json = serde_json::to_value(value)?;
    json.as_object_mut()
        .expect("Radar records are objects")
        .remove("digest");
    sha256_digest(&json)
}

fn locked_digest(hex: &str) -> Result<Sha256Digest, RadarExportError> {
    Sha256Digest::parse(&format!("sha256:{hex}"))
        .map_err(|_| RadarExportError::ProfileInvariant("invalid locked digest"))
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
