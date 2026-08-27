use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    AuthorityDocumentStatus, Blocker, BlockerStatus, CheckpointFreshnessRecord,
    CheckpointFreshnessStatus, ClaimClassification, ClaimRecord, ConflictKind, ConflictRecord,
    ConflictStatus, ContinuityCheckpoint, Decision, DecisionStatus, Event, EventEnvelope,
    EvidenceRecord, EvidenceStatus, GateCompletionOperator, GateCriticality, GateDefinition,
    GateEvaluation, GateEvaluationResult, GateStatus, GateWaiver, NextAction,
    NextActionClassification, PhaseStatus, ProjectLifecycleStatus, ProjectState, RepositoryBinding,
    RepositoryBindingStatus, RepositorySnapshot, Risk, RiskStatus, Roadmap, SchemaVersion,
    ScopeKind, Severity, StableId, UnknownCriticality, UnknownRecord, UnknownStatus, WorkRecord,
};
use serde::Serialize;

use crate::project_state::{
    ProjectionError, advance_material_state, advance_non_material_state, genesis_state,
    validate_next_event,
};

mod auxiliary;
use auxiliary::AuxiliaryRecords;

/// Stateful deterministic fold retaining indexes that are intentionally absent
/// from the compact public `ProjectState` contract.
#[derive(Clone, Debug)]
pub struct ProjectStateProjection {
    auxiliary: AuxiliaryRecords,
    state: ProjectState,
    head_digest: Sha256Digest,
    phase_statuses: BTreeMap<StableId, PhaseStatus>,
    decisions: BTreeMap<StableId, Decision>,
    blockers: BTreeMap<StableId, Blocker>,
    risks: BTreeMap<StableId, Risk>,
    next_actions: BTreeMap<StableId, NextAction>,
    conflicts: BTreeMap<StableId, ConflictRecord>,
    gates: BTreeMap<StableId, GateDefinition>,
    gate_statuses: BTreeMap<StableId, GateStatus>,
    gate_evaluations: BTreeMap<StableId, GateEvaluation>,
    current_gate_evaluation_refs: BTreeMap<StableId, StableId>,
    gate_waivers: BTreeMap<StableId, GateWaiver>,
    evidence: BTreeMap<StableId, EvidenceRecord>,
    external_evidence_refs: BTreeSet<StableId>,
    claims: BTreeMap<StableId, ClaimRecord>,
    repository_bindings: BTreeMap<StableId, RepositoryBinding>,
    work_records: BTreeMap<StableId, WorkRecord>,
    latest_work_record_ref: Option<StableId>,
    unknowns: BTreeMap<StableId, UnknownRecord>,
    checkpoints: BTreeMap<StableId, ContinuityCheckpoint>,
    checkpoint_freshness_records: Vec<CheckpointFreshnessRecord>,
    checkpoint_freshness: BTreeMap<StableId, CheckpointFreshnessStatus>,
}

impl ProjectStateProjection {
    /// Rebuilds the lifecycle projection from canonical stream genesis.
    ///
    /// # Errors
    ///
    /// Fails closed when event integrity, ordering, identity or lifecycle
    /// invariants do not hold.
    pub fn rebuild(events: &[EventEnvelope]) -> Result<Self, ProjectionError> {
        let (state, head_digest) = genesis_state(events)?;
        let mut projection = Self {
            auxiliary: AuxiliaryRecords::default(),
            state,
            head_digest,
            phase_statuses: BTreeMap::new(),
            decisions: BTreeMap::new(),
            blockers: BTreeMap::new(),
            risks: BTreeMap::new(),
            next_actions: BTreeMap::new(),
            conflicts: BTreeMap::new(),
            gates: BTreeMap::new(),
            gate_statuses: BTreeMap::new(),
            gate_evaluations: BTreeMap::new(),
            current_gate_evaluation_refs: BTreeMap::new(),
            gate_waivers: BTreeMap::new(),
            evidence: BTreeMap::new(),
            external_evidence_refs: BTreeSet::new(),
            claims: BTreeMap::new(),
            repository_bindings: BTreeMap::new(),
            work_records: BTreeMap::new(),
            latest_work_record_ref: None,
            unknowns: BTreeMap::new(),
            checkpoints: BTreeMap::new(),
            checkpoint_freshness_records: Vec::new(),
            checkpoint_freshness: BTreeMap::new(),
        };
        for event in &events[1..] {
            let expected_head = projection.head_digest;
            projection = projection.apply(expected_head, event)?;
        }
        if projection.state.project.lifecycle_status != projection.derived_lifecycle_status() {
            return Err(invariant(
                "the complete stream ends without its required project status derivation",
            ));
        }
        projection.validate_completed_phase_gates()?;
        Ok(projection)
    }

    /// Applies exactly one canonical successor while preserving private indexes.
    ///
    /// # Errors
    ///
    /// Fails closed for a wrong head, invalid event or forbidden transition.
    pub fn apply(
        mut self,
        expected_previous_event_digest: Sha256Digest,
        event: &EventEnvelope,
    ) -> Result<Self, ProjectionError> {
        if expected_previous_event_digest != self.head_digest {
            return Err(invariant(
                "the supplied head does not match the projection head",
            ));
        }
        validate_next_event(&self.state, expected_previous_event_digest, event)?;
        self.reduce(event)?;
        if matches!(
            event.event,
            Event::NoteRecorded(_) | Event::WorkProgressRecorded(_)
        ) {
            self.auxiliary
                .descriptive_events
                .insert(event.event_id.clone(), event.event.clone());
        }
        if is_non_material_continuity_event(&event.event) {
            advance_non_material_state(&mut self.state, event)?;
        } else {
            advance_material_state(&mut self.state, event)?;
        }
        self.head_digest = event.event_digest;
        Ok(self)
    }

    /// Returns the compact canonical project state.
    pub const fn state(&self) -> &ProjectState {
        &self.state
    }

    /// Consumes the cursor and returns its compact canonical project state.
    pub fn into_state(self) -> ProjectState {
        self.state
    }

    /// Returns the exact private lifecycle status of a roadmap phase.
    pub fn phase_status(&self, phase_ref: &StableId) -> Option<PhaseStatus> {
        self.phase_statuses.get(phase_ref).copied()
    }

    /// Returns a current immutable gate definition.
    pub fn gate(&self, gate_ref: &StableId) -> Option<&GateDefinition> {
        self.gates.get(gate_ref)
    }

    /// Returns all immutable gate definitions in stable identifier order.
    pub fn gate_definitions(&self) -> Vec<&GateDefinition> {
        self.gates.values().collect()
    }

    /// Returns the current derived status of a gate.
    pub fn gate_status(&self, gate_ref: &StableId) -> Option<GateStatus> {
        self.gate_statuses.get(gate_ref).copied()
    }

    /// Returns an immutable gate evaluation by reference.
    pub fn gate_evaluation(&self, evaluation_ref: &StableId) -> Option<&GateEvaluation> {
        self.gate_evaluations.get(evaluation_ref)
    }

    /// Returns the current evaluation selected for a gate, if any.
    pub fn current_gate_evaluation(&self, gate_ref: &StableId) -> Option<&GateEvaluation> {
        self.current_gate_evaluation_refs
            .get(gate_ref)
            .and_then(|evaluation_ref| self.gate_evaluations.get(evaluation_ref))
    }

    /// Returns the latest durable waiver for a gate, if any.
    pub fn gate_waiver(&self, gate_ref: &StableId) -> Option<&GateWaiver> {
        self.gate_waivers.get(gate_ref)
    }

    /// Returns the current derived form of an evidence record.
    pub fn evidence(&self, evidence_ref: &StableId) -> Option<&EvidenceRecord> {
        self.evidence.get(evidence_ref)
    }

    /// Returns all current evidence records in stable identifier order.
    pub fn evidence_records(&self) -> Vec<&EvidenceRecord> {
        self.evidence.values().collect()
    }

    /// Returns the current classification and evidence links of a claim.
    pub fn claim(&self, claim_ref: &StableId) -> Option<&ClaimRecord> {
        self.claims.get(claim_ref)
    }

    /// Returns the current derived repository binding.
    pub fn repository_binding(&self, binding_ref: &StableId) -> Option<&RepositoryBinding> {
        self.repository_bindings.get(binding_ref)
    }

    /// Returns repository bindings in stable identifier order.
    pub fn repository_bindings(&self) -> Vec<&RepositoryBinding> {
        self.repository_bindings.values().collect()
    }

    /// Returns an immutable continuity checkpoint.
    pub fn checkpoint(&self, checkpoint_ref: &StableId) -> Option<&ContinuityCheckpoint> {
        self.checkpoints.get(checkpoint_ref)
    }

    /// Returns the latest derived freshness status for a checkpoint.
    pub fn checkpoint_freshness(
        &self,
        checkpoint_ref: &StableId,
    ) -> Option<CheckpointFreshnessStatus> {
        self.checkpoint_freshness.get(checkpoint_ref).copied()
    }

    /// Returns every append-only derived checkpoint freshness record.
    pub fn checkpoint_freshness_records(&self) -> &[CheckpointFreshnessRecord] {
        &self.checkpoint_freshness_records
    }

    /// Returns the most recently recorded material work fact.
    pub fn latest_work_record(&self) -> Option<&WorkRecord> {
        self.latest_work_record_ref
            .as_ref()
            .and_then(|work_ref| self.work_records.get(work_ref))
    }

    /// Returns a current derived unknown record.
    pub fn unknown(&self, unknown_ref: &StableId) -> Option<&UnknownRecord> {
        self.unknowns.get(unknown_ref)
    }

    /// Returns open material unknown references in stable order.
    pub fn critical_unknown_refs(&self) -> Vec<StableId> {
        self.unknowns
            .values()
            .filter(|unknown| {
                unknown.status == UnknownStatus::Open
                    && unknown.criticality == UnknownCriticality::Material
            })
            .map(|unknown| unknown.unknown_id.clone())
            .collect()
    }

    /// Returns every open unknown reference in stable identifier order.
    pub fn open_unknown_refs(&self) -> Vec<StableId> {
        self.unknowns
            .values()
            .filter(|unknown| unknown.status == UnknownStatus::Open)
            .map(|unknown| unknown.unknown_id.clone())
            .collect()
    }

