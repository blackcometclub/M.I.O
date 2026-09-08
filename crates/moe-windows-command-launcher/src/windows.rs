use super::{
    PreparedWindowsCommandLaunch, WindowsCommandIsolationError, WindowsCommandLaunchCompletion,
    WindowsCommandLaunchOutcome,
};
use moe_command_broker::RegisteredTool;
use moe_command_helper_protocol::MAXIMUM_COMMAND_HELPER_REQUEST_BYTES;
use std::ffi::{OsStr, c_void};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::mem::{size_of, size_of_val};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(test)]
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, LocalFree, SetHandleInformation, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::{
    Authorization::ConvertSidToStringSidW,
    DeriveCapabilitySidsFromName, FreeSid,
    Isolation::{CreateAppContainerProfile, DeleteAppContainerProfile, GetAppContainerFolderPath},
    PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    UpdateProcThreadAttribute, WaitForSingleObject,
};
use windows_sys::core::PWSTR;

static PROFILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const MAXIMUM_STAGED_NPM_FILES: usize = 4_096;
const MAXIMUM_STAGED_NPM_DIRECTORIES: usize = 1_024;
const MAXIMUM_STAGED_NPM_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
const MAXIMUM_STAGED_NPM_TOTAL_BYTES: u64 = 64 * 1_024 * 1_024;

pub(super) struct IsolationResources {
    job: JobHandle,
    drive: WorkspaceDrive,
    tool_drive: Option<WorkspaceDrive>,
    grant: TemporaryAclGrant,
    staged_tool: Option<StagedTool>,
    tool_executable: PathBuf,
    profile: AppContainerProfile,
    #[cfg(test)]
    sid: String,
}

impl IsolationResources {
    pub(super) fn establish(
        prepared: &PreparedWindowsCommandLaunch,
    ) -> Result<Self, WindowsCommandIsolationError> {
        Self::establish_with_acl_scope(prepared, WorkspaceAclScope::Recursive)
    }

    #[cfg(test)]
    pub(super) fn establish_root_only_for_test(
        prepared: &PreparedWindowsCommandLaunch,
    ) -> Result<Self, WindowsCommandIsolationError> {
        Self::establish_with_acl_scope(prepared, WorkspaceAclScope::RootOnly)
    }

    fn establish_with_acl_scope(
        prepared: &PreparedWindowsCommandLaunch,
        acl_scope: WorkspaceAclScope,
    ) -> Result<Self, WindowsCommandIsolationError> {
        let profile = AppContainerProfile::create()?;
        let sid = profile.sid_string()?;
        let staged_tool = StagedTool::for_command(&profile, prepared)?;
        let grant = TemporaryAclGrant::grant(
            prepared.workspace_root(),
            &sid,
            prepared.workspace_writable(),
            acl_scope,
        )?;
        let drive = WorkspaceDrive::create(prepared.workspace_root())?;
        let tool_drive = staged_tool
            .as_ref()
            .map(|tool| WorkspaceDrive::create(&tool.directory))
            .transpose()?;
        let tool_executable = tool_drive.as_ref().map_or_else(
            || prepared.tool_executable().to_owned(),
            |drive| drive.root().join("node.exe"),
        );
        let job = JobHandle::create_kill_on_close()?;
        Ok(Self {
            job,
            drive,
            tool_drive,
            grant,
            staged_tool,
            tool_executable,
            profile,
            #[cfg(test)]
            sid,
        })
    }

    pub(super) fn workspace_drive_root(&self) -> &Path {
        self.drive.root()
    }

