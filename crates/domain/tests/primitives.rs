use project_ledger_domain::{
    CommandId, ErrorCode, EventId, LedgerTimestamp, ProjectId, ResourceRef, StableId,
};

#[test]
fn generated_identifiers_are_uuid_v7_and_keep_their_kind_prefix() {
    let event_id = EventId::new();
    let command_id = CommandId::new();
    let project_id = ProjectId::new();

    assert!(event_id.as_str().starts_with("event:"));
    assert!(command_id.as_str().starts_with("command:"));
    assert!(project_id.as_str().starts_with("project:"));
    assert_eq!(event_id.uuid().get_version_num(), 7);
    assert_eq!(command_id.uuid().get_version_num(), 7);
    assert_eq!(project_id.uuid().get_version_num(), 7);
}

#[test]
fn event_id_rejects_a_non_v7_uuid() {
    let error = EventId::parse("event:550e8400-e29b-41d4-a716-446655440000").unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidIdentifier);
}

#[test]
fn identifier_rejects_the_wrong_resource_prefix() {
    let error = ProjectId::parse("event:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidIdentifier);
}

#[test]
fn timestamp_requires_an_explicit_rfc3339_offset() {
    let error = LedgerTimestamp::parse("2026-08-22T12:00:00").unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidTimestamp);
}

#[test]
fn timestamp_roundtrips_in_canonical_rfc3339_form() {
    let timestamp = LedgerTimestamp::parse("2026-08-22T12:00:00-04:00").unwrap();
    let json = serde_json::to_string(&timestamp).unwrap();

    assert_eq!(json, r#""2026-08-22T12:00:00-04:00""#);
}

#[test]
fn resource_reference_rejects_whitespace_and_relative_paths() {
    assert!(ResourceRef::parse(" project-ledger://projects/01H ").is_err());
    assert!(ResourceRef::parse("../project").is_err());
    assert!(ResourceRef::parse("project-ledger://projects/01H").is_ok());
    assert!(ResourceRef::parse("urn:project-ledger:command:start-phase:0.2.0").is_ok());
}

#[test]
fn stable_id_accepts_legible_ids_and_rejects_ambiguous_text() {
    assert!(StableId::parse("phase-2").is_ok());
    assert!(StableId::parse("gate:sqlite-roundtrip").is_ok());
    assert!(StableId::parse("").is_err());
    assert!(StableId::parse("phase 2").is_err());
    assert!(StableId::parse("../phase-2").is_err());
}
