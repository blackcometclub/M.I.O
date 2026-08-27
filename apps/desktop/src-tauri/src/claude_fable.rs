use moe_adapter_sdk::{
    AdapterMetadata, TextTurnAdapter, TextTurnContinuity, TextTurnError, TextTurnRequest,
    TextTurnResponse,
};
use moe_protocol::{AdapterCapability, AdapterDescriptor};
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const MODEL: &str = "claude-fable-5";
const TURN_TIMEOUT: Duration = Duration::from_secs(210);
const MAXIMUM_STDOUT_BYTES: usize = 65_536;
const MAXIMUM_STDERR_BYTES: usize = 16_384;
const MAXIMUM_RESPONSE_CHARS: usize = 800;

#[derive(Debug, Clone)]
struct ClaudeLauncher {
    program: PathBuf,
}

impl ClaudeLauncher {
    fn product() -> Self {
        if let Some(path) = env::var_os("MOE_CLAUDE_BIN").filter(|value| !value.is_empty()) {
            return Self {
                program: PathBuf::from(path),
            };
        }
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            let installed = PathBuf::from(user_profile)
                .join(".local")
                .join("bin")
                .join("claude.exe");
            if installed.is_file() {
                return Self { program: installed };
            }
        }
        Self {
            program: PathBuf::from("claude"),
        }
    }

    fn available(&self) -> bool {
        if self.program.components().count() > 1 {
            return self.program.is_file();
        }
        let Some(path) = env::var_os("PATH") else {
            return false;
        };
        env::split_paths(&path).any(|directory| {
            ["claude", "claude.exe", "claude.cmd"]
                .into_iter()
                .any(|candidate| directory.join(candidate).is_file())
        })
    }
}

pub(crate) struct ClaudeFableAdapter {
    descriptor: AdapterDescriptor,
    launcher: ClaudeLauncher,
    runtime_root: PathBuf,
    live_response_seen: AtomicBool,
}

impl ClaudeFableAdapter {
    pub(crate) fn product(app_data_dir: &Path) -> Self {
        let runtime_root = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| root.join("M.O.E").join("ClaudeFableChat"))
            .unwrap_or_else(|| app_data_dir.join("ClaudeFableChat"));
        Self {
            descriptor: AdapterDescriptor {
                id: "claude-code-fable-chat".to_owned(),
                display_name: "Claude Code Fable Chat".to_owned(),
                capabilities: vec![AdapterCapability::TextInput],
            },
            launcher: ClaudeLauncher::product(),
            runtime_root,
            live_response_seen: AtomicBool::new(false),
        }
    }

    pub(crate) fn installed(&self) -> bool {
        self.launcher.available()
    }

    pub(crate) fn live_response_seen(&self) -> bool {
        self.live_response_seen.load(Ordering::Acquire)
    }

    fn run(&self, request: &TextTurnRequest) -> Result<TextTurnResponse, TextTurnError> {
        if request.workspace().is_some() || !self.installed() {
            return Err(TextTurnError::Unavailable);
        }
        fs::create_dir_all(&self.runtime_root).map_err(|_| TextTurnError::Unavailable)?;
        let runtime_root = self
            .runtime_root
            .canonicalize()
            .map_err(|_| TextTurnError::Unavailable)?;
        let mut command = Command::new(&self.launcher.program);
        command
            .args(claude_args(request)?)
            .current_dir(runtime_root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }

        let output = run_bounded(command, TURN_TIMEOUT)?;
        if !output.status.success() || output.stdout.exceeded {
            return Err(TextTurnError::Rejected);
        }
        let response = parse_response(&output.stdout.bytes)?;
        self.live_response_seen.store(true, Ordering::Release);
        Ok(response)
    }
}

impl AdapterMetadata for ClaudeFableAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }
}

impl TextTurnAdapter for ClaudeFableAdapter {
    fn run_text_turn(&self, request: &TextTurnRequest) -> Result<TextTurnResponse, TextTurnError> {
        self.run(request)
    }
}

fn claude_args(request: &TextTurnRequest) -> Result<Vec<OsString>, TextTurnError> {
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(request.prompt()),
        OsString::from("--output-format"),
        OsString::from("json"),
        OsString::from("--model"),
        OsString::from(MODEL),
        OsString::from("--tools"),
        OsString::new(),
        OsString::from("--disable-slash-commands"),
        OsString::from("--no-chrome"),
        OsString::from("--safe-mode"),
        OsString::from("--permission-mode"),
        OsString::from("dontAsk"),
    ];
    match request.continuity() {
        Some(TextTurnContinuity::Resume { session_id }) => {
            if !valid_session_id(session_id) {
                return Err(TextTurnError::InvalidResponse);
            }
            args.push(OsString::from("--resume"));
            args.push(OsString::from(session_id));
        }
        Some(TextTurnContinuity::StartPersistent) | None => {}
    }
    Ok(args)
}

