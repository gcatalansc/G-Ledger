use std::{fs, path::PathBuf};

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    LedgerRadarProjectProjection, LedgerTimestamp, RadarIdentityMethod, RadarLinkStatus,
    RadarProjectLink, ResourceRef, SchemaVersion, StableId,
};
use project_ledger_interop_radar::{
    RadarEnvelopeRequest, RadarImportSnapshot, RadarMergeDecision, RadarOwnedFields,
    build_sync_envelope, evaluate_import,
};
use serde_json::{Value, json};

#[test]
fn pl_22_pl_23_only_exact_confirmed_identity_can_apply() {
    assert_eq!(
        serde_json::to_value(RadarIdentityMethod::UserConfirmed).unwrap(),
        "USER_CONFIRMED"
    );
    assert_eq!(
        serde_json::to_value(RadarIdentityMethod::ExactStableId).unwrap(),
        "EXACT_STABLE_ID"
    );
    assert_eq!(
        serde_json::to_value(RadarIdentityMethod::ExactRepositoryRemoteDigest).unwrap(),
        "EXACT_REPOSITORY_REMOTE_DIGEST"
    );
    assert_eq!(
        serde_json::to_value(RadarIdentityMethod::SignedMapping).unwrap(),
        "SIGNED_MAPPING"
    );
    assert_eq!(
        serde_json::to_value(RadarIdentityMethod::ImportedMappingReviewed).unwrap(),
        "IMPORTED_MAPPING_REVIEWED"
    );

    let project = projection();
    let envelope = envelope(project.clone());
    let snapshot = snapshot(None, None);
    let candidate = evaluate_import(&snapshot, &envelope, None).unwrap();
    assert_eq!(
        candidate.decision,
        RadarMergeDecision::CandidateReviewRequired
    );
    assert!(candidate.overlay.is_none());
    assert_eq!(
        candidate.candidate_ledger_refs,
        vec![project.source.project_ref.clone()]
    );

    let mut unconfirmed = link(&project, RadarLinkStatus::Candidate);
    refresh_link_digest(&mut unconfirmed);
    let rejected = evaluate_import(&snapshot, &envelope, Some(&unconfirmed)).unwrap();
    assert_eq!(rejected.decision, RadarMergeDecision::LinkRequired);
    assert!(rejected.overlay.is_none());
}

