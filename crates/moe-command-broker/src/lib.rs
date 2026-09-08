#![forbid(unsafe_code)]

//! Provider-neutral policy boundary for Room workspace commands.
//!
//! This crate classifies structured command intents. It deliberately does not
//! resolve executables, invoke a shell, start a process, display confirmation
//! UI, or persist an approval. It does provide the in-memory host-owned
//! confirmation state contract consumed by those separate host and isolated
//! runner layers.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

mod confirmation;

pub use confirmation::{
    AuthorizationLifetime, CommandAuthorization, CommandConfirmationRegistry,
    CommandConfirmationRequest, CommandConfirmationScope, ConfirmationContractError,
    ConfirmationDecision, ConfirmationDenial, ConfirmationRegisterOutcome,
    ConfirmationResolveOutcome,
};

const MAXIMUM_RELATIVE_PATH_BYTES: usize = 4_096;
const MAXIMUM_RELATIVE_PATH_COMPONENTS: usize = 256;
const MAXIMUM_NPM_PACKAGE_NAME_BYTES: usize = 214;
const MAXIMUM_NPM_EXACT_VERSION_BYTES: usize = 128;
pub const MAXIMUM_COMMAND_OUTPUT_BYTES: usize = 256 * 1_024;
pub const MAXIMUM_COMMAND_SECONDS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCommandAccess {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisteredTool {
    Git,
    Node,
    Npm,
    Cargo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineCommand {
    GitStatus,
    GitDiff,
    GitDiffStaged,
    GitLog,
    NodeRun,
    NpmBuild,
    NpmTest,
    NpmTypecheck,
    CargoCheck,
    CargoBuild,
    CargoTest,
    CargoClippy,
    CargoFmtCheck,
}

impl BaselineCommand {
    pub fn tool(self) -> RegisteredTool {
        match self {
            Self::GitStatus | Self::GitDiff | Self::GitDiffStaged | Self::GitLog => {
                RegisteredTool::Git
            }
            Self::NodeRun => RegisteredTool::Node,
            Self::NpmBuild | Self::NpmTest | Self::NpmTypecheck => RegisteredTool::Npm,
            Self::CargoCheck
            | Self::CargoBuild
            | Self::CargoTest
            | Self::CargoClippy
            | Self::CargoFmtCheck => RegisteredTool::Cargo,
        }
    }

    pub fn writes_workspace(self) -> bool {
        !matches!(
            self,
            Self::GitStatus | Self::GitDiff | Self::GitDiffStaged | Self::GitLog
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionTimeCommand {
    GitPush,
    PackageInstall,
    CargoFetch,
    UnregisteredTool,
    CredentialUse,
    AdministratorOperation,
    DestructiveOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationReason {
    ExternalMutation,
    NetworkDownload,
    UnregisteredTool,
    CredentialUse,
    PrivilegeExpansion,
    DestructiveChange,
}

impl ActionTimeCommand {
    pub fn reason(self) -> ConfirmationReason {
        match self {
            Self::GitPush => ConfirmationReason::ExternalMutation,
            Self::PackageInstall | Self::CargoFetch => ConfirmationReason::NetworkDownload,
            Self::UnregisteredTool => ConfirmationReason::UnregisteredTool,
            Self::CredentialUse => ConfirmationReason::CredentialUse,
            Self::AdministratorOperation => ConfirmationReason::PrivilegeExpansion,
            Self::DestructiveOperation => ConfirmationReason::DestructiveChange,
        }
    }

    pub fn writes_workspace(self) -> bool {
        matches!(
            self,
            Self::GitPush | Self::PackageInstall | Self::CargoFetch | Self::DestructiveOperation
        )
    }

    pub fn allows_room_session(self) -> bool {
        matches!(
            self,
            Self::GitPush | Self::PackageInstall | Self::CargoFetch
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCommandIntent {
    Baseline(BaselineCommand),
    ActionTime(ActionTimeCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandRequest {
    working_directory: PathBuf,
    intent: WorkspaceCommandIntent,
}

impl WorkspaceCommandRequest {
    pub fn new(working_directory: PathBuf, intent: WorkspaceCommandIntent) -> Self {
        Self {
            working_directory,
            intent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCommandPlan {
    working_directory: PathBuf,
    command: BaselineCommand,
    tool: RegisteredTool,
    network_enabled: bool,
    accepts_shell_command_string: bool,
    maximum_output_bytes: usize,
    maximum_seconds: u64,
}

impl WorkspaceCommandPlan {
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn command(&self) -> BaselineCommand {
        self.command
    }

    pub fn tool(&self) -> RegisteredTool {
        self.tool
    }

    pub fn network_enabled(&self) -> bool {
        self.network_enabled
    }

    pub fn accepts_shell_command_string(&self) -> bool {
        self.accepts_shell_command_string
    }

    pub fn maximum_output_bytes(&self) -> usize {
        self.maximum_output_bytes
    }

    pub fn maximum_seconds(&self) -> u64 {
        self.maximum_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandDenialReason {
    InvalidWorkingDirectory,
    InvalidPackageName,
    InvalidExactVersion,
    WorkspaceWriteRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpmPackageInstallRequest {
    working_directory: PathBuf,
    package_name: String,
    exact_version: String,
}

impl NpmPackageInstallRequest {
    pub fn new(working_directory: PathBuf, package_name: String, exact_version: String) -> Self {
        Self {
            working_directory,
            package_name,
            exact_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpmPackageInstallPlan {
    working_directory: PathBuf,
    package_name: String,
    exact_version: String,
}

impl NpmPackageInstallPlan {
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn exact_version(&self) -> &str {
        &self.exact_version
    }

    pub fn target_label(&self) -> String {
        format!(
            "{}@{} in {}",
            self.package_name,
            self.exact_version,
            self.working_directory.display()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NpmPackageInstallClassification {
    ActionTimeConfirmationRequired(NpmPackageInstallPlan),
    Denied(CommandDenialReason),
}

pub fn classify_npm_package_install(
    request: NpmPackageInstallRequest,
    access: WorkspaceCommandAccess,
) -> NpmPackageInstallClassification {
    let Ok(working_directory) = workspace_relative_directory(&request.working_directory) else {
        return NpmPackageInstallClassification::Denied(
            CommandDenialReason::InvalidWorkingDirectory,
        );
    };
    if access != WorkspaceCommandAccess::ReadWrite {
        return NpmPackageInstallClassification::Denied(
            CommandDenialReason::WorkspaceWriteRequired,
        );
    }
    if !valid_npm_package_name(&request.package_name) {
        return NpmPackageInstallClassification::Denied(CommandDenialReason::InvalidPackageName);
    }
    if !valid_exact_npm_version(&request.exact_version) {
        return NpmPackageInstallClassification::Denied(CommandDenialReason::InvalidExactVersion);
    }
    NpmPackageInstallClassification::ActionTimeConfirmationRequired(NpmPackageInstallPlan {
        working_directory,
        package_name: request.package_name,
        exact_version: request.exact_version,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandClassification {
    Baseline(WorkspaceCommandPlan),
    ActionTimeConfirmationRequired(ConfirmationReason),
    Denied(CommandDenialReason),
}

pub fn classify_command(
    request: WorkspaceCommandRequest,
    access: WorkspaceCommandAccess,
) -> CommandClassification {
    let Ok(working_directory) = workspace_relative_directory(&request.working_directory) else {
        return CommandClassification::Denied(CommandDenialReason::InvalidWorkingDirectory);
    };
    match request.intent {
        WorkspaceCommandIntent::Baseline(command) => {
            if command.writes_workspace() && access != WorkspaceCommandAccess::ReadWrite {
                return CommandClassification::Denied(CommandDenialReason::WorkspaceWriteRequired);
            }
            CommandClassification::Baseline(WorkspaceCommandPlan {
                working_directory,
                command,
                tool: command.tool(),
                network_enabled: false,
                accepts_shell_command_string: false,
                maximum_output_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
                maximum_seconds: MAXIMUM_COMMAND_SECONDS,
            })
        }
        WorkspaceCommandIntent::ActionTime(command) => {
            if command.writes_workspace() && access != WorkspaceCommandAccess::ReadWrite {
                return CommandClassification::Denied(CommandDenialReason::WorkspaceWriteRequired);
            }
            CommandClassification::ActionTimeConfirmationRequired(command.reason())
        }
    }
}

fn workspace_relative_directory(path: &Path) -> Result<PathBuf, ()> {
    if path == Path::new(".") {
        return Ok(PathBuf::from("."));
    }
    if path.as_os_str().is_empty()
        || path.as_os_str().as_encoded_bytes().len() > MAXIMUM_RELATIVE_PATH_BYTES
        || path.is_absolute()
    {
        return Err(());
    }
    let mut relative = PathBuf::new();
    let mut component_count = 0usize;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(());
        };
        if !valid_path_segment(segment) {
            return Err(());
        }
        component_count += 1;
        if component_count > MAXIMUM_RELATIVE_PATH_COMPONENTS {
            return Err(());
        }
        relative.push(segment);
    }
    if relative.as_os_str().is_empty() {
        Err(())
    } else {
        Ok(relative)
    }
}

fn valid_npm_package_name(name: &str) -> bool {
    if name.is_empty()
        || name.len() > MAXIMUM_NPM_PACKAGE_NAME_BYTES
        || !name.is_ascii()
        || name.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return false;
    }
    let mut parts = name.split('/');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if parts.next().is_some() {
        return false;
    }
    match second {
        Some(package) => {
            first
                .strip_prefix('@')
                .is_some_and(valid_npm_name_component)
                && valid_npm_name_component(package)
        }
        None => valid_npm_name_component(first),
    }
}

fn valid_npm_name_component(component: &str) -> bool {
    component
        .as_bytes()
        .first()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && component.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn valid_exact_npm_version(version: &str) -> bool {
    if version.is_empty() || version.len() > MAXIMUM_NPM_EXACT_VERSION_BYTES || !version.is_ascii()
    {
        return false;
    }
    semver::Version::parse(version)
        .is_ok_and(|parsed| parsed.to_string() == version && parsed.build.is_empty())
}

#[cfg(windows)]
fn valid_path_segment(segment: &OsStr) -> bool {
    let Some(segment) = segment.to_str() else {
        return false;
    };
    if segment.is_empty() || segment.ends_with([' ', '.']) || segment.contains([':', '\0']) {
        return false;
    }
    let device_name = segment
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(
        device_name.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) && !windows_numbered_device_name(&device_name)
}

#[cfg(windows)]
fn windows_numbered_device_name(device_name: &str) -> bool {
    let Some(number) = device_name
        .strip_prefix("COM")
        .or_else(|| device_name.strip_prefix("LPT"))
    else {
        return false;
    };
    matches!(
        number,
        "0" | "1"
            | "2"
            | "3"
            | "4"
            | "5"
            | "6"
            | "7"
            | "8"
            | "9"
            | "⁰"
            | "¹"
            | "²"
            | "³"
            | "⁴"
            | "⁵"
            | "⁶"
            | "⁷"
            | "⁸"
            | "⁹"
    )
}

#[cfg(not(windows))]
fn valid_path_segment(segment: &OsStr) -> bool {
    !segment.as_encoded_bytes().contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str, intent: WorkspaceCommandIntent) -> WorkspaceCommandRequest {
        WorkspaceCommandRequest::new(PathBuf::from(path), intent)
    }

    #[test]
    fn allows_only_structured_baseline_operations_without_shell_or_network() {
        for command in [
            BaselineCommand::GitStatus,
            BaselineCommand::GitDiff,
            BaselineCommand::GitDiffStaged,
            BaselineCommand::GitLog,
            BaselineCommand::NodeRun,
            BaselineCommand::NpmBuild,
            BaselineCommand::NpmTest,
            BaselineCommand::NpmTypecheck,
            BaselineCommand::CargoCheck,
            BaselineCommand::CargoBuild,
            BaselineCommand::CargoTest,
            BaselineCommand::CargoClippy,
            BaselineCommand::CargoFmtCheck,
        ] {
            let classification = classify_command(
                request("packages/app", WorkspaceCommandIntent::Baseline(command)),
                WorkspaceCommandAccess::ReadWrite,
            );
            let CommandClassification::Baseline(plan) = classification else {
                panic!("expected baseline plan for {command:?}");
            };
            assert_eq!(plan.working_directory(), Path::new("packages/app"));
            assert_eq!(plan.command(), command);
            assert_eq!(plan.tool(), command.tool());
            assert!(!plan.network_enabled());
            assert!(!plan.accepts_shell_command_string());
            assert_eq!(plan.maximum_output_bytes(), MAXIMUM_COMMAND_OUTPUT_BYTES);
            assert_eq!(plan.maximum_seconds(), MAXIMUM_COMMAND_SECONDS);
        }
    }

    #[test]
    fn read_only_access_allows_git_inspection_and_denies_build_output() {
        assert!(matches!(
            classify_command(
                request(
                    ".",
                    WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
                ),
                WorkspaceCommandAccess::ReadOnly,
            ),
            CommandClassification::Baseline(_)
        ));
        for command in [
            BaselineCommand::NodeRun,
            BaselineCommand::NpmBuild,
            BaselineCommand::CargoTest,
        ] {
            assert_eq!(
                classify_command(
                    request(".", WorkspaceCommandIntent::Baseline(command)),
                    WorkspaceCommandAccess::ReadOnly,
                ),
                CommandClassification::Denied(CommandDenialReason::WorkspaceWriteRequired)
            );
        }
    }

    #[test]
    fn separates_every_action_time_category_from_baseline_execution() {
        for (command, reason) in [
            (
                ActionTimeCommand::GitPush,
                ConfirmationReason::ExternalMutation,
            ),
            (
                ActionTimeCommand::PackageInstall,
                ConfirmationReason::NetworkDownload,
            ),
            (
                ActionTimeCommand::CargoFetch,
                ConfirmationReason::NetworkDownload,
            ),
            (
                ActionTimeCommand::UnregisteredTool,
                ConfirmationReason::UnregisteredTool,
            ),
            (
                ActionTimeCommand::CredentialUse,
                ConfirmationReason::CredentialUse,
            ),
            (
                ActionTimeCommand::AdministratorOperation,
                ConfirmationReason::PrivilegeExpansion,
            ),
            (
                ActionTimeCommand::DestructiveOperation,
                ConfirmationReason::DestructiveChange,
            ),
        ] {
            assert_eq!(
                classify_command(
                    request(".", WorkspaceCommandIntent::ActionTime(command)),
                    WorkspaceCommandAccess::ReadWrite,
                ),
                CommandClassification::ActionTimeConfirmationRequired(reason)
            );
        }
        for command in [
            ActionTimeCommand::GitPush,
            ActionTimeCommand::PackageInstall,
            ActionTimeCommand::CargoFetch,
            ActionTimeCommand::DestructiveOperation,
        ] {
            assert_eq!(
                classify_command(
                    request(".", WorkspaceCommandIntent::ActionTime(command)),
                    WorkspaceCommandAccess::ReadOnly,
                ),
                CommandClassification::Denied(CommandDenialReason::WorkspaceWriteRequired)
            );
        }
    }

    #[test]
    fn rejects_absolute_parent_current_and_unbounded_working_directories() {
        for path in ["", "..", "../outside", "./nested", "/outside"] {
            assert_eq!(
                classify_command(
                    request(
                        path,
                        WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
                    ),
                    WorkspaceCommandAccess::ReadOnly,
                ),
                CommandClassification::Denied(CommandDenialReason::InvalidWorkingDirectory)
            );
        }
        #[cfg(windows)]
        for path in ["C:outside", "nested.", "nested ", "NUL", "COM1"] {
            assert_eq!(
                classify_command(
                    request(
                        path,
                        WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
                    ),
                    WorkspaceCommandAccess::ReadOnly,
                ),
                CommandClassification::Denied(CommandDenialReason::InvalidWorkingDirectory)
            );
        }
        let oversized = "a".repeat(MAXIMUM_RELATIVE_PATH_BYTES + 1);
        assert_eq!(
            classify_command(
                request(
                    &oversized,
                    WorkspaceCommandIntent::Baseline(BaselineCommand::GitStatus),
                ),
                WorkspaceCommandAccess::ReadOnly,
            ),
            CommandClassification::Denied(CommandDenialReason::InvalidWorkingDirectory)
        );
    }

    #[test]
    fn package_install_accepts_only_a_named_package_at_an_exact_version() {
        for (name, version) in [
            ("typescript", "5.9.2"),
            ("@types/node", "24.3.0"),
            ("eslint-plugin-example", "1.0.0-beta.2"),
        ] {
            let classification = classify_npm_package_install(
                NpmPackageInstallRequest::new(
                    PathBuf::from("packages/app"),
                    name.to_owned(),
                    version.to_owned(),
                ),
                WorkspaceCommandAccess::ReadWrite,
            );
            let NpmPackageInstallClassification::ActionTimeConfirmationRequired(plan) =
                classification
            else {
                panic!("expected a package-install confirmation plan");
            };
            assert_eq!(plan.working_directory(), Path::new("packages/app"));
            assert_eq!(plan.package_name(), name);
            assert_eq!(plan.exact_version(), version);
            assert_eq!(
                plan.target_label(),
                format!("{name}@{version} in {}", plan.working_directory().display())
            );
        }
    }

    #[test]
    fn package_install_rejects_tags_ranges_urls_paths_options_and_embedded_versions() {
        for name in [
            "",
            "TypeScript",
            "-D",
            ".hidden",
            "@scope",
            "@scope/",
            "@scope/pkg/extra",
            "pkg@1.2.3",
            "https://example.invalid/pkg.tgz",
            "../local-package",
            "git+https://example.invalid/repo.git",
            "package name",
        ] {
            assert_eq!(
                classify_npm_package_install(
                    NpmPackageInstallRequest::new(
                        PathBuf::from("."),
                        name.to_owned(),
                        "1.2.3".to_owned(),
                    ),
                    WorkspaceCommandAccess::ReadWrite,
                ),
                NpmPackageInstallClassification::Denied(CommandDenialReason::InvalidPackageName),
                "unexpectedly accepted package name {name:?}"
            );
        }
        for version in [
            "",
            "latest",
            "next",
            "^1.2.3",
            "~1.2.3",
            ">=1.2.3",
            "1.x",
            "1.2",
            "v1.2.3",
            "01.2.3",
            "1.2.3+local",
            "https://example.invalid/pkg.tgz",
        ] {
            assert_eq!(
                classify_npm_package_install(
                    NpmPackageInstallRequest::new(
                        PathBuf::from("."),
                        "typescript".to_owned(),
                        version.to_owned(),
                    ),
                    WorkspaceCommandAccess::ReadWrite,
                ),
                NpmPackageInstallClassification::Denied(CommandDenialReason::InvalidExactVersion),
                "unexpectedly accepted version {version:?}"
            );
        }
    }

    #[test]
    fn package_install_requires_write_access_and_a_safe_relative_directory() {
        assert_eq!(
            classify_npm_package_install(
                NpmPackageInstallRequest::new(
                    PathBuf::from("packages/app"),
                    "typescript".to_owned(),
                    "5.9.2".to_owned(),
                ),
                WorkspaceCommandAccess::ReadOnly,
            ),
            NpmPackageInstallClassification::Denied(CommandDenialReason::WorkspaceWriteRequired)
        );
        for directory in ["", "..", "../outside", "./nested", "C:\\outside"] {
            assert_eq!(
                classify_npm_package_install(
                    NpmPackageInstallRequest::new(
                        PathBuf::from(directory),
                        "typescript".to_owned(),
                        "5.9.2".to_owned(),
                    ),
                    WorkspaceCommandAccess::ReadWrite,
                ),
                NpmPackageInstallClassification::Denied(
                    CommandDenialReason::InvalidWorkingDirectory
                )
            );
        }
    }
}
