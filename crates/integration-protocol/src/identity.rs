use std::collections::BTreeSet;

use project_ledger_domain::{IdentityMapping, IdentityMappingStatus, ResourceRef};

/// Exact external identity supplied by an integration request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityQuery {
    pub external_system: String,
    pub external_ref: ResourceRef,
}

/// Result of resolving only reviewed, confirmed identity mappings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityResolution {
    Resolved(ResourceRef),
    Unresolved,
    Ambiguous,
}

/// Resolves an external identity without aliases, names or last-write-wins.
pub fn resolve_identity(query: &IdentityQuery, mappings: &[IdentityMapping]) -> IdentityResolution {
    let matches = mappings
        .iter()
        .filter(|mapping| {
            mapping.status == IdentityMappingStatus::Confirmed
                && mapping.external_system == query.external_system
                && mapping.external_ref == query.external_ref
        })
        .map(|mapping| mapping.local_ref.clone())
        .collect::<BTreeSet<_>>();

    let mut matches = matches.into_iter();
    match (matches.next(), matches.next()) {
        (None, _) => IdentityResolution::Unresolved,
        (Some(local_ref), None) => IdentityResolution::Resolved(local_ref),
        (Some(_), Some(_)) => IdentityResolution::Ambiguous,
    }
}