    pub(super) fn execute(
        &self,
        prepared: &PreparedWindowsCommandLaunch,
    ) -> Result<WindowsCommandLaunchOutcome, WindowsCommandIsolationError> {
        let request_pipe = InheritedPipe::input(MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1)?;
        let stdout_pipe = InheritedPipe::output()?;
        let stderr_pipe = InheritedPipe::output()?;

        let mut attribute_bytes = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut attribute_bytes);
        }
        if attribute_bytes == 0 {
            return Err(process_launch_failed("attribute-list-size"));
        }
        let words = attribute_bytes.div_ceil(size_of::<usize>());
        let mut attribute_storage = vec![0usize; words];
        let attribute_list = attribute_storage.as_mut_ptr().cast::<c_void>();
        if unsafe { InitializeProcThreadAttributeList(attribute_list, 2, 0, &mut attribute_bytes) }
            == 0
        {
            return Err(process_launch_failed("attribute-list-initialize"));
        }
        let attributes = ProcThreadAttributes(attribute_list);
        let internet_client = prepared
            .network_client_enabled()
            .then(|| DerivedCapabilitySid::new("internetClient"))
            .transpose()?;
        let mut capability_attribute =
            internet_client
                .as_ref()
                .map(|capability| SID_AND_ATTRIBUTES {
                    Sid: capability.sid,
                    Attributes: SE_GROUP_ENABLED as u32,
                });
        let capabilities = SECURITY_CAPABILITIES {
            AppContainerSid: self.profile.sid,
            Capabilities: capability_attribute
                .as_mut()
                .map_or(null_mut(), std::ptr::from_mut),
            CapabilityCount: u32::from(capability_attribute.is_some()),
            Reserved: 0,
        };
        if unsafe {
            UpdateProcThreadAttribute(
                attributes.0,
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                (&capabilities as *const SECURITY_CAPABILITIES).cast(),
                size_of::<SECURITY_CAPABILITIES>(),
                null_mut(),
                null(),
            )
        } == 0
        {
            return Err(process_launch_failed("security-capabilities"));
        }
        let inherited_handles = [
            request_pipe.child.raw(),
            stdout_pipe.child.raw(),
            stderr_pipe.child.raw(),
        ];
        if unsafe {
            UpdateProcThreadAttribute(
                attributes.0,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                inherited_handles.as_ptr().cast(),
                size_of_val(&inherited_handles),
                null_mut(),
                null(),
            )
        } == 0
        {
            return Err(process_launch_failed("inherited-handle-list"));
        }

        let application = wide(prepared.command_helper().as_os_str());
        let command_line = format!(
            "\"{}\" --tool-executable \"{}\"",
            prepared.command_helper().display(),
            self.tool_executable.display()
        );
        let mut command_line = wide(command_line);
        let current_directory = wide(self.drive.root().as_os_str());
        let app_data = self
            .tool_drive
            .as_ref()
            .map(|drive| drive.root().to_owned())
            .map_or_else(|| self.profile.folder_path(), Ok)?;
        let mut environment = clean_environment_block(self.drive.root(), &app_data)?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = request_pipe.child.raw();
        startup.StartupInfo.hStdOutput = stdout_pipe.child.raw();
        startup.StartupInfo.hStdError = stderr_pipe.child.raw();
        startup.lpAttributeList = attributes.0;
        let mut process = PROCESS_INFORMATION::default();
        if unsafe {
            CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                null(),
                null(),
                1,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
                environment.as_mut_ptr().cast(),
                current_directory.as_ptr(),
                &startup.StartupInfo,
                &mut process,
            )
        } == 0
        {
            return Err(process_launch_failed("create-process"));
        }
        let process = ProcessHandles(process);
        if let Err(error) = self.job.assign(process.0.hProcess) {
            if unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(process.0.hProcess, 125)
            } == 0
            {
                return Err(WindowsCommandIsolationError::ProcessWaitFailed);
            }
            unsafe { WaitForSingleObject(process.0.hProcess, 5_000) };
            return Err(error);
        }
        drop(request_pipe.child);
        drop(stdout_pipe.child);
        drop(stderr_pipe.child);

        let mut request_writer = request_pipe.parent.into_file();
        if request_writer.write_all(prepared.request()).is_err() {
            self.job.terminate(125);
            return Err(WindowsCommandIsolationError::RequestWriteFailed);
        }
        drop(request_writer);
        let stdout = stdout_pipe.parent.into_file();
        let stderr = stderr_pipe.parent.into_file();
        let budget = Arc::new(OutputBudget {
            limit: prepared.maximum_output_bytes(),
            used: AtomicUsize::new(0),
            exceeded: AtomicBool::new(false),
        });
        let stdout_reader = capture_bounded(stdout, Arc::clone(&budget));
        let stderr_reader = capture_bounded(stderr, Arc::clone(&budget));

        if unsafe { ResumeThread(process.0.hThread) } == u32::MAX {
            self.job.terminate(125);
            return Err(WindowsCommandIsolationError::ProcessResumeFailed);
        }
        let wait_millis = prepared
            .maximum_seconds()
            .saturating_add(10)
            .saturating_mul(1_000)
            .min(u64::from(u32::MAX)) as u32;
        let wait = unsafe { WaitForSingleObject(process.0.hProcess, wait_millis) };
        let host_timed_out = match wait {
            WAIT_OBJECT_0 => false,
            WAIT_TIMEOUT => {
                self.job.terminate(124);
                let terminated = unsafe { WaitForSingleObject(process.0.hProcess, 5_000) };
                if terminated != WAIT_OBJECT_0 {
                    return Err(WindowsCommandIsolationError::ProcessWaitFailed);
                }
                true
            }
            _ => {
                self.job.terminate(125);
                return Err(WindowsCommandIsolationError::ProcessWaitFailed);
            }
        };
        let mut exit_code = 0u32;
        if unsafe { GetExitCodeProcess(process.0.hProcess, &mut exit_code) } == 0 {
            return Err(WindowsCommandIsolationError::ProcessExitUnavailable);
        }
        // The helper is the only process whose exit code is authoritative. It waits for the
        // fixed command before returning, so any process still alive in the Job at this point is
        // an unexpected descendant. Terminate those descendants before joining the pipe readers;
        // otherwise an inherited stdout/stderr handle could keep the host blocked indefinitely.
        if !host_timed_out {
            self.job.terminate(125);
        }
        let stdout = stdout_reader
            .join()
            .map_err(|_| WindowsCommandIsolationError::OutputReaderPanicked)??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| WindowsCommandIsolationError::OutputReaderPanicked)??;
        let completion = if host_timed_out {
            WindowsCommandLaunchCompletion::HostTimedOut
        } else if budget.exceeded.load(Ordering::Relaxed) {
            WindowsCommandLaunchCompletion::OutputLimitExceeded
        } else {
            WindowsCommandLaunchCompletion::Completed
        };
        Ok(WindowsCommandLaunchOutcome {
            completion,
            exit_code,
            stdout,
            stderr,
        })
    }

    #[cfg(test)]
    pub(super) fn sid(&self) -> &str {
        &self.sid
    }

    pub(super) fn close(&mut self) -> Result<(), WindowsCommandIsolationError> {
        let mut failed = false;
        failed |= self.job.close().is_err();
        if let Some(tool_drive) = self.tool_drive.as_mut() {
            failed |= tool_drive.remove().is_err();
        }
        failed |= self.drive.remove().is_err();
        failed |= self.grant.remove().is_err();
        if let Some(tool) = self.staged_tool.as_mut() {
            failed |= tool.remove().is_err();
        }
        failed |= self.profile.delete().is_err();
        if failed {
            Err(WindowsCommandIsolationError::CleanupFailed)
        } else {
            Ok(())
        }
    }
}

