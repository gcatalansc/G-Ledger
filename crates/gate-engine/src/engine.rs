use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{
    ActorKind, AuthorityDocumentStatus, ClaimClassification, ClaimRecordedEvent,
    ClaimStatusChangedEvent, Command, CommandEnvelope, EngineIdentity, Event, EventId,
    EvidenceInvalidatedEvent, EvidenceStatus, ExternalEvidenceLinkedEvent,
    ExternalEvidenceUnlinkedEvent, GateCompletionOperator, GateDefinedEvent, GateEvaluatedEvent,
    GateEvaluation, GateEvaluationRequestedEvent, GateEvaluationResult, GateMarkedStaleEvent,
    GateRejectedEvent, GateRevalidationRequestedEvent, GateRevalidationRequiredEvent,
    GateRuleResult, GateStatus, GateVerifiedEvent, GateWaivedEvent, PhaseCompletedEvent,
    PhaseReopenedEvent, PhaseStatus, ProjectStatusDerivedEvent, RejectedEvidence,
    RepositoryBindingRefreshedEvent, RepositoryBindingRevokedEvent, RepositoryBoundEvent,
    ResourceRef, RevalidationTrigger, RuleResult, SchemaVersion, StableId,
    VerificationRecordedEvent,
};
use project_ledger_event_store::{EventDraft, IntegrityError, build_event};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectionError, validate_state_integrity,
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::evaluate_predicate;

/// Pure, storage-independent owner of S5 gate/evidence/phase decisions.
#[derive(Clone, Copy, Debug, Default)]
pub struct GateEngine;

/// A fail-closed Gate/Phase Engine command validation failure.
#[derive(Debug, Error)]
pub enum GateEngineError {
    #[error("invalid gate command: {message}")]
    InvalidCommand {
        message: String,
        subject_refs: Vec<StableId>,
    },
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("closed contract encoding failed: {0}")]
    ContractEncoding(#[from] serde_json::Error),
    #[error("could not derive deterministic gate identity")]
    Identity,
}

impl GateEngineError {
    /// Returns stable domain subjects implicated in a validation failure.
    pub fn subject_refs(&self) -> Vec<StableId> {
        match self {
            Self::InvalidCommand { subject_refs, .. } => subject_refs.clone(),
            Self::Projection(_)
            | Self::Integrity(_)
            | Self::Canonical(_)
            | Self::ContractEncoding(_)
            | Self::Identity => Vec::new(),
        }
    }
}

