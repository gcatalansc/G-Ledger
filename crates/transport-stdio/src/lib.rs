//! Bounded JSON-RPC 2.0 framing for local stdio integration processes.

use std::str;

use project_ledger_canonical_json::{CanonicalJsonError, to_canonical_vec};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;

const HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";

/// Resource bounds applied before a JSON-RPC request reaches a handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLimits {
    pub max_header_bytes: usize,
    pub max_content_bytes: usize,
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_header_bytes: 8 * 1024,
            max_content_bytes: 1024 * 1024,
        }
    }
}

/// Strict JSON-RPC 2.0 request object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Value,
}

/// JSON-RPC error returned by a method handler.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

/// Strict JSON-RPC response with exactly one of result or error populated.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

/// A generic method dispatcher with no storage or product dependency.
pub trait JsonRpcHandler {
    /// Handles one already-framed and validated JSON-RPC method call.
    ///
    /// # Errors
    ///
    /// Returns a typed JSON-RPC error object when the method or its parameters
    /// are unsupported by the concrete adapter.
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject>;
}

/// Failures in strict framing, JSON parsing or JSON-RPC validation.
#[derive(Debug, Error)]
pub enum FrameError {
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error("invalid Content-Length frame: {message}")]
    InvalidHeader { message: &'static str },
    #[error("frame exceeds configured {kind} bound: limit {limit}, actual {actual}")]
    TooLarge {
        kind: &'static str,
        limit: usize,
        actual: usize,
    },
    #[error("frame payload is truncated: expected {expected}, actual {actual}")]
    Truncated { expected: usize, actual: usize },
    #[error("frame contains {count} trailing bytes")]
    TrailingBytes { count: usize },
    #[error("frame payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("frame payload is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid JSON-RPC request: {message}")]
    InvalidJsonRpc { message: &'static str },
}

/// Encodes canonical JSON behind one exact `Content-Length` header.
///
/// # Errors
///
/// Returns [`FrameError`] when canonical serialization fails or either bound
/// would be exceeded.
pub fn encode_frame<T: Serialize>(value: &T, limits: FrameLimits) -> Result<Vec<u8>, FrameError> {
    let payload = to_canonical_vec(value)?;
    if payload.len() > limits.max_content_bytes {
        return Err(FrameError::TooLarge {
            kind: "content",
            limit: limits.max_content_bytes,
            actual: payload.len(),
        });
    }
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    if header.len() > limits.max_header_bytes {
        return Err(FrameError::TooLarge {
            kind: "header",
            limit: limits.max_header_bytes,
            actual: header.len(),
        });
    }
    let mut frame = Vec::with_capacity(header.len() + payload.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Decodes exactly one bounded frame and rejects trailing bytes.
///
/// # Errors
///
/// Returns [`FrameError`] for invalid/oversized framing, non-UTF-8 content,
/// malformed JSON or a payload that does not match `T`.
pub fn decode_frame<T: DeserializeOwned>(
    frame: &[u8],
    limits: FrameLimits,
) -> Result<T, FrameError> {
    let terminator = find_subslice(frame, HEADER_TERMINATOR).ok_or(FrameError::InvalidHeader {
        message: "missing CRLF header terminator",
    })?;
    let body_offset = terminator + HEADER_TERMINATOR.len();
    if body_offset > limits.max_header_bytes {
        return Err(FrameError::TooLarge {
            kind: "header",
            limit: limits.max_header_bytes,
            actual: body_offset,
        });
    }
    let header = str::from_utf8(&frame[..terminator]).map_err(|_| FrameError::InvalidHeader {
        message: "header is not ASCII",
    })?;
    if !header.is_ascii() {
        return Err(FrameError::InvalidHeader {
            message: "header is not ASCII",
        });
    }
    let value = header
        .strip_prefix("Content-Length: ")
        .ok_or(FrameError::InvalidHeader {
            message: "expected exact Content-Length header",
        })?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(FrameError::InvalidHeader {
            message: "Content-Length is not an unsigned decimal integer",
        });
    }
    let content_length = value
        .parse::<usize>()
        .map_err(|_| FrameError::InvalidHeader {
            message: "Content-Length cannot be represented",
        })?;
    if content_length > limits.max_content_bytes {
        return Err(FrameError::TooLarge {
            kind: "content",
            limit: limits.max_content_bytes,
            actual: content_length,
        });
    }
    let actual = frame.len().saturating_sub(body_offset);
    if actual < content_length {
        return Err(FrameError::Truncated {
            expected: content_length,
            actual,
        });
    }
    if actual > content_length {
        return Err(FrameError::TrailingBytes {
            count: actual - content_length,
        });
    }
    let payload = &frame[body_offset..];
    str::from_utf8(payload).map_err(|_| FrameError::InvalidUtf8)?;
    Ok(serde_json::from_slice(payload)?)
}

/// Dispatches one valid JSON-RPC request and returns one framed response.
///
/// # Errors
///
/// Returns [`FrameError`] when request framing/JSON-RPC validation fails or the
/// resulting response cannot be encoded within the configured bounds.
pub fn dispatch_frame<H: JsonRpcHandler>(
    frame: &[u8],
    limits: FrameLimits,
    handler: &H,
) -> Result<Vec<u8>, FrameError> {
    let request: JsonRpcRequest = decode_frame(frame, limits)?;
    if request.jsonrpc != "2.0" || request.method.is_empty() {
        return Err(FrameError::InvalidJsonRpc {
            message: "version must be 2.0 and method must be non-empty",
        });
    }
    let response = match handler.handle(&request.method, &request.params) {
        Ok(result) => JsonRpcResponse {
            jsonrpc: "2.0".to_owned(),
            id: request.id,
            result: Some(result),
            error: None,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_owned(),
            id: request.id,
            result: None,
            error: Some(error),
        },
    };
    encode_frame(&response, limits)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