struct StagedTool {
    directory: PathBuf,
    path: PathBuf,
    active: bool,
}

impl StagedTool {
    fn for_command(
        profile: &AppContainerProfile,
        prepared: &PreparedWindowsCommandLaunch,
    ) -> Result<Option<Self>, WindowsCommandIsolationError> {
        if !matches!(prepared.tool(), RegisteredTool::Node | RegisteredTool::Npm) {
            return Ok(None);
        }
        let directory = profile.folder_path()?.join("MIOCommandTool");
        fs::create_dir(&directory).map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
        let path = directory.join("node.exe");
        let staged = Self {
            directory,
            path,
            active: true,
        };
        fs::copy(prepared.tool_executable(), &staged.path)
            .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
        fs::create_dir(staged.directory.join("Temp"))
            .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
        if let Some(npm_runtime) = prepared.npm_runtime() {
            let destination = staged.directory.join("mio-npm-runtime").join("npm");
            copy_npm_runtime(npm_runtime, &destination)?;
        }
        Ok(Some(staged))
    }

    fn remove(&mut self) -> Result<(), WindowsCommandIsolationError> {
        if !self.active {
            return Ok(());
        }
        fs::remove_dir_all(&self.directory)
            .map_err(|_| WindowsCommandIsolationError::CleanupFailed)?;
        self.active = false;
        Ok(())
    }
}

