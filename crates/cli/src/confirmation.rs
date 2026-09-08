//! Opt-in native historical confirmation. No recording operation is reachable here.
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    time::Instant,
};

use project_ledger_canonical_json::{sha256_bytes, sha256_digest, to_canonical_vec};
use project_ledger_domain::{CommandId, CommandResultStatus, ProjectId};
use project_ledger_event_store::StoreError;
use project_ledger_projection_engine::HistoryView;
use project_ledger_storage_sqlite::{CommandSnapshot, ConfirmationSnapshotError, SqliteEventStore};
use project_ledger_transport_stdio::{
    FrameLimits, JsonRpcErrorObject, JsonRpcHandler, dispatch_frame,
};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{CliError, ServeSinkArgs};

const PROFILE: &str = "gskills-record-confirmation/0.1.0";
const REQUEST_BYTES: usize = 32768;
const RESPONSE_BYTES: usize = 262_144;
const SCHEMAS: [(&str, &str); 4] = [
    (
        "request",
        include_str!("../../../integrations/gskills-confirmation/v0.1/request.schema.json"),
    ),
    (
        "response",
        include_str!("../../../integrations/gskills-confirmation/v0.1/response.schema.json"),
    ),
    (
        "manifest",
        include_str!("../../../integrations/gskills-confirmation/v0.1/manifest.schema.json"),
    ),
    (
        "policy",
        include_str!("../../../integrations/gskills-confirmation/v0.1/policy.schema.json"),
    ),
];

#[derive(Debug)]
struct Confirmation {
    database: std::path::PathBuf,
    policy: Value,
    observed_at: String,
    request_schema: jsonschema::Validator,
    response_schema: jsonschema::Validator,
}

