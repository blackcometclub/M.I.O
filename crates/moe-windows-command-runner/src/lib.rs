//! Windows process backend for fixed Room workspace commands.
//!
//! The caller resolves the executable and establishes the AppContainer and
//! Job Object boundaries. This backend verifies those process boundaries
//! again immediately before starting a fixed command, clears the inherited
//! environment, and enforces the prepared output and time limits.

use moe_command_broker::{NpmPackageInstallPlan, RegisteredTool, WorkspaceCommandPlan};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsCommandCompletion {
    Completed,
    OutputLimitExceeded,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsCommandOutcome {
    completion: WindowsCommandCompletion,
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl WindowsCommandOutcome {
    pub fn completion(&self) -> WindowsCommandCompletion {
        self.completion
    }

    pub fn success(&self) -> bool {
        self.success
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

#[derive(Debug)]
pub enum WindowsRunnerError {
    UnsupportedPlatform,
    InvalidPlan,
    ExecutableNotAbsolute(PathBuf),
    WorkspaceNotAbsolute(PathBuf),
    UnsupportedTool(RegisteredTool),
    ExecutableNameMismatch {
        expected: &'static str,
        actual: PathBuf,
    },
    CurrentProcessNotAppContainer,
    CurrentProcessNotInJob,
    ChildProcessNotInJob,
    DeadlineOverflow,
    MissingOutputPipe(&'static str),
    OutputReaderPanicked(&'static str),
    Io {
        operation: &'static str,
        source: std::io::Error,
    },
}

impl fmt::Display for WindowsRunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => write!(formatter, "the Windows runner is Windows-only"),
            Self::InvalidPlan => write!(formatter, "the command runner rejected the plan"),
            Self::ExecutableNotAbsolute(path) => {
                write!(
                    formatter,
                    "executable path is not absolute: {}",
                    path.display()
                )
            }
            Self::WorkspaceNotAbsolute(path) => {
                write!(
                    formatter,
                    "workspace path is not absolute: {}",
                    path.display()
                )
            }
            Self::UnsupportedTool(tool) => {
                write!(
                    formatter,
                    "the Windows runner does not yet support {tool:?}"
                )
            }
            Self::ExecutableNameMismatch { expected, actual } => write!(
                formatter,
                "expected resolved executable {expected}, got {}",
                actual.display()
            ),
            Self::CurrentProcessNotAppContainer => {
                write!(
                    formatter,
                    "the runner process is not inside an AppContainer"
                )
            }
            Self::CurrentProcessNotInJob => {
                write!(formatter, "the runner process is not inside a Job Object")
            }
            Self::ChildProcessNotInJob => {
                write!(
                    formatter,
                    "the command child did not inherit the Job Object"
                )
            }
            Self::DeadlineOverflow => write!(formatter, "the command deadline overflowed"),
            Self::MissingOutputPipe(stream) => {
                write!(formatter, "the command {stream} pipe was unavailable")
            }
            Self::OutputReaderPanicked(stream) => {
                write!(formatter, "the command {stream} reader panicked")
            }
            Self::Io { operation, source } => write!(formatter, "{operation}: {source}"),
        }
    }
}

impl std::error::Error for WindowsRunnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl WindowsRunnerError {
    pub fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::InvalidPlan => "invalid_plan",
            Self::ExecutableNotAbsolute(_) => "executable_not_absolute",
            Self::WorkspaceNotAbsolute(_) => "workspace_not_absolute",
            Self::UnsupportedTool(_) => "unsupported_tool",
            Self::ExecutableNameMismatch { .. } => "executable_name_mismatch",
            Self::CurrentProcessNotAppContainer => "current_process_not_appcontainer",
            Self::CurrentProcessNotInJob => "current_process_not_in_job",
            Self::ChildProcessNotInJob => "child_process_not_in_job",
            Self::DeadlineOverflow => "deadline_overflow",
            Self::MissingOutputPipe(_) => "missing_output_pipe",
            Self::OutputReaderPanicked(_) => "output_reader_panicked",
            Self::Io { operation, .. } => operation,
        }
    }

    pub fn diagnostic_os_code(&self) -> Option<i32> {
        match self {
            Self::Io { source, .. } => source.raw_os_error(),
            _ => None,
        }
    }
}

fn validate_inputs(
    executable: &Path,
    workspace: &Path,
    plan: &WorkspaceCommandPlan,
) -> Result<(), WindowsRunnerError> {
    if !executable.is_absolute() {
        return Err(WindowsRunnerError::ExecutableNotAbsolute(
            executable.to_owned(),
        ));
    }
    if !workspace.is_absolute() {
        return Err(WindowsRunnerError::WorkspaceNotAbsolute(
            workspace.to_owned(),
        ));
    }
    let expected = match plan.tool() {
        RegisteredTool::Git => "git.exe",
        RegisteredTool::Node | RegisteredTool::Npm => "node.exe",
        RegisteredTool::Cargo => {
            return Err(WindowsRunnerError::UnsupportedTool(plan.tool()));
        }
    };
    let matches = executable
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected));
    if !matches {
        return Err(WindowsRunnerError::ExecutableNameMismatch {
            expected,
            actual: executable.to_owned(),
        });
    }
    Ok(())
}

