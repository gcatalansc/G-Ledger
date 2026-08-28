//! Bounded process supervision; no shell, inherited credentials or durable raw output.
use process_wrap::std::{ChildWrapper, CommandWrap};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

#[cfg(windows)]
#[path = "supervisor/windows_job.rs"]
mod windows_job;

/// Explicit process limits and the complete environment (not additions to the parent).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProcessPolicy {
    pub timeout_ms: u64,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub environment: BTreeMap<String, String>,
}
impl Default for ProcessPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 60_000,
            max_stdout_bytes: 1_048_576,
            max_stderr_bytes: 1_048_576,
            environment: BTreeMap::new(),
        }
    }
}
/// An explicit native executable and literal argument vector.
#[derive(Clone, Debug)]
pub struct ProcessRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
}
/// Cooperative cancellation, optionally requested through an explicitly selected file.
#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    requested: Arc<AtomicBool>,
    file: Option<PathBuf>,
}
impl CancelToken {
    pub fn with_file(file: Option<PathBuf>) -> Self {
        Self {
            file,
            ..Self::default()
        }
    }
    pub fn cancel(&self) {
        self.requested.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
            || self.file.as_ref().is_some_and(|path| path.exists())
    }
}
/// Ephemeral bounded bytes for a parser. Deliberately not serializable.
#[derive(Debug)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: u64,
}
/// Public diagnostics deliberately exclude argv, output and environment values.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SupervisionError {
    #[error(
        "process policy requires a native absolute executable, absolute cwd and nonzero bounds"
    )]
    PolicyDenied,
    #[error("process exceeded its time limit")]
    Timeout,
    #[error("process was cancelled")]
    Cancelled,
    #[error("process exceeded its output limit")]
    OutputLimit,
    #[error("process or pipe operation failed")]
    Io,
    #[error("process cleanup could not be confirmed within the cleanup deadline")]
    Cleanup,
}
impl ProcessPolicy {
    /// Validates before starting a process or reserving an execution attempt.
    /// # Errors
    /// Returns a value-free policy error for invalid executable, cwd or bounds.
    pub fn validate(&self, request: &ProcessRequest) -> Result<(), SupervisionError> {
        if self.timeout_ms == 0
            || self.max_stdout_bytes == 0
            || self.max_stderr_bytes == 0
            || !request.program.is_absolute()
            || !request.program.is_file()
            || !request.working_directory.is_absolute()
            || !request.working_directory.is_dir()
            || self
                .environment
                .keys()
                .any(|key| key.is_empty() || key.contains(['=', '\0']))
            || self.environment.values().any(|value| value.contains('\0'))
            || request.args.iter().any(|arg| arg.contains('\0'))
        {
            return Err(SupervisionError::PolicyDenied);
        }
        // Windows otherwise implicitly routes .bat/.cmd through a shell.
        #[cfg(windows)]
        if !request
            .program
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Err(SupervisionError::PolicyDenied);
        }
        Ok(())
    }
}
struct OwnedChild(Box<dyn ChildWrapper>, bool);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.1 {
            let _ = self.0.start_kill();
        }
    }
}
type Captured = Receiver<Result<Vec<u8>, SupervisionError>>;
fn capture(
    mut pipe: impl Read + Send + 'static,
    limit: usize,
    exceeded: Arc<AtomicBool>,
) -> Result<Captured, SupervisionError> {
    let (send, receive) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("ledger-bounded-output".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 8192];
            let result = loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => break Ok(bytes),
                    Ok(count) => {
                        if count > limit.saturating_sub(bytes.len()) {
                            exceeded.store(true, Ordering::Release);
                            break Err(SupervisionError::OutputLimit);
                        }
                        bytes.extend_from_slice(&chunk[..count]);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => break Err(SupervisionError::Io),
                }
            };
            let _ = send.send(result);
        })
        .map_err(|_| SupervisionError::Io)?;
    Ok(receive)
}