pub(super) fn serve(args: &ServeSinkArgs) -> Result<String, CliError> {
    if !args.stdio || args.profile != PROFILE {
        return Err(CliError::input(
            "confirmation requires explicit stdio and exact profile",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(&args.policy)
        .map_err(CliError::input)?
        .take((REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(CliError::input)?;
    if bytes.len() > REQUEST_BYTES {
        return Err(CliError::input("confirmation policy exceeds input limit"));
    }
    let policy: Value = serde_json::from_slice(&bytes).map_err(CliError::input)?;
    let policy_schema = validator(3)?;
    policy_schema.validate(&policy).map_err(CliError::input)?;
    check_digest(&policy).map_err(|e| CliError::input(e.message))?;
    ProjectId::parse(&args.project_ref).map_err(CliError::input)?;
    if policy["native_project_ref"] != args.project_ref
        || policy["gskills_project_ref"] != args.gskills_project_ref
    {
        return Err(CliError::input(
            "confirmation CLI identity differs from policy",
        ));
    }
    utc(&args.observed_at).map_err(|e| CliError::input(e.message))?;
    let handler = Confirmation {
        database: args.database.clone(),
        policy,
        observed_at: args.observed_at.clone(),
        request_schema: validator(0)?,
        response_schema: validator(1)?,
    };
    let output = FrameLimits {
        max_header_bytes: 1024,
        max_content_bytes: RESPONSE_BYTES,
    };
    let mut reader = io::stdin().lock();
    let mut writer = io::stdout().lock();
    while let Some(frame) = read_input(&mut reader)? {
        let response = dispatch_frame(&frame, output, &handler).map_err(CliError::input)?;
        writer.write_all(&response).map_err(CliError::input)?;
        writer.flush().map_err(CliError::input)?;
    }
    Ok(String::new())
}

fn read_input(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, CliError> {
    let mut header = Vec::new();
    if reader
        .take(1025)
        .read_until(b'\n', &mut header)
        .map_err(CliError::input)?
        == 0
    {
        return Ok(None);
    }
    if header.len() > 1024 || !header.ends_with(b"\r\n") {
        return Err(CliError::input("invalid confirmation header"));
    }
    let mut terminator = Vec::new();
    reader
        .take(1025)
        .read_until(b'\n', &mut terminator)
        .map_err(CliError::input)?;
    if terminator != b"\r\n" || header.len() + terminator.len() > 1024 {
        return Err(CliError::input("invalid confirmation header terminator"));
    }
    let length = std::str::from_utf8(&header)
        .ok()
        .and_then(|v| v.strip_prefix("Content-Length: "))
        .and_then(|v| v.strip_suffix("\r\n"))
        .filter(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or_else(|| CliError::input("invalid confirmation Content-Length"))?;
    if length > REQUEST_BYTES {
        return Err(CliError::input("confirmation request exceeds byte limit"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(CliError::input)?;
    header.extend(terminator);
    header.extend(body);
    Ok(Some(header))
}

fn validator(index: usize) -> Result<jsonschema::Validator, CliError> {
    let schema: Value = serde_json::from_str(SCHEMAS[index].1).map_err(CliError::input)?;
    jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(CliError::input)
}

impl JsonRpcHandler for Confirmation {
    fn handle(&self, method: &str, params: &Value) -> Result<Value, JsonRpcErrorObject> {
        match method {
            "ledger.manifest" if params == &json!({}) => self.manifest(),
            "ledger.manifest" => Err(invalid("manifest requires empty params")),
            "ledger.command_confirmation" => {
                if params
                    .as_object()
                    .is_none_or(|o| o.len() != 1 || !o.contains_key("request"))
                {
                    return Err(invalid("confirmation requires exactly request"));
                }
                let request = &params["request"];
                self.request_schema
                    .validate(request)
                    .map_err(|_| invalid("invalid confirmation request schema"))?;
                check_digest(request)?;
                let requested = utc(text(request, "requested_at")?)?;
                let observed = utc(&self.observed_at)?;
                let age = (observed - requested).whole_nanoseconds();
                if !(-2_000_000_000..=30_000_000_000).contains(&age) {
                    return Err(invalid("confirmation request is outside freshness window"));
                }
                let started = Instant::now();
                let result = self.prove(request);
                let outcome = if started.elapsed().as_millis() > 5000 {
                    Err(("UNAVAILABLE", "READ_LIMIT_EXCEEDED"))
                } else {
                    result
                };
                let mut response = self.base(request);
                match outcome {
                    Ok(body) => response
                        .as_object_mut()
                        .ok_or_else(internal)?
                        .extend(body.as_object().ok_or_else(internal)?.clone()),
                    Err((status, code)) => {
                        response["status"] = json!(status);
                        response["error"] =
                            json!({"code":code,"retryable_read":status=="UNAVAILABLE"});
                    }
                }
                sign(&mut response)?;
                self.response_schema
                    .validate(&response)
                    .map_err(|_| internal())?;
                if to_canonical_vec(&response).map_err(|_| internal())?.len()
                    > RESPONSE_BYTES - REQUEST_BYTES
                {
                    return Err(internal());
                }
                Ok(response)
            }
            _ => Err(JsonRpcErrorObject {
                code: -32601,
                message: "method not found".into(),
                data: None,
            }),
        }
    }
}

type Failure = (&'static str, &'static str);
fn contradiction(code: &'static str) -> Failure {
    ("CONTRADICTION", code)
}

impl Confirmation {
    fn manifest(&self) -> Result<Value, JsonRpcErrorObject> {
        let hashes: serde_json::Map<String, Value> = SCHEMAS
            .iter()
            .map(|(name, schema)| {
                (
                    name.to_string(),
                    json!(
                        sha256_bytes(schema.as_bytes())
                            .to_string()
                            .trim_start_matches("sha256:")
                    ),
                )
            })
            .collect();
        let mut result = json!({"schema_version":"0.1.0","profile":PROFILE,"provider_id":"project-ledger","provider_version":"0.2.0",
            "transport":"gskills-stdio-jsonrpc-content-length/1.0.0","operations":["ledger.manifest","ledger.command_confirmation"],"read_only":true,
            "schema_sha256":hashes,"policy_digest":self.policy["digest"],"native_project_ref":self.policy["native_project_ref"],
            "gskills_project_ref":self.policy["gskills_project_ref"],"authority_binding_ref":self.authority(),"limits":self.policy["limits"],"created_at":self.policy["created_at"]});
        sign(&mut result)?;
        validator(2)
            .map_err(|_| internal())?
            .validate(&result)
            .map_err(|_| internal())?;
        Ok(result)
    }
    fn authority(&self) -> String {
        format!(
            "project-ledger://authority/confirmation/{}",
            self.policy["digest"].as_str().unwrap_or_default()
        )
    }
    fn base(&self, request: &Value) -> Value {
        let digest = request["digest"].as_str().unwrap_or_default();
        json!({"schema_version":"0.1.0","profile":PROFILE,"confirmation_ref":format!("project-ledger://confirmations/{digest}"),
            "request_ref":request["request_ref"],"request_digest":request["digest"],"observed_at":self.observed_at,
            "project_ref":request["project_ref"],"command_ref":request["command_ref"],"read_only":true,
            "provenance_ref":format!("project-ledger://provenance/confirmation/{digest}"),"authority_binding_ref":self.authority()})
    }
    fn prove(&self, request: &Value) -> Result<Value, Failure> {
        if request["project_ref"] != self.policy["native_project_ref"] {
            return Err(contradiction("IDENTITY_MISMATCH"));
        }
        let scope = &request["idempotency_scope"];
        if scope["actor_ref"] != self.policy["native_actor_ref"]
            || scope["trust_domain"] != self.policy["native_trust_domain"]
            || !self.policy["allowed_command_types"]
                .as_array()
                .is_some_and(|a| a.contains(&scope["command_type"]))
        {
            return Err(contradiction("SCOPE_MISMATCH"));
        }
        let project = ProjectId::parse(request["project_ref"].as_str().unwrap_or_default())
            .map_err(|_| contradiction("IDENTITY_MISMATCH"))?;
        let command_id = CommandId::parse(request["command_ref"].as_str().unwrap_or_default())
            .map_err(|_| contradiction("IDENTITY_MISMATCH"))?;
        let store =
            SqliteEventStore::open_read_only(&self.database).map_err(|error| match error {
                project_ledger_storage_sqlite::SqliteStoreError::MigrationIntegrity { .. } => {
                    contradiction("INTEGRITY_VIOLATION")
                }
                _ => ("UNAVAILABLE", "STORAGE_UNAVAILABLE"),
            })?;
        let snapshot = store
            .load_command_snapshot(&project, &command_id)
            .map_err(|error| match error {
                ConfirmationSnapshotError::ReadLimitExceeded => {
                    ("UNAVAILABLE", "READ_LIMIT_EXCEEDED")
                }
                ConfirmationSnapshotError::Store(StoreError::Persistence { .. }) => {
                    ("UNAVAILABLE", "STORAGE_UNAVAILABLE")
                }
                ConfirmationSnapshotError::Store(_) => contradiction("INTEGRITY_VIOLATION"),
            })?;
        proof_body(request, snapshot)
    }
}

fn proof_body(request: &Value, snapshot: CommandSnapshot) -> Result<Value, Failure> {
    let command = snapshot.command.ok_or(("NOT_FOUND", "COMMAND_NOT_FOUND"))?;
    if json!(command.project_ref) != request["project_ref"] {
        return Err(contradiction("IDENTITY_MISMATCH"));
    }
    let scope = &request["idempotency_scope"];
    if json!(command.command_type()) != scope["command_type"]
        || json!(command.actor.actor_id) != scope["actor_ref"]
        || json!(command.actor.trust_level) != scope["trust_domain"]
        || json!(command.idempotency_key) != scope["key"]
    {
        return Err(contradiction("SCOPE_MISMATCH"));
    }
    let result = snapshot.result.ok_or(("NOT_FOUND", "RESULT_NOT_FOUND"))?;
    let native_ref = format!(
        "project-ledger://command-results/{}",
        command.command_id.uuid()
    );
    if json!(result.digest) != request["expected_native_result_digest"]
        || json!(result.new_state_version) != request["expected_result_version"]
        || native_ref != request["expected_native_result_ref"]
        || result.command_ref != command.command_id
        || result.idempotency.replayed
        || result.status == CommandResultStatus::DuplicateReplay
    {
        return Err(contradiction("RESULT_MISMATCH"));
    }
    if result.emitted_event_refs.len() > 256 {
        return Err(("UNAVAILABLE", "READ_LIMIT_EXCEEDED"));
    }
    let cut = HistoryView::at(&snapshot.events, Some(result.new_state_version))
        .map_err(|_| contradiction("INVALID_CUT"))?
        .cut_identity();
    let current = HistoryView::at(&snapshot.events, None)
        .map_err(|_| contradiction("INVALID_CUT"))?
        .cut_identity();
    if result.previous_state_version > result.new_state_version {
        return Err(contradiction("EVENT_MISMATCH"));
    }
    if result.previous_state_version > 0 {
        HistoryView::at(&snapshot.events, Some(result.previous_state_version))
            .map_err(|_| contradiction("INVALID_CUT"))?;
    }
    let emitted: Vec<_> = snapshot
        .events
        .iter()
        .filter(|e| {
            e.stream_version > result.previous_state_version
                && e.stream_version <= result.new_state_version
        })
        .collect();
    let causal: Vec<_> = snapshot
        .events
        .iter()
        .filter(|e| e.causation.command_ref == command.command_id)
        .collect();
    if result.status == CommandResultStatus::Accepted {
        if result.previous_state_version != command.expected_state_version
            || emitted.is_empty()
            || causal != emitted
            || emitted.len() != result.emitted_event_refs.len()
            || emitted
                .iter()
                .zip(&result.emitted_event_refs)
                .any(|(e, r)| {
                    e.event_id != *r
                        || e.project_ref != command.project_ref
                        || e.causation.command_ref != command.command_id
                        || e.causation.command_digest != command.command_digest
                })
        {
            return Err(contradiction("EVENT_MISMATCH"));
        }
    } else if result.previous_state_version != result.new_state_version
        || !result.emitted_event_refs.is_empty()
        || !causal.is_empty()
        || result.resulting_checkpoint_ref.is_some()
    {
        return Err(contradiction("EVENT_MISMATCH"));
    }
    let checkpoint_refs: Vec<_> = emitted
        .iter()
        .filter_map(|e| match &e.event {
            project_ledger_domain::Event::CheckpointCreated(payload) => {
                Some(&payload.checkpoint.checkpoint_id)
            }
            _ => None,
        })
        .collect();
    if checkpoint_refs != result.resulting_checkpoint_ref.iter().collect::<Vec<_>>() {
        return Err(contradiction("EVENT_MISMATCH"));
    }
    let effect = json!({"project_ref":command.project_ref,"command_ref":command.command_id,"native_command_digest":command.command_digest,
        "native_result_digest":result.digest,"result_cut_head_digest":cut["head_digest"],"previous_state_version":result.previous_state_version,
        "new_state_version":result.new_state_version,"emitted_event_refs":result.emitted_event_refs});
    Ok(
        json!({"status":"VERIFIED","native_result":{"ref":native_ref,"digest":result.digest,"status":result.status,
        "native_command_digest":command.command_digest,"previous_state_version":result.previous_state_version,"new_state_version":result.new_state_version,
        "resulting_checkpoint_ref":result.resulting_checkpoint_ref,"emitted_event_refs":result.emitted_event_refs},"result_cut":cut,"current_cut":current,
        "effect_verification":{"status":"EXACT","event_count":emitted.len(),"digest":sha256_digest(&effect).map_err(|_|contradiction("INTEGRITY_VIOLATION"))?}}),
    )
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, JsonRpcErrorObject> {
    v[key].as_str().ok_or_else(|| invalid("missing string"))
}
fn check_digest(value: &Value) -> Result<(), JsonRpcErrorObject> {
    let mut unsigned = value.clone();
    unsigned
        .as_object_mut()
        .ok_or_else(|| invalid("object required"))?
        .remove("digest");
    if json!(sha256_digest(&unsigned).map_err(|_| invalid("invalid canonical JSON"))?)
        != value["digest"]
    {
        return Err(invalid("digest mismatch"));
    }
    Ok(())
}
fn sign(value: &mut Value) -> Result<(), JsonRpcErrorObject> {
    value.as_object_mut().ok_or_else(internal)?.remove("digest");
    value["digest"] = json!(sha256_digest(value).map_err(|_| internal())?);
    Ok(())
}
fn utc(value: &str) -> Result<OffsetDateTime, JsonRpcErrorObject> {
    if !value.ends_with('Z') {
        return Err(invalid("UTC Z timestamp required"));
    }
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid("invalid timestamp"))
}
fn invalid(message: &str) -> JsonRpcErrorObject {
    JsonRpcErrorObject {
        code: -32602,
        message: message.into(),
        data: None,
    }
}
fn internal() -> JsonRpcErrorObject {
    JsonRpcErrorObject {
        code: -32603,
        message: "confirmation unavailable".into(),
        data: None,
    }
}
