use super::WriteArgs;
use crate::errors::CliError;
use project_ledger_canonical_json::{sha256_bytes, sha256_digest};
use project_ledger_domain::LedgerTimestamp;
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn read(path: &Path, kind: &str) -> Result<Value, CliError> {
    let bytes = fs::read(path).map_err(CliError::input)?;
    let value: Value = if path.extension().is_some_and(|s| s == "yaml" || s == "yml") {
        serde_yaml::from_slice(&bytes).map_err(CliError::input)?
    } else {
        serde_json::from_slice(&bytes).map_err(CliError::input)?
    };
    let wrapper = match kind {
        "DEFINE_ROADMAP" | "REVISE_ROADMAP" => "roadmap",
        "DEFINE_GATE" => "gate",
        "PROPOSE_DECISION" => "decision",
        "OPEN_BLOCKER" | "UPDATE_BLOCKER" => "blocker",
        "OPEN_RISK" | "UPDATE_RISK" => "risk",
        "SET_NEXT_ACTION" => "action",
        "ADD_ARTIFACT_REFERENCE" => "artifact",
        "REGISTER_AUTHORITY_DOCUMENT" => "document",
        "BIND_REPOSITORY" => "binding",
        "RECORD_WORK_PROGRESS" => "work_record",
        "RECORD_CLAIM" => "claim",
        "RECORD_UNKNOWN" => "unknown",
        "START_SESSION" => "session",
        _ => return Ok(value),
    };
    if value.get(wrapper).is_some_and(Value::is_object) {
        Ok(value)
    } else {
        Ok(json!({wrapper: value}))
    }
}

