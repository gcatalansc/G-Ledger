//! Pure derived local project catalog.

use std::collections::BTreeSet;

use project_ledger_domain::{
    ProjectCatalogEntry, ProjectId, ProjectState, ResourceRef, SchemaVersion,
};
use project_ledger_projection_engine::{ProjectionError, validate_state_integrity};
use thiserror::Error;

/// One explicitly known ledger and its minimized caller-supplied location.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownProject {
    pub state: ProjectState,
    pub ledger_location_ref: ResourceRef,
}

/// Failures that prevent a catalog from representing trusted known projects.
#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("project {project_ref} has an invalid derived state: {source}")]
    InvalidState {
        project_ref: ProjectId,
        #[source]
        source: ProjectionError,
    },
    #[error("project {project_ref} appears more than once in the known-project input")]
    DuplicateProject { project_ref: ProjectId },
    #[error("could not derive the catalog project reference for {project_ref}")]
    InvalidProjectReference { project_ref: ProjectId },
}

/// Deterministic builder for the optional, non-authoritative local catalog.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectCatalog;

impl ProjectCatalog {
    /// Rebuilds catalog entries only from the explicit known ledgers supplied by
    /// the caller. This function performs no filesystem discovery or writes.
    ///
    /// # Errors
    ///
    /// Rejects invalid projected state, duplicate project identity, or an
    /// impossible derived resource reference.
    pub fn rebuild(
        known_projects: &[KnownProject],
    ) -> Result<Vec<ProjectCatalogEntry>, CatalogError> {
        let mut seen = BTreeSet::new();
        let mut entries = Vec::with_capacity(known_projects.len());
        for known in known_projects {
            let project_id = known.state.project.project_id.clone();
            validate_state_integrity(&known.state).map_err(|source| {
                CatalogError::InvalidState {
                    project_ref: project_id.clone(),
                    source,
                }
            })?;
            if !seen.insert(project_id.clone()) {
                return Err(CatalogError::DuplicateProject {
                    project_ref: project_id,
                });
            }
            let project_ref =
                ResourceRef::parse(&format!("project-ledger://projects/{}", project_id.uuid()))
                    .map_err(|_| CatalogError::InvalidProjectReference {
                        project_ref: project_id,
                    })?;
            entries.push(ProjectCatalogEntry {
                schema_version: SchemaVersion::V0_2_0,
                project_ref,
                canonical_title: known.state.project.title.clone(),
                ledger_location_ref: known.ledger_location_ref.clone(),
                last_seen_state_version: known.state.state_version,
                last_seen_state_digest: known.state.state_digest,
            });
        }
        entries.sort_by(|left, right| left.project_ref.cmp(&right.project_ref));
        Ok(entries)
    }
}
