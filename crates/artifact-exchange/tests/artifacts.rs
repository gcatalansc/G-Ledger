use std::fs;

use project_ledger_artifact_exchange::{
    ArtifactExchangeError, ArtifactLimits, read_canonical_json, write_canonical_json,
};
use project_ledger_canonical_json::{sha256_bytes, to_canonical_vec};
use serde_json::{Value, json};
use tempfile::tempdir;

#[cfg(windows)]
#[test]
fn long_path_artifact_publication_roundtrips_without_overwriting() {
    use std::os::windows::ffi::OsStrExt;

    let temporary = tempdir().unwrap();
    let mut root = temporary.path().to_path_buf();
    while root.as_os_str().encode_wide().count() < 1100 {
        root.push("publicación de artefactos con espacios y acentos á");
    }
    fs::create_dir_all(&root).unwrap();
    let value = json!({"state_version": 42});
    let receipt =
        write_canonical_json(&root, "projection.json", &value, ArtifactLimits::default()).unwrap();
    assert_eq!(
        read_canonical_json::<Value>(
            &root,
            "projection.json",
            receipt.digest,
            ArtifactLimits::default()
        )
        .unwrap(),
        value
    );
    assert!(matches!(
        write_canonical_json(
            &root,
            "projection.json",
            &json!({}),
            ArtifactLimits::default()
        ),
        Err(ArtifactExchangeError::AlreadyExists { .. })
    ));
    assert_eq!(
        fs::read(root.join("projection.json")).unwrap(),
        to_canonical_vec(&value).unwrap()
    );
}

#[test]
fn canonical_artifact_roundtrip_is_atomic_and_never_overwrites() {
    let root = tempdir().unwrap();
    let value = json!({"z":2,"a":{"b":1}});
    let receipt = write_canonical_json(
        root.path(),
        "projection.json",
        &value,
        ArtifactLimits::default(),
    )
    .unwrap();
    let expected = br#"{"a":{"b":1},"z":2}"#;
    assert_eq!(
        fs::read(root.path().join("projection.json")).unwrap(),
        expected
    );
    assert_eq!(receipt.digest, sha256_bytes(expected));
    assert_eq!(receipt.byte_len, expected.len() as u64);

    let loaded: Value = read_canonical_json(
        root.path(),
        "projection.json",
        receipt.digest,
        ArtifactLimits::default(),
    )
    .unwrap();
    assert_eq!(loaded, value);

    assert!(matches!(
        write_canonical_json(
            root.path(),
            "projection.json",
            &json!({"replacement":true}),
            ArtifactLimits::default(),
        ),
        Err(ArtifactExchangeError::AlreadyExists { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("projection.json")).unwrap(),
        expected
    );
}

#[test]
fn malicious_paths_noncanonical_json_tampering_and_oversize_fail_closed() {
    let root = tempdir().unwrap();
    let limits = ArtifactLimits { max_bytes: 32 };
    for name in [
        "../escape.json",
        "nested/file.json",
        "file.txt",
        ".json",
        "C:/escape.json",
    ] {
        assert!(matches!(
            write_canonical_json(root.path(), name, &json!({}), limits),
            Err(ArtifactExchangeError::UnsafeFileName { .. })
        ));
    }

    fs::write(root.path().join("noncanonical.json"), br#"{"z":2,"a":1}"#).unwrap();
    assert!(matches!(
        read_canonical_json::<Value>(
            root.path(),
            "noncanonical.json",
            sha256_bytes(br#"{"z":2,"a":1}"#),
            limits,
        ),
        Err(ArtifactExchangeError::NonCanonicalJson { .. })
    ));

    let canonical = to_canonical_vec(&json!({"a":1})).unwrap();
    fs::write(root.path().join("tampered.json"), &canonical).unwrap();
    assert!(matches!(
        read_canonical_json::<Value>(
            root.path(),
            "tampered.json",
            sha256_bytes(b"different"),
            limits,
        ),
        Err(ArtifactExchangeError::DigestMismatch { .. })
    ));

    assert!(matches!(
        write_canonical_json(
            root.path(),
            "large.json",
            &json!({"payload":"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"}),
            limits,
        ),
        Err(ArtifactExchangeError::TooLarge { .. })
    ));
}
