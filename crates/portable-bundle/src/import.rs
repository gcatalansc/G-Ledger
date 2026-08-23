use std::{collections::BTreeMap, fs, path::Path};

use project_ledger_domain::{
    EventEnvelope, EventId, PortableExport, PortableImportPolicy, ProjectId,
};
use project_ledger_event_store::EventImportStore;

use crate::export::{PortableBundle, PortableBundleError, validate_stream};

/// Provenance that must accompany a separately materialized explicit fork.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkProvenance {
    pub target_project_ref: ProjectId,
    pub forked_from_project_ref: ProjectId,
    pub forked_from_event_ref: EventId,
    pub forked_from_digest: project_ledger_canonical_json::Sha256Digest,
}

/// Verified, non-ambiguous disposition for a portable event chain.
#[derive(Clone, Debug, PartialEq)]
pub enum PortableImportPlan {
    IdenticalChain {
        project_ref: ProjectId,
        state_version: u64,
        chain_digest: project_ledger_canonical_json::Sha256Digest,
    },
    NewProject {
        project_ref: ProjectId,
        events: Vec<EventEnvelope>,
    },
    FastForward {
        project_ref: ProjectId,
        expected_version: u64,
        events: Vec<EventEnvelope>,
    },
    /// No event mutation is implied: a new, provenance-aware chain must be built explicitly.
    ForkExplicit { provenance: ForkProvenance },
}

/// Observable result of executing a previously verified import plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortableImportOutcome {
    IdenticalChain {
        project_ref: ProjectId,
        state_version: u64,
    },
    NewProject {
        project_ref: ProjectId,
        appended_events: u64,
    },
    FastForward {
        project_ref: ProjectId,
        previous_version: u64,
        appended_events: u64,
    },
    ForkExplicit {
        provenance: ForkProvenance,
    },
}

/// Executes only the unambiguous storage effect in a verified import plan.
///
/// `FORK_EXPLICIT` remains a no-write handoff because the frozen v0.2.0 event
/// contracts contain no fork mutation. The returned provenance is explicit.
///
/// # Errors
///
/// Returns [`PortableBundleError`] when the destination changed after planning
/// or the storage adapter rejects any event. Storage implementations must roll
/// back the complete suffix on error.
pub fn apply_import<S: EventImportStore>(
    store: &mut S,
    plan: &PortableImportPlan,
) -> Result<PortableImportOutcome, PortableBundleError> {
    match plan {
        PortableImportPlan::IdenticalChain {
            project_ref,
            state_version,
            chain_digest,
        } => {
            let actual = store.current_version(project_ref)?;
            if actual != *state_version {
                return Err(
                    project_ledger_event_store::StoreError::StreamVersionConflict {
                        expected: *state_version,
                        actual,
                    }
                    .into(),
                );
            }
            store.verify_event_chain(project_ref)?;
            let actual_digest = store
                .load_stream(project_ref)?
                .last()
                .map(|event| event.event_digest);
            if actual_digest != Some(*chain_digest) {
                return Err(PortableBundleError::StreamDivergence {
                    version: *state_version,
                });
            }
            Ok(PortableImportOutcome::IdenticalChain {
                project_ref: project_ref.clone(),
                state_version: *state_version,
            })
        }
        PortableImportPlan::NewProject {
            project_ref,
            events,
        } => {
            store.append_imported_events(project_ref, 0, events)?;
            Ok(PortableImportOutcome::NewProject {
                project_ref: project_ref.clone(),
                appended_events: events.len() as u64,
            })
        }
        PortableImportPlan::FastForward {
            project_ref,
            expected_version,
            events,
        } => {
            store.append_imported_events(project_ref, *expected_version, events)?;
            Ok(PortableImportOutcome::FastForward {
                project_ref: project_ref.clone(),
                previous_version: *expected_version,
                appended_events: events.len() as u64,
            })
        }
        PortableImportPlan::ForkExplicit { provenance } => {
            Ok(PortableImportOutcome::ForkExplicit {
                provenance: provenance.clone(),
            })
        }
    }
}

impl PortableBundle {
    /// Reads only the fixed §31.1 directory tree and verifies it before return.
    ///
    /// # Errors
    ///
    /// Returns [`PortableBundleError`] for missing/extra/symlinked files, invalid
    /// JSONL, checksum or manifest divergence, and any invalid event/state chain.
    pub fn read_directory(directory: &Path) -> Result<Self, PortableBundleError> {
        let upstream_locks = directory.join("upstream-locks");
        if !upstream_locks.is_dir() {
            return Err(PortableBundleError::Integrity {
                message: "required upstream-locks directory is missing".to_owned(),
            });
        }
        let mut entries = BTreeMap::new();
        collect_entries(directory, directory, &mut entries)?;
        validate_entry_names(&entries)?;

        let manifest_bytes = required(&entries, "manifest.json")?;
        let manifest: PortableExport = serde_json::from_slice(manifest_bytes)?;
        let events = decode_events(required(&entries, "events.jsonl")?)?;
        let bundle = Self {
            manifest,
            events,
            entries,
        };
        bundle.verify()?;
        Ok(bundle)
    }

