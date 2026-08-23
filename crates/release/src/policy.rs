use std::path::{Path, PathBuf};

use project_ledger_domain::TrustLevel;
use thiserror::Error;

/// Capabilities that may be granted at an explicit trusted boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // The explicit capability vector is the audited policy output.
pub struct CapabilitySet {
    pub adapter_read: bool,
    pub adapter_write: bool,
    pub lifecycle_authority: bool,
    pub network: bool,
    pub credentials: bool,
    pub executable_hooks: bool,
}

/// Secure defaults shared by release tooling and repository trust checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleasePolicy;

impl ReleasePolicy {
    /// Derives capabilities from trusted state, never from repository-local configuration.
    #[must_use]
    pub const fn capabilities(trust_level: TrustLevel) -> CapabilitySet {
        match trust_level {
            TrustLevel::TrustedForProjection => CapabilitySet {
                adapter_read: true,
                adapter_write: false,
                lifecycle_authority: false,
                network: false,
                credentials: false,
                executable_hooks: false,
            },
            TrustLevel::TrustedForRecording => CapabilitySet {
                adapter_read: true,
                adapter_write: true,
                lifecycle_authority: false,
                network: false,
                credentials: false,
                executable_hooks: false,
            },
            TrustLevel::Untrusted
            | TrustLevel::LocalUser
            | TrustLevel::TrustedProject
            | TrustLevel::Revoked => CapabilitySet {
                adapter_read: false,
                adapter_write: false,
                lifecycle_authority: false,
                network: false,
                credentials: false,
                executable_hooks: false,
            },
        }
    }
}

/// A process invocation represented as an executable plus literal argument vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProcessPlan {
    executable: String,
    arguments: Vec<String>,
    cwd: PathBuf,
}

/// Invalid exact-process plans.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ProcessPlanError {
    #[error("executable must not be empty")]
    EmptyExecutable,
    #[error("working directory must not be empty")]
    EmptyWorkingDirectory,
}

impl ExactProcessPlan {
    /// Creates a no-shell plan. Metacharacters remain ordinary argument bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the executable or working directory is empty.
    pub fn new<I, S>(
        executable: impl Into<String>,
        arguments: I,
        cwd: &Path,
    ) -> Result<Self, ProcessPlanError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let executable = executable.into();
        if executable.trim().is_empty() {
            return Err(ProcessPlanError::EmptyExecutable);
        }
        if cwd.as_os_str().is_empty() {
            return Err(ProcessPlanError::EmptyWorkingDirectory);
        }
        Ok(Self {
            executable,
            arguments: arguments.into_iter().map(Into::into).collect(),
            cwd: cwd.to_path_buf(),
        })
    }

    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    #[must_use]
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    #[must_use]
    pub const fn uses_shell(&self) -> bool {
        false
    }
}
