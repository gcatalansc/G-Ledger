use project_ledger_domain::{
    ActorKind, AuthorityDigestObservation, AuthorityDocumentDigestMismatchEvent,
    AuthorityDocumentDigestRefreshedEvent, AuthorityDocumentMarkedUnavailableEvent,
    AuthorityDocumentRegisteredEvent, AuthorityDocumentStatus, AuthorityDocumentSupersededEvent,
    BlockerOpenedEvent, BlockerResolvedEvent, BlockerUpdatedEvent, Command, CommandEnvelope,
    ConflictKind, ConflictOpenedEvent, ConflictRecord, ConflictResolvedEvent, ConflictStatus,
    DecisionAcceptedEvent, DecisionProposedEvent, DecisionRejectedEvent,
    DecisionRevalidationRequiredEvent, DecisionSupersededEvent, Event, EventId,
    NextActionCancelledEvent, NextActionCompletedEvent, NextActionSetEvent, PhaseBlockedEvent,
    PhaseReopenedEvent, PhaseSkippedEvent, PhaseStartedEvent, PhaseStatus, ProjectArchivedEvent,
    ProjectMetadataUpdatedEvent, ProjectPausedEvent, ProjectResumedEvent,
    ProjectStatusDerivedEvent, RiskClosedEvent, RiskMitigatedEvent, RiskOpenedEvent,
    RiskUpdatedEvent, RoadmapDefinedEvent, RoadmapRevisedEvent, ScopeKind, Severity, StableId,
};
use project_ledger_event_store::{EventDraft, IntegrityError, build_event};
use project_ledger_projection_engine::{
    ProjectStateProjection, ProjectionError, validate_state_integrity,
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Pure, storage-independent S4 command engine.
#[derive(Clone, Copy, Debug, Default)]
pub struct LifecycleEngine;

/// A fail-closed lifecycle command validation failure.
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("invalid lifecycle command: {message}")]
    InvalidCommand {
        message: String,
        subject_refs: Vec<StableId>,
    },
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
    #[error("closed contract encoding failed: {0}")]
    ContractEncoding(#[from] serde_json::Error),
    #[error("could not derive deterministic lifecycle identity")]
    Identity,
}

impl LifecycleError {
    /// Returns stable domain subjects implicated in a validation failure.
    pub fn subject_refs(&self) -> Vec<StableId> {
        match self {
            Self::InvalidCommand { subject_refs, .. } => subject_refs.clone(),
            Self::Projection(_)
            | Self::Integrity(_)
            | Self::ContractEncoding(_)
            | Self::Identity => Vec::new(),
        }
    }
}