fn copy_npm_runtime(source: &Path, destination: &Path) -> Result<(), WindowsCommandIsolationError> {
    if !source.is_absolute()
        || !source.is_dir()
        || filesystem_entry_is_link(source)?
        || destination.exists()
    {
        return Err(WindowsCommandIsolationError::ToolStagingFailed);
    }
    fs::create_dir_all(destination).map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
    let mut pending = vec![(source.to_owned(), destination.to_owned())];
    let mut file_count = 0usize;
    let mut directory_count = 0usize;
    let mut total_bytes = 0u64;
    while let Some((source_directory, destination_directory)) = pending.pop() {
        directory_count = directory_count
            .checked_add(1)
            .filter(|count| *count <= MAXIMUM_STAGED_NPM_DIRECTORIES)
            .ok_or(WindowsCommandIsolationError::ToolStagingFailed)?;
        for entry in fs::read_dir(&source_directory)
            .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?
        {
            let entry = entry.map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
            let source_path = entry.path();
            if filesystem_entry_is_link(&source_path)? {
                return Err(WindowsCommandIsolationError::ToolStagingFailed);
            }
            let destination_path = destination_directory.join(entry.file_name());
            let file_type = entry
                .file_type()
                .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
            if file_type.is_dir() {
                fs::create_dir(&destination_path)
                    .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
                pending.push((source_path, destination_path));
                continue;
            }
            if !file_type.is_file() {
                return Err(WindowsCommandIsolationError::ToolStagingFailed);
            }
            file_count = file_count
                .checked_add(1)
                .filter(|count| *count <= MAXIMUM_STAGED_NPM_FILES)
                .ok_or(WindowsCommandIsolationError::ToolStagingFailed)?;
            let length = entry
                .metadata()
                .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?
                .len();
            if length > MAXIMUM_STAGED_NPM_FILE_BYTES {
                return Err(WindowsCommandIsolationError::ToolStagingFailed);
            }
            total_bytes = total_bytes
                .checked_add(length)
                .filter(|total| *total <= MAXIMUM_STAGED_NPM_TOTAL_BYTES)
                .ok_or(WindowsCommandIsolationError::ToolStagingFailed)?;
            let copied = fs::copy(&source_path, &destination_path)
                .map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
            if copied != length {
                return Err(WindowsCommandIsolationError::ToolStagingFailed);
            }
        }
    }
    if file_count == 0
        || !destination.join("bin").join("npm-cli.js").is_file()
        || !destination.join("package.json").is_file()
    {
        return Err(WindowsCommandIsolationError::ToolStagingFailed);
    }
    Ok(())
}

fn filesystem_entry_is_link(path: &Path) -> Result<bool, WindowsCommandIsolationError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata =
        fs::symlink_metadata(path).map_err(|_| WindowsCommandIsolationError::ToolStagingFailed)?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

