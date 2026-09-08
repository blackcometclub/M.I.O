use crate::command_confirmations::{
    DesktopCommandAuthorizationOutcome, DesktopCommandConfirmations, DesktopRoomCommandContext,
};
use crate::command_helper_product::{BundledCommandHelper, DesktopCommandHelperProduct};
use crate::command_toolchain_product::{
    DesktopCommandToolchainProduct, VerifiedGitExecutable, VerifiedNodeExecutable,
    VerifiedNpmRuntime,
};
use crate::room_workspace::DesktopRoomWorkspaces;
use moe_adapter_sdk::{TextTurnRequest, TextTurnWorkspaceAccess};
use moe_command_broker::{
    ActionTimeCommand, BaselineCommand, CommandClassification, NpmPackageInstallClassification,
    NpmPackageInstallRequest, RegisteredTool, WorkspaceCommandAccess, WorkspaceCommandIntent,
    WorkspaceCommandRequest, classify_command, classify_npm_package_install,
};
use moe_command_helper_protocol::{encode_npm_package_install_request, encode_request};
use moe_windows_command_launcher::{
    PreparedWindowsCommandLaunch, WindowsCommandLaunchOutcome, execute_windows_command_launch,
    prepare_windows_command_launch, prepare_windows_npm_package_install_launch,
    prepare_windows_npm_script_launch,
};
use moe_workspace_broker::WorkspaceBoundary;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopCommandPreparationError {
    CommandHelperUnavailable,
    RoomSessionUnavailable,
    WorkspaceUnavailable,
    WorkspaceChanged,
    WorkspaceOwnerUnavailable,
    WorkspaceOwnerMismatch,
    CommandDenied,
    ToolNotReady,
    GitExecutableUnavailable,
    NodeExecutableUnavailable,
    NpmRuntimeUnavailable,
    CommandConfirmationUnavailable,
    CommandAuthorizationDenied,
    BackendPreparationFailed,
    RequestEncodingFailed,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopCommandExecutionError {
    RoomSessionUnavailable,
    WorkspaceUnavailable,
    WorkspaceChanged,
    WorkspaceOwnerUnavailable,
    WorkspaceOwnerChanged,
    CommandHelperChanged,
    GitExecutableChanged,
    NodeExecutableChanged,
    NpmRuntimeChanged,
    BackendPreparationFailed,
    BackendExecutionFailed,
}

fn require_current_user_owner(path: &Path) -> Result<(), DesktopCommandPreparationError> {
    match current_user_owns_directory(path) {
        Ok(true) => Ok(()),
        Ok(false) => Err(DesktopCommandPreparationError::WorkspaceOwnerMismatch),
        Err(()) => Err(DesktopCommandPreparationError::WorkspaceOwnerUnavailable),
    }
}

fn revalidate_current_user_owner(path: &Path) -> Result<(), DesktopCommandExecutionError> {
    match current_user_owns_directory(path) {
        Ok(true) => Ok(()),
        Ok(false) => Err(DesktopCommandExecutionError::WorkspaceOwnerChanged),
        Err(()) => Err(DesktopCommandExecutionError::WorkspaceOwnerUnavailable),
    }
}

#[cfg(not(windows))]
fn current_user_owns_directory(path: &Path) -> Result<bool, ()> {
    path.is_dir().then_some(true).ok_or(())
}

#[cfg(windows)]
fn current_user_owns_directory(path: &Path) -> Result<bool, ()> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        EqualSid, GetTokenInformation, OWNER_SECURITY_INFORMATION, PSID, TOKEN_QUERY, TOKEN_USER,
        TokenUser,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    struct SecurityDescriptor(windows_sys::Win32::Security::PSECURITY_DESCRIPTOR);

    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    LocalFree(self.0.cast());
                }
            }
        }
    }

    if !path.is_absolute() || !path.is_dir() {
        return Err(());
    }
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut owner: PSID = null_mut();
    let mut descriptor = null_mut();
    if unsafe {
        GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    } != 0
        || owner.is_null()
        || descriptor.is_null()
    {
        return Err(());
    }
    let _descriptor = SecurityDescriptor(descriptor);

    let mut token = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0
        || token.is_null()
    {
        return Err(());
    }
    let token = Handle(token);
    let mut required = 0u32;
    unsafe {
        GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut required);
    }
    if required < size_of::<TOKEN_USER>() as u32 {
        return Err(());
    }
    let mut storage = vec![0usize; (required as usize).div_ceil(size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            storage.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(());
    }
    let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
    if user.User.Sid.is_null() {
        return Err(());
    }
    Ok(unsafe { EqualSid(owner, user.User.Sid) } != 0)
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedDesktopCommand {
    room_context: DesktopRoomCommandContext,
    workspace_root: PathBuf,
    execution_root: PathBuf,
    execution_guard: Option<PreparedExecutionGuard>,
    helper: BundledCommandHelper,
    tool_executable: VerifiedCommandExecutable,
    npm_runtime: Option<VerifiedNpmRuntime>,
    backend_kind: PreparedBackendKind,
    request: Vec<u8>,
    backend: PreparedWindowsCommandLaunch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreparedExecutionGuard {
    NodeEntry(PathBuf),
    PackageManifest(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreparedBackendKind {
    Baseline,
    NpmScript,
    NpmPackageInstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VerifiedCommandExecutable {
    Git(VerifiedGitExecutable),
    Node(VerifiedNodeExecutable),
}

impl VerifiedCommandExecutable {
    fn path(&self) -> &Path {
        match self {
            Self::Git(executable) => executable.path(),
            Self::Node(executable) => executable.path(),
        }
    }

    fn revalidate(&self) -> Result<(), DesktopCommandExecutionError> {
        match self {
            Self::Git(executable) => executable
                .revalidate()
                .map_err(|_| DesktopCommandExecutionError::GitExecutableChanged),
            Self::Node(executable) => executable
                .revalidate()
                .map_err(|_| DesktopCommandExecutionError::NodeExecutableChanged),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopCommandTurnContext {
    room_context: DesktopRoomCommandContext,
    workspace_root: PathBuf,
    access: WorkspaceCommandAccess,
}

#[allow(dead_code)]
impl PreparedDesktopCommand {
    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub(crate) fn helper(&self) -> &BundledCommandHelper {
        &self.helper
    }

    pub(crate) fn tool_executable_path(&self) -> &Path {
        self.tool_executable.path()
    }

    pub(crate) fn request(&self) -> &[u8] {
        &self.request
    }

    pub(crate) fn backend(&self) -> &PreparedWindowsCommandLaunch {
        &self.backend
    }
}

#[allow(dead_code)]
pub(crate) struct DesktopCommandExecution {
    workspaces: Arc<DesktopRoomWorkspaces>,
    confirmations: Arc<DesktopCommandConfirmations>,
    helper: Option<BundledCommandHelper>,
    toolchain: OnceLock<DesktopCommandToolchain>,
}

struct DesktopCommandToolchain {
    git_executable: Option<VerifiedGitExecutable>,
    node_executable: Option<VerifiedNodeExecutable>,
    npm_runtime: Option<VerifiedNpmRuntime>,
    discovery_elapsed: Duration,
}

#[allow(dead_code)]
impl DesktopCommandExecution {
    pub(crate) fn new(
        workspaces: Arc<DesktopRoomWorkspaces>,
        confirmations: Arc<DesktopCommandConfirmations>,
        helper_product: &DesktopCommandHelperProduct,
        toolchain_product: &DesktopCommandToolchainProduct,
    ) -> Self {
        let toolchain = OnceLock::new();
        assert!(
            toolchain
                .set(DesktopCommandToolchain::from_product(
                    toolchain_product,
                    Duration::ZERO,
                ))
                .is_ok(),
            "a new command execution has no toolchain snapshot"
        );
        Self {
            workspaces,
            confirmations,
            helper: helper_product.ready().cloned(),
            toolchain,
        }
    }

    pub(crate) fn pending_toolchain(
        workspaces: Arc<DesktopRoomWorkspaces>,
        confirmations: Arc<DesktopCommandConfirmations>,
        helper_product: &DesktopCommandHelperProduct,
    ) -> Self {
        Self {
            workspaces,
            confirmations,
            helper: helper_product.ready().cloned(),
            toolchain: OnceLock::new(),
        }
    }

    pub(crate) fn finish_toolchain_discovery(
        &self,
        toolchain_product: &DesktopCommandToolchainProduct,
        elapsed: Duration,
    ) {
        let _ = self.toolchain.set(DesktopCommandToolchain::from_product(
            toolchain_product,
            elapsed,
        ));
    }

    pub(crate) fn toolchain_discovery_elapsed(&self) -> Option<Duration> {
        self.toolchain
            .get()
            .map(|toolchain| toolchain.discovery_elapsed)
    }

    pub(crate) fn context_for_turn(
        &self,
        request: &TextTurnRequest,
    ) -> Result<DesktopCommandTurnContext, DesktopCommandPreparationError> {
        let room_id = request
            .room_id()
            .ok_or(DesktopCommandPreparationError::RoomSessionUnavailable)?;
        let requested_workspace = request
            .workspace()
            .ok_or(DesktopCommandPreparationError::WorkspaceUnavailable)?;
        let workspace = self
            .workspaces
            .available_workspace(room_id)
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?
            .ok_or(DesktopCommandPreparationError::WorkspaceUnavailable)?;
        if workspace.root() != requested_workspace.root() {
            return Err(DesktopCommandPreparationError::WorkspaceChanged);
        }
        let room_context = self
            .confirmations
            .active_room_context(room_id, &workspace)
            .map_err(|_| DesktopCommandPreparationError::RoomSessionUnavailable)?
            .ok_or(DesktopCommandPreparationError::RoomSessionUnavailable)?;
        let access = match requested_workspace.access() {
            TextTurnWorkspaceAccess::ReadOnly => WorkspaceCommandAccess::ReadOnly,
            TextTurnWorkspaceAccess::ReadWrite => WorkspaceCommandAccess::ReadWrite,
        };
        Ok(DesktopCommandTurnContext {
            room_context,
            workspace_root: workspace.root().to_owned(),
            access,
        })
    }

    pub(crate) fn prepare_baseline(
        &self,
        context: &DesktopCommandTurnContext,
        working_directory: PathBuf,
        command: BaselineCommand,
    ) -> Result<PreparedDesktopCommand, DesktopCommandPreparationError> {
        let helper = self
            .helper
            .as_ref()
            .ok_or(DesktopCommandPreparationError::CommandHelperUnavailable)?;
        let workspace_root = self.revalidate_turn_workspace(context)?;
        helper
            .revalidate()
            .map_err(|_| DesktopCommandPreparationError::CommandHelperUnavailable)?;
        let npm_script = matches!(
            command,
            BaselineCommand::NpmBuild | BaselineCommand::NpmTest | BaselineCommand::NpmTypecheck
        );
        let (execution_root, execution_guard, broker_working_directory) =
            if command == BaselineCommand::NodeRun || npm_script {
                let boundary = WorkspaceBoundary::new(&workspace_root)
                    .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
                let directory =
                    execution_directory_root(&boundary, &workspace_root, &working_directory)
                        .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
                let required_file = if npm_script {
                    "package.json"
                } else {
                    "mio-main.mjs"
                };
                boundary
                    .existing_file(&workspace_child(&working_directory, required_file))
                    .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
                (
                    directory,
                    Some(if npm_script {
                        PreparedExecutionGuard::PackageManifest(working_directory)
                    } else {
                        PreparedExecutionGuard::NodeEntry(working_directory)
                    }),
                    PathBuf::from("."),
                )
            } else {
                (workspace_root.clone(), None, working_directory)
            };
        let classification = classify_command(
            WorkspaceCommandRequest::new(
                broker_working_directory,
                WorkspaceCommandIntent::Baseline(command),
            ),
            context.access,
        );
        let CommandClassification::Baseline(plan) = classification else {
            return Err(DesktopCommandPreparationError::CommandDenied);
        };
        if execution_guard.is_some() {
            require_current_user_owner(&execution_root)?;
        }
        let toolchain = self
            .toolchain
            .get()
            .ok_or(DesktopCommandPreparationError::ToolNotReady)?;
        let (tool_executable, npm_runtime) = match plan.tool() {
            RegisteredTool::Git => (
                VerifiedCommandExecutable::Git(
                    toolchain
                        .git_executable
                        .as_ref()
                        .ok_or(DesktopCommandPreparationError::GitExecutableUnavailable)?
                        .clone(),
                ),
                None,
            ),
            RegisteredTool::Node => (
                VerifiedCommandExecutable::Node(
                    toolchain
                        .node_executable
                        .as_ref()
                        .ok_or(DesktopCommandPreparationError::NodeExecutableUnavailable)?
                        .clone(),
                ),
                None,
            ),
            RegisteredTool::Npm => (
                VerifiedCommandExecutable::Node(
                    toolchain
                        .node_executable
                        .as_ref()
                        .ok_or(DesktopCommandPreparationError::NodeExecutableUnavailable)?
                        .clone(),
                ),
                Some(
                    toolchain
                        .npm_runtime
                        .as_ref()
                        .ok_or(DesktopCommandPreparationError::NpmRuntimeUnavailable)?
                        .clone(),
                ),
            ),
            RegisteredTool::Cargo => {
                return Err(DesktopCommandPreparationError::ToolNotReady);
            }
        };
        tool_executable.revalidate().map_err(|error| match error {
            DesktopCommandExecutionError::GitExecutableChanged => {
                DesktopCommandPreparationError::GitExecutableUnavailable
            }
            DesktopCommandExecutionError::NodeExecutableChanged => {
                DesktopCommandPreparationError::NodeExecutableUnavailable
            }
            _ => DesktopCommandPreparationError::ToolNotReady,
        })?;
        if let Some(npm_runtime) = &npm_runtime {
            npm_runtime
                .revalidate()
                .map_err(|_| DesktopCommandPreparationError::NpmRuntimeUnavailable)?;
        }
        let request = encode_request(&plan, context.access)
            .map_err(|_| DesktopCommandPreparationError::RequestEncodingFailed)?;
        let backend = if let Some(npm_runtime) = &npm_runtime {
            prepare_windows_npm_script_launch(
                &execution_root,
                helper.path(),
                tool_executable.path(),
                npm_runtime.root(),
                &request,
            )
        } else {
            prepare_windows_command_launch(
                &execution_root,
                helper.path(),
                tool_executable.path(),
                &request,
            )
        }
        .map_err(|_| DesktopCommandPreparationError::BackendPreparationFailed)?;
        Ok(PreparedDesktopCommand {
            room_context: context.room_context.clone(),
            workspace_root,
            execution_root,
            execution_guard,
            helper: helper.clone(),
            tool_executable,
            npm_runtime,
            backend_kind: if npm_script {
                PreparedBackendKind::NpmScript
            } else {
                PreparedBackendKind::Baseline
            },
            request,
            backend,
        })
    }

    pub(crate) fn prepare_npm_package_install(
        &self,
        context: &DesktopCommandTurnContext,
        working_directory: PathBuf,
        package_name: String,
        exact_version: String,
    ) -> Result<PreparedDesktopCommand, DesktopCommandPreparationError> {
        let helper = self
            .helper
            .as_ref()
            .ok_or(DesktopCommandPreparationError::CommandHelperUnavailable)?;
        let toolchain = self
            .toolchain
            .get()
            .ok_or(DesktopCommandPreparationError::ToolNotReady)?;
        let node = toolchain
            .node_executable
            .as_ref()
            .ok_or(DesktopCommandPreparationError::NodeExecutableUnavailable)?;
        let npm_runtime = toolchain
            .npm_runtime
            .as_ref()
            .ok_or(DesktopCommandPreparationError::NpmRuntimeUnavailable)?;
        let workspace_root = self.revalidate_turn_workspace(context)?;
        helper
            .revalidate()
            .map_err(|_| DesktopCommandPreparationError::CommandHelperUnavailable)?;
        node.revalidate()
            .map_err(|_| DesktopCommandPreparationError::NodeExecutableUnavailable)?;
        npm_runtime
            .revalidate()
            .map_err(|_| DesktopCommandPreparationError::NpmRuntimeUnavailable)?;
        let boundary = WorkspaceBoundary::new(&workspace_root)
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        let initial_execution_root =
            execution_directory_root(&boundary, &workspace_root, &working_directory)
                .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        require_current_user_owner(&initial_execution_root)?;
        boundary
            .existing_file(&workspace_child(&working_directory, "package.json"))
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        let classification = classify_npm_package_install(
            NpmPackageInstallRequest::new(PathBuf::from("."), package_name, exact_version),
            context.access,
        );
        let NpmPackageInstallClassification::ActionTimeConfirmationRequired(plan) = classification
        else {
            return Err(DesktopCommandPreparationError::CommandDenied);
        };
        let target_label = format!(
            "{}@{} in {}",
            plan.package_name(),
            plan.exact_version(),
            working_directory.display()
        );
        match self
            .confirmations
            .wait_for_room_authorization(
                &self.workspaces,
                context.room_context.room_id(),
                ActionTimeCommand::PackageInstall,
                target_label,
            )
            .map_err(|_| DesktopCommandPreparationError::CommandConfirmationUnavailable)?
        {
            DesktopCommandAuthorizationOutcome::Authorized(_) => {}
            DesktopCommandAuthorizationOutcome::Denied(_)
            | DesktopCommandAuthorizationOutcome::RoomSessionEnded => {
                return Err(DesktopCommandPreparationError::CommandAuthorizationDenied);
            }
        }
        let current_workspace = self.revalidate_turn_workspace(context)?;
        if current_workspace != workspace_root {
            return Err(DesktopCommandPreparationError::WorkspaceChanged);
        }
        let boundary = WorkspaceBoundary::new(&current_workspace)
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        let execution_root =
            execution_directory_root(&boundary, &current_workspace, &working_directory)
                .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        require_current_user_owner(&execution_root)?;
        boundary
            .existing_file(&workspace_child(&working_directory, "package.json"))
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?;
        let request = encode_npm_package_install_request(&plan, context.access)
            .map_err(|_| DesktopCommandPreparationError::RequestEncodingFailed)?;
        let backend = prepare_windows_npm_package_install_launch(
            &execution_root,
            helper.path(),
            node.path(),
            npm_runtime.root(),
            &request,
        )
        .map_err(|_| DesktopCommandPreparationError::BackendPreparationFailed)?;
        Ok(PreparedDesktopCommand {
            room_context: context.room_context.clone(),
            workspace_root,
            execution_root,
            execution_guard: Some(PreparedExecutionGuard::PackageManifest(working_directory)),
            helper: helper.clone(),
            tool_executable: VerifiedCommandExecutable::Node(node.clone()),
            npm_runtime: Some(npm_runtime.clone()),
            backend_kind: PreparedBackendKind::NpmPackageInstall,
            request,
            backend,
        })
    }

    pub(crate) fn revalidate_turn_workspace(
        &self,
        context: &DesktopCommandTurnContext,
    ) -> Result<PathBuf, DesktopCommandPreparationError> {
        let workspace = self
            .workspaces
            .available_workspace(context.room_context.room_id())
            .map_err(|_| DesktopCommandPreparationError::WorkspaceUnavailable)?
            .ok_or(DesktopCommandPreparationError::WorkspaceUnavailable)?;
        if workspace.root() != context.workspace_root
            || workspace.identity_key() != context.room_context.workspace_key()
        {
            return Err(DesktopCommandPreparationError::WorkspaceChanged);
        }
        let active = self
            .confirmations
            .matches_active_context(&context.room_context)
            .map_err(|_| DesktopCommandPreparationError::RoomSessionUnavailable)?;
        if !active {
            return Err(DesktopCommandPreparationError::RoomSessionUnavailable);
        }
        Ok(workspace.root().to_owned())
    }

    pub(crate) fn execute_prepared(
        &self,
        prepared: PreparedDesktopCommand,
    ) -> Result<WindowsCommandLaunchOutcome, DesktopCommandExecutionError> {
        let workspace = self
            .workspaces
            .available_workspace(prepared.room_context.room_id())
            .map_err(|_| DesktopCommandExecutionError::WorkspaceUnavailable)?
            .ok_or(DesktopCommandExecutionError::WorkspaceUnavailable)?;
        if workspace.identity_key() != prepared.room_context.workspace_key()
            || workspace.root() != prepared.workspace_root
        {
            return Err(DesktopCommandExecutionError::WorkspaceChanged);
        }
        if !self
            .confirmations
            .matches_active_context(&prepared.room_context)
            .map_err(|_| DesktopCommandExecutionError::RoomSessionUnavailable)?
        {
            return Err(DesktopCommandExecutionError::RoomSessionUnavailable);
        }
        prepared
            .helper
            .revalidate()
            .map_err(|_| DesktopCommandExecutionError::CommandHelperChanged)?;
        prepared.tool_executable.revalidate()?;
        if let Some(npm_runtime) = &prepared.npm_runtime {
            npm_runtime
                .revalidate()
                .map_err(|_| DesktopCommandExecutionError::NpmRuntimeChanged)?;
        }
        let execution_root = if let Some(guard) = &prepared.execution_guard {
            let boundary = WorkspaceBoundary::new(workspace.root())
                .map_err(|_| DesktopCommandExecutionError::WorkspaceUnavailable)?;
            let relative = match guard {
                PreparedExecutionGuard::NodeEntry(relative) => {
                    boundary
                        .existing_file(&workspace_child(relative, "mio-main.mjs"))
                        .map_err(|_| DesktopCommandExecutionError::WorkspaceUnavailable)?;
                    relative
                }
                PreparedExecutionGuard::PackageManifest(relative) => {
                    boundary
                        .existing_file(&workspace_child(relative, "package.json"))
                        .map_err(|_| DesktopCommandExecutionError::WorkspaceUnavailable)?;
                    relative
                }
            };
            let current = execution_directory_root(&boundary, workspace.root(), relative)
                .map_err(|_| DesktopCommandExecutionError::WorkspaceUnavailable)?;
            if current != prepared.execution_root {
                return Err(DesktopCommandExecutionError::WorkspaceChanged);
            }
            current
        } else {
            workspace.root().to_owned()
        };
        if prepared.execution_guard.is_some() {
            revalidate_current_user_owner(&execution_root)?;
        }
        let backend = match prepared.backend_kind {
            PreparedBackendKind::NpmPackageInstall => prepare_windows_npm_package_install_launch(
                &execution_root,
                prepared.helper.path(),
                prepared.tool_executable.path(),
                prepared
                    .npm_runtime
                    .as_ref()
                    .ok_or(DesktopCommandExecutionError::NpmRuntimeChanged)?
                    .root(),
                &prepared.request,
            ),
            PreparedBackendKind::NpmScript => prepare_windows_npm_script_launch(
                &execution_root,
                prepared.helper.path(),
                prepared.tool_executable.path(),
                prepared
                    .npm_runtime
                    .as_ref()
                    .ok_or(DesktopCommandExecutionError::NpmRuntimeChanged)?
                    .root(),
                &prepared.request,
            ),
            PreparedBackendKind::Baseline => prepare_windows_command_launch(
                &execution_root,
                prepared.helper.path(),
                prepared.tool_executable.path(),
                &prepared.request,
            ),
        }
        .map_err(|_| DesktopCommandExecutionError::BackendPreparationFailed)?;
        execute_windows_command_launch(&backend)
            .map_err(|_| DesktopCommandExecutionError::BackendExecutionFailed)
    }
}

impl DesktopCommandToolchain {
    fn from_product(product: &DesktopCommandToolchainProduct, discovery_elapsed: Duration) -> Self {
        Self {
            git_executable: product.ready_git().cloned(),
            node_executable: product.ready_node().cloned(),
            npm_runtime: product.ready_npm_runtime().cloned(),
            discovery_elapsed,
        }
    }
}

fn workspace_child(directory: &Path, file_name: &str) -> PathBuf {
    if directory == Path::new(".") {
        PathBuf::from(file_name)
    } else {
        directory.join(file_name)
    }
}

fn execution_directory_root(
    boundary: &WorkspaceBoundary,
    workspace_root: &Path,
    relative: &Path,
) -> Result<PathBuf, ()> {
    if relative == Path::new(".") {
        Ok(workspace_root.to_owned())
    } else {
        boundary
            .existing_directory(relative)
            .map(|directory| directory.resolved_for_host().to_owned())
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_confirmations::DesktopConfirmationDecision;
    use crate::command_helper_product::DesktopCommandHelperProduct;
    use crate::command_toolchain_product::DesktopCommandToolchainProduct;
    use moe_command_helper_protocol::decode_request;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "moe-command-execution-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn accepts_a_directory_owned_by_the_current_process_user() {
        let fixture = TestDirectory::new("current-user-owner");

        assert_eq!(current_user_owns_directory(&fixture.0), Ok(true));
        assert_eq!(require_current_user_owner(&fixture.0), Ok(()));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "checks one explicitly configured directory owned by another Windows account"]
    fn rejects_a_configured_directory_owned_by_another_user() {
        let directory = std::env::var_os("MOE_TEST_OTHER_OWNER_DIRECTORY")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_OTHER_OWNER_DIRECTORY must name an existing directory");

        assert_eq!(current_user_owns_directory(&directory), Ok(false));
        assert_eq!(
            require_current_user_owner(&directory),
            Err(DesktopCommandPreparationError::WorkspaceOwnerMismatch)
        );
    }

    fn ready_helper(root: &Path) -> DesktopCommandHelperProduct {
        let desktop = root.join("mio-desktop.exe");
        let helper = root.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop").unwrap();
        fs::write(&helper, b"helper").unwrap();
        let expected = format!("{:x}", Sha256::digest(b"helper"));
        DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected).unwrap(),
        )
    }

    fn active_execution(
        label: &str,
    ) -> (
        TestDirectory,
        DesktopCommandExecution,
        Arc<DesktopCommandConfirmations>,
    ) {
        let root = TestDirectory::new(label);
        let workspace = root.0.join("workspace");
        let product = root.0.join("product");
        let toolchain = root.0.join("toolchain");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&product).unwrap();
        fs::create_dir_all(toolchain.join("Git").join("bin")).unwrap();
        fs::create_dir_all(toolchain.join("nodejs")).unwrap();
        fs::create_dir_all(
            toolchain
                .join("nodejs")
                .join("node_modules")
                .join("npm")
                .join("bin"),
        )
        .unwrap();
        fs::write(toolchain.join("Git").join("bin").join("git.exe"), b"git").unwrap();
        fs::write(toolchain.join("nodejs").join("node.exe"), b"node").unwrap();
        fs::write(
            toolchain
                .join("nodejs")
                .join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js"),
            b"npm cli",
        )
        .unwrap();
        fs::write(
            toolchain
                .join("nodejs")
                .join("node_modules")
                .join("npm")
                .join("package.json"),
            b"{}",
        )
        .unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let helper = ready_helper(&product);
        let toolchain = DesktopCommandToolchainProduct::locate_for_test(&toolchain).unwrap();
        let execution =
            DesktopCommandExecution::new(workspaces, confirmations.clone(), &helper, &toolchain);
        (root, execution, confirmations)
    }

    fn turn_context(
        execution: &DesktopCommandExecution,
        access: TextTurnWorkspaceAccess,
    ) -> DesktopCommandTurnContext {
        let workspace_root = execution
            .workspaces
            .available_root("room-1")
            .unwrap()
            .unwrap();
        let request = TextTurnRequest::new("dispatch-1".to_owned(), "inspect".to_owned())
            .with_room_id("room-1".to_owned())
            .with_workspace(moe_adapter_sdk::TextTurnWorkspace::new(
                workspace_root,
                access,
            ));
        execution.context_for_turn(&request).unwrap()
    }

    #[test]
    fn prepares_only_a_fixed_git_request_for_the_active_room_workspace() {
        let (_root, execution, _confirmations) = active_execution("git");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        assert!(prepared.workspace_root().is_absolute());
        assert_eq!(
            prepared.helper().path().file_name().unwrap(),
            "mio-command-helper.exe"
        );
        assert_eq!(
            prepared.tool_executable_path().file_name().unwrap(),
            "git.exe"
        );
        let decoded = decode_request(prepared.request()).unwrap();
        assert_eq!(decoded.command(), BaselineCommand::GitStatus);
        let wire = String::from_utf8(prepared.request().to_vec()).unwrap();
        assert!(!wire.contains(&prepared.workspace_root().to_string_lossy().into_owned()));
        assert!(!wire.contains("git.exe"));
        assert_eq!(prepared.backend().request(), prepared.request());
        assert_eq!(
            prepared.backend().workspace_root(),
            prepared.workspace_root()
        );
    }

    #[test]
    fn prepares_only_the_fixed_node_entry_in_one_existing_subdirectory() {
        let (_root, execution, _confirmations) = active_execution("node");
        let workspace = execution
            .workspaces
            .available_root("room-1")
            .unwrap()
            .unwrap();
        let target = workspace.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("mio-main.mjs"), b"console.log('ok');\n").unwrap();
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("target"), BaselineCommand::NodeRun)
            .unwrap();

        assert_eq!(prepared.workspace_root(), workspace);
        assert_eq!(
            prepared.backend().workspace_root(),
            target.canonicalize().unwrap()
        );
        assert_eq!(
            prepared.tool_executable_path().file_name().unwrap(),
            "node.exe"
        );
        let decoded = decode_request(prepared.request()).unwrap();
        assert_eq!(decoded.command(), BaselineCommand::NodeRun);
        assert_eq!(decoded.working_directory(), Path::new("."));
        assert!(prepared.backend().workspace_writable());
    }

    #[test]
    fn turn_context_requires_the_host_room_and_exact_workspace() {
        let (root, execution, _confirmations) = active_execution("turn-context");
        let workspace_root = execution
            .workspaces
            .available_root("room-1")
            .unwrap()
            .unwrap();
        let missing_room = TextTurnRequest::new("dispatch-1".to_owned(), "inspect".to_owned())
            .with_workspace(moe_adapter_sdk::TextTurnWorkspace::new(
                workspace_root,
                TextTurnWorkspaceAccess::ReadOnly,
            ));
        assert_eq!(
            execution.context_for_turn(&missing_room),
            Err(DesktopCommandPreparationError::RoomSessionUnavailable)
        );

        let different_workspace = root.0.join("different-workspace");
        fs::create_dir(&different_workspace).unwrap();
        let wrong_root = TextTurnRequest::new("dispatch-2".to_owned(), "inspect".to_owned())
            .with_room_id("room-1".to_owned())
            .with_workspace(moe_adapter_sdk::TextTurnWorkspace::new(
                different_workspace.canonicalize().unwrap(),
                TextTurnWorkspaceAccess::ReadOnly,
            ));
        assert_eq!(
            execution.context_for_turn(&wrong_root),
            Err(DesktopCommandPreparationError::WorkspaceChanged)
        );
    }

    #[test]
    fn host_owned_read_only_inspection_revalidates_without_helper_or_git() {
        let root = TestDirectory(
            std::env::current_dir()
                .unwrap()
                .join("target")
                .join(format!(
                    "moe-command-execution-host-inspection-{}-{}",
                    std::process::id(),
                    TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                )),
        );
        fs::create_dir_all(&root.0).unwrap();
        let workspace = root.0.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let mut init = git2::RepositoryInitOptions::new();
        init.initial_head("main").external_template(false);
        git2::Repository::init_opts(&workspace, &init).unwrap();
        fs::write(workspace.join("note.txt"), "untracked\n").unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace.clone()).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations.clone(),
            &DesktopCommandHelperProduct::NotBundled,
            &DesktopCommandToolchainProduct::GitUnavailable,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);

        let revalidated = execution.revalidate_turn_workspace(&context).unwrap();
        assert_eq!(revalidated, workspace.canonicalize().unwrap());
        let status = moe_git_status_broker::read_git_status(&revalidated)
            .unwrap()
            .render_porcelain()
            .unwrap();
        assert!(status.contains("?? note.txt"));

        confirmations.end_room_session("room-1").unwrap();
        assert_eq!(
            execution.revalidate_turn_workspace(&context),
            Err(DesktopCommandPreparationError::RoomSessionUnavailable)
        );
    }

    #[test]
    fn pending_toolchain_refuses_commands_then_exposes_the_finished_snapshot() {
        let root = TestDirectory::new("pending-toolchain");
        let workspace = root.0.join("workspace");
        let product = root.0.join("product");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&product).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::pending_toolchain(
            workspaces,
            confirmations,
            &ready_helper(&product),
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);

        assert_eq!(execution.toolchain_discovery_elapsed(), None);
        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus),
            Err(DesktopCommandPreparationError::ToolNotReady)
        );

        execution.finish_toolchain_discovery(
            &DesktopCommandToolchainProduct::GitUnavailable,
            Duration::from_millis(42),
        );
        assert_eq!(
            execution.toolchain_discovery_elapsed(),
            Some(Duration::from_millis(42))
        );
        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus),
            Err(DesktopCommandPreparationError::GitExecutableUnavailable)
        );
    }

    #[test]
    fn refuses_inactive_rooms_unready_tools_and_read_only_builds() {
        let (_root, execution, confirmations) = active_execution("deny");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        confirmations.end_room_session("room-1").unwrap();
        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus),
            Err(DesktopCommandPreparationError::RoomSessionUnavailable)
        );

        let (root, execution, _confirmations) = active_execution("tool");
        fs::write(root.0.join("workspace").join("package.json"), b"{}").unwrap();
        let write_context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let npm = execution
            .prepare_baseline(
                &write_context,
                PathBuf::from("."),
                BaselineCommand::NpmBuild,
            )
            .unwrap();
        assert_eq!(npm.backend().tool(), RegisteredTool::Npm);
        assert!(npm.backend().workspace_writable());
        assert!(!npm.backend().network_client_enabled());
        let read_context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        assert_eq!(
            execution.prepare_baseline(
                &read_context,
                PathBuf::from("."),
                BaselineCommand::NpmBuild,
            ),
            Err(DesktopCommandPreparationError::CommandDenied)
        );
        assert_eq!(
            execution.prepare_baseline(
                &write_context,
                PathBuf::from("."),
                BaselineCommand::CargoBuild,
            ),
            Err(DesktopCommandPreparationError::ToolNotReady)
        );
    }

    #[test]
    fn refuses_to_prepare_without_a_verified_bundled_helper() {
        let root = TestDirectory::new("no-helper");
        let workspace = root.0.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &DesktopCommandHelperProduct::NotBundled,
            &DesktopCommandToolchainProduct::GitUnavailable,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus),
            Err(DesktopCommandPreparationError::CommandHelperUnavailable)
        );
    }

    #[test]
    fn refuses_to_prepare_without_a_verified_git_executable() {
        let root = TestDirectory::new("no-git");
        let workspace = root.0.join("workspace");
        let product = root.0.join("product");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&product).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let helper = ready_helper(&product);
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &helper,
            &DesktopCommandToolchainProduct::GitUnavailable,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus),
            Err(DesktopCommandPreparationError::GitExecutableUnavailable)
        );
    }

    #[test]
    fn execute_prepared_rechecks_helper_git_and_workspace_before_launch() {
        let (_root, execution, _confirmations) = active_execution("execute-helper-change");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        let helper = prepared.helper().path().to_owned();
        fs::remove_file(&helper).unwrap();
        fs::write(&helper, b"replaced helper").unwrap();
        assert_eq!(
            execution.execute_prepared(prepared),
            Err(DesktopCommandExecutionError::CommandHelperChanged)
        );

        let (_root, execution, _confirmations) = active_execution("execute-git-change");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        let git = prepared.tool_executable_path().to_owned();
        fs::remove_file(&git).unwrap();
        fs::write(&git, b"replaced git").unwrap();
        assert_eq!(
            execution.execute_prepared(prepared),
            Err(DesktopCommandExecutionError::GitExecutableChanged)
        );

        let (_root, execution, _confirmations) = active_execution("execute-workspace-change");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        let workspace = prepared.workspace_root().to_owned();
        fs::remove_dir_all(&workspace).unwrap();
        fs::create_dir(&workspace).unwrap();
        assert_eq!(
            execution.execute_prepared(prepared),
            Err(DesktopCommandExecutionError::WorkspaceChanged)
        );
    }

    #[test]
    fn execute_prepared_rejects_a_restarted_room_session() {
        let (_root, execution, confirmations) = active_execution("execute-session-change");
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &execution
                    .workspaces
                    .available_workspace("room-1")
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(
            execution.prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus,),
            Err(DesktopCommandPreparationError::RoomSessionUnavailable)
        );
        assert_eq!(
            execution.execute_prepared(prepared),
            Err(DesktopCommandExecutionError::RoomSessionUnavailable)
        );
    }

    #[test]
    fn package_install_waits_for_exact_confirmation_before_preparing_network_launch() {
        let (_root, execution, confirmations) = active_execution("package-install-confirm");
        let workspace = execution
            .workspaces
            .available_root("room-1")
            .unwrap()
            .unwrap();
        let package = workspace.join("packages").join("app");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.json"), b"{}").unwrap();
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let waiter = std::thread::spawn(move || {
            execution.prepare_npm_package_install(
                &context,
                PathBuf::from("packages/app"),
                "is-number".to_owned(),
                "7.0.0".to_owned(),
            )
        });

        let request_id = (0..200)
            .find_map(|_| {
                let pending = confirmations
                    .pending_request_id_for_room_for_test("room-1")
                    .unwrap();
                if pending.is_none() {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                pending
            })
            .expect("package-install confirmation should become pending");
        confirmations
            .resolve_for_room_for_test(
                "room-1",
                &request_id,
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap();
        let prepared = waiter.join().unwrap().unwrap();
        assert_eq!(prepared.workspace_root(), workspace);
        assert_eq!(prepared.execution_root, package.canonicalize().unwrap());
        assert!(prepared.npm_runtime.is_some());
        assert!(prepared.backend().network_client_enabled());
        assert!(prepared.backend().workspace_writable());
        let decoded =
            moe_command_helper_protocol::decode_npm_package_install_request(prepared.request())
                .unwrap();
        assert_eq!(decoded.working_directory(), Path::new("."));
        assert_eq!(decoded.package_name(), "is-number");
        assert_eq!(decoded.exact_version(), "7.0.0");
    }

    #[test]
    fn package_install_rejects_invalid_specs_before_requesting_confirmation() {
        let (_root, execution, confirmations) = active_execution("package-install-reject");
        let workspace = execution
            .workspaces
            .available_root("room-1")
            .unwrap()
            .unwrap();
        fs::write(workspace.join("package.json"), b"{}").unwrap();
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        assert_eq!(
            execution.prepare_npm_package_install(
                &context,
                PathBuf::from("."),
                "https://example.invalid/package.tgz".to_owned(),
                "latest".to_owned(),
            ),
            Err(DesktopCommandPreparationError::CommandDenied)
        );
        assert_eq!(
            confirmations
                .pending_request_id_for_room_for_test("room-1")
                .unwrap(),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "launches the verified bundled helper and Git through the desktop AppContainer backend"]
    fn live_desktop_execution_runs_only_the_prepared_git_request() {
        let root = TestDirectory::new("live-execute");
        let workspace = root.0.join("workspace");
        let product = root.0.join("product");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&product).unwrap();

        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let git_source = std::env::var_os("MOE_TEST_GIT_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_GIT_EXE must name the verified Git executable");
        let install_root = git_source
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("MOE_TEST_GIT_EXE must use the Git/bin/git.exe product layout");

        let desktop = product.join("mio-desktop.exe");
        let helper = product.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop fixture").unwrap();
        fs::copy(&helper_source, &helper).unwrap();
        let expected_helper_hash = format!("{:x}", Sha256::digest(fs::read(&helper).unwrap()));
        let helper_product = DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected_helper_hash).unwrap(),
        );
        let toolchain_product =
            DesktopCommandToolchainProduct::locate_for_test(install_root).unwrap();
        assert_eq!(
            toolchain_product.ready_git().unwrap().path(),
            git_source.canonicalize().unwrap()
        );
        let initialized = std::process::Command::new(toolchain_product.ready_git().unwrap().path())
            .args(["init", "--quiet"])
            .current_dir(&workspace)
            .status()
            .unwrap();
        assert!(initialized.success());

        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &helper_product,
            &toolchain_product,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadOnly);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("."), BaselineCommand::GitStatus)
            .unwrap();
        let outcome = execution.execute_prepared(prepared).unwrap();
        assert!(outcome.success());
        assert!(!outcome.stdout().is_empty());
        assert!(outcome.stderr().is_empty());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "launches the verified bundled helper and Node through the desktop AppContainer backend"]
    fn live_desktop_execution_runs_only_the_fixed_node_entry() {
        let root = TestDirectory::new("live-node-execute");
        let workspace = root.0.join("workspace");
        let target = workspace.join("target");
        let product = root.0.join("product");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir(&product).unwrap();
        fs::write(target.join("input.txt"), b"MIO_NODE_PRODUCT_INPUT\n").unwrap();
        fs::write(
            target.join("mio-main.mjs"),
            b"import { readFile, writeFile } from 'node:fs/promises';\nconst text = await readFile('input.txt', 'utf8');\nawait writeFile('result.txt', text.replace('INPUT', 'OK'), { flag: 'wx' });\nconsole.log('MIO_NODE_PRODUCT_OK');\n",
        )
        .unwrap();

        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node_source = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let install_root = node_source
            .parent()
            .and_then(Path::parent)
            .expect("MOE_TEST_NODE_EXE must use the nodejs/node.exe product layout");

        let desktop = product.join("mio-desktop.exe");
        let helper = product.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop fixture").unwrap();
        fs::copy(&helper_source, &helper).unwrap();
        let expected_helper_hash = format!("{:x}", Sha256::digest(fs::read(&helper).unwrap()));
        let helper_product = DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected_helper_hash).unwrap(),
        );
        let toolchain_product =
            DesktopCommandToolchainProduct::locate_for_test(install_root).unwrap();
        assert_eq!(
            toolchain_product.ready_node().unwrap().path(),
            node_source.canonicalize().unwrap()
        );

        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &helper_product,
            &toolchain_product,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let prepared = execution
            .prepare_baseline(&context, PathBuf::from("target"), BaselineCommand::NodeRun)
            .unwrap();
        let outcome = execution.execute_prepared(prepared).unwrap();
        assert!(outcome.success());
        assert_eq!(outcome.stdout(), b"MIO_NODE_PRODUCT_OK\n");
        assert!(outcome.stderr().is_empty());
        assert_eq!(
            fs::read(target.join("result.txt")).unwrap(),
            b"MIO_NODE_PRODUCT_OK\n"
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "runs only the fixed package.json build, test, and typecheck scripts through the desktop AppContainer backend"]
    fn live_desktop_execution_runs_only_the_fixed_npm_scripts() {
        let root = TestDirectory::new("live-npm-build-execute");
        let workspace = root.0.join("workspace");
        let target = workspace.join("target");
        let product = root.0.join("product");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir(&product).unwrap();
        fs::write(
            target.join("package.json"),
            br#"{"name":"mio-npm-build-smoke","private":true,"version":"1.0.0","scripts":{"build":"echo MIO_NPM_BUILD_OK>build.txt","test":"echo MIO_NPM_TEST_OK>test.txt","typecheck":"echo MIO_NPM_TYPECHECK_OK>typecheck.txt"}}"#,
        )
        .unwrap();

        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node_source = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let install_root = node_source
            .parent()
            .and_then(Path::parent)
            .expect("MOE_TEST_NODE_EXE must use the nodejs/node.exe product layout");

        let desktop = product.join("mio-desktop.exe");
        let helper = product.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop fixture").unwrap();
        fs::copy(&helper_source, &helper).unwrap();
        let expected_helper_hash = format!("{:x}", Sha256::digest(fs::read(&helper).unwrap()));
        let helper_product = DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected_helper_hash).unwrap(),
        );
        let toolchain_product =
            DesktopCommandToolchainProduct::locate_for_test(install_root).unwrap();
        assert_eq!(
            toolchain_product.ready_node().unwrap().path(),
            node_source.canonicalize().unwrap()
        );
        assert!(toolchain_product.ready_npm_runtime().is_some());

        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &helper_product,
            &toolchain_product,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        for (command, output, marker) in [
            (BaselineCommand::NpmBuild, "build.txt", "MIO_NPM_BUILD_OK"),
            (BaselineCommand::NpmTest, "test.txt", "MIO_NPM_TEST_OK"),
            (
                BaselineCommand::NpmTypecheck,
                "typecheck.txt",
                "MIO_NPM_TYPECHECK_OK",
            ),
        ] {
            let prepared = execution
                .prepare_baseline(&context, PathBuf::from("target"), command)
                .unwrap();
            assert_eq!(prepared.backend().tool(), RegisteredTool::Npm);
            assert!(!prepared.backend().network_client_enabled());
            let outcome = execution.execute_prepared(prepared).unwrap();
            assert!(
                outcome.success(),
                "command={command:?} exit={} stdout={} stderr={}",
                outcome.exit_code(),
                String::from_utf8_lossy(outcome.stdout()),
                String::from_utf8_lossy(outcome.stderr())
            );
            assert_eq!(
                fs::read_to_string(target.join(output)).unwrap().trim(),
                marker
            );
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "downloads one exact npm package through the full desktop confirmation and AppContainer path"]
    fn live_desktop_execution_installs_one_confirmed_exact_npm_package() {
        let root = TestDirectory::new("live-npm-product-execute");
        let workspace = root.0.join("workspace");
        let package = workspace.join("target").join("mio-npm-product-smoke");
        let product = root.0.join("product");
        fs::create_dir_all(&package).unwrap();
        fs::create_dir(&product).unwrap();
        fs::write(
            package.join("package.json"),
            br#"{"name":"mio-npm-product-smoke","private":true,"version":"1.0.0"}"#,
        )
        .unwrap();

        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node_source = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let install_root = node_source
            .parent()
            .and_then(Path::parent)
            .expect("MOE_TEST_NODE_EXE must use the nodejs/node.exe product layout");

        let desktop = product.join("mio-desktop.exe");
        let helper = product.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop fixture").unwrap();
        fs::copy(&helper_source, &helper).unwrap();
        let expected_helper_hash = format!("{:x}", Sha256::digest(fs::read(&helper).unwrap()));
        let helper_product = DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected_helper_hash).unwrap(),
        );
        let toolchain_product =
            DesktopCommandToolchainProduct::locate_for_test(install_root).unwrap();
        assert_eq!(
            toolchain_product.ready_node().unwrap().path(),
            node_source.canonicalize().unwrap()
        );
        assert!(toolchain_product.ready_npm_runtime().is_some());

        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = Arc::new(DesktopCommandExecution::new(
            workspaces,
            Arc::clone(&confirmations),
            &helper_product,
            &toolchain_product,
        ));
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let runner = Arc::clone(&execution);
        let waiter = std::thread::spawn(move || {
            let prepared = runner
                .prepare_npm_package_install(
                    &context,
                    PathBuf::from("target/mio-npm-product-smoke"),
                    "is-number".to_owned(),
                    "7.0.0".to_owned(),
                )
                .unwrap();
            runner.execute_prepared(prepared).unwrap()
        });

        let request_id = (0..2_000)
            .find_map(|_| {
                let pending = confirmations
                    .pending_request_id_for_room_for_test("room-1")
                    .unwrap();
                if pending.is_none() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                pending
            })
            .expect("package-install confirmation should become pending");
        confirmations
            .resolve_for_room_for_test(
                "room-1",
                &request_id,
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap();
        let outcome = waiter.join().unwrap();
        assert!(
            outcome.success(),
            "exit={} stdout={} stderr={}",
            outcome.exit_code(),
            String::from_utf8_lossy(outcome.stdout()),
            String::from_utf8_lossy(outcome.stderr())
        );
        assert!(package.join("package-lock.json").is_file());
        assert!(
            package
                .join("node_modules")
                .join("is-number")
                .join("package.json")
                .is_file()
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "runs mio-main.mjs once in an explicitly configured small workspace directory"]
    fn live_desktop_execution_runs_configured_fixed_node_directory() {
        let workspace = std::env::var_os("MOE_TEST_NODE_WORKSPACE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_NODE_WORKSPACE must name the selected workspace");
        let directory = std::env::var_os("MOE_TEST_NODE_DIRECTORY")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty() && !path.is_absolute())
            .expect("MOE_TEST_NODE_DIRECTORY must name one relative directory");
        let helper_source = std::env::var_os("MOE_TEST_COMMAND_HELPER")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_COMMAND_HELPER must name the built helper executable");
        let node_source = std::env::var_os("MOE_TEST_NODE_EXE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_file())
            .expect("MOE_TEST_NODE_EXE must name the verified Node executable");
        let output = workspace
            .join(&directory)
            .join("mio-node-execution-result.json");
        assert!(!output.exists(), "configured output must not already exist");

        let fixture = TestDirectory::new("configured-node-product");
        let product = fixture.0.join("product");
        fs::create_dir(&product).unwrap();
        let desktop = product.join("mio-desktop.exe");
        let helper = product.join("mio-command-helper.exe");
        fs::write(&desktop, b"desktop fixture").unwrap();
        fs::copy(&helper_source, &helper).unwrap();
        let expected_helper_hash = format!("{:x}", Sha256::digest(fs::read(&helper).unwrap()));
        let helper_product = DesktopCommandHelperProduct::Ready(
            BundledCommandHelper::locate_for_test(&desktop, &expected_helper_hash).unwrap(),
        );
        let install_root = node_source
            .parent()
            .and_then(Path::parent)
            .expect("MOE_TEST_NODE_EXE must use the nodejs/node.exe product layout");
        let toolchain_product =
            DesktopCommandToolchainProduct::locate_for_test(install_root).unwrap();

        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        confirmations
            .begin_room_session_for_test(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let execution = DesktopCommandExecution::new(
            workspaces,
            confirmations,
            &helper_product,
            &toolchain_product,
        );
        let context = turn_context(&execution, TextTurnWorkspaceAccess::ReadWrite);
        let prepared = execution
            .prepare_baseline(&context, directory, BaselineCommand::NodeRun)
            .unwrap();
        let outcome = execution.execute_prepared(prepared).unwrap();
        assert!(outcome.success());
        assert!(outcome.stderr().is_empty());
        assert!(
            String::from_utf8_lossy(outcome.stdout()).contains("M.I.O. isolated fixed Node entry")
        );
        assert!(output.is_file());
    }
}
