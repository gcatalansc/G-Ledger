use project_ledger_transport_stdio::{JsonRpcErrorObject, JsonRpcHandler};
use serde_json::{Value, json};

use crate::{GSkillsReadProvider, ProviderError};

/// Strict JSON-RPC method handler for the three read-only S9 operations.
#[derive(Debug)]
pub struct GSkillsJsonRpcHandler<'a, P> {
    provider: &'a P,
}

impl<'a, P> GSkillsJsonRpcHandler<'a, P> {
    pub const fn new(provider: &'a P) -> Self {
        Self { provider }
    }
}

impl<P: GSkillsReadProvider> JsonRpcHandler for GSkillsJsonRpcHandler<'_, P> {
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject> {
        match method {
            "ledger.manifest" => {
                require_empty_params(params)?;
                self.provider
                    .manifest()
                    .map_err(|error| internal_error(&error))
            }
            "ledger.health" => {
                require_empty_params(params)?;
                self.provider
                    .health()
                    .map_err(|error| internal_error(&error))
            }
            "ledger.project_continuity" => {
                let result = self
                    .provider
                    .project(params)
                    .map_err(|error| invalid_params(&error))?;
                Ok(json!({
                    "envelope": result.envelope,
                    "freshness": result.freshness,
                    "projection": result.payload,
                }))
            }
            _ => Err(JsonRpcErrorObject {
                code: -32601,
                message: "method not found".to_owned(),
                data: Some(json!({
                    "allowed_methods": [
                        "ledger.manifest",
                        "ledger.health",
                        "ledger.project_continuity"
                    ]
                })),
            }),
        }
    }
}

fn require_empty_params(params: &Value) -> Result<(), JsonRpcErrorObject> {
    if params == &json!({}) {
        return Ok(());
    }
    Err(JsonRpcErrorObject {
        code: -32602,
        message: "manifest and health require empty object params".to_owned(),
        data: None,
    })
}

fn invalid_params(error: &ProviderError) -> JsonRpcErrorObject {
    JsonRpcErrorObject {
        code: -32602,
        message: "invalid exact G-Skills projection request".to_owned(),
        data: Some(json!({"provider_error": error.to_string()})),
    }
}

fn internal_error(error: &ProviderError) -> JsonRpcErrorObject {
    JsonRpcErrorObject {
        code: -32603,
        message: "exact G-Skills provider unavailable".to_owned(),
        data: Some(json!({"provider_error": error.to_string()})),
    }
}
