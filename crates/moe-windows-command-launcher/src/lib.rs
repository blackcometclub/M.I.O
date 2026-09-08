#![deny(unsafe_op_in_unsafe_fn)]

//! Host-side preparation contract for the Windows isolated command launcher.
//!
//! This crate accepts only already-verified product paths and a bounded helper
//! request, then fixes the OS isolation requirements that the future Windows
//! backend must satisfy. It does not resolve tools, change ACLs, create an
//! AppContainer, map a drive, or start a process yet.

use moe_command_broker::{MAXIMUM_COMMAND_OUTPUT_BYTES, MAXIMUM_COMMAND_SECONDS, RegisteredTool};
use moe_command_helper_protocol::{
    MAXIMUM_COMMAND_HELPER_REQUEST_BYTES, decode_npm_package_install_request, decode_request,
};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
mod windows;

const COMMAND_HELPER_FILE_NAME: &str = "mio-command-helper.exe";
const GIT_EXECUTABLE_FILE_NAME: &str = "git.exe";
const NODE_EXECUTABLE_FILE_NAME: &str = "node.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsHostIsolationRequirement {
    UniqueAppContainerProfile,
    NoNetworkCapabilities,
    InternetClientOnlyCapability,
    WorkspaceScopedAcl,
    TemporaryWorkspaceDrive,
    KillOnCloseJob,
    BoundedAnonymousRequestPipe,
    BoundedOutputCapture,
    CleanupEveryTemporaryGrant,
}

const WINDOWS_HOST_ISOLATION_REQUIREMENTS: [WindowsHostIsolationRequirement; 8] = [
    WindowsHostIsolationRequirement::UniqueAppContainerProfile,
    WindowsHostIsolationRequirement::NoNetworkCapabilities,
    WindowsHostIsolationRequirement::WorkspaceScopedAcl,
    WindowsHostIsolationRequirement::TemporaryWorkspaceDrive,
    WindowsHostIsolationRequirement::KillOnCloseJob,
    WindowsHostIsolationRequirement::BoundedAnonymousRequestPipe,
    WindowsHostIsolationRequirement::BoundedOutputCapture,
    WindowsHostIsolationRequirement::CleanupEveryTemporaryGrant,
];

