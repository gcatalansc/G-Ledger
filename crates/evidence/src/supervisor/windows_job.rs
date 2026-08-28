use super::SupervisionError;
use process_wrap::std::{CommandWrap, CommandWrapper};
use std::{
    io,
    os::windows::io::AsRawHandle,
    process::{Child, Command},
};
use win32job::{ExtendedLimitInfo, Job};

/// Outer non-inheritable job: OS-enforced cleanup even if our supervisor is killed.
/// process-wrap's std `JobObject` performs suspension/resumption and active killing,
/// but does not set `KILL_ON_JOB_CLOSE`. Nested jobs preserve both guarantees.
#[derive(Debug)]
pub(super) struct KillOnClose(Job);
impl KillOnClose {
    pub(super) fn new() -> io::Result<Self> {
        let mut limits = ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        Job::create_with_limit_info(&limits)
            .map(Self)
            .map_err(io::Error::other)
    }
    pub(super) fn has_processes(&self) -> Result<bool, SupervisionError> {
        self.0
            .query_process_id_list()
            .map(|ids| !ids.is_empty())
            .map_err(|_| SupervisionError::Cleanup)
    }
}
impl CommandWrapper for KillOnClose {
    fn post_spawn(
        &mut self,
        _: &mut Command,
        child: &mut Child,
        _: &CommandWrap,
    ) -> io::Result<()> {
        // All post_spawn hooks run after native creation but before JobObject's
        // wrap_child resumes the temporarily suspended process. No child code has run.
        if let Err(error) = self.0.assign_process(child.as_raw_handle() as isize) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(error));
        }
        Ok(())
    }
}