impl Drop for StagedTool {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

fn process_launch_failed(stage: &'static str) -> WindowsCommandIsolationError {
    #[cfg(test)]
    eprintln!("process launch failed at {stage}: win32={}", unsafe {
        GetLastError()
    });
    #[cfg(not(test))]
    let _ = stage;
    WindowsCommandIsolationError::ProcessLaunchFailed
}

struct DerivedCapabilitySid {
    sid: PSID,
}

impl DerivedCapabilitySid {
    fn new(name: &str) -> Result<Self, WindowsCommandIsolationError> {
        let name = wide(name);
        let mut group_sids: *mut PSID = null_mut();
        let mut group_count = 0u32;
        let mut capability_sids: *mut PSID = null_mut();
        let mut capability_count = 0u32;
        if unsafe {
            DeriveCapabilitySidsFromName(
                name.as_ptr(),
                &mut group_sids,
                &mut group_count,
                &mut capability_sids,
                &mut capability_count,
            )
        } == 0
        {
            #[cfg(test)]
            eprintln!("DeriveCapabilitySidsFromName failed: win32={}", unsafe {
                GetLastError()
            });
            return Err(WindowsCommandIsolationError::NetworkCapabilityUnavailable);
        }
        if capability_count != 1 || capability_sids.is_null() {
            unsafe {
                free_local_sid_array(group_sids, group_count);
                free_local_sid_array(capability_sids, capability_count);
            }
            #[cfg(test)]
            eprintln!(
                "DeriveCapabilitySidsFromName returned group_count={group_count} capability_count={capability_count}"
            );
            return Err(WindowsCommandIsolationError::NetworkCapabilityUnavailable);
        }
        let sid = unsafe { *capability_sids };
        unsafe {
            free_local_sid_array(group_sids, group_count);
            LocalFree(capability_sids.cast());
        }
        if sid.is_null() {
            return Err(WindowsCommandIsolationError::NetworkCapabilityUnavailable);
        }
        Ok(Self { sid })
    }
}

impl Drop for DerivedCapabilitySid {
    fn drop(&mut self) {
        if !self.sid.is_null() {
            unsafe {
                LocalFree(self.sid.cast());
            }
            self.sid = null_mut();
        }
    }
}

unsafe fn free_local_sid_array(array: *mut PSID, count: u32) {
    if array.is_null() {
        return;
    }
    for index in 0..count as usize {
        let sid = unsafe { *array.add(index) };
        if !sid.is_null() {
            unsafe {
                LocalFree(sid.cast());
            }
        }
    }
    unsafe {
        LocalFree(array.cast());
    }
}

impl Drop for IsolationResources {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

struct AppContainerProfile {
    name: Vec<u16>,
    sid: PSID,
    active: bool,
}

impl AppContainerProfile {
    fn create() -> Result<Self, WindowsCommandIsolationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| WindowsCommandIsolationError::ProfileUnavailable)?
            .as_nanos();
        let sequence = PROFILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = wide(format!(
            "MIO.Command.{}.{}.{}",
            std::process::id(),
            now,
            sequence
        ));
        let display_name = wide("M.I.O. isolated command");
        let description = wide("Temporary profile for one isolated M.I.O. command session");
        let mut sid = null_mut();
        let result = unsafe {
            CreateAppContainerProfile(
                name.as_ptr(),
                display_name.as_ptr(),
                description.as_ptr(),
                null(),
                0,
                &mut sid,
            )
        };
        if result < 0 || sid.is_null() {
            #[cfg(test)]
            eprintln!(
                "CreateAppContainerProfile failed: hresult={result:#x} sid_null={}",
                sid.is_null()
            );
            return Err(WindowsCommandIsolationError::ProfileUnavailable);
        }
        Ok(Self {
            name,
            sid,
            active: true,
        })
    }

    fn sid_string(&self) -> Result<String, WindowsCommandIsolationError> {
        let mut value: PWSTR = null_mut();
        if unsafe { ConvertSidToStringSidW(self.sid, &mut value) } == 0 || value.is_null() {
            #[cfg(test)]
            eprintln!("ConvertSidToStringSidW failed: win32={}", unsafe {
                GetLastError()
            });
            return Err(WindowsCommandIsolationError::ProfileUnavailable);
        }
        let mut length = 0usize;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        let result = String::from_utf16(unsafe { std::slice::from_raw_parts(value, length) })
            .map_err(|_| WindowsCommandIsolationError::ProfileUnavailable);
        unsafe {
            LocalFree(value.cast());
        }
        result
    }