impl LifecycleEngine {
    /// Validates one S4 command against a verified stream and returns a complete
    /// deterministic atomic batch, including status derivation when required.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError`] when the stream is invalid, the command is not
    /// an S4 lifecycle command, or any transition would violate the contract.
    #[allow(clippy::too_many_lines)]
    pub fn handle(
        command: &CommandEnvelope,
        stream: &[project_ledger_domain::EventEnvelope],
    ) -> Result<Vec<EventDraft>, LifecycleError> {
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
        if projection.state().project.archived_at.is_some()
            && !matches!(command.command, Command::ResolveConflict(_))
        {
            return Err(invalid(
                "an archived project rejects further lifecycle mutation",
            ));
        }

        let mut drafts = Vec::new();
        match &command.command {
            Command::CreateProject(_) => {
                return Err(invalid("CREATE_PROJECT belongs to the genesis handler"));
            }
            Command::UpdateProjectMetadata(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                    title: payload.title.clone(),
                    summary: payload.summary.clone(),
                }),
            )?,
            Command::PauseProject(payload) => {
                require_human(command)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::ProjectPaused(ProjectPausedEvent {
                        reason: payload.reason.clone(),
                    }),
                )?;
            }
            Command::ResumeProject(payload) => {
                require_human(command)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::ProjectResumed(ProjectResumedEvent {
                        reason: payload.reason.clone(),
                    }),
                )?;
            }
            Command::ArchiveProject(payload) => {
                require_human(command)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::ProjectArchived(ProjectArchivedEvent {
                        reason: payload.reason.clone(),
                    }),
                )?;
            }
            Command::RegisterAuthorityDocument(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::AuthorityDocumentRegistered(AuthorityDocumentRegisteredEvent {
                    document: payload.document.clone(),
                }),
            )?,
            Command::RefreshAuthorityDocumentDigest(payload) => {
                let canonical = projection
                    .authority_document(&payload.document_ref)
                    .ok_or_else(|| {
                        invalid_subject("authority document does not exist", &payload.document_ref)
                    })?
                    .clone();
                match &payload.observation {
                    AuthorityDigestObservation::Match { observed_digest } => append(
                        command,
                        &mut projection,
                        &mut drafts,
                        Event::AuthorityDocumentDigestRefreshed(
                            AuthorityDocumentDigestRefreshedEvent {
                                document_ref: payload.document_ref.clone(),
                                observed_digest: *observed_digest,
                            },
                        ),
                    )?,
                    AuthorityDigestObservation::Unavailable { reason } => append(
                        command,
                        &mut projection,
                        &mut drafts,
                        Event::AuthorityDocumentMarkedUnavailable(
                            AuthorityDocumentMarkedUnavailableEvent {
                                document_ref: payload.document_ref.clone(),
                                reason: reason.clone(),
                            },
                        ),
                    )?,
                    AuthorityDigestObservation::Mismatch { observed_digest } => {
                        append(
                            command,
                            &mut projection,
                            &mut drafts,
                            Event::AuthorityDocumentDigestMismatch(
                                AuthorityDocumentDigestMismatchEvent {
                                    document_ref: payload.document_ref.clone(),
                                    expected_digest: canonical.digest,
                                    observed_digest: *observed_digest,
                                },
                            ),
                        )?;
                        append(
                            command,
                            &mut projection,
                            &mut drafts,
                            Event::ConflictOpened(ConflictOpenedEvent {
                                conflict: ConflictRecord {
                                    schema_version: command.schema_version,
                                    conflict_id: stable_command_id("conflict", command)?,
                                    kind: ConflictKind::AuthorityDocumentDivergence,
                                    severity: Severity::Critical,
                                    domain: "authority".to_owned(),
                                    subject_refs: vec![payload.document_ref.clone()],
                                    source_refs: vec![canonical.uri],
                                    status: ConflictStatus::Open,
                                    required_action: "register a new authority version or confirm the canonical artifact".to_owned(),
                                    opened_at: command.submitted_at.clone(),
                                    resolved_at: None,
                                    resolution_ref: None,
                                },
                            }),
                        )?;
                    }
                }
            }
            Command::SupersedeAuthorityDocument(payload) => {
                let replacement = projection
                    .authority_document(&payload.replacement.document_id)
                    .ok_or_else(|| {
                        invalid_subject(
                            "replacement authority must already be registered",
                            &payload.replacement.document_id,
                        )
                    })?;
                if replacement != &payload.replacement
                    || payload.replacement.supersedes_ref.as_ref() != Some(&payload.document_ref)
                {
                    return Err(invalid_subject(
                        "registered authority replacement differs from the command payload",
                        &payload.replacement.document_id,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::AuthorityDocumentSuperseded(AuthorityDocumentSupersededEvent {
                        document_ref: payload.document_ref.clone(),
                        replacement_ref: payload.replacement.document_id.clone(),
                    }),
                )?;
            }
            Command::DefineRoadmap(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RoadmapDefined(RoadmapDefinedEvent {
                    roadmap: payload.roadmap.clone(),
                }),
            )?,
            Command::ReviseRoadmap(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RoadmapRevised(RoadmapRevisedEvent {
                    roadmap: payload.roadmap.clone(),
                }),
            )?,
            Command::StartPhase(payload) => {
                if projection.has_open_critical_authority_conflict() {
                    return Err(invalid(
                        "a critical authority conflict prevents phase start",
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::PhaseStarted(PhaseStartedEvent {
                        phase_ref: payload.phase_ref.clone(),
                    }),
                )?;
            }
            Command::ReopenPhase(payload) => {
                require_human(command)?;
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::PhaseReopened(PhaseReopenedEvent {
                        phase_ref: payload.phase_ref.clone(),
                        resulting_status: PhaseStatus::Active,
                        reason: payload.reason.clone(),
                    }),
                )?;
            }
            Command::SkipPhase(payload) => {
                require_human(command)?;
                let authority = projection
                    .authority_document(&payload.authority_ref)
                    .ok_or_else(|| {
                        invalid_subject("skip authority does not exist", &payload.authority_ref)
                    })?;
                if authority.status != AuthorityDocumentStatus::Current {
                    return Err(invalid_subject(
                        "skip authority must be current",
                        &payload.authority_ref,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::PhaseSkipped(PhaseSkippedEvent {
                        phase_ref: payload.phase_ref.clone(),
                        authority_ref: payload.authority_ref.clone(),
                        reason: payload.reason.clone(),
                    }),
                )?;
            }
            Command::ProposeDecision(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::DecisionProposed(DecisionProposedEvent {
                    decision: payload.decision.clone(),
                }),
            )?,
            Command::AcceptDecision(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::DecisionAccepted(DecisionAcceptedEvent {
                    decision_ref: payload.decision_ref.clone(),
                    authority_ref: payload.authority_ref.clone(),
                }),
            )?,
            Command::SupersedeDecision(payload) => {
                let replacement = projection
                    .decision(&payload.replacement.decision_id)
                    .ok_or_else(|| {
                        invalid_subject(
                            "replacement decision must already be durable",
                            &payload.replacement.decision_id,
                        )
                    })?;
                if replacement != &payload.replacement
                    || payload.replacement.supersedes_ref.as_ref() != Some(&payload.decision_ref)
                {
                    return Err(invalid_subject(
                        "durable decision replacement differs from the command payload",
                        &payload.replacement.decision_id,
                    ));
                }
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::DecisionSuperseded(DecisionSupersededEvent {
                        decision_ref: payload.decision_ref.clone(),
                        replacement_ref: payload.replacement.decision_id.clone(),
                    }),
                )?;
            }
            Command::RejectDecision(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::DecisionRejected(DecisionRejectedEvent {
                    decision_ref: payload.decision_ref.clone(),
                    reason: payload.reason.clone(),
                }),
            )?,
            Command::OpenBlocker(payload) => {
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::BlockerOpened(BlockerOpenedEvent {
                        blocker: payload.blocker.clone(),
                    }),
                )?;
                maybe_block_phase(command, &mut projection, &mut drafts, &payload.blocker)?;
            }
            Command::UpdateBlocker(payload) => {
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::BlockerUpdated(BlockerUpdatedEvent {
                        blocker: payload.blocker.clone(),
                    }),
                )?;
                maybe_block_phase(command, &mut projection, &mut drafts, &payload.blocker)?;
            }
            Command::ResolveBlocker(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::BlockerResolved(BlockerResolvedEvent {
                    blocker_ref: payload.blocker_ref.clone(),
                    resolution_summary: payload.resolution_summary.clone(),
                    evidence_refs: payload.evidence_refs.clone(),
                }),
            )?,
            Command::OpenRisk(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RiskOpened(RiskOpenedEvent {
                    risk: payload.risk.clone(),
                }),
            )?,
            Command::UpdateRisk(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RiskUpdated(RiskUpdatedEvent {
                    risk: payload.risk.clone(),
                }),
            )?,
            Command::MitigateRisk(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RiskMitigated(RiskMitigatedEvent {
                    risk_ref: payload.risk_ref.clone(),
                    note: payload.note.clone(),
                    evidence_refs: payload.evidence_refs.clone(),
                }),
            )?,
            Command::CloseRisk(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::RiskClosed(RiskClosedEvent {
                    risk_ref: payload.risk_ref.clone(),
                    note: payload.note.clone(),
                    evidence_refs: payload.evidence_refs.clone(),
                }),
            )?,
            Command::SetNextAction(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::NextActionSet(NextActionSetEvent {
                    action: payload.action.clone(),
                }),
            )?,
            Command::CompleteNextAction(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::NextActionCompleted(NextActionCompletedEvent {
                    action_ref: payload.action_ref.clone(),
                    evidence_refs: payload.evidence_refs.clone(),
                }),
            )?,
            Command::CancelNextAction(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::NextActionCancelled(NextActionCancelledEvent {
                    action_ref: payload.action_ref.clone(),
                    reason: payload.reason.clone(),
                }),
            )?,
            Command::ResolveConflict(payload) => append(
                command,
                &mut projection,
                &mut drafts,
                Event::ConflictResolved(ConflictResolvedEvent {
                    conflict_ref: payload.conflict_ref.clone(),
                    resolution_ref: payload.resolution_ref.clone(),
                    reason: payload.reason.clone(),
                }),
            )?,
            _ => {
                return Err(invalid(
                    "command belongs to a later slice or a different bounded engine",
                ));
            }
        }

        if matches!(
            command.command,
            Command::RegisterAuthorityDocument(_)
                | Command::RefreshAuthorityDocumentDigest(_)
                | Command::SupersedeAuthorityDocument(_)
        ) {
            let affected: Vec<_> = projection
                .state()
                .current_decision_refs
                .iter()
                .filter(|id| {
                    projection
                        .decision(id)
                        .and_then(|decision| decision.authority_ref.as_ref())
                        .and_then(|id| projection.authority_document(id))
                        .is_none_or(|authority| {
                            authority.status != AuthorityDocumentStatus::Current
                        })
                })
                .cloned()
                .collect();
            for decision_ref in affected {
                append(
                    command,
                    &mut projection,
                    &mut drafts,
                    Event::DecisionRevalidationRequired(DecisionRevalidationRequiredEvent {
                        decision_ref,
                        reason_codes: vec!["AUTHORITY_NOT_CURRENT".to_owned()],
                    }),
                )?;
            }
        }
        append_status_if_changed(command, &mut projection, &mut drafts)?;
        Ok(drafts)
    }
}

