use project_ledger_canonical_json::{CanonicalJsonError, Sha256Digest, sha256_digest};
use project_ledger_domain::{CheckpointFreshnessStatus, ResumeProjection, SchemaVersion, StableId};
use project_ledger_projection_engine::ProjectStateProjection;
use thiserror::Error;

/// Normative S6 resume output profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResumeBudget {
    Compact,
    AgentMarkdown,
}

impl ResumeBudget {
    const fn limit(self) -> u32 {
        match self {
            Self::Compact => 220,
            Self::AgentMarkdown => 450,
        }
    }
}

/// Conservative deterministic counter used because v0.2.0 fixes no tokenizer.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConservativeTokenCounter;

impl ConservativeTokenCounter {
    /// Counts UTF-8 bytes as conservative, reproducible budget units.
    pub fn count(value: &str) -> u32 {
        u32::try_from(value.len()).unwrap_or(u32::MAX)
    }
}

/// Resume construction failures.
#[derive(Debug, Error)]
pub enum ResumeError {
    #[error("resume cannot fit the requested deterministic budget")]
    BudgetTooSmall,
    #[error("resume encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

/// Builds bounded resume projections from trusted derived state.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResumeBuilder;

impl ResumeBuilder {
    /// Builds a deterministic projection and removes lowest-priority tails until
    /// the selected conservative budget fits.
    ///
    /// # Errors
    ///
    /// Returns an error only when the irreducible critical form cannot fit or
    /// canonical digest construction fails.
    pub fn build(
        projection: &ProjectStateProjection,
        budget: ResumeBudget,
    ) -> Result<ResumeProjection, ResumeError> {
        let state = projection.state();
        let checkpoint_freshness = state
            .checkpoint_ref
            .as_ref()
            .and_then(|checkpoint_ref| projection.checkpoint_freshness(checkpoint_ref))
            .unwrap_or(CheckpointFreshnessStatus::Unknown);
        let mut resume = ResumeProjection {
            schema_version: SchemaVersion::V0_2_0,
            project_ref: state.project.project_id.clone(),
            state_version: state.state_version,
            state_digest: state.state_digest,
            checkpoint_freshness,
            authority_document_refs: state
                .authority_documents
                .iter()
                .map(|document| document.document_id.clone())
                .collect(),
            active_phase_refs: state.active_phase_refs.clone(),
            completed_phase_refs: state.completed_phase_refs.clone(),
            current_decision_refs: state.current_decision_refs.clone(),
            open_blocker_refs: state.open_blocker_refs.clone(),
            open_risk_refs: state.open_risk_refs.clone(),
            pending_gate_refs: state.pending_gate_refs.clone(),
            last_material_progress_summary: projection
                .latest_work_record()
                .map(|work| normalize_summary(&work.summary, 96)),
            preferred_next_action_ref: state.preferred_next_action_ref.clone(),
            critical_unknown_refs: projection.critical_unknown_refs(),
            conflict_refs: state.open_conflict_refs.clone(),
            checkpoint_ref: state.checkpoint_ref.clone(),
            rendered_token_count: 0,
            digest: Sha256Digest::from([0_u8; 32]),
        };

        loop {
            let rendered = match budget {
                ResumeBudget::Compact => render_compact(&resume),
                ResumeBudget::AgentMarkdown => render_agent_markdown(&resume),
            };
            let count = ConservativeTokenCounter::count(&rendered);
            if count <= budget.limit() {
                resume.rendered_token_count = count;
                resume.digest = resume_digest(&resume)?;
                return Ok(resume);
            }
            if !reduce_lowest_priority(&mut resume) {
                return Err(ResumeError::BudgetTooSmall);
            }
        }
    }
}

/// Renders the bounded Markdown handoff intended for an agent context.
pub fn render_agent_markdown(resume: &ResumeProjection) -> String {
    format!(
        "# Project Ledger Resume\n- Project: `{}`\n- State: v{}\n- Freshness: `{}`\n- Active: {}\n- Completed: {}\n- Decisions: {}\n- Blockers: {}\n- Risks: {}\n- Gates: {}\n- Progress: {}\n- Next: {}\n- Critical unknowns: {}\n- Conflicts: {}\n- Checkpoint: {}\n",
        short_ref(resume.project_ref.as_str()),
        resume.state_version,
        freshness_name(resume.checkpoint_freshness),
        render_refs(&resume.active_phase_refs),
        render_refs(&resume.completed_phase_refs),
        render_refs(&resume.current_decision_refs),
        render_refs(&resume.open_blocker_refs),
        render_refs(&resume.open_risk_refs),
        render_refs(&resume.pending_gate_refs),
        resume
            .last_material_progress_summary
            .as_deref()
            .unwrap_or("-"),
        resume
            .preferred_next_action_ref
            .as_ref()
            .map_or("-".to_owned(), |value| short_ref(value.as_str())),
        render_refs(&resume.critical_unknown_refs),
        render_refs(&resume.conflict_refs),
        resume
            .checkpoint_ref
            .as_ref()
            .map_or("-".to_owned(), |value| short_ref(value.as_str())),
    )
}

/// Renders the compact deterministic text measured by `rendered_token_count`.
pub fn render_compact(resume: &ResumeProjection) -> String {
    format!(
        "project={} v={} freshness={} active={} completed={} decisions={} blockers={} risks={} gates={} progress={} next={} unknowns={} conflicts={} checkpoint={}",
        short_ref(resume.project_ref.as_str()),
        resume.state_version,
        freshness_name(resume.checkpoint_freshness),
        render_refs(&resume.active_phase_refs),
        render_refs(&resume.completed_phase_refs),
        render_refs(&resume.current_decision_refs),
        render_refs(&resume.open_blocker_refs),
        render_refs(&resume.open_risk_refs),
        render_refs(&resume.pending_gate_refs),
        resume
            .last_material_progress_summary
            .as_deref()
            .unwrap_or("-"),
        resume
            .preferred_next_action_ref
            .as_ref()
            .map_or("-".to_owned(), |value| short_ref(value.as_str())),
        render_refs(&resume.critical_unknown_refs),
        render_refs(&resume.conflict_refs),
        resume
            .checkpoint_ref
            .as_ref()
            .map_or("-".to_owned(), |value| short_ref(value.as_str())),
    )
}

fn reduce_lowest_priority(resume: &mut ResumeProjection) -> bool {
    if resume.last_material_progress_summary.take().is_some() {
        return true;
    }
    for refs in [
        &mut resume.authority_document_refs,
        &mut resume.completed_phase_refs,
        &mut resume.current_decision_refs,
        &mut resume.open_risk_refs,
        &mut resume.open_blocker_refs,
        &mut resume.pending_gate_refs,
        &mut resume.active_phase_refs,
    ] {
        if refs.pop().is_some() {
            return true;
        }
    }
    if resume.critical_unknown_refs.len() > 1 {
        resume.critical_unknown_refs.pop();
        return true;
    }
    if resume.conflict_refs.len() > 1 {
        resume.conflict_refs.pop();
        return true;
    }
    false
}

fn render_refs(refs: &[StableId]) -> String {
    if refs.is_empty() {
        return "-".to_owned();
    }
    refs.iter()
        .map(|value| short_ref(value.as_str()))
        .collect::<Vec<_>>()
        .join(",")
}

fn short_ref(value: &str) -> String {
    const MAX_CHARS: usize = 20;
    if value.chars().count() <= MAX_CHARS {
        return value.to_owned();
    }
    let mut shortened = value.chars().take(MAX_CHARS - 1).collect::<String>();
    shortened.push('…');
    shortened
}

fn normalize_summary(value: &str, max_chars: usize) -> String {
    let normalized = value
        .split_whitespace()
        .map(redact_secret_word)
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut bounded = normalized.chars().take(max_chars - 1).collect::<String>();
    bounded.push('…');
    bounded
}

fn redact_secret_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    for prefix in [
        "api_key=",
        "apikey=",
        "access_token=",
        "token=",
        "secret=",
        "password=",
    ] {
        if lower.starts_with(prefix) {
            return format!("{}[REDACTED]", &word[..prefix.len()]);
        }
    }
    word.to_owned()
}

const fn freshness_name(status: CheckpointFreshnessStatus) -> &'static str {
    match status {
        CheckpointFreshnessStatus::Current => "CURRENT",
        CheckpointFreshnessStatus::CurrentForProjectOnly => "CURRENT_FOR_PROJECT_ONLY",
        CheckpointFreshnessStatus::StaleForRepository => "STALE_FOR_REPOSITORY",
        CheckpointFreshnessStatus::StaleForAuthority => "STALE_FOR_AUTHORITY",
        CheckpointFreshnessStatus::SupersededByNewerCheckpoint => "SUPERSEDED_BY_NEWER_CHECKPOINT",
        CheckpointFreshnessStatus::Conflicted => "CONFLICTED",
        CheckpointFreshnessStatus::Invalidated => "INVALIDATED",
        CheckpointFreshnessStatus::Unknown => "UNKNOWN",
    }
}

fn resume_digest(resume: &ResumeProjection) -> Result<Sha256Digest, ResumeError> {
    let mut value = serde_json::to_value(resume)?;
    value
        .as_object_mut()
        .ok_or(ResumeError::BudgetTooSmall)?
        .remove("digest");
    Ok(sha256_digest(&value)?)
}
