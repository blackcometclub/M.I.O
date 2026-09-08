#![forbid(unsafe_code)]

//! Fail-closed invocation contract for isolated Room workspace commands.
//!
//! This crate translates a provider-neutral, already-classified baseline plan
//! into fixed tool arguments and process limits. It deliberately does not
//! resolve an executable, inherit the host environment, start a process, or
//! claim that network and filesystem isolation exist. A platform backend must
//! prove every [`IsolationRequirement`] before it may execute a prepared
//! command.

use moe_command_broker::{
    BaselineCommand, MAXIMUM_COMMAND_OUTPUT_BYTES, MAXIMUM_COMMAND_SECONDS, NpmPackageInstallPlan,
    RegisteredTool, WorkspaceCommandPlan,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationRequirement {
    WorkspaceFilesystemBoundary,
    NetworkDenied,
    NetworkClientOnly,
    CleanEnvironment,
    ChildProcessContainment,
}

const ISOLATION_REQUIREMENTS: [IsolationRequirement; 4] = [
    IsolationRequirement::WorkspaceFilesystemBoundary,
    IsolationRequirement::NetworkDenied,
    IsolationRequirement::CleanEnvironment,
    IsolationRequirement::ChildProcessContainment,
];

const PACKAGE_INSTALL_ISOLATION_REQUIREMENTS: [IsolationRequirement; 4] = [
    IsolationRequirement::WorkspaceFilesystemBoundary,
    IsolationRequirement::NetworkClientOnly,
    IsolationRequirement::CleanEnvironment,
    IsolationRequirement::ChildProcessContainment,
];

pub const STAGED_NPM_CLI_RELATIVE_PATH: &str = "mio-npm-runtime/npm/bin/npm-cli.js";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardInputPolicy {
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCommand {
    working_directory: PathBuf,
    tool: RegisteredTool,
    arguments: Vec<OsString>,
    environment: Vec<(OsString, OsString)>,
    inherit_environment: bool,
    standard_input: StandardInputPolicy,
    maximum_output_bytes: usize,
    maximum_seconds: u64,
    requirements: &'static [IsolationRequirement],
}

impl PreparedCommand {
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn tool(&self) -> RegisteredTool {
        self.tool
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn environment(&self) -> &[(OsString, OsString)] {
        &self.environment
    }

    pub fn inherit_environment(&self) -> bool {
        self.inherit_environment
    }

    pub fn standard_input(&self) -> StandardInputPolicy {
        self.standard_input
    }

    pub fn maximum_output_bytes(&self) -> usize {
        self.maximum_output_bytes
    }

    pub fn maximum_seconds(&self) -> u64 {
        self.maximum_seconds
    }

    pub fn isolation_requirements(&self) -> &'static [IsolationRequirement] {
        self.requirements
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerContractError {
    InvalidPlan,
}

pub fn prepare_command(
    plan: &WorkspaceCommandPlan,
) -> Result<PreparedCommand, RunnerContractError> {
    if plan.tool() != plan.command().tool()
        || plan.network_enabled()
        || plan.accepts_shell_command_string()
        || plan.maximum_output_bytes() != MAXIMUM_COMMAND_OUTPUT_BYTES
        || plan.maximum_seconds() != MAXIMUM_COMMAND_SECONDS
    {
        return Err(RunnerContractError::InvalidPlan);
    }
    Ok(PreparedCommand {
        working_directory: plan.working_directory().to_owned(),
        tool: plan.tool(),
        arguments: fixed_arguments(plan.command()),
        environment: isolated_environment(plan.tool()),
        inherit_environment: false,
        standard_input: StandardInputPolicy::Null,
        maximum_output_bytes: plan.maximum_output_bytes(),
        maximum_seconds: plan.maximum_seconds(),
        requirements: &ISOLATION_REQUIREMENTS,
    })
}

pub fn prepare_npm_package_install(plan: &NpmPackageInstallPlan) -> PreparedCommand {
    let package_spec = format!("{}@{}", plan.package_name(), plan.exact_version());
    PreparedCommand {
        working_directory: plan.working_directory().to_owned(),
        tool: RegisteredTool::Node,
        arguments: [
            STAGED_NPM_CLI_RELATIVE_PATH,
            "install",
            "--ignore-scripts",
            "--save-exact",
            "--package-lock=true",
            "--global=false",
            "--registry=https://registry.npmjs.org/",
            "--no-audit",
            "--no-fund",
            "--",
            package_spec.as_str(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
        environment: npm_package_install_environment(),
        inherit_environment: false,
        standard_input: StandardInputPolicy::Null,
        maximum_output_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
        maximum_seconds: MAXIMUM_COMMAND_SECONDS,
        requirements: &PACKAGE_INSTALL_ISOLATION_REQUIREMENTS,
    }
}

fn fixed_arguments(command: BaselineCommand) -> Vec<OsString> {
    let arguments: &[&str] = match command {
        BaselineCommand::GitStatus => &[
            "--no-pager",
            "-c",
            "core.fsmonitor=false",
            "status",
            "--short",
            "--branch",
            "--untracked-files=all",
        ],
        BaselineCommand::GitDiff => &["diff", "--no-ext-diff", "--no-textconv", "--"],
        BaselineCommand::GitDiffStaged => {
            &["diff", "--cached", "--no-ext-diff", "--no-textconv", "--"]
        }
        BaselineCommand::GitLog => &["log", "-10", "--oneline", "--decorate=no"],
        BaselineCommand::NodeRun => &["mio-main.mjs"],
        BaselineCommand::NpmBuild => &["run", "build"],
        BaselineCommand::NpmTest => &["run", "test"],
        BaselineCommand::NpmTypecheck => &["run", "typecheck"],
        BaselineCommand::CargoCheck => &["check", "--locked", "--offline"],
        BaselineCommand::CargoBuild => &["build", "--locked", "--offline"],
        BaselineCommand::CargoTest => &["test", "--locked", "--offline"],
        BaselineCommand::CargoClippy => &[
            "clippy",
            "--locked",
            "--offline",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        BaselineCommand::CargoFmtCheck => &["fmt", "--all", "--", "--check"],
    };
    arguments.iter().map(OsString::from).collect()
}

fn isolated_environment(tool: RegisteredTool) -> Vec<(OsString, OsString)> {
    let mut environment = vec![pair("CI", "true"), pair("NO_COLOR", "1")];
    match tool {
        RegisteredTool::Git => environment.extend([
            pair("GCM_INTERACTIVE", "Never"),
            pair("GIT_CONFIG_NOSYSTEM", "1"),
            pair("GIT_OPTIONAL_LOCKS", "0"),
            pair("GIT_PAGER", "cat"),
        ]),
        RegisteredTool::Node => environment.extend([
            pair("NODE_DISABLE_COLORS", "1"),
            pair("NODE_NO_WARNINGS", "1"),
        ]),
        RegisteredTool::Npm => environment.extend([
            pair("npm_config_audit", "false"),
            pair("npm_config_fund", "false"),
            pair("npm_config_offline", "true"),
            pair("npm_config_update_notifier", "false"),
            pair("npm_config_yes", "false"),
        ]),
        RegisteredTool::Cargo => environment.extend([
            pair("CARGO_NET_OFFLINE", "true"),
            pair("CARGO_TERM_COLOR", "never"),
        ]),
    }
    environment.sort_by(|left, right| left.0.cmp(&right.0));
    environment
}

fn npm_package_install_environment() -> Vec<(OsString, OsString)> {
    let mut environment = vec![
        pair("CI", "true"),
        pair("NO_COLOR", "1"),
        pair("NODE_DISABLE_COLORS", "1"),
        pair("NODE_NO_WARNINGS", "1"),
        pair("npm_config_audit", "false"),
        pair("npm_config_fund", "false"),
        pair("npm_config_ignore_scripts", "true"),
        pair("npm_config_global", "false"),
        pair("npm_config_package_lock", "true"),
        pair("npm_config_registry", "https://registry.npmjs.org/"),
        pair("npm_config_save_exact", "true"),
        pair("npm_config_update_notifier", "false"),
        pair("npm_config_yes", "false"),
    ];
    environment.sort_by(|left, right| left.0.cmp(&right.0));
    environment
}

fn pair(name: &'static str, value: &'static str) -> (OsString, OsString) {
    (OsString::from(name), OsString::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_command_broker::{
        CommandClassification, NpmPackageInstallClassification, NpmPackageInstallRequest,
        WorkspaceCommandAccess, WorkspaceCommandIntent, WorkspaceCommandRequest, classify_command,
        classify_npm_package_install,
    };

    fn plan(command: BaselineCommand) -> WorkspaceCommandPlan {
        let classification = classify_command(
            WorkspaceCommandRequest::new(
                PathBuf::from("packages/app"),
                WorkspaceCommandIntent::Baseline(command),
            ),
            WorkspaceCommandAccess::ReadWrite,
        );
        let CommandClassification::Baseline(plan) = classification else {
            panic!("expected a baseline command plan");
        };
        plan
    }

    fn strings(values: &[OsString]) -> Vec<&str> {
        values.iter().map(|value| value.to_str().unwrap()).collect()
    }

    fn package_install_plan() -> NpmPackageInstallPlan {
        let classification = classify_npm_package_install(
            NpmPackageInstallRequest::new(
                PathBuf::from("packages/app"),
                "@types/node".to_owned(),
                "24.3.0".to_owned(),
            ),
            WorkspaceCommandAccess::ReadWrite,
        );
        let NpmPackageInstallClassification::ActionTimeConfirmationRequired(plan) = classification
        else {
            panic!("expected a package-install plan");
        };
        plan
    }

    #[test]
    fn maps_every_baseline_command_to_fixed_arguments() {
        for (command, expected) in [
            (
                BaselineCommand::GitStatus,
                vec![
                    "--no-pager",
                    "-c",
                    "core.fsmonitor=false",
                    "status",
                    "--short",
                    "--branch",
                    "--untracked-files=all",
                ],
            ),
            (
                BaselineCommand::GitDiff,
                vec!["diff", "--no-ext-diff", "--no-textconv", "--"],
            ),
            (
                BaselineCommand::GitDiffStaged,
                vec!["diff", "--cached", "--no-ext-diff", "--no-textconv", "--"],
            ),
            (
                BaselineCommand::GitLog,
                vec!["log", "-10", "--oneline", "--decorate=no"],
            ),
            (BaselineCommand::NodeRun, vec!["mio-main.mjs"]),
            (BaselineCommand::NpmBuild, vec!["run", "build"]),
            (BaselineCommand::NpmTest, vec!["run", "test"]),
            (BaselineCommand::NpmTypecheck, vec!["run", "typecheck"]),
            (
                BaselineCommand::CargoCheck,
                vec!["check", "--locked", "--offline"],
            ),
            (
                BaselineCommand::CargoBuild,
                vec!["build", "--locked", "--offline"],
            ),
            (
                BaselineCommand::CargoTest,
                vec!["test", "--locked", "--offline"],
            ),
            (
                BaselineCommand::CargoClippy,
                vec![
                    "clippy",
                    "--locked",
                    "--offline",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            (
                BaselineCommand::CargoFmtCheck,
                vec!["fmt", "--all", "--", "--check"],
            ),
        ] {
            let prepared = prepare_command(&plan(command)).unwrap();
            assert_eq!(strings(prepared.arguments()), expected);
            assert_eq!(prepared.tool(), command.tool());
        }
    }

    #[test]
    fn requires_every_platform_boundary_without_inheriting_process_inputs() {
        for command in [
            BaselineCommand::GitStatus,
            BaselineCommand::NodeRun,
            BaselineCommand::NpmTest,
            BaselineCommand::CargoTest,
        ] {
            let prepared = prepare_command(&plan(command)).unwrap();
            assert_eq!(prepared.working_directory(), Path::new("packages/app"));
            assert!(!prepared.inherit_environment());
            assert_eq!(prepared.standard_input(), StandardInputPolicy::Null);
            assert_eq!(
                prepared.maximum_output_bytes(),
                MAXIMUM_COMMAND_OUTPUT_BYTES
            );
            assert_eq!(prepared.maximum_seconds(), MAXIMUM_COMMAND_SECONDS);
            assert_eq!(prepared.isolation_requirements(), &ISOLATION_REQUIREMENTS);
            assert!(prepared.environment().iter().all(|(name, value)| {
                name.to_str().is_some_and(|value| value.is_ascii())
                    && value.to_str().is_some_and(|value| value.is_ascii())
            }));
        }
    }

    #[test]
    fn offline_flags_do_not_claim_to_replace_platform_network_isolation() {
        let npm = prepare_command(&plan(BaselineCommand::NpmBuild)).unwrap();
        let cargo = prepare_command(&plan(BaselineCommand::CargoBuild)).unwrap();
        assert!(
            npm.environment()
                .contains(&pair("npm_config_offline", "true"))
        );
        assert!(
            cargo
                .environment()
                .contains(&pair("CARGO_NET_OFFLINE", "true"))
        );
        assert!(
            npm.isolation_requirements()
                .contains(&IsolationRequirement::NetworkDenied)
        );
        assert!(
            cargo
                .isolation_requirements()
                .contains(&IsolationRequirement::NetworkDenied)
        );
    }

    #[test]
    fn prepared_arguments_never_contain_a_shell_command_string() {
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
            let prepared = prepare_command(&plan(command)).unwrap();
            assert!(prepared.arguments().iter().all(|argument| {
                let argument = argument.to_string_lossy();
                !argument.contains(['&', '|', '>', '<', '\n', '\r', '\0'])
            }));
        }
    }

    #[test]
    fn package_install_uses_only_fixed_npm_cli_arguments_and_one_validated_spec() {
        let prepared = prepare_npm_package_install(&package_install_plan());
        assert_eq!(prepared.tool(), RegisteredTool::Node);
        assert_eq!(prepared.working_directory(), Path::new("packages/app"));
        assert_eq!(
            strings(prepared.arguments()),
            vec![
                STAGED_NPM_CLI_RELATIVE_PATH,
                "install",
                "--ignore-scripts",
                "--save-exact",
                "--package-lock=true",
                "--global=false",
                "--registry=https://registry.npmjs.org/",
                "--no-audit",
                "--no-fund",
                "--",
                "@types/node@24.3.0",
            ]
        );
        assert!(!prepared.inherit_environment());
        assert_eq!(prepared.standard_input(), StandardInputPolicy::Null);
        assert_eq!(
            prepared.isolation_requirements(),
            &PACKAGE_INSTALL_ISOLATION_REQUIREMENTS
        );
        assert!(
            prepared
                .isolation_requirements()
                .contains(&IsolationRequirement::NetworkClientOnly)
        );
        assert!(
            !prepared
                .isolation_requirements()
                .contains(&IsolationRequirement::NetworkDenied)
        );
    }

    #[test]
    fn package_install_repeats_script_and_version_guards_in_the_clean_environment() {
        let prepared = prepare_npm_package_install(&package_install_plan());
        for expected in [
            pair("npm_config_audit", "false"),
            pair("npm_config_fund", "false"),
            pair("npm_config_ignore_scripts", "true"),
            pair("npm_config_global", "false"),
            pair("npm_config_package_lock", "true"),
            pair("npm_config_registry", "https://registry.npmjs.org/"),
            pair("npm_config_save_exact", "true"),
            pair("npm_config_update_notifier", "false"),
            pair("npm_config_yes", "false"),
        ] {
            assert!(prepared.environment().contains(&expected));
        }
        assert!(
            !prepared
                .environment()
                .iter()
                .any(|(name, _)| name == "npm_config_offline")
        );
    }
}
