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

const GROK_MODEL: &str = "grok-4.6";
const TURN_TIMEOUT: Duration = Duration::from_secs(180);
const MAXIMUM_STDOUT_BYTES: usize = 65_536;
const MAXIMUM_STDERR_BYTES: usize = 16_384;
const MAXIMUM_RESPONSE_CHARS: usize = 800;
const GROK_CHAT_ONLY_AGENT: &str = r#"---
name: moe-chat-only
description: Chat-only Grok participant for M.I.O.
prompt_mode: full
model: inherit
permission_mode: default
agents_md: false
tools: []
---

You are the Grok participant in an M.I.O. talk room.
Use the response language explicitly requested in the current question.
Otherwise, respond in the same language as the current question.
If the language is unclear, respond in Japanese.
You have no tools and must not inspect or change the local computer.
Do not browse, invoke subagents, use external memory, or claim access beyond the supplied Room record.
Keep the final answer under 800 characters.
"#;

#[derive(Debug, Clone)]
struct GrokLauncher {
    program: PathBuf,
}

impl GrokLauncher {
    fn product() -> Self {
        if let Some(path) = env::var_os("MOE_GROK_BIN").filter(|value| !value.is_empty()) {
            return Self {
                program: PathBuf::from(path),
            };
        }
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            let installed = PathBuf::from(user_profile)
                .join(".grok")
                .join("bin")
                .join("grok.exe");
            if installed.is_file() {
                return Self { program: installed };
            }
        }
        Self {
            program: PathBuf::from("grok"),
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
            ["grok", "grok.exe", "grok.cmd"]
                .into_iter()
                .any(|candidate| directory.join(candidate).is_file())
        })
    }
}

pub(crate) struct GrokCliAdapter {
    descriptor: AdapterDescriptor,
    launcher: GrokLauncher,
    runtime_root: PathBuf,
    live_response_seen: AtomicBool,
}

impl GrokCliAdapter {
    pub(crate) fn product(app_data_dir: &Path) -> Self {
        Self {
            descriptor: AdapterDescriptor {
                id: "grok-cli-chat".to_owned(),
                display_name: "Grok CLI Chat".to_owned(),
                capabilities: vec![AdapterCapability::TextInput],
            },
            launcher: GrokLauncher::product(),
            runtime_root: app_data_dir.join("grok-chat-runtime"),
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
        let profile_path = runtime_root.join("moe-chat-only-agent.md");
        ensure_profile(&profile_path)?;

        let mut command = Command::new(&self.launcher.program);
        command
            .args(grok_args(request, &profile_path)?)
            .current_dir(&runtime_root)
            .env("GROK_DISABLE_AUTOUPDATER", "1")
            .env("GROK_MEMORY", "0")
            .env("GROK_SUBAGENTS", "0")
            .env("GROK_WEB_FETCH", "0")
            .env("GROK_CURSOR_SKILLS_ENABLED", "0")
            .env("GROK_CURSOR_RULES_ENABLED", "0")
            .env("GROK_CURSOR_AGENTS_ENABLED", "0")
            .env("GROK_CURSOR_MCPS_ENABLED", "0")
            .env("GROK_CURSOR_HOOKS_ENABLED", "0")
            .env("GROK_CLAUDE_SKILLS_ENABLED", "0")
            .env("GROK_CLAUDE_RULES_ENABLED", "0")
            .env("GROK_CLAUDE_AGENTS_ENABLED", "0")
            .env("GROK_CLAUDE_MCPS_ENABLED", "0")
            .env("GROK_CLAUDE_HOOKS_ENABLED", "0")
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

impl AdapterMetadata for GrokCliAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }
}

impl TextTurnAdapter for GrokCliAdapter {
    fn run_text_turn(&self, request: &TextTurnRequest) -> Result<TextTurnResponse, TextTurnError> {
        self.run(request)
    }
}

fn ensure_profile(path: &Path) -> Result<(), TextTurnError> {
    if fs::read_to_string(path).ok().as_deref() == Some(GROK_CHAT_ONLY_AGENT) {
        return Ok(());
    }
    fs::write(path, GROK_CHAT_ONLY_AGENT).map_err(|_| TextTurnError::Unavailable)
}

fn grok_args(
    request: &TextTurnRequest,
    profile_path: &Path,
) -> Result<Vec<OsString>, TextTurnError> {
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(request.prompt()),
        OsString::from("--verbatim"),
        OsString::from("--model"),
        OsString::from(GROK_MODEL),
        OsString::from("--output-format"),
        OsString::from("json"),
        OsString::from("--agent"),
        profile_path.as_os_str().to_owned(),
        OsString::from("--no-subagents"),
        OsString::from("--disable-web-search"),
        OsString::from("--no-memory"),
        OsString::from("--max-turns"),
        OsString::from("1"),
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
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
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
    if value.get("type").and_then(Value::as_str) == Some("error") {
        return Err(TextTurnError::Rejected);
    }
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty() && text.chars().count() <= MAXIMUM_RESPONSE_CHARS)
        .ok_or(TextTurnError::InvalidResponse)?;
    if value.get("stopReason").and_then(Value::as_str) != Some("EndTurn") {
        return Err(TextTurnError::InvalidResponse);
    }
    let session_id = value
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|session_id| valid_session_id(session_id))
        .ok_or(TextTurnError::InvalidResponse)?;
    Ok(TextTurnResponse::new(text.to_owned()).with_session_id(session_id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_profile_has_no_tools_or_external_context_sources() {
        assert!(GROK_CHAT_ONLY_AGENT.contains("tools: []"));
        assert!(GROK_CHAT_ONLY_AGENT.contains("You have no tools"));
        assert!(GROK_CHAT_ONLY_AGENT.contains("Do not browse"));
        assert!(GROK_CHAT_ONLY_AGENT.contains("explicitly requested in the current question"));
        assert!(GROK_CHAT_ONLY_AGENT.contains("same language as the current question"));
        assert!(!GROK_CHAT_ONLY_AGENT.contains("directly in Japanese"));
    }

    #[test]
    fn builds_new_and_resumed_headless_turns() {
        let profile = Path::new("C:/isolated/moe-chat.md");
        let start = grok_args(
            &TextTurnRequest::new("dispatch-1".to_owned(), "hello".to_owned())
                .with_continuity(TextTurnContinuity::StartPersistent),
            profile,
        )
        .unwrap();
        assert!(!start.iter().any(|arg| arg == "--resume"));
        assert!(start.iter().any(|arg| arg == "--no-memory"));

        let resumed = grok_args(
            &TextTurnRequest::new("dispatch-2".to_owned(), "again".to_owned())
                .with_continuity(TextTurnContinuity::resume("session-1".to_owned())),
            profile,
        )
        .unwrap();
        assert!(
            resumed
                .windows(2)
                .any(|args| args[0] == "--resume" && args[1] == "session-1")
        );
    }

    #[test]
    fn parses_only_bounded_completed_json_with_a_session() {
        let response = parse_response(
            br#"{"text":"Grok response","stopReason":"EndTurn","sessionId":"session-1","requestId":"request-1"}"#,
        )
        .unwrap();
        assert_eq!(response.text(), "Grok response");
        assert_eq!(response.session_id(), Some("session-1"));

        assert!(parse_response(br#"{"type":"error","message":"no"}"#).is_err());
        assert!(
            parse_response(br#"{"text":"x","stopReason":"MaxTurns","sessionId":"s"}"#).is_err()
        );
        assert!(
            parse_response(br#"{"text":"x","stopReason":"EndTurn","sessionId":"bad session"}"#)
                .is_err()
        );
    }
}
