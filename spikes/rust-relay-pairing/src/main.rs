#![forbid(unsafe_op_in_unsafe_fn)]

use moe_credential_store::{
    CredentialStore, PlatformCredentialStore, RelayCredentialId, SecretBytes,
};
use moe_desktop_lib::relay_runtime::{
    DesktopRelayCancellation, DesktopRelayConnectionTaskFactory, DesktopRelayRuntimeEventKind,
    desktop_relay_orchestrator,
};
use moe_relay_client::{
    PairingCode, PairingResponse, RelayAccountId, RelayClientError, RelayClientService,
    RelayConnectionErrorCode, RelayConnectionManager, RelayConnectionPhase, RelayConnectionRequest,
    RelayDeviceId, RelayPairingRequest, RelayPairingTransport, RelayPairingTransportError,
    RelayPairingTransportErrorKind, RelayRuntimePhase, RelayServiceError, RelayTransport,
    RelayTransportError, RelayTransportErrorKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    env,
    error::Error,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const BASE_URL_ENV: &str = "MOE_RELAY_PAIRING_BASE_URL";
const ACCOUNT_ENV: &str = "MOE_RELAY_PAIRING_ACCOUNT";
const DEVICE_ENV: &str = "MOE_RELAY_PAIRING_DEVICE";
const CODE_ENV: &str = "MOE_RELAY_PAIRING_CODE";
const CLEANUP_ONLY_ENV: &str = "MOE_RELAY_PAIRING_CLEANUP_ONLY";
const CONNECTION_INTEGRATION_ENV: &str = "MOE_RELAY_CONNECTION_INTEGRATION";
const MAXIMUM_HTTP_RESPONSE_BYTES: usize = 2_048;
const MAXIMUM_STREAM_HEADER_BYTES: usize = 8 * 1_024;
const MAXIMUM_STREAM_FRAME_BYTES: usize = 8 * 1_024;

struct CleanupCredential(RelayCredentialId);

impl Drop for CleanupCredential {
    fn drop(&mut self) {
        let _ = PlatformCredentialStore.delete(&self.0);
    }
}

#[derive(Clone, Copy)]
struct LoopbackEndpoint {
    port: u16,
}

impl LoopbackEndpoint {
    fn new(value: &str) -> Result<Self, RelayPairingTransportError> {
        let port = value
            .strip_prefix("http://127.0.0.1:")
            .filter(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|suffix| suffix.parse::<u16>().ok())
            .filter(|port| *port != 0)
            .ok_or_else(protocol_error)?;
        Ok(Self { port })
    }
}

struct LoopbackHttpPairingTransport {
    endpoint: LoopbackEndpoint,
    timeout: Duration,
}

#[derive(Clone, Copy)]
struct LoopbackDesktopTransport {
    endpoint: LoopbackEndpoint,
    timeout: Duration,
}

struct ChunkedHttpResponse {
    reader: BufReader<TcpStream>,
    decoded: Vec<u8>,
    finished: bool,
}

struct LoopbackDesktopConnection {
    writer: TcpStream,
    response: ChunkedHttpResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairRequestBody<'a> {
    device_id: &'a str,
    pairing_code: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PairSuccessBody<'a> {
    ok: bool,
    #[serde(borrow)]
    device_id: &'a str,
    #[serde(borrow)]
    device_credential: &'a str,
}

#[derive(Deserialize)]
struct PairFailureBody<'a> {
    ok: bool,
    #[serde(borrow)]
    code: &'a str,
}

fn protocol_error() -> RelayPairingTransportError {
    RelayPairingTransportError::new(RelayPairingTransportErrorKind::Protocol)
}

fn unavailable_error() -> RelayPairingTransportError {
    RelayPairingTransportError::new(RelayPairingTransportErrorKind::Unavailable)
}

fn connection_protocol_error() -> RelayTransportError {
    RelayTransportError::new(RelayTransportErrorKind::Protocol)
}

fn connection_unavailable_error() -> RelayTransportError {
    RelayTransportError::new(RelayTransportErrorKind::Unavailable)
}