#[test]
fn pl_24_pl_27_merge_matrix_is_idempotent_non_destructive_and_preserves_radar_fields() {
    let project = projection();
    let envelope = envelope(project.clone());
    let confirmed = link(&project, RadarLinkStatus::Confirmed);
    let owned = owned_fields();

    let first = evaluate_import(&snapshot(None, None), &envelope, Some(&confirmed)).unwrap();
    assert_eq!(first.decision, RadarMergeDecision::ApplyOverlayUpdate);
    assert_eq!(first.overlay.as_ref(), Some(&project));
    assert_eq!(first.preserved_radar_fields, owned);
    assert_eq!(first.lifecycle_authority, "project-ledger");
    assert_eq!(first.provenance_chain, vec![project.provenance_ref.clone()]);

    let duplicate = evaluate_import(
        &snapshot(
            Some(project.source.state_version),
            Some(project.source.state_digest),
        ),
        &envelope,
        Some(&confirmed),
    )
    .unwrap();
    assert_eq!(duplicate.decision, RadarMergeDecision::DuplicateReplay);
    assert!(duplicate.overlay.is_none());

    let stale = evaluate_import(
        &snapshot(
            Some(project.source.state_version + 1),
            Some(Sha256Digest::from([9; 32])),
        ),
        &envelope,
        Some(&confirmed),
    )
    .unwrap();
    assert_eq!(stale.decision, RadarMergeDecision::StaleIgnored);
    assert!(stale.overlay.is_none());

    let divergence = evaluate_import(
        &snapshot(
            Some(project.source.state_version),
            Some(Sha256Digest::from([8; 32])),
        ),
        &envelope,
        Some(&confirmed),
    )
    .unwrap();
    assert_eq!(divergence.decision, RadarMergeDecision::SourceDivergence);
    assert!(divergence.overlay.is_none());
    assert_eq!(divergence.conflicts.len(), 1);
    assert_eq!(divergence.conflicts[0].code, "SOURCE_DIVERGENCE");
    assert_eq!(divergence.preserved_radar_fields, owned);

    let empty = build_sync_envelope(Vec::new(), &envelope_request()).unwrap();
    let omitted = evaluate_import(&snapshot(None, None), &empty, Some(&confirmed)).unwrap();
    assert_eq!(omitted.decision, RadarMergeDecision::NoAction);
    assert!(omitted.overlay.is_none());
    assert_eq!(omitted.preserved_radar_fields, owned);
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn projection() -> LedgerRadarProjectProjection {
    let path = repository_root()
        .join("schemas/v0.2/fixtures/positive/radar-ledger-radar-project-projection.json");
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    refresh_value_digest(&mut value);
    serde_json::from_value(value).unwrap()
}

fn envelope(project: LedgerRadarProjectProjection) -> project_ledger_domain::RadarSyncEnvelope {
    build_sync_envelope(vec![project], &envelope_request()).unwrap()
}

fn envelope_request() -> RadarEnvelopeRequest {
    RadarEnvelopeRequest {
        sync_envelope_id: id("radar-sync:importer-test"),
        source_instance_ref: ResourceRef::parse("project-ledger://instances/test").unwrap(),
        exported_at: at(),
    }
}

fn link(project: &LedgerRadarProjectProjection, status: RadarLinkStatus) -> RadarProjectLink {
    let mut link = RadarProjectLink {
        schema_version: SchemaVersion::V0_1_1,
        link_id: id("radar-link:test"),
        ledger_project_ref: project.source.project_ref.clone(),
        radar_project_ref: ResourceRef::parse("project-radar://projects/01").unwrap(),
        status,
        method: RadarIdentityMethod::UserConfirmed,
        actor_ref: id("human:owner"),
        created_at: at(),
        digest: Sha256Digest::from([0; 32]),
    };
    refresh_link_digest(&mut link);
    link
}

fn snapshot(
    source_state_version: Option<u64>,
    source_state_digest: Option<Sha256Digest>,
) -> RadarImportSnapshot {
    RadarImportSnapshot {
        radar_project_ref: ResourceRef::parse("project-radar://projects/01").unwrap(),
        radar_title: "Project Ledger".to_owned(),
        source_project_ref: source_state_version
            .map(|_| ResourceRef::parse("project-ledger://projects/01").unwrap()),
        source_state_version,
        source_state_digest,
        external_continuity: None,
        radar_owned_fields: owned_fields(),
    }
}

fn owned_fields() -> RadarOwnedFields {
    RadarOwnedFields {
        priority: json!("P1"),
        tags: json!(["local", "owner"]),
        notes: json!({"private": "keep"}),
        ideas: json!([{"id": 1}]),
        chats: json!([{"id": "chat-1"}]),
        tasks: json!([{"id": "task-1", "done": false}]),
        artifacts: json!([{"id": "artifact-1"}]),
        presentation: json!({"color": "violet"}),
    }
}

fn refresh_link_digest(link: &mut RadarProjectLink) {
    let mut value = serde_json::to_value(&*link).unwrap();
    value.as_object_mut().unwrap().remove("digest");
    link.digest = sha256_digest(&value).unwrap();
}

fn refresh_value_digest(value: &mut Value) {
    value["digest"] = Value::Null;
    value.as_object_mut().unwrap().remove("digest");
    value["digest"] = json!(sha256_digest(&*value).unwrap().to_string());
}

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn at() -> LedgerTimestamp {
    LedgerTimestamp::parse("2026-08-23T11:00:00-04:00").unwrap()
}