fn append_status_if_changed(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), LifecycleError> {
    let previous_status = projection.state().project.lifecycle_status;
    let new_status = projection.derived_lifecycle_status();
    if previous_status == new_status {
        return Ok(());
    }
    let reason_code = serde_json::to_value(command.command_type())?
        .as_str()
        .ok_or(LifecycleError::Identity)?
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

fn maybe_block_phase(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    blocker: &project_ledger_domain::Blocker,
) -> Result<(), LifecycleError> {
    if blocker.scope_kind == ScopeKind::Phase
        && blocker.status == project_ledger_domain::BlockerStatus::Open
        && projection.phase_status(&blocker.scope_ref) == Some(PhaseStatus::Active)
    {
        append(
            command,
            projection,
            drafts,
            Event::PhaseBlocked(PhaseBlockedEvent {
                phase_ref: blocker.scope_ref.clone(),
                blocker_refs: vec![blocker.blocker_id.clone()],
            }),
        )?;
    }
    Ok(())
}

fn append(
    command: &CommandEnvelope,
    projection: &mut ProjectStateProjection,
    drafts: &mut Vec<EventDraft>,
    event: Event,
) -> Result<(), LifecycleError> {
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

fn deterministic_event_id(
    command: &CommandEnvelope,
    ordinal: usize,
) -> Result<EventId, LifecycleError> {
    let command_uuid = command.command_id.uuid();
    let mut hasher = Sha256::new();
    hasher.update(b"project-ledger:event-id:v1");
    hasher.update(command_uuid.as_bytes());
    hasher.update(
        u64::try_from(ordinal)
            .map_err(|_| LifecycleError::Identity)?
            .to_be_bytes(),
    );
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes[..6].copy_from_slice(&command_uuid.as_bytes()[..6]);
    bytes[6..].copy_from_slice(&digest[..10]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let uuid = Uuid::from_bytes(bytes);
    EventId::parse(&format!("event:{uuid}")).map_err(|_| LifecycleError::Identity)
}

fn stable_command_id(prefix: &str, command: &CommandEnvelope) -> Result<StableId, LifecycleError> {
    StableId::parse(&format!("{prefix}:{}", command.command_id.uuid()))
        .map_err(|_| LifecycleError::Identity)
}

fn require_human(command: &CommandEnvelope) -> Result<(), LifecycleError> {
    if command.actor.actor_kind != ActorKind::Human {
        return Err(invalid(
            "this lifecycle command requires an explicit human actor",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> LifecycleError {
    LifecycleError::InvalidCommand {
        message: message.to_owned(),
        subject_refs: Vec::new(),
    }
}

fn invalid_subject(message: &str, subject_ref: &StableId) -> LifecycleError {
    LifecycleError::InvalidCommand {
        message: message.to_owned(),
        subject_refs: vec![subject_ref.clone()],
    }
}