fn write_http_chunk(stream: &mut TcpStream, value: &[u8]) -> std::io::Result<()> {
    write!(stream, "{:X}\r\n", value.len())?;
    stream.write_all(value)?;
    stream.write_all(b"\r\n")?;
    stream.flush()
}

fn finish_http_chunks(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.write_all(b"0\r\n\r\n")?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)
}

fn read_bounded_line(
    reader: &mut BufReader<TcpStream>,
    maximum: usize,
) -> Result<Vec<u8>, RelayTransportError> {
    let mut line = Vec::new();
    reader
        .read_until(b'\n', &mut line)
        .map_err(|_| connection_unavailable_error())?;
    if line.is_empty() {
        return Err(connection_unavailable_error());
    }
    if line.len() > maximum || !line.ends_with(b"\n") {
        return Err(connection_protocol_error());
    }
    Ok(line)
}

impl ChunkedHttpResponse {
    fn open(stream: TcpStream) -> Result<Self, RelayTransportError> {
        let mut reader = BufReader::new(stream);
        let status_line = read_bounded_line(&mut reader, 256)?;
        let status = std::str::from_utf8(&status_line)
            .ok()
            .and_then(|line| line.split_ascii_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(connection_protocol_error)?;

        let mut header_bytes = status_line.len();
        let mut chunked = false;
        loop {
            let line = read_bounded_line(&mut reader, MAXIMUM_STREAM_HEADER_BYTES)?;
            header_bytes += line.len();
            if header_bytes > MAXIMUM_STREAM_HEADER_BYTES {
                return Err(connection_protocol_error());
            }
            if line == b"\r\n" || line == b"\n" {
                break;
            }
            if let Ok(line) = std::str::from_utf8(&line)
                && let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
            {
                chunked = true;
            }
        }

        if status != 200 {
            return Err(RelayTransportError::new(if status == 401 {
                RelayTransportErrorKind::Rejected
            } else {
                RelayTransportErrorKind::Protocol
            }));
        }
        if !chunked {
            return Err(connection_protocol_error());
        }

        Ok(Self {
            reader,
            decoded: Vec::new(),
            finished: false,
        })
    }

    fn read_frame(&mut self) -> Result<Value, RelayTransportError> {
        loop {
            if let Some(newline) = self.decoded.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.decoded.drain(..=newline).collect();
                return serde_json::from_slice(&line).map_err(|_| connection_protocol_error());
            }
            if self.finished {
                return Err(connection_protocol_error());
            }

            let size_line = read_bounded_line(&mut self.reader, 64)?;
            let size_text = std::str::from_utf8(&size_line)
                .map_err(|_| connection_protocol_error())?
                .trim();
            let size = usize::from_str_radix(
                size_text
                    .split_once(';')
                    .map_or(size_text, |(size, _)| size),
                16,
            )
            .map_err(|_| connection_protocol_error())?;
            if size == 0 {
                loop {
                    let trailer = read_bounded_line(&mut self.reader, 1_024)?;
                    if trailer == b"\r\n" || trailer == b"\n" {
                        break;
                    }
                }
                self.finished = true;
                continue;
            }
            if size > MAXIMUM_STREAM_FRAME_BYTES
                || self.decoded.len() + size > MAXIMUM_STREAM_FRAME_BYTES
            {
                return Err(connection_protocol_error());
            }

            let start = self.decoded.len();
            self.decoded.resize(start + size, 0);
            self.reader
                .read_exact(&mut self.decoded[start..])
                .map_err(|_| connection_unavailable_error())?;
            let mut terminator = [0_u8; 2];
            self.reader
                .read_exact(&mut terminator)
                .map_err(|_| connection_unavailable_error())?;
            if terminator != *b"\r\n" {
                return Err(connection_protocol_error());
            }
        }
    }
}

impl RelayTransport for LoopbackDesktopTransport {
    type Connection = LoopbackDesktopConnection;