    /// Returns current gate definitions for a phase in stable identifier order.
    pub fn gates_for_phase(&self, phase_ref: &StableId) -> Vec<&GateDefinition> {
        self.gates
            .values()
            .filter(|gate| &gate.phase_ref == phase_ref)
            .collect()
    }

    /// Returns gates whose current evaluation selected the evidence reference.
    pub fn gates_depending_on_evidence(&self, evidence_ref: &StableId) -> Vec<&GateDefinition> {
        self.gates
            .values()
            .filter(|gate| {
                self.current_gate_evaluation(&gate.gate_id)
                    .is_some_and(|evaluation| {
                        evaluation.selected_evidence_refs.contains(evidence_ref)
                    })
            })
            .collect()
    }

    /// Reports whether every required gate of a phase is currently satisfied.
    pub fn phase_required_gates_satisfied(&self, phase_ref: &StableId) -> bool {
        self.gates
            .values()
            .filter(|gate| {
                &gate.phase_ref == phase_ref && gate.criticality == GateCriticality::Required
            })
            .all(|gate| {
                matches!(
                    self.gate_status(&gate.gate_id),
                    Some(GateStatus::Verified | GateStatus::Waived)
                )
            })
    }

    /// Reports whether a phase/project blocker or critical open conflict prevents closure.
    pub fn phase_has_closure_impediment(&self, phase_ref: &StableId) -> bool {
        self.blockers.values().any(|blocker| {
            is_open_blocker(blocker)
                && (blocker.scope_kind == ScopeKind::Project
                    || (blocker.scope_kind == ScopeKind::Phase && &blocker.scope_ref == phase_ref))
        }) || self.conflicts.values().any(|conflict| {
            conflict.status == ConflictStatus::Open && conflict.severity == Severity::Critical
        })
    }

    /// Returns the current projection head used to chain a deterministic preview.
    pub const fn head_digest(&self) -> Sha256Digest {
        self.head_digest
    }

    /// Returns an authority document by stable reference.
    pub fn authority_document(
        &self,
        document_ref: &StableId,
    ) -> Option<&project_ledger_domain::AuthorityDocument> {
        self.state
            .authority_documents
            .iter()
            .find(|document| &document.document_id == document_ref)
    }

    /// Returns the latest derived form of a decision while its complete event
    /// history remains in the canonical stream.
    pub fn decision(&self, decision_ref: &StableId) -> Option<&Decision> {
        self.decisions.get(decision_ref)
    }

    /// Returns the latest derived form of a blocker.
    pub fn blocker(&self, blocker_ref: &StableId) -> Option<&Blocker> {
        self.blockers.get(blocker_ref)
    }

    /// Returns the latest derived form of a risk.
    pub fn risk(&self, risk_ref: &StableId) -> Option<&Risk> {
        self.risks.get(risk_ref)
    }

    /// Returns the latest derived form of a next action.
    pub fn next_action(&self, action_ref: &StableId) -> Option<&NextAction> {
        self.next_actions.get(action_ref)
    }

    /// Returns the latest derived form of a conflict record.
    pub fn conflict(&self, conflict_ref: &StableId) -> Option<&ConflictRecord> {
        self.conflicts.get(conflict_ref)
    }

    /// Reports whether unresolved critical authority divergence prevents a new
    /// phase from starting.
    pub fn has_open_critical_authority_conflict(&self) -> bool {
        self.conflicts.values().any(|conflict| {
            conflict.status == ConflictStatus::Open
                && conflict.severity == Severity::Critical
                && conflict.kind == ConflictKind::AuthorityDocumentDivergence
        })
    }

    /// Derives lifecycle status using the normative precedence order:
    /// archived, paused, blocked, completed, active, initializing.
    pub fn derived_lifecycle_status(&self) -> ProjectLifecycleStatus {
        if self.state.project.archived_at.is_some() {
            return ProjectLifecycleStatus::Archived;
        }
        if self.state.project.paused_reason.is_some() {
            return ProjectLifecycleStatus::Paused;
        }
        if self
            .phase_statuses
            .values()
            .any(|status| *status == PhaseStatus::Blocked)
            || self.blockers.values().any(|blocker| {
                is_open_blocker(blocker)
                    && (blocker.scope_kind == ScopeKind::Project
                        || (blocker.scope_kind == ScopeKind::Phase
                            && self.state.active_phase_refs.contains(&blocker.scope_ref)))
            })
        {
            return ProjectLifecycleStatus::Blocked;
        }
        if !self.phase_statuses.is_empty()
            && self
                .phase_statuses
                .values()
                .all(|status| matches!(status, PhaseStatus::Completed | PhaseStatus::Skipped))
        {
            return ProjectLifecycleStatus::Completed;
        }
        if self
            .phase_statuses
            .values()
            .any(|status| *status != PhaseStatus::NotStarted)
        {
            return ProjectLifecycleStatus::Active;
        }
        ProjectLifecycleStatus::Initializing
    }

