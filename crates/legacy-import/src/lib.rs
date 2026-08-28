//! Conservative, review-first import of existing Markdown project ledgers.

use std::collections::{BTreeMap, BTreeSet};

use project_ledger_canonical_json::{
    CanonicalJsonError, Sha256Digest, sha256_bytes, sha256_digest,
};
use project_ledger_domain::{
    ClaimClassification, LedgerTimestamp, LegacyCandidateAction, LegacyCandidateSource,
    LegacyImportCandidate, LegacyImportDecision, LegacyImportMode, LegacyImportReview, LineRange,
    ResourceRef, SchemaVersion, StableId,
};
use serde::Serialize;
use thiserror::Error;

/// Immutable inputs retained as provenance for one legacy preview.
#[derive(Clone, Debug)]
pub struct LegacyReviewRequest {
    pub review_id: StableId,
    pub source_uri: ResourceRef,
    pub source_path: String,
    pub markdown: Vec<u8>,
    pub reviewed_at: LedgerTimestamp,
}

/// Non-durable proposal produced only by an explicit `IMPORT_AS_ASSERTED` decision.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyAssertedProposal {
    pub candidate_ref: StableId,
    pub kind: String,
    pub value: String,
    pub source: LegacyCandidateSource,
    pub classification: ClaimClassification,
}

/// Reviewed record plus the bounded asserted proposals awaiting explicit commands.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewedLegacyImport {
    pub review: LegacyImportReview,
    pub asserted_proposals: Vec<LegacyAssertedProposal>,
}

/// Fail-closed legacy parsing and review failures.
#[derive(Debug, Error)]
pub enum LegacyImportError {
    #[error("legacy review exceeds 1 MiB or 1024 candidates; split the source explicitly")]
    BudgetExceeded,
    #[error("legacy Markdown is not valid UTF-8")]
    InvalidUtf8,
    #[error("legacy source path must be a safe non-empty relative path")]
    InvalidSourcePath,
    #[error("legacy preview contains no recognized explicit candidates")]
    NoCandidates,
    #[error("legacy preview digest or mode is invalid")]
    InvalidPreview,
    #[error("review contains a decision for unknown candidate {candidate_ref}")]
    UnknownCandidate { candidate_ref: StableId },
    #[error("review contains more than one decision for candidate {candidate_ref}")]
    DuplicateDecision { candidate_ref: StableId },
    #[error("review is missing an explicit decision for candidate {candidate_ref}")]
    MissingDecision { candidate_ref: StableId },
    #[error("candidate {candidate_ref} remains REVIEW and cannot be applied")]
    UnresolvedDecision { candidate_ref: StableId },
    #[error("candidate {candidate_ref} has an empty review reason")]
    EmptyReason { candidate_ref: StableId },
    #[error("legacy candidate identifier could not be constructed")]
    InvalidCandidateId,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

/// Conservative legacy Markdown workflow. It never emits commands or events.
#[derive(Clone, Copy, Debug, Default)]
pub struct LegacyImporter;

impl LegacyImporter {
    /// Extracts only explicitly labelled candidate lines and recommends review.
    ///
    /// # Errors
    ///
    /// Returns [`LegacyImportError`] for invalid provenance, encoding or a source
    /// with no recognized explicit candidate. Chronology is never authority.
    pub fn preview(request: LegacyReviewRequest) -> Result<LegacyImportReview, LegacyImportError> {
        if request.markdown.len() > 1024 * 1024 {
            return Err(LegacyImportError::BudgetExceeded);
        }
        validate_source_path(&request.source_path)?;
        let markdown =
            std::str::from_utf8(&request.markdown).map_err(|_| LegacyImportError::InvalidUtf8)?;
        let source_digest = sha256_bytes(&request.markdown);
        let mut section = "(sin sección)".to_owned();
        let mut candidates = Vec::new();
        for (index, original_line) in markdown.lines().enumerate() {
            let line_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let trimmed = original_line.trim();
            if let Some(heading) = parse_heading(trimmed) {
                heading.clone_into(&mut section);
                continue;
            }
            let Some((kind, value)) = parse_candidate(trimmed) else {
                continue;
            };
            let candidate_id = candidate_id(source_digest, line_number, kind, value)?;
            if candidates.len() >= 1024 {
                return Err(LegacyImportError::BudgetExceeded);
            }
            candidates.push(LegacyImportCandidate {
                schema_version: SchemaVersion::V0_2_0,
                candidate_id,
                kind: kind.to_owned(),
                value: value.to_owned(),
                source: LegacyCandidateSource {
                    path: request.source_path.clone(),
                    section: section.clone(),
                    line_range: LineRange([line_number, line_number]),
                },
                conflicts_with: Vec::new(),
                recommended_action: LegacyCandidateAction::Review,
            });
        }
        if candidates.is_empty() {
            return Err(LegacyImportError::NoCandidates);
        }
        link_contradictions(&mut candidates);
        candidates.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

        let mut review = LegacyImportReview {
            schema_version: SchemaVersion::V0_2_0,
            review_id: request.review_id,
            source_uri: request.source_uri,
            source_digest,
            mode: LegacyImportMode::Preview,
            candidates,
            decisions: Vec::new(),
            reviewed_at: request.reviewed_at,
            digest: zero_digest(),
        };
        review.digest = record_digest(&review)?;
        Ok(review)
    }

