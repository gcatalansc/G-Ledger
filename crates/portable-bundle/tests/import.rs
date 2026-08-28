mod support;

use project_ledger_domain::{PortableImportPolicy, ProjectId};
use project_ledger_portable_bundle::{PortableBundle, PortableBundleError, PortableImportPlan};

#[test]
fn reader_rejects_unlisted_empty_directories_before_traversal() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("bundle");
    support::bundle(
        support::stream_with_title("safe", 1),
        PortableImportPolicy::FastForward,
    )
    .write_directory(&directory)
    .unwrap();
    std::fs::create_dir(directory.join("unexpected")).unwrap();
    assert!(matches!(
        PortableBundle::read_directory(&directory),
        Err(PortableBundleError::UnsupportedEntry { .. })
    ));
}

#[test]
fn ac_005_directory_roundtrip_verifies_equivalent_chain_and_rejects_tampering() {
    let source = support::stream_with_title("source-title", 2);
    let bundle = support::bundle(source.clone(), PortableImportPolicy::FastForward);
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("project-ledger-export");
    bundle.write_directory(&directory).unwrap();

    let loaded = PortableBundle::read_directory(&directory).unwrap();
    assert_eq!(loaded.events(), source);
    assert_eq!(loaded.manifest(), bundle.manifest());
    assert!(loaded.verify().is_ok());

    std::fs::write(directory.join("projections/state.json"), b"{}\n").unwrap();
    assert!(PortableBundle::read_directory(&directory).is_err());
}

#[test]
fn ac_035_import_classifies_identical_new_and_fast_forward_without_overwrite() {
    let complete = support::stream_with_title("source-title", 2);
    let bundle = support::bundle(complete.clone(), PortableImportPolicy::FastForward);

    assert!(matches!(
        bundle.plan_import(&[], None).unwrap(),
        PortableImportPlan::NewProject { ref events, .. } if events == &complete
    ));
    assert!(matches!(
        bundle.plan_import(&complete[..1], None).unwrap(),
        PortableImportPlan::FastForward { expected_version: 1, ref events, .. }
            if events == &complete[1..]
    ));
    assert!(matches!(
        bundle.plan_import(&complete, None).unwrap(),
        PortableImportPlan::IdenticalChain {
            state_version: 2,
            ..
        }
    ));

    let newer = support::stream_with_title("source-title", 3);
    assert!(matches!(
        bundle.plan_import(&newer, None),
        Err(PortableBundleError::DestinationNewer { .. })
    ));
}

#[test]
fn ac_035_divergence_fails_closed_or_returns_explicit_fork_provenance() {
    let source = support::stream_with_title("source-branch", 2);
    let sibling = support::stream_with_title("local-branch", 2);
    let ordinary = support::bundle(source.clone(), PortableImportPolicy::FastForward);

    assert!(matches!(
        ordinary.plan_import(&sibling, None),
        Err(PortableBundleError::StreamDivergence { version: 2 })
    ));

    let explicit = support::bundle(source, PortableImportPolicy::ForkExplicit);
    let target = ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df99").unwrap();
    let plan = explicit
        .plan_import(&sibling, Some(target.clone()))
        .unwrap();
    let PortableImportPlan::ForkExplicit { provenance } = plan else {
        panic!("divergent explicit fork must not be a merge plan");
    };
    assert_eq!(provenance.target_project_ref, target);
    assert_eq!(
        provenance.forked_from_project_ref,
        explicit.manifest().project_ref
    );
    assert_eq!(
        provenance.forked_from_event_ref,
        explicit.manifest().last_event_ref
    );
    assert_eq!(
        provenance.forked_from_digest,
        explicit.manifest().chain_digest
    );

    assert!(matches!(
        explicit.plan_import(&sibling, Some(explicit.manifest().project_ref.clone())),
        Err(PortableBundleError::InvalidForkTarget)
    ));
}