fn validate_npm_package_install_inputs(
    executable: &Path,
    workspace: &Path,
) -> Result<(), WindowsRunnerError> {
    if !executable.is_absolute() {
        return Err(WindowsRunnerError::ExecutableNotAbsolute(
            executable.to_owned(),
        ));
    }
    if !workspace.is_absolute() {
        return Err(WindowsRunnerError::WorkspaceNotAbsolute(
            workspace.to_owned(),
        ));
    }
    let matches = executable
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("node.exe"));
    if !matches {
        return Err(WindowsRunnerError::ExecutableNameMismatch {
            expected: "node.exe",
            actual: executable.to_owned(),
        });
    }
    Ok(())
}

fn resolve_working_directory(workspace: &Path, relative: &Path) -> PathBuf {
    if relative == Path::new(".") {
        workspace.to_owned()
    } else {
        workspace.join(relative)
    }
}

#[cfg(windows)]
pub fn execute_prepared_command(
    executable: &Path,
    workspace: &Path,
    plan: &WorkspaceCommandPlan,
) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
    windows::execute(executable, workspace, plan)
}

#[cfg(windows)]
pub fn execute_npm_package_install(
    executable: &Path,
    workspace: &Path,
    plan: &NpmPackageInstallPlan,
) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
    windows::execute_npm_package_install(executable, workspace, plan)
}

