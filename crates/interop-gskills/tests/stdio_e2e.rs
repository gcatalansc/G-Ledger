mod support;

use project_ledger_interop_gskills::{GSkillsJsonRpcHandler, GSkillsReadProvider};
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcHandler, JsonRpcRequest, JsonRpcResponse, decode_frame, dispatch_frame,
    encode_frame,
};
use serde_json::{Value, json};

#[test]
fn framed_profile_matches_direct_manifest_health_and_projection_calls() {
    let provider = support::provider();
    let handler = GSkillsJsonRpcHandler::new(&provider);
    let limits = FrameLimits::default();

    for (id, method, params, expected) in [
        (
            1,
            "ledger.manifest",
            json!({}),
            provider.manifest().unwrap(),
        ),
        (2, "ledger.health", json!({}), provider.health().unwrap()),
        (
            3,
            "ledger.project_continuity",
            support::current_request(&provider),
            direct_projection_bundle(&provider, &support::current_request(&provider)),
        ),
    ] {
        let response = framed_call(&handler, limits, id, method, params);
        assert_eq!(response.result, Some(expected));
        assert!(response.error.is_none());
    }
}

#[test]
fn mutable_methods_and_bad_requests_are_rejected_without_poisoning_next_call() {
    let provider = support::provider();
    let handler = GSkillsJsonRpcHandler::new(&provider);
    let limits = FrameLimits::default();

    let mutable = framed_call(&handler, limits, 1, "ledger.record", json!({}));
    assert_eq!(mutable.error.unwrap().code, -32601);

    let mut invalid = support::current_request(&provider);
    invalid["write_capability"] = json!("RECORD_SUBMIT");
    support::refresh_digest(&mut invalid);
    let rejected = framed_call(&handler, limits, 2, "ledger.project_continuity", invalid);
    assert_eq!(rejected.error.unwrap().code, -32602);

    let recovered = framed_call(
        &handler,
        limits,
        3,
        "ledger.project_continuity",
        support::current_request(&provider),
    );
    assert!(recovered.result.is_some());
    assert!(recovered.error.is_none());
}

fn direct_projection_bundle<P: GSkillsReadProvider>(provider: &P, request: &Value) -> Value {
    let result = provider.project(request).unwrap();
    json!({
        "envelope": result.envelope,
        "freshness": result.freshness,
        "projection": result.payload,
    })
}

fn framed_call<H: JsonRpcHandler>(
    handler: &H,
    limits: FrameLimits,
    id: u64,
    method: &str,
    params: Value,
) -> JsonRpcResponse {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!(id),
        method: method.to_owned(),
        params,
    };
    let response =
        dispatch_frame(&encode_frame(&request, limits).unwrap(), limits, handler).unwrap();
    decode_frame(&response, limits).unwrap()
}
