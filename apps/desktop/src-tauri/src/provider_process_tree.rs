use std::io;
use std::process::Child;

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::ptr::{null, null_mut};
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

/// Owns the Windows process tree for one provider turn.
///
/// Closing a configured Job Object terminates every process still assigned to
/// it. On other platforms the direct child remains the only portable boundary.
pub(crate) struct ProviderProcessTree {
    #[cfg(windows)]
    job: HANDLE,
}

impl ProviderProcessTree {
    pub(crate) fn attach(child: &Child) -> io::Result<Self> {
        #[cfg(windows)]
        {
            let handle = unsafe { CreateJobObjectW(null(), null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let tree = Self { job: handle };
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    tree.job,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            if unsafe { AssignProcessToJobObject(tree.job, child.as_raw_handle().cast::<c_void>()) }
                == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(tree)
        }

        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(Self {})
        }
    }

    pub(crate) fn terminate(&mut self, child: &mut Child) {
        #[cfg(windows)]
        if !self.job.is_null() {
            unsafe {
                TerminateJobObject(self.job, 1);
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        self.close();
    }

    fn close(&mut self) {
        #[cfg(windows)]
        if !self.job.is_null() {
            unsafe {
                CloseHandle(self.job);
            }
            self.job = null_mut();
        }
    }
}

impl Drop for ProviderProcessTree {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn attaches_and_terminates_a_live_provider_process() {
        let mut child = Command::new("ping.exe")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the Windows ping fixture should start");
        let mut tree = ProviderProcessTree::attach(&child)
            .expect("the provider process should join a kill-on-close job");

        tree.terminate(&mut child);

        assert!(
            child
                .try_wait()
                .expect("the terminated process should be queryable")
                .is_some()
        );
    }
}
