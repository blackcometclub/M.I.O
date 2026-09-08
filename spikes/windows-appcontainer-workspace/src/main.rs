#[cfg(not(windows))]
fn main() {
    eprintln!("This probe is Windows-only.");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_probe::run() {
        eprintln!("AppContainer workspace probe failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod windows_probe {
    use moe_command_broker::{
        BaselineCommand, CommandClassification, MAXIMUM_COMMAND_OUTPUT_BYTES,
        WorkspaceCommandAccess, WorkspaceCommandIntent, WorkspaceCommandRequest, classify_command,
    };
    use moe_command_helper_protocol::{
        MAXIMUM_COMMAND_HELPER_REQUEST_BYTES, decode_request_from_reader, encode_request,
    };
    use std::{
        collections::BTreeMap,
        ffi::{OsStr, c_void},
        fs,
        io::{BufRead, BufReader, Read, Write},
        mem::{size_of, size_of_val},
        net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            io::{AsRawHandle, FromRawHandle},
        },
        path::{Path, PathBuf},
        process::{Command, Stdio},
        ptr::{null, null_mut},
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use windows_sys::{
        Win32::{
            Foundation::{
                CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, LocalFree, SetHandleInformation,
                WAIT_OBJECT_0,
            },
            Security::{
                Authorization::ConvertSidToStringSidW, FreeSid,
                Isolation::CreateAppContainerProfile, Isolation::DeleteAppContainerProfile,
                Isolation::GetAppContainerFolderPath, PSID, SECURITY_ATTRIBUTES,
                SECURITY_CAPABILITIES,
            },
            Storage::FileSystem::GetLogicalDrives,
            System::Com::CoTaskMemFree,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
            },
            System::Pipes::CreatePipe,
            System::Threading::{
                CREATE_SUSPENDED, CreateProcessW, DeleteProcThreadAttributeList,
                EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
                InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
                STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
            },
        },
        core::PWSTR,
    };

    const CHILD_FLAG: &str = "--appcontainer-child";
    const OUTPUT_LIMIT_CHILD_FLAG: &str = "--output-limit-child";
    const TIMEOUT_CHILD_FLAG: &str = "--timeout-child";
    const CHILD_TIMEOUT_MILLIS: u32 = 30_000;

    pub(super) fn run() -> Result<(), String> {
        let args = std::env::args_os().collect::<Vec<_>>();
        match args.get(1).and_then(|value| value.to_str()) {
            Some(CHILD_FLAG) => return child_run(&args),
            Some(OUTPUT_LIMIT_CHILD_FLAG) => {
                std::io::stdout()
                    .write_all(&vec![b'o'; 32 * 1_024])
                    .map_err(io_error("write output-limit stdout fixture"))?;
                std::io::stderr()
                    .write_all(&vec![b'e'; 32 * 1_024])
                    .map_err(io_error("write output-limit stderr fixture"))?;
                return Ok(());
            }
            Some(TIMEOUT_CHILD_FLAG) => {
                thread::sleep(Duration::from_secs(5));
                return Ok(());
            }
            _ => {}
        }
        parent_run()
    }

    fn parent_run() -> Result<(), String> {
        let run_id = unique_id()?;
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| "repository root could not be resolved".to_owned())?;
        let evidence_parent = repository_root
            .join(".tools")
            .join("appcontainer-workspace-spike");
        fs::create_dir_all(&evidence_parent).map_err(io_error("create evidence parent"))?;
        let run_root = evidence_parent.join(format!("run-{run_id}"));
        let workspace = run_root.join("workspace");
        let outside = run_root.join("outside");
        let launcher = run_root.join("launcher");
        fs::create_dir_all(&workspace).map_err(io_error("create workspace"))?;
        fs::create_dir_all(&outside).map_err(io_error("create outside fixture"))?;
        fs::create_dir_all(&launcher).map_err(io_error("create launcher directory"))?;

        let profile_name = format!("MIO.WorkspaceSpike.{run_id}");
        let mut profile = AppContainerProfile::create(&profile_name)?;
        let sid = profile.sid_string()?;
        let profile_folder = profile.folder_path(&sid)?;
        println!("appcontainer_sid={sid}");

        let codex_home = profile_folder.join("codex-home");
        let codex_temp = profile_folder.join("codex-temp");
        fs::create_dir_all(&codex_home).map_err(io_error("create AppContainer Codex home"))?;
        fs::create_dir_all(&codex_temp).map_err(io_error("create AppContainer temp"))?;

        let mut temporary_grants = TemporaryAclGrants::new(&sid);
        temporary_grants.grant(&profile_folder, "(OI)(CI)M")?;
        temporary_grants.grant(&workspace, "(OI)(CI)M")?;
        temporary_grants.grant(&launcher, "(OI)(CI)RX")?;

        fs::write(workspace.join("inside.txt"), b"inside-marker")
            .map_err(io_error("write inside marker"))?;
        fs::write(outside.join("outside.txt"), b"outside-marker")
            .map_err(io_error("write outside marker"))?;
        create_junction(&workspace.join("nested-link"), &outside)?;

        let git_program = resolve_program("git.exe")?;
        let git_init = Command::new(&git_program)
            .args(["init", "--quiet"])
            .current_dir(&workspace)
            .status()
            .map_err(io_error("start Git fixture initialization"))?;
        if !git_init.success() {
            return Err("Git fixture initialization failed".to_owned());
        }
        let network_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(io_error("bind network isolation fixture"))?;
        let network_port = network_listener
            .local_addr()
            .map_err(io_error("read network isolation fixture address"))?
            .port();
        let mut workspace_drive = WorkspaceDrive::create(&workspace)?;
        let command_request = classify_command(
            WorkspaceCommandRequest::new(
                PathBuf::from("."),
                WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
            ),
            WorkspaceCommandAccess::ReadOnly,
        );
        let CommandClassification::Baseline(command_plan) = command_request else {
            return Err("Git baseline command was not classified as baseline".to_owned());
        };
        let encoded_request = encode_request(&command_plan, WorkspaceCommandAccess::ReadOnly)
            .map_err(|error| format!("encode helper command request: {error:?}"))?;
        fs::write(
            run_root.join("command-request-evidence.json"),
            &encoded_request,
        )
        .map_err(io_error("write helper command request evidence"))?;

        let child_executable = launcher.join("probe-child.exe");
        fs::copy(
            std::env::current_exe().map_err(io_error("resolve current executable"))?,
            &child_executable,
        )
        .map_err(io_error("copy child executable"))?;
        let command_helper_source = std::env::current_exe()
            .map_err(io_error("resolve command helper directory"))?
            .parent()
            .map(|parent| parent.join("moe-command-helper.exe"))
            .ok_or_else(|| "command helper directory was unavailable".to_owned())?;
        if !command_helper_source.is_file() {
            return Err(format!(
                "command helper was not built: {}",
                command_helper_source.display()
            ));
        }
        let command_helper = launcher.join("moe-command-helper.exe");
        fs::copy(&command_helper_source, &command_helper)
            .map_err(io_error("copy command helper executable"))?;
        let codex_source = std::env::var_os("MOE_CODEX_BIN").map(PathBuf::from);
        let codex_requested = codex_source.is_some();
        if let Some(codex_source) = codex_source {
            if !codex_source.is_file() {
                return Err(format!(
                    "MOE_CODEX_BIN is not a file: {}",
                    codex_source.display()
                ));
            }
            fs::copy(codex_source, launcher.join("codex-under-test.exe"))
                .map_err(io_error("copy Codex executable under test"))?;
        }
        let report = workspace.join("child-report.txt");
        let exit_code = profile.launch_child(
            &child_executable,
            workspace_drive.path(),
            &outside,
            &report,
            &profile_folder,
            &git_program,
            &command_helper,
            &encoded_request,
            network_port,
        )?;
        if exit_code != 0 {
            return Err(format!("sandboxed child exited with code {exit_code}"));
        }
        let job_process_containment = true;

        let results = parse_report(&report)?;
        let boundary_keys = [
            "inside_read",
            "inside_write",
            "outside_read_blocked",
            "outside_write_blocked",
            "nested_read_blocked",
            "nested_write_blocked",
            "created_link_escape_blocked",
        ];
        let boundary_pass = boundary_keys
            .iter()
            .all(|key| results.get(*key) == Some(&true));
        let child_process_pass = results.get("child_process_write") == Some(&true);
        let network_pass = results.get("network_blocked") == Some(&true);
        let helper_request_pass = results.get("helper_request_decoded") == Some(&true);
        let command_helper_pass = results.get("command_helper_process") == Some(&true);
        let git_baseline_pass = results.get("git_baseline_status") == Some(&true);
        let output_limit_pass = results.get("output_limit_enforced") == Some(&true);
        let timeout_pass = results.get("timeout_enforced") == Some(&true);
        let toolchain_keys = [
            "git_process",
            "node_process",
            "npm_process",
            "cargo_process",
        ];
        let toolchain_pass = toolchain_keys
            .iter()
            .all(|key| results.get(*key) == Some(&true));
        let codex_pass = results.get("codex_process") == Some(&true);
        let codex_app_server_pass = results.get("codex_app_server_initialize") == Some(&true);
        let drive_cleanup_error = workspace_drive.remove().err();
        let acl_cleanup_error = temporary_grants.remove_all().err();
        let profile_cleanup_error = profile.delete().err();

        println!("evidence={}", run_root.display());
        println!(
            "temporary_workspace_drive_removed={}",
            drive_cleanup_error.is_none()
        );
        println!("job_process_containment={job_process_containment}");
        println!("temporary_acl_removed={}", acl_cleanup_error.is_none());
        println!(
            "appcontainer_profile_deleted={}",
            profile_cleanup_error.is_none()
        );
        for (key, value) in &results {
            println!("{key}={value}");
        }
        println!("BOUNDARY={}", if boundary_pass { "PASS" } else { "FAIL" });
        println!(
            "CHILD_PROCESS_COMPATIBILITY={}",
            if child_process_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "NETWORK_ISOLATION={}",
            if network_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "GIT_BASELINE_STATUS={}",
            if git_baseline_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "HELPER_REQUEST_PIPE={}",
            if helper_request_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "COMMAND_HELPER_PROCESS={}",
            if command_helper_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "RUNNER_OUTPUT_LIMIT={}",
            if output_limit_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "RUNNER_TIMEOUT={}",
            if timeout_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "TOOLCHAIN_COMPATIBILITY={}",
            if toolchain_pass { "PASS" } else { "FAIL" }
        );
        println!(
            "CODEX_CLI_COMPATIBILITY={}",
            if !codex_requested {
                "NOT_RUN"
            } else if codex_pass {
                "PASS"
            } else {
                "FAIL"
            }
        );
        println!(
            "CODEX_APP_SERVER_INITIALIZE={}",
            if !codex_requested {
                "NOT_RUN"
            } else if codex_app_server_pass {
                "PASS"
            } else {
                "FAIL"
            }
        );
        let codex_version = workspace.join("codex-version.txt");
        if codex_version.is_file() {
            let value = fs::read_to_string(codex_version)
                .map_err(io_error("read Codex version evidence"))?;
            println!("codex_version={}", value.trim());
        }

        if let Some(error) = drive_cleanup_error {
            return Err(error);
        }
        if let Some(error) = acl_cleanup_error {
            return Err(error);
        }
        if let Some(error) = profile_cleanup_error {
            return Err(error);
        }

        let required_processes_pass = if codex_requested {
            codex_pass && codex_app_server_pass
        } else {
            results.get("git_process") == Some(&true)
        };
        if !boundary_pass
            || !child_process_pass
            || !job_process_containment
            || !network_pass
            || !helper_request_pass
            || !command_helper_pass
            || !git_baseline_pass
            || !output_limit_pass
            || !timeout_pass
            || !required_processes_pass
        {
            return Err("one or more required observations failed".to_owned());
        }
        Ok(())
    }

    struct BoundedProcessOutput {
        status_success: bool,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        output_exceeded: bool,
        timed_out: bool,
        process_in_job: bool,
    }

    struct OutputBudget {
        limit: usize,
        used: AtomicUsize,
        exceeded: AtomicBool,
    }

    fn capture_bounded<R>(
        mut reader: R,
        budget: Arc<OutputBudget>,
    ) -> thread::JoinHandle<Result<Vec<u8>, String>>
    where
        R: Read + Send + 'static,
    {
        thread::spawn(move || {
            let mut captured = Vec::new();
            let mut buffer = [0u8; 8 * 1_024];
            loop {
                let count = reader
                    .read(&mut buffer)
                    .map_err(io_error("read bounded process output"))?;
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

    fn run_bounded_process(
        command: Command,
        maximum_output_bytes: usize,
        maximum_duration: Duration,
    ) -> Result<BoundedProcessOutput, String> {
        run_bounded_process_with_input(command, None, maximum_output_bytes, maximum_duration)
    }

    fn run_bounded_process_with_input(
        mut command: Command,
        standard_input: Option<&[u8]>,
        maximum_output_bytes: usize,
        maximum_duration: Duration,
    ) -> Result<BoundedProcessOutput, String> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        command.stdin(if standard_input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = command
            .spawn()
            .map_err(io_error("start bounded child process"))?;
        let mut in_job = 0;
        let process_in_job =
            unsafe { IsProcessInJob(child.as_raw_handle().cast(), null_mut(), &mut in_job) } != 0
                && in_job != 0;
        let budget = Arc::new(OutputBudget {
            limit: maximum_output_bytes,
            used: AtomicUsize::new(0),
            exceeded: AtomicBool::new(false),
        });
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "bounded child stdout was unavailable".to_owned())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "bounded child stderr was unavailable".to_owned())?;
        let stdout_reader = capture_bounded(stdout, Arc::clone(&budget));
        let stderr_reader = capture_bounded(stderr, Arc::clone(&budget));
        if let Some(standard_input) = standard_input {
            let Some(mut child_stdin) = child.stdin.take() else {
                let _ = child.kill();
                let _ = child.wait();
                return Err("bounded child stdin was unavailable".to_owned());
            };
            let write_result = child_stdin.write_all(standard_input);
            drop(child_stdin);
            if let Err(error) = write_result {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("write bounded child stdin: {error}"));
            }
        }
        let deadline = Instant::now()
            .checked_add(maximum_duration)
            .ok_or_else(|| "bounded child deadline overflowed".to_owned())?;
        let (status, timed_out) = loop {
            match child
                .try_wait()
                .map_err(io_error("poll bounded child process"))?
            {
                Some(status) => break (status, false),
                None if Instant::now() >= deadline => {
                    child
                        .kill()
                        .map_err(io_error("terminate timed-out child process"))?;
                    let status = child
                        .wait()
                        .map_err(io_error("reap timed-out child process"))?;
                    break (status, true);
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        };
        let stdout = stdout_reader
            .join()
            .map_err(|_| "bounded stdout reader panicked".to_owned())??;
        let stderr = stderr_reader
            .join()
            .map_err(|_| "bounded stderr reader panicked".to_owned())??;
        Ok(BoundedProcessOutput {
            status_success: status.success(),
            stdout,
            stderr,
            output_exceeded: budget.exceeded.load(Ordering::Relaxed),
            timed_out,
            process_in_job,
        })
    }

    fn child_run(args: &[std::ffi::OsString]) -> Result<(), String> {
        if args.len() != 10 {
            return Err("child arguments were invalid".to_owned());
        }
        let workspace = PathBuf::from(&args[2]);
        let outside = PathBuf::from(&args[3]);
        let report = PathBuf::from(&args[4]);
        let profile_folder = PathBuf::from(&args[5]);
        let git_program = PathBuf::from(&args[6]);
        let command_helper = PathBuf::from(&args[7]);
        let command_request_handle = args[8]
            .to_str()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value != 0)
            .ok_or_else(|| "helper request pipe handle was invalid".to_owned())?;
        let command_request =
            unsafe { fs::File::from_raw_handle(command_request_handle as *mut c_void) };
        let git_plan = decode_request_from_reader(command_request)
            .map_err(|error| format!("decode helper command request: {error:?}"))?;
        let helper_request_decoded = true;
        let network_port = args[9]
            .to_str()
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| "network fixture port was invalid".to_owned())?;
        let nested = workspace.join("nested-link");
        let created_link = workspace.join("created-link");

        let inside_read = fs::read_to_string(workspace.join("inside.txt")).is_ok();
        let inside_write = fs::write(workspace.join("inside-write.txt"), b"inside-write").is_ok();
        let workspace_canonicalize = workspace.canonicalize().is_ok();
        let profile_folder_canonicalize = profile_folder.canonicalize().is_ok();
        let outside_read_blocked = fs::read_to_string(outside.join("outside.txt")).is_err();
        let outside_write_blocked =
            fs::write(outside.join("outside-write.txt"), b"outside-write").is_err();
        let nested_read_blocked = fs::read_to_string(nested.join("outside.txt")).is_err();
        let nested_write_blocked =
            fs::write(nested.join("nested-write.txt"), b"nested-write").is_err();

        let command_file = workspace.join("child-process.txt");
        let child_process_write = command_shell()
            .args([
                OsStr::new("/d"),
                OsStr::new("/c"),
                OsStr::new("echo"),
                OsStr::new("child-process-ok"),
                OsStr::new(">"),
                command_file.as_os_str(),
            ])
            .status()
            .is_ok_and(|status| status.success())
            && fs::read_to_string(&command_file)
                .is_ok_and(|value| value.contains("child-process-ok"));
        let network_blocked = TcpStream::connect_timeout(
            &SocketAddrV4::new(Ipv4Addr::LOCALHOST, network_port).into(),
            Duration::from_secs(2),
        )
        .is_err();
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let system_drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());
        let helper_request = encode_request(&git_plan, WorkspaceCommandAccess::ReadOnly)
            .map_err(|error| format!("re-encode command helper request: {error:?}"))?;
        let mut helper_command = Command::new(&command_helper);
        helper_command
            .arg("--git-executable")
            .arg(&git_program)
            .current_dir(&workspace)
            .env_clear()
            .env("SystemDrive", &system_drive)
            .env("SystemRoot", &system_root);
        let git_baseline_output = run_bounded_process_with_input(
            helper_command,
            Some(&helper_request),
            MAXIMUM_COMMAND_OUTPUT_BYTES + 4 * 1_024,
            Duration::from_secs(10),
        );
        let command_helper_process = git_baseline_output
            .as_ref()
            .is_ok_and(|output| output.process_in_job);
        let runner_contract_applied = command_helper_process && git_baseline_output.is_ok();
        let git_baseline_status = runner_contract_applied
            && git_baseline_output.as_ref().is_ok_and(|output| {
                output.status_success && !output.output_exceeded && !output.timed_out
            });
        let evidence = match &git_baseline_output {
            Ok(output) => {
                let mut evidence = Vec::new();
                evidence.extend_from_slice(b"stdout:\n");
                evidence.extend_from_slice(&output.stdout[..output.stdout.len().min(16 * 1_024)]);
                evidence.extend_from_slice(b"\nstderr:\n");
                evidence.extend_from_slice(&output.stderr[..output.stderr.len().min(16 * 1_024)]);
                evidence
            }
            Err(error) => format!("command_helper_error:\n{error}\n").into_bytes(),
        };
        let _ = fs::write(workspace.join("git-baseline-output.txt"), evidence);

        let current_executable = std::env::current_exe()
            .map_err(io_error("resolve output and timeout fixture executable"))?;
        let mut output_limit_command = Command::new(&current_executable);
        output_limit_command
            .arg(OUTPUT_LIMIT_CHILD_FLAG)
            .current_dir(&workspace)
            .env_clear()
            .env("SystemDrive", &system_drive)
            .env("SystemRoot", &system_root);
        let output_limit_result =
            run_bounded_process(output_limit_command, 4 * 1_024, Duration::from_secs(5))?;
        let output_limit_enforced = output_limit_result.status_success
            && output_limit_result.output_exceeded
            && !output_limit_result.timed_out
            && output_limit_result.process_in_job
            && output_limit_result
                .stdout
                .len()
                .saturating_add(output_limit_result.stderr.len())
                <= 4 * 1_024;

        let mut timeout_command = Command::new(current_executable);
        timeout_command
            .arg(TIMEOUT_CHILD_FLAG)
            .current_dir(&workspace)
            .env_clear()
            .env("SystemDrive", system_drive)
            .env("SystemRoot", system_root);
        let timeout_result =
            run_bounded_process(timeout_command, 4 * 1_024, Duration::from_millis(100))?;
        let timeout_enforced = timeout_result.timed_out && timeout_result.process_in_job;

        let created_link_succeeded = command_shell()
            .args([
                OsStr::new("/d"),
                OsStr::new("/c"),
                OsStr::new("mklink"),
                OsStr::new("/J"),
                created_link.as_os_str(),
                outside.as_os_str(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let created_link_escape_blocked = !created_link_succeeded
            || fs::read_to_string(created_link.join("outside.txt")).is_err();
        let git_process = tool_version("git.exe", &["--version"]);
        let node_process = tool_version("node.exe", &["--version"]);
        let npm_process = tool_version("npm.cmd", &["--version"]);
        let cargo_process = tool_version("cargo.exe", &["--version"]);
        let codex_home = profile_folder.join("codex-home");
        let codex_temp = profile_folder.join("codex-temp");
        let codex_program = std::env::current_exe().ok().and_then(|path| {
            path.parent()
                .map(|parent| parent.join("codex-under-test.exe"))
        });
        let codex_directories_ready =
            fs::create_dir_all(&codex_home).is_ok() && fs::create_dir_all(&codex_temp).is_ok();
        let codex_process = codex_directories_ready
            && codex_program.as_ref().is_some_and(|program| {
                Command::new(program)
                    .arg("--version")
                    .env("CODEX_HOME", &codex_home)
                    .env("TEMP", &codex_temp)
                    .env("TMP", &codex_temp)
                    .env("NO_COLOR", "1")
                    .output()
                    .is_ok_and(|output| {
                        if !output.status.success() {
                            return false;
                        }
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        fs::write(workspace.join("codex-version.txt"), stdout.as_bytes()).is_ok()
                            && stdout.trim_start().starts_with("codex-cli ")
                    })
            });
        let codex_app_server_initialize = codex_directories_ready
            && codex_program.as_ref().is_some_and(|program| {
                app_server_initialize(program, &workspace, &codex_home, &codex_temp)
            });

        let report_body = [
            ("inside_read", inside_read),
            ("inside_write", inside_write),
            ("workspace_canonicalize", workspace_canonicalize),
            ("profile_folder_canonicalize", profile_folder_canonicalize),
            ("outside_read_blocked", outside_read_blocked),
            ("outside_write_blocked", outside_write_blocked),
            ("nested_read_blocked", nested_read_blocked),
            ("nested_write_blocked", nested_write_blocked),
            ("child_process_write", child_process_write),
            ("network_blocked", network_blocked),
            ("runner_contract_applied", runner_contract_applied),
            ("helper_request_decoded", helper_request_decoded),
            ("command_helper_process", command_helper_process),
            ("output_limit_enforced", output_limit_enforced),
            ("timeout_enforced", timeout_enforced),
            ("git_baseline_status", git_baseline_status),
            ("created_link_succeeded", created_link_succeeded),
            ("created_link_escape_blocked", created_link_escape_blocked),
            ("git_process", git_process),
            ("node_process", node_process),
            ("npm_process", npm_process),
            ("cargo_process", cargo_process),
            ("codex_process", codex_process),
            ("codex_app_server_initialize", codex_app_server_initialize),
        ]
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
        fs::write(report, format!("{report_body}\n")).map_err(io_error("write child report"))
    }

    fn command_shell() -> Command {
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        Command::new(PathBuf::from(system_root).join("System32").join("cmd.exe"))
    }

    fn tool_version(program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn app_server_initialize(
        program: &Path,
        workspace: &Path,
        codex_home: &Path,
        codex_temp: &Path,
    ) -> bool {
        let stderr = match fs::File::create(workspace.join("codex-app-server-stderr.txt")) {
            Ok(file) => file,
            Err(_) => return false,
        };
        let mut child = match Command::new(program)
            .args(["app-server", "--listen", "stdio://"])
            .current_dir(workspace)
            .env("CODEX_HOME", codex_home)
            .env("TEMP", codex_temp)
            .env("TMP", codex_temp)
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return false,
        };
        let result = (|| {
            let stdin = child.stdin.as_mut()?;
            let request = serde_json::json!({
                "method": "initialize",
                "id": 1,
                "params": {
                    "clientInfo": {
                        "name": "mio_appcontainer_probe",
                        "title": "M.I.O. AppContainer Probe",
                        "version": "0.0.0"
                    },
                    "capabilities": null
                }
            });
            writeln!(stdin, "{request}").ok()?;
            stdin.flush().ok()?;

            let stdout = child.stdout.take()?;
            let (sender, receiver) = mpsc::channel();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
                        continue;
                    };
                    if message.get("id").and_then(serde_json::Value::as_i64) == Some(1) {
                        let _ = sender.send(message.get("result").is_some());
                        return;
                    }
                }
                let _ = sender.send(false);
            });
            receiver.recv_timeout(Duration::from_secs(15)).ok()
        })()
        .unwrap_or(false);
        let _ = child.kill();
        let _ = child.wait();
        if result {
            let _ = fs::write(
                workspace.join("codex-app-server-initialize.txt"),
                b"initialize-response=true\n",
            );
        }
        result
    }

    fn parse_report(path: &Path) -> Result<BTreeMap<String, bool>, String> {
        let body = fs::read_to_string(path).map_err(io_error("read child report"))?;
        body.lines()
            .map(|line| {
                let (key, value) = line
                    .split_once('=')
                    .ok_or_else(|| format!("invalid report line: {line}"))?;
                let value = value
                    .parse::<bool>()
                    .map_err(|_| format!("invalid report value for {key}"))?;
                Ok((key.to_owned(), value))
            })
            .collect()
    }

    fn create_junction(link: &Path, target: &Path) -> Result<(), String> {
        let output = command_shell()
            .args([
                OsStr::new("/d"),
                OsStr::new("/c"),
                OsStr::new("mklink"),
                OsStr::new("/J"),
                link.as_os_str(),
                target.as_os_str(),
            ])
            .output()
            .map_err(io_error("start junction setup"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "junction setup failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn resolve_program(name: &str) -> Result<PathBuf, String> {
        let output = Command::new("where.exe")
            .arg(name)
            .output()
            .map_err(io_error("resolve tool executable"))?;
        if !output.status.success() {
            return Err(format!("{name} was not found"));
        }
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .map(PathBuf::from)
            .ok_or_else(|| format!("{name} resolution returned no path"))?;
        path.canonicalize()
            .map_err(io_error("canonicalize tool executable"))
    }

    fn grant_directory(path: &Path, sid: &str, permission: &str) -> Result<(), String> {
        let status = Command::new("icacls.exe")
            .arg(path)
            .arg("/grant")
            .arg(format!("*{sid}:{permission}"))
            .arg("/Q")
            .status()
            .map_err(io_error("start icacls"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("icacls failed for {}", path.display()))
        }
    }

    fn remove_grant(path: &Path, sid: &str) -> Result<(), String> {
        let status = Command::new("icacls.exe")
            .arg(path)
            .arg("/remove:g")
            .arg(format!("*{sid}"))
            .arg("/Q")
            .status()
            .map_err(io_error("start icacls cleanup"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("icacls cleanup failed for {}", path.display()))
        }
    }

    fn unique_id() -> Result<String, String> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "system clock is before Unix epoch".to_owned())?
            .as_nanos();
        Ok(format!("{}-{nanos}", std::process::id()))
    }

    fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
        value.as_ref().encode_wide().chain(Some(0)).collect()
    }

    fn io_error(context: &'static str) -> impl FnOnce(std::io::Error) -> String {
        move |error| format!("{context}: {error}")
    }

    struct AppContainerProfile {
        name: Vec<u16>,
        sid: PSID,
        active: bool,
    }

    struct TemporaryAclGrants {
        sid: String,
        paths: Vec<PathBuf>,
    }

    struct WorkspaceDrive {
        name: String,
        path: PathBuf,
        active: bool,
    }

    struct JobHandle(windows_sys::Win32::Foundation::HANDLE);

    struct OwnedWinHandle(HANDLE);

    struct RequestPipe {
        read: OwnedWinHandle,
        write: OwnedWinHandle,
    }

    impl OwnedWinHandle {
        fn raw(&self) -> HANDLE {
            self.0
        }

        fn into_file(mut self) -> fs::File {
            let handle = self.0;
            self.0 = null_mut();
            unsafe { fs::File::from_raw_handle(handle.cast()) }
        }
    }

    impl Drop for OwnedWinHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    impl RequestPipe {
        fn create() -> Result<Self, String> {
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
                    (MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1) as u32,
                )
            } == 0
            {
                return Err(format!(
                    "CreatePipe failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let pipe = Self {
                read: OwnedWinHandle(read),
                write: OwnedWinHandle(write),
            };
            if unsafe { SetHandleInformation(pipe.write.raw(), HANDLE_FLAG_INHERIT, 0) } == 0 {
                return Err(format!(
                    "SetHandleInformation failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(pipe)
        }
    }

    impl JobHandle {
        fn create_kill_on_close() -> Result<Self, String> {
            let handle = unsafe { CreateJobObjectW(null(), null()) };
            if handle.is_null() {
                return Err(format!(
                    "CreateJobObjectW failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let job = Self(handle);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            } == 0
            {
                return Err(format!(
                    "SetInformationJobObject failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(job)
        }

        fn assign(&self, process: windows_sys::Win32::Foundation::HANDLE) -> Result<(), String> {
            if unsafe { AssignProcessToJobObject(self.0, process) } == 0 {
                Err(format!(
                    "AssignProcessToJobObject failed: {}",
                    std::io::Error::last_os_error()
                ))
            } else {
                Ok(())
            }
        }

        fn terminate(&self, exit_code: u32) {
            unsafe {
                TerminateJobObject(self.0, exit_code);
            }
        }
    }

    impl Drop for JobHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    impl WorkspaceDrive {
        fn create(target: &Path) -> Result<Self, String> {
            let drive_mask = unsafe { GetLogicalDrives() };
            let letter = (b'P'..=b'Z')
                .rev()
                .find(|letter| drive_mask & (1 << (letter - b'A')) == 0)
                .ok_or_else(|| "no unused workspace drive letter was available".to_owned())?;
            let name = format!("{}:", char::from(letter));
            let status = Command::new("subst.exe")
                .arg(&name)
                .arg(target)
                .status()
                .map_err(io_error("start temporary workspace drive mapping"))?;
            if !status.success() {
                return Err("temporary workspace drive mapping failed".to_owned());
            }
            Ok(Self {
                path: PathBuf::from(format!("{name}\\.")),
                name,
                active: true,
            })
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn remove(&mut self) -> Result<(), String> {
            let status = Command::new("subst.exe")
                .arg(&self.name)
                .arg("/D")
                .status()
                .map_err(io_error("start temporary workspace drive cleanup"))?;
            if status.success() {
                self.active = false;
                Ok(())
            } else {
                Err("temporary workspace drive cleanup failed".to_owned())
            }
        }
    }

    impl Drop for WorkspaceDrive {
        fn drop(&mut self) {
            if self.active {
                let _ = self.remove();
            }
        }
    }

    impl TemporaryAclGrants {
        fn new(sid: &str) -> Self {
            Self {
                sid: sid.to_owned(),
                paths: Vec::new(),
            }
        }

        fn grant(&mut self, path: &Path, permission: &str) -> Result<(), String> {
            grant_directory(path, &self.sid, permission)?;
            self.paths.push(path.to_owned());
            Ok(())
        }

        fn remove_all(&mut self) -> Result<(), String> {
            let mut first_error = None;
            while let Some(path) = self.paths.pop() {
                if let Err(error) = remove_grant(&path, &self.sid) {
                    first_error.get_or_insert(error);
                }
            }
            first_error.map_or(Ok(()), Err)
        }
    }

    impl Drop for TemporaryAclGrants {
        fn drop(&mut self) {
            let _ = self.remove_all();
        }
    }

    impl AppContainerProfile {
        fn create(name: &str) -> Result<Self, String> {
            let name = wide(name);
            let display_name = wide("M.I.O. workspace boundary spike");
            let description = wide("Temporary profile for an isolated M.I.O. workspace probe");
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
                return Err(format!(
                    "CreateAppContainerProfile failed with HRESULT 0x{:08X}",
                    result as u32
                ));
            }
            Ok(Self {
                name,
                sid,
                active: true,
            })
        }

        fn sid_string(&self) -> Result<String, String> {
            let mut value: PWSTR = null_mut();
            if unsafe { ConvertSidToStringSidW(self.sid, &mut value) } == 0 || value.is_null() {
                return Err(format!(
                    "ConvertSidToStringSidW failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut length = 0;
            while unsafe { *value.add(length) } != 0 {
                length += 1;
            }
            let result = String::from_utf16(unsafe { std::slice::from_raw_parts(value, length) })
                .map_err(|_| "AppContainer SID was not valid UTF-16".to_owned());
            unsafe {
                LocalFree(value.cast());
            }
            result
        }

        fn folder_path(&self, sid: &str) -> Result<PathBuf, String> {
            let sid = wide(sid);
            let mut value = null_mut();
            let result = unsafe { GetAppContainerFolderPath(sid.as_ptr(), &mut value) };
            if result < 0 || value.is_null() {
                return Err(format!(
                    "GetAppContainerFolderPath failed with HRESULT 0x{:08X}",
                    result as u32
                ));
            }
            let mut length = 0usize;
            unsafe {
                while *value.add(length) != 0 {
                    length += 1;
                }
            }
            let path = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
                std::slice::from_raw_parts(value, length)
            }));
            unsafe {
                CoTaskMemFree(value.cast());
            }
            Ok(path)
        }

        fn launch_child(
            &self,
            executable: &Path,
            workspace: &Path,
            outside: &Path,
            report: &Path,
            profile_folder: &Path,
            git_program: &Path,
            command_helper: &Path,
            command_request: &[u8],
            network_port: u16,
        ) -> Result<u32, String> {
            let RequestPipe {
                read: request_read,
                write: request_write,
            } = RequestPipe::create()?;
            let mut attribute_bytes = 0usize;
            unsafe {
                InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut attribute_bytes);
            }
            if attribute_bytes == 0 {
                return Err("attribute-list size was zero".to_owned());
            }
            let words = attribute_bytes.div_ceil(size_of::<usize>());
            let mut attribute_storage = vec![0usize; words];
            let attribute_list = attribute_storage.as_mut_ptr().cast::<c_void>();
            if unsafe {
                InitializeProcThreadAttributeList(attribute_list, 2, 0, &mut attribute_bytes)
            } == 0
            {
                return Err(format!(
                    "InitializeProcThreadAttributeList failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let attributes = ProcThreadAttributes(attribute_list);
            let capabilities = SECURITY_CAPABILITIES {
                AppContainerSid: self.sid,
                Capabilities: null_mut(),
                CapabilityCount: 0,
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
                return Err(format!(
                    "UpdateProcThreadAttribute failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let inherited_handles = [request_read.raw()];
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
                return Err(format!(
                    "UpdateProcThreadAttribute handle list failed: {}",
                    std::io::Error::last_os_error()
                ));
            }

            let application = wide(executable.as_os_str());
            let command_line = format!(
                "\"{}\" {CHILD_FLAG} \"{}\" \"{}\" \"{}\" \"{}\" \"{}\" \"{}\" \"{}\" {network_port}",
                executable.display(),
                workspace.display(),
                outside.display(),
                report.display(),
                profile_folder.display(),
                git_program.display(),
                command_helper.display(),
                request_read.raw() as usize
            );
            let mut command_line = wide(command_line);
            let current_directory = wide(workspace.as_os_str());
            let mut startup = STARTUPINFOEXW::default();
            startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            startup.lpAttributeList = attributes.0;
            let mut process = PROCESS_INFORMATION::default();
            let job = JobHandle::create_kill_on_close()?;
            if unsafe {
                CreateProcessW(
                    application.as_ptr(),
                    command_line.as_mut_ptr(),
                    null(),
                    null(),
                    1,
                    EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
                    null(),
                    current_directory.as_ptr(),
                    &startup.StartupInfo,
                    &mut process,
                )
            } == 0
            {
                return Err(format!(
                    "CreateProcessW failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            drop(request_read);
            let process = ProcessHandles(process);
            if let Err(error) = job.assign(process.0.hProcess) {
                unsafe {
                    TerminateProcess(process.0.hProcess, 125);
                }
                return Err(error);
            }
            if unsafe { ResumeThread(process.0.hThread) } == u32::MAX {
                job.terminate(125);
                return Err(format!(
                    "ResumeThread failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut request_writer = request_write.into_file();
            if let Err(error) = request_writer.write_all(command_request) {
                job.terminate(125);
                return Err(format!("write helper request pipe: {error}"));
            }
            drop(request_writer);
            let wait = unsafe { WaitForSingleObject(process.0.hProcess, CHILD_TIMEOUT_MILLIS) };
            if wait != WAIT_OBJECT_0 {
                job.terminate(124);
                return Err(format!("sandboxed child wait failed with code {wait}"));
            }
            let mut exit_code = 0;
            if unsafe { GetExitCodeProcess(process.0.hProcess, &mut exit_code) } == 0 {
                return Err(format!(
                    "GetExitCodeProcess failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            drop(attributes);
            Ok(exit_code)
        }

        fn delete(&mut self) -> Result<(), String> {
            let result = unsafe { DeleteAppContainerProfile(self.name.as_ptr()) };
            unsafe {
                FreeSid(self.sid);
            }
            self.sid = null_mut();
            self.active = false;
            if result < 0 {
                Err(format!(
                    "DeleteAppContainerProfile failed with HRESULT 0x{:08X}",
                    result as u32
                ))
            } else {
                Ok(())
            }
        }
    }

    impl Drop for AppContainerProfile {
        fn drop(&mut self) {
            if self.active {
                unsafe {
                    DeleteAppContainerProfile(self.name.as_ptr());
                    FreeSid(self.sid);
                }
            }
        }
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
}
