use std::collections::BTreeMap;

use project_ledger_canonical_json::Sha256Digest;
use project_ledger_domain::{
    EvidenceKind, EvidenceMetrics, EvidencePrivacy, EvidenceRecord, EvidenceResult, EvidenceSource,
    EvidenceSourceType, EvidenceStatus, LedgerTimestamp, PredicateAst, ProjectId,
    RepositorySnapshot, ResourceRef, RuleResult, ScalarValue, SchemaVersion, StableId,
};
use project_ledger_gate_engine::evaluate_predicate;

#[test]
fn evaluates_the_closed_predicate_ast_without_scripts_or_truthy_missing_fields() {
    let candidate = evidence("evidence:candidate", Some(snapshot("snapshot:one", 1)));
    let reference = evidence("evidence:reference", Some(snapshot("snapshot:one", 1)));
    let records = BTreeMap::from([
        (candidate.evidence_id.clone(), candidate.clone()),
        (reference.evidence_id.clone(), reference.clone()),
    ]);
    let predicate = PredicateAst::All {
        predicates: vec![
            PredicateAst::Equals {
                path: "result.exit_code".to_owned(),
                value: ScalarValue::Integer(0),
            },
            PredicateAst::LessOrEqual {
                path: "result.metrics.failed".to_owned(),
                value: ScalarValue::Unsigned(0),
            },
            PredicateAst::NotEquals {
                path: "result.exit_code".to_owned(),
                value: ScalarValue::Integer(1),
            },
            PredicateAst::LessThan {
                path: "result.duration_ms".to_owned(),
                value: ScalarValue::Unsigned(11),
            },
            PredicateAst::GreaterThan {
                path: "result.metrics.passed".to_owned(),
                value: ScalarValue::Unsigned(9),
            },
            PredicateAst::GreaterOrEqual {
                path: "result.metrics.passed".to_owned(),
                value: ScalarValue::Unsigned(10),
            },
            PredicateAst::Contains {
                path: "result.summary".to_owned(),
                value: ScalarValue::String("passed".to_owned()),
            },
            PredicateAst::StatusIn {
                path: "status".to_owned(),
                values: vec!["VERIFIED_SOURCE".to_owned()],
            },
            PredicateAst::CountAtLeast {
                path: "artifacts".to_owned(),
                value: 0,
            },
            PredicateAst::CountEquals {
                path: "artifacts".to_owned(),
                value: 0,
            },
            PredicateAst::FreshnessIn {
                path: "status".to_owned(),
                values: vec!["VERIFIED_SOURCE".to_owned()],
            },
            PredicateAst::SameSnapshotAs {
                evidence_ref: reference.evidence_id,
            },
            PredicateAst::Not {
                predicate: Box::new(PredicateAst::Missing {
                    path: "result.exit_code".to_owned(),
                }),
            },
        ],
    };

    assert_eq!(
        evaluate_predicate(&predicate, &candidate, &records),
        RuleResult::Pass
    );
    assert_eq!(
        evaluate_predicate(
            &PredicateAst::Exists {
                path: "result.metrics.unknown".to_owned(),
            },
            &candidate,
            &records,
        ),
        RuleResult::Missing
    );
    assert_eq!(
        evaluate_predicate(
            &PredicateAst::DigestEquals {
                path: "repository_binding.snapshot_digest".to_owned(),
                value: Sha256Digest::from([9_u8; 32]),
            },
            &candidate,
            &records,
        ),
        RuleResult::Fail
    );
}

#[test]
fn any_and_not_preserve_unknown_instead_of_converting_it_to_true() {
    let candidate = evidence("evidence:candidate", None);
    let records = BTreeMap::from([(candidate.evidence_id.clone(), candidate.clone())]);
    let missing = PredicateAst::Equals {
        path: "repository_binding.snapshot_ref".to_owned(),
        value: ScalarValue::String("snapshot:none".to_owned()),
    };

    assert_eq!(
        evaluate_predicate(
            &PredicateAst::Any {
                predicates: vec![
                    missing.clone(),
                    PredicateAst::Equals {
                        path: "result.exit_code".to_owned(),
                        value: ScalarValue::Integer(1),
                    },
                ],
            },
            &candidate,
            &records,
        ),
        RuleResult::Missing
    );
    assert_eq!(
        evaluate_predicate(
            &PredicateAst::Not {
                predicate: Box::new(missing),
            },
            &candidate,
            &records,
        ),
        RuleResult::Missing
    );
}

fn evidence(evidence_id: &str, repository_binding: Option<RepositorySnapshot>) -> EvidenceRecord {
    EvidenceRecord {
        schema_version: SchemaVersion::V0_2_0,
        evidence_id: id(evidence_id),
        project_ref: ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        kind: EvidenceKind::TestResult,
        status: EvidenceStatus::VerifiedSource,
        source: EvidenceSource {
            source_type: EvidenceSourceType::CommandExecution,
            tool: "cargo".to_owned(),
            command_display: Some("cargo test".to_owned()),
            working_directory_ref: Some(ResourceRef::parse("workspace://project/").unwrap()),
            runner_version: "project-ledger/0.2.0".to_owned(),
        },
        result: EvidenceResult {
            exit_code: Some(0),
            duration_ms: 10,
            metrics: EvidenceMetrics {
                passed: Some(10),
                failed: Some(0),
                skipped: Some(0),
                errors: Some(0),
            },
            summary: "10 passed".to_owned(),
        },
        repository_binding,
        artifacts: Vec::new(),
        privacy: EvidencePrivacy {
            raw_output_stored: false,
            redaction_applied: true,
        },
        observed_at: LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
        payload_digest: Sha256Digest::from([2_u8; 32]),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/evidence/test").unwrap(),
    }
}

fn snapshot(snapshot_ref: &str, digest: u8) -> RepositorySnapshot {
    RepositorySnapshot {
        repository_ref: id("repository:main"),
        snapshot_ref: id(snapshot_ref),
        snapshot_digest: Sha256Digest::from([digest; 32]),
    }
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
