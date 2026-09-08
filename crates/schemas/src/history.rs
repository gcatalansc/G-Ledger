//! Additive history contracts. The v0.2 catalog remains byte-for-byte frozen.
use crate::{SchemaEntry, SchemaError};
use project_ledger_domain::{
    CommandEnvelope, EpisodeEntryRecord, EpisodeProjection, EpisodeUpdate, EventEnvelope,
    PortableExport,
};
use schemars::{JsonSchema, schema_for};
use serde_json::{Value, json};

/// Versioned episode, envelope and query schemas. Legacy payloads remain v0.2.
/// # Errors
/// Returns an error when a schema cannot be serialized.
pub fn schema_catalog_v0_3() -> Result<Vec<SchemaEntry>, SchemaError> {
    let episode = derived::<EpisodeUpdate>()?;
    let mut command = derived::<CommandEnvelope>()?;
    command["oneOf"] = json!([variant("command_type", "RECORD_EPISODE_UPDATE")]);
    command["properties"]["schema_version"] = json!({"const":"0.3.0"});
    install_episode(&mut command, &episode);
    let mut event = derived::<EventEnvelope>()?;
    extend_event(&mut event, &episode);
    let mut portable = derived::<PortableExport>()?;
    portable["properties"]["schema_version"] = json!({"const":"0.3.0"});
    let mut command_payload = json!({"type":"object", "oneOf":[variant("command_type", "RECORD_EPISODE_UPDATE")], "unevaluatedProperties":false});
    install_episode(&mut command_payload, &episode);
    let mut event_payload = json!({"type":"object", "oneOf":[variant("event_type", "EPISODE_UPDATE_RECORDED")], "unevaluatedProperties":false});
    install_episode(&mut event_payload, &episode);
    let mut response = response_schema()?;
    response["$defs"]["EventEnvelope"] = without_defs(&event);
    merge_defs(&mut response, &event);
    let mut schemas = vec![
        ("episode-update", episode),
        ("episode-projection", derived::<EpisodeProjection>()?),
        ("command-payload", command_payload),
        ("command-envelope", command),
        ("event-payload", event_payload),
        ("event-envelope", event),
        ("portable-export", portable),
        ("history-response", response),
    ]
    .into_iter()
    .map(|(name, mut schema)| {
        let schema_id = format!("urn:project-ledger:schema:{name}:0.3.0");
        schema["$id"] = json!(schema_id);
        schema["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
        SchemaEntry {
            schema_id,
            relative_path: format!("{name}.schema.json"),
            schema,
        }
    })
    .collect::<Vec<_>>();
    schemas.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(schemas)
}

fn derived<T: JsonSchema>() -> Result<Value, SchemaError> {
    let mut value = serde_json::to_value(schema_for!(T))?;
    crate::catalog::apply_primitive_constraints(
        value.as_object_mut().ok_or(SchemaError::NonObjectRoot)?,
    );
    Ok(value)
}
fn without_defs(schema: &Value) -> Value {
    let mut value = schema.clone();
    let root = value.as_object_mut().expect("schema object");
    root.remove("$defs");
    root.remove("$schema");
    root.remove("$id");
    value
}
fn merge_defs(target: &mut Value, source: &Value) {
    if !target["$defs"].is_object() {
        target["$defs"] = json!({});
    }
    if let Some(defs) = source["$defs"].as_object() {
        target["$defs"]
            .as_object_mut()
            .expect("definitions object")
            .extend(defs.clone());
    }
}
fn install_episode(target: &mut Value, episode: &Value) {
    merge_defs(target, episode);
    target["$defs"]["EpisodeUpdate"] = without_defs(episode);
}
fn variant(field: &str, name: &str) -> Value {
    json!({"type":"object","properties":{field:{"const":name},"payload":{"$ref":"#/$defs/EpisodeUpdate"}},"required":[field,"payload"]})
}
fn extend_event(event: &mut Value, episode: &Value) {
    event["oneOf"]
        .as_array_mut()
        .expect("event variants")
        .push(variant("event_type", "EPISODE_UPDATE_RECORDED"));
    event["properties"]["schema_version"] = json!({"enum":["0.2.0","0.3.0"]});
    event["allOf"] = json!([{"if":{"properties":{"event_type":{"const":"EPISODE_UPDATE_RECORDED"}}},
        "then":{"properties":{"schema_version":{"const":"0.3.0"}}},
        "else":{"properties":{"schema_version":{"const":"0.2.0"}}}}]);
    install_episode(event, episode);
}

fn response_schema() -> Result<Value, SchemaError> {
    let mut result = json!({"type":"object", "unevaluatedProperties":false,
    "properties":{"schema_version":{"const":"0.3.0"},"kind":{"enum":["STATE","DIFF","TRACE","RECORD"]},
        "project_ref":{"$ref":"#/$defs/ProjectId"},"state_version":{"type":"integer","minimum":1},
        "state_digest":{"$ref":"#/$defs/Sha256Digest"},"head_digest":{"$ref":"#/$defs/Sha256Digest"},"read_only":{"const":true}},
    "required":["schema_version","kind","project_ref","state_version","state_digest","head_digest","read_only"],
    "oneOf":[
        {"properties":{"kind":{"const":"STATE"},"state":{"$ref":"#/$defs/ProjectState"},"episodes":{"type":"array","items":{"$ref":"#/$defs/EpisodeProjection"}}},"required":["state","episodes"]},
        {"properties":{"kind":{"const":"DIFF"},"from_version":{"type":"integer","minimum":1},"from_head_digest":{"$ref":"#/$defs/Sha256Digest"},
            "changes":{"type":"array","items":{"type":"object","additionalProperties":false,"properties":{"path":{"type":"string"},"before":true,"after":true},"required":["path","before","after"]}},
            "events":{"type":"array","items":{"$ref":"#/$defs/EventEnvelope"}}},"required":["from_version","from_head_digest","changes","events"]},
        {"properties":{"kind":{"const":"RECORD"},"record":{"type":"object"},"source_event_refs":{"type":"array","items":{"$ref":"#/$defs/EventId"}}},"required":["record"]},
        {"properties":{"kind":{"const":"TRACE"},"decision_ref":{"$ref":"#/$defs/StableId"},
            "decisions":{"type":"array","items":{"$ref":"#/$defs/Decision"}},
            "evidence":{"type":"array","items":{"type":"object","additionalProperties":false,"properties":{"evidence_ref":{"$ref":"#/$defs/StableId"},"record":{"anyOf":[{"$ref":"#/$defs/EvidenceRecord"},{"type":"null"}]}},"required":["evidence_ref","record"]}},
            "episode_refs":{"type":"array","items":{"$ref":"#/$defs/StableId"}},
            "missing":{"type":"array","items":{"type":"object","additionalProperties":false,"properties":{"subject_ref":{"$ref":"#/$defs/StableId"},"reason":{"const":"NO_STRUCTURED_EPISODE_RECORDED"},"legacy_alternatives":{"type":"array","items":{"type":"string"}}},"required":["subject_ref","reason","legacy_alternatives"]}},
            "entries":{"type":"array","maxItems":50,"items":{"type":"object","additionalProperties":false,
                "properties":{"stream_version":{"type":"integer","minimum":1},"entry_ref":{"type":"string"},"event":{"$ref":"#/$defs/EventEnvelope"},"record":{"$ref":"#/$defs/EpisodeEntryRecord"}},
                "required":["stream_version","entry_ref"],"oneOf":[{"required":["event"]},{"required":["record"]}]}},
            "total_entries":{"type":"integer","minimum":0},"next_cursor":{"type":["string","null"]}},
            "required":["decision_ref","decisions","evidence","episode_refs","missing","entries","total_entries","next_cursor"]}
    ]});
    for (name, schema) in [
        (
            "ProjectState",
            derived::<project_ledger_domain::ProjectState>()?,
        ),
        ("EpisodeProjection", derived::<EpisodeProjection>()?),
        ("EpisodeEntryRecord", derived::<EpisodeEntryRecord>()?),
        ("Decision", derived::<project_ledger_domain::Decision>()?),
        (
            "EvidenceRecord",
            derived::<project_ledger_domain::EvidenceRecord>()?,
        ),
    ] {
        merge_defs(&mut result, &schema);
        result["$defs"][name] = without_defs(&schema);
    }
    result["$defs"]["ProjectId"] = json!({"type":"string","pattern":"^project:[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"});
    result["$defs"]["EventId"] = json!({"type":"string","pattern":"^event:[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"});
    Ok(result)
}
