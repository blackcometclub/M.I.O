#![forbid(unsafe_code)]

//! Bounded host-to-helper request protocol for fixed workspace commands.
//!
//! The wire request contains no workspace root, executable path, shell text,
//! arbitrary arguments, credentials, or environment variables. The isolated
//! helper must receive the host-owned workspace and resolved executable by a
//! separate trusted launch path, then reclassify this request before use.

use moe_command_broker::{
    BaselineCommand, CommandClassification, NpmPackageInstallClassification, NpmPackageInstallPlan,
    NpmPackageInstallRequest, WorkspaceCommandAccess, WorkspaceCommandIntent, WorkspaceCommandPlan,
    WorkspaceCommandRequest, classify_command, classify_npm_package_install,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;

pub const COMMAND_HELPER_PROTOCOL_VERSION: u16 = 1;
pub const MAXIMUM_COMMAND_HELPER_REQUEST_BYTES: usize = 16 * 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandHelperProtocolError {
    InvalidPlan,
    NonUnicodeWorkingDirectory,
    RequestTooLarge,
    InvalidEncoding,
    UnsupportedVersion,
    RejectedByCommandBroker,
    InputReadFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireBaselineCommand {
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

impl From<BaselineCommand> for WireBaselineCommand {
    fn from(command: BaselineCommand) -> Self {
        match command {
            BaselineCommand::GitStatus => Self::GitStatus,
            BaselineCommand::GitDiff => Self::GitDiff,
            BaselineCommand::GitDiffStaged => Self::GitDiffStaged,
            BaselineCommand::GitLog => Self::GitLog,
            BaselineCommand::NodeRun => Self::NodeRun,
            BaselineCommand::NpmBuild => Self::NpmBuild,
            BaselineCommand::NpmTest => Self::NpmTest,
            BaselineCommand::NpmTypecheck => Self::NpmTypecheck,
            BaselineCommand::CargoCheck => Self::CargoCheck,
            BaselineCommand::CargoBuild => Self::CargoBuild,
            BaselineCommand::CargoTest => Self::CargoTest,
            BaselineCommand::CargoClippy => Self::CargoClippy,
            BaselineCommand::CargoFmtCheck => Self::CargoFmtCheck,
        }
    }
}

impl From<WireBaselineCommand> for BaselineCommand {
    fn from(command: WireBaselineCommand) -> Self {
        match command {
            WireBaselineCommand::GitStatus => Self::GitStatus,
            WireBaselineCommand::GitDiff => Self::GitDiff,
            WireBaselineCommand::GitDiffStaged => Self::GitDiffStaged,
            WireBaselineCommand::GitLog => Self::GitLog,
            WireBaselineCommand::NodeRun => Self::NodeRun,
            WireBaselineCommand::NpmBuild => Self::NpmBuild,
            WireBaselineCommand::NpmTest => Self::NpmTest,
            WireBaselineCommand::NpmTypecheck => Self::NpmTypecheck,
            WireBaselineCommand::CargoCheck => Self::CargoCheck,
            WireBaselineCommand::CargoBuild => Self::CargoBuild,
            WireBaselineCommand::CargoTest => Self::CargoTest,
            WireBaselineCommand::CargoClippy => Self::CargoClippy,
            WireBaselineCommand::CargoFmtCheck => Self::CargoFmtCheck,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireWorkspaceAccess {
    ReadOnly,
    ReadWrite,
}

impl From<WorkspaceCommandAccess> for WireWorkspaceAccess {
    fn from(access: WorkspaceCommandAccess) -> Self {
        match access {
            WorkspaceCommandAccess::ReadOnly => Self::ReadOnly,
            WorkspaceCommandAccess::ReadWrite => Self::ReadWrite,
        }
    }
}

impl From<WireWorkspaceAccess> for WorkspaceCommandAccess {
    fn from(access: WireWorkspaceAccess) -> Self {
        match access {
            WireWorkspaceAccess::ReadOnly => Self::ReadOnly,
            WireWorkspaceAccess::ReadWrite => Self::ReadWrite,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    version: u16,
    command: WireBaselineCommand,
    working_directory: String,
    access: WireWorkspaceAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireOperation {
    NpmPackageInstall,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireNpmPackageInstallRequest {
    version: u16,
    operation: WireOperation,
    working_directory: String,
    package_name: String,
    exact_version: String,
    access: WireWorkspaceAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedCommandRequest {
    Baseline(WorkspaceCommandPlan),
    NpmPackageInstall(NpmPackageInstallPlan),
}

pub fn encode_request(
    plan: &WorkspaceCommandPlan,
    access: WorkspaceCommandAccess,
) -> Result<Vec<u8>, CommandHelperProtocolError> {
    let reclassified = classify(plan.working_directory().to_owned(), plan.command(), access)?;
    if &reclassified != plan {
        return Err(CommandHelperProtocolError::InvalidPlan);
    }
    let working_directory = plan
        .working_directory()
        .to_str()
        .ok_or(CommandHelperProtocolError::NonUnicodeWorkingDirectory)?
        .to_owned();
    let request = WireRequest {
        version: COMMAND_HELPER_PROTOCOL_VERSION,
        command: plan.command().into(),
        working_directory,
        access: access.into(),
    };
    let encoded =
        serde_json::to_vec(&request).map_err(|_| CommandHelperProtocolError::InvalidEncoding)?;
    if encoded.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(CommandHelperProtocolError::RequestTooLarge);
    }
    Ok(encoded)
}

pub fn decode_request(encoded: &[u8]) -> Result<WorkspaceCommandPlan, CommandHelperProtocolError> {
    if encoded.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(CommandHelperProtocolError::RequestTooLarge);
    }
    let request: WireRequest =
        serde_json::from_slice(encoded).map_err(|_| CommandHelperProtocolError::InvalidEncoding)?;
    if request.version != COMMAND_HELPER_PROTOCOL_VERSION {
        return Err(CommandHelperProtocolError::UnsupportedVersion);
    }
    classify(
        PathBuf::from(request.working_directory),
        request.command.into(),
        request.access.into(),
    )
}

pub fn encode_npm_package_install_request(
    plan: &NpmPackageInstallPlan,
    access: WorkspaceCommandAccess,
) -> Result<Vec<u8>, CommandHelperProtocolError> {
    let reclassified = classify_npm_package_install_plan(
        plan.working_directory().to_owned(),
        plan.package_name().to_owned(),
        plan.exact_version().to_owned(),
        access,
    )?;
    if &reclassified != plan {
        return Err(CommandHelperProtocolError::InvalidPlan);
    }
    let working_directory = plan
        .working_directory()
        .to_str()
        .ok_or(CommandHelperProtocolError::NonUnicodeWorkingDirectory)?
        .to_owned();
    let request = WireNpmPackageInstallRequest {
        version: COMMAND_HELPER_PROTOCOL_VERSION,
        operation: WireOperation::NpmPackageInstall,
        working_directory,
        package_name: plan.package_name().to_owned(),
        exact_version: plan.exact_version().to_owned(),
        access: access.into(),
    };
    let encoded =
        serde_json::to_vec(&request).map_err(|_| CommandHelperProtocolError::InvalidEncoding)?;
    if encoded.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(CommandHelperProtocolError::RequestTooLarge);
    }
    Ok(encoded)
}

pub fn decode_npm_package_install_request(
    encoded: &[u8],
) -> Result<NpmPackageInstallPlan, CommandHelperProtocolError> {
    if encoded.len() > MAXIMUM_COMMAND_HELPER_REQUEST_BYTES {
        return Err(CommandHelperProtocolError::RequestTooLarge);
    }
    let request: WireNpmPackageInstallRequest =
        serde_json::from_slice(encoded).map_err(|_| CommandHelperProtocolError::InvalidEncoding)?;
    if request.version != COMMAND_HELPER_PROTOCOL_VERSION {
        return Err(CommandHelperProtocolError::UnsupportedVersion);
    }
    classify_npm_package_install_plan(
        PathBuf::from(request.working_directory),
        request.package_name,
        request.exact_version,
        request.access.into(),
    )
}

pub fn decode_command_request(
    encoded: &[u8],
) -> Result<DecodedCommandRequest, CommandHelperProtocolError> {
    match decode_request(encoded) {
        Ok(plan) => Ok(DecodedCommandRequest::Baseline(plan)),
        Err(CommandHelperProtocolError::RequestTooLarge) => {
            Err(CommandHelperProtocolError::RequestTooLarge)
        }
        Err(_) => decode_npm_package_install_request(encoded)
            .map(DecodedCommandRequest::NpmPackageInstall),
    }
}

pub fn decode_command_request_from_reader<R: Read>(
    reader: R,
) -> Result<DecodedCommandRequest, CommandHelperProtocolError> {
    let mut encoded = Vec::new();
    reader
        .take((MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut encoded)
        .map_err(|_| CommandHelperProtocolError::InputReadFailed)?;
    decode_command_request(&encoded)
}

pub fn decode_request_from_reader<R: Read>(
    reader: R,
) -> Result<WorkspaceCommandPlan, CommandHelperProtocolError> {
    let mut encoded = Vec::new();
    reader
        .take((MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut encoded)
        .map_err(|_| CommandHelperProtocolError::InputReadFailed)?;
    decode_request(&encoded)
}

fn classify(
    working_directory: PathBuf,
    command: BaselineCommand,
    access: WorkspaceCommandAccess,
) -> Result<WorkspaceCommandPlan, CommandHelperProtocolError> {
    let classification = classify_command(
        WorkspaceCommandRequest::new(working_directory, WorkspaceCommandIntent::Baseline(command)),
        access,
    );
    match classification {
        CommandClassification::Baseline(plan) => Ok(plan),
        _ => Err(CommandHelperProtocolError::RejectedByCommandBroker),
    }
}

fn classify_npm_package_install_plan(
    working_directory: PathBuf,
    package_name: String,
    exact_version: String,
    access: WorkspaceCommandAccess,
) -> Result<NpmPackageInstallPlan, CommandHelperProtocolError> {
    let classification = classify_npm_package_install(
        NpmPackageInstallRequest::new(working_directory, package_name, exact_version),
        access,
    );
    match classification {
        NpmPackageInstallClassification::ActionTimeConfirmationRequired(plan) => Ok(plan),
        NpmPackageInstallClassification::Denied(_) => {
            Err(CommandHelperProtocolError::RejectedByCommandBroker)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(command: BaselineCommand, access: WorkspaceCommandAccess) -> WorkspaceCommandPlan {
        classify(PathBuf::from("packages/app"), command, access).unwrap()
    }

    fn npm_install_plan() -> NpmPackageInstallPlan {
        classify_npm_package_install_plan(
            PathBuf::from("packages/app"),
            "@types/node".to_owned(),
            "24.3.0".to_owned(),
            WorkspaceCommandAccess::ReadWrite,
        )
        .unwrap()
    }

    #[test]
    fn round_trips_every_fixed_baseline_without_host_paths_or_shell_text() {
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
            let access = if command.writes_workspace() {
                WorkspaceCommandAccess::ReadWrite
            } else {
                WorkspaceCommandAccess::ReadOnly
            };
            let expected = plan(command, access);
            let encoded = encode_request(&expected, access).unwrap();
            let text = std::str::from_utf8(&encoded).unwrap();
            assert!(!text.contains("workspace_root"));
            assert!(!text.contains("executable"));
            assert!(!text.contains("arguments"));
            assert_eq!(decode_request(&encoded).unwrap(), expected);
        }
    }

    #[test]
    fn refuses_to_encode_a_write_plan_as_read_only() {
        let expected = plan(
            BaselineCommand::CargoBuild,
            WorkspaceCommandAccess::ReadWrite,
        );
        assert_eq!(
            encode_request(&expected, WorkspaceCommandAccess::ReadOnly),
            Err(CommandHelperProtocolError::RejectedByCommandBroker)
        );
    }

    #[test]
    fn rejects_oversized_unknown_or_shell_shaped_requests() {
        assert_eq!(
            decode_request(&vec![b'x'; MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1]),
            Err(CommandHelperProtocolError::RequestTooLarge)
        );
        for invalid in [
            br#"{"version":2,"command":"git_status","working_directory":".","access":"read_only"}"#.as_slice(),
            br#"{"version":1,"command":"git_status","working_directory":".","access":"read_only","extra":true}"#.as_slice(),
            br#"{"version":1,"command":"git status && whoami","working_directory":".","access":"read_only"}"#.as_slice(),
        ] {
            assert!(decode_request(invalid).is_err());
        }
    }

    #[test]
    fn bounded_reader_never_accepts_more_than_the_protocol_limit() {
        let oversized = vec![b'x'; MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 8 * 1_024];
        assert_eq!(
            decode_request_from_reader(std::io::Cursor::new(oversized)),
            Err(CommandHelperProtocolError::RequestTooLarge)
        );
    }

    #[test]
    fn helper_reclassification_rejects_unsafe_working_directories() {
        for directory in ["C:\\outside", "../outside", "nested/../../outside"] {
            let request = format!(
                "{{\"version\":1,\"command\":\"git_status\",\"working_directory\":{directory:?},\"access\":\"read_only\"}}"
            );
            assert_eq!(
                decode_request(request.as_bytes()),
                Err(CommandHelperProtocolError::RejectedByCommandBroker)
            );
        }
    }

    #[test]
    fn package_install_round_trips_only_the_structured_exact_request() {
        let expected = npm_install_plan();
        let encoded =
            encode_npm_package_install_request(&expected, WorkspaceCommandAccess::ReadWrite)
                .unwrap();
        let text = std::str::from_utf8(&encoded).unwrap();
        assert!(text.contains("npm_package_install"));
        assert!(text.contains("@types/node"));
        assert!(text.contains("24.3.0"));
        for forbidden in [
            "workspace_root",
            "executable",
            "arguments",
            "environment",
            "command_string",
        ] {
            assert!(!text.contains(forbidden));
        }
        assert_eq!(
            decode_npm_package_install_request(&encoded).unwrap(),
            expected
        );
        assert_eq!(
            decode_command_request(&encoded).unwrap(),
            DecodedCommandRequest::NpmPackageInstall(expected)
        );
    }

    #[test]
    fn package_install_wire_rejects_read_only_tags_urls_options_and_extra_fields() {
        for invalid in [
            br#"{"version":1,"operation":"npm_package_install","working_directory":".","package_name":"typescript","exact_version":"5.9.2","access":"read_only"}"#.as_slice(),
            br#"{"version":1,"operation":"npm_package_install","working_directory":".","package_name":"typescript","exact_version":"latest","access":"read_write"}"#.as_slice(),
            br#"{"version":1,"operation":"npm_package_install","working_directory":".","package_name":"https://example.invalid/pkg.tgz","exact_version":"5.9.2","access":"read_write"}"#.as_slice(),
            br#"{"version":1,"operation":"npm_package_install","working_directory":".","package_name":"--global","exact_version":"5.9.2","access":"read_write"}"#.as_slice(),
            br#"{"version":1,"operation":"npm_package_install","working_directory":".","package_name":"typescript","exact_version":"5.9.2","access":"read_write","arguments":["--global"]}"#.as_slice(),
        ] {
            assert!(decode_npm_package_install_request(invalid).is_err());
            assert!(decode_command_request(invalid).is_err());
        }
    }

    #[test]
    fn generic_reader_decodes_baseline_and_package_install_without_overread() {
        let baseline = plan(BaselineCommand::GitStatus, WorkspaceCommandAccess::ReadOnly);
        let baseline_encoded = encode_request(&baseline, WorkspaceCommandAccess::ReadOnly).unwrap();
        assert_eq!(
            decode_command_request_from_reader(std::io::Cursor::new(baseline_encoded)).unwrap(),
            DecodedCommandRequest::Baseline(baseline)
        );

        let package = npm_install_plan();
        let package_encoded =
            encode_npm_package_install_request(&package, WorkspaceCommandAccess::ReadWrite)
                .unwrap();
        assert_eq!(
            decode_command_request_from_reader(std::io::Cursor::new(package_encoded)).unwrap(),
            DecodedCommandRequest::NpmPackageInstall(package)
        );

        let oversized = vec![b'x'; MAXIMUM_COMMAND_HELPER_REQUEST_BYTES + 1];
        assert_eq!(
            decode_command_request_from_reader(std::io::Cursor::new(oversized)),
            Err(CommandHelperProtocolError::RequestTooLarge)
        );
    }
}
