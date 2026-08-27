use crate::room_source::DesktopRoomSource;
#[cfg(test)]
use crate::room_source::OWNER_PARTICIPANT_ID;
use crate::time::current_rfc3339_timestamp;
use moe_core::{RoomMessage, RoomMessageDraft, RoomSource, RoomStore, RoomWriteStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub(crate) const GEMINI_SEARCH_PARTICIPANT_ID: &str = "gemini";
pub(crate) const BROWSER_BRIDGE_EXPERIMENT_ENV: &str = "MOE_EXPERIMENT_GOOGLE_AI_BRIDGE";
const BRIDGE_ADDRESS: &str = "127.0.0.1:38473";
const BRIDGE_EVENT: &str = "moe-browser-bridge-reply";
const BRIDGE_HEADER: &str = "x-moe-browser-bridge";
const BRIDGE_HEADER_VALUE: &str = "google-ai-mode-poc-v1";
const MAXIMUM_HTTP_BYTES: usize = 64_000;
const MAXIMUM_REPLY_BODY_BYTES: usize = 4_000;
const EXTENSION_RECENT_SECONDS: u64 = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserBridgeDispatch {
    dispatch_id: String,
    room_id: String,
    source_message_id: String,
    prompt: String,
    reply_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BrowserDispatchState {
    Pending(BrowserBridgeDispatch),
    Completed(BrowserBridgeDispatch, RoomMessage),
}

#[derive(Debug, Default)]
struct BrowserBridgeLedger {
    order: VecDeque<String>,
    states: HashMap<String, BrowserDispatchState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserBridgeQueueResult {
    Queued,
    Completed(RoomMessage),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBridgeReplySuccess {
    ok: bool,
    status: &'static str,
    truncated: bool,
    message: RoomMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBridgeOutboxSuccess {
    ok: bool,
    dispatch: Option<BrowserBridgeDispatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBridgeStatusSuccess {
    ok: bool,
    service: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BrowserBridgeReplyInput {
    dispatch_id: String,
    reply_token: String,
    body: String,
    source_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserBridgeError {
    InvalidRequest,
    DispatchNotFound,
    SourceUnavailable,
}

pub(crate) struct DesktopBrowserBridge {
    enabled: bool,
    ledger: Mutex<BrowserBridgeLedger>,
    listening: AtomicBool,
    last_extension_seen: AtomicU64,
}

impl DesktopBrowserBridge {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ledger: Mutex::new(BrowserBridgeLedger::default()),
            listening: AtomicBool::new(false),
            last_extension_seen: AtomicU64::new(0),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self::new(true)
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn listening(&self) -> bool {
        self.listening.load(Ordering::Relaxed)
    }

    pub(crate) fn extension_recently_seen(&self) -> bool {
        let seen = self.last_extension_seen.load(Ordering::Relaxed);
        seen > 0 && unix_seconds().saturating_sub(seen) <= EXTENSION_RECENT_SECONDS
    }

    fn touch_extension(&self) {
        self.last_extension_seen
            .store(unix_seconds(), Ordering::Relaxed);
    }

    pub(crate) fn queue(
        &self,
        source_message: &RoomMessage,
        prompt: String,
    ) -> Result<BrowserBridgeQueueResult, BrowserBridgeError> {
        if prompt.trim().is_empty() || prompt.len() > 32_000 {
            return Err(BrowserBridgeError::InvalidRequest);
        }
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| BrowserBridgeError::SourceUnavailable)?;
        match ledger.states.get(&source_message.id) {
            Some(BrowserDispatchState::Pending(_)) => {
                return Ok(BrowserBridgeQueueResult::Queued);
            }
            Some(BrowserDispatchState::Completed(_, message)) => {
                return Ok(BrowserBridgeQueueResult::Completed(message.clone()));
            }
            None => {}
        }
        let dispatch = BrowserBridgeDispatch {
            dispatch_id: format!(
                "browser-gemini:{}:{}",
                source_message.room_id, source_message.id
            ),
            room_id: source_message.room_id.clone(),
            source_message_id: source_message.id.clone(),
            prompt,
            reply_token: reply_token(source_message),
        };
        ledger.order.push_back(source_message.id.clone());
        ledger.states.insert(
            source_message.id.clone(),
            BrowserDispatchState::Pending(dispatch),
        );
        Ok(BrowserBridgeQueueResult::Queued)
    }

    fn next_dispatch(&self) -> Result<Option<BrowserBridgeDispatch>, BrowserBridgeError> {
        let ledger = self
            .ledger
            .lock()
            .map_err(|_| BrowserBridgeError::SourceUnavailable)?;
        Ok(ledger.order.iter().find_map(|source_message_id| {
            match ledger.states.get(source_message_id) {
                Some(BrowserDispatchState::Pending(dispatch)) => Some(dispatch.clone()),
                _ => None,
            }
        }))
    }

    fn complete_reply(
        &self,
        source: &DesktopRoomSource,
        input: BrowserBridgeReplyInput,
    ) -> Result<BrowserBridgeReplySuccess, BrowserBridgeError> {
        if !valid_google_source(&input.source_url) {
            return Err(BrowserBridgeError::InvalidRequest);
        }
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| BrowserBridgeError::SourceUnavailable)?;
        let source_message_id = ledger
            .states
            .iter()
            .find_map(|(source_message_id, state)| match state {
                BrowserDispatchState::Pending(dispatch)
                    if dispatch.dispatch_id == input.dispatch_id
                        && dispatch.reply_token == input.reply_token =>
                {
                    Some(source_message_id.clone())
                }
                BrowserDispatchState::Completed(dispatch, _)
                    if dispatch.dispatch_id == input.dispatch_id
                        && dispatch.reply_token == input.reply_token =>
                {
                    None
                }
                _ => None,
            })
            .ok_or(BrowserBridgeError::DispatchNotFound)?;
        let dispatch = match ledger.states.get(&source_message_id) {
            Some(BrowserDispatchState::Pending(dispatch)) => dispatch.clone(),
            Some(BrowserDispatchState::Completed(_, message)) => {
                return Ok(BrowserBridgeReplySuccess {
                    ok: true,
                    status: "duplicate",
                    truncated: false,
                    message: message.clone(),
                });
            }
            _ => return Err(BrowserBridgeError::DispatchNotFound),
        };
        let (body, truncated) = bounded_reply_body(&input.body)?;
        let source_message = source
            .find_message(&dispatch.room_id, &dispatch.source_message_id)
            .map_err(|_| BrowserBridgeError::SourceUnavailable)?;
        if !source.is_human_participant(&source_message.author_id) {
            return Err(BrowserBridgeError::InvalidRequest);
        }
        let created_at =
            current_rfc3339_timestamp().ok_or(BrowserBridgeError::SourceUnavailable)?;
        let draft = RoomMessageDraft::try_new(
            browser_reply_message_id(&dispatch.source_message_id),
            dispatch.room_id.clone(),
            GEMINI_SEARCH_PARTICIPANT_ID.to_owned(),
            vec![source_message.author_id],
            body,
            created_at,
            Vec::new(),
        )
        .map_err(|_| BrowserBridgeError::InvalidRequest)?;
        let saved = source
            .append_message(draft)
            .map_err(|_| BrowserBridgeError::SourceUnavailable)?;
        let status = match saved.status() {
            RoomWriteStatus::Appended => "appended",
            RoomWriteStatus::Duplicate => "duplicate",
        };
        let message = saved.message().clone();
        ledger.states.insert(
            source_message_id.clone(),
            BrowserDispatchState::Completed(dispatch, message.clone()),
        );
        ledger.order.retain(|id| id != &source_message_id);
        Ok(BrowserBridgeReplySuccess {
            ok: true,
            status,
            truncated,
            message,
        })
    }
}

pub(crate) fn start_browser_bridge(
    source: Arc<DesktopRoomSource>,
    app: AppHandle,
    enabled: bool,
) -> Arc<DesktopBrowserBridge> {
    let bridge = Arc::new(DesktopBrowserBridge::new(enabled));
    if !enabled {
        return bridge;
    }
    let listener = match TcpListener::bind(BRIDGE_ADDRESS) {
        Ok(listener) => listener,
        Err(_) => return bridge,
    };
    bridge.listening.store(true, Ordering::Relaxed);
    let thread_bridge = bridge.clone();
    let _ = std::thread::Builder::new()
        .name("moe-browser-bridge".to_owned())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let _ = handle_connection(stream, &thread_bridge, source.as_ref(), &app);
            }
        });
    bridge
}

pub(crate) fn browser_bridge_experiment_enabled() -> bool {
    std::env::var_os(BROWSER_BRIDGE_EXPERIMENT_ENV).is_some_and(|value| value == "1")
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn handle_connection(
    mut stream: TcpStream,
    bridge: &DesktopBrowserBridge,
    source: &DesktopRoomSource,
    app: &AppHandle,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(_) => return write_error(&mut stream, 400, "invalidRequest", None),
    };
    let origin = request.headers.get("origin").map(String::as_str);
    if request.method == "OPTIONS" {
        return if valid_extension_origin(origin) {
            write_empty(&mut stream, 204, origin)
        } else {
            write_error(&mut stream, 403, "extensionOriginRequired", None)
        };
    }
    if !valid_bridge_request_origin(origin) {
        return write_error(&mut stream, 403, "extensionOriginRequired", None);
    }
    if request.headers.get(BRIDGE_HEADER).map(String::as_str) != Some(BRIDGE_HEADER_VALUE) {
        return write_error(&mut stream, 403, "bridgeHeaderRequired", origin);
    }
    bridge.touch_extension();
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/status") => write_json(
            &mut stream,
            200,
            &BrowserBridgeStatusSuccess {
                ok: true,
                service: "moe-google-ai-browser-bridge",
            },
            origin,
        ),
        ("GET", "/v1/outbox/next") => match bridge.next_dispatch() {
            Ok(dispatch) => write_json(
                &mut stream,
                200,
                &BrowserBridgeOutboxSuccess { ok: true, dispatch },
                origin,
            ),
            Err(_) => write_error(&mut stream, 503, "bridgeUnavailable", origin),
        },
        ("POST", "/v1/replies") => {
            let input = match serde_json::from_slice::<BrowserBridgeReplyInput>(&request.body) {
                Ok(input) => input,
                Err(_) => return write_error(&mut stream, 400, "invalidReply", origin),
            };
            match bridge.complete_reply(source, input) {
                Ok(success) => {
                    if success.status == "appended" {
                        let _ = app.emit(BRIDGE_EVENT, success.message.clone());
                    }
                    write_json(&mut stream, 200, &success, origin)
                }
                Err(BrowserBridgeError::DispatchNotFound) => {
                    write_error(&mut stream, 409, "dispatchNotFound", origin)
                }
                Err(BrowserBridgeError::InvalidRequest) => {
                    write_error(&mut stream, 400, "invalidReply", origin)
                }
                Err(BrowserBridgeError::SourceUnavailable) => {
                    write_error(&mut stream, 503, "roomUnavailable", origin)
                }
            }
        }
        _ => write_error(&mut stream, 404, "notFound", origin),
    }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, ()> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4_096];
    let header_end = loop {
        if buffer.len() > MAXIMUM_HTTP_BYTES {
            return Err(());
        }
        let read = stream.read(&mut chunk).map_err(|_| ())?;
        if read == 0 {
            return Err(());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
    };
    let header = std::str::from_utf8(&buffer[..header_end]).map_err(|_| ())?;
    let mut lines = header.split("\r\n");
    let mut request_line = lines.next().ok_or(())?.split_whitespace();
    let method = request_line.next().ok_or(())?.to_owned();
    let path = request_line.next().ok_or(())?.to_owned();
    if request_line.next() != Some("HTTP/1.1") || request_line.next().is_some() {
        return Err(());
    }
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or(())?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }
    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>().map_err(|_| ()))
        .transpose()?
        .unwrap_or(0);
    if header_end + 4 + content_length > MAXIMUM_HTTP_BYTES {
        return Err(());
    }
    while buffer.len() < header_end + 4 + content_length {
        let read = stream.read(&mut chunk).map_err(|_| ())?;
        if read == 0 {
            return Err(());
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    Ok(HttpRequest {
        method,
        path,
        headers,
        body: buffer[header_end + 4..header_end + 4 + content_length].to_vec(),
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn valid_extension_origin(origin: Option<&str>) -> bool {
    origin.is_some_and(|origin| {
        (origin.starts_with("chrome-extension://")
            || origin.starts_with("extension://")
            || origin.starts_with("moz-extension://"))
            && !origin.contains(['\r', '\n'])
    })
}

fn valid_bridge_request_origin(origin: Option<&str>) -> bool {
    origin.is_none() || valid_extension_origin(origin)
}

fn valid_google_source(source_url: &str) -> bool {
    [
        "https://www.google.com/search",
        "https://www.google.com/ai",
        "https://www.google.co.jp/search",
        "https://www.google.co.jp/ai",
    ]
    .iter()
    .any(|prefix| source_url == *prefix || source_url.starts_with(&format!("{prefix}/")))
}

fn bounded_reply_body(input: &str) -> Result<(String, bool), BrowserBridgeError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(BrowserBridgeError::InvalidRequest);
    }
    if trimmed.len() <= MAXIMUM_REPLY_BODY_BYTES {
        return Ok((trimmed.to_owned(), false));
    }
    const SUFFIX: &str = "\n\n（M.I.O.の取り込み上限により末尾を省略しました）";
    let maximum_prefix = MAXIMUM_REPLY_BODY_BYTES.saturating_sub(SUFFIX.len());
    let mut end = maximum_prefix.min(trimmed.len());
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    Ok((format!("{}{SUFFIX}", &trimmed[..end]), true))
}

pub(crate) fn browser_reply_message_id(source_message_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source_message_id.hash(&mut hasher);
    format!("browser-reply-gemini-{:016x}", hasher.finish())
}

fn reply_token(source_message: &RoomMessage) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut first = DefaultHasher::new();
    source_message.id.hash(&mut first);
    source_message.room_id.hash(&mut first);
    now.hash(&mut first);
    let mut second = DefaultHasher::new();
    first.finish().hash(&mut second);
    source_message.body.hash(&mut second);
    format!("{:016x}{:016x}", first.finish(), second.finish())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn write_json<T: Serialize>(
    stream: &mut TcpStream,
    status: u16,
    value: &T,
    origin: Option<&str>,
) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    write_response(
        stream,
        status,
        "application/json; charset=utf-8",
        &body,
        origin,
    )
}

fn write_error(
    stream: &mut TcpStream,
    status: u16,
    code: &str,
    origin: Option<&str>,
) -> std::io::Result<()> {
    write_json(
        stream,
        status,
        &serde_json::json!({ "ok": false, "code": code }),
        origin,
    )
}

fn write_empty(stream: &mut TcpStream, status: u16, origin: Option<&str>) -> std::io::Result<()> {
    write_response(stream, status, "text/plain", &[], origin)
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    origin: Option<&str>,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Service Unavailable",
    };
    let cors = origin
        .filter(|origin| valid_extension_origin(Some(origin)))
        .map(|origin| {
            format!(
                "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, X-MOE-Browser-Bridge\r\nVary: Origin\r\n"
            )
        })
        .unwrap_or_default();
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{cors}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room_source::desktop_room_source;
    use moe_core::{RoomSource, RoomStore};

    fn source_message(source: &DesktopRoomSource, id: &str) -> RoomMessage {
        source
            .append_message(
                RoomMessageDraft::try_new(
                    id.to_owned(),
                    "moe-dev-room".to_owned(),
                    OWNER_PARTICIPANT_ID.to_owned(),
                    vec![GEMINI_SEARCH_PARTICIPANT_ID.to_owned()],
                    "Gemini Searchへ質問".to_owned(),
                    "2026-08-13T12:00:00+09:00".to_owned(),
                    Vec::new(),
                )
                .unwrap(),
            )
            .unwrap()
            .message()
            .clone()
    }

    #[test]
    fn queues_once_and_persists_reply_as_gemini() {
        let source = desktop_room_source();
        let source_message = source_message(source.as_ref(), "browser-source-1");
        let bridge = DesktopBrowserBridge::for_tests();

        assert_eq!(
            bridge.queue(&source_message, "M.I.O.からの質問".to_owned()),
            Ok(BrowserBridgeQueueResult::Queued)
        );
        assert_eq!(
            bridge.queue(&source_message, "M.I.O.からの質問".to_owned()),
            Ok(BrowserBridgeQueueResult::Queued)
        );
        let dispatch = bridge.next_dispatch().unwrap().unwrap();
        let success = bridge
            .complete_reply(
                source.as_ref(),
                BrowserBridgeReplyInput {
                    dispatch_id: dispatch.dispatch_id,
                    reply_token: dispatch.reply_token,
                    body: "Google Search Geminiからの返事".to_owned(),
                    source_url: "https://www.google.com/search".to_owned(),
                },
            )
            .unwrap();

        assert_eq!(success.status, "appended");
        assert_eq!(success.message.author_id, GEMINI_SEARCH_PARTICIPANT_ID);
        assert_eq!(success.message.recipients, [OWNER_PARTICIPANT_ID]);
        assert_eq!(success.message.body, "Google Search Geminiからの返事");
        assert!(bridge.next_dispatch().unwrap().is_none());
        assert_eq!(
            source
                .find_message("moe-dev-room", &success.message.id)
                .unwrap(),
            success.message
        );
    }

    #[test]
    fn rejects_wrong_token_and_non_google_source() {
        let source = desktop_room_source();
        let source_message = source_message(source.as_ref(), "browser-source-2");
        let bridge = DesktopBrowserBridge::for_tests();
        bridge
            .queue(&source_message, "M.I.O.からの質問".to_owned())
            .unwrap();
        let dispatch = bridge.next_dispatch().unwrap().unwrap();

        assert_eq!(
            bridge.complete_reply(
                source.as_ref(),
                BrowserBridgeReplyInput {
                    dispatch_id: dispatch.dispatch_id.clone(),
                    reply_token: "wrong".to_owned(),
                    body: "返事".to_owned(),
                    source_url: "https://www.google.com/search".to_owned(),
                },
            ),
            Err(BrowserBridgeError::DispatchNotFound)
        );
        assert_eq!(
            bridge.complete_reply(
                source.as_ref(),
                BrowserBridgeReplyInput {
                    dispatch_id: dispatch.dispatch_id,
                    reply_token: dispatch.reply_token,
                    body: "返事".to_owned(),
                    source_url: "https://example.com/".to_owned(),
                },
            ),
            Err(BrowserBridgeError::InvalidRequest)
        );
    }

    #[test]
    fn bounds_long_utf8_replies_without_cutting_a_character() {
        let input = "あ".repeat(2_000);
        let (body, truncated) = bounded_reply_body(&input).unwrap();

        assert!(truncated);
        assert!(body.len() <= MAXIMUM_REPLY_BODY_BYTES);
        assert!(body.ends_with("（M.I.O.の取り込み上限により末尾を省略しました）"));
    }

    #[test]
    fn accepts_only_extension_origins() {
        assert!(valid_extension_origin(Some(
            "chrome-extension://abcdefghijklmnop"
        )));
        assert!(!valid_extension_origin(Some("https://www.google.com")));
        assert!(!valid_extension_origin(None));
        assert!(valid_bridge_request_origin(None));
        assert!(valid_bridge_request_origin(Some(
            "chrome-extension://abcdefghijklmnop"
        )));
        assert!(!valid_bridge_request_origin(Some("https://www.google.com")));
    }

    #[test]
    fn experiment_is_disabled_unless_explicitly_enabled() {
        assert!(!DesktopBrowserBridge::new(false).enabled());
        assert!(DesktopBrowserBridge::new(true).enabled());
    }
}