    #[allow(clippy::too_many_lines)] // One exhaustive closed-event reducer keeps lifecycle audit coverage local.
    fn reduce(&mut self, event: &EventEnvelope) -> Result<(), ProjectionError> {
        match &event.event {
            Event::ProjectCreated(_) => return Err(ProjectionError::DuplicateGenesis),
            Event::ProjectMetadataUpdated(metadata) => {
                if let Some(title) = &metadata.title {
                    self.state.project.title.clone_from(title);
                }
                if let Some(summary) = &metadata.summary {
                    self.state.project.summary.clone_from(summary);
                }
            }
            Event::ProjectPaused(paused) => {
                if self.state.project.archived_at.is_some()
                    || self.state.project.paused_reason.is_some()
                {
                    return Err(invariant("only an unpaused, unarchived project can pause"));
                }
                self.state.project.paused_reason = Some(paused.reason.clone());
            }
            Event::ProjectResumed(_) => {
                if self.state.project.archived_at.is_some()
                    || self.state.project.paused_reason.is_none()
                {
                    return Err(invariant("only a paused, unarchived project can resume"));
                }
                self.state.project.paused_reason = None;
            }
            Event::ProjectArchived(_) => {
                if self.state.project.archived_at.is_some() {
                    return Err(invariant("an archived project cannot be archived twice"));
                }
                self.state.project.archived_at = Some(event.occurred_at.clone());
                self.state.project.paused_reason = None;
            }
            Event::ProjectStatusDerived(status) => {
                let expected = self.derived_lifecycle_status();
                if status.previous_status != self.state.project.lifecycle_status
                    || status.new_status != expected
                    || status.previous_status == status.new_status
                {
                    return Err(invariant(
                        "PROJECT_STATUS_DERIVED must record the exact changed derived status",
                    ));
                }
                self.state.project.lifecycle_status = status.new_status;
            }
            Event::AuthorityDocumentRegistered(registered) => {
                validate_authority_document(&registered.document)?;
                if self
                    .state
                    .authority_documents
                    .iter()
                    .any(|document| document.document_id == registered.document.document_id)
                {
                    return Err(invariant("authority document identifiers are immutable"));
                }
                self.state
                    .authority_documents
                    .push(registered.document.clone());
                sort_authority(&mut self.state.authority_documents);
            }
            Event::AuthorityDocumentDigestRefreshed(refreshed) => {
                let document =
                    authority_mut(&mut self.state.authority_documents, &refreshed.document_ref)?;
                if document.digest != refreshed.observed_digest {
                    return Err(invariant(
                        "a refreshed authority digest must equal its canonical digest",
                    ));
                }
                if document.status != AuthorityDocumentStatus::Superseded {
                    document.status = AuthorityDocumentStatus::Current;
                }
            }
            Event::AuthorityDocumentSuperseded(superseded) => {
                if superseded.document_ref == superseded.replacement_ref {
                    return Err(invariant("an authority document cannot supersede itself"));
                }
                let replacement =
                    authority(&self.state.authority_documents, &superseded.replacement_ref)?;
                if replacement.supersedes_ref.as_ref() != Some(&superseded.document_ref) {
                    return Err(invariant(
                        "replacement authority must declare the superseded reference",
                    ));
                }
                authority_mut(
                    &mut self.state.authority_documents,
                    &superseded.document_ref,
                )?
                .status = AuthorityDocumentStatus::Superseded;
                sort_authority(&mut self.state.authority_documents);
            }
            Event::AuthorityDocumentMarkedUnavailable(unavailable) => {
                let document = authority_mut(
                    &mut self.state.authority_documents,
                    &unavailable.document_ref,
                )?;
                if document.status != AuthorityDocumentStatus::Superseded {
                    document.status = AuthorityDocumentStatus::Unavailable;
                }
            }
            Event::AuthorityDocumentDigestMismatch(mismatch) => {
                let document =
                    authority_mut(&mut self.state.authority_documents, &mismatch.document_ref)?;
                if document.digest != mismatch.expected_digest
                    || mismatch.observed_digest == mismatch.expected_digest
                {
                    return Err(invariant(
                        "authority mismatch must preserve and cite the canonical digest",
                    ));
                }
                if document.status != AuthorityDocumentStatus::Superseded {
                    document.status = AuthorityDocumentStatus::DigestMismatch;
                }
            }
            Event::RepositoryBound(bound) => {
                validate_record(&bound.binding, bound.binding.digest)?;
                if bound.binding.schema_version != SchemaVersion::V0_2_0
                    || bound.binding.status != RepositoryBindingStatus::Current
                    || bound.binding.current_snapshot_ref.is_some()
                        != bound.binding.current_snapshot_digest.is_some()
                    || self
                        .repository_bindings
                        .contains_key(&bound.binding.repository_binding_id)
                    || self.repository_bindings.values().any(|binding| {
                        binding.repository_ref == bound.binding.repository_ref
                            && binding.status != RepositoryBindingStatus::Revoked
                    })
                {
                    return Err(invariant("repository binding has invalid initial state"));
                }
                self.repository_bindings.insert(
                    bound.binding.repository_binding_id.clone(),
                    bound.binding.clone(),
                );
                self.state
                    .project
                    .repository_binding_refs
                    .push(bound.binding.repository_binding_id.clone());
                self.state.project.repository_binding_refs.sort();
                self.state.project.repository_binding_refs.dedup();
            }
            Event::RepositoryBindingRefreshed(refreshed) => {
                let binding = self
                    .repository_bindings
                    .get_mut(&refreshed.binding_ref)
                    .ok_or_else(|| invariant("repository binding does not exist"))?;
                if binding.status == RepositoryBindingStatus::Revoked
                    || refreshed.snapshot.repository_ref != binding.repository_ref
                {
                    return Err(invariant(
                        "repository refresh conflicts with binding identity or status",
                    ));
                }
                binding.current_snapshot_ref = Some(refreshed.snapshot.snapshot_ref.clone());
                binding.current_snapshot_digest = Some(refreshed.snapshot.snapshot_digest);
                binding.status = RepositoryBindingStatus::Current;
                binding.updated_at = event.occurred_at.clone();
                refresh_record_digest(binding, |record, digest| record.digest = digest)?;
            }
            Event::RepositoryBindingRevoked(revoked) => {
                let binding = self
                    .repository_bindings
                    .get_mut(&revoked.binding_ref)
                    .ok_or_else(|| invariant("repository binding does not exist"))?;
                if binding.status == RepositoryBindingStatus::Revoked
                    || revoked.reason.trim().is_empty()
                {
                    return Err(invariant(
                        "repository binding is already revoked or lacks a reason",
                    ));
                }
                binding.status = RepositoryBindingStatus::Revoked;
                binding.updated_at = event.occurred_at.clone();
                refresh_record_digest(binding, |record, digest| record.digest = digest)?;
            }
            Event::RoadmapDefined(defined) => {
                if self.state.roadmap.is_some() {
                    return Err(invariant("ROADMAP_DEFINED may occur only once"));
                }
                self.install_roadmap(&defined.roadmap, false)?;
            }
            Event::RoadmapRevised(revised) => {
                self.install_roadmap(&revised.roadmap, true)?;
            }
            Event::PhaseStarted(started) => self.start_phase(&started.phase_ref)?,
            Event::PhaseBlocked(blocked) => {
                self.require_phase_status(&blocked.phase_ref, &[PhaseStatus::Active])?;
                self.phase_statuses
                    .insert(blocked.phase_ref.clone(), PhaseStatus::Blocked);
                self.sync_phase_refs();
            }
            Event::PhaseMarkedReadyToClose(ready) => {
                self.require_phase_status(&ready.phase_ref, &[PhaseStatus::Active])?;
                if !self.phase_required_gates_satisfied(&ready.phase_ref) {
                    return Err(invariant(
                        "a phase cannot be ready to close with an unsatisfied required gate",
                    ));
                }
                self.phase_statuses
                    .insert(ready.phase_ref.clone(), PhaseStatus::ReadyToClose);
                self.sync_phase_refs();
            }
            Event::PhaseCompleted(completed) => {
                self.require_phase_status(
                    &completed.phase_ref,
                    &[
                        PhaseStatus::Active,
                        PhaseStatus::ReadyToClose,
                        PhaseStatus::RevalidationRequired,
                    ],
                )?;
                self.validate_phase_gate_closure(
                    &completed.phase_ref,
                    &completed.gate_evaluation_refs,
                )?;
                self.phase_statuses
                    .insert(completed.phase_ref.clone(), PhaseStatus::Completed);
                self.sync_phase_refs();
            }
            Event::PhaseReopened(reopened) => {
                self.require_phase_status(
                    &reopened.phase_ref,
                    &[PhaseStatus::Completed, PhaseStatus::Skipped],
                )?;
                if !matches!(
                    reopened.resulting_status,
                    PhaseStatus::Active | PhaseStatus::RevalidationRequired
                ) {
                    return Err(invariant(
                        "a reopened phase must become active or require revalidation",
                    ));
                }
                self.ensure_no_other_active(&reopened.phase_ref)?;
                self.phase_statuses
                    .insert(reopened.phase_ref.clone(), reopened.resulting_status);
                self.sync_phase_refs();
            }
            Event::PhaseSkipped(skipped) => {
                self.require_phase_status(&skipped.phase_ref, &[PhaseStatus::NotStarted])?;
                authority(&self.state.authority_documents, &skipped.authority_ref)?;
                self.ensure_predecessors_complete(&skipped.phase_ref)?;
                self.phase_statuses
                    .insert(skipped.phase_ref.clone(), PhaseStatus::Skipped);
                self.sync_phase_refs();
            }
            Event::GateDefined(defined) => {
                validate_gate_definition(&defined.gate)?;
                self.require_phase_status(
                    &defined.gate.phase_ref,
                    &[
                        PhaseStatus::NotStarted,
                        PhaseStatus::Active,
                        PhaseStatus::Blocked,
                        PhaseStatus::RevalidationRequired,
                    ],
                )?;
                if self.gates.contains_key(&defined.gate.gate_id) {
                    return Err(invariant("gate identifiers and definitions are immutable"));
                }
                self.gate_statuses
                    .insert(defined.gate.gate_id.clone(), GateStatus::Pending);
                self.gates
                    .insert(defined.gate.gate_id.clone(), defined.gate.clone());
                self.sync_gate_refs();
            }
            Event::GateEvaluationRequested(requested) => {
                self.require_gate_status(
                    &requested.gate_ref,
                    &[
                        GateStatus::Pending,
                        GateStatus::Rejected,
                        GateStatus::Stale,
                        GateStatus::RevalidationRequired,
                        GateStatus::Verified,
                    ],
                )?;
                self.gate_statuses
                    .insert(requested.gate_ref.clone(), GateStatus::Evaluating);
                self.sync_gate_refs();
            }
            Event::GateEvaluated(evaluated) => {
                self.install_gate_evaluation(&evaluated.evaluation, event.stream_version)?;
            }
            Event::GateVerified(verified) => {
                self.require_gate_status(&verified.gate_ref, &[GateStatus::Evaluating])?;
                let evaluation = self
                    .gate_evaluations
                    .get(&verified.evaluation_ref)
                    .ok_or_else(|| invariant("verified gate evaluation does not exist"))?;
                if evaluation.gate_ref != verified.gate_ref
                    || evaluation.result != GateEvaluationResult::Pass
                    || evaluation.proposed_gate_status != GateStatus::Verified
                    || self.current_gate_evaluation_refs.get(&verified.gate_ref)
                        != Some(&verified.evaluation_ref)
                    || !self.evaluation_has_accepted_evidence(evaluation)
                {
                    return Err(invariant(
                        "GATE_VERIFIED requires the current valid PASS evaluation and accepted evidence",
                    ));
                }
                self.gate_statuses
                    .insert(verified.gate_ref.clone(), GateStatus::Verified);
                self.sync_gate_refs();
            }
            Event::GateRejected(rejected) => {
                self.require_gate_status(&rejected.gate_ref, &[GateStatus::Evaluating])?;
                let evaluation = self
                    .gate_evaluations
                    .get(&rejected.evaluation_ref)
                    .ok_or_else(|| invariant("rejected gate evaluation does not exist"))?;
                if evaluation.gate_ref != rejected.gate_ref
                    || evaluation.result != GateEvaluationResult::Fail
                    || evaluation.proposed_gate_status != GateStatus::Rejected
                    || self.current_gate_evaluation_refs.get(&rejected.gate_ref)
                        != Some(&rejected.evaluation_ref)
                {
                    return Err(invariant(
                        "GATE_REJECTED requires the current valid FAIL evaluation",
                    ));
                }
                self.gate_statuses
                    .insert(rejected.gate_ref.clone(), GateStatus::Rejected);
                self.sync_gate_refs();
            }
            Event::GateWaived(waived) => {
                validate_record(&waived.waiver, waived.waiver.digest)?;
                self.require_gate_status(
                    &waived.waiver.gate_ref,
                    &[
                        GateStatus::Pending,
                        GateStatus::Rejected,
                        GateStatus::Stale,
                        GateStatus::RevalidationRequired,
                    ],
                )?;
                let gate = self
                    .gates
                    .get(&waived.waiver.gate_ref)
                    .ok_or_else(|| invariant("waived gate does not exist"))?;
                if gate.waiver_policy == project_ledger_domain::GateWaiverPolicy::Forbidden
                    || waived.waiver.schema_version != SchemaVersion::V0_2_0
                    || waived.waiver.gate_definition_version != gate.definition_version
                    || waived.waiver.gate_definition_digest != gate.digest
                    || waived.waiver.reason.trim().is_empty()
                    || waived.waiver.scope.trim().is_empty()
                    || waived.waiver.review_condition.trim().is_empty()
                    || self.gate_waivers.contains_key(&gate.gate_id)
                    || authority(
                        &self.state.authority_documents,
                        &waived.waiver.authority_ref,
                    )?
                    .status
                        != AuthorityDocumentStatus::Current
                {
                    return Err(invariant("gate waiver violates policy or provenance"));
                }
                self.gate_waivers
                    .insert(gate.gate_id.clone(), waived.waiver.clone());
                self.gate_statuses
                    .insert(gate.gate_id.clone(), GateStatus::Waived);
                self.sync_gate_refs();
            }
            Event::GateMarkedStale(stale) => {
                self.require_gate_status(
                    &stale.gate_ref,
                    &[GateStatus::Verified, GateStatus::Waived],
                )?;
                if stale.reason_codes.is_empty() {
                    return Err(invariant("a stale gate requires reason codes"));
                }
                self.gate_statuses
                    .insert(stale.gate_ref.clone(), GateStatus::Stale);
                self.sync_gate_refs();
            }
            Event::GateRevalidationRequested(requested) => {
                self.require_gate_status(
                    &requested.gate_ref,
                    &[
                        GateStatus::Pending,
                        GateStatus::Rejected,
                        GateStatus::Verified,
                        GateStatus::Stale,
                        GateStatus::RevalidationRequired,
                    ],
                )?;
                self.gate_statuses
                    .insert(requested.gate_ref.clone(), GateStatus::Evaluating);
                self.sync_gate_refs();
            }
            Event::GateRevalidationRequired(required) => {
                self.require_gate_status(&required.gate_ref, &[required.prior_status])?;
                if required.reason_codes.is_empty() {
                    return Err(invariant("gate revalidation requires reason codes"));
                }
                self.gate_statuses
                    .insert(required.gate_ref.clone(), GateStatus::RevalidationRequired);
                self.sync_gate_refs();
            }
            Event::WorkProgressRecorded(recorded) => {
                validate_record(&recorded.work_record, recorded.work_record.digest)?;
                if recorded.work_record.schema_version != SchemaVersion::V0_2_0
                    || recorded.work_record.summary.trim().is_empty()
                    || recorded.work_record.actor_ref != event.actor.actor_id
                    || recorded.work_record.recorded_at != event.recorded_at
                    || self
                        .work_records
                        .contains_key(&recorded.work_record.work_record_id)
                    || recorded
                        .work_record
                        .phase_ref
                        .as_ref()
                        .is_some_and(|phase_ref| !self.phase_statuses.contains_key(phase_ref))
                    || recorded
                        .work_record
                        .claim_refs
                        .iter()
                        .any(|claim_ref| !self.claims.contains_key(claim_ref))
                    || recorded
                        .work_record
                        .evidence_refs
                        .iter()
                        .any(|evidence_ref| !self.evidence.contains_key(evidence_ref))
                {
                    return Err(invariant(
                        "work progress has invalid identity or references",
                    ));
                }
                self.latest_work_record_ref = Some(recorded.work_record.work_record_id.clone());
                self.work_records.insert(
                    recorded.work_record.work_record_id.clone(),
                    recorded.work_record.clone(),
                );
            }
            Event::UnknownRecorded(recorded) => {
                validate_record(&recorded.unknown, recorded.unknown.digest)?;
                if recorded.unknown.schema_version != SchemaVersion::V0_2_0
                    || recorded.unknown.status != UnknownStatus::Open
                    || recorded.unknown.statement.trim().is_empty()
                    || recorded.unknown.subject_kind.trim().is_empty()
                    || recorded.unknown.field_path.trim().is_empty()
                    || recorded.unknown.resolved_at.is_some()
                    || recorded.unknown.resolution_ref.is_some()
                    || self.unknowns.contains_key(&recorded.unknown.unknown_id)
                    || recorded
                        .unknown
                        .evidence_refs
                        .iter()
                        .any(|evidence_ref| !self.evidence.contains_key(evidence_ref))
                {
                    return Err(invariant("unknown record has invalid initial state"));
                }
                self.unknowns.insert(
                    recorded.unknown.unknown_id.clone(),
                    recorded.unknown.clone(),
                );
            }
            Event::UnknownResolved(resolved) => {
                let unknown = self
                    .unknowns
                    .get_mut(&resolved.unknown_ref)
                    .ok_or_else(|| invariant("resolved unknown does not exist"))?;
                if unknown.status != UnknownStatus::Open {
                    return Err(invariant("unknown is already terminal"));
                }
                unknown.status = UnknownStatus::Resolved;
                unknown.resolved_at = Some(event.occurred_at.clone());
                unknown.resolution_ref = Some(resolved.resolution_ref.clone());
                refresh_record_digest(unknown, |record, digest| record.digest = digest)?;
            }
            Event::VerificationRecorded(recorded) => {
                validate_record(&recorded.evidence, recorded.evidence.payload_digest)?;
                if recorded.evidence.schema_version != SchemaVersion::V0_2_0
                    || recorded.evidence.project_ref != self.state.project.project_id
                    || matches!(
                        recorded.evidence.status,
                        EvidenceStatus::Stale | EvidenceStatus::Invalidated
                    )
                    || recorded.evidence.source.tool.trim().is_empty()
                    || recorded.evidence.source.runner_version.trim().is_empty()
                    || recorded.evidence.result.summary.trim().is_empty()
                    || self.evidence.contains_key(&recorded.evidence.evidence_id)
                    || self
                        .external_evidence_refs
                        .contains(&recorded.evidence.evidence_id)
                {
                    return Err(invariant(
                        "evidence record has invalid provenance or initial state",
                    ));
                }
                self.evidence.insert(
                    recorded.evidence.evidence_id.clone(),
                    recorded.evidence.clone(),
                );
            }
            Event::ExternalEvidenceLinked(linked) => {
                if self.evidence.contains_key(&linked.evidence_ref)
                    || !self
                        .external_evidence_refs
                        .insert(linked.evidence_ref.clone())
                {
                    return Err(invariant("external evidence identifier already exists"));
                }
            }
            Event::ExternalEvidenceUnlinked(unlinked) => {
                if !self.external_evidence_refs.remove(&unlinked.evidence_ref) {
                    return Err(invariant("external evidence reference does not exist"));
                }
            }
            Event::EvidenceInvalidated(invalidated) => {
                let evidence = self
                    .evidence
                    .get_mut(&invalidated.evidence_ref)
                    .ok_or_else(|| invariant("invalidated evidence does not exist"))?;
                if evidence.status == EvidenceStatus::Invalidated
                    || invalidated.reason.trim().is_empty()
                {
                    return Err(invariant(
                        "evidence is already invalidated or lacks a reason",
                    ));
                }
                evidence.status = EvidenceStatus::Invalidated;
                refresh_record_digest(evidence, |record, digest| {
                    record.payload_digest = digest;
                })?;
            }
            Event::ClaimRecorded(recorded) => {
                if recorded.claim.schema_version != SchemaVersion::V0_2_0
                    || recorded.claim.classification != ClaimClassification::Asserted
                    || recorded.claim.statement.trim().is_empty()
                    || recorded.claim.subject_refs.is_empty()
                    || !recorded.claim.evidence_refs.is_empty()
                    || recorded.claim.authority_ref.is_some()
                    || self.claims.contains_key(&recorded.claim.claim_id)
                {
                    return Err(invariant(
                        "a newly recorded manual/agent claim must be an unevidenced ASSERTED claim",
                    ));
                }
                self.claims
                    .insert(recorded.claim.claim_id.clone(), recorded.claim.clone());
            }
            Event::ClaimStatusChanged(changed) => {
                let claim = self
                    .claims
                    .get_mut(&changed.claim_ref)
                    .ok_or_else(|| invariant("claim reference does not exist"))?;
                if claim.classification != changed.prior_status
                    || changed.new_status == changed.prior_status
                    || changed.new_status == ClaimClassification::Asserted
                {
                    return Err(invariant("claim classification transition is invalid"));
                }
                let evidence_is_accepted = |evidence_ref: &StableId| {
                    self.evidence
                        .get(evidence_ref)
                        .is_some_and(|evidence| evidence.status == EvidenceStatus::VerifiedSource)
                };
                if changed.new_status == ClaimClassification::Verified
                    && (changed.evidence_refs.is_empty()
                        || !changed.evidence_refs.iter().all(evidence_is_accepted))
                {
                    return Err(invariant(
                        "a VERIFIED claim requires accepted verified-source evidence",
                    ));
                }
                if changed.new_status == ClaimClassification::Observed
                    && changed.evidence_refs.is_empty()
                {
                    return Err(invariant("an OBSERVED claim requires evidence"));
                }
                claim.classification = changed.new_status;
                merge_stable_refs(&mut claim.evidence_refs, &changed.evidence_refs);
            }
            Event::DecisionProposed(proposed) => {
                validate_record(&proposed.decision, proposed.decision.digest)?;
                if proposed.decision.schema_version != SchemaVersion::V0_2_0
                    || proposed.decision.status != DecisionStatus::Proposed
                    || proposed.decision.authority_ref.is_some()
                    || proposed.decision.accepted_at.is_some()
                    || self.decisions.contains_key(&proposed.decision.decision_id)
                {
                    return Err(invariant("a proposed decision has invalid initial state"));
                }
                match &proposed.decision.supersedes_ref {
                    Some(prior_ref) => {
                        let prior = self
                            .decisions
                            .get(prior_ref)
                            .ok_or_else(|| invariant("decision predecessor does not exist"))?;
                        if proposed.decision.version != prior.version.saturating_add(1)
                            || proposed.decision.domain != prior.domain
                            || proposed.decision.scope != prior.scope
                        {
                            return Err(invariant(
                                "a replacement decision must increment its predecessor in the same domain and scope",
                            ));
                        }
                    }
                    None if proposed.decision.version != 1 => {
                        return Err(invariant("an initial decision version must be 1"));
                    }
                    None => {}
                }
                self.decisions.insert(
                    proposed.decision.decision_id.clone(),
                    proposed.decision.clone(),
                );
                self.sync_decision_refs();
            }
            Event::DecisionAccepted(accepted) => {
                let authority =
                    authority(&self.state.authority_documents, &accepted.authority_ref)?;
                if authority.status != AuthorityDocumentStatus::Current {
                    return Err(invariant("decision authority must be current"));
                }
                let decision = self
                    .decisions
                    .get_mut(&accepted.decision_ref)
                    .ok_or_else(|| invariant("decision reference does not exist"))?;
                if !matches!(
                    decision.status,
                    DecisionStatus::Proposed | DecisionStatus::RevalidationRequired
                ) {
                    return Err(invariant(
                        "only a proposed or revalidation-required decision can be accepted",
                    ));
                }
                if decision.status == DecisionStatus::RevalidationRequired
                    && (event.actor.actor_kind != project_ledger_domain::ActorKind::Human
                        || event.actor.trust_level != project_ledger_domain::TrustLevel::LocalUser)
                {
                    return Err(invariant("decision reacceptance requires a local human"));
                }
                decision.status = DecisionStatus::Accepted;
                decision.authority_ref = Some(accepted.authority_ref.clone());
                decision.accepted_at = Some(event.occurred_at.clone());
                refresh_record_digest(decision, |record, digest| record.digest = digest)?;
                self.sync_decision_refs();
            }
            Event::DecisionSuperseded(superseded) => {
                if superseded.decision_ref == superseded.replacement_ref {
                    return Err(invariant("a decision cannot supersede itself"));
                }
                let replacement = self
                    .decisions
                    .get(&superseded.replacement_ref)
                    .ok_or_else(|| invariant("replacement decision must already exist"))?;
                if replacement.status != DecisionStatus::Accepted
                    || replacement.supersedes_ref.as_ref() != Some(&superseded.decision_ref)
                {
                    return Err(invariant(
                        "replacement decision must be accepted and cite its predecessor",
                    ));
                }
                let decision = self
                    .decisions
                    .get_mut(&superseded.decision_ref)
                    .ok_or_else(|| invariant("superseded decision does not exist"))?;
                if decision.status != DecisionStatus::Accepted {
                    return Err(invariant("only an accepted decision can be superseded"));
                }
                decision.status = DecisionStatus::Superseded;
                refresh_record_digest(decision, |record, digest| record.digest = digest)?;
                self.sync_decision_refs();
            }
            Event::DecisionRejected(rejected) => {
                let decision = self
                    .decisions
                    .get_mut(&rejected.decision_ref)
                    .ok_or_else(|| invariant("rejected decision does not exist"))?;
                if decision.status != DecisionStatus::Proposed {
                    return Err(invariant("only a proposed decision can be rejected"));
                }
                decision.status = DecisionStatus::Rejected;
                refresh_record_digest(decision, |record, digest| record.digest = digest)?;
                self.sync_decision_refs();
            }
            Event::DecisionRevalidationRequired(required) => {
                let decision = self
                    .decisions
                    .get_mut(&required.decision_ref)
                    .ok_or_else(|| invariant("decision requiring revalidation does not exist"))?;
                if decision.status != DecisionStatus::Accepted {
                    return Err(invariant(
                        "only an accepted decision can require revalidation",
                    ));
                }
                decision.status = DecisionStatus::RevalidationRequired;
                refresh_record_digest(decision, |record, digest| record.digest = digest)?;
                self.sync_decision_refs();
            }
            Event::BlockerOpened(opened) => {
                validate_record(&opened.blocker, opened.blocker.digest)?;
                if opened.blocker.schema_version != SchemaVersion::V0_2_0
                    || opened.blocker.status != BlockerStatus::Open
                    || opened.blocker.resolved_at.is_some()
                    || opened.blocker.resolution_summary.is_some()
                    || self.blockers.contains_key(&opened.blocker.blocker_id)
                {
                    return Err(invariant("a blocker has invalid initial state"));
                }
                self.blockers
                    .insert(opened.blocker.blocker_id.clone(), opened.blocker.clone());
                self.sync_blocker_refs();
            }
            Event::BlockerUpdated(updated) => {
                validate_record(&updated.blocker, updated.blocker.digest)?;
                let prior = self
                    .blockers
                    .get(&updated.blocker.blocker_id)
                    .ok_or_else(|| invariant("updated blocker does not exist"))?;
                if updated.blocker.schema_version != SchemaVersion::V0_2_0
                    || updated.blocker.opened_at != prior.opened_at
                    || updated.blocker.status == BlockerStatus::Resolved
                    || prior.status == BlockerStatus::Resolved
                {
                    return Err(invariant("blocker update violates immutable history"));
                }
                self.blockers
                    .insert(updated.blocker.blocker_id.clone(), updated.blocker.clone());
                self.sync_blocker_refs();
            }
            Event::BlockerResolved(resolved) => {
                let blocker = self
                    .blockers
                    .get_mut(&resolved.blocker_ref)
                    .ok_or_else(|| invariant("resolved blocker does not exist"))?;
                if !is_open_blocker(blocker) {
                    return Err(invariant("blocker is already terminal"));
                }
                blocker.status = BlockerStatus::Resolved;
                blocker.updated_at = event.occurred_at.clone();
                blocker.resolved_at = Some(event.occurred_at.clone());
                blocker.resolution_summary = Some(resolved.resolution_summary.clone());
                merge_stable_refs(&mut blocker.evidence_refs, &resolved.evidence_refs);
                refresh_record_digest(blocker, |record, digest| record.digest = digest)?;
                let phase_ref =
                    (blocker.scope_kind == ScopeKind::Phase).then(|| blocker.scope_ref.clone());
                self.sync_blocker_refs();
                if let Some(phase_ref) = phase_ref
                    && self.phase_statuses.get(&phase_ref) == Some(&PhaseStatus::Blocked)
                    && !self.blockers.values().any(|candidate| {
                        is_open_blocker(candidate)
                            && candidate.scope_kind == ScopeKind::Phase
                            && candidate.scope_ref == phase_ref
                    })
                {
                    self.phase_statuses.insert(phase_ref, PhaseStatus::Active);
                    self.sync_phase_refs();
                }
            }
            Event::RiskOpened(opened) => {
                validate_record(&opened.risk, opened.risk.digest)?;
                if opened.risk.schema_version != SchemaVersion::V0_2_0
                    || opened.risk.status != RiskStatus::Open
                    || opened.risk.closed_at.is_some()
                    || self.risks.contains_key(&opened.risk.risk_id)
                {
                    return Err(invariant("a risk has invalid initial state"));
                }
                self.risks
                    .insert(opened.risk.risk_id.clone(), opened.risk.clone());
                self.sync_risk_refs();
            }
            Event::RiskUpdated(updated) => {
                validate_record(&updated.risk, updated.risk.digest)?;
                let prior = self
                    .risks
                    .get(&updated.risk.risk_id)
                    .ok_or_else(|| invariant("updated risk does not exist"))?;
                if updated.risk.schema_version != SchemaVersion::V0_2_0
                    || updated.risk.opened_at != prior.opened_at
                    || updated.risk.status == RiskStatus::Closed
                    || prior.status == RiskStatus::Closed
                {
                    return Err(invariant("risk update violates immutable history"));
                }
                self.risks
                    .insert(updated.risk.risk_id.clone(), updated.risk.clone());
                self.sync_risk_refs();
            }
            Event::RiskMitigated(mitigated) => {
                let risk = self
                    .risks
                    .get_mut(&mitigated.risk_ref)
                    .ok_or_else(|| invariant("mitigated risk does not exist"))?;
                if risk.status == RiskStatus::Closed {
                    return Err(invariant("a closed risk cannot be mitigated"));
                }
                risk.status = RiskStatus::Mitigated;
                risk.updated_at = event.occurred_at.clone();
                risk.mitigation_actions.push(mitigated.note.clone());
                merge_stable_refs(&mut risk.evidence_refs, &mitigated.evidence_refs);
                refresh_record_digest(risk, |record, digest| record.digest = digest)?;
                self.sync_risk_refs();
            }
            Event::RiskClosed(closed) => {
                let risk = self
                    .risks
                    .get_mut(&closed.risk_ref)
                    .ok_or_else(|| invariant("closed risk does not exist"))?;
                if risk.status == RiskStatus::Closed {
                    return Err(invariant("risk is already closed"));
                }
                risk.status = RiskStatus::Closed;
                risk.updated_at = event.occurred_at.clone();
                risk.closed_at = Some(event.occurred_at.clone());
                risk.mitigation_actions.push(closed.note.clone());
                merge_stable_refs(&mut risk.evidence_refs, &closed.evidence_refs);
                refresh_record_digest(risk, |record, digest| record.digest = digest)?;
                self.sync_risk_refs();
            }
            Event::NextActionSet(set) => {
                validate_record(&set.action, set.action.digest)?;
                validate_next_action(&self.state, &set.action)?;
                if self.next_actions.contains_key(&set.action.next_action_id) {
                    return Err(invariant("next action identifiers are immutable"));
                }
                self.next_actions
                    .insert(set.action.next_action_id.clone(), set.action.clone());
                self.sync_action_refs();
            }
            Event::NextActionCompleted(completed) => {
                let action = self
                    .next_actions
                    .get_mut(&completed.action_ref)
                    .ok_or_else(|| invariant("completed next action does not exist"))?;
                if matches!(
                    action.classification,
                    NextActionClassification::Completed | NextActionClassification::Cancelled
                ) {
                    return Err(invariant("next action is already terminal"));
                }
                action.classification = NextActionClassification::Completed;
                action.updated_at = event.occurred_at.clone();
                action.completed_at = Some(event.occurred_at.clone());
                refresh_record_digest(action, |record, digest| record.digest = digest)?;
                self.sync_action_refs();
            }
            Event::NextActionCancelled(cancelled) => {
                let action = self
                    .next_actions
                    .get_mut(&cancelled.action_ref)
                    .ok_or_else(|| invariant("cancelled next action does not exist"))?;
                if matches!(
                    action.classification,
                    NextActionClassification::Completed | NextActionClassification::Cancelled
                ) {
                    return Err(invariant("next action is already terminal"));
                }
                action.classification = NextActionClassification::Cancelled;
                action.updated_at = event.occurred_at.clone();
                refresh_record_digest(action, |record, digest| record.digest = digest)?;
                self.sync_action_refs();
            }
            Event::CheckpointCreated(created) => {
                self.install_checkpoint(&created.checkpoint, event)?;
            }
            Event::CheckpointInvalidated(invalidated) => {
                if !self.checkpoints.contains_key(&invalidated.checkpoint_ref)
                    || invalidated.reason_codes.is_empty()
                    || matches!(
                        invalidated.freshness,
                        CheckpointFreshnessStatus::Current
                            | CheckpointFreshnessStatus::CurrentForProjectOnly
                            | CheckpointFreshnessStatus::SupersededByNewerCheckpoint
                    )
                    || self.checkpoint_freshness(&invalidated.checkpoint_ref)
                        == Some(CheckpointFreshnessStatus::SupersededByNewerCheckpoint)
                {
                    return Err(invariant(
                        "checkpoint invalidation is not a valid freshness transition",
                    ));
                }
                self.append_checkpoint_freshness(
                    &invalidated.checkpoint_ref,
                    invalidated.freshness,
                    invalidated.reason_codes.clone(),
                    event,
                    "checkpoint-freshness",
                )?;
            }
            Event::ConflictOpened(opened) => {
                if opened.conflict.schema_version != SchemaVersion::V0_2_0
                    || opened.conflict.status != ConflictStatus::Open
                    || opened.conflict.resolved_at.is_some()
                    || opened.conflict.resolution_ref.is_some()
                    || self.conflicts.contains_key(&opened.conflict.conflict_id)
                {
                    return Err(invariant("a conflict has invalid initial state"));
                }
                self.conflicts
                    .insert(opened.conflict.conflict_id.clone(), opened.conflict.clone());
                self.sync_conflict_refs();
            }
            Event::ConflictResolved(resolved) => {
                let conflict = self
                    .conflicts
                    .get_mut(&resolved.conflict_ref)
                    .ok_or_else(|| invariant("resolved conflict does not exist"))?;
                if conflict.status != ConflictStatus::Open {
                    return Err(invariant("conflict is already terminal"));
                }
                conflict.status = ConflictStatus::Resolved;
                conflict.resolved_at = Some(event.occurred_at.clone());
                conflict.resolution_ref = Some(resolved.resolution_ref.clone());
                self.sync_conflict_refs();
            }
            _ => self.reduce_auxiliary(event)?,
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // Exact field-by-field N/N+1 validation remains auditable in one place.
    fn install_checkpoint(
        &mut self,
        checkpoint: &ContinuityCheckpoint,
        event: &EventEnvelope,
    ) -> Result<(), ProjectionError> {
        validate_record(checkpoint, checkpoint.checkpoint_digest)?;
        checkpoint
            .validate()
            .map_err(|_| invariant("checkpoint violates the N/N+1 contract"))?;
        let repository_binding = self.expected_checkpoint_repository_snapshot()?;
        let authority_document_refs = self
            .state
            .authority_documents
            .iter()
            .map(|document| document.document_id.clone())
            .collect::<Vec<_>>();
        let verified_completed_phase_refs = self
            .state
            .completed_phase_refs
            .iter()
            .filter(|phase_ref| {
                self.gates_for_phase(phase_ref).into_iter().all(|gate| {
                    gate.criticality != GateCriticality::Required
                        || self.gate_status(&gate.gate_id) == Some(GateStatus::Verified)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let unknown_refs = self
            .unknowns
            .values()
            .filter(|unknown| unknown.status == UnknownStatus::Open)
            .map(|unknown| unknown.unknown_id.clone())
            .collect::<Vec<_>>();
        let last_material_event_refs = self
            .state
            .last_material_event_ref
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if checkpoint.schema_version != SchemaVersion::V0_2_0
            || checkpoint.project_ref != self.state.project.project_id
            || checkpoint.captured_state_version != self.state.state_version
            || checkpoint.captured_material_state_version != self.state.material_state_version
            || checkpoint.captured_state_digest != self.state.state_digest
            || checkpoint.checkpoint_event_version != event.stream_version
            || checkpoint.checkpoint_event_ref != event.event_id
            || checkpoint.created_by != event.actor.actor_id
            || checkpoint.created_at != event.occurred_at
            || checkpoint.authority_document_refs != authority_document_refs
            || checkpoint.active_phase_refs != self.state.active_phase_refs
            || checkpoint.primary_active_phase_ref != self.state.primary_active_phase_ref
            || checkpoint.verified_completed_phase_refs != verified_completed_phase_refs
            || checkpoint.pending_gate_refs != self.state.pending_gate_refs
            || checkpoint.current_decision_refs != self.state.current_decision_refs
            || checkpoint.open_blocker_refs != self.state.open_blocker_refs
            || checkpoint.open_risk_refs != self.state.open_risk_refs
            || checkpoint.next_action_refs != self.state.ordered_next_action_refs
            || checkpoint.last_material_event_refs != last_material_event_refs
            || checkpoint.repository_binding != repository_binding
            || checkpoint.unknown_refs != unknown_refs
            || checkpoint.conflict_refs != self.state.open_conflict_refs
            || self.checkpoints.contains_key(&checkpoint.checkpoint_id)
        {
            return Err(invariant(
                "checkpoint does not exactly capture the pre-event project state",
            ));
        }

        if let Some(prior_ref) = self.state.checkpoint_ref.clone() {
            self.append_checkpoint_freshness(
                &prior_ref,
                CheckpointFreshnessStatus::SupersededByNewerCheckpoint,
                vec!["NEWER_CHECKPOINT_CREATED".to_owned()],
                event,
                "checkpoint-freshness-superseded",
            )?;
        }
        let initial_status = self.initial_checkpoint_freshness()?;
        let reason_codes = match initial_status {
            CheckpointFreshnessStatus::Current => vec!["CURRENT_REPOSITORY_SNAPSHOT".to_owned()],
            CheckpointFreshnessStatus::CurrentForProjectOnly => {
                vec!["NO_REPOSITORY_SNAPSHOT_ASSERTED".to_owned()]
            }
            CheckpointFreshnessStatus::StaleForRepository => {
                vec!["REPOSITORY_SNAPSHOT_UNAVAILABLE".to_owned()]
            }
            CheckpointFreshnessStatus::StaleForAuthority => {
                vec!["AUTHORITY_NOT_CURRENT".to_owned()]
            }
            CheckpointFreshnessStatus::Conflicted => vec!["CRITICAL_CONFLICT_OPEN".to_owned()],
            _ => {
                return Err(invariant(
                    "checkpoint cannot start in a terminal freshness state",
                ));
            }
        };
        self.checkpoints
            .insert(checkpoint.checkpoint_id.clone(), checkpoint.clone());
        self.state.checkpoint_ref = Some(checkpoint.checkpoint_id.clone());
        self.append_checkpoint_freshness(
            &checkpoint.checkpoint_id,
            initial_status,
            reason_codes,
            event,
            "checkpoint-freshness",
        )?;
        Ok(())
    }

    fn expected_checkpoint_repository_snapshot(
        &self,
    ) -> Result<Option<RepositorySnapshot>, ProjectionError> {
        let current = self
            .repository_bindings
            .values()
            .filter(|binding| binding.status == RepositoryBindingStatus::Current)
            .collect::<Vec<_>>();
        if current.len() > 1 {
            return Err(invariant(
                "checkpoint cannot select implicitly between multiple current repositories",
            ));
        }
        let Some(binding) = current.first() else {
            return Ok(None);
        };
        match (
            &binding.current_snapshot_ref,
            binding.current_snapshot_digest,
        ) {
            (Some(snapshot_ref), Some(snapshot_digest)) => Ok(Some(RepositorySnapshot {
                repository_ref: binding.repository_ref.clone(),
                snapshot_ref: snapshot_ref.clone(),
                snapshot_digest,
            })),
            (None, None) => Ok(None),
            _ => Err(invariant(
                "repository binding snapshot reference and digest must agree",
            )),
        }
    }

    fn initial_checkpoint_freshness(&self) -> Result<CheckpointFreshnessStatus, ProjectionError> {
        if self.conflicts.values().any(|conflict| {
            conflict.status == ConflictStatus::Open && conflict.severity == Severity::Critical
        }) {
            return Ok(CheckpointFreshnessStatus::Conflicted);
        }
        if self
            .state
            .authority_documents
            .iter()
            .any(|document| document.status != AuthorityDocumentStatus::Current)
        {
            return Ok(CheckpointFreshnessStatus::StaleForAuthority);
        }
        let current = self
            .repository_bindings
            .values()
            .filter(|binding| binding.status == RepositoryBindingStatus::Current)
            .collect::<Vec<_>>();
        match current.as_slice() {
            [] => Ok(CheckpointFreshnessStatus::CurrentForProjectOnly),
            [binding]
                if binding.current_snapshot_ref.is_some()
                    && binding.current_snapshot_digest.is_some() =>
            {
                Ok(CheckpointFreshnessStatus::Current)
            }
            [_] => Ok(CheckpointFreshnessStatus::StaleForRepository),
            _ => Err(invariant(
                "checkpoint freshness cannot choose among current repositories",
            )),
        }
    }

    fn append_checkpoint_freshness(
        &mut self,
        checkpoint_ref: &StableId,
        status: CheckpointFreshnessStatus,
        reason_codes: Vec<String>,
        event: &EventEnvelope,
        id_prefix: &str,
    ) -> Result<(), ProjectionError> {
        let freshness_record_id =
            StableId::parse(&format!("{id_prefix}:{}", event.event_id.uuid()))
                .map_err(|_| invariant("checkpoint freshness identity is invalid"))?;
        let mut record = CheckpointFreshnessRecord {
            schema_version: SchemaVersion::V0_2_0,
            freshness_record_id,
            checkpoint_ref: checkpoint_ref.clone(),
            status,
            evaluated_at: event.occurred_at.clone(),
            reason_codes,
            digest: Sha256Digest::from([0_u8; 32]),
        };
        refresh_record_digest(&mut record, |value, digest| value.digest = digest)?;
        self.checkpoint_freshness
            .insert(checkpoint_ref.clone(), status);
        self.checkpoint_freshness_records.push(record);
        Ok(())
    }

    fn install_roadmap(
        &mut self,
        roadmap: &Roadmap,
        revision: bool,
    ) -> Result<(), ProjectionError> {
        validate_roadmap(roadmap)?;
        let source = authority(
            &self.state.authority_documents,
            &roadmap.source_document_ref,
        )?;
        if source.status != AuthorityDocumentStatus::Current {
            return Err(invariant("roadmap authority source must be current"));
        }

        if revision {
            let prior = self
                .state
                .roadmap
                .as_ref()
                .ok_or_else(|| invariant("ROADMAP_REVISED requires an existing roadmap"))?;
            if prior.roadmap_id != roadmap.roadmap_id
                || roadmap.version != prior.version.saturating_add(1)
            {
                return Err(invariant(
                    "roadmap revisions retain identity and increment version exactly once",
                ));
            }
            for (phase_ref, status) in &self.phase_statuses {
                if !roadmap.phase_order.contains(phase_ref) && *status != PhaseStatus::NotStarted {
                    return Err(invariant(
                        "a roadmap revision cannot remove a phase with lifecycle history",
                    ));
                }
            }
        } else if roadmap.version != 1 {
            return Err(invariant("the first roadmap version must be 1"));
        }

        let mut statuses = BTreeMap::new();
        for phase_ref in &roadmap.phase_order {
            statuses.insert(
                phase_ref.clone(),
                self.phase_statuses
                    .get(phase_ref)
                    .copied()
                    .unwrap_or(PhaseStatus::NotStarted),
            );
        }
        self.phase_statuses = statuses;
        self.state.roadmap = Some(roadmap.clone());
        self.sync_phase_refs();
        Ok(())
    }

    fn start_phase(&mut self, phase_ref: &StableId) -> Result<(), ProjectionError> {
        self.require_phase_status(phase_ref, &[PhaseStatus::NotStarted])?;
        self.ensure_no_other_active(phase_ref)?;
        self.ensure_predecessors_complete(phase_ref)?;
        self.phase_statuses
            .insert(phase_ref.clone(), PhaseStatus::Active);
        self.sync_phase_refs();
        Ok(())
    }

    fn require_phase_status(
        &self,
        phase_ref: &StableId,
        expected: &[PhaseStatus],
    ) -> Result<(), ProjectionError> {
        let actual = self
            .phase_statuses
            .get(phase_ref)
            .ok_or_else(|| invariant("phase is not present in the current roadmap"))?;
        if !expected.contains(actual) {
            return Err(invariant(
                "phase transition is invalid from its current status",
            ));
        }
        Ok(())
    }

    fn ensure_no_other_active(&self, phase_ref: &StableId) -> Result<(), ProjectionError> {
        if self.phase_statuses.iter().any(|(candidate, status)| {
            candidate != phase_ref
                && matches!(
                    status,
                    PhaseStatus::Active
                        | PhaseStatus::Blocked
                        | PhaseStatus::ReadyToClose
                        | PhaseStatus::RevalidationRequired
                )
        }) {
            return Err(invariant(
                "a sequential roadmap permits at most one active phase",
            ));
        }
        Ok(())
    }

    fn ensure_predecessors_complete(&self, phase_ref: &StableId) -> Result<(), ProjectionError> {
        let roadmap = self
            .state
            .roadmap
            .as_ref()
            .ok_or_else(|| invariant("phase transitions require a roadmap"))?;
        let index = roadmap
            .phase_order
            .iter()
            .position(|candidate| candidate == phase_ref)
            .ok_or_else(|| invariant("phase is not present in the current roadmap"))?;
        if roadmap.phase_order[..index].iter().any(|predecessor| {
            !matches!(
                self.phase_statuses.get(predecessor),
                Some(PhaseStatus::Completed | PhaseStatus::Skipped)
            )
        }) {
            return Err(invariant(
                "all preceding phases must be completed or skipped",
            ));
        }
        Ok(())
    }

    fn require_gate_status(
        &self,
        gate_ref: &StableId,
        expected: &[GateStatus],
    ) -> Result<(), ProjectionError> {
        let actual = self
            .gate_statuses
            .get(gate_ref)
            .ok_or_else(|| invariant("gate reference does not exist"))?;
        if !expected.contains(actual) {
            return Err(invariant(
                "gate transition is invalid from its current status",
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // Keeping the closed evaluation audit in one validator avoids split invariants.
    fn install_gate_evaluation(
        &mut self,
        evaluation: &GateEvaluation,
        event_version: u64,
    ) -> Result<(), ProjectionError> {
        validate_record(evaluation, evaluation.digest)?;
        self.require_gate_status(&evaluation.gate_ref, &[GateStatus::Evaluating])?;
        let gate = self
            .gates
            .get(&evaluation.gate_ref)
            .ok_or_else(|| invariant("evaluated gate does not exist"))?;
        if evaluation.schema_version != SchemaVersion::V0_2_0
            || evaluation.gate_definition_version != gate.definition_version
            || evaluation.gate_definition_digest != gate.digest
            || evaluation.project_state_version != event_version.saturating_sub(2)
            || self
                .gate_evaluations
                .contains_key(&evaluation.gate_evaluation_id)
            || evaluation.rule_results.len() != gate.required_evidence_rules.len()
            || evaluation.evaluated_by.engine != "project-ledger-gate-engine"
            || evaluation.evaluated_by.version.trim().is_empty()
        {
            return Err(invariant(
                "gate evaluation does not match its definition or evaluated state",
            ));
        }
        let expected_rule_refs = gate
            .required_evidence_rules
            .iter()
            .map(|rule| &rule.rule_id)
            .collect::<BTreeSet<_>>();
        let actual_rule_refs = evaluation
            .rule_results
            .iter()
            .map(|rule| &rule.rule_id)
            .collect::<BTreeSet<_>>();
        if expected_rule_refs != actual_rule_refs
            || !evaluation_result_matches_rules(evaluation, gate.completion_rule)
            || evaluation
                .selected_evidence_refs
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != evaluation.selected_evidence_refs.len()
            || evaluation
                .selected_evidence_refs
                .iter()
                .any(|evidence_ref| {
                    !self
                        .evidence
                        .get(evidence_ref)
                        .is_some_and(|evidence| evidence.status == EvidenceStatus::VerifiedSource)
                })
            || !valid_evaluation_status_pair(evaluation.result, evaluation.proposed_gate_status)
        {
            return Err(invariant(
                "gate evaluation rule results, evidence, or proposed status are invalid",
            ));
        }
        if let Some(binding) = &evaluation.repository_binding
            && evaluation
                .selected_evidence_refs
                .iter()
                .any(|evidence_ref| {
                    self.evidence
                        .get(evidence_ref)
                        .and_then(|evidence| evidence.repository_binding.as_ref())
                        != Some(binding)
                })
        {
            return Err(invariant(
                "gate evaluation repository binding must match all selected evidence",
            ));
        }
        if gate
            .freshness_policy
            .evidence_must_match_same_repository_snapshot
        {
            let mut selected_bindings = Vec::new();
            let mut selected_without_binding = false;
            for evidence_ref in &evaluation.selected_evidence_refs {
                match self
                    .evidence
                    .get(evidence_ref)
                    .and_then(|evidence| evidence.repository_binding.as_ref())
                {
                    Some(binding) if !selected_bindings.contains(&binding) => {
                        selected_bindings.push(binding);
                    }
                    Some(_) => {}
                    None => selected_without_binding = true,
                }
            }
            let reports_snapshot_conflict = evaluation
                .explanation_codes
                .iter()
                .any(|code| code == "REPOSITORY_SNAPSHOT_CONFLICT");
            let repository_binding_is_exact = match selected_bindings.as_slice() {
                [] => evaluation.repository_binding.is_none() && !reports_snapshot_conflict,
                [binding] => {
                    evaluation.repository_binding.as_ref() == Some(*binding)
                        && !reports_snapshot_conflict
                }
                _ => {
                    evaluation.repository_binding.is_none()
                        && evaluation.result == GateEvaluationResult::Conflicted
                        && reports_snapshot_conflict
                }
            };
            if selected_without_binding || !repository_binding_is_exact {
                return Err(invariant(
                    "same-snapshot gate evaluation lacks a consistent repository binding",
                ));
            }
        }
        self.current_gate_evaluation_refs.insert(
            evaluation.gate_ref.clone(),
            evaluation.gate_evaluation_id.clone(),
        );
        self.gate_evaluations
            .insert(evaluation.gate_evaluation_id.clone(), evaluation.clone());
        match evaluation.result {
            GateEvaluationResult::Pass | GateEvaluationResult::Fail => {}
            GateEvaluationResult::Stale => {
                self.gate_statuses
                    .insert(evaluation.gate_ref.clone(), GateStatus::Stale);
            }
            GateEvaluationResult::Incomplete
            | GateEvaluationResult::Conflicted
            | GateEvaluationResult::Error => {
                self.gate_statuses
                    .insert(evaluation.gate_ref.clone(), GateStatus::Pending);
            }
        }
        self.sync_gate_refs();
        Ok(())
    }

    fn evaluation_has_accepted_evidence(&self, evaluation: &GateEvaluation) -> bool {
        !evaluation.selected_evidence_refs.is_empty()
            && evaluation
                .selected_evidence_refs
                .iter()
                .all(|evidence_ref| {
                    self.evidence
                        .get(evidence_ref)
                        .is_some_and(|evidence| evidence.status == EvidenceStatus::VerifiedSource)
                })
    }

    fn validate_phase_gate_closure(
        &self,
        phase_ref: &StableId,
        evaluation_refs: &[StableId],
    ) -> Result<(), ProjectionError> {
        if !self.phase_required_gates_satisfied(phase_ref) {
            return Err(invariant(
                "a phase cannot close with an unsatisfied required gate",
            ));
        }
        let supplied = evaluation_refs.iter().collect::<BTreeSet<_>>();
        if supplied.len() != evaluation_refs.len() {
            return Err(invariant("phase gate evaluation references must be unique"));
        }
        let required_verified = self
            .gates
            .values()
            .filter(|gate| {
                &gate.phase_ref == phase_ref
                    && gate.criticality == GateCriticality::Required
                    && self.gate_status(&gate.gate_id) == Some(GateStatus::Verified)
            })
            .map(|gate| {
                self.current_gate_evaluation_refs
                    .get(&gate.gate_id)
                    .ok_or_else(|| invariant("verified required gate lacks current evaluation"))
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        if !required_verified.is_subset(&supplied)
            || evaluation_refs.iter().any(|evaluation_ref| {
                !self
                    .gate_evaluations
                    .get(evaluation_ref)
                    .is_some_and(|evaluation| {
                        self.gates.get(&evaluation.gate_ref).is_some_and(|gate| {
                            &gate.phase_ref == phase_ref
                                && self.gate_status(&gate.gate_id) == Some(GateStatus::Verified)
                                && self.current_gate_evaluation_refs.get(&gate.gate_id)
                                    == Some(evaluation_ref)
                        })
                    })
            })
        {
            return Err(invariant(
                "phase completion must cite every current verified required-gate evaluation",
            ));
        }
        Ok(())
    }

    fn validate_completed_phase_gates(&self) -> Result<(), ProjectionError> {
        for (phase_ref, status) in &self.phase_statuses {
            if *status == PhaseStatus::Completed && !self.phase_required_gates_satisfied(phase_ref)
            {
                return Err(invariant(
                    "a completed phase cannot end with an unsatisfied required gate",
                ));
            }
        }
        Ok(())
    }

    fn sync_phase_refs(&mut self) {
        let Some(roadmap) = &self.state.roadmap else {
            self.state.active_phase_refs.clear();
            self.state.primary_active_phase_ref = None;
            self.state.completed_phase_refs.clear();
            return;
        };
        self.state.active_phase_refs = roadmap
            .phase_order
            .iter()
            .filter(|phase_ref| {
                matches!(
                    self.phase_statuses.get(*phase_ref),
                    Some(
                        PhaseStatus::Active
                            | PhaseStatus::Blocked
                            | PhaseStatus::ReadyToClose
                            | PhaseStatus::RevalidationRequired
                    )
                )
            })
            .cloned()
            .collect();
        self.state.primary_active_phase_ref = self.state.active_phase_refs.first().cloned();
        self.state.completed_phase_refs = roadmap
            .phase_order
            .iter()
            .filter(|phase_ref| {
                self.phase_statuses.get(*phase_ref) == Some(&PhaseStatus::Completed)
            })
            .cloned()
            .collect();
    }

    fn sync_gate_refs(&mut self) {
        self.state.pending_gate_refs = self
            .gate_statuses
            .iter()
            .filter(|(_, status)| !matches!(status, GateStatus::Verified | GateStatus::Waived))
            .map(|(gate_ref, _)| gate_ref.clone())
            .collect();
        self.state.verified_gate_refs = self
            .gate_statuses
            .iter()
            .filter(|(_, status)| **status == GateStatus::Verified)
            .map(|(gate_ref, _)| gate_ref.clone())
            .collect();
        self.state.revalidation_required_gate_refs = self
            .gate_statuses
            .iter()
            .filter(|(_, status)| **status == GateStatus::RevalidationRequired)
            .map(|(gate_ref, _)| gate_ref.clone())
            .collect();
    }

    fn sync_decision_refs(&mut self) {
        self.state.current_decision_refs = self
            .decisions
            .values()
            .filter(|decision| decision.status == DecisionStatus::Accepted)
            .map(|decision| decision.decision_id.clone())
            .collect();
    }

    fn sync_blocker_refs(&mut self) {
        self.state.open_blocker_refs = self
            .blockers
            .values()
            .filter(|blocker| is_open_blocker(blocker))
            .map(|blocker| blocker.blocker_id.clone())
            .collect();
    }

    fn sync_risk_refs(&mut self) {
        self.state.open_risk_refs = self
            .risks
            .values()
            .filter(|risk| risk.status != RiskStatus::Closed)
            .map(|risk| risk.risk_id.clone())
            .collect();
    }

    fn sync_action_refs(&mut self) {
        let mut actions = self
            .next_actions
            .values()
            .filter(|action| action_priority(action.classification).is_some())
            .collect::<Vec<_>>();
        actions.sort_by(|left, right| {
            action_priority(left.classification)
                .cmp(&action_priority(right.classification))
                .then_with(|| left.next_action_id.cmp(&right.next_action_id))
        });
        self.state.ordered_next_action_refs = actions
            .iter()
            .map(|action| action.next_action_id.clone())
            .collect();
        self.state.preferred_next_action_ref = self.state.ordered_next_action_refs.first().cloned();
    }

    fn sync_conflict_refs(&mut self) {
        self.state.open_conflict_refs = self
            .conflicts
            .values()
            .filter(|conflict| conflict.status == ConflictStatus::Open)
            .map(|conflict| conflict.conflict_id.clone())
            .collect();
    }
}

fn validate_authority_document(
    document: &project_ledger_domain::AuthorityDocument,
) -> Result<(), ProjectionError> {
    if document.schema_version != SchemaVersion::V0_2_0 {
        return Err(ProjectionError::UnsupportedSchemaVersion);
    }
    if document.supersedes_ref.as_ref() == Some(&document.document_id) {
        return Err(invariant("an authority document cannot supersede itself"));
    }
    Ok(())
}

fn validate_roadmap(roadmap: &Roadmap) -> Result<(), ProjectionError> {
    if roadmap.schema_version != SchemaVersion::V0_2_0 {
        return Err(ProjectionError::UnsupportedSchemaVersion);
    }
    if roadmap.phase_order.is_empty() {
        return Err(invariant("a roadmap must contain at least one phase"));
    }
    let unique = roadmap.phase_order.iter().collect::<BTreeSet<_>>();
    if unique.len() != roadmap.phase_order.len() {
        return Err(invariant("roadmap phase references must be unique"));
    }
    Ok(())
}

fn validate_gate_definition(gate: &GateDefinition) -> Result<(), ProjectionError> {
    validate_record(gate, gate.digest)?;
    if gate.schema_version != SchemaVersion::V0_2_0
        || gate.definition_version != 1
        || gate.title.trim().is_empty()
        || gate.required_evidence_rules.is_empty()
    {
        return Err(invariant("gate definition has invalid initial state"));
    }
    let rule_ids = gate
        .required_evidence_rules
        .iter()
        .map(|rule| &rule.rule_id)
        .collect::<BTreeSet<_>>();
    if rule_ids.len() != gate.required_evidence_rules.len()
        || gate
            .repository_dependencies
            .paths
            .iter()
            .any(|path| path.trim().is_empty())
    {
        return Err(invariant(
            "gate evidence rules and repository dependencies must be unique and explicit",
        ));
    }
    Ok(())
}

const fn is_non_material_continuity_event(event: &Event) -> bool {
    matches!(
        event,
        Event::CheckpointCreated(_)
            | Event::CheckpointInvalidated(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
    )
}

const fn valid_evaluation_status_pair(result: GateEvaluationResult, status: GateStatus) -> bool {
    matches!(
        (result, status),
        (GateEvaluationResult::Pass, GateStatus::Verified)
            | (GateEvaluationResult::Fail, GateStatus::Rejected)
            | (
                GateEvaluationResult::Incomplete
                    | GateEvaluationResult::Conflicted
                    | GateEvaluationResult::Error,
                GateStatus::Pending
            )
            | (
                GateEvaluationResult::Stale,
                GateStatus::RevalidationRequired
            )
    )
}

fn evaluation_result_matches_rules(
    evaluation: &GateEvaluation,
    completion_rule: GateCompletionOperator,
) -> bool {
    let any = |result| {
        evaluation
            .rule_results
            .iter()
            .any(|rule| rule.result == result)
    };
    let all = |result| {
        evaluation
            .rule_results
            .iter()
            .all(|rule| rule.result == result)
    };
    let reports_snapshot_conflict = evaluation
        .explanation_codes
        .iter()
        .any(|code| code == "REPOSITORY_SNAPSHOT_CONFLICT");
    let expected = if reports_snapshot_conflict {
        GateEvaluationResult::Conflicted
    } else {
        match completion_rule {
            GateCompletionOperator::All if all(project_ledger_domain::RuleResult::Pass) => {
                GateEvaluationResult::Pass
            }
            GateCompletionOperator::Any if any(project_ledger_domain::RuleResult::Pass) => {
                GateEvaluationResult::Pass
            }
            _ if any(project_ledger_domain::RuleResult::Error) => GateEvaluationResult::Error,
            _ if any(project_ledger_domain::RuleResult::Conflicted) => {
                GateEvaluationResult::Conflicted
            }
            _ if any(project_ledger_domain::RuleResult::Stale) => GateEvaluationResult::Stale,
            GateCompletionOperator::All if any(project_ledger_domain::RuleResult::Fail) => {
                GateEvaluationResult::Fail
            }
            GateCompletionOperator::Any if all(project_ledger_domain::RuleResult::Fail) => {
                GateEvaluationResult::Fail
            }
            GateCompletionOperator::All | GateCompletionOperator::Any => {
                GateEvaluationResult::Incomplete
            }
        }
    };
    evaluation.result == expected
        && (evaluation.result != GateEvaluationResult::Pass
            || !evaluation.selected_evidence_refs.is_empty())
}

fn authority<'a>(
    documents: &'a [project_ledger_domain::AuthorityDocument],
    document_ref: &StableId,
) -> Result<&'a project_ledger_domain::AuthorityDocument, ProjectionError> {
    documents
        .iter()
        .find(|document| &document.document_id == document_ref)
        .ok_or_else(|| invariant("authority document reference does not exist"))
}

fn authority_mut<'a>(
    documents: &'a mut [project_ledger_domain::AuthorityDocument],
    document_ref: &StableId,
) -> Result<&'a mut project_ledger_domain::AuthorityDocument, ProjectionError> {
    documents
        .iter_mut()
        .find(|document| &document.document_id == document_ref)
        .ok_or_else(|| invariant("authority document reference does not exist"))
}

fn sort_authority(documents: &mut [project_ledger_domain::AuthorityDocument]) {
    documents.sort_by(|left, right| {
        left.authority_rank
            .cmp(&right.authority_rank)
            .then_with(|| left.document_id.cmp(&right.document_id))
    });
}

fn validate_next_action(state: &ProjectState, action: &NextAction) -> Result<(), ProjectionError> {
    if action.schema_version != SchemaVersion::V0_2_0
        || action.completed_at.is_some()
        || matches!(
            action.classification,
            NextActionClassification::Completed | NextActionClassification::Cancelled
        )
    {
        return Err(invariant("a next action has invalid initial state"));
    }
    if let Some(phase_ref) = &action.phase_ref
        && !state
            .roadmap
            .as_ref()
            .is_some_and(|roadmap| roadmap.phase_order.contains(phase_ref))
    {
        return Err(invariant("next action phase is not in the current roadmap"));
    }
    if action.classification == NextActionClassification::Authorized {
        let authority_ref = action
            .authority_ref
            .as_ref()
            .ok_or_else(|| invariant("an authorized next action requires authority"))?;
        if authority(&state.authority_documents, authority_ref)?.status
            != AuthorityDocumentStatus::Current
        {
            return Err(invariant("next action authority must be current"));
        }
    }
    Ok(())
}

fn action_priority(classification: NextActionClassification) -> Option<u8> {
    match classification {
        NextActionClassification::Authorized => Some(0),
        NextActionClassification::Eligible => Some(1),
        NextActionClassification::Recommended => Some(2),
        NextActionClassification::Proposed => Some(3),
        NextActionClassification::Blocked
        | NextActionClassification::Completed
        | NextActionClassification::Cancelled => None,
    }
}

fn is_open_blocker(blocker: &Blocker) -> bool {
    matches!(
        blocker.status,
        BlockerStatus::Open | BlockerStatus::Mitigated
    )
}

fn merge_stable_refs(target: &mut Vec<StableId>, additional: &[StableId]) {
    target.extend_from_slice(additional);
    target.sort();
    target.dedup();
}

fn validate_record<T: Serialize>(
    record: &T,
    expected_digest: Sha256Digest,
) -> Result<(), ProjectionError> {
    if record_digest(record)? != expected_digest {
        return Err(invariant(
            "embedded record digest does not match its contents",
        ));
    }
    Ok(())
}

fn refresh_record_digest<T, F>(record: &mut T, assign: F) -> Result<(), ProjectionError>
where
    T: Serialize,
    F: FnOnce(&mut T, Sha256Digest),
{
    let digest = record_digest(record)?;
    assign(record, digest);
    Ok(())
}

fn record_digest<T: Serialize>(record: &T) -> Result<Sha256Digest, ProjectionError> {
    let mut value = serde_json::to_value(record)
        .map_err(|error| ProjectionError::StateSerialization(error.to_string()))?;
    let object = value.as_object_mut().ok_or_else(|| {
        ProjectionError::StateSerialization("domain record must be a JSON object".to_owned())
    })?;
    object.remove("digest");
    object.remove("payload_digest");
    object.remove("checkpoint_digest");
    Ok(sha256_digest(&value)?)
}

fn invariant(message: &str) -> ProjectionError {
    ProjectionError::LifecycleInvariant {
        message: message.to_owned(),
    }
}