    fn folder_path(&self) -> Result<PathBuf, WindowsCommandIsolationError> {
        let sid = wide(self.sid_string()?);
        let mut value: PWSTR = null_mut();
        let result = unsafe { GetAppContainerFolderPath(sid.as_ptr(), &mut value) };
        if result < 0 || value.is_null() {
            return Err(WindowsCommandIsolationError::ProfileUnavailable);
        }
        let mut length = 0usize;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        let path = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
            std::slice::from_raw_parts(value, length)
        }));
        unsafe {
            CoTaskMemFree(value.cast());
        }
        if path.is_absolute() {
            Ok(path)
        } else {
            Err(WindowsCommandIsolationError::ProfileUnavailable)
        }
    }

    fn delete(&mut self) -> Result<(), WindowsCommandIsolationError> {
        if !self.active {
            return Ok(());
        }
        let result = unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
        if result < 0 {
            return Err(WindowsCommandIsolationError::CleanupFailed);
        }
        unsafe {
            FreeSid(self.sid);
        }
        self.sid = null_mut();
        self.active = false;
        Ok(())
    }
}

impl Drop for AppContainerProfile {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                DeleteAppContainerProfile(self.name.as_ptr());
                FreeSid(self.sid);
            }
            self.sid = null_mut();
            self.active = false;
        }
    }
}

struct TemporaryAclGrant {
    path: PathBuf,
    sid: String,
    scope: WorkspaceAclScope,
    active: bool,
}

#[derive(Clone, Copy)]
enum WorkspaceAclScope {
    Recursive,
    #[cfg(test)]
    RootOnly,
}

impl TemporaryAclGrant {
    fn grant(
        path: &Path,
        sid: &str,
        writable: bool,
        scope: WorkspaceAclScope,
    ) -> Result<Self, WindowsCommandIsolationError> {
        let mut grant = Self {
            path: path.to_owned(),
            sid: sid.to_owned(),
            scope,
            active: true,
        };
        let mut command = system_command("icacls.exe")?;
        let rights = if writable { "M" } else { "RX" };
        command
            .arg(path)
            .arg("/grant:r")
            .arg(format!("*{sid}:(OI)(CI){rights}"));
        if matches!(scope, WorkspaceAclScope::Recursive) {
            command
                .arg("/T")
                // Existing children that do not inherit must be handled explicitly. Continue past
                // inaccessible items while protected paths remain fail-closed.
                .arg("/C");
        }
        let status = command
            .arg("/L")
            .arg("/Q")
            .status()
            .map_err(|_| WindowsCommandIsolationError::WorkspaceGrantFailed)?;
        if !status.success() {
            let _ = grant.remove();
            return Err(WindowsCommandIsolationError::WorkspaceGrantFailed);
        }
        Ok(grant)
    }

    fn remove(&mut self) -> Result<(), WindowsCommandIsolationError> {
        if !self.active {
            return Ok(());
        }
        let mut command = system_command("icacls.exe")?;
        command
            .arg(&self.path)
            .arg("/remove:g")
            .arg(format!("*{}", self.sid));
        if matches!(self.scope, WorkspaceAclScope::Recursive) {
            command
                .arg("/T")
                // Continue after inherited entries disappear so explicit descendant grants are
                // still removed.
                .arg("/C");
        }
        let status = command
            .arg("/L")
            .arg("/Q")
            .status()
            .map_err(|_| WindowsCommandIsolationError::CleanupFailed)?;
        if status.success() {
            self.active = false;
            Ok(())
        } else {
            Err(WindowsCommandIsolationError::CleanupFailed)
        }
    }
}

