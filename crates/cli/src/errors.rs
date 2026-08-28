use project_ledger_application::ProcessorError;
use project_ledger_domain::{CommandResult, CommandResultStatus, ErrorCode};
use project_ledger_event_store::StoreError;
use serde_json::{Value, json};

#[derive(Debug)]
pub struct CliError {
    pub exit: u8,
    pub code: &'static str,
    pub message: String,
    pub result: Option<Value>,
}
impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self::input(message)
    }
}
impl CliError {
    pub fn new(exit: u8, code: &'static str, message: impl std::fmt::Display) -> Self {
        Self {
            exit,
            code,
            message: message.to_string(),
            result: None,
        }
    }
    pub fn input(error: impl std::fmt::Display) -> Self {
        Self::new(2, "INVALID_INPUT", error)
    }
    pub fn storage(error: impl std::fmt::Display) -> Self {
        Self::new(11, "STORAGE_ERROR", error)
    }
    pub fn domain(error: impl std::fmt::Display) -> Self {
        Self::new(3, "DOMAIN_VALIDATION", error)
    }
    pub fn json(&self) -> Value {
        self.result.clone().unwrap_or_else(
            || json!({"error":{"code":self.code,"message":self.message,"exit_code":self.exit}}),
        )
    }
    pub fn command(result: CommandResult) -> Result<String, Self> {
        if matches!(
            result.status,
            CommandResultStatus::Accepted | CommandResultStatus::DuplicateReplay
        ) {
            return crate::canonical_stdout(&result).map_err(Self::input);
        }
        let (exit, code) = match (result.status, result.rejection.as_ref().map(|r| r.code)) {
            (CommandResultStatus::PolicyDenied, _) => (9, "POLICY_DENIED"),
            (CommandResultStatus::VersionConflict, _) | (_, Some(ErrorCode::VersionConflict)) => {
                (5, "VERSION_CONFLICT")
            }
            (_, Some(ErrorCode::IdempotencyConflict)) => (10, "IDEMPOTENCY_CONFLICT"),
            (CommandResultStatus::IncompatibleContract, _) => (2, "INCOMPATIBLE_CONTRACT"),
            _ => (3, "DOMAIN_VALIDATION"),
        };
        let mut error = Self::new(
            exit,
            code,
            result
                .rejection
                .as_ref()
                .map_or("command rejected", |r| r.message.as_str()),
        );
        error.result = Some(serde_json::to_value(result).map_err(Self::input)?);
        Err(error)
    }
}
impl From<StoreError> for CliError {
    fn from(error: StoreError) -> Self {
        let (exit, code) = match &error {
            StoreError::StreamVersionConflict { .. } => (5, "VERSION_CONFLICT"),
            StoreError::IdempotencyConflict { .. } => (10, "IDEMPOTENCY_CONFLICT"),
            StoreError::CorruptStream { .. }
            | StoreError::InvalidEventSequence { .. }
            | StoreError::Integrity(_) => (15, "STREAM_DIVERGENCE"),
            StoreError::SubmissionStateConflict { .. } => (8, "CONFLICT_OPEN"),
            _ => (11, "STORAGE_ERROR"),
        };
        Self::new(exit, code, error)
    }
}
impl From<ProcessorError> for CliError {
    fn from(error: ProcessorError) -> Self {
        match error {
            ProcessorError::Store(e) => e.into(),
            other => Self::input(other),
        }
    }
}
impl From<project_ledger_storage_sqlite::SqliteStoreError> for CliError {
    fn from(error: project_ledger_storage_sqlite::SqliteStoreError) -> Self {
        Self::storage(error)
    }
}
