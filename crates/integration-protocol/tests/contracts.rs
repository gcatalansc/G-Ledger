use project_ledger_canonical_json::{Sha256Digest, sha256_digest, to_canonical_vec};
use project_ledger_domain::{
    FreshnessStatus, IdentityMapping, IdentityMappingStatus, LedgerTimestamp, LossItem,
    LossSeverity, ProjectionEnvelope, ProjectionRequest, ProviderHealthStatus, ResourceRef,
    StableId,
};
use project_ledger_integration_protocol::{
    IdentityQuery, IdentityResolution, LossReportDraft, ProjectionEnvelopeDraft, ProtocolError,
    UpstreamLockStatus, build_default_manifest, build_health, build_loss_report,
    build_projection_envelope, inspect_upstream_lock, resolve_identity, validate_health_integrity,
    validate_loss_report_integrity, validate_manifest_integrity, validate_projection_envelope,
};
use serde::Serialize;
use serde_json::{Value, json};

const ZERO_DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn manifest_health_and_projection_are_integrity_bound_and_bounded() {
    let manifest = build_default_manifest(Vec::new()).unwrap();
    validate_manifest_integrity(&manifest).unwrap();
    assert_eq!(manifest.digest, digest_without(&manifest, "digest"));
    assert!(!manifest.storage.shared_database_required);

    let checked_at = LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap();
    let health = build_health(
        &manifest,
        checked_at,
        ProviderHealthStatus::Healthy,
        Vec::new(),
    )
    .unwrap();
    validate_health_integrity(&health, &manifest).unwrap();
    assert_eq!(health.digest, digest_without(&health, "digest"));

    let request: ProjectionRequest = fixture("projection-request.json");
    let fixture_envelope: ProjectionEnvelope = fixture("projection-envelope.json");
    let fixture_state_digest = fixture_envelope.state_digest;
    let envelope = build_projection_envelope(ProjectionEnvelopeDraft {
        request: &request,
        manifest: &manifest,
        source_product: "project-ledger",
        source_version: "0.2.0",
        state_version: 4,
        state_digest: fixture_state_digest,
        payload_schema_ref: ResourceRef::parse("urn:project-ledger:schema:resume-projection:0.2.0")
            .unwrap(),
        coverage: vec!["continuity".to_owned()],
        omissions: Vec::new(),
        provenance_ref: ResourceRef::parse("project-ledger://provenance/generic-projection-01")
            .unwrap(),
        payload: fixture_envelope.payload,
    })
    .unwrap();
    validate_projection_envelope(&envelope, &request, &manifest).unwrap();
    assert_eq!(envelope.freshness, FreshnessStatus::Current);
    assert_eq!(
        envelope.payload_digest,
        sha256_digest(&envelope.payload).unwrap()
    );
    assert!(
        u64::try_from(to_canonical_vec(&envelope.payload).unwrap().len()).unwrap()
            <= request.budget.max_bytes
    );

    let mut stale_request = request.clone();
    stale_request.expected_source_version = Some(3);
    let stale = build_projection_envelope(ProjectionEnvelopeDraft {
        request: &stale_request,
        manifest: &manifest,
        source_product: "project-ledger",
        source_version: "0.2.0",
        state_version: 4,
        state_digest: fixture_state_digest,
        payload_schema_ref: envelope.payload_schema_ref.clone(),
        coverage: envelope.coverage.clone(),
        omissions: Vec::new(),
        provenance_ref: envelope.provenance_ref.clone(),
        payload: envelope.payload.clone(),
    })
    .unwrap();
    assert_eq!(stale.freshness, FreshnessStatus::Stale);

    let mut tiny = request;
    tiny.budget.max_bytes = 1;
    let rejected = build_projection_envelope(ProjectionEnvelopeDraft {
        request: &tiny,
        manifest: &manifest,
        source_product: "project-ledger",
        source_version: "0.2.0",
        state_version: 4,
        state_digest: fixture_state_digest,
        payload_schema_ref: envelope.payload_schema_ref,
        coverage: envelope.coverage,
        omissions: Vec::new(),
        provenance_ref: envelope.provenance_ref,
        payload: envelope.payload,
    });
    assert!(matches!(
        rejected,
        Err(ProtocolError::BudgetExceeded { .. })
    ));
}