    /// Classifies the verified bundle against one verified local stream.
    ///
    /// # Errors
    ///
    /// Returns fail-closed on identity mismatch, a newer destination, policy
    /// mismatch or divergence without a distinct explicit fork target.
    pub fn plan_import(
        &self,
        local_events: &[EventEnvelope],
        explicit_fork_target: Option<ProjectId>,
    ) -> Result<PortableImportPlan, PortableBundleError> {
        self.verify()?;
        if local_events.is_empty() {
            if self.manifest.import_policy == PortableImportPolicy::IdenticalChain {
                return Err(PortableBundleError::ImportPolicyViolation);
            }
            return Ok(PortableImportPlan::NewProject {
                project_ref: self.manifest.project_ref.clone(),
                events: self.events.clone(),
            });
        }

        let local = validate_stream(local_events)?;
        if local.state().project.project_id != self.manifest.project_ref {
            return Err(PortableBundleError::InvalidStream {
                message: "local and portable project identities differ".to_owned(),
            });
        }

        for (index, (local_event, imported_event)) in
            local_events.iter().zip(&self.events).enumerate()
        {
            if local_event.event_id == imported_event.event_id
                && local_event.event_digest != imported_event.event_digest
            {
                return self.divergence(index as u64 + 1, explicit_fork_target);
            }
            if local_event.event_id != imported_event.event_id
                || local_event.event_digest != imported_event.event_digest
            {
                return self.divergence(index as u64 + 1, explicit_fork_target);
            }
        }

        match local_events.len().cmp(&self.events.len()) {
            std::cmp::Ordering::Equal => Ok(PortableImportPlan::IdenticalChain {
                project_ref: self.manifest.project_ref.clone(),
                state_version: self.manifest.project_state_version,
                chain_digest: self.manifest.chain_digest,
            }),
            std::cmp::Ordering::Less => {
                if self.manifest.import_policy != PortableImportPolicy::FastForward {
                    return Err(PortableBundleError::ImportPolicyViolation);
                }
                Ok(PortableImportPlan::FastForward {
                    project_ref: self.manifest.project_ref.clone(),
                    expected_version: local_events.len() as u64,
                    events: self.events[local_events.len()..].to_vec(),
                })
            }
            std::cmp::Ordering::Greater => Err(PortableBundleError::DestinationNewer {
                local_version: local_events.len() as u64,
                bundle_version: self.events.len() as u64,
            }),
        }
    }

    fn divergence(
        &self,
        version: u64,
        target: Option<ProjectId>,
    ) -> Result<PortableImportPlan, PortableBundleError> {
        if self.manifest.import_policy != PortableImportPolicy::ForkExplicit {
            return Err(PortableBundleError::StreamDivergence { version });
        }
        let Some(target_project_ref) = target else {
            return Err(PortableBundleError::StreamDivergence { version });
        };
        if target_project_ref == self.manifest.project_ref {
            return Err(PortableBundleError::InvalidForkTarget);
        }
        Ok(PortableImportPlan::ForkExplicit {
            provenance: ForkProvenance {
                target_project_ref,
                forked_from_project_ref: self.manifest.project_ref.clone(),
                forked_from_event_ref: self.manifest.last_event_ref.clone(),
                forked_from_digest: self.manifest.chain_digest,
            },
        })
    }
}

fn collect_entries(
    root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), PortableBundleError> {
    let iterator = fs::read_dir(directory).map_err(|source| io(directory, source))?;
    for item in iterator {
        let item = item.map_err(|source| io(directory, source))?;
        let file_type = item
            .file_type()
            .map_err(|source| io(&item.path(), source))?;
        if file_type.is_symlink() {
            return Err(PortableBundleError::UnsupportedEntry {
                path: item.path().display().to_string(),
            });
        }
        if file_type.is_dir() {
            collect_entries(root, &item.path(), entries)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(PortableBundleError::UnsupportedEntry {
                path: item.path().display().to_string(),
            });
        }
        let relative = item
            .path()
            .strip_prefix(root)
            .map_err(|_| PortableBundleError::UnsupportedEntry {
                path: item.path().display().to_string(),
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(item.path()).map_err(|source| io(&item.path(), source))?;
        entries.insert(relative, bytes);
    }
    Ok(())
}

fn validate_entry_names(entries: &BTreeMap<String, Vec<u8>>) -> Result<(), PortableBundleError> {
    const REQUIRED: &[&str] = &[
        "checksums.sha256",
        "events.jsonl",
        "evidence/references.json",
        "generated/BITACORA_PROYECTO.md",
        "integrations/radar-project-projection.json",
        "manifest.json",
        "project.json",
        "projections/resume.json",
        "projections/state.json",
        "schemas-lock.json",
    ];
    for required_path in REQUIRED {
        required(entries, required_path)?;
    }
    for path in entries.keys() {
        if REQUIRED.contains(&path.as_str()) {
            continue;
        }
        let Some(name) = path.strip_prefix("upstream-locks/") else {
            return Err(PortableBundleError::UnsupportedEntry { path: path.clone() });
        };
        if name.is_empty()
            || !name.to_ascii_lowercase().ends_with(".json")
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
        {
            return Err(PortableBundleError::UnsupportedEntry { path: path.clone() });
        }
    }
    Ok(())
}

fn decode_events(bytes: &[u8]) -> Result<Vec<EventEnvelope>, PortableBundleError> {
    if bytes.is_empty() || !bytes.ends_with(b"\n") || bytes.contains(&b'\r') {
        return Err(PortableBundleError::Integrity {
            message: "events.jsonl must be non-empty LF-terminated JSONL".to_owned(),
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| PortableBundleError::Integrity {
        message: "events.jsonl is not UTF-8".to_owned(),
    })?;
    text.lines()
        .map(|line| serde_json::from_str(line).map_err(PortableBundleError::from))
        .collect()
}

fn required<'a>(
    entries: &'a BTreeMap<String, Vec<u8>>,
    path: &str,
) -> Result<&'a [u8], PortableBundleError> {
    entries
        .get(path)
        .map(Vec::as_slice)
        .ok_or_else(|| PortableBundleError::Integrity {
            message: format!("required entry is missing: {path}"),
        })
}

fn io(path: &Path, source: std::io::Error) -> PortableBundleError {
    PortableBundleError::Io {
        path: path.to_path_buf(),
        source,
    }
}
