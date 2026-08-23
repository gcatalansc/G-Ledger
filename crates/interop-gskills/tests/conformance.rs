mod support;

use std::{cell::Cell, time::Duration};

use project_ledger_interop_gskills::{
    ConformanceError, GSkillsObservableState, GSkillsProjectionResult, GSkillsProvider,
    GSkillsReadConformanceSuite, GSkillsReadProvider, ProviderError, malicious_gskills_requests,
    run_gskills_read_conformance,
};
use serde_json::Value;

#[test]
fn exact_read_conformance_is_deterministic_bounded_and_side_effect_free() {
    let provider = support::provider();
    let current = support::current_request(&provider);
    let suite = GSkillsReadConformanceSuite {
        stale_request: support::stale_request(&provider),
        malicious_requests: malicious_gskills_requests(&current),
        current_request: current,
        projection_repetitions: 100,
        max_call_duration: Duration::from_secs(1),
    };

    let report = run_gskills_read_conformance(&provider, &suite).unwrap();

    assert_eq!(report.projection_calls, 102);
    assert_eq!(
        report.rejected_malicious,
        u32::try_from(suite.malicious_requests.len()).unwrap()
    );
    assert_eq!(report.observable_mutations, 0);
    assert_eq!(report.material_losses, 0);
    assert!(report.checks.iter().all(|check| check.passed));
}

#[test]
fn harness_detects_observable_mutation() {
    let provider = support::provider();
    let current = support::current_request(&provider);
    let mutating = MutatingProvider {
        inner: provider,
        mutated: Cell::new(false),
    };
    let suite = GSkillsReadConformanceSuite {
        stale_request: support::stale_request(&mutating.inner),
        malicious_requests: malicious_gskills_requests(&current),
        current_request: current,
        projection_repetitions: 1,
        max_call_duration: Duration::from_secs(1),
    };

    assert!(matches!(
        run_gskills_read_conformance(&mutating, &suite),
        Err(ConformanceError::ObservableMutation { .. })
    ));
}

struct MutatingProvider {
    inner: GSkillsProvider,
    mutated: Cell<bool>,
}

impl GSkillsReadProvider for MutatingProvider {
    fn observable_state(&self) -> GSkillsObservableState {
        let mut state = self.inner.observable_state();
        if self.mutated.get() {
            state.event_count += 1;
        }
        state
    }

    fn manifest(&self) -> Result<Value, ProviderError> {
        self.inner.manifest()
    }

    fn health(&self) -> Result<Value, ProviderError> {
        self.inner.health()
    }

    fn project(&self, request: &Value) -> Result<GSkillsProjectionResult, ProviderError> {
        let result = self.inner.project(request)?;
        self.mutated.set(true);
        Ok(result)
    }
}