    /// Validates one explicit terminal decision per candidate.
    ///
    /// # Errors
    ///
    /// Returns [`LegacyImportError`] if the preview changed, any candidate is
    /// missing/duplicated/foreign, a reason is empty or an action remains REVIEW.
    pub fn finalize(
        preview: &LegacyImportReview,
        decisions: Vec<LegacyImportDecision>,
        reviewed_at: LedgerTimestamp,
    ) -> Result<ReviewedLegacyImport, LegacyImportError> {
        validate_preview(preview)?;
        let candidates = preview
            .candidates
            .iter()
            .map(|candidate| (candidate.candidate_id.clone(), candidate))
            .collect::<BTreeMap<_, _>>();
        let mut by_candidate = BTreeMap::new();
        for decision in decisions {
            if !candidates.contains_key(&decision.candidate_ref) {
                return Err(LegacyImportError::UnknownCandidate {
                    candidate_ref: decision.candidate_ref,
                });
            }
            if decision.reason.trim().is_empty() {
                return Err(LegacyImportError::EmptyReason {
                    candidate_ref: decision.candidate_ref,
                });
            }
            if decision.action == LegacyCandidateAction::Review {
                return Err(LegacyImportError::UnresolvedDecision {
                    candidate_ref: decision.candidate_ref,
                });
            }
            let candidate_ref = decision.candidate_ref.clone();
            if by_candidate
                .insert(candidate_ref.clone(), decision)
                .is_some()
            {
                return Err(LegacyImportError::DuplicateDecision { candidate_ref });
            }
        }
        for candidate in &preview.candidates {
            if !by_candidate.contains_key(&candidate.candidate_id) {
                return Err(LegacyImportError::MissingDecision {
                    candidate_ref: candidate.candidate_id.clone(),
                });
            }
        }

        let asserted_proposals = preview
            .candidates
            .iter()
            .filter(|candidate| {
                by_candidate
                    .get(&candidate.candidate_id)
                    .is_some_and(|decision| {
                        decision.action == LegacyCandidateAction::ImportAsAsserted
                    })
            })
            .map(|candidate| LegacyAssertedProposal {
                candidate_ref: candidate.candidate_id.clone(),
                kind: candidate.kind.clone(),
                value: candidate.value.clone(),
                source: candidate.source.clone(),
                classification: ClaimClassification::Asserted,
            })
            .collect();
        let mut review = LegacyImportReview {
            schema_version: SchemaVersion::V0_2_0,
            review_id: preview.review_id.clone(),
            source_uri: preview.source_uri.clone(),
            source_digest: preview.source_digest,
            mode: LegacyImportMode::ReviewedImport,
            candidates: preview.candidates.clone(),
            decisions: by_candidate.into_values().collect(),
            reviewed_at,
            digest: zero_digest(),
        };
        review.digest = record_digest(&review)?;
        Ok(ReviewedLegacyImport {
            review,
            asserted_proposals,
        })
    }

