use std::{cell::Cell, thread, time::Duration};

use project_ledger_canonical_json::{Sha256Digest, sha256_digest};
use project_ledger_domain::{
    IntegrationManifest, LedgerTimestamp, ProjectionEnvelope, ProjectionPayload, ProjectionRequest,
    ProviderHealthReport, ProviderHealthStatus, ResourceRef,
};
use project_ledger_integration_protocol::{
    ConformanceError, ObservableState, ProjectionEnvelopeDraft, ProviderFailure,
    ReadConformanceSuite, ReadOnlyProvider, build_default_manifest, build_health,
    build_projection_envelope, malicious_read_requests, run_read_conformance,
};

#[derive(Debug)]
struct FixtureProvider {
    manifest: IntegrationManifest,
    health: ProviderHealthReport,
    payload: ProjectionPayload,
    state_version: u64,
    state_digest: Sha256Digest,
    event_count: Cell<u64>,
    mutate_on_read: bool,
    misbind_observable_state: bool,
    leak_secret: bool,
    delay: Duration,
}

impl FixtureProvider {
    fn new() -> Self {
        let manifest = build_default_manifest(Vec::new()).unwrap();
        let health = build_health(
            &manifest,
            LedgerTimestamp::parse("2026-08-23T12:00:00-04:00").unwrap(),
            ProviderHealthStatus::Healthy,
            Vec::new(),
        )
        .unwrap();
        let envelope: ProjectionEnvelope = fixture("projection-envelope.json");
        Self {
            manifest,
            health,
            payload: envelope.payload,
            state_version: envelope.state_version,
            state_digest: envelope.state_digest,
            event_count: Cell::new(4),
            mutate_on_read: false,
            misbind_observable_state: false,
            leak_secret: false,
            delay: Duration::ZERO,
        }
    }
}

impl ReadOnlyProvider for FixtureProvider {
    fn observable_state(&self) -> Result<ObservableState, ProviderFailure> {
        Ok(ObservableState {
            state_version: self.state_version,
            event_count: self.event_count.get(),
            state_digest: self.state_digest,
        })
    }

    fn manifest(&self) -> Result<IntegrationManifest, ProviderFailure> {
        Ok(self.manifest.clone())
    }

    fn health(&self) -> Result<ProviderHealthReport, ProviderFailure> {
        Ok(self.health.clone())
    }

    fn project(&self, request: &ProjectionRequest) -> Result<ProjectionEnvelope, ProviderFailure> {
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }
        if request.repository_identity_digest.is_some() {
            return Err(ProviderFailure::new(
                "IDENTITY_AMBIGUOUS",
                "repository identity resolves to multiple projects",
            ));
        }
        if self.mutate_on_read {
            self.event_count.set(self.event_count.get() + 1);
        }
        let response_state_digest = if self.misbind_observable_state {
            Sha256Digest::from([9_u8; 32])
        } else {
            self.state_digest
        };
        let mut payload = self.payload.clone();
        if let ProjectionPayload::ProjectContinuity(resume) = &mut payload {
            resume.state_digest = response_state_digest;
        }
        let mut envelope = build_projection_envelope(ProjectionEnvelopeDraft {
            request,
            manifest: &self.manifest,
            source_product: "project-ledger",
            source_version: "0.2.0",
            state_version: self.state_version,
            state_digest: response_state_digest,
            payload_schema_ref: ResourceRef::parse(
                "urn:project-ledger:schema:resume-projection:0.2.0",
            )
            .unwrap(),
            coverage: vec!["continuity".to_owned()],
            omissions: Vec::new(),
            provenance_ref: ResourceRef::parse(
                "project-ledger://provenance/conformance-projection",
            )
            .unwrap(),
            payload,
        })
        .map_err(|error| ProviderFailure::new("REQUEST_REJECTED", error.to_string()))?;
        if self.leak_secret {
            if let ProjectionPayload::ProjectContinuity(resume) = &mut envelope.payload {
                resume.last_material_progress_summary = Some("token=unredacted-secret".to_owned());
            }
            envelope.payload_digest = sha256_digest(&envelope.payload).unwrap();
        }
        Ok(envelope)
    }
}

#[test]
fn ac_043_read_conformance_runs_one_hundred_projections_with_zero_side_effects() {
    let provider = FixtureProvider::new();
    let current: ProjectionRequest = fixture("projection-request.json");
    let mut stale = current.clone();
    stale.expected_source_version = Some(3);
    let malicious = malicious_read_requests(&current);
    let suite = ReadConformanceSuite {
        current_request: current,
        stale_request: stale,
        malicious_requests: malicious.clone(),
        projection_repetitions: 100,
        max_call_duration: Duration::from_millis(100),
    };

    let report = run_read_conformance(&provider, &suite).unwrap();
    assert_eq!(report.projection_calls, 102);
    assert_eq!(
        report.rejected_malicious,
        u32::try_from(malicious.len()).unwrap()
    );
    assert_eq!(report.observable_mutations, 0);
    assert!(report.checks.iter().all(|check| check.passed));
    assert_eq!(provider.event_count.get(), 4);
}

#[test]
fn observable_mutation_is_detected_on_the_first_read() {
    let mut provider = FixtureProvider::new();
    provider.mutate_on_read = true;
    let suite = suite();
    assert!(matches!(
        run_read_conformance(&provider, &suite),
        Err(ConformanceError::ObservableMutation { .. })
    ));
}

#[test]
fn response_state_must_match_the_observable_provider_state() {
    let mut provider = FixtureProvider::new();
    provider.misbind_observable_state = true;
    let suite = suite();
    assert!(matches!(
        run_read_conformance(&provider, &suite),
        Err(ConformanceError::InvalidResponse { .. })
    ));
}

#[test]
fn unredacted_secret_in_a_typed_projection_is_rejected() {
    let mut provider = FixtureProvider::new();
    provider.leak_secret = true;
    let suite = suite();
    assert!(matches!(
        run_read_conformance(&provider, &suite),
        Err(ConformanceError::InvalidResponse { .. })
    ));
}

#[test]
fn provider_call_exceeding_the_declared_timeout_fails_conformance() {
    let mut provider = FixtureProvider::new();
    provider.delay = Duration::from_millis(5);
    let mut suite = suite();
    suite.max_call_duration = Duration::from_millis(1);
    assert!(matches!(
        run_read_conformance(&provider, &suite),
        Err(ConformanceError::Timeout { .. })
    ));
}

fn suite() -> ReadConformanceSuite {
    let current: ProjectionRequest = fixture("projection-request.json");
    let mut stale = current.clone();
    stale.expected_source_version = Some(3);
    ReadConformanceSuite {
        malicious_requests: malicious_read_requests(&current),
        current_request: current,
        stale_request: stale,
        projection_repetitions: 1,
        max_call_duration: Duration::from_millis(100),
    }
}

fn fixture<T>(name: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join(format!("schemas/v0.2/fixtures/positive/{name}"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
