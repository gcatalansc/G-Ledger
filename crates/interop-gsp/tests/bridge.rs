mod gsp_support;

use project_ledger_domain::ResourceRef;
use project_ledger_event_store::EventStore;
use project_ledger_interop_gsp::{GspBridge, GspBridgeError, GspCompilerSidecar, GspSidecarError};
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::Value;

#[derive(Debug)]
struct FixtureSidecar {
    result: Value,
}

impl GspCompilerSidecar for FixtureSidecar {
    fn compile(&self, _query: &str, _evidence_bundle: &Value) -> Result<Value, GspSidecarError> {
        Ok(self.result.clone())
    }
}

#[derive(Debug)]
struct FailingSidecar;

impl GspCompilerSidecar for FailingSidecar {
    fn compile(&self, _query: &str, _evidence_bundle: &Value) -> Result<Value, GspSidecarError> {
        Err(GspSidecarError::Unavailable {
            message: "sidecar disabled".to_owned(),
        })
    }
}

#[test]
fn exact_result_is_digest_addressed_and_review_signals_remain_visible() {
    let corpus = gsp_support::corpus();
    let raw = gsp_support::with_review_signals(gsp_support::exact_result(&corpus));
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();

    let output = bridge
        .compile(
            &FixtureSidecar {
                result: raw.clone(),
            },
            &corpus,
        )
        .unwrap();

    assert_eq!(serde_json::to_value(&output.result).unwrap(), raw);
    assert_eq!(output.receipt.artifacts.len(), 6);
    assert_eq!(output.receipt.semantic_validation_status, "partial");
    assert!(output.receipt.conflict_visibility.contradictions_present);
    assert!(output.receipt.conflict_visibility.uncertainty_present);
    assert!(output.receipt.conflict_visibility.requires_human_review);
    assert!(!output.receipt.authority_elevation);
    assert_eq!(output.report.semantic_degradation_count, 0);
    assert_eq!(output.report.authority_elevation_count, 0);

    let payload = output
        .receipt
        .link_payload(ResourceRef::parse("artifact://gsp/compilation-receipt.json").unwrap());
    assert_eq!(payload.external_system, "GSP-Gemma Compiler");
    assert_eq!(payload.artifact_digest, output.receipt.receipt_digest);
    assert_eq!(
        payload.schema_ref.as_str(),
        "urn:project-ledger:schema:gsp-compilation-receipt:1.0.0"
    );
}

#[test]
fn sidecar_failure_is_isolated_from_a_real_ledger_store() {
    let corpus = gsp_support::corpus();
    let bridge = GspBridge::load(&gsp_support::repository_root()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteEventStore::open(&directory.path().join("ledger.db")).unwrap();
    let before = store.event_count(&corpus.manifest.project_ref).unwrap();

    assert!(matches!(
        bridge.compile(&FailingSidecar, &corpus),
        Err(GspBridgeError::Sidecar(GspSidecarError::Unavailable { .. }))
    ));

    let after = store.event_count(&corpus.manifest.project_ref).unwrap();
    assert_eq!(before, after);
    assert_eq!(after, 0);
}
