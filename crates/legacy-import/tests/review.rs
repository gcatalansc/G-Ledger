use project_ledger_domain::{
    ClaimClassification, LedgerTimestamp, LegacyCandidateAction, LegacyImportDecision,
    LegacyImportMode, ResourceRef, StableId,
};
use project_ledger_legacy_import::{LegacyImportError, LegacyImporter, LegacyReviewRequest};

const MARKDOWN: &str = "# Estado ejecutivo actual\n\
\n\
- Fase activa: phase-2\n\
- Gate verificado: gate:release\n\
- Próximo paso: ejecutar build\n\
\n\
## Historial\n\
\n\
- Fase activa: phase-0\n\
- Fase completada: phase-2\n";

#[test]
fn excessive_candidates_are_rejected_before_quadratic_contradiction_linking() {
    let mut input = request();
    input.markdown = "- Active phase: phase-one\n".repeat(1025).into_bytes();
    assert!(LegacyImporter::preview(input).is_err());
}

#[test]
fn ac_034_preview_is_deterministic_line_addressed_and_preserves_contradictions() {
    let first = LegacyImporter::preview(request()).unwrap();
    let second = LegacyImporter::preview(request()).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.mode, LegacyImportMode::Preview);
    assert!(first.decisions.is_empty());
    assert_eq!(first.candidates.len(), 5);
    assert!(
        first
            .candidates
            .iter()
            .all(|candidate| candidate.recommended_action == LegacyCandidateAction::Review)
    );

    let active = first
        .candidates
        .iter()
        .find(|candidate| candidate.kind == "ACTIVE_PHASE" && candidate.value == "phase-2")
        .unwrap();
    assert_eq!(active.source.path, "BITACORA_PROYECTO.md");
    assert_eq!(active.source.section, "Estado ejecutivo actual");
    assert_eq!(active.source.line_range.0, [3, 3]);
    assert_eq!(active.conflicts_with.len(), 2);

    let gate = first
        .candidates
        .iter()
        .find(|candidate| candidate.value == "gate:release")
        .unwrap();
    assert_eq!(gate.kind, "GATE_STATUS_CLAIM");
    assert_eq!(gate.recommended_action, LegacyCandidateAction::Review);
}

#[test]
fn ac_034_review_requires_one_explicit_non_review_decision_per_candidate() {
    let preview = LegacyImporter::preview(request()).unwrap();
    let review_decisions = decisions(&preview.candidates);
    let reviewed =
        LegacyImporter::finalize(&preview, review_decisions.clone(), timestamp()).unwrap();

    assert_eq!(reviewed.review.mode, LegacyImportMode::ReviewedImport);
    assert_eq!(reviewed.review.decisions.len(), preview.candidates.len());
    assert_eq!(reviewed.asserted_proposals.len(), 1);
    assert_eq!(
        reviewed.asserted_proposals[0].classification,
        ClaimClassification::Asserted
    );
    assert_eq!(
        reviewed.asserted_proposals[0].candidate_ref,
        preview.candidates[0].candidate_id
    );

    assert!(matches!(
        LegacyImporter::finalize(
            &preview,
            review_decisions[..review_decisions.len() - 1].to_vec(),
            timestamp()
        ),
        Err(LegacyImportError::MissingDecision { .. })
    ));
    let mut duplicate = review_decisions.clone();
    duplicate.push(review_decisions[0].clone());
    assert!(matches!(
        LegacyImporter::finalize(&preview, duplicate, timestamp()),
        Err(LegacyImportError::DuplicateDecision { .. })
    ));
    let mut unresolved = review_decisions;
    unresolved[0].action = LegacyCandidateAction::Review;
    assert!(matches!(
        LegacyImporter::finalize(&preview, unresolved, timestamp()),
        Err(LegacyImportError::UnresolvedDecision { .. })
    ));

    let mut foreign = decisions(&preview.candidates);
    foreign[0].candidate_ref = id("legacy:foreign");
    assert!(matches!(
        LegacyImporter::finalize(&preview, foreign, timestamp()),
        Err(LegacyImportError::UnknownCandidate { .. })
    ));
    let mut empty_reason = decisions(&preview.candidates);
    empty_reason[0].reason = "  ".to_owned();
    assert!(matches!(
        LegacyImporter::finalize(&preview, empty_reason, timestamp()),
        Err(LegacyImportError::EmptyReason { .. })
    ));
}

#[test]
fn archive_only_is_explicit_per_candidate_and_produces_no_asserted_proposals() {
    let preview = LegacyImporter::preview(request()).unwrap();
    let actor_ref = id("actor:archivist");
    let archived =
        LegacyImporter::archive_only(&preview, &actor_ref, "retain provenance", timestamp())
            .unwrap();
    assert_eq!(archived.mode, LegacyImportMode::ArchiveOnly);
    assert_eq!(archived.decisions.len(), preview.candidates.len());
    assert!(
        archived
            .decisions
            .iter()
            .all(|decision| decision.action == LegacyCandidateAction::ArchiveOnly)
    );
}

#[test]
fn ac_034_contract_has_no_trust_all_mode() {
    assert!(serde_json::from_str::<LegacyImportMode>("\"TRUST_ALL\"").is_err());
}

fn request() -> LegacyReviewRequest {
    LegacyReviewRequest {
        review_id: id("legacy-review:s7"),
        source_uri: ResourceRef::parse("project-ledger://legacy/BITACORA_PROYECTO.md").unwrap(),
        source_path: "BITACORA_PROYECTO.md".to_owned(),
        markdown: MARKDOWN.as_bytes().to_vec(),
        reviewed_at: timestamp(),
    }
}

fn decisions(
    candidates: &[project_ledger_domain::LegacyImportCandidate],
) -> Vec<LegacyImportDecision> {
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| LegacyImportDecision {
            candidate_ref: candidate.candidate_id.clone(),
            action: if index == 0 {
                LegacyCandidateAction::ImportAsAsserted
            } else {
                LegacyCandidateAction::Ignore
            },
            actor_ref: id("actor:reviewer"),
            reason: "reviewed candidate individually".to_owned(),
        })
        .collect()
}

fn timestamp() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T19:00:00-04:00").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
