use std::collections::BTreeSet;

use project_ledger_schemas::{fixture_catalog, schema_catalog};

#[test]
fn every_schema_has_one_positive_and_one_negative_fixture() {
    let schemas = schema_catalog().unwrap();
    let fixtures = fixture_catalog().unwrap();
    let schema_ids: BTreeSet<_> = schemas
        .iter()
        .map(|entry| entry.schema_id.as_str())
        .collect();
    let fixture_ids: BTreeSet<_> = fixtures
        .iter()
        .map(|entry| entry.schema_id.as_str())
        .collect();

    assert_eq!(schema_ids, fixture_ids);
    assert_eq!(fixtures.len(), 46);
}

#[test]
fn positive_fixtures_validate_and_negative_fixtures_fail() {
    let schemas = schema_catalog().unwrap();
    let fixtures = fixture_catalog().unwrap();

    for schema in schemas {
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.schema_id == schema.schema_id)
            .unwrap();
        let validator = jsonschema::validator_for(&schema.schema).unwrap();
        assert!(
            validator.is_valid(&fixture.positive),
            "positive fixture failed for {}: {:?}",
            schema.schema_id,
            validator.iter_errors(&fixture.positive).collect::<Vec<_>>()
        );
        assert!(
            !validator.is_valid(&fixture.negative),
            "negative fixture unexpectedly passed for {}",
            schema.schema_id
        );
    }
}
