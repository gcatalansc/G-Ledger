use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    AuthorityDocumentStatus, Blocker, BlockerStatus, ConflictKind, ConflictRecord, ConflictStatus,
    Decision, DecisionStatus, Event, EventEnvelope, NextAction, NextActionClassification,
    PhaseStatus, ProjectLifecycleStatus, ProjectState, Risk, RiskStatus, Roadmap, SchemaVersion,
    ScopeKind, Severity, StableId,
};
use serde::Serialize;

use crate::project_state::{
    ProjectionError, advance_material_state, genesis_state, validate_next_event,
};

/// Stateful deterministic fold retaining indexes that are intentionally absent
/// from the compact public `ProjectState` contract.
#[derive(Clone, Debug)]
pub struct ProjectStateProjection {
    state: ProjectState,
    head_digest: Sha256Digest,
    phase_statuses: BTreeMap<StableId, PhaseStatus>,
    decisions: BTreeMap<StableId, Decision>,
    blockers: BTreeMap<StableId, Blocker>,
    risks: BTreeMap<StableId, Risk>,
    next_actions: BTreeMap<StableId, NextAction>,
    conflicts: BTreeMap<StableId, ConflictRecord>,
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
            state,
            head_digest,
            phase_statuses: BTreeMap::new(),
            decisions: BTreeMap::new(),
            blockers: BTreeMap::new(),
            risks: BTreeMap::new(),
            next_actions: BTreeMap::new(),
            conflicts: BTreeMap::new(),
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
        advance_material_state(&mut self.state, event)?;
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
                authority_mut(
                    &mut self.state.authority_documents,
                    &unavailable.document_ref,
                )?
                .status = AuthorityDocumentStatus::Unavailable;
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
                document.status = AuthorityDocumentStatus::DigestMismatch;
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
                self.phase_statuses
                    .insert(ready.phase_ref.clone(), PhaseStatus::ReadyToClose);
                self.sync_phase_refs();
            }
            Event::PhaseCompleted(completed) => {
                self.require_phase_status(
                    &completed.phase_ref,
                    &[PhaseStatus::Active, PhaseStatus::ReadyToClose],
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
                if decision.status != DecisionStatus::Proposed {
                    return Err(invariant("only a proposed decision can be accepted"));
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
            _ => return Err(ProjectionError::UnsupportedEvent),
        }
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
    value
        .as_object_mut()
        .ok_or_else(|| {
            ProjectionError::StateSerialization("domain record must be a JSON object".to_owned())
        })?
        .remove("digest");
    Ok(sha256_digest(&value)?)
}

fn invariant(message: &str) -> ProjectionError {
    ProjectionError::LifecycleInvariant {
        message: message.to_owned(),
    }
}