    fn connect(
        &self,
        request: RelayConnectionRequest<'_>,
    ) -> Result<Self::Connection, RelayTransportError> {
        let mut writer = TcpStream::connect(("127.0.0.1", self.endpoint.port))
            .map_err(|_| connection_unavailable_error())?;
        writer
            .set_read_timeout(Some(self.timeout))
            .and_then(|_| writer.set_write_timeout(Some(self.timeout)))
            .map_err(|_| connection_unavailable_error())?;
        let reader = writer
            .try_clone()
            .map_err(|_| connection_unavailable_error())?;

        writer
            .write_all(b"POST /desktop-link HTTP/1.1\r\nHost: 127.0.0.1:")
            .and_then(|_| write!(writer, "{}", self.endpoint.port))
            .and_then(|_| writer.write_all(b"\r\nAuthorization: Bearer "))
            .and_then(|_| writer.write_all(request.device_credential()))
            .and_then(|_| {
                writer.write_all(
                    b"\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                )
            })
            .map_err(|_| connection_unavailable_error())?;

        let mut hello = serde_json::to_vec(&json!({
            "type": "hello",
            "deviceId": request.device_id().as_str(),
            "protocolVersion": "0.1.0",
            "capabilities": ["moe_read_room"]
        }))
        .map_err(|_| connection_protocol_error())?;
        hello.push(b'\n');
        write_http_chunk(&mut writer, &hello).map_err(|_| connection_unavailable_error())?;

        let mut response = ChunkedHttpResponse::open(reader)?;
        let acknowledgement = response.read_frame()?;
        if acknowledgement.get("type").and_then(Value::as_str) != Some("hello_ack")
            || acknowledgement
                .get("connectionId")
                .and_then(Value::as_str)
                .is_none()
        {
            return Err(connection_protocol_error());
        }

        Ok(LoopbackDesktopConnection { writer, response })
    }
}

impl LoopbackDesktopConnection {
    fn register_cancellation(
        &self,
        cancellation: &DesktopRelayCancellation,
    ) -> Result<(), RelayTransportError> {
        let stream = self
            .writer
            .try_clone()
            .map_err(|_| connection_unavailable_error())?;
        cancellation.on_cancel(move || {
            let _ = stream.shutdown(Shutdown::Both);
        });
        Ok(())
    }

    fn serve_one_room_request(&mut self) -> Result<(), RelayTransportError> {
        let request = self.response.read_frame()?;
        if request.get("type").and_then(Value::as_str) != Some("request")
            || request.get("method").and_then(Value::as_str) != Some("moe_read_room")
        {
            return Err(connection_protocol_error());
        }
        let request_id = request
            .get("requestId")
            .and_then(Value::as_str)
            .ok_or_else(connection_protocol_error)?;
        let room_id = request
            .get("params")
            .and_then(|params| params.get("roomId"))
            .and_then(Value::as_str)
            .unwrap_or("moe-dev-room");
        if room_id != "moe-dev-room" {
            return Err(connection_protocol_error());
        }

        let mut response = serde_json::to_vec(&json!({
            "type": "response",
            "requestId": request_id,
            "result": {
                "room": {
                    "id": room_id,
                    "title": "M.O.E. Rust connection integration",
                    "participants": ["owner", "codex"],
                    "messages": [{
                        "id": "rust-product-connection-message",
                        "roomId": room_id,
                        "authorId": "codex",
                        "recipients": ["owner"],
                        "body": "RUST_PRODUCT_CONNECTION_OK",
                        "createdAt": "2026-08-12T00:00:00.000Z",
                        "artifactIds": []
                    }]
                }
            }
        }))
        .map_err(|_| connection_protocol_error())?;
        response.push(b'\n');
        write_http_chunk(&mut self.writer, &response)
            .and_then(|_| finish_http_chunks(&mut self.writer))
            .map_err(|_| connection_unavailable_error())
    }

    fn wait_for_disconnect(&mut self) -> RelayTransportError {
        match self.response.read_frame() {
            Ok(_) => connection_protocol_error(),
            Err(error) => error,
        }
    }
}

fn connection_error_code(error: RelayTransportError) -> RelayConnectionErrorCode {
    match error.kind() {
        RelayTransportErrorKind::Rejected => RelayConnectionErrorCode::RelayRejected,
        RelayTransportErrorKind::Unavailable => RelayConnectionErrorCode::RelayUnavailable,
        RelayTransportErrorKind::Protocol => RelayConnectionErrorCode::Protocol,
        RelayTransportErrorKind::Cancelled => RelayConnectionErrorCode::Cancelled,
    }
}

fn parse_http_response(response: &[u8]) -> Result<(u16, &[u8]), RelayPairingTransportError> {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(protocol_error)?;
    let headers = std::str::from_utf8(&response[..separator]).map_err(|_| protocol_error())?;
    let mut lines = headers.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(protocol_error)?;
    if lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && !value.trim().eq_ignore_ascii_case("identity")
        })
    }) {
        return Err(protocol_error());
    }
    Ok((status, &response[separator + 4..]))
}

