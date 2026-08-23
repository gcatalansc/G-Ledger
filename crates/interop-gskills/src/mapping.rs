use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    GateStatus, Phase, RepositoryBinding, RepositoryBindingStatus, StableId,
};
use project_ledger_projection_engine::ProjectStateProjection;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{GSKILLS_PROFILE, ProviderContext, ProviderError};

#[allow(clippy::too_many_lines)] // One literal normative mapping keeps all 20 audited fields together.
pub(crate) fn continuity_projection(
    projection: &ProjectStateProjection,
    context: &ProviderContext,
) -> Result<Value, ProviderError> {
    let state = projection.state();
    let primary =
        state
            .primary_active_phase_ref
            .as_ref()
            .ok_or(ProviderError::MissingSourceField {
                field: "primary_active_phase_ref",
            })?;
    let last_activity =
        state
            .last_material_activity_at
            .as_ref()
            .ok_or(ProviderError::MissingSourceField {
                field: "last_material_activity_at",
            })?;
    let repository = current_repository(projection)?;
    let mut value = json!({
        "schema_version": "0.3.0",
        "project_continuity_projection_id": format!("continuity:ledger-{}", state.state_version),
        "source_profile": GSKILLS_PROFILE,
        "project": {
            "project_ref": context.gskills_project_ref,
            "external_project_ref": context.external_project_ref,
            "title": state.project.title,
            "lifecycle_status": enum_name(state.project.lifecycle_status)?,
            "state_version": state.state_version,
            "material_state_version": state.material_state_version,
            "state_digest": state.state_digest,
        },
        "authority_documents": state.authority_documents.iter().map(|document| {
            Ok(json!({
                "ref": document.document_id,
                "uri": document.uri,
                "digest": document.digest,
                "status": enum_name(document.status)?,
                "authority_rank": document.authority_rank,
            }))
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "active_phases": state.active_phase_refs.iter().map(|phase_ref| {
            phase_value(projection, context, phase_ref)
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "primary_active_phase_ref": primary,
        "verified_completed_phases": state.completed_phase_refs.iter().map(|phase_ref| {
            let mut evidence_refs = projection.gates_for_phase(phase_ref).into_iter()
                .filter_map(|gate| projection.current_gate_evaluation(&gate.gate_id))
                .flat_map(|evaluation| evaluation.selected_evidence_refs.iter().cloned())
                .collect::<Vec<_>>();
            evidence_refs.sort();
            evidence_refs.dedup();
            json!({"ref": phase_ref, "status": "COMPLETED", "gate_evidence_refs": evidence_refs})
        }).collect::<Vec<_>>(),
        "current_decisions": state.current_decision_refs.iter().map(|decision_ref| {
            let decision = projection.decision(decision_ref).ok_or_else(|| ProviderError::MissingIndexedRecord {
                kind: "decision",
                reference: decision_ref.to_string(),
            })?;
            Ok(json!({
                "ref": decision.decision_id,
                "title": decision.title,
                "status": enum_name(decision.status)?,
                "decision_version": decision.version,
            }))
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "open_blockers": state.open_blocker_refs.iter().map(|blocker_ref| {
            let blocker = projection.blocker(blocker_ref).ok_or_else(|| ProviderError::MissingIndexedRecord {
                kind: "blocker",
                reference: blocker_ref.to_string(),
            })?;
            Ok(json!({
                "ref": blocker.blocker_id,
                "title": blocker.title,
                "severity": enum_name(blocker.severity)?,
                "status": enum_name(blocker.status)?,
            }))
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "open_risks": state.open_risk_refs.iter().map(|risk_ref| {
            let risk = projection.risk(risk_ref).ok_or_else(|| ProviderError::MissingIndexedRecord {
                kind: "risk",
                reference: risk_ref.to_string(),
            })?;
            Ok(json!({
                "ref": risk.risk_id,
                "title": risk.title,
                "severity": enum_name(risk.severity)?,
                "status": enum_name(risk.status)?,
            }))
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "gates": gate_buckets(projection)?,
        "next_actions": {
            "ordered": state.ordered_next_action_refs.iter().map(|action_ref| {
                let action = projection.next_action(action_ref).ok_or_else(|| ProviderError::MissingIndexedRecord {
                    kind: "next action",
                    reference: action_ref.to_string(),
                })?;
                let classification = enum_name(action.classification)?;
                Ok(json!({
                    "ref": action.next_action_id,
                    "title": action.title,
                    "classification": classification,
                    "status": classification,
                    "authority_ref": action.authority_ref,
                    "gskills_derivation": null,
                }))
            }).collect::<Result<Vec<_>, ProviderError>>()?,
            "preferred_ref": state.preferred_next_action_ref,
        },
        "checkpoint": checkpoint_value(projection)?,
        "repository_binding": repository.map(repository_value).transpose()?.unwrap_or_else(|| json!({})),
        "recent_material_events": recent_material_events(projection),
        "last_material_activity_at": last_activity,
        "unknowns": projection.open_unknown_refs().iter().map(|unknown_ref| {
            let unknown = projection.unknown(unknown_ref).ok_or_else(|| ProviderError::MissingIndexedRecord {
                kind: "unknown",
                reference: unknown_ref.to_string(),
            })?;
            Ok(json!({
                "ref": unknown.unknown_id,
                "subject_kind": unknown.subject_kind,
                "subject_ref": unknown.subject_ref,
                "field_path": unknown.field_path,
                "statement": unknown.statement,
                "criticality": enum_name(unknown.criticality)?,
                "status": enum_name(unknown.status)?,
            }))
        }).collect::<Result<Vec<_>, ProviderError>>()?,
        "conflict_refs": state.open_conflict_refs,
        "provenance_ref": format!("project-ledger://provenance/gskills/continuity/{}", state.state_version),
        "digest": Sha256Digest::from([0; 32]),
    });
    assign_digest(&mut value)?;
    Ok(value)
}

pub(crate) fn current_repository(
    projection: &ProjectStateProjection,
) -> Result<Option<&RepositoryBinding>, ProviderError> {
    let current = projection
        .repository_bindings()
        .into_iter()
        .filter(|binding| binding.status == RepositoryBindingStatus::Current)
        .collect::<Vec<_>>();
    if current.len() > 1 {
        return Err(ProviderError::AmbiguousRepositoryBinding {
            count: current.len(),
        });
    }
    Ok(current.first().copied())
}

fn phase_value(
    projection: &ProjectStateProjection,
    context: &ProviderContext,
    phase_ref: &StableId,
) -> Result<Value, ProviderError> {
    let phase = context.phase_catalog.get(phase_ref).ok_or_else(|| {
        ProviderError::PhaseMetadataMissing {
            phase_ref: phase_ref.to_string(),
        }
    })?;
    let projected_status =
        projection
            .phase_status(phase_ref)
            .ok_or_else(|| ProviderError::MissingIndexedRecord {
                kind: "phase status",
                reference: phase_ref.to_string(),
            })?;
    if phase.status != projected_status {
        return Err(ProviderError::PhaseMetadataMismatch {
            phase_ref: phase_ref.to_string(),
        });
    }
    Ok(json!({
        "ref": phase.phase_id,
        "title": phase.title,
        "kind": enum_name(phase.kind)?,
        "status": enum_name(projected_status)?,
        "started_at": phase.started_at,
    }))
}

fn gate_buckets(projection: &ProjectStateProjection) -> Result<Value, ProviderError> {
    let mut pending = Vec::new();
    let mut revalidation = Vec::new();
    let mut stale = Vec::new();
    let mut verified = Vec::new();
    for gate in projection.gate_definitions() {
        let status = projection.gate_status(&gate.gate_id).ok_or_else(|| {
            ProviderError::MissingIndexedRecord {
                kind: "gate status",
                reference: gate.gate_id.to_string(),
            }
        })?;
        let record = gate_value(gate, status)?;
        match status {
            GateStatus::Stale => stale.push(record),
            GateStatus::RevalidationRequired => revalidation.push(record),
            GateStatus::Verified => verified.push(record),
            GateStatus::Defined
            | GateStatus::Pending
            | GateStatus::Evaluating
            | GateStatus::Rejected => pending.push(record),
            GateStatus::Waived => {}
        }
    }
    Ok(json!({
        "pending": pending,
        "revalidation_required": revalidation,
        "stale": stale,
        "verified": verified,
    }))
}

fn gate_value(
    gate: &project_ledger_domain::GateDefinition,
    status: GateStatus,
) -> Result<Value, ProviderError> {
    let evidence_kinds = gate
        .required_evidence_rules
        .iter()
        .map(|rule| enum_name(rule.kind))
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = gate
        .repository_dependencies
        .paths
        .iter()
        .cloned()
        .chain(
            gate.repository_dependencies
                .external_refs
                .iter()
                .map(ToString::to_string),
        )
        .collect::<Vec<_>>();
    Ok(json!({
        "ref": gate.gate_id,
        "title": gate.title,
        "status": enum_name(status)?,
        "required_evidence_kinds": evidence_kinds,
        "repository_dependency_refs": dependencies,
    }))
}

fn checkpoint_value(projection: &ProjectStateProjection) -> Result<Value, ProviderError> {
    let Some(checkpoint_ref) = projection.state().checkpoint_ref.as_ref() else {
        return Ok(json!({}));
    };
    let checkpoint = projection.checkpoint(checkpoint_ref).ok_or_else(|| {
        ProviderError::MissingIndexedRecord {
            kind: "checkpoint",
            reference: checkpoint_ref.to_string(),
        }
    })?;
    let freshness = projection
        .checkpoint_freshness(checkpoint_ref)
        .ok_or_else(|| ProviderError::MissingIndexedRecord {
            kind: "checkpoint freshness",
            reference: checkpoint_ref.to_string(),
        })?;
    Ok(json!({
        "ref": checkpoint.checkpoint_id,
        "digest": checkpoint.checkpoint_digest,
        "captured_state_version": checkpoint.captured_state_version,
        "captured_material_state_version": checkpoint.captured_material_state_version,
        "captured_state_digest": checkpoint.captured_state_digest,
        "checkpoint_event_version": checkpoint.checkpoint_event_version,
        "checkpoint_event_ref": checkpoint.checkpoint_event_ref,
        "freshness": map_checkpoint_freshness(freshness),
        "created_at": checkpoint.created_at,
    }))
}

fn repository_value(binding: &RepositoryBinding) -> Result<Value, ProviderError> {
    Ok(json!({
        "ref": binding.repository_binding_id,
        "status": enum_name(binding.status)?,
        "repository_ref": binding.repository_ref,
        "repository_snapshot_ref": binding.current_snapshot_ref,
        "repository_snapshot_digest": binding.current_snapshot_digest,
    }))
}

fn recent_material_events(projection: &ProjectStateProjection) -> Vec<Value> {
    projection
        .latest_work_record()
        .zip(projection.state().last_material_event_ref.as_ref())
        .map(|(work, event_ref)| {
            vec![json!({
                "event_ref": event_ref,
                "kind": "WORK_PROGRESS_RECORDED",
                "summary": work.summary,
                "occurred_at": work.occurred_at,
            })]
        })
        .unwrap_or_default()
}

pub(crate) fn map_checkpoint_freshness(
    status: project_ledger_domain::CheckpointFreshnessStatus,
) -> &'static str {
    use project_ledger_domain::CheckpointFreshnessStatus as Status;
    match status {
        Status::Current => "CURRENT",
        Status::CurrentForProjectOnly => "CURRENT_FOR_PROJECT_ONLY",
        Status::StaleForRepository => "STALE_FOR_REPOSITORY_STATE",
        Status::StaleForAuthority => "STALE_FOR_AUTHORITY_DOCUMENTS",
        Status::SupersededByNewerCheckpoint => "SUPERSEDED_BY_NEWER_CHECKPOINT",
        Status::Conflicted => "CONFLICTED",
        Status::Invalidated => "INVALIDATED",
        Status::Unknown => "COMPATIBILITY_UNKNOWN",
    }
}

pub(crate) fn assign_digest(value: &mut Value) -> Result<(), ProviderError> {
    let object = value
        .as_object_mut()
        .ok_or(ProviderError::InternalContract {
            message: "digest-bearing value is not an object",
        })?;
    object.remove("digest");
    let digest = sha256_digest(&*object)?;
    object.insert("digest".to_owned(), Value::String(digest.to_string()));
    Ok(())
}

pub(crate) fn enum_name<T: Serialize>(value: T) -> Result<String, ProviderError> {
    serde_json::to_value(value)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or(ProviderError::InternalContract {
            message: "closed enum did not serialize as a string",
        })
}

pub(crate) fn phase_metadata_is_complete(phase: &Phase) -> bool {
    !phase.title.trim().is_empty()
        && (phase.status != project_ledger_domain::PhaseStatus::Active
            || phase.started_at.is_some())
}