#[test]
fn exact_confirmed_identity_never_falls_back_to_name_and_ambiguity_fails_closed() {
    let mut mapping: IdentityMapping = fixture("identity-mapping.json");
    let query = IdentityQuery {
        external_system: "project-radar".to_owned(),
        external_ref: ResourceRef::parse("project-radar://projects/01").unwrap(),
    };
    assert_eq!(
        resolve_identity(&query, &[mapping.clone()]),
        IdentityResolution::Resolved(mapping.local_ref.clone())
    );

    mapping.status = IdentityMappingStatus::Candidate;
    assert_eq!(
        resolve_identity(&query, &[mapping.clone()]),
        IdentityResolution::Unresolved
    );

    mapping.status = IdentityMappingStatus::Confirmed;
    let mut sibling = mapping.clone();
    sibling.mapping_id = StableId::parse("mapping:02").unwrap();
    sibling.local_ref = ResourceRef::parse("project-ledger://projects/02").unwrap();
    assert_eq!(
        resolve_identity(&query, &[mapping, sibling]),
        IdentityResolution::Ambiguous
    );

    let name_only = IdentityQuery {
        external_system: "project-radar".to_owned(),
        external_ref: ResourceRef::parse("project-radar://names/project-ledger").unwrap(),
    };
    assert_eq!(
        resolve_identity(&name_only, &[]),
        IdentityResolution::Unresolved
    );
}

#[test]
fn upstream_locks_preserve_evidence_state_and_reject_certification_overclaim() {
    for (relative, expected_status) in [
        (
            "../../../integrations/gskills/upstream-lock.json",
            UpstreamLockStatus::Complete,
        ),
        (
            "../../../integrations/gsp/upstream-lock.json",
            UpstreamLockStatus::Pending,
        ),
        (
            "../../../integrations/project-radar/upstream-lock.json",
            UpstreamLockStatus::Pending,
        ),
    ] {
        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(workspace_file(relative)).unwrap())
                .unwrap();
        let inspection = inspect_upstream_lock(&value).unwrap();
        assert_eq!(inspection.status, expected_status);
        assert_eq!(
            inspection.missing_evidence.is_empty(),
            expected_status == UpstreamLockStatus::Complete
        );
        assert!(!inspection.certified_compatibility);
    }

    let complete = json!({
        "schema_version": "1.0.0",
        "integration": "example",
        "product_version": "1.2.3",
        "source_spec_sha256": ZERO_DIGEST,
        "schema_registry_sha256": ZERO_DIGEST,
        "fixture_manifest_sha256": ZERO_DIGEST,
        "compatibility_range": {
            "protocol": "project-ledger-integration",
            "minimum": "0.2.0",
            "maximum": "0.2.0"
        },
        "certified_compatibility": false
    });
    let inspection = inspect_upstream_lock(&complete).unwrap();
    assert_eq!(inspection.status, UpstreamLockStatus::Complete);
    assert!(inspection.missing_evidence.is_empty());

    let mut blocked_profile = complete.clone();
    blocked_profile["provider_field_mapping_status"] =
        Value::String("BLOCKED_UPSTREAM_SCHEMA_CONFLICT".to_owned());
    blocked_profile["provider_loss_report_status"] = Value::String("MATERIAL_LOSS".to_owned());
    blocked_profile["provider_conformance_status"] =
        Value::String("BLOCKED_BY_MATERIAL_LOSS".to_owned());
    let inspection = inspect_upstream_lock(&blocked_profile).unwrap();
    assert_eq!(inspection.status, UpstreamLockStatus::Pending);
    assert_eq!(
        inspection.missing_evidence,
        vec![
            "provider_conformance",
            "provider_field_mapping",
            "provider_loss_report"
        ]
    );

    let mut overclaim = complete;
    overclaim["schema_registry_sha256"] = Value::Null;
    overclaim["certified_compatibility"] = Value::Bool(true);
    assert!(matches!(
        inspect_upstream_lock(&overclaim),
        Err(ProtocolError::CompatibilityOverclaim)
    ));
}

#[test]
fn loss_report_counts_material_items_and_detects_tampering() {
    let created_at = LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap();
    let mut report = build_loss_report(LossReportDraft {
        loss_report_id: StableId::parse("loss:s8").unwrap(),
        source_schema_ref: ResourceRef::parse("urn:project-ledger:schema:project-state:0.2.0")
            .unwrap(),
        target_schema_ref: ResourceRef::parse("urn:project-ledger:schema:resume-projection:0.2.0")
            .unwrap(),
        items: vec![
            LossItem {
                field_path: "event_history".to_owned(),
                severity: LossSeverity::Informational,
                reason: "bounded projection".to_owned(),
            },
            LossItem {
                field_path: "private.raw_output".to_owned(),
                severity: LossSeverity::Material,
                reason: "privacy redaction".to_owned(),
            },
        ],
        created_at,
    })
    .unwrap();
    assert_eq!(report.material_loss_count, 1);
    validate_loss_report_integrity(&report).unwrap();
    report.material_loss_count = 0;
    assert!(matches!(
        validate_loss_report_integrity(&report),
        Err(ProtocolError::IntegrityMismatch { .. })
    ));
}

fn fixture<T>(name: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let path = workspace_file(&format!("schemas/v0.2/fixtures/positive/{name}"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn workspace_file(relative: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.join(relative.trim_start_matches("../../../"))
}

fn digest_without<T: Serialize>(value: &T, field: &str) -> Sha256Digest {
    let mut value = serde_json::to_value(value).unwrap();
    value.as_object_mut().unwrap().remove(field);
    sha256_digest(&value).unwrap()
}
