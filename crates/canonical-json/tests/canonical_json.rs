use project_ledger_canonical_json::{Sha256Digest, sha256_bytes, sha256_digest, to_canonical_vec};

#[test]
fn object_keys_are_sorted_recursively() {
    let value = serde_json::json!({"z": 1, "a": {"y": 2, "b": 3}});

    let canonical = String::from_utf8(to_canonical_vec(&value).unwrap()).unwrap();

    assert_eq!(canonical, r#"{"a":{"b":3,"y":2},"z":1}"#);
}

#[test]
fn numbers_follow_rfc_8785_representation() {
    let value = serde_json::json!({"whole": 1.0, "small": 1e-7});

    let canonical = String::from_utf8(to_canonical_vec(&value).unwrap()).unwrap();

    assert_eq!(canonical, r#"{"small":1e-7,"whole":1}"#);
}

#[test]
fn digest_matches_a_hand_checked_sha256_fixture() {
    let value = serde_json::json!({"a": 1});

    let digest = sha256_digest(&value).unwrap();

    assert_eq!(
        digest.to_string(),
        "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
}

#[test]
fn digest_parser_rejects_noncanonical_text() {
    let uppercase = "sha256:015ABD7F5CC57A2DD94B7590F04AD8084273905EE33EC5CEBEAE62276A97F862";
    let missing_prefix = "015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862";

    assert!(Sha256Digest::parse(uppercase).is_err());
    assert!(Sha256Digest::parse(missing_prefix).is_err());
}

#[test]
fn raw_byte_digest_does_not_apply_json_transformation() {
    assert_eq!(
        sha256_bytes(b"{}").to_string(),
        "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}
