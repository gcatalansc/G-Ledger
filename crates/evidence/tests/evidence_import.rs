use project_ledger_canonical_json::{Sha256Digest, sha256_bytes, sha256_digest};
use project_ledger_domain::{
    EvidenceKind, EvidenceMetrics, EvidencePrivacy, EvidenceRecord, EvidenceResult, EvidenceSource,
    EvidenceSourceType, EvidenceStatus, LedgerTimestamp, ProjectId, ResourceRef, SchemaVersion,
    StableId,
};
use project_ledger_evidence::{EvidenceImportContext, EvidenceImporter, EvidenceImporterError};

#[test]
fn unsigned_local_import_validates_digest_and_never_promotes_verified_source() {
    let original = evidence();
    let bytes = serde_json::to_vec(&original).unwrap();
    let context = EvidenceImportContext {
        expected_project_ref: project_id(),
        artifact_uri: ResourceRef::parse("file:///evidence/result.json").unwrap(),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/import/result").unwrap(),
        importer_version: "project-ledger/0.2.0".to_owned(),
    };

    let imported = EvidenceImporter::import_unsigned_json(&bytes, &context).unwrap();

    assert_eq!(imported.status, EvidenceStatus::Observed);
    assert_eq!(
        imported.source.source_type,
        EvidenceSourceType::ImportedArtifact
    );
    assert_eq!(imported.source.tool, "project-ledger-evidence-import");
    assert_eq!(imported.provenance_ref, context.provenance_ref);
    assert_eq!(imported.artifacts.len(), 1);
    assert_eq!(imported.artifacts[0].digest, sha256_bytes(&bytes));
    assert!(!imported.privacy.raw_output_stored);
    assert_eq!(evidence_digest(&imported), imported.payload_digest);
}

#[test]
fn import_rejects_tampered_digest_unknown_fields_and_cross_project_evidence() {
    let original = evidence();
    let context = EvidenceImportContext {
        expected_project_ref: project_id(),
        artifact_uri: ResourceRef::parse("file:///evidence/result.json").unwrap(),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/import/result").unwrap(),
        importer_version: "project-ledger/0.2.0".to_owned(),
    };
    let mut tampered = serde_json::to_value(&original).unwrap();
    tampered["result"]["summary"] = serde_json::json!("tampered");
    assert!(matches!(
        EvidenceImporter::import_unsigned_json(&serde_json::to_vec(&tampered).unwrap(), &context),
        Err(EvidenceImporterError::DigestMismatch)
    ));

    let mut unknown = serde_json::to_value(&original).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(matches!(
        EvidenceImporter::import_unsigned_json(&serde_json::to_vec(&unknown).unwrap(), &context),
        Err(EvidenceImporterError::Contract(_))
    ));

    let mut wrong_project = original;
    wrong_project.project_ref =
        ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df6e").unwrap();
    wrong_project.payload_digest = evidence_digest(&wrong_project);
    assert!(matches!(
        EvidenceImporter::import_unsigned_json(
            &serde_json::to_vec(&wrong_project).unwrap(),
            &context,
        ),
        Err(EvidenceImporterError::ProjectMismatch)
    ));
}

#[test]
fn imported_secrets_are_rejected_before_persistence_without_echoing_values() {
    let mut record = evidence();
    let canary = "CANARY_NEVER_PERSIST_9258";
    record.result.summary = format!("token={canary}");
    record.payload_digest = evidence_digest(&record);
    let context = EvidenceImportContext {
        expected_project_ref: project_id(),
        artifact_uri: ResourceRef::parse("urn:test:artifact").unwrap(),
        provenance_ref: ResourceRef::parse("urn:test:import").unwrap(),
        importer_version: "test/1".into(),
    };
    let result =
        EvidenceImporter::import_unsigned_json(&serde_json::to_vec(&record).unwrap(), &context);
    assert!(
        result.is_err(),
        "import must not persist supported secret patterns"
    );
    assert!(!result.unwrap_err().to_string().contains(canary));
}

fn evidence() -> EvidenceRecord {
    let mut record = EvidenceRecord {
        schema_version: SchemaVersion::V0_2_0,
        evidence_id: id("evidence:imported"),
        project_ref: project_id(),
        kind: EvidenceKind::TestResult,
        status: EvidenceStatus::VerifiedSource,
        source: EvidenceSource {
            source_type: EvidenceSourceType::ExternalSystem,
            tool: "external-ci".to_owned(),
            command_display: None,
            working_directory_ref: None,
            runner_version: "external/1".to_owned(),
        },
        result: EvidenceResult {
            exit_code: Some(0),
            duration_ms: 50,
            metrics: EvidenceMetrics {
                passed: Some(4),
                failed: Some(0),
                skipped: Some(0),
                errors: Some(0),
            },
            summary: "4 passed".to_owned(),
        },
        repository_binding: None,
        artifacts: Vec::new(),
        privacy: EvidencePrivacy {
            raw_output_stored: false,
            redaction_applied: true,
        },
        observed_at: LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
        payload_digest: Sha256Digest::from([0_u8; 32]),
        provenance_ref: ResourceRef::parse("external-ci://runs/1").unwrap(),
    };
    record.payload_digest = evidence_digest(&record);
    record
}

fn evidence_digest(record: &EvidenceRecord) -> Sha256Digest {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove("payload_digest");
    sha256_digest(&value).unwrap()
}

fn project_id() -> ProjectId {
    ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap()
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}
