use std::collections::BTreeSet;

use project_ledger_schemas::schema_catalog;

#[test]
fn registry_contains_every_normative_schema_once() {
    let catalog = schema_catalog().unwrap();
    let ids: BTreeSet<_> = catalog
        .iter()
        .map(|entry| entry.schema_id.as_str())
        .collect();
    let paths: BTreeSet<_> = catalog
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();

    assert_eq!(catalog.len(), 46);
    assert_eq!(ids.len(), catalog.len());
    assert_eq!(paths.len(), catalog.len());
}

#[test]
fn all_schema_references_are_offline_fragments() {
    let catalog = schema_catalog().unwrap();
    for entry in catalog {
        assert_refs_are_offline(&entry.schema);
    }
}

#[test]
fn radar_profile_schemas_use_the_profile_version() {
    let catalog = schema_catalog().unwrap();
    for entry in catalog
        .iter()
        .filter(|entry| entry.relative_path.starts_with("radar/"))
    {
        assert!(entry.schema_id.ends_with(":0.1.1"), "{}", entry.schema_id);
    }
}

fn assert_refs_are_offline(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(serde_json::Value::as_str) {
                assert!(
                    reference.starts_with('#'),
                    "external schema ref found: {reference}"
                );
            }
            for child in object.values() {
                assert_refs_are_offline(child);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                assert_refs_are_offline(child);
            }
        }
        _ => {}
    }
}