fn map_pairing_failure(code: &str) -> RelayPairingTransportError {
    let kind = match code {
        "pairing_code_invalid" => RelayPairingTransportErrorKind::InvalidCode,
        "pairing_code_expired" => RelayPairingTransportErrorKind::Expired,
        "pairing_code_used" => RelayPairingTransportErrorKind::Used,
        "pairing_code_locked" => RelayPairingTransportErrorKind::Locked,
        _ => RelayPairingTransportErrorKind::Protocol,
    };
    RelayPairingTransportError::new(kind)
}

impl RelayPairingTransport for LoopbackHttpPairingTransport {
    fn exchange(
        &self,
        request: RelayPairingRequest<'_>,
    ) -> Result<PairingResponse, RelayPairingTransportError> {
        let request_body = serde_json::to_vec(&PairRequestBody {
            device_id: request.device_id().as_str(),
            pairing_code: request.pairing_code(),
        })
        .map_err(|_| protocol_error())?;
        let request_body = SecretBytes::new(request_body).map_err(|_| protocol_error())?;
        let headers = format!(
            "POST /pair HTTP/1.0\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.endpoint.port,
            request_body.expose().len()
        );

        let mut stream = TcpStream::connect(("127.0.0.1", self.endpoint.port))
            .map_err(|_| unavailable_error())?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|_| unavailable_error())?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|_| unavailable_error())?;
        stream
            .write_all(headers.as_bytes())
            .and_then(|_| stream.write_all(request_body.expose()))
            .and_then(|_| stream.flush())
            .and_then(|_| stream.shutdown(Shutdown::Write))
            .map_err(|_| unavailable_error())?;

        let mut response = Vec::new();
        stream
            .take((MAXIMUM_HTTP_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut response)
            .map_err(|_| unavailable_error())?;
        let response = SecretBytes::new(response).map_err(|_| protocol_error())?;
        if response.expose().len() > MAXIMUM_HTTP_RESPONSE_BYTES {
            return Err(protocol_error());
        }
        let (status, body) = parse_http_response(response.expose())?;

        if status == 200 {
            let payload: PairSuccessBody<'_> =
                serde_json::from_slice(body).map_err(|_| protocol_error())?;
            if !payload.ok {
                return Err(protocol_error());
            }
            let device_id = RelayDeviceId::new(payload.device_id).map_err(|_| protocol_error())?;
            return PairingResponse::new(device_id, payload.device_credential.as_bytes().to_vec())
                .map_err(|_| protocol_error());
        }

        let failure: PairFailureBody<'_> =
            serde_json::from_slice(body).map_err(|_| protocol_error())?;
        if failure.ok {
            return Err(protocol_error());
        }
        Err(map_pairing_failure(failure.code))
    }
}

fn take_environment(name: &str) -> Result<String, Box<dyn Error>> {
    let value = env::var(name)?;
    // SAFETY: this probe is single-threaded and removes only its own private input variable.
    unsafe { env::remove_var(name) };
    Ok(value)
}

