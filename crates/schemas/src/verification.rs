use crate::{SchemaError, command_event_matrix, fixture_catalog, schema_catalog};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContractValidationReport {
    pub schema_count: usize,
    pub positive_fixture_count: usize,
    pub negative_fixture_count: usize,
    pub matrix_row_count: usize,
}

/// Validates every positive and negative fixture against its matching schema.
///
/// # Errors
///
/// Returns [`SchemaError`] when catalogs cannot be built, a fixture is missing,
/// a schema cannot compile, a positive fixture fails, or a negative one passes.
pub fn validate_contracts() -> Result<ContractValidationReport, SchemaError> {
    let schemas = schema_catalog()?;
    let fixtures = fixture_catalog()?;
    for schema in &schemas {
        let fixture = fixtures
            .iter()
            .find(|fixture| fixture.schema_id == schema.schema_id)
            .ok_or_else(|| SchemaError::MissingFixture(schema.schema_id.clone()))?;
        let validator =
            jsonschema::validator_for(&schema.schema).map_err(|error| SchemaError::Validation {
                schema_id: schema.schema_id.clone(),
                detail: error.to_string(),
            })?;
        if let Err(error) = validator.validate(&fixture.positive) {
            return Err(SchemaError::Validation {
                schema_id: schema.schema_id.clone(),
                detail: format!("positive fixture: {error}"),
            });
        }
        if validator.is_valid(&fixture.negative) {
            return Err(SchemaError::Validation {
                schema_id: schema.schema_id.clone(),
                detail: "negative fixture unexpectedly passed".to_owned(),
            });
        }
    }
    Ok(ContractValidationReport {
        schema_count: schemas.len(),
        positive_fixture_count: fixtures.len(),
        negative_fixture_count: fixtures.len(),
        matrix_row_count: command_event_matrix().rows.len(),
    })
}
