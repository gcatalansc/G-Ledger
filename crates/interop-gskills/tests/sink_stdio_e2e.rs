mod sink_support;

use project_ledger_interop_gskills::{GSkillsRecordSink, GSkillsRecordSinkJsonRpcHandler};
use project_ledger_transport_stdio::{
    FrameError, FrameLimits, JsonRpcHandler, JsonRpcRequest, JsonRpcResponse, decode_frame,
    dispatch_frame, encode_frame,
};
use serde_json::{Value, json};

use sink_support::{ProcessingHost, policy, work_progress_command};

#[test]
fn framed_mutable_methods_match_direct_calls_and_keep_read_paths_side_effect_free() {
    let host = ProcessingHost::new();
    let command = work_progress_command(host.current_state());
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let handler = GSkillsRecordSinkJsonRpcHandler::new(&sink);
    let baseline_events = host.event_count();

    let manifest = rpc(&handler, 1, "ledger.manifest", json!({}));
    assert_eq!(manifest, sink.manifest().unwrap());
    assert_eq!(
        manifest["record_sink_capabilities"]
            .as_array()
            .unwrap()
            .len(),
        5
    );

    let validated = rpc(
        &handler,
        2,
        "ledger.command_validate",
        json!({"command": command}),
    );
    assert_eq!(validated["status"], "VALID");
    assert_eq!(host.execute_calls(), 0);
    assert_eq!(host.event_count(), baseline_events);

    let submitted = rpc(
        &handler,
        3,
        "ledger.command_submit",
        json!({"command": command}),
    );
    assert_eq!(submitted["status"], "ACCEPTED");
    assert_eq!(host.event_count(), baseline_events + 1);

    let status = rpc(
        &handler,
        4,
        "ledger.command_status",
        json!({"command": command}),
    );
    let direct = sink.status(&command).unwrap();
    assert_eq!(status, direct);
    assert_eq!(host.execute_calls(), 1);

    let reconciled = rpc(
        &handler,
        5,
        "ledger.command_reconcile",
        json!({"command": command}),
    );
    assert_eq!(reconciled, direct);
    assert_eq!(host.execute_calls(), 1);
}

#[test]
fn malformed_unknown_and_rejected_requests_are_isolated_from_later_frames() {
    let host = ProcessingHost::new();
    let sink = GSkillsRecordSink::new(&host, policy()).unwrap();
    let handler = GSkillsRecordSinkJsonRpcHandler::new(&sink);
    let limits = FrameLimits::default();

    let truncated = b"Content-Length: 20\r\n\r\n{}";
    assert!(matches!(
        dispatch_frame(truncated, limits, &handler),
        Err(FrameError::Truncated { .. })
    ));

    let unknown = rpc_response(&handler, 2, "ledger.phase_complete", json!({}));
    assert_eq!(unknown.error.unwrap().code, -32601);

    let invalid = rpc_response(&handler, 3, "ledger.command_submit", json!({}));
    assert_eq!(invalid.error.unwrap().code, -32602);
    assert_eq!(host.execute_calls(), 0);

    let recovered = rpc_response(&handler, 4, "ledger.manifest", json!({}));
    assert!(recovered.error.is_none());
    assert_eq!(recovered.result.unwrap()["provider_id"], "project-ledger");
    assert_eq!(host.execute_calls(), 0);
}

fn rpc<H: JsonRpcHandler>(handler: &H, id: u64, method: &str, params: Value) -> Value {
    rpc_response(handler, id, method, params).result.unwrap()
}

fn rpc_response<H: JsonRpcHandler>(
    handler: &H,
    id: u64,
    method: &str,
    params: Value,
) -> JsonRpcResponse {
    let limits = FrameLimits::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!(id),
        method: method.to_owned(),
        params,
    };
    let frame = encode_frame(&request, limits).unwrap();
    let response = dispatch_frame(&frame, limits, handler).unwrap();
    decode_frame(&response, limits).unwrap()
}