fn cleanup_only() -> Result<(), Box<dyn Error>> {
    let account_id = RelayCredentialId::new(take_environment(ACCOUNT_ENV)?)?;
    let store = PlatformCredentialStore;
    let _ = store.delete(&account_id)?;
    if store.contains(&account_id)? {
        return Err("probe credential cleanup verification failed".into());
    }
    println!("CLEANUP_OK");
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let endpoint = LoopbackEndpoint::new(&take_environment(BASE_URL_ENV)?)?;
    let account_id = RelayAccountId::new(take_environment(ACCOUNT_ENV)?)?;
    let device_id = RelayDeviceId::new(take_environment(DEVICE_ENV)?)?;
    let pairing_code = PairingCode::new(take_environment(CODE_ENV)?)?;
    let credential_id = RelayCredentialId::new(account_id.as_str())?;
    let store = PlatformCredentialStore;
    let _ = store.delete(&credential_id)?;
    let _cleanup = CleanupCredential(credential_id.clone());
    if store.contains(&credential_id)? {
        return Err("probe credential existed before pairing".into());
    }

    let manager = RelayConnectionManager::new(store);
    let transport = LoopbackHttpPairingTransport {
        endpoint,
        timeout: Duration::from_secs(2),
    };
    let receipt = manager.pair(&account_id, &device_id, pairing_code, &transport)?;
    if receipt.account_id() != &account_id || receipt.device_id() != &device_id {
        return Err("pairing receipt metadata mismatch".into());
    }
    if !store.contains(&credential_id)? {
        return Err("paired credential was not stored".into());
    }

    let connection_integration = env::var(CONNECTION_INTEGRATION_ENV).as_deref() == Ok("1");
    if connection_integration {
        // SAFETY: this probe is single-threaded and removes only its own mode flag.
        unsafe { env::remove_var(CONNECTION_INTEGRATION_ENV) };
        let connection_transport = LoopbackDesktopTransport {
            endpoint,
            timeout: Duration::from_secs(5),
        };
        let service = Arc::new(RelayClientService::new(store));
        let orchestrator = desktop_relay_orchestrator();
        let factory_service = Arc::clone(&service);
        let factory_account = account_id.clone();
        let factory_device = device_id.clone();
        let task_factory: DesktopRelayConnectionTaskFactory = Arc::new(move || {
            let service = Arc::clone(&factory_service);
            let account_id = factory_account.clone();
            let device_id = factory_device.clone();
            Box::new(move |context| {
                let mut connection =
                    match service.connect(&account_id, &device_id, &connection_transport) {
                        Ok(connection) => connection,
                        Err(error) => return error.safe_error_code(),
                    };
                if let Err(error) = connection
                    .connection()
                    .register_cancellation(context.cancellation())
                {
                    return connection_error_code(error);
                }
                context.report_connected();
                match connection.connection_mut().serve_one_room_request() {
                    Ok(()) => {
                        let error = connection.connection_mut().wait_for_disconnect();
                        connection_error_code(error)
                    }
                    Err(error) => connection_error_code(error),
                }
            })
        });
        orchestrator
            .start(&account_id, task_factory)
            .map_err(|_| "relay orchestrator start failed")?;

        let deadline = Instant::now() + Duration::from_secs(15);
        let mut connected_generations = Vec::new();
        let mut unexpected_disconnect_observed = false;
        let mut retry_wait_observed = false;
        let mut retry_elapsed_observed = false;
        while Instant::now() < deadline && connected_generations.len() < 2 {
            if let Some(update) = orchestrator
                .poll_next()
                .map_err(|_| "relay orchestrator event failed")?
            {
                retry_wait_observed |= update.status().phase() == RelayRuntimePhase::RetryWaiting;
                match update.event().kind() {
                    DesktopRelayRuntimeEventKind::Connected => {
                        connected_generations.push(update.event().generation());
                        if service.status(&account_id).phase() != RelayConnectionPhase::Connected {
                            return Err("relay service did not report connected".into());
                        }
                    }
                    DesktopRelayRuntimeEventKind::UnexpectedDisconnect(
                        RelayConnectionErrorCode::RelayUnavailable,
                    ) => unexpected_disconnect_observed = true,
                    DesktopRelayRuntimeEventKind::RetryElapsed => retry_elapsed_observed = true,
                    DesktopRelayRuntimeEventKind::ConnectionFailed(_)
                    | DesktopRelayRuntimeEventKind::UnexpectedDisconnect(_) => {}
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        if connected_generations.len() != 2
            || !unexpected_disconnect_observed
            || !retry_wait_observed
            || !retry_elapsed_observed
            || connected_generations[0] == connected_generations[1]
        {
            return Err("relay orchestrator did not complete automatic reconnect".into());
        }

        let stop_started = Instant::now();
        if !orchestrator
            .stop(&account_id)
            .map_err(|_| "relay orchestrator stop failed")?
            || stop_started.elapsed() >= Duration::from_secs(1)
        {
            return Err("relay orchestrator cancellation was not immediate".into());
        }
        if service.status(&account_id).phase() != RelayConnectionPhase::Offline {
            return Err("relay service did not return offline after connection drop".into());
        }
        if orchestrator.status(&account_id).phase() != RelayRuntimePhase::Offline {
            return Err("relay orchestrator did not return offline after stop".into());
        }

        if !manager.delete_credential(&account_id)? || store.contains(&credential_id)? {
            return Err("paired credential was not removed after connection".into());
        }
        if !matches!(
            service.connect(&account_id, &device_id, &connection_transport),
            Err(RelayServiceError::Client(
                RelayClientError::CredentialNotStored
            ))
        ) {
            return Err("connection was not rejected after credential deletion".into());
        }
        let deleted_status = service.status(&account_id);
        if deleted_status.phase() != RelayConnectionPhase::Error
            || deleted_status.credential_stored()
        {
            return Err("relay service metadata did not reflect credential deletion".into());
        }

        println!(
            "{{\n  \"result\": \"PASS\",\n  \"transport\": \"loopbackHttpChunkedNdjson\",\n  \"credentialLoadedByProductManager\": true,\n  \"authorizationWrittenFromBorrowedSecret\": true,\n  \"authenticatedHello\": true,\n  \"roomRead\": \"RUST_PRODUCT_CONNECTION_OK\",\n  \"serviceReportedConnected\": true,\n  \"serviceReturnedOfflineOnDrop\": true,\n  \"automaticReconnect\": true,\n  \"retryTimerElapsed\": true,\n  \"runtimeGenerationAdvanced\": true,\n  \"cancellationInterruptedSocket\": true,\n  \"orchestratorReturnedOfflineOnStop\": true,\n  \"credentialEnteredWebView\": false,\n  \"probeCredentialRemoved\": true,\n  \"connectionRejectedAfterDelete\": true,\n  \"serviceReportedSafeErrorAfterDelete\": true,\n  \"publicNetworkUsed\": false\n}}"
        );
        return Ok(());
    }

    if !manager.delete_credential(&account_id)? || store.contains(&credential_id)? {
        return Err("paired credential was not removed".into());
    }

    println!(
        "{{\n  \"result\": \"PASS\",\n  \"transport\": \"loopbackHttp\",\n  \"pairingResponseHandledInRust\": true,\n  \"credentialEnteredWebView\": false,\n  \"credentialStoredInWindowsCredentialManager\": true,\n  \"probeCredentialRemoved\": true,\n  \"publicNetworkUsed\": false\n}}"
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    if env::var(CLEANUP_ONLY_ENV).as_deref() == Ok("1") {
        // SAFETY: this probe is single-threaded and removes only its own mode flag.
        unsafe { env::remove_var(CLEANUP_ONLY_ENV) };
        return cleanup_only();
    }
    run()
}

#[cfg(test)]
mod tests {
    use super::{LoopbackEndpoint, map_pairing_failure, parse_http_response};
    use moe_relay_client::RelayPairingTransportErrorKind;

    #[test]
    fn accepts_only_bare_ipv4_loopback_http_endpoint() {
        assert!(LoopbackEndpoint::new("http://127.0.0.1:43123").is_ok());
        for rejected in [
            "https://127.0.0.1:43123",
            "http://localhost:43123",
            "http://127.0.0.1:43123/pair",
            "http://127.0.0.1:0",
            "http://127.0.0.1:99999",
        ] {
            assert!(LoopbackEndpoint::new(rejected).is_err());
        }
    }

    #[test]
    fn parses_bounded_non_chunked_http_response() {
        let response = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{\"ok\":true}";
        let (status, body) = parse_http_response(response).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, b"{\"ok\":true}");
    }

    #[test]
    fn maps_known_pairing_state_without_server_message() {
        assert_eq!(
            map_pairing_failure("pairing_code_expired").kind(),
            RelayPairingTransportErrorKind::Expired
        );
        assert_eq!(
            map_pairing_failure("unexpected_detail").kind(),
            RelayPairingTransportErrorKind::Protocol
        );
    }
}