impl GateEngine {
    /// Validates one S5 command and returns its deterministic atomic event batch.
    ///
    /// # Errors
    ///
    /// Fails closed when the stream, evidence, gate transition, waiver or phase
    /// closure violates the v0.2.0 contracts.
    #[allow(clippy::too_many_lines)]
    pub fn handle(
        command: &CommandEnvelope,
        stream: &[project_ledger_domain::EventEnvelope],
    ) -> Result<Vec<EventDraft>, GateEngineError> {
        if stream.len() as u64 != command.expected_state_version {
            return Err(invalid(
                "command expected version does not match the supplied stream",
            ));
        }
        let mut projection = ProjectStateProjection::rebuild(stream)?;
        validate_state_integrity(projection.state())?;
        if projection.state().project.project_id != command.project_ref {
            return Err(invalid(
                "command project does not match the supplied stream",
            ));
        }
        if projection.state().project.archived_at.is_some() {
            return Err(invalid(
                "an archived project rejects gate/evidence mutation",
            ));
        }

        let mut drafts = Vec::new();
        match &command.command {
            Command::BindRepository(payload) => {
                require_human(command)?;
                if payload.binding.confirmed_by.as_ref() != Some(&command.actor.actor_id) {
                    return Err(invalid_subject(
                        "repository binding confirmation must match the submitting human",
                        &payload.binding.repository_binding_id,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::RepositoryBound(RepositoryBoundEvent {
                        binding: payload.binding.clone(),
                    }),
                )?;
            }
            Command::RefreshRepositoryBinding(payload) => {
                let affected = repository_freshness_targets(&projection);
                ensure_dependency_change_is_safe(&projection, &affected)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::RepositoryBindingRefreshed(RepositoryBindingRefreshedEvent {
                        binding_ref: payload.binding_ref.clone(),
                        snapshot: payload.snapshot.clone(),
                    }),
                )?;
                append_repository_freshness(command, &mut projection, &mut drafts, affected)?;
            }
            Command::RevokeRepositoryBinding(payload) => {
                require_human(command)?;
                let affected = repository_freshness_targets(&projection);
                ensure_dependency_change_is_safe(&projection, &affected)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::RepositoryBindingRevoked(RepositoryBindingRevokedEvent {
                        binding_ref: payload.binding_ref.clone(),
                        reason: payload.reason.clone(),
                    }),
                )?;
                append_repository_freshness(command, &mut projection, &mut drafts, affected)?;
            }
            Command::DefineGate(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::GateDefined(GateDefinedEvent {
                    gate: payload.gate.clone(),
                }),
            )?,
            Command::RecordVerificationResult(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::VerificationRecorded(VerificationRecordedEvent {
                    evidence: payload.evidence.clone(),
                }),
            )?,
            Command::RecordClaim(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ClaimRecorded(ClaimRecordedEvent {
                    claim: payload.claim.clone(),
                }),
            )?,
            Command::UpdateClaimStatus(payload) => {
                let prior_status = projection
                    .claim(&payload.claim_ref)
                    .ok_or_else(|| invalid_subject("claim does not exist", &payload.claim_ref))?
                    .classification;
                if payload.classification == ClaimClassification::Verified {
                    require_human(command)?;
                    if payload.evidence_refs.is_empty()
                        || payload.evidence_refs.iter().any(|evidence_ref| {
                            projection.evidence(evidence_ref).is_none_or(|evidence| {
                                evidence.status != EvidenceStatus::VerifiedSource
                            })
                        })
                    {
                        return Err(invalid_subject(
                            "VERIFIED claim requires accepted evidence",
                            &payload.claim_ref,
                        ));
                    }
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::ClaimStatusChanged(ClaimStatusChangedEvent {
                        claim_ref: payload.claim_ref.clone(),
                        prior_status,
                        new_status: payload.classification,
                        evidence_refs: payload.evidence_refs.clone(),
                    }),
                )?;
            }
            Command::RequestGateEvaluation(payload) => {
                let evaluated_state_version = projection.state().state_version;
                let phase_ref = projection
                    .gate(&payload.gate_ref)
                    .ok_or_else(|| invalid_subject("gate does not exist", &payload.gate_ref))?
                    .phase_ref
                    .clone();
                if projection.phase_status(&phase_ref) == Some(PhaseStatus::Completed) {
                    return Err(invalid_subject(
                        "a completed phase requires REQUEST_GATE_REVALIDATION",
                        &payload.gate_ref,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::GateEvaluationRequested(GateEvaluationRequestedEvent {
                        gate_ref: payload.gate_ref.clone(),
                    }),
                )?;
                let evaluation = evaluate_gate(
                    command,
                    &projection,
                    &payload.gate_ref,
                    evaluated_state_version,
                )?;
                let result = evaluation.result;
                let evaluation_ref = evaluation.gate_evaluation_id.clone();
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::GateEvaluated(GateEvaluatedEvent { evaluation }),
                )?;
                append_evaluation_outcome(
                    command,
                    &mut projection,
                    &mut drafts,
                    &payload.gate_ref,
                    &evaluation_ref,
                    result,
                    false,
                )?;
            }
            Command::RequestGateRevalidation(payload) => {
                let evaluated_state_version = projection.state().state_version;
                let phase_ref = projection
                    .gate(&payload.gate_ref)
                    .ok_or_else(|| invalid_subject("gate does not exist", &payload.gate_ref))?
                    .phase_ref
                    .clone();
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::GateRevalidationRequested(GateRevalidationRequestedEvent {
                        gate_ref: payload.gate_ref.clone(),
                        signal_ref: payload.signal_ref.clone(),
                    }),
                )?;
                let evaluation = evaluate_gate(
                    command,
                    &projection,
                    &payload.gate_ref,
                    evaluated_state_version,
                )?;
                let result = evaluation.result;
                let evaluation_ref = evaluation.gate_evaluation_id.clone();
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::GateEvaluated(GateEvaluatedEvent { evaluation }),
                )?;
                append_evaluation_outcome(
                    command,
                    &mut projection,
                    &mut drafts,
                    &payload.gate_ref,
                    &evaluation_ref,
                    result,
                    true,
                )?;
                if result != GateEvaluationResult::Pass {
                    maybe_reopen_phase(
                        command,
                        &mut projection,
                        &mut drafts,
                        &phase_ref,
                        format!("gate revalidation failed: {}", payload.gate_ref),
                    )?;
                }
            }
            Command::WaiveGate(payload) => {
                require_human(command)?;
                if payload.waiver.actor_ref != command.actor.actor_id {
                    return Err(invalid_subject(
                        "waiver actor must match the submitting human",
                        &payload.waiver.actor_ref,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::GateWaived(GateWaivedEvent {
                        waiver: payload.waiver.clone(),
                    }),
                )?;
            }
            Command::RequestPhaseCompletion(payload) => {
                require_human(command)?;
                if !matches!(
                    projection.phase_status(&payload.phase_ref),
                    Some(
                        PhaseStatus::Active
                            | PhaseStatus::ReadyToClose
                            | PhaseStatus::RevalidationRequired
                    )
                ) {
                    return Err(invalid_subject(
                        "only an active, ready, or repaired revalidation phase can request completion",
                        &payload.phase_ref,
                    ));
                }
                if !projection.phase_required_gates_satisfied(&payload.phase_ref) {
                    return Err(invalid_subject(
                        "required phase gates are not all verified or waived",
                        &payload.phase_ref,
                    ));
                }
                if projection.phase_has_closure_impediment(&payload.phase_ref) {
                    return Err(invalid_subject(
                        "a blocker or critical conflict prevents phase completion",
                        &payload.phase_ref,
                    ));
                }
                let roadmap = projection
                    .state()
                    .roadmap
                    .as_ref()
                    .ok_or_else(|| invalid("phase completion requires a roadmap"))?;
                if projection
                    .authority_document(&roadmap.source_document_ref)
                    .is_none_or(|authority| authority.status != AuthorityDocumentStatus::Current)
                {
                    return Err(invalid_subject(
                        "roadmap authority must be current for phase completion",
                        &roadmap.source_document_ref,
                    ));
                }
                let mut gate_evaluation_refs = projection
                    .gates_for_phase(&payload.phase_ref)
                    .into_iter()
                    .filter(|gate| {
                        gate.criticality == project_ledger_domain::GateCriticality::Required
                            && projection.gate_status(&gate.gate_id) == Some(GateStatus::Verified)
                    })
                    .filter_map(|gate| {
                        projection
                            .current_gate_evaluation(&gate.gate_id)
                            .map(|evaluation| evaluation.gate_evaluation_id.clone())
                    })
                    .collect::<Vec<_>>();
                gate_evaluation_refs.sort();
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::PhaseCompleted(PhaseCompletedEvent {
                        phase_ref: payload.phase_ref.clone(),
                        gate_evaluation_refs,
                    }),
                )?;
            }
            Command::InvalidateEvidence(payload) => {
                let dependent = projection
                    .gates_depending_on_evidence(&payload.evidence_ref)
                    .into_iter()
                    .filter(|gate| {
                        matches!(
                            projection.gate_status(&gate.gate_id),
                            Some(GateStatus::Verified | GateStatus::Waived)
                        )
                    })
                    .map(|gate| (gate.gate_id.clone(), gate.phase_ref.clone()))
                    .collect::<Vec<_>>();
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::EvidenceInvalidated(EvidenceInvalidatedEvent {
                        evidence_ref: payload.evidence_ref.clone(),
                        reason: payload.reason.clone(),
                    }),
                )?;
                let mut affected_phases = BTreeSet::new();
                for (gate_ref, phase_ref) in dependent {
                    append(
                        command,
                        &mut projection,
                        &mut drafts,
                        Event::GateMarkedStale(GateMarkedStaleEvent {
                            gate_ref: gate_ref.clone(),
                            reason_codes: vec!["EVIDENCE_INVALIDATED".to_owned()],
                        }),
                    )?;
                    append(
                        command,
                        &mut projection,
                        &mut drafts,
                        Event::GateRevalidationRequired(GateRevalidationRequiredEvent {
                            gate_ref,
                            prior_status: GateStatus::Stale,
                            reason_codes: vec!["EVIDENCE_INVALIDATED".to_owned()],
                        }),
                    )?;
                    affected_phases.insert(phase_ref);
                }
                for phase_ref in affected_phases {
                    maybe_reopen_phase(
                        command,
                        &mut projection,
                        &mut drafts,
                        &phase_ref,
                        format!("evidence invalidated: {}", payload.evidence_ref),
                    )?;
                }
            }
            Command::LinkExternalEvidence(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ExternalEvidenceLinked(ExternalEvidenceLinkedEvent {
                    evidence_ref: stable_command_id("evidence", command)?,
                    external_system: payload.external_system.clone(),
                    artifact_uri: payload.artifact_uri.clone(),
                    artifact_digest: payload.artifact_digest,
                    schema_ref: payload.schema_ref.clone(),
                }),
            )?,
            Command::UnlinkExternalEvidence(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ExternalEvidenceUnlinked(ExternalEvidenceUnlinkedEvent {
                    evidence_ref: payload.evidence_ref.clone(),
                    reason: payload.reason.clone(),
                }),
            )?,
            _ => return Err(invalid("command does not belong to the S5 gate engine")),
        }
        append_status_if_changed(command, &mut projection, &mut drafts)?;
        Ok(drafts)
    }

    /// Extends an authority-owned lifecycle batch with matrix-allowed gate
    /// freshness transitions.
    ///
    /// # Errors
    ///
    /// Fails closed for an unrelated command, invalid base draft, or an
    /// affected completed/ready required phase that the authority command's
    /// closed event matrix cannot safely reopen.
    pub fn extend_authority_freshness(
        command: &CommandEnvelope,
        stream: &[project_ledger_domain::EventEnvelope],
        base_drafts: Vec<EventDraft>,
    ) -> Result<Vec<EventDraft>, GateEngineError> {
        if stream.len() as u64 != command.expected_state_version {
            return Err(invalid(
                "command expected version does not match the supplied stream",
            ));
        }
        if !matches!(
            &command.command,
            Command::RegisterAuthorityDocument(_)
                | Command::RefreshAuthorityDocumentDigest(_)
                | Command::SupersedeAuthorityDocument(_)
        ) {
            return Err(invalid(
                "freshness extension accepts only authority dependency commands",
            ));
        }
        let material_change = match &command.command {
            Command::RefreshAuthorityDocumentDigest(_) => base_drafts.iter().any(|draft| {
                matches!(
                    &draft.event,
                    Event::AuthorityDocumentMarkedUnavailable(_)
                        | Event::AuthorityDocumentDigestMismatch(_)
                )
            }),
            _ => true,
        };
        let mut projection = ProjectStateProjection::rebuild(stream)?;
        let affected = if material_change {
            projection
                .gate_definitions()
                .into_iter()
                .filter(|gate| {
                    gate.revalidate_on
                        .contains(&RevalidationTrigger::AuthorityDocumentChanged)
                        && matches!(
                            projection.gate_status(&gate.gate_id),
                            Some(GateStatus::Verified | GateStatus::Waived)
                        )
                })
                .map(|gate| {
                    (
                        gate.gate_id.clone(),
                        gate.phase_ref.clone(),
                        gate.criticality,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if affected.iter().any(|(_, phase_ref, criticality)| {
            *criticality == project_ledger_domain::GateCriticality::Required
                && matches!(
                    projection.phase_status(phase_ref),
                    Some(PhaseStatus::Completed | PhaseStatus::ReadyToClose)
                )
        }) {
            return Err(invalid(
                "the closed authority-command matrix cannot safely reopen an affected completed phase",
            ));
        }

        let mut drafts = Vec::with_capacity(base_drafts.len() + affected.len() * 2);
        for draft in base_drafts {
            preview_existing(command, &mut projection, &mut drafts, draft)?;
        }
        for (gate_ref, _, _) in affected {
            append(
                command,
                &mut projection,
                &mut drafts,
                Event::GateMarkedStale(GateMarkedStaleEvent {
                    gate_ref: gate_ref.clone(),
                    reason_codes: vec!["AUTHORITY_DOCUMENT_CHANGED".to_owned()],
                }),
            )?;
            append(
                command,
                &mut projection,
                &mut drafts,
                Event::GateRevalidationRequired(GateRevalidationRequiredEvent {
                    gate_ref,
                    prior_status: GateStatus::Stale,
                    reason_codes: vec!["AUTHORITY_DOCUMENT_CHANGED".to_owned()],
                }),
            )?;
        }
        append_status_if_changed(command, &mut projection, &mut drafts)?;
        Ok(drafts)
    }
}

fn repository_freshness_targets(
    projection: &ProjectStateProjection,
) -> Vec<(StableId, StableId, project_ledger_domain::GateCriticality)> {
    projection
        .gate_definitions()
        .into_iter()
        .filter(|gate| {
            gate.revalidate_on
                .contains(&RevalidationTrigger::RepositoryDependencyChanged)
                && matches!(
                    projection.gate_status(&gate.gate_id),
                    Some(GateStatus::Verified | GateStatus::Waived)
                )
        })
        .map(|gate| {
            (
                gate.gate_id.clone(),
                gate.phase_ref.clone(),
                gate.criticality,
            )
        })
        .collect()
}

fn ensure_dependency_change_is_safe(
    projection: &ProjectStateProjection,
    affected: &[(StableId, StableId, project_ledger_domain::GateCriticality)],
) -> Result<(), GateEngineError> {
    if affected.iter().any(|(_, phase_ref, criticality)| {
        *criticality == project_ledger_domain::GateCriticality::Required
            && matches!(
                projection.phase_status(phase_ref),
                Some(PhaseStatus::Completed | PhaseStatus::ReadyToClose)
            )
    }) {
        return Err(invalid(
            "the closed dependency-command matrix cannot safely reopen an affected completed phase",
        ));
    }
    Ok(())
}

fn append_repository_freshness(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    affected: Vec<(StableId, StableId, project_ledger_domain::GateCriticality)>,
) -> Result<(), GateEngineError> {
    for (gate_ref, _, _) in affected {
        append(
            command,
            projection,
            drafts,
            Event::GateMarkedStale(GateMarkedStaleEvent {
                gate_ref: gate_ref.clone(),
                reason_codes: vec!["REPOSITORY_DEPENDENCY_CHANGED".to_owned()],
            }),
        )?;
        append(
            command,
            projection,
            drafts,
            Event::GateRevalidationRequired(GateRevalidationRequiredEvent {
                gate_ref,
                prior_status: GateStatus::Stale,
                reason_codes: vec!["REPOSITORY_DEPENDENCY_CHANGED".to_owned()],
            }),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // One linear construction keeps every GateEvaluation field auditable.
fn evaluate_gate(
    command: &CommandEnvelope,
    projection: &ProjectStateProjection,
    gate_ref: &StableId,
    project_state_version: u64,
) -> Result<GateEvaluation, GateEngineError> {
    let gate = projection
        .gate(gate_ref)
        .ok_or_else(|| invalid_subject("gate does not exist", gate_ref))?;
    let records = projection
        .evidence_records()
        .into_iter()
        .map(|record| (record.evidence_id.clone(), record.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut selected_evidence_refs = Vec::new();
    let mut rejected_evidence = Vec::new();
    let mut rule_results = Vec::with_capacity(gate.required_evidence_rules.len());

    for rule in &gate.required_evidence_rules {
        let mut candidates = records
            .values()
            .filter(|record| record.kind == rule.kind)
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
        let mut outcomes = Vec::new();
        let mut selected = None;
        for candidate in candidates {
            if candidate.status != EvidenceStatus::VerifiedSource {
                rejected_evidence.push(RejectedEvidence {
                    evidence_ref: candidate.evidence_id.clone(),
                    reason: format!("EVIDENCE_STATUS_{:?}", candidate.status).to_uppercase(),
                });
                outcomes.push(
                    if matches!(
                        candidate.status,
                        EvidenceStatus::Stale | EvidenceStatus::Invalidated
                    ) {
                        RuleResult::Stale
                    } else {
                        RuleResult::Missing
                    },
                );
                continue;
            }
            if gate
                .freshness_policy
                .evidence_must_match_same_repository_snapshot
                && candidate.repository_binding.is_none()
            {
                rejected_evidence.push(RejectedEvidence {
                    evidence_ref: candidate.evidence_id.clone(),
                    reason: "MISSING_REPOSITORY_SNAPSHOT".to_owned(),
                });
                outcomes.push(RuleResult::Missing);
                continue;
            }
            let outcome = evaluate_predicate(&rule.predicate, candidate, &records);
            outcomes.push(outcome);
            if outcome == RuleResult::Pass && selected.is_none() {
                selected = Some(candidate.evidence_id.clone());
            } else if outcome != RuleResult::Pass {
                rejected_evidence.push(RejectedEvidence {
                    evidence_ref: candidate.evidence_id.clone(),
                    reason: format!("PREDICATE_{outcome:?}").to_uppercase(),
                });
            }
        }
        let result = if selected.is_some() {
            RuleResult::Pass
        } else {
            aggregate_candidate_outcomes(&outcomes)
        };
        if let Some(evidence_ref) = selected {
            selected_evidence_refs.push(evidence_ref);
        }
        rule_results.push(GateRuleResult {
            rule_id: rule.rule_id.clone(),
            result,
        });
    }
    selected_evidence_refs.sort();
    selected_evidence_refs.dedup();
    rejected_evidence.sort_by(|left, right| {
        left.evidence_ref
            .cmp(&right.evidence_ref)
            .then_with(|| left.reason.cmp(&right.reason))
    });
    rejected_evidence.dedup_by(|left, right| {
        left.evidence_ref == right.evidence_ref && left.reason == right.reason
    });

    let mut selected_bindings = Vec::new();
    for binding in selected_evidence_refs.iter().filter_map(|evidence_ref| {
        records
            .get(evidence_ref)
            .and_then(|record| record.repository_binding.clone())
    }) {
        if !selected_bindings.contains(&binding) {
            selected_bindings.push(binding);
        }
    }
    let snapshot_conflict = gate
        .freshness_policy
        .evidence_must_match_same_repository_snapshot
        && selected_bindings.len() > 1;
    let result = if snapshot_conflict {
        GateEvaluationResult::Conflicted
    } else {
        aggregate_gate_result(gate.completion_rule, &rule_results)
    };
    let proposed_gate_status = match result {
        GateEvaluationResult::Pass => GateStatus::Verified,
        GateEvaluationResult::Fail => GateStatus::Rejected,
        GateEvaluationResult::Stale => GateStatus::RevalidationRequired,
        GateEvaluationResult::Incomplete
        | GateEvaluationResult::Conflicted
        | GateEvaluationResult::Error => GateStatus::Pending,
    };
    let mut explanation_codes = Vec::new();
    if rule_results
        .iter()
        .any(|rule| rule.result == RuleResult::Missing)
    {
        explanation_codes.push("REQUIRED_EVIDENCE_MISSING".to_owned());
    }
    if rule_results
        .iter()
        .any(|rule| rule.result == RuleResult::Fail)
    {
        explanation_codes.push("EVIDENCE_PREDICATE_FAILED".to_owned());
    }
    if rule_results
        .iter()
        .any(|rule| rule.result == RuleResult::Stale)
    {
        explanation_codes.push("EVIDENCE_STALE".to_owned());
    }
    if snapshot_conflict {
        explanation_codes.push("REPOSITORY_SNAPSHOT_CONFLICT".to_owned());
    }
    let gate_evaluation_id = stable_command_id("gate-eval", command)?;
    let provenance_ref = ResourceRef::parse(&format!(
        "project-ledger://provenance/gate-evaluations/{}",
        command.command_id.uuid()
    ))
    .map_err(|_| GateEngineError::Identity)?;
    let repository_binding = if selected_bindings.len() == 1 {
        selected_bindings.into_iter().next()
    } else {
        None
    };
    let mut evaluation = GateEvaluation {
        schema_version: SchemaVersion::V0_2_0,
        gate_evaluation_id,
        gate_ref: gate.gate_id.clone(),
        gate_definition_version: gate.definition_version,
        gate_definition_digest: gate.digest,
        project_state_version,
        repository_binding,
        selected_evidence_refs,
        rejected_evidence,
        rule_results,
        result,
        proposed_gate_status,
        explanation_codes,
        evaluated_by: EngineIdentity {
            engine: "project-ledger-gate-engine".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        evaluated_at: command.submitted_at.clone(),
        provenance_ref,
        digest: Sha256Digest::from([0_u8; 32]),
    };
    evaluation.digest = record_digest(&evaluation)?;
    Ok(evaluation)
}

fn append_evaluation_outcome(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    gate_ref: &StableId,
    evaluation_ref: &StableId,
    result: GateEvaluationResult,
    revalidation: bool,
) -> Result<(), GateEngineError> {
    match result {
        GateEvaluationResult::Pass => append(
            command,
            projection,
            drafts,
            Event::GateVerified(GateVerifiedEvent {
                gate_ref: gate_ref.clone(),
                evaluation_ref: evaluation_ref.clone(),
            }),
        )?,
        GateEvaluationResult::Fail => append(
            command,
            projection,
            drafts,
            Event::GateRejected(GateRejectedEvent {
                gate_ref: gate_ref.clone(),
                evaluation_ref: evaluation_ref.clone(),
            }),
        )?,
        GateEvaluationResult::Incomplete
        | GateEvaluationResult::Stale
        | GateEvaluationResult::Conflicted
        | GateEvaluationResult::Error
            if revalidation =>
        {
            let prior_status = projection
                .gate_status(gate_ref)
                .ok_or_else(|| invalid_subject("gate does not exist", gate_ref))?;
            append(
                command,
                projection,
                drafts,
                Event::GateRevalidationRequired(GateRevalidationRequiredEvent {
                    gate_ref: gate_ref.clone(),
                    prior_status,
                    reason_codes: vec![format!("EVALUATION_{result:?}").to_uppercase()],
                }),
            )?;
        }
        GateEvaluationResult::Incomplete
        | GateEvaluationResult::Stale
        | GateEvaluationResult::Conflicted
        | GateEvaluationResult::Error => {}
    }
    Ok(())
}

fn aggregate_candidate_outcomes(outcomes: &[RuleResult]) -> RuleResult {
    if outcomes.is_empty() {
        return RuleResult::Missing;
    }
    for result in [
        RuleResult::Error,
        RuleResult::Conflicted,
        RuleResult::Stale,
        RuleResult::Missing,
        RuleResult::Fail,
    ] {
        if outcomes.contains(&result) {
            return result;
        }
    }
    RuleResult::Missing
}

fn aggregate_gate_result(
    operator: GateCompletionOperator,
    rules: &[GateRuleResult],
) -> GateEvaluationResult {
    let any = |result| rules.iter().any(|rule| rule.result == result);
    let all = |result| rules.iter().all(|rule| rule.result == result);
    match operator {
        GateCompletionOperator::All if all(RuleResult::Pass) => GateEvaluationResult::Pass,
        GateCompletionOperator::Any if any(RuleResult::Pass) => GateEvaluationResult::Pass,
        _ if any(RuleResult::Error) => GateEvaluationResult::Error,
        _ if any(RuleResult::Conflicted) => GateEvaluationResult::Conflicted,
        _ if any(RuleResult::Stale) => GateEvaluationResult::Stale,
        GateCompletionOperator::All if any(RuleResult::Fail) => GateEvaluationResult::Fail,
        GateCompletionOperator::Any if all(RuleResult::Fail) => GateEvaluationResult::Fail,
        _ => GateEvaluationResult::Incomplete,
    }
}

fn maybe_reopen_phase(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    phase_ref: &StableId,
    reason: String,
) -> Result<(), GateEngineError> {
    if projection.phase_status(phase_ref) == Some(PhaseStatus::Completed) {
        append(
            command,
            projection,
            drafts,
            Event::PhaseReopened(PhaseReopenedEvent {
                phase_ref: phase_ref.clone(),
                resulting_status: PhaseStatus::RevalidationRequired,
                reason,
            }),
        )?;
    }
    Ok(())
}

fn append_status_if_changed(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), GateEngineError> {
    let previous_status = projection.state().project.lifecycle_status;
    let new_status = projection.derived_lifecycle_status();
    if previous_status == new_status {
        return Ok(());
    }
    let reason_code = serde_json::to_value(command.command_type())?
        .as_str()
        .ok_or(GateEngineError::Identity)?
        .to_owned();
    append(
        command,
        projection,
        drafts,
        Event::ProjectStatusDerived(ProjectStatusDerivedEvent {
            previous_status,
            new_status,
            reason_codes: vec![reason_code],
        }),
    )
}

fn append(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    event: Event,
) -> Result<(), GateEngineError> {
    let draft = EventDraft {
        event_id: deterministic_event_id(command, drafts.len())?,
        event,
        correlation_ref: stable_command_id("correlation", command)?,
        occurred_at: command.submitted_at.clone(),
        recorded_at: command.submitted_at.clone(),
    };
    let head = projection.head_digest();
    let envelope = build_event(
        command,
        projection.state().state_version + 1,
        Some(head),
        draft.clone(),
    )?;
    *projection = projection.clone().apply(head, &envelope)?;
    drafts.push(draft);
    Ok(())
}

fn preview_existing(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    draft: EventDraft,
) -> Result<(), GateEngineError> {
    let head = projection.head_digest();
    let envelope = build_event(
        command,
        projection.state().state_version + 1,
        Some(head),
        draft.clone(),
    )?;
    *projection = projection.clone().apply(head, &envelope)?;
    drafts.push(draft);
    Ok(())
}

fn deterministic_event_id(
    command: &CommandEnvelope,
    ordinal: usize,
) -> Result<EventId, GateEngineError> {
    let command_uuid = command.command_id.uuid();
    let mut hasher = Sha256::new();
    hasher.update(b"project-ledger:event-id:v1");
    hasher.update(command_uuid.as_bytes());
    hasher.update(
        u64::try_from(ordinal)
            .map_err(|_| GateEngineError::Identity)?
            .to_be_bytes(),
    );
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes[..6].copy_from_slice(&command_uuid.as_bytes()[..6]);
    bytes[6..].copy_from_slice(&digest[..10]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    EventId::parse(&format!("event:{}", Uuid::from_bytes(bytes)))
        .map_err(|_| GateEngineError::Identity)
}

fn stable_command_id(prefix: &str, command: &CommandEnvelope) -> Result<StableId, GateEngineError> {
    StableId::parse(&format!("{prefix}:{}", command.command_id.uuid()))
        .map_err(|_| GateEngineError::Identity)
}

fn record_digest<T: serde::Serialize>(record: &T) -> Result<Sha256Digest, GateEngineError> {
    let mut value = serde_json::to_value(record)?;
    value
        .as_object_mut()
        .ok_or(GateEngineError::Identity)?
        .remove("digest");
    Ok(sha256_digest(&value)?)
}

fn require_human(command: &CommandEnvelope) -> Result<(), GateEngineError> {
    if command.actor.actor_kind != ActorKind::Human {
        return Err(invalid(
            "this phase/gate command requires an explicit human actor",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> GateEngineError {
    GateEngineError::InvalidCommand {
        message: message.to_owned(),
        subject_refs: Vec::new(),
    }
}

fn invalid_subject(message: &str, subject_ref: &StableId) -> GateEngineError {
    GateEngineError::InvalidCommand {
        message: message.to_owned(),
        subject_refs: vec![subject_ref.clone()],
    }
}