const WINDOWS_PACKAGE_INSTALL_ISOLATION_REQUIREMENTS: [WindowsHostIsolationRequirement; 8] = [
    WindowsHostIsolationRequirement::UniqueAppContainerProfile,
    WindowsHostIsolationRequirement::InternetClientOnlyCapability,
    WindowsHostIsolationRequirement::WorkspaceScopedAcl,
    WindowsHostIsolationRequirement::TemporaryWorkspaceDrive,
    WindowsHostIsolationRequirement::KillOnCloseJob,
    WindowsHostIsolationRequirement::BoundedAnonymousRequestPipe,
    WindowsHostIsolationRequirement::BoundedOutputCapture,
    WindowsHostIsolationRequirement::CleanupEveryTemporaryGrant,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsCommandLaunchPreparationError {
    WorkspaceUnavailable,
    UnsafeWorkspace,
    UnsafeCommandHelper,
    UnsafeGitExecutable,
    UnsafeToolExecutable,
    UnsafeNpmRuntime,
    InvalidRequest,
    UnsupportedTool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsCommandIsolationError {
    UnsupportedPlatform,
    ProfileUnavailable,
    NetworkCapabilityUnavailable,
    SystemToolUnavailable,
    WorkspaceGrantFailed,
    DriveUnavailable,
    DriveMappingFailed,
    JobUnavailable,
    RequestPipeUnavailable,
    OutputPipeUnavailable,
    ProcessLaunchFailed,
    JobAssignmentFailed,
    RequestWriteFailed,
    ProcessResumeFailed,
    ProcessWaitFailed,
    ProcessExitUnavailable,
    OutputReadFailed,
    OutputReaderPanicked,
    ToolStagingFailed,
    CleanupFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsCommandLaunchCompletion {
    Completed,
    OutputLimitExceeded,
    HostTimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsCommandLaunchOutcome {
    completion: WindowsCommandLaunchCompletion,
    exit_code: u32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl WindowsCommandLaunchOutcome {
    pub fn completion(&self) -> WindowsCommandLaunchCompletion {
        self.completion
    }

    pub fn success(&self) -> bool {
        self.completion == WindowsCommandLaunchCompletion::Completed && self.exit_code == 0
    }

    pub fn exit_code(&self) -> u32 {
        self.exit_code
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWindowsCommandLaunch {
    workspace_root: PathBuf,
    command_helper: PathBuf,
    tool_executable: PathBuf,
    tool: RegisteredTool,
    workspace_writable: bool,
    npm_runtime: Option<PathBuf>,
    network_client_enabled: bool,
    request: Vec<u8>,
}

pub struct WindowsCommandIsolation {
    #[cfg(windows)]
    resources: windows::IsolationResources,
}

impl PreparedWindowsCommandLaunch {
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn command_helper(&self) -> &Path {
        &self.command_helper
    }

    pub fn tool_executable(&self) -> &Path {
        &self.tool_executable
    }

    pub fn tool(&self) -> RegisteredTool {
        self.tool
    }

    pub fn workspace_writable(&self) -> bool {
        self.workspace_writable
    }

    pub fn npm_runtime(&self) -> Option<&Path> {
        self.npm_runtime.as_deref()
    }

    pub fn network_client_enabled(&self) -> bool {
        self.network_client_enabled
    }

    pub fn request(&self) -> &[u8] {
        &self.request
    }

    pub fn maximum_output_bytes(&self) -> usize {
        MAXIMUM_COMMAND_OUTPUT_BYTES
    }

    pub fn maximum_seconds(&self) -> u64 {
        MAXIMUM_COMMAND_SECONDS
    }

    pub fn isolation_requirements(&self) -> &'static [WindowsHostIsolationRequirement] {
        if self.network_client_enabled {
            &WINDOWS_PACKAGE_INSTALL_ISOLATION_REQUIREMENTS
        } else {
            &WINDOWS_HOST_ISOLATION_REQUIREMENTS
        }
    }
}

impl WindowsCommandIsolation {
    pub fn workspace_drive_root(&self) -> &Path {
        #[cfg(windows)]
        {
            self.resources.workspace_drive_root()
        }
        #[cfg(not(windows))]
        {
            unreachable!("Windows command isolation cannot exist on this platform")
        }
    }

    pub fn close(mut self) -> Result<(), WindowsCommandIsolationError> {
        #[cfg(windows)]
        {
            self.resources.close()
        }
        #[cfg(not(windows))]
        {
            Err(WindowsCommandIsolationError::UnsupportedPlatform)
        }
    }

    #[cfg(all(test, windows))]
    fn sid_for_test(&self) -> &str {
        self.resources.sid()
    }
}

pub fn establish_windows_command_isolation(
    prepared: &PreparedWindowsCommandLaunch,
) -> Result<WindowsCommandIsolation, WindowsCommandIsolationError> {
    #[cfg(windows)]
    {
        Ok(WindowsCommandIsolation {
            resources: windows::IsolationResources::establish(prepared)?,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = prepared;
        Err(WindowsCommandIsolationError::UnsupportedPlatform)
    }
}

pub fn execute_windows_command_launch(
    prepared: &PreparedWindowsCommandLaunch,
) -> Result<WindowsCommandLaunchOutcome, WindowsCommandIsolationError> {
    #[cfg(windows)]
    {
        let mut resources = windows::IsolationResources::establish(prepared)?;
        let outcome = resources.execute(prepared);
        let cleanup = resources.close();
        cleanup?;
        outcome
    }
    #[cfg(not(windows))]
    {
        let _ = prepared;
        Err(WindowsCommandIsolationError::UnsupportedPlatform)
    }
}

#[cfg(all(test, windows))]
fn execute_windows_command_launch_with_root_only_acl_for_test(
    prepared: &PreparedWindowsCommandLaunch,
) -> Result<(WindowsCommandLaunchOutcome, String), WindowsCommandIsolationError> {
    let mut resources = windows::IsolationResources::establish_root_only_for_test(prepared)?;
    let sid = resources.sid().to_owned();
    let outcome = resources.execute(prepared);
    let cleanup = resources.close();
    cleanup?;
    outcome.map(|outcome| (outcome, sid))
}

pub fn prepare_windows_command_launch(
    workspace_root: &Path,
    command_helper: &Path,
    tool_executable: &Path,
    request: &[u8],
) -> Result<PreparedWindowsCommandLaunch, WindowsCommandLaunchPreparationError> {
    let workspace_root = canonical_directory(workspace_root)?;
    let command_helper = canonical_executable(
        command_helper,
        COMMAND_HELPER_FILE_NAME,
        WindowsCommandLaunchPreparationError::UnsafeCommandHelper,
    )?;
    if request.is_empty() || request.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(WindowsCommandLaunchPreparationError::InvalidRequest);
    }
    let plan = decode_request(request)
        .map_err(|_| WindowsCommandLaunchPreparationError::InvalidRequest)?;
    let (expected_name, executable_error) = match plan.tool() {
        RegisteredTool::Git => (
            GIT_EXECUTABLE_FILE_NAME,
            WindowsCommandLaunchPreparationError::UnsafeGitExecutable,
        ),
        RegisteredTool::Node => (
            NODE_EXECUTABLE_FILE_NAME,
            WindowsCommandLaunchPreparationError::UnsafeToolExecutable,
        ),
        RegisteredTool::Npm | RegisteredTool::Cargo => {
            return Err(WindowsCommandLaunchPreparationError::UnsupportedTool);
        }
    };
    let workspace_writable = plan.command().writes_workspace();
    let tool_executable = canonical_executable(tool_executable, expected_name, executable_error)?;
    Ok(PreparedWindowsCommandLaunch {
        workspace_root,
        command_helper,
        tool_executable,
        tool: plan.tool(),
        workspace_writable,
        npm_runtime: None,
        network_client_enabled: false,
        request: request.to_vec(),
    })
}

pub fn prepare_windows_npm_package_install_launch(
    workspace_root: &Path,
    command_helper: &Path,
    node_executable: &Path,
    npm_runtime: &Path,
    request: &[u8],
) -> Result<PreparedWindowsCommandLaunch, WindowsCommandLaunchPreparationError> {
    let workspace_root = canonical_directory(workspace_root)?;
    let command_helper = canonical_executable(
        command_helper,
        COMMAND_HELPER_FILE_NAME,
        WindowsCommandLaunchPreparationError::UnsafeCommandHelper,
    )?;
    if request.is_empty() || request.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(WindowsCommandLaunchPreparationError::InvalidRequest);
    }
    let _plan = decode_npm_package_install_request(request)
        .map_err(|_| WindowsCommandLaunchPreparationError::InvalidRequest)?;
    let node_executable = canonical_executable(
        node_executable,
        NODE_EXECUTABLE_FILE_NAME,
        WindowsCommandLaunchPreparationError::UnsafeToolExecutable,
    )?;
    let npm_runtime = canonical_npm_runtime(&node_executable, npm_runtime)?;
    Ok(PreparedWindowsCommandLaunch {
        workspace_root,
        command_helper,
        tool_executable: node_executable,
        tool: RegisteredTool::Node,
        workspace_writable: true,
        npm_runtime: Some(npm_runtime),
        network_client_enabled: true,
        request: request.to_vec(),
    })
}

pub fn prepare_windows_npm_script_launch(
    workspace_root: &Path,
    command_helper: &Path,
    node_executable: &Path,
    npm_runtime: &Path,
    request: &[u8],
) -> Result<PreparedWindowsCommandLaunch, WindowsCommandLaunchPreparationError> {
    let workspace_root = canonical_directory(workspace_root)?;
    let command_helper = canonical_executable(
        command_helper,
        COMMAND_HELPER_FILE_NAME,
        WindowsCommandLaunchPreparationError::UnsafeCommandHelper,
    )?;
    if request.is_empty() || request.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(WindowsCommandLaunchPreparationError::InvalidRequest);
    }
    let plan = decode_request(request)
        .map_err(|_| WindowsCommandLaunchPreparationError::InvalidRequest)?;
    if plan.tool() != RegisteredTool::Npm {
        return Err(WindowsCommandLaunchPreparationError::UnsupportedTool);
    }
    let node_executable = canonical_executable(
        node_executable,
        NODE_EXECUTABLE_FILE_NAME,
        WindowsCommandLaunchPreparationError::UnsafeToolExecutable,
    )?;
    let npm_runtime = canonical_npm_runtime(&node_executable, npm_runtime)?;
    Ok(PreparedWindowsCommandLaunch {
        workspace_root,
        command_helper,
        tool_executable: node_executable,
        tool: RegisteredTool::Npm,
        workspace_writable: plan.command().writes_workspace(),
        npm_runtime: Some(npm_runtime),
        network_client_enabled: false,
        request: request.to_vec(),
    })
}

fn canonical_npm_runtime(
    node_executable: &Path,
    npm_runtime: &Path,
) -> Result<PathBuf, WindowsCommandLaunchPreparationError> {
    let canonical = canonical_directory(npm_runtime)
        .map_err(|_| WindowsCommandLaunchPreparationError::UnsafeNpmRuntime)?;
    let expected = node_executable
        .parent()
        .map(|parent| parent.join("node_modules").join("npm"))
        .ok_or(WindowsCommandLaunchPreparationError::UnsafeNpmRuntime)?
        .canonicalize()
        .map_err(|_| WindowsCommandLaunchPreparationError::UnsafeNpmRuntime)?;
    if canonical != expected
        || !canonical.join("bin").join("npm-cli.js").is_file()
        || !canonical.join("package.json").is_file()
    {
        return Err(WindowsCommandLaunchPreparationError::UnsafeNpmRuntime);
    }
    Ok(canonical)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, WindowsCommandLaunchPreparationError> {
    if !path.is_absolute() || is_filesystem_link(path)? {
        return Err(WindowsCommandLaunchPreparationError::UnsafeWorkspace);
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| WindowsCommandLaunchPreparationError::WorkspaceUnavailable)?;
    if !canonical.is_dir() || is_filesystem_link(&canonical)? {
        return Err(WindowsCommandLaunchPreparationError::UnsafeWorkspace);
    }
    Ok(canonical)
}

fn canonical_executable(
    path: &Path,
    expected_name: &'static str,
    error: WindowsCommandLaunchPreparationError,
) -> Result<PathBuf, WindowsCommandLaunchPreparationError> {
    if !path.is_absolute() || is_filesystem_link(path).map_err(|_| error)? {
        return Err(error);
    }
    let canonical = path.canonicalize().map_err(|_| error)?;
    let metadata = fs::metadata(&canonical).map_err(|_| error)?;
    let matches_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected_name));
    if !metadata.is_file() || metadata.len() == 0 || !matches_name {
        return Err(error);
    }
    Ok(canonical)
}

#[cfg(windows)]
fn is_filesystem_link(path: &Path) -> Result<bool, WindowsCommandLaunchPreparationError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| WindowsCommandLaunchPreparationError::WorkspaceUnavailable)?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_filesystem_link(path: &Path) -> Result<bool, WindowsCommandLaunchPreparationError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| WindowsCommandLaunchPreparationError::WorkspaceUnavailable)?;
    Ok(metadata.file_type().is_symlink())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_command_broker::{
        BaselineCommand, CommandClassification, NpmPackageInstallClassification,
        NpmPackageInstallRequest, WorkspaceCommandAccess, WorkspaceCommandIntent,
        WorkspaceCommandRequest, classify_command, classify_npm_package_install,
    };
    use moe_command_helper_protocol::{encode_npm_package_install_request, encode_request};
    use std::ffi::OsStr;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        cleanup_parent: PathBuf,
        workspace: PathBuf,
        helper: PathBuf,
        git: PathBuf,
        node: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            Self::new_in(&std::env::temp_dir(), label)
        }

        fn new_in(parent: &Path, label: &str) -> Self {
            let root = parent.join(format!(
                "moe-windows-command-launcher-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let workspace = root.join("workspace");
            let product = root.join("product");
            let toolchain = root.join("toolchain");
            fs::create_dir_all(&workspace).unwrap();
            fs::create_dir_all(&product).unwrap();
            fs::create_dir_all(&toolchain).unwrap();
            let helper = product.join(COMMAND_HELPER_FILE_NAME);
            let git = toolchain.join(GIT_EXECUTABLE_FILE_NAME);
            let node = toolchain.join(NODE_EXECUTABLE_FILE_NAME);
            fs::write(&helper, b"helper").unwrap();
            fs::write(&git, b"git").unwrap();
            fs::write(&node, b"node").unwrap();
            Self {
                root,
                cleanup_parent: parent.to_owned(),
                workspace,
                helper,
                git,
                node,
            }
        }

        fn request(&self, command: BaselineCommand) -> Vec<u8> {
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
            encode_request(
                &plan,
                if command.writes_workspace() {
                    WorkspaceCommandAccess::ReadWrite
                } else {
                    WorkspaceCommandAccess::ReadOnly
                },
            )
            .unwrap()
        }

        fn npm_runtime(&self) -> PathBuf {
            self.node.parent().unwrap().join("node_modules").join("npm")
        }

        fn package_install_request(&self) -> Vec<u8> {
            let classification = classify_npm_package_install(
                NpmPackageInstallRequest::new(
                    PathBuf::from("."),
                    "is-number".to_owned(),
                    "7.0.0".to_owned(),
                ),
                WorkspaceCommandAccess::ReadWrite,
            );
            let NpmPackageInstallClassification::ActionTimeConfirmationRequired(plan) =
                classification
            else {
                panic!("expected package-install plan");
            };
            encode_npm_package_install_request(&plan, WorkspaceCommandAccess::ReadWrite).unwrap()
        }

        fn create_npm_runtime(&self) -> PathBuf {
            let npm = self.npm_runtime();
            fs::create_dir_all(npm.join("bin")).unwrap();
            fs::write(npm.join("bin").join("npm-cli.js"), b"npm cli").unwrap();
            fs::write(npm.join("package.json"), b"{}").unwrap();
            npm
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let is_direct_fixture = self.root.parent() == Some(self.cleanup_parent.as_path())
                && self
                    .root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("moe-windows-command-launcher-"));
            if is_direct_fixture {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
    }

    #[test]
    fn fixes_every_required_boundary_for_a_git_request() {
        let fixture = Fixture::new("accept");
        let request = fixture.request(BaselineCommand::GitStatus);
        let prepared = prepare_windows_command_launch(
            &fixture.workspace,
            &fixture.helper,
            &fixture.git,
            &request,
        )
        .unwrap();
        assert!(prepared.workspace_root().is_absolute());
        assert_eq!(
            prepared.command_helper().file_name().unwrap(),
            COMMAND_HELPER_FILE_NAME
        );
        assert_eq!(
            prepared.tool_executable().file_name().unwrap(),
            GIT_EXECUTABLE_FILE_NAME
        );
        assert!(!prepared.workspace_writable());
        assert_eq!(prepared.tool(), RegisteredTool::Git);
        assert_eq!(prepared.request(), request);
        assert_eq!(
            prepared.maximum_output_bytes(),
            MAXIMUM_COMMAND_OUTPUT_BYTES
        );
        assert_eq!(prepared.maximum_seconds(), MAXIMUM_COMMAND_SECONDS);
        assert_eq!(
            prepared.isolation_requirements(),
            &WINDOWS_HOST_ISOLATION_REQUIREMENTS
        );
    }

    #[test]
    fn selects_only_node_exe_for_a_node_request() {
        let fixture = Fixture::new("node");
        let request = fixture.request(BaselineCommand::NodeRun);
        let prepared = prepare_windows_command_launch(
            &fixture.workspace,
            &fixture.helper,
            &fixture.node,
            &request,
        )
        .unwrap();
        assert_eq!(
            prepared.tool_executable().file_name().unwrap(),
            NODE_EXECUTABLE_FILE_NAME
        );
        assert!(prepared.workspace_writable());
        assert_eq!(prepared.tool(), RegisteredTool::Node);
        assert_eq!(prepared.request(), request);

        assert_eq!(
            prepare_windows_command_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.git,
                &fixture.request(BaselineCommand::NodeRun),
            ),
            Err(WindowsCommandLaunchPreparationError::UnsafeToolExecutable)
        );
    }

    #[test]
    fn rejects_relative_wrong_named_and_malformed_inputs() {
        let fixture = Fixture::new("reject");
        let request = fixture.request(BaselineCommand::GitStatus);
        assert_eq!(
            prepare_windows_command_launch(
                Path::new("workspace"),
                &fixture.helper,
                &fixture.git,
                &request,
            ),
            Err(WindowsCommandLaunchPreparationError::UnsafeWorkspace)
        );
        let wrong_helper = fixture.root.join("product").join("other.exe");
        fs::write(&wrong_helper, b"helper").unwrap();
        assert_eq!(
            prepare_windows_command_launch(
                &fixture.workspace,
                &wrong_helper,
                &fixture.git,
                &request,
            ),
            Err(WindowsCommandLaunchPreparationError::UnsafeCommandHelper)
        );
        assert_eq!(
            prepare_windows_command_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.git,
                b"not-json",
            ),
            Err(WindowsCommandLaunchPreparationError::InvalidRequest)
        );
    }

    #[test]
    fn rejects_tools_not_implemented_by_the_product_launcher() {
        let fixture = Fixture::new("tool");
        let request = fixture.request(BaselineCommand::NpmTest);
        assert_eq!(
            prepare_windows_command_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.git,
                &request,
            ),
            Err(WindowsCommandLaunchPreparationError::UnsupportedTool)
        );
    }

    #[test]
    fn package_install_preparation_enables_only_internet_client_and_verified_npm() {
        let fixture = Fixture::new("npm-install");
        let npm = fixture.create_npm_runtime();
        let request = fixture.package_install_request();
        let prepared = prepare_windows_npm_package_install_launch(
            &fixture.workspace,
            &fixture.helper,
            &fixture.node,
            &npm,
            &request,
        )
        .unwrap();
        assert_eq!(prepared.tool(), RegisteredTool::Node);
        assert!(prepared.workspace_writable());
        assert!(prepared.network_client_enabled());
        let canonical_npm = npm.canonicalize().unwrap();
        assert_eq!(prepared.npm_runtime(), Some(canonical_npm.as_path()));
        assert_eq!(
            prepared.isolation_requirements(),
            &WINDOWS_PACKAGE_INSTALL_ISOLATION_REQUIREMENTS
        );
        assert!(
            prepared
                .isolation_requirements()
                .contains(&WindowsHostIsolationRequirement::InternetClientOnlyCapability)
        );
        assert!(
            !prepared
                .isolation_requirements()
                .contains(&WindowsHostIsolationRequirement::NoNetworkCapabilities)
        );
    }

    #[test]
    fn package_install_preparation_rejects_baseline_requests_and_wrong_npm_layouts() {
        let fixture = Fixture::new("npm-reject");
        let npm = fixture.create_npm_runtime();
        assert_eq!(
            prepare_windows_npm_package_install_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.node,
                &npm,
                &fixture.request(BaselineCommand::NodeRun),
            ),
            Err(WindowsCommandLaunchPreparationError::InvalidRequest)
        );
        let outside = fixture.root.join("other-npm");
        fs::create_dir_all(outside.join("bin")).unwrap();
        fs::write(outside.join("bin").join("npm-cli.js"), b"npm cli").unwrap();
        fs::write(outside.join("package.json"), b"{}").unwrap();
        assert_eq!(
            prepare_windows_npm_package_install_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.node,
                &outside,
                &fixture.package_install_request(),
            ),
            Err(WindowsCommandLaunchPreparationError::UnsafeNpmRuntime)
        );
    }

    #[test]
    fn npm_script_preparation_is_offline_and_accepts_only_fixed_npm_plans() {
        let fixture = Fixture::new("npm-script");
        let npm = fixture.create_npm_runtime();
        let request = fixture.request(BaselineCommand::NpmBuild);
        let prepared = prepare_windows_npm_script_launch(
            &fixture.workspace,
            &fixture.helper,
            &fixture.node,
            &npm,
            &request,
        )
        .unwrap();
        assert_eq!(prepared.tool(), RegisteredTool::Npm);
        assert!(prepared.workspace_writable());
        assert!(!prepared.network_client_enabled());
        assert_eq!(
            prepared.npm_runtime(),
            Some(npm.canonicalize().unwrap().as_path())
        );
        assert_eq!(
            prepared.isolation_requirements(),
            &WINDOWS_HOST_ISOLATION_REQUIREMENTS
        );

        assert_eq!(
            prepare_windows_npm_script_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.node,
                &npm,
                &fixture.request(BaselineCommand::NodeRun),
            ),
            Err(WindowsCommandLaunchPreparationError::UnsupportedTool)
        );
        assert_eq!(
            prepare_windows_npm_script_launch(
                &fixture.workspace,
                &fixture.helper,
                &fixture.node,
                &npm,
                &fixture.package_install_request(),
            ),
            Err(WindowsCommandLaunchPreparationError::InvalidRequest)
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "creates and removes an isolated AppContainer profile, ACL, drive mapping, and Job Object"]
    fn live_isolation_resources_are_bounded_and_removed() {
        use std::process::Command;

        let fixture = Fixture::new("live-isolation");
        fs::write(fixture.workspace.join("existing.txt"), b"existing").unwrap();
        let request = fixture.request(BaselineCommand::GitStatus);
        let prepared = prepare_windows_command_launch(
            &fixture.workspace,
            &fixture.helper,
            &fixture.git,
            &request,
        )
        .unwrap();
        let isolation = establish_windows_command_isolation(&prepared).unwrap();
        let sid = isolation.sid_for_test().to_owned();
        let mapped_root = isolation.workspace_drive_root().to_owned();
        let child_acl = Command::new("icacls.exe")
            .arg(fixture.workspace.join("existing.txt"))
            .output()
            .unwrap();
        assert!(child_acl.status.success());
        assert!(String::from_utf8_lossy(&child_acl.stdout).contains(&sid));
        assert_eq!(
            fs::read(mapped_root.join("existing.txt")).unwrap(),
            b"existing"
        );
        isolation.close().unwrap();
        assert!(!mapped_root.exists());
        let acl = Command::new("icacls.exe")
            .arg(&fixture.workspace)
            .output()
            .unwrap();
        assert!(acl.status.success());
        assert!(!String::from_utf8_lossy(&acl.stdout).contains(&sid));
        let child_acl = Command::new("icacls.exe")
            .arg(fixture.workspace.join("existing.txt"))
            .output()
            .unwrap();
        assert!(child_acl.status.success());
        assert!(!String::from_utf8_lossy(&child_acl.stdout).contains(&sid));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "builds and launches the real command helper in an isolated temporary Git fixture"]
    fn live_helper_process_runs_after_job_assignment_and_cleans_up() {
        let fixture = Fixture::new("live-helper");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let git = std::env::var_os("MOE_TEST_GIT_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_GIT_EXE must name the verified Git executable");
        let helper = fixture.workspace.join(COMMAND_HELPER_FILE_NAME);
        fs::copy(helper_source, &helper).unwrap();
        let initialized = std::process::Command::new(&git)
            .args(["init", "--quiet"])
            .current_dir(&fixture.workspace)
            .status()
            .unwrap();
        assert!(initialized.success());
        let request = fixture.request(BaselineCommand::GitStatus);
        let prepared =
            prepare_windows_command_launch(&fixture.workspace, &helper, &git, &request).unwrap();
        let outcome = execute_windows_command_launch(&prepared).unwrap();
        assert!(outcome.success());
        assert_eq!(
            outcome.completion(),
            WindowsCommandLaunchCompletion::Completed
        );
        assert!(!outcome.stdout().is_empty());
        assert!(outcome.stderr().is_empty());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "builds and launches the real command helper for one fixed Node entry file in an isolated temporary fixture"]
    fn live_helper_runs_fixed_node_entry_and_cleans_up() {
        let fixture = Fixture::new("live-node-helper");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let helper = fixture.workspace.join(COMMAND_HELPER_FILE_NAME);
        fs::copy(helper_source, &helper).unwrap();
        fs::write(
            fixture.workspace.join("mio-main.mjs"),
            b"import { writeFileSync } from 'node:fs';\nwriteFileSync('node-result.txt', 'node-ok\\n');\nconsole.log('MIO_NODE_OK');\n",
        )
        .unwrap();

        let request = fixture.request(BaselineCommand::NodeRun);
        let prepared =
            prepare_windows_command_launch(&fixture.workspace, &helper, &node, &request).unwrap();
        let outcome = execute_windows_command_launch(&prepared).unwrap();

        assert!(
            outcome.success(),
            "exit={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert_eq!(
            outcome.completion(),
            WindowsCommandLaunchCompletion::Completed
        );
        assert_eq!(outcome.stdout(), b"MIO_NODE_OK\n");
        assert!(outcome.stderr().is_empty());
        assert_eq!(
            fs::read(fixture.workspace.join("node-result.txt")).unwrap(),
            b"node-ok\n"
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "downloads one exact public npm package into an isolated temporary fixture"]
    fn live_helper_installs_one_exact_npm_package_without_scripts() {
        let fixture = Fixture::new("live-npm-install");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let npm = node
            .parent()
            .expect("node.exe must have a parent")
            .join("node_modules")
            .join("npm");
        let helper = fixture.workspace.join(COMMAND_HELPER_FILE_NAME);
        fs::copy(helper_source, &helper).unwrap();
        fs::write(
            fixture.workspace.join("package.json"),
            br#"{"name":"mio-npm-boundary-fixture","private":true,"version":"1.0.0"}"#,
        )
        .unwrap();

        let request = fixture.package_install_request();
        let prepared = prepare_windows_npm_package_install_launch(
            &fixture.workspace,
            &helper,
            &node,
            &npm,
            &request,
        )
        .unwrap();
        let outcome = execute_windows_command_launch(&prepared).unwrap();
        assert!(
            outcome.success(),
            "exit={} stdout={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stdout()),
            String::from_utf8_lossy(outcome.stderr())
        );
        let package_json = fs::read_to_string(fixture.workspace.join("package.json")).unwrap();
        assert!(package_json.contains(r#""is-number""#));
        assert!(package_json.contains(r#""7.0.0""#));
        assert!(fixture.workspace.join("package-lock.json").is_file());
        assert!(
            fixture
                .workspace
                .join("node_modules")
                .join("is-number")
                .join("package.json")
                .is_file()
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "downloads one exact public npm package with the product helper outside the workspace"]
    fn live_external_product_helper_installs_one_exact_npm_package_without_scripts() {
        let fixture = Fixture::new("live-external-npm-install");
        let helper = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let npm = node
            .parent()
            .expect("node.exe must have a parent")
            .join("node_modules")
            .join("npm");
        fs::write(
            fixture.workspace.join("package.json"),
            br#"{"name":"mio-npm-external-helper-fixture","private":true,"version":"1.0.0"}"#,
        )
        .unwrap();

        let request = fixture.package_install_request();
        let prepared = prepare_windows_npm_package_install_launch(
            &fixture.workspace,
            &helper,
            &node,
            &npm,
            &request,
        )
        .unwrap();
        let outcome = std::thread::spawn(move || execute_windows_command_launch(&prepared))
            .join()
            .unwrap()
            .unwrap();
        assert!(
            outcome.success(),
            "exit={} stdout={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stdout()),
            String::from_utf8_lossy(outcome.stderr())
        );
        let package_json = fs::read_to_string(fixture.workspace.join("package.json")).unwrap();
        assert!(package_json.contains(r#""is-number""#));
        assert!(package_json.contains(r#""7.0.0""#));
        assert!(fixture.workspace.join("package-lock.json").is_file());
        assert!(
            fixture
                .workspace
                .join("node_modules")
                .join("is-number")
                .join("package.json")
                .is_file()
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "builds and launches the real command helper in a small D-drive Git fixture"]
    fn live_helper_process_runs_from_configured_d_drive_fixture() {
        let fixture_parent = std::env::var_os("MOE_TEST_D_DRIVE_FIXTURE_ROOT")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_D_DRIVE_FIXTURE_ROOT must name an existing directory");
        assert_eq!(
            fixture_parent
                .components()
                .next()
                .map(|part| part.as_os_str()),
            Some(OsStr::new("D:")),
            "the configured fixture root must be on D:"
        );
        let fixture = Fixture::new_in(&fixture_parent, "live-helper-d-drive");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let git = std::env::var_os("MOE_TEST_GIT_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_GIT_EXE must name the verified Git executable");
        let helper = fixture.workspace.join(COMMAND_HELPER_FILE_NAME);
        fs::copy(helper_source, &helper).unwrap();
        let initialized = std::process::Command::new(&git)
            .args(["init", "--quiet"])
            .current_dir(&fixture.workspace)
            .status()
            .unwrap();
        assert!(initialized.success());
        let request = fixture.request(BaselineCommand::GitStatus);
        let prepared =
            prepare_windows_command_launch(&fixture.workspace, &helper, &git, &request).unwrap();
        let outcome = execute_windows_command_launch(&prepared).unwrap();
        assert!(
            outcome.success(),
            "exit={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert_eq!(
            outcome.completion(),
            WindowsCommandLaunchCompletion::Completed
        );
        assert!(!outcome.stdout().is_empty());
        assert!(outcome.stderr().is_empty());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "downloads one exact public npm package in a small D-drive fixture"]
    fn live_external_product_helper_installs_npm_in_configured_d_drive_fixture() {
        let fixture_parent = std::env::var_os("MOE_TEST_D_DRIVE_FIXTURE_ROOT")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_D_DRIVE_FIXTURE_ROOT must name an existing directory");
        let fixture = Fixture::new_in(&fixture_parent, "live-npm-d-drive");
        let helper = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let npm = node
            .parent()
            .expect("node.exe must have a parent")
            .join("node_modules")
            .join("npm");
        fs::write(
            fixture.workspace.join("package.json"),
            br#"{"name":"mio-npm-d-drive-fixture","private":true,"version":"1.0.0"}"#,
        )
        .unwrap();

        let request = fixture.package_install_request();
        let prepared = prepare_windows_npm_package_install_launch(
            &fixture.workspace,
            &helper,
            &node,
            &npm,
            &request,
        )
        .unwrap();
        let outcome = execute_windows_command_launch(&prepared).unwrap();
        assert!(
            outcome.success(),
            "exit={} stdout={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stdout()),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert!(fixture.workspace.join("package-lock.json").is_file());
        assert!(
            fixture
                .workspace
                .join("node_modules")
                .join("is-number")
                .join("package.json")
                .is_file()
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "measures a root-only temporary ACL in an isolated D-drive Git fixture"]
    fn live_root_only_acl_runs_git_status_quickly_and_cleans_descendants() {
        use std::process::Command;
        use std::time::{Duration, Instant};

        let fixture_parent = std::env::var_os("MOE_TEST_D_DRIVE_FIXTURE_ROOT")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_D_DRIVE_FIXTURE_ROOT must name an existing directory");
        let fixture = Fixture::new_in(&fixture_parent, "root-only-acl");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let git = std::env::var_os("MOE_TEST_GIT_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_GIT_EXE must name the verified Git executable");
        let helper = fixture.workspace.join(COMMAND_HELPER_FILE_NAME);
        fs::copy(helper_source, &helper).unwrap();
        let nested = fixture.workspace.join("nested").join("deeper");
        fs::create_dir_all(&nested).unwrap();
        for index in 0..256 {
            fs::write(
                nested.join(format!("fixture-{index:03}.txt")),
                format!("fixture {index}\n"),
            )
            .unwrap();
        }
        let initialized = Command::new(&git)
            .args(["init", "--quiet"])
            .current_dir(&fixture.workspace)
            .status()
            .unwrap();
        assert!(initialized.success());
        let request = fixture.request(BaselineCommand::GitStatus);
        let prepared =
            prepare_windows_command_launch(&fixture.workspace, &helper, &git, &request).unwrap();

        let started = Instant::now();
        let (outcome, sid) =
            execute_windows_command_launch_with_root_only_acl_for_test(&prepared).unwrap();
        let elapsed = started.elapsed();

        assert!(
            outcome.success(),
            "exit={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert!(
            String::from_utf8_lossy(outcome.stdout()).contains("nested/"),
            "Git status did not observe the existing descendant fixture"
        );
        assert!(
            elapsed < Duration::from_secs(30),
            "root-only ACL fixture took {elapsed:?}"
        );
        for path in [&fixture.workspace, &nested.join("fixture-255.txt")] {
            let acl = Command::new("icacls.exe").arg(path).output().unwrap();
            assert!(acl.status.success());
            assert!(
                !String::from_utf8_lossy(&acl.stdout).contains(&sid),
                "temporary SID remained on {}",
                path.display()
            );
        }
        eprintln!("root-only ACL fixture completed in {elapsed:?}");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "runs fixed Git status once against an explicitly configured real workspace"]
    fn live_root_only_acl_runs_git_status_in_configured_workspace() {
        use std::process::Command;
        use std::time::{Duration, Instant};

        let workspace = std::env::var_os("MOE_TEST_REAL_WORKSPACE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_REAL_WORKSPACE must name an existing absolute directory");
        let helper = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let git = std::env::var_os("MOE_TEST_GIT_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_GIT_EXE must name the verified Git executable");
        let request_fixture = Fixture::new("real-workspace-request");
        let request = request_fixture.request(BaselineCommand::GitStatus);
        let prepared = prepare_windows_command_launch(&workspace, &helper, &git, &request).unwrap();
        let index = workspace.join(".git").join("index");
        let index_before = fs::read(&index).unwrap();

        let started = Instant::now();
        let (outcome, sid) =
            execute_windows_command_launch_with_root_only_acl_for_test(&prepared).unwrap();
        let elapsed = started.elapsed();

        assert!(
            outcome.success(),
            "exit={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert!(
            String::from_utf8_lossy(outcome.stdout()).starts_with("## "),
            "Git status did not return a branch header: {}",
            String::from_utf8_lossy(outcome.stdout())
        );
        assert!(
            elapsed < Duration::from_secs(30),
            "real-workspace root-only ACL run took {elapsed:?}"
        );
        assert_eq!(fs::read(&index).unwrap(), index_before, "Git index changed");
        for relative in [".", ".git", ".editorconfig", "apps", "target"] {
            let path = workspace.join(relative);
            let acl = Command::new("icacls.exe").arg(&path).output().unwrap();
            assert!(acl.status.success());
            assert!(
                !String::from_utf8_lossy(&acl.stdout).contains(&sid),
                "temporary SID remained on {}",
                path.display()
            );
        }
        eprintln!(
            "real-workspace root-only ACL run completed in {elapsed:?}\n{}",
            String::from_utf8_lossy(outcome.stdout())
        );
    }
}