impl Drop for TemporaryAclGrant {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

struct WorkspaceDrive {
    name: String,
    root: PathBuf,
    active: bool,
}

impl WorkspaceDrive {
    fn create(workspace: &Path) -> Result<Self, WindowsCommandIsolationError> {
        let mask = unsafe { GetLogicalDrives() };
        if mask == 0 {
            return Err(WindowsCommandIsolationError::DriveUnavailable);
        }
        for letter in (b'P'..=b'Z').rev() {
            if mask & (1 << (letter - b'A')) != 0 {
                continue;
            }
            let name = format!("{}:", char::from(letter));
            let status = system_command("subst.exe")?
                .arg(&name)
                .arg(workspace)
                .status()
                .map_err(|_| WindowsCommandIsolationError::DriveMappingFailed)?;
            if status.success() {
                return Ok(Self {
                    root: PathBuf::from(format!("{name}\\")),
                    name,
                    active: true,
                });
            }
        }
        Err(WindowsCommandIsolationError::DriveUnavailable)
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn remove(&mut self) -> Result<(), WindowsCommandIsolationError> {
        if !self.active {
            return Ok(());
        }
        let status = system_command("subst.exe")?
            .arg(&self.name)
            .arg("/D")
            .status()
            .map_err(|_| WindowsCommandIsolationError::CleanupFailed)?;
        if status.success() {
            self.active = false;
            Ok(())
        } else {
            Err(WindowsCommandIsolationError::CleanupFailed)
        }
    }
}

impl Drop for WorkspaceDrive {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

struct JobHandle(HANDLE);

impl JobHandle {
    fn create_kill_on_close() -> Result<Self, WindowsCommandIsolationError> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(WindowsCommandIsolationError::JobUnavailable);
        }
        let job = Self(handle);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(WindowsCommandIsolationError::JobUnavailable);
        }
        Ok(job)
    }

    fn assign(&self, process: HANDLE) -> Result<(), WindowsCommandIsolationError> {
        if unsafe { AssignProcessToJobObject(self.0, process) } == 0 {
            Err(WindowsCommandIsolationError::JobAssignmentFailed)
        } else {
            Ok(())
        }
    }

    fn terminate(&self, exit_code: u32) {
        if !self.0.is_null() {
            unsafe {
                TerminateJobObject(self.0, exit_code);
            }
        }
    }

    fn close(&mut self) -> Result<(), WindowsCommandIsolationError> {
        if self.0.is_null() {
            return Ok(());
        }
        if unsafe { CloseHandle(self.0) } == 0 {
            return Err(WindowsCommandIsolationError::CleanupFailed);
        }
        self.0 = null_mut();
        Ok(())
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
            self.0 = null_mut();
        }
    }
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn raw(&self) -> HANDLE {
        self.0
    }

    fn into_file(mut self) -> File {
        let handle = self.0;
        self.0 = null_mut();
        unsafe { File::from_raw_handle(handle.cast()) }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
            self.0 = null_mut();
        }
    }
}

struct InheritedPipe {
    child: OwnedHandle,
    parent: OwnedHandle,
}

impl InheritedPipe {
    fn input(buffer_bytes: usize) -> Result<Self, WindowsCommandIsolationError> {
        let (read, write) = create_pipe(buffer_bytes, false)?;
        Ok(Self {
            child: read,
            parent: write,
        })
    }

    fn output() -> Result<Self, WindowsCommandIsolationError> {
        let (read, write) = create_pipe(0, true)?;
        Ok(Self {
            child: write,
            parent: read,
        })
    }
}

fn create_pipe(
    buffer_bytes: usize,
    output: bool,
) -> Result<(OwnedHandle, OwnedHandle), WindowsCommandIsolationError> {
    let mut read = null_mut();
    let mut write = null_mut();
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    if unsafe {
        CreatePipe(
            &mut read,
            &mut write,
            &attributes,
            buffer_bytes.min(u32::MAX as usize) as u32,
        )
    } == 0
    {
        return Err(if output {
            WindowsCommandIsolationError::OutputPipeUnavailable
        } else {
            WindowsCommandIsolationError::RequestPipeUnavailable
        });
    }
    let handles = (OwnedHandle(read), OwnedHandle(write));
    let parent = if output {
        handles.0.raw()
    } else {
        handles.1.raw()
    };
    if unsafe { SetHandleInformation(parent, HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(if output {
            WindowsCommandIsolationError::OutputPipeUnavailable
        } else {
            WindowsCommandIsolationError::RequestPipeUnavailable
        });
    }
    Ok(handles)
}

struct ProcThreadAttributes(*mut c_void);

impl Drop for ProcThreadAttributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.0);
        }
    }
}

