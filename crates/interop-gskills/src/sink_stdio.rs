use project_ledger_transport_stdio::{JsonRpcErrorObject, JsonRpcHandler};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{GSkillsRecordSink, RecordSinkError, RecordSinkHost};

/// Strict JSON-RPC handler for the exact S10 mutable profile.
#[derive(Debug)]
pub struct GSkillsRecordSinkJsonRpcHandler<'sink, 'host, Host: RecordSinkHost + ?Sized> {
    sink: &'sink GSkillsRecordSink<'host, Host>,
}

impl<'sink, 'host, Host: RecordSinkHost + ?Sized>
    GSkillsRecordSinkJsonRpcHandler<'sink, 'host, Host>
{
    pub const fn new(sink: &'sink GSkillsRecordSink<'host, Host>) -> Self {
        Self { sink }
    }
}

impl<Host: RecordSinkHost + ?Sized> JsonRpcHandler
    for GSkillsRecordSinkJsonRpcHandler<'_, '_, Host>
{
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject> {
        match method {
            "ledger.manifest" => {
                require_empty_params(params)?;
                self.sink.manifest().map_err(|error| rpc_error(&error))
            }
            "ledger.command_validate" => {
                let command = command_param(params)?;
                let prepared = self
                    .sink
                    .validate(command)
                    .map_err(|error| rpc_error(&error))?;
                Ok(json!({
                    "status": "VALID",
                    "command_ref": prepared.external_command["external_record_command_id"],
                    "external_command_digest": prepared.external_command_digest,
                    "native_command_type": serde_json::to_value(prepared.command.command_type())
                        .map_err(|error| JsonRpcErrorObject {
                            code: -32603,
                            message: "native command type encoding failed".to_owned(),
                            data: Some(json!({"error": error.to_string()})),
                        })?,
                    "expected_state_version": prepared.expected_state_version
                }))
            }
            "ledger.command_submit" => self
                .sink
                .submit(command_param(params)?)
                .map_err(|error| rpc_error(&error)),
            "ledger.command_status" => self
                .sink
                .status(command_param(params)?)
                .map_err(|error| rpc_error(&error)),
            "ledger.command_reconcile" => self
                .sink
                .reconcile(command_param(params)?)
                .map_err(|error| rpc_error(&error)),
            _ => Err(JsonRpcErrorObject {
                code: -32601,
                message: "method not found".to_owned(),
                data: Some(json!({
                    "allowed_methods": [
                        "ledger.manifest",
                        "ledger.command_validate",
                        "ledger.command_submit",
                        "ledger.command_status",
                        "ledger.command_reconcile"
                    ]
                })),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandParams {
    #[serde(rename = "command")]
    _command: Value,
}

fn command_param(params: &Value) -> Result<&Value, JsonRpcErrorObject> {
    serde_json::from_value::<CommandParams>(params.clone()).map_err(|error| {
        JsonRpcErrorObject {
            code: -32602,
            message: "method requires exactly one command parameter".to_owned(),
            data: Some(json!({"error": error.to_string()})),
        }
    })?;
    params.get("command").ok_or(JsonRpcErrorObject {
        code: -32602,
        message: "command parameter could not be retained".to_owned(),
        data: None,
    })
}

fn require_empty_params(params: &Value) -> Result<(), JsonRpcErrorObject> {
    if params == &json!({}) {
        return Ok(());
    }
    Err(JsonRpcErrorObject {
        code: -32602,
        message: "manifest requires empty object params".to_owned(),
        data: None,
    })
}

fn rpc_error(error: &RecordSinkError) -> JsonRpcErrorObject {
    let internal = matches!(
        error,
        RecordSinkError::Host(_)
            | RecordSinkError::NativeIntegrity(_)
            | RecordSinkError::DurableResultMismatch
    );
    JsonRpcErrorObject {
        code: if internal { -32603 } else { -32602 },
        message: if internal {
            "exact G-Skills RecordSink unavailable"
        } else {
            "invalid exact G-Skills record command"
        }
        .to_owned(),
        data: Some(json!({"record_sink_error": error.to_string()})),
    }
}
