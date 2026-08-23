use project_ledger_transport_stdio::{
    FrameError, FrameLimits, JsonRpcErrorObject, JsonRpcHandler, JsonRpcRequest, JsonRpcResponse,
    decode_frame, dispatch_frame, encode_frame,
};
use serde_json::{Value, json};

#[derive(Debug)]
struct ManifestHandler;

impl JsonRpcHandler for ManifestHandler {
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject> {
        if method == "ledger.manifest" && params == &json!({}) {
            return Ok(json!({"provider_id":"project-ledger","version":"0.2.0"}));
        }
        Err(JsonRpcErrorObject {
            code: -32601,
            message: "method not found".to_owned(),
            data: None,
        })
    }
}

#[test]
fn ac_003_content_length_roundtrip_preserves_direct_json_semantics() {
    let limits = FrameLimits {
        max_header_bytes: 256,
        max_content_bytes: 1024,
    };
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!("request:01"),
        method: "ledger.manifest".to_owned(),
        params: json!({}),
    };
    let frame = encode_frame(&request, limits).unwrap();
    let literal_payload =
        br#"{"id":"request:01","jsonrpc":"2.0","method":"ledger.manifest","params":{}}"#;
    let expected = [
        format!("Content-Length: {}\r\n\r\n", literal_payload.len()).into_bytes(),
        literal_payload.to_vec(),
    ]
    .concat();
    assert_eq!(frame, expected);
    assert_eq!(
        decode_frame::<JsonRpcRequest>(&frame, limits).unwrap(),
        request
    );

    let response_frame = dispatch_frame(&frame, limits, &ManifestHandler).unwrap();
    let response = decode_frame::<JsonRpcResponse>(&response_frame, limits).unwrap();
    assert_eq!(
        response.result,
        Some(
            ManifestHandler
                .handle(&request.method, &request.params)
                .unwrap()
        )
    );
    assert!(response.error.is_none());
}

#[test]
fn malicious_frames_fail_closed_before_dispatch() {
    let limits = FrameLimits {
        max_header_bytes: 64,
        max_content_bytes: 64,
    };
    let cases: Vec<Vec<u8>> = vec![
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
        b"Content-Length: -1\r\n\r\n".to_vec(),
        b"content-length: 2\r\n\r\n{}".to_vec(),
        b"Content-Length: 65\r\n\r\n{}".to_vec(),
        b"Content-Length: 5\r\n\r\n{}".to_vec(),
        b"Content-Length: 2\r\n\r\n{}x".to_vec(),
        b"Content-Length: 2\r\n\r\n\xff\xff".to_vec(),
        b"Content-Length: 1\r\n\r\n{".to_vec(),
    ];
    for bytes in cases {
        assert!(decode_frame::<JsonRpcRequest>(&bytes, limits).is_err());
    }
}

#[test]
fn invalid_json_rpc_version_and_handler_failure_are_typed_and_isolated() {
    let limits = FrameLimits::default();
    let invalid = json!({"jsonrpc":"1.0","id":1,"method":"ledger.manifest","params":{}});
    let invalid_frame = encode_frame(&invalid, limits).unwrap();
    assert!(matches!(
        dispatch_frame(&invalid_frame, limits, &ManifestHandler),
        Err(FrameError::InvalidJsonRpc { .. })
    ));

    let missing = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!(2),
        method: "ledger.missing".to_owned(),
        params: json!({}),
    };
    let missing_frame = dispatch_frame(
        &encode_frame(&missing, limits).unwrap(),
        limits,
        &ManifestHandler,
    )
    .unwrap();
    let missing_response = decode_frame::<JsonRpcResponse>(&missing_frame, limits).unwrap();
    assert!(missing_response.result.is_none());
    assert_eq!(missing_response.error.unwrap().code, -32601);

    let valid = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: json!(3),
        method: "ledger.manifest".to_owned(),
        params: json!({}),
    };
    let valid_response = dispatch_frame(
        &encode_frame(&valid, limits).unwrap(),
        limits,
        &ManifestHandler,
    )
    .unwrap();
    assert!(
        decode_frame::<JsonRpcResponse>(&valid_response, limits)
            .unwrap()
            .result
            .is_some()
    );
}