/// Supervises one own process family. This is not a hostile-code/network sandbox.
/// # Errors
/// Fails closed on policy, timeout, cancellation, overflow, I/O or unconfirmed cleanup.
#[allow(clippy::too_many_lines)] // Keep preflight, bounded capture and mandatory family cleanup in audit order.
pub fn supervise(
    policy: &ProcessPolicy,
    request: &ProcessRequest,
    cancel: &CancelToken,
) -> Result<ProcessOutput, SupervisionError> {
    policy.validate(request)?;
    if cancel.file.as_ref().is_some_and(|file| !file.is_absolute()) {
        return Err(SupervisionError::PolicyDenied);
    }
    if cancel.is_cancelled() {
        return Err(SupervisionError::Cancelled);
    }
    let started = Instant::now();
    let mut command = CommandWrap::with_new(&request.program, |command| {
        command
            .args(&request.args)
            .current_dir(&request.working_directory)
            .env_clear()
            .envs(&policy.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(windows)]
    command
        .wrap(process_wrap::std::CreationFlags(
            windows::Win32::System::Threading::CREATE_NO_WINDOW,
        ))
        .wrap(process_wrap::std::JobObject)
        .wrap(windows_job::KillOnClose::new().map_err(|_| SupervisionError::Io)?);
    #[cfg(unix)]
    command.wrap(process_wrap::std::ProcessGroup::leader());
    let mut child = OwnedChild(command.spawn().map_err(|_| SupervisionError::Io)?, false);
    let exceeded = Arc::new(AtomicBool::new(false));
    let stdout = capture(
        child.0.stdout().take().ok_or(SupervisionError::Io)?,
        policy.max_stdout_bytes,
        exceeded.clone(),
    )?;
    let stderr = capture(
        child.0.stderr().take().ok_or(SupervisionError::Io)?,
        policy.max_stderr_bytes,
        exceeded.clone(),
    )?;
    let outcome = loop {
        if cancel.is_cancelled() {
            break Err(SupervisionError::Cancelled);
        }
        if exceeded.load(Ordering::Acquire) {
            break Err(SupervisionError::OutputLimit);
        }
        if started.elapsed() >= Duration::from_millis(policy.timeout_ms) {
            break Err(SupervisionError::Timeout);
        }
        match child.0.try_wait() {
            Ok(Some(status)) => break Ok(status.code()),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => break Err(SupervisionError::Io),
        }
    };
    // Also terminate descendants after a successful parent exit: their inherited
    // pipes must not keep capture alive beyond the bounded cleanup deadline.
    let kill = child.0.start_kill();
    #[cfg(unix)]
    let already_gone = kill
        .as_ref()
        .err()
        .is_some_and(|error| error.raw_os_error() == Some(3));
    #[cfg(not(unix))]
    let already_gone = false;
    if kill.is_err() && !already_gone {
        return Err(SupervisionError::Cleanup);
    }
    child.1 = true;
    let cleanup_deadline = Instant::now() + Duration::from_secs(2);
    while child
        .0
        .try_wait()
        .map_err(|_| SupervisionError::Cleanup)?
        .is_none()
    {
        if Instant::now() >= cleanup_deadline {
            return Err(SupervisionError::Cleanup);
        }
        thread::sleep(Duration::from_millis(10));
    }
    #[cfg(windows)]
    while command
        .get_wrap::<windows_job::KillOnClose>()
        .ok_or(SupervisionError::Cleanup)?
        .has_processes()?
    {
        if Instant::now() >= cleanup_deadline {
            return Err(SupervisionError::Cleanup);
        }
        thread::sleep(Duration::from_millis(10));
    }
    let out = stdout
        .recv_timeout(cleanup_deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| SupervisionError::Cleanup)?;
    let err = stderr
        .recv_timeout(cleanup_deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| SupervisionError::Cleanup)?;
    Ok(ProcessOutput {
        exit_code: outcome?,
        stdout: out?,
        stderr: err?,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}
