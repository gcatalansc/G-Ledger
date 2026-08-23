use project_ledger_schemas::{check_generated, generated_files, write_generated};

#[test]
fn generated_file_set_is_complete_and_deterministic() {
    let first = generated_files().unwrap();
    let second = generated_files().unwrap();

    assert_eq!(first, second);
    assert_eq!(first.len(), 142);
    assert!(first.contains_key("schema-registry.json"));
    assert!(first.contains_key("command-event-matrix.json"));
    assert!(first.contains_key("array-semantics.json"));
    assert!(first.contains_key("fixtures/manifest.json"));
}

#[test]
fn every_generated_json_document_ends_with_one_lf() {
    for (path, bytes) in generated_files().unwrap() {
        assert!(bytes.ends_with(b"\n"), "missing LF: {path}");
        assert!(!bytes.ends_with(b"\n\n"), "multiple trailing LFs: {path}");
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    }
}

#[test]
fn generated_tree_check_detects_byte_drift() {
    let temporary = tempfile::tempdir().unwrap();
    write_generated(temporary.path()).unwrap();
    check_generated(temporary.path()).unwrap();

    let registry = temporary.path().join("schemas/v0.2/schema-registry.json");
    std::fs::write(&registry, b"{}\n").unwrap();

    let error = check_generated(temporary.path()).unwrap_err();
    assert!(error.to_string().contains("schema-registry.json"));
}