fn valid_session_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

struct BoundedBytes {
    bytes: Vec<u8>,
    exceeded: bool,
}

struct ProcessOutput {
    status: ExitStatus,
    stdout: BoundedBytes,
}

fn run_bounded(mut command: Command, timeout: Duration) -> Result<ProcessOutput, TextTurnError> {
    let mut child = command.spawn().map_err(|_| TextTurnError::Unavailable)?;
    let stdout = child.stdout.take().ok_or(TextTurnError::Unavailable)?;
    let stderr = child.stderr.take().ok_or(TextTurnError::Unavailable)?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, MAXIMUM_STDOUT_BYTES));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAXIMUM_STDERR_BYTES));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| TextTurnError::Unavailable)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(TextTurnError::TimedOut);
        }
        thread::sleep(Duration::from_millis(25));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| TextTurnError::Unavailable)?;
    let _stderr = stderr_reader
        .join()
        .map_err(|_| TextTurnError::Unavailable)?;
    Ok(ProcessOutput { status, stdout })
}

fn read_bounded(mut reader: impl Read, limit: usize) -> BoundedBytes {
    let mut bytes = Vec::with_capacity(limit.min(8_192));
    let mut exceeded = false;
    let mut chunk = [0_u8; 4_096];
    while let Ok(read) = reader.read(&mut chunk) {
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..read.min(remaining)]);
        exceeded |= read > remaining;
    }
    BoundedBytes { bytes, exceeded }
}

fn parse_response(bytes: &[u8]) -> Result<TextTurnResponse, TextTurnError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| TextTurnError::InvalidResponse)?;
    if value.get("type").and_then(Value::as_str) != Some("result")
        || value.get("is_error").and_then(Value::as_bool) != Some(false)
    {
        return Err(TextTurnError::Rejected);
    }
    let text = value
        .get("result")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty() && text.chars().count() <= MAXIMUM_RESPONSE_CHARS)
        .ok_or(TextTurnError::InvalidResponse)?;
    let session_id = value
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|session_id| valid_session_id(session_id))
        .ok_or(TextTurnError::InvalidResponse)?;
    Ok(TextTurnResponse::new(text.to_owned()).with_session_id(session_id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tool_free_fable_arguments_and_resume() {
        let start = claude_args(
            &TextTurnRequest::new("dispatch-1".to_owned(), "hello".to_owned())
                .with_continuity(TextTurnContinuity::StartPersistent),
        )
        .unwrap();
        assert!(
            start
                .windows(2)
                .any(|args| args[0] == "--model" && args[1] == MODEL)
        );
        assert!(
            start
                .windows(2)
                .any(|args| args[0] == "--tools" && args[1].is_empty())
        );
        assert!(start.iter().any(|arg| arg == "--safe-mode"));
        assert!(!start.iter().any(|arg| arg == "--resume"));

        let resumed = claude_args(
            &TextTurnRequest::new("dispatch-2".to_owned(), "again".to_owned()).with_continuity(
                TextTurnContinuity::resume("d1e9e4f2-96cc-424d-a60e-fb18035ba6f4".to_owned()),
            ),
        )
        .unwrap();
        assert!(resumed.windows(2).any(|args| {
            args[0] == "--resume" && args[1] == "d1e9e4f2-96cc-424d-a60e-fb18035ba6f4"
        }));
    }

    #[test]
    fn parses_only_successful_bounded_json_with_a_session() {
        let response = parse_response(
            br#"{"type":"result","is_error":false,"result":"Fable response\n","session_id":"d1e9e4f2-96cc-424d-a60e-fb18035ba6f4"}"#,
        )
        .unwrap();
        assert_eq!(response.text(), "Fable response");
        assert_eq!(
            response.session_id(),
            Some("d1e9e4f2-96cc-424d-a60e-fb18035ba6f4")
        );
        assert!(parse_response(
            br#"{"type":"result","is_error":true,"result":"expired","session_id":"d1e9e4f2-96cc-424d-a60e-fb18035ba6f4"}"#
        )
        .is_err());
    }
}