#[allow(clippy::too_many_lines)] // Explicit closed command-to-payload translations, without hidden defaults.
pub fn build(args: &WriteArgs, kind: &str) -> Result<Value, CliError> {
    let generated_id = |prefix: &str| {
        args.record_id.clone().unwrap_or_else(|| {
            format!(
                "{prefix}:{}",
                &sha256_bytes(
                    format!(
                        "{}:{}:{}",
                        args.idempotency_key, args.actor_id, args.as_human
                    )
                    .as_bytes()
                )
                .to_string()[7..]
            )
        })
    };
    let normalized_at = LedgerTimestamp::parse(&args.observed_at)
        .map_err(CliError::input)?
        .to_string();
    let at = &normalized_at;
    let provenance = "urn:project-ledger:cli:local";
    match kind {
        "PAUSE_PROJECT" | "RESUME_PROJECT" | "ARCHIVE_PROJECT" => Ok(json!({"reason":args.reason})),
        "RECORD_NOTE" => Ok(json!({"kind":args.kind,"text":args.text,"scope_ref":args.phase})),
        "RECORD_WORK_PROGRESS" => {
            let mut record = json!({"schema_version":"0.2.0","work_record_id":generated_id("work"),"summary":args.summary,"phase_ref":args.phase,"task_ref":null,"files_or_artifacts":args.files,"commit_refs":[],"claim_refs":[],"evidence_refs":[],"actor_ref":args.actor_id,"influence":null,"occurred_at":at,"recorded_at":at,"provenance_ref":"urn:project-ledger:cli:local"});
            record["digest"] = json!(sha256_digest(&record).map_err(CliError::input)?);
            Ok(json!({"work_record":record}))
        }
        "REGISTER_AUTHORITY_DOCUMENT" | "SUPERSEDE_AUTHORITY_DOCUMENT" => {
            let uri = uri(args
                .uri
                .as_deref()
                .or(args.with.as_deref())
                .ok_or_else(|| CliError::input("--uri or --with required"))?)?;
            if kind == "SUPERSEDE_AUTHORITY_DOCUMENT" {
                let (_, p) = args.selection.load()?;
                let matches: Vec<_> = p
                    .state()
                    .authority_documents
                    .iter()
                    .filter(|d| {
                        d.uri.as_str() == uri
                            && d.supersedes_ref
                                .as_ref()
                                .map(project_ledger_domain::StableId::as_str)
                                == args.id.as_deref()
                            && args
                                .record_id
                                .as_ref()
                                .is_none_or(|id| d.document_id.as_str() == id)
                    })
                    .collect();
                if matches.len() != 1 {
                    return Err(CliError::domain(
                        "register exactly one replacement with --supersedes before superseding",
                    ));
                }
                return Ok(json!({"document_ref":args.id,"replacement":matches[0]}));
            }
            Ok(
                json!({"document":{"schema_version":"0.2.0","document_id":generated_id("authority"),"uri":uri,"digest":args.digest,"version":"1","status":"CURRENT","authority_rank":1,"effective_from":at,"supersedes_ref":args.supersedes}}),
            )
        }
        "REFRESH_AUTHORITY_DOCUMENT_DIGEST" => {
            let (_, p) = args.selection.load()?;
            let doc = p
                .state()
                .authority_documents
                .iter()
                .find(|d| Some(d.document_id.as_str()) == args.id.as_deref())
                .ok_or_else(|| CliError::domain("unknown authority document"))?;
            let observation = match local_file(doc.uri.as_str())
                .and_then(|path| fs::read(path).map_err(CliError::input))
            {
                Ok(bytes) => {
                    let digest = sha256_bytes(&bytes);
                    json!({"outcome":if digest == doc.digest {"MATCH"} else {"MISMATCH"},"observed_digest":digest})
                }
                Err(_) => {
                    json!({"outcome":"UNAVAILABLE","reason":"authority URI is not a readable local file"})
                }
            };
            Ok(json!({"document_ref":args.id,"observation":observation}))
        }
        "ADD_ARTIFACT_REFERENCE" => Ok(
            json!({"artifact":{"schema_version":"0.2.0","artifact_ref_id":generated_id("artifact"),"name":args.title.as_deref().unwrap_or("CLI artifact"),"kind":args.kind.as_deref().unwrap_or("OTHER"),"uri":args.uri,"digest":args.digest,"version":null,"sensitivity":"INTERNAL","source_ref":null,"created_at":at,"provenance_ref":provenance}}),
        ),
        "RECORD_CLAIM" => {
            let subject = args
                .subject
                .clone()
                .unwrap_or(args.selection.resolve()?.1.to_string());
            Ok(
                json!({"claim":{"schema_version":"0.2.0","claim_id":generated_id("claim"),"statement":args.statement,"classification":args.classification.as_deref().unwrap_or("ASSERTED"),"subject_refs":[subject],"evidence_refs":[],"authority_ref":null}}),
            )
        }
        "RECORD_UNKNOWN" => signed(
            "unknown",
            json!({"schema_version":"0.2.0","unknown_id":generated_id("unknown"),"subject_kind":"PROJECT","subject_ref":args.subject,"field_path":"unspecified","statement":args.statement,"criticality":args.criticality.as_deref().unwrap_or("MATERIAL"),"status":"OPEN","blocks":[],"required_evidence":[],"evidence_refs":[],"opened_at":at,"resolved_at":null,"resolution_ref":null,"provenance_ref":provenance}),
        ),
        "OPEN_BLOCKER" | "OPEN_RISK" => {
            let (_, project) = args.selection.resolve()?;
            let mut record = json!({"schema_version":"0.2.0","title":args.title,"description":args.summary.as_deref().unwrap_or(""),"scope_kind":"PROJECT","scope_ref":project,"severity":args.severity.as_deref().unwrap_or("MEDIUM"),"status":"OPEN","owner_ref":null,"evidence_refs":[],"opened_at":at,"updated_at":at,"provenance_ref":provenance});
            if kind == "OPEN_BLOCKER" {
                record["blocker_id"] = json!(generated_id("blocker"));
                record["source_kind"] = json!("LOCAL_RECORD");
                record["source_ref"] = Value::Null;
                record["resolved_at"] = Value::Null;
                record["resolution_summary"] = Value::Null;
                signed("blocker", record)
            } else {
                record["risk_id"] = json!(generated_id("risk"));
                record["likelihood_class"] = json!("UNKNOWN");
                record["trigger_conditions"] = json!([]);
                record["mitigation_actions"] = json!([]);
                record["closed_at"] = Value::Null;
                signed("risk", record)
            }
        }
        "SET_NEXT_ACTION" => signed(
            "action",
            json!({"schema_version":"0.2.0","next_action_id":generated_id("action"),"title":args.title,"description":args.summary.as_deref().unwrap_or(""),"phase_ref":args.phase,"gate_ref":null,"classification":args.classification.as_deref().unwrap_or("PROPOSED"),"authority_ref":null,"precondition_refs":[],"blocked_by_refs":[],"owner_ref":null,"created_at":at,"updated_at":at,"completed_at":null,"provenance_ref":provenance}),
        ),
        "START_PHASE" | "REQUEST_PHASE_COMPLETION" => Ok(json!({"phase_ref":args.id})),
        "REOPEN_PHASE" => {
            Ok(json!({"phase_ref":args.id,"reason":args.reason,"authority_ref":args.authority}))
        }
        "REQUEST_GATE_EVALUATION" => Ok(json!({"gate_ref":args.id})),
        "REQUEST_GATE_REVALIDATION" => {
            let signal = args
                .signal
                .as_ref()
                .map(|path| {
                    fs::read(path).map_err(CliError::input)?;
                    uri(&path.to_string_lossy())
                })
                .transpose()?;
            Ok(json!({"gate_ref":args.id,"signal_ref":signal}))
        }
        "UPDATE_CLAIM_STATUS" => {
            Ok(json!({"claim_ref":args.id,"classification":args.set,"evidence_refs":args.evidence}))
        }
        "RESOLVE_UNKNOWN" => Ok(json!({"unknown_ref":args.id,"resolution_ref":args.resolution})),
        "LINK_EXTERNAL_EVIDENCE" => Ok(
            json!({"external_system":args.system,"artifact_uri":args.artifact_ref,"artifact_digest":args.digest,"schema_ref":args.schema_ref}),
        ),
        "UNLINK_EXTERNAL_EVIDENCE" => Ok(json!({"evidence_ref":args.id,"reason":args.reason})),
        "ACCEPT_DECISION" => Ok(json!({"decision_ref":args.id,"authority_ref":args.authority})),
        "SUPERSEDE_DECISION" => Ok(
            json!({"decision_ref":args.id,"replacement":read(Path::new(args.with.as_deref().ok_or_else(|| CliError::input("--with required"))?), kind)?}),
        ),
        "RESOLVE_BLOCKER" => Ok(
            json!({"blocker_ref":args.id,"evidence_refs":args.evidence,"resolution_summary":args.summary.as_deref().unwrap_or("Resolved by explicit local action")}),
        ),
        "MITIGATE_RISK" => {
            Ok(json!({"risk_ref":args.id,"note":args.note,"evidence_refs":args.evidence}))
        }
        "BIND_REPOSITORY" => {
            let root = args
                .root
                .as_ref()
                .ok_or_else(|| CliError::input("--root required"))?;
            if !root.is_dir() {
                return Err(CliError::input(
                    "repository root is not an existing directory",
                ));
            }
            signed(
                "binding",
                json!({"schema_version":"0.2.0","repository_binding_id":generated_id("binding"),"repository_ref":generated_id("repository"),"workspace_root_ref":uri(&root.to_string_lossy())?,"identity_method":"USER_CONFIRMED","remote_identity_digest":args.remote.as_ref().map(|r| sha256_bytes(r.as_bytes())),"current_snapshot_ref":null,"current_snapshot_digest":null,"status":"CURRENT","confirmed_by":args.actor_id,"created_at":at,"updated_at":at,"provenance_ref":provenance}),
            )
        }
        "REFRESH_REPOSITORY_BINDING" => {
            let (_, p) = args.selection.load()?;
            let id = project_ledger_domain::StableId::parse(
                args.id
                    .as_deref()
                    .ok_or_else(|| CliError::input("binding id required"))?,
            )
            .map_err(CliError::input)?;
            let binding = p
                .repository_binding(&id)
                .ok_or_else(|| CliError::domain("unknown binding"))?;
            Ok(
                json!({"binding_ref":id,"snapshot":{"repository_ref":binding.repository_ref,"snapshot_ref":args.snapshot_ref,"snapshot_digest":args.snapshot_digest}}),
            )
        }
        "REVOKE_REPOSITORY_BINDING" => Ok(json!({"binding_ref":args.id,"reason":args.reason})),
        "WAIVE_GATE" => {
            if args.approved_by.as_deref() != Some(&args.actor_id) {
                return Err(CliError::new(
                    9,
                    "POLICY_DENIED",
                    "--approved-by must equal the explicit actor",
                ));
            }
            let (_, p) = args.selection.load()?;
            let id = project_ledger_domain::StableId::parse(
                args.id
                    .as_deref()
                    .ok_or_else(|| CliError::input("gate id required"))?,
            )
            .map_err(CliError::input)?;
            let gate = p
                .gate(&id)
                .ok_or_else(|| CliError::domain("unknown gate"))?;
            signed(
                "waiver",
                json!({"schema_version":"0.2.0","waiver_id":generated_id("waiver"),"gate_ref":id,"gate_definition_version":gate.definition_version,"gate_definition_digest":gate.digest,"actor_ref":args.actor_id,"reason":args.reason,"scope":gate.phase_ref,"expires_at":null,"review_condition":"Revalidate on material dependency changes","authority_ref":args.authority,"created_at":at}),
            )
        }
        _ => Err(CliError::input(
            "this operation requires a canonical payload --file (JSON or YAML)",
        )),
    }
}

pub fn uri(input: &str) -> Result<String, CliError> {
    if !Path::new(input).exists() {
        return Ok(input.to_owned());
    }
    let path = fs::canonicalize(input).map_err(CliError::input)?;
    let path = path
        .to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .replace('\\', "/");
    let encoded: String = path
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    Ok(if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    })
}
fn local_file(uri: &str) -> Result<std::path::PathBuf, CliError> {
    let raw = uri
        .strip_prefix("file://")
        .ok_or_else(|| CliError::input("only explicit local file URIs are refreshable"))?;
    let mut decoded = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = raw
                .get(i + 1..i + 3)
                .ok_or_else(|| CliError::input("invalid file URI"))?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(CliError::input)?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(decoded).map_err(CliError::input)?;
    #[cfg(windows)]
    let decoded = if decoded.as_bytes().get(2) == Some(&b':') {
        decoded[1..].to_owned()
    } else {
        decoded
    };
    Ok(decoded.into())
}

fn signed(wrapper: &str, mut record: Value) -> Result<Value, CliError> {
    record["digest"] = json!(sha256_digest(&record).map_err(CliError::input)?);
    Ok(json!({wrapper:record}))
}