    /// Produces an explicit archive-only review without canonical proposals.
    ///
    /// # Errors
    ///
    /// Returns [`LegacyImportError`] when the preview is invalid or the archive
    /// reason is empty. Every candidate receives its own `ARCHIVE_ONLY` decision.
    pub fn archive_only(
        preview: &LegacyImportReview,
        actor_ref: &StableId,
        reason: &str,
        reviewed_at: LedgerTimestamp,
    ) -> Result<LegacyImportReview, LegacyImportError> {
        validate_preview(preview)?;
        if reason.trim().is_empty() {
            return Err(LegacyImportError::EmptyReason {
                candidate_ref: preview.candidates[0].candidate_id.clone(),
            });
        }
        let decisions = preview
            .candidates
            .iter()
            .map(|candidate| LegacyImportDecision {
                candidate_ref: candidate.candidate_id.clone(),
                action: LegacyCandidateAction::ArchiveOnly,
                actor_ref: actor_ref.clone(),
                reason: reason.to_owned(),
            })
            .collect();
        let mut review = LegacyImportReview {
            schema_version: SchemaVersion::V0_2_0,
            review_id: preview.review_id.clone(),
            source_uri: preview.source_uri.clone(),
            source_digest: preview.source_digest,
            mode: LegacyImportMode::ArchiveOnly,
            candidates: preview.candidates.clone(),
            decisions,
            reviewed_at,
            digest: zero_digest(),
        };
        review.digest = record_digest(&review)?;
        Ok(review)
    }
}

fn parse_heading(line: &str) -> Option<&str> {
    let without_hashes = line.strip_prefix('#')?.trim_start_matches('#').trim();
    (!without_hashes.is_empty()).then_some(without_hashes)
}

fn parse_candidate(line: &str) -> Option<(&'static str, &str)> {
    let content = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line)
        .trim();
    let (label, value) = content.split_once(':')?;
    let kind = match label.trim().to_lowercase().as_str() {
        "fase activa" | "active phase" => "ACTIVE_PHASE",
        "fase completada" | "completed phase" => "COMPLETED_PHASE_CLAIM",
        "gate verificado" | "verified gate" => "GATE_STATUS_CLAIM",
        "próximo paso" | "siguiente paso" | "next action" => "NEXT_ACTION",
        "decisión" | "decision" => "DECISION",
        "blocker" | "bloqueo" => "BLOCKER",
        "riesgo" | "risk" => "RISK",
        _ => return None,
    };
    let value = value.trim();
    (!value.is_empty()).then_some((kind, value))
}

fn link_contradictions(candidates: &mut [LegacyImportCandidate]) {
    let mut links = BTreeSet::new();
    for left in 0..candidates.len() {
        for right in left + 1..candidates.len() {
            if contradict(&candidates[left], &candidates[right]) {
                links.insert((left, right));
            }
        }
    }
    for (left, right) in links {
        let left_id = candidates[left].candidate_id.clone();
        let right_id = candidates[right].candidate_id.clone();
        candidates[left].conflicts_with.push(right_id);
        candidates[right].conflicts_with.push(left_id);
    }
    for candidate in candidates {
        candidate.conflicts_with.sort();
        candidate.conflicts_with.dedup();
    }
}

fn contradict(left: &LegacyImportCandidate, right: &LegacyImportCandidate) -> bool {
    (left.kind == "ACTIVE_PHASE" && right.kind == "ACTIVE_PHASE" && left.value != right.value)
        || ((left.kind == "ACTIVE_PHASE" && right.kind == "COMPLETED_PHASE_CLAIM")
            || (right.kind == "ACTIVE_PHASE" && left.kind == "COMPLETED_PHASE_CLAIM"))
            && left.value == right.value
}

fn candidate_id(
    source_digest: Sha256Digest,
    line: u32,
    kind: &str,
    value: &str,
) -> Result<StableId, LegacyImportError> {
    let digest = sha256_digest(&(source_digest, line, kind, value))?.to_string();
    StableId::parse(&format!("legacy:{line:06}:{}", &digest[7..19]))
        .map_err(|_| LegacyImportError::InvalidCandidateId)
}

fn validate_preview(preview: &LegacyImportReview) -> Result<(), LegacyImportError> {
    if preview.schema_version != SchemaVersion::V0_2_0
        || preview.mode != LegacyImportMode::Preview
        || !preview.decisions.is_empty()
        || preview.candidates.is_empty()
        || preview.candidates.iter().any(|candidate| {
            candidate.schema_version != SchemaVersion::V0_2_0
                || candidate.recommended_action != LegacyCandidateAction::Review
        })
        || record_digest(preview)? != preview.digest
    {
        return Err(LegacyImportError::InvalidPreview);
    }
    Ok(())
}

fn validate_source_path(path: &str) -> Result<(), LegacyImportError> {
    let bytes = path.as_bytes();
    if path.trim().is_empty()
        || path.contains("..")
        || path.starts_with('/')
        || path.starts_with("\\\\")
        || path.contains('\0')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
    {
        return Err(LegacyImportError::InvalidSourcePath);
    }
    Ok(())
}

fn record_digest<T: Serialize>(record: &T) -> Result<Sha256Digest, LegacyImportError> {
    let mut value = serde_json::to_value(record)?;
    value
        .as_object_mut()
        .ok_or(LegacyImportError::InvalidPreview)?
        .remove("digest");
    Ok(sha256_digest(&value)?)
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from([0_u8; 32])
}
