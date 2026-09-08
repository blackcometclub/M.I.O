#[cfg(not(windows))]
fn main() {
    eprintln!("M.I.O. command helper is Windows-only.");
    std::process::exit(30);
}

#[cfg(windows)]
fn main() {
    std::process::exit(windows_helper::run());
}

#[cfg(windows)]
mod windows_helper {
    use moe_command_helper_protocol::{DecodedCommandRequest, decode_command_request_from_reader};
    use moe_windows_command_runner::{
        WindowsCommandCompletion, WindowsRunnerError, execute_npm_package_install,
        execute_prepared_command,
    };
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_PIPE, GetFileType};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};

    const ARG_TOOL_EXECUTABLE: &str = "--tool-executable";

    const EXIT_SUCCESS: i32 = 0;
    const EXIT_COMMAND_FAILED: i32 = 10;
    const EXIT_OUTPUT_LIMIT: i32 = 11;
    const EXIT_TIMEOUT: i32 = 12;
    const EXIT_PROTOCOL_REJECTED: i32 = 20;
    const EXIT_RUNNER_REJECTED: i32 = 21;
    const EXIT_INVALID_LAUNCH: i32 = 22;
    const EXIT_OUTPUT_WRITE_FAILED: i32 = 23;

    #[derive(Debug, PartialEq, Eq)]
    struct Invocation {
        tool_executable: PathBuf,
    }

    pub(super) fn run() -> i32 {
        let arguments = std::env::args_os().collect::<Vec<_>>();
        let invocation = match parse_invocation(&arguments) {
            Ok(invocation) => invocation,
            Err(()) => {
                eprintln!("M.I.O. command helper launch was rejected.");
                return EXIT_INVALID_LAUNCH;
            }
        };
        let workspace = match std::env::current_dir() {
            Ok(workspace) if workspace.is_absolute() => workspace,
            _ => {
                eprintln!("M.I.O. command helper workspace was rejected.");
                return EXIT_INVALID_LAUNCH;
            }
        };
        let input_handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        if input_handle.is_null() || unsafe { GetFileType(input_handle) } != FILE_TYPE_PIPE {
            eprintln!("M.I.O. command helper request channel was rejected.");
            return EXIT_INVALID_LAUNCH;
        }
        let input = io::stdin();
        let request = input.lock();
        match execute_request(request, &invocation.tool_executable, &workspace) {
            Ok(output) => output,
            Err(HelperError::Protocol) => {
                eprintln!("M.I.O. command helper request was rejected.");
                EXIT_PROTOCOL_REJECTED
            }
            Err(HelperError::Runner { code, os_code }) => {
                if let Some(os_code) = os_code {
                    eprintln!(
                        "M.I.O. command helper boundary was unavailable ({code}; win32={os_code})."
                    );
                } else {
                    eprintln!("M.I.O. command helper boundary was unavailable ({code}).");
                }
                EXIT_RUNNER_REJECTED
            }
            Err(HelperError::Output) => EXIT_OUTPUT_WRITE_FAILED,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum HelperError {
        Protocol,
        Runner {
            code: &'static str,
            os_code: Option<i32>,
        },
        Output,
    }

    fn execute_request<R: io::Read>(
        request: R,
        tool_executable: &Path,
        workspace: &Path,
    ) -> Result<i32, HelperError> {
        let request =
            decode_command_request_from_reader(request).map_err(|_| HelperError::Protocol)?;
        let outcome = match request {
            DecodedCommandRequest::Baseline(plan) => {
                execute_prepared_command(tool_executable, workspace, &plan)
            }
            DecodedCommandRequest::NpmPackageInstall(plan) => {
                execute_npm_package_install(tool_executable, workspace, &plan)
            }
        }
        .map_err(map_runner_error)?;
        io::stdout()
            .write_all(outcome.stdout())
            .map_err(|_| HelperError::Output)?;
        io::stderr()
            .write_all(outcome.stderr())
            .map_err(|_| HelperError::Output)?;
        Ok(match outcome.completion() {
            WindowsCommandCompletion::Completed if outcome.success() => EXIT_SUCCESS,
            WindowsCommandCompletion::Completed => EXIT_COMMAND_FAILED,
            WindowsCommandCompletion::OutputLimitExceeded => EXIT_OUTPUT_LIMIT,
            WindowsCommandCompletion::TimedOut => EXIT_TIMEOUT,
        })
    }

    fn map_runner_error(error: WindowsRunnerError) -> HelperError {
        HelperError::Runner {
            code: error.diagnostic_code(),
            os_code: error.diagnostic_os_code(),
        }
    }

    fn parse_invocation(arguments: &[OsString]) -> Result<Invocation, ()> {
        if arguments.len() != 3 || arguments[1] != OsStr::new(ARG_TOOL_EXECUTABLE) {
            return Err(());
        }
        let tool_executable = PathBuf::from(&arguments[2]);
        let is_registered = tool_executable
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.eq_ignore_ascii_case("git.exe") || name.eq_ignore_ascii_case("node.exe")
            });
        if !tool_executable.is_absolute() || !is_registered {
            return Err(());
        }
        Ok(Invocation { tool_executable })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use moe_command_broker::{
            BaselineCommand, CommandClassification, WorkspaceCommandAccess, WorkspaceCommandIntent,
            WorkspaceCommandRequest, classify_command,
        };
        use moe_command_helper_protocol::encode_request;

        #[test]
        fn accepts_only_the_exact_host_launch_shape() {
            let invocation = parse_invocation(&[
                OsString::from("mio-command-helper.exe"),
                OsString::from(ARG_TOOL_EXECUTABLE),
                OsString::from("C:\\Program Files\\Git\\cmd\\git.exe"),
            ])
            .unwrap();
            assert_eq!(
                invocation.tool_executable,
                Path::new("C:\\Program Files\\Git\\cmd\\git.exe")
            );

            let node = parse_invocation(&[
                OsString::from("mio-command-helper.exe"),
                OsString::from(ARG_TOOL_EXECUTABLE),
                OsString::from("C:\\Program Files\\nodejs\\node.exe"),
            ])
            .unwrap();
            assert_eq!(
                node.tool_executable,
                Path::new("C:\\Program Files\\nodejs\\node.exe")
            );
        }

        #[test]
        fn rejects_extra_flags_and_unresolved_tools() {
            for arguments in [
                vec![OsString::from("mio-command-helper.exe")],
                vec![
                    OsString::from("mio-command-helper.exe"),
                    OsString::from(ARG_TOOL_EXECUTABLE),
                    OsString::from("git.exe"),
                ],
                vec![
                    OsString::from("mio-command-helper.exe"),
                    OsString::from(ARG_TOOL_EXECUTABLE),
                    OsString::from("C:\\Windows\\System32\\cmd.exe"),
                ],
                vec![
                    OsString::from("mio-command-helper.exe"),
                    OsString::from(ARG_TOOL_EXECUTABLE),
                    OsString::from("C:\\Program Files\\nodejs\\npm.cmd"),
                ],
                vec![
                    OsString::from("mio-command-helper.exe"),
                    OsString::from(ARG_TOOL_EXECUTABLE),
                    OsString::from("C:\\Program Files\\Git\\cmd\\git.exe"),
                    OsString::from("--unexpected"),
                ],
            ] {
                assert_eq!(parse_invocation(&arguments), Err(()));
            }
        }

        #[test]
        fn normal_process_cannot_execute_an_otherwise_valid_request() {
            let classification = classify_command(
                WorkspaceCommandRequest::new(
                    PathBuf::from("."),
                    WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
                ),
                WorkspaceCommandAccess::ReadOnly,
            );
            let CommandClassification::Baseline(plan) = classification else {
                panic!("expected baseline plan");
            };
            let request = encode_request(&plan, WorkspaceCommandAccess::ReadOnly).unwrap();
            let error = execute_request(
                io::Cursor::new(request),
                Path::new("C:\\Program Files\\Git\\cmd\\git.exe"),
                Path::new("C:\\workspace"),
            )
            .unwrap_err();
            assert_eq!(
                error,
                HelperError::Runner {
                    code: "current_process_not_appcontainer",
                    os_code: None,
                }
            );
        }
    }
}