struct ProcessHandles(PROCESS_INFORMATION);

impl Drop for ProcessHandles {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0.hThread);
            CloseHandle(self.0.hProcess);
        }
    }
}

struct OutputBudget {
    limit: usize,
    used: AtomicUsize,
    exceeded: AtomicBool,
}

fn capture_bounded(
    mut file: File,
    budget: Arc<OutputBudget>,
) -> thread::JoinHandle<Result<Vec<u8>, WindowsCommandIsolationError>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0_u8; 8 * 1_024];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| WindowsCommandIsolationError::OutputReadFailed)?;
            if count == 0 {
                break;
            }
            let previous = budget
                .used
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                    Some(
                        used.saturating_add(count)
                            .min(budget.limit.saturating_add(1)),
                    )
                })
                .unwrap_or_else(|used| used);
            let retained = budget.limit.saturating_sub(previous).min(count);
            captured.extend_from_slice(&buffer[..retained]);
            if retained < count {
                budget.exceeded.store(true, Ordering::Relaxed);
            }
        }
        Ok(captured)
    })
}

fn clean_environment_block(
    current_directory: &Path,
    app_data: &Path,
) -> Result<Vec<u16>, WindowsCommandIsolationError> {
    let system_directory = system_directory()?;
    let system_root = system_directory
        .parent()
        .filter(|path| path.is_absolute())
        .ok_or(WindowsCommandIsolationError::SystemToolUnavailable)?;
    let system_drive = system_root
        .components()
        .next()
        .map(|component| component.as_os_str().to_owned())
        .ok_or(WindowsCommandIsolationError::SystemToolUnavailable)?;
    let current_drive = current_directory
        .components()
        .next()
        .map(|component| component.as_os_str().to_owned())
        .ok_or(WindowsCommandIsolationError::DriveUnavailable)?;
    let temp = app_data.join("Temp");
    let mut variables = [
        format!(
            "={}={}",
            current_drive.to_string_lossy(),
            current_directory.display()
        ),
        format!("LOCALAPPDATA={}", app_data.display()),
        "NO_COLOR=1".to_owned(),
        format!("SystemDrive={}", system_drive.to_string_lossy()),
        format!("SystemRoot={}", system_root.display()),
        format!("TEMP={}", temp.display()),
        format!("TMP={}", temp.display()),
    ];
    variables.sort_by_key(|value| value.to_ascii_uppercase());
    let mut block = Vec::new();
    for variable in variables {
        block.extend(OsStr::new(&variable).encode_wide());
        block.push(0);
    }
    block.push(0);
    Ok(block)
}

fn system_directory() -> Result<PathBuf, WindowsCommandIsolationError> {
    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(WindowsCommandIsolationError::SystemToolUnavailable);
    }
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &buffer[..length],
    )))
}

fn system_command(name: &'static str) -> Result<Command, WindowsCommandIsolationError> {
    let system_directory = system_directory()?;
    let system_root = system_directory
        .parent()
        .filter(|path| path.is_absolute())
        .ok_or(WindowsCommandIsolationError::SystemToolUnavailable)?
        .to_owned();
    let executable = system_directory.join(name);
    let canonical_directory = system_directory
        .canonicalize()
        .map_err(|_| WindowsCommandIsolationError::SystemToolUnavailable)?;
    let canonical_executable = executable
        .canonicalize()
        .map_err(|_| WindowsCommandIsolationError::SystemToolUnavailable)?;
    if canonical_executable.parent() != Some(canonical_directory.as_path())
        || !canonical_executable
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(name))
    {
        return Err(WindowsCommandIsolationError::SystemToolUnavailable);
    }
    let mut command = Command::new(canonical_executable);
    command.env_clear().env("SystemRoot", system_root);
    Ok(command)
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