#[cfg(not(windows))]
pub fn execute_npm_package_install(
    executable: &Path,
    workspace: &Path,
    _plan: &NpmPackageInstallPlan,
) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
    validate_npm_package_install_inputs(executable, workspace)?;
    Err(WindowsRunnerError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn execute_prepared_command(
    executable: &Path,
    workspace: &Path,
    plan: &WorkspaceCommandPlan,
) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
    validate_inputs(executable, workspace, plan)?;
    Err(WindowsRunnerError::UnsupportedPlatform)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use moe_command_runner::{
        STAGED_NPM_CLI_RELATIVE_PATH, prepare_command, prepare_npm_package_install,
    };
    use std::ffi::c_void;
    use std::io::Read;
    use std::os::windows::io::AsRawHandle;
    use std::process::{Child, Command, Stdio};
    use std::ptr::null_mut;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use std::thread;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_QUERY, TokenIsAppContainer},
        System::{
            JobObjects::IsProcessInJob,
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
    };

    struct OutputBudget {
        limit: usize,
        used: AtomicUsize,
        exceeded: AtomicBool,
    }

    pub(super) fn execute(
        executable: &Path,
        workspace: &Path,
        plan: &WorkspaceCommandPlan,
    ) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
        validate_inputs(executable, workspace, plan)?;
        let prepared = prepare_command(plan).map_err(|_| WindowsRunnerError::InvalidPlan)?;
        if !current_process_is_appcontainer()? {
            return Err(WindowsRunnerError::CurrentProcessNotAppContainer);
        }
        if !process_is_in_job(unsafe { GetCurrentProcess() })? {
            return Err(WindowsRunnerError::CurrentProcessNotInJob);
        }

        let working_directory = resolve_working_directory(workspace, prepared.working_directory());
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let system_drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());
        let mut command = Command::new(executable);
        if prepared.tool() == RegisteredTool::Git {
            command
                .arg("-c")
                .arg(format!("safe.directory={}", workspace.display()));
        }
        let npm_environment = if prepared.tool() == RegisteredTool::Npm {
            let executable_root =
                executable
                    .parent()
                    .ok_or_else(|| WindowsRunnerError::ExecutableNameMismatch {
                        expected: "node.exe",
                        actual: executable.to_owned(),
                    })?;
            let npm_cli = executable_root.join(STAGED_NPM_CLI_RELATIVE_PATH);
            if !npm_cli.is_absolute() || !npm_cli.is_file() || !npm_cli.starts_with(executable_root)
            {
                return Err(WindowsRunnerError::InvalidPlan);
            }
            let local_app_data = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .ok_or(WindowsRunnerError::InvalidPlan)?;
            let temp = std::env::var_os("TEMP")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute() && path.starts_with(&local_app_data))
                .ok_or(WindowsRunnerError::InvalidPlan)?;
            command.arg(npm_cli);
            Some((local_app_data, temp))
        } else {
            None
        };
        command
            .args(prepared.arguments())
            .current_dir(working_directory)
            .env_clear()
            .env("SystemDrive", &system_drive)
            .env("SystemRoot", &system_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some((local_app_data, temp)) = npm_environment {
            command
                .env(
                    "ComSpec",
                    PathBuf::from(&system_root).join("System32").join("cmd.exe"),
                )
                .env("LOCALAPPDATA", &local_app_data)
                .env("TEMP", &temp)
                .env("TMP", &temp)
                .env("USERPROFILE", &local_app_data)
                .env("npm_config_cache", local_app_data.join("npm-cache"));
        }
        for (name, value) in prepared.environment() {
            command.env(name, value);
        }

        let child = command
            .spawn()
            .map_err(io_error("start fixed workspace command"))?;
        run_child(
            child,
            prepared.maximum_output_bytes(),
            Duration::from_secs(prepared.maximum_seconds()),
        )
    }

    pub(super) fn execute_npm_package_install(
        executable: &Path,
        workspace: &Path,
        plan: &NpmPackageInstallPlan,
    ) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
        validate_npm_package_install_inputs(executable, workspace)?;
        let prepared = prepare_npm_package_install(plan);
        if !current_process_is_appcontainer()? {
            return Err(WindowsRunnerError::CurrentProcessNotAppContainer);
        }
        if !process_is_in_job(unsafe { GetCurrentProcess() })? {
            return Err(WindowsRunnerError::CurrentProcessNotInJob);
        }

        let executable_root =
            executable
                .parent()
                .ok_or_else(|| WindowsRunnerError::ExecutableNameMismatch {
                    expected: "node.exe",
                    actual: executable.to_owned(),
                })?;
        let npm_cli = executable_root.join(STAGED_NPM_CLI_RELATIVE_PATH);
        if !npm_cli.is_absolute() || !npm_cli.is_file() || !npm_cli.starts_with(executable_root) {
            return Err(WindowsRunnerError::InvalidPlan);
        }
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or(WindowsRunnerError::InvalidPlan)?;
        let temp = std::env::var_os("TEMP")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.starts_with(&local_app_data))
            .ok_or(WindowsRunnerError::InvalidPlan)?;
        let working_directory = resolve_working_directory(workspace, prepared.working_directory());
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let system_drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());
        let mut command = Command::new(executable);
        let separator = prepared
            .arguments()
            .iter()
            .position(|argument| argument == "--")
            .ok_or(WindowsRunnerError::InvalidPlan)?;
        command
            .arg(&npm_cli)
            .args(&prepared.arguments()[1..separator])
            .arg("--prefix")
            .arg(&working_directory)
            .args(&prepared.arguments()[separator..])
            .current_dir(working_directory)
            .env_clear()
            .env("LOCALAPPDATA", &local_app_data)
            .env("SystemDrive", system_drive)
            .env("SystemRoot", system_root)
            .env("TEMP", &temp)
            .env("TMP", &temp)
            .env("USERPROFILE", &local_app_data)
            .env("npm_config_cache", local_app_data.join("npm-cache"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in prepared.environment() {
            command.env(name, value);
        }

        let child = command
            .spawn()
            .map_err(io_error("start fixed npm package install"))?;
        run_child(
            child,
            prepared.maximum_output_bytes(),
            Duration::from_secs(prepared.maximum_seconds()),
        )
    }

    fn run_child(
        mut child: Child,
        maximum_output_bytes: usize,
        maximum_duration: Duration,
    ) -> Result<WindowsCommandOutcome, WindowsRunnerError> {
        match process_is_in_job(child.as_raw_handle().cast()) {
            Ok(true) => {}
            Ok(false) => {
                terminate_and_reap(&mut child);
                return Err(WindowsRunnerError::ChildProcessNotInJob);
            }
            Err(error) => {
                terminate_and_reap(&mut child);
                return Err(error);
            }
        }
        let deadline = Instant::now()
            .checked_add(maximum_duration)
            .ok_or(WindowsRunnerError::DeadlineOverflow)?;
        let budget = Arc::new(OutputBudget {
            limit: maximum_output_bytes,
            used: AtomicUsize::new(0),
            exceeded: AtomicBool::new(false),
        });
        let stdout = child.stdout.take().ok_or_else(|| {
            terminate_and_reap(&mut child);
            WindowsRunnerError::MissingOutputPipe("stdout")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            terminate_and_reap(&mut child);
            WindowsRunnerError::MissingOutputPipe("stderr")
        })?;
        let stdout_reader = capture_bounded(stdout, Arc::clone(&budget), "stdout");
        let stderr_reader = capture_bounded(stderr, Arc::clone(&budget), "stderr");
        let (status, timed_out) = loop {
            match child.try_wait() {
                Ok(Some(status)) => break (status, false),
                Ok(None) if Instant::now() >= deadline => {
                    if let Err(source) = child.kill() {
                        terminate_and_reap(&mut child);
                        return Err(WindowsRunnerError::Io {
                            operation: "terminate timed-out workspace command",
                            source,
                        });
                    }
                    let status = child
                        .wait()
                        .map_err(io_error("reap timed-out workspace command"))?;
                    break (status, true);
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(source) => {
                    terminate_and_reap(&mut child);
                    return Err(WindowsRunnerError::Io {
                        operation: "poll fixed workspace command",
                        source,
                    });
                }
            }
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| WindowsRunnerError::OutputReaderPanicked("stdout"))??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| WindowsRunnerError::OutputReaderPanicked("stderr"))??;
        let completion = if timed_out {
            WindowsCommandCompletion::TimedOut
        } else if budget.exceeded.load(Ordering::Relaxed) {
            WindowsCommandCompletion::OutputLimitExceeded
        } else {
            WindowsCommandCompletion::Completed
        };
        Ok(WindowsCommandOutcome {
            completion,
            success: status.success() && completion == WindowsCommandCompletion::Completed,
            exit_code: status.code(),
            stdout,
            stderr,
        })
    }

    fn capture_bounded<R>(
        mut reader: R,
        budget: Arc<OutputBudget>,
        stream: &'static str,
    ) -> thread::JoinHandle<Result<Vec<u8>, WindowsRunnerError>>
    where
        R: Read + Send + 'static,
    {
        thread::spawn(move || {
            let mut captured = Vec::new();
            let mut buffer = [0u8; 8 * 1_024];
            loop {
                let count = reader
                    .read(&mut buffer)
                    .map_err(io_error(if stream == "stdout" {
                        "read fixed command stdout"
                    } else {
                        "read fixed command stderr"
                    }))?;
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

    fn current_process_is_appcontainer() -> Result<bool, WindowsRunnerError> {
        let mut token: HANDLE = null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(last_os_error("open current process token"));
        }
        let mut is_appcontainer = 0u32;
        let mut returned_length = 0u32;
        let result = unsafe {
            GetTokenInformation(
                token,
                TokenIsAppContainer,
                (&mut is_appcontainer as *mut u32).cast::<c_void>(),
                size_of::<u32>() as u32,
                &mut returned_length,
            )
        };
        let query_error = if result == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };
        unsafe {
            CloseHandle(token);
        }
        if let Some(source) = query_error {
            return Err(WindowsRunnerError::Io {
                operation: "query AppContainer token state",
                source,
            });
        }
        Ok(is_appcontainer != 0)
    }

    fn process_is_in_job(process: HANDLE) -> Result<bool, WindowsRunnerError> {
        let mut in_job = 0;
        if unsafe { IsProcessInJob(process, null_mut(), &mut in_job) } == 0 {
            return Err(last_os_error("query process Job Object membership"));
        }
        Ok(in_job != 0)
    }

    fn terminate_and_reap(child: &mut Child) {
        let _ = child.kill();
        let _ = child.wait();
    }

    fn io_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> WindowsRunnerError {
        move |source| WindowsRunnerError::Io { operation, source }
    }

    fn last_os_error(operation: &'static str) -> WindowsRunnerError {
        WindowsRunnerError::Io {
            operation,
            source: std::io::Error::last_os_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_command_broker::{
        BaselineCommand, CommandClassification, WorkspaceCommandAccess, WorkspaceCommandIntent,
        WorkspaceCommandRequest, classify_command,
    };

    fn plan(command: BaselineCommand) -> WorkspaceCommandPlan {
        let classification = classify_command(
            WorkspaceCommandRequest::new(
                PathBuf::from("."),
                WorkspaceCommandIntent::Baseline(command),
            ),
            if command.writes_workspace() {
                WorkspaceCommandAccess::ReadWrite
            } else {
                WorkspaceCommandAccess::ReadOnly
            },
        );
        let CommandClassification::Baseline(plan) = classification else {
            panic!("expected baseline plan");
        };
        plan
    }

    #[test]
    fn rejects_relative_workspace_before_platform_checks() {
        let error = execute_prepared_command(
            Path::new("C:\\Program Files\\Git\\cmd\\git.exe"),
            Path::new("relative"),
            &plan(BaselineCommand::GitStatus),
        )
        .unwrap_err();
        assert!(matches!(error, WindowsRunnerError::WorkspaceNotAbsolute(_)));
    }

    #[test]
    fn rejects_unresolved_executable_before_platform_checks() {
        let error = execute_prepared_command(
            Path::new("git.exe"),
            Path::new("C:\\workspace"),
            &plan(BaselineCommand::GitStatus),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WindowsRunnerError::ExecutableNotAbsolute(_)
        ));
    }

    #[test]
    fn accepts_npm_only_through_the_staged_node_executable_name() {
        let error = execute_prepared_command(
            Path::new("C:\\Program Files\\nodejs\\npm.cmd"),
            Path::new("C:\\workspace"),
            &plan(BaselineCommand::NpmTest),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WindowsRunnerError::ExecutableNameMismatch {
                expected: "node.exe",
                ..
            }
        ));

        assert!(
            validate_inputs(
                Path::new("C:\\Program Files\\nodejs\\node.exe"),
                Path::new("C:\\workspace"),
                &plan(BaselineCommand::NpmTest),
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_executable_name_substitution_before_platform_checks() {
        let error = execute_prepared_command(
            Path::new("C:\\Windows\\System32\\cmd.exe"),
            Path::new("C:\\workspace"),
            &plan(BaselineCommand::GitStatus),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WindowsRunnerError::ExecutableNameMismatch {
                expected: "git.exe",
                ..
            }
        ));
    }

    #[test]
    fn rejects_node_executable_name_substitution_before_platform_checks() {
        let error = execute_prepared_command(
            Path::new("C:\\Windows\\System32\\cmd.exe"),
            Path::new("C:\\workspace"),
            &plan(BaselineCommand::NodeRun),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WindowsRunnerError::ExecutableNameMismatch {
                expected: "node.exe",
                ..
            }
        ));
    }

    #[test]
    fn exposes_only_the_numeric_os_code_for_io_diagnostics() {
        let error = WindowsRunnerError::Io {
            operation: "start fixed npm package install",
            source: std::io::Error::from_raw_os_error(5),
        };

        assert_eq!(error.diagnostic_code(), "start fixed npm package install");
        assert_eq!(error.diagnostic_os_code(), Some(5));
    }

    #[test]
    fn keeps_the_mapped_workspace_root_for_dot_working_directory() {
        let workspace = Path::new("Q:\\");

        assert_eq!(
            resolve_working_directory(workspace, Path::new(".")),
            workspace
        );
        assert_eq!(
            resolve_working_directory(workspace, Path::new("packages/app")),
            Path::new("Q:\\packages/app")
        );
    }
}
