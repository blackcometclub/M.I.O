use crate::{
    relay_client::DesktopRelayService,
    relay_runtime::{
        DesktopRelayConnectionTaskFactory, DesktopRelayOrchestrator, DesktopRelayOrchestratorError,
    },
    room_source::DesktopRoomSource,
};
use moe_core::{RoomReadQuery, RoomReadResult, RoomSource};
use moe_credential_store::CredentialStore;
use moe_protocol::{
    RELAY_MAXIMUM_REQUESTS_PER_CONNECTION, RELAY_READ_ROOM_METHOD, RelayFrameError,
    RelayRequestFrame, RelayResponseErrorCode, RelayResponseFrame,
};
use moe_relay_client::{
    RelayAccountId, RelayClientService, RelayConnectionErrorCode, RelayDeviceId,
    RelayRuntimeStatus, RelayTransportError,
};
use moe_relay_transport::{RelayHttpsEndpoint, RelayHttpsTransport};
use std::{collections::HashSet, sync::Arc, time::Duration};

const PRODUCT_IO_TIMEOUT: Duration = Duration::from_secs(30);
const BUNDLED_RELAY_ENDPOINT: Option<&str> = option_env!("MOE_RELAY_ENDPOINT");
const BUNDLED_RELAY_ACCOUNT_ID: Option<&str> = option_env!("MOE_RELAY_ACCOUNT_ID");
const BUNDLED_RELAY_DEVICE_ID: Option<&str> = option_env!("MOE_RELAY_DEVICE_ID");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopRelayProductConfigError {
    Incomplete,
    InvalidEndpoint,
    InvalidIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopRelayProductStartError {
    Config(DesktopRelayProductConfigError),
    Transport,
    Runtime(DesktopRelayOrchestratorError),
}

#[derive(Debug)]
struct DesktopRelayProductConfig {
    account_id: RelayAccountId,
    device_id: RelayDeviceId,
    endpoint: RelayHttpsEndpoint,
}

impl DesktopRelayProductConfig {
    fn bundled() -> Result<Option<Self>, DesktopRelayProductConfigError> {
        Self::from_values(
            BUNDLED_RELAY_ENDPOINT,
            BUNDLED_RELAY_ACCOUNT_ID,
            BUNDLED_RELAY_DEVICE_ID,
        )
    }

    fn from_values(
        endpoint: Option<&str>,
        account_id: Option<&str>,
        device_id: Option<&str>,
    ) -> Result<Option<Self>, DesktopRelayProductConfigError> {
        let (endpoint, account_id, device_id) = match (endpoint, account_id, device_id) {
            (None, None, None) => return Ok(None),
            (Some(endpoint), Some(account_id), Some(device_id)) => {
                (endpoint, account_id, device_id)
            }
            _ => return Err(DesktopRelayProductConfigError::Incomplete),
        };

        let endpoint = RelayHttpsEndpoint::parse(endpoint)
            .map_err(|_| DesktopRelayProductConfigError::InvalidEndpoint)?;
        let account_id = RelayAccountId::new(account_id.to_owned())
            .map_err(|_| DesktopRelayProductConfigError::InvalidIdentity)?;
        let device_id = RelayDeviceId::new(device_id.to_owned())
            .map_err(|_| DesktopRelayProductConfigError::InvalidIdentity)?;
        Ok(Some(Self {
            account_id,
            device_id,
            endpoint,
        }))
    }
}

pub(crate) fn start_bundled_product_relay(
    service: Arc<DesktopRelayService>,
    room_source: Arc<DesktopRoomSource>,
    runtime: &DesktopRelayOrchestrator,
) -> Result<Option<RelayRuntimeStatus>, DesktopRelayProductStartError> {
    let Some(config) =
        DesktopRelayProductConfig::bundled().map_err(DesktopRelayProductStartError::Config)?
    else {
        return Ok(None);
    };
    let transport = RelayHttpsTransport::new(config.endpoint, PRODUCT_IO_TIMEOUT)
        .map_err(|_| DesktopRelayProductStartError::Transport)?;
    let task_factory = relay_https_task_factory(
        service,
        config.account_id.clone(),
        config.device_id,
        Arc::new(transport),
        room_source,
    );
    runtime
        .start(&config.account_id, task_factory)
        .map(Some)
        .map_err(DesktopRelayProductStartError::Runtime)
}

fn relay_https_task_factory<S, R>(
    service: Arc<RelayClientService<S>>,
    account_id: RelayAccountId,
    device_id: RelayDeviceId,
    transport: Arc<RelayHttpsTransport>,
    room_source: Arc<R>,
) -> DesktopRelayConnectionTaskFactory
where
    S: CredentialStore + Send + Sync + 'static,
    R: RoomSource + Send + Sync + 'static,
{
    Arc::new(move || {
        let service = Arc::clone(&service);
        let account_id = account_id.clone();
        let device_id = device_id.clone();
        let transport = Arc::clone(&transport);
        let room_source = Arc::clone(&room_source);
        Box::new(move |context| {
            let mut connection = match service.connect(&account_id, &device_id, transport.as_ref())
            {
                Ok(connection) => connection,
                Err(error) => return error.safe_error_code(),
            };
            let shutdown = connection.connection().shutdown_handle();
            context
                .cancellation()
                .on_cancel(move || shutdown.shutdown());
            context.report_connected();
            serve_relay_requests(context, connection.connection_mut(), room_source.as_ref())
        })
    })
}

fn serve_relay_requests<R: RoomSource>(
    context: &crate::relay_runtime::DesktopRelayTaskContext,
    connection: &mut moe_relay_transport::RelayHttpsConnection,
    room_source: &R,
) -> RelayConnectionErrorCode {
    let mut request_ids = HashSet::new();
    loop {
        let value = match connection.read_frame() {
            Ok(value) => value,
            Err(error) => return transport_error_code(error),
        };
        if context.cancellation().is_cancelled() {
            return RelayConnectionErrorCode::Cancelled;
        }
        let request = match RelayRequestFrame::parse(value) {
            Ok(request) => request,
            Err(_) => return RelayConnectionErrorCode::Protocol,
        };
        match register_request_id(&mut request_ids, request.request_id()) {
            RequestRegistration::Duplicate => {
                if let Some(error) = write_error_response(
                    context,
                    connection,
                    request.request_id(),
                    RelayResponseErrorCode::DuplicateRequest,
                    "The Relay request ID was already handled.",
                ) {
                    return error;
                }
                continue;
            }
            RequestRegistration::LimitReached => return RelayConnectionErrorCode::Protocol,
            RequestRegistration::Accepted => {}
        }

        let result = match route_request(&request, room_source) {
            Ok(result) => result,
            Err(error) => {
                if let Some(write_error) = write_error_response(
                    context,
                    connection,
                    request.request_id(),
                    error.code,
                    error.message,
                ) {
                    return write_error;
                }
                continue;
            }
        };
        if context.cancellation().is_cancelled() {
            return RelayConnectionErrorCode::Cancelled;
        }
        if let Err(error) =
            connection.write_frame(&RelayResponseFrame::success(request.request_id(), &result))
        {
            return transport_error_code(error);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestRegistration {
    Accepted,
    Duplicate,
    LimitReached,
}

fn register_request_id(request_ids: &mut HashSet<String>, request_id: &str) -> RequestRegistration {
    if request_ids.contains(request_id) {
        return RequestRegistration::Duplicate;
    }
    if request_ids.len() >= RELAY_MAXIMUM_REQUESTS_PER_CONNECTION {
        return RequestRegistration::LimitReached;
    }
    request_ids.insert(request_id.to_owned());
    RequestRegistration::Accepted
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequestRouteError {
    code: RelayResponseErrorCode,
    message: &'static str,
}

fn route_request<R: RoomSource>(
    request: &RelayRequestFrame,
    room_source: &R,
) -> Result<RoomReadResult, RequestRouteError> {
    if request.method() != RELAY_READ_ROOM_METHOD {
        return Err(RequestRouteError {
            code: RelayResponseErrorCode::UnsupportedMethod,
            message: "The requested Desktop method is not supported.",
        });
    }
    let params = request.read_room_params().map_err(|error| match error {
        RelayFrameError::InvalidReadRoomParams => RequestRouteError {
            code: RelayResponseErrorCode::InvalidRequest,
            message: "The Room request parameters are invalid.",
        },
        RelayFrameError::InvalidFrame | RelayFrameError::InvalidRequestId => RequestRouteError {
            code: RelayResponseErrorCode::InvalidRequest,
            message: "The Relay request is invalid.",
        },
    })?;
    let query = RoomReadQuery::try_new(
        params.room_id().to_owned(),
        params.after_message_id().map(str::to_owned),
        params.limit(),
    )
    .map_err(|_| RequestRouteError {
        code: RelayResponseErrorCode::InvalidRequest,
        message: "The Room request parameters are invalid.",
    })?;
    Ok(room_source.read_room(&query))
}

fn write_error_response(
    context: &crate::relay_runtime::DesktopRelayTaskContext,
    connection: &mut moe_relay_transport::RelayHttpsConnection,
    request_id: &str,
    code: RelayResponseErrorCode,
    message: &str,
) -> Option<RelayConnectionErrorCode> {
    if context.cancellation().is_cancelled() {
        return Some(RelayConnectionErrorCode::Cancelled);
    }
    connection
        .write_frame(&RelayResponseFrame::<RoomReadResult>::error(
            request_id, code, message,
        ))
        .err()
        .map(transport_error_code)
}

fn transport_error_code(error: RelayTransportError) -> RelayConnectionErrorCode {
    error.safe_error_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay_runtime::{DesktopRelayRuntimeEventKind, desktop_relay_orchestrator};
    use crate::room_source::desktop_room_source;
    use moe_core::{RoomMessageDraft, RoomStore};
    use moe_credential_store::{CredentialStoreError, RelayCredentialId, SecretBytes};
    use moe_relay_client::RelayRuntimePhase;
    use rcgen::{CertifiedKey, generate_simple_self_signed};
    use rustls::{
        ServerConfig, ServerConnection, StreamOwned,
        pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
    };
    use serde_json::Value;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread::{self, JoinHandle},
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    const TEST_CREDENTIAL: &[u8] = b"desktop-product-test-credential";

    #[cfg(windows)]
    struct WindowsCredentialCleanup(RelayCredentialId);

    #[cfg(windows)]
    impl Drop for WindowsCredentialCleanup {
        fn drop(&mut self) {
            use moe_credential_store::PlatformCredentialStore;

            let _ = PlatformCredentialStore.delete(&self.0);
        }
    }

    struct TestCredentialStore;

    impl CredentialStore for TestCredentialStore {
        fn store(
            &self,
            _id: &RelayCredentialId,
            _credential: &SecretBytes,
        ) -> Result<(), CredentialStoreError> {
            Ok(())
        }

        fn load(
            &self,
            _id: &RelayCredentialId,
        ) -> Result<Option<SecretBytes>, CredentialStoreError> {
            SecretBytes::new(TEST_CREDENTIAL.to_vec()).map(Some)
        }

        fn delete(&self, _id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
            Ok(true)
        }
    }

    struct ReconnectingTlsFixture {
        endpoint: RelayHttpsEndpoint,
        certificate_der: Vec<u8>,
        request_receiver: mpsc::Receiver<Vec<u8>>,
        response_receiver: mpsc::Receiver<Value>,
        worker: JoinHandle<()>,
    }

    impl ReconnectingTlsFixture {
        fn start() -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind TLS Relay fixture");
            let port = listener.local_addr().expect("fixture address").port();
            let CertifiedKey { cert, signing_key } =
                generate_simple_self_signed(vec!["localhost".to_owned()])
                    .expect("generate TLS fixture certificate");
            let certificate = cert.der().clone();
            let private_key =
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(signing_key.serialize_der()));
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let mut server_config = ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("safe TLS versions")
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], private_key)
                .expect("fixture certificate and key");
            server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
            let server_config = Arc::new(server_config);
            let (request_sender, request_receiver) = mpsc::channel();
            let (response_sender, response_receiver) = mpsc::channel();
            let worker = thread::spawn(move || {
                for generation in 0..2 {
                    let (tcp, _) = listener.accept().expect("accept TLS Relay connection");
                    tcp.set_read_timeout(Some(Duration::from_secs(5)))
                        .and_then(|_| tcp.set_write_timeout(Some(Duration::from_secs(5))))
                        .expect("fixture timeouts");
                    let connection = ServerConnection::new(Arc::clone(&server_config))
                        .expect("TLS server connection");
                    let stream = StreamOwned::new(connection, tcp);
                    let mut reader = BufReader::new(stream);
                    let mut request = Vec::new();
                    loop {
                        let mut line = Vec::new();
                        reader.read_until(b'\n', &mut line).expect("request header");
                        request.extend_from_slice(&line);
                        if line == b"\r\n" || line == b"\n" {
                            break;
                        }
                        assert!(request.len() <= 8 * 1024);
                    }
                    let mut chunk_size_line = Vec::new();
                    reader
                        .read_until(b'\n', &mut chunk_size_line)
                        .expect("hello chunk size");
                    let chunk_size = usize::from_str_radix(
                        std::str::from_utf8(&chunk_size_line)
                            .expect("ASCII chunk size")
                            .trim(),
                        16,
                    )
                    .expect("hex chunk size");
                    let mut hello_and_terminator = vec![0_u8; chunk_size + 2];
                    reader
                        .read_exact(&mut hello_and_terminator)
                        .expect("hello frame");
                    request.extend_from_slice(&hello_and_terminator[..chunk_size]);
                    request_sender.send(request).expect("capture Relay request");

                    let stream = reader.get_mut();
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n",
                        )
                        .expect("successful Relay response");
                    write_fixture_chunk(
                        stream,
                        format!(
                            "{{\"type\":\"hello_ack\",\"connectionId\":\"tls-{generation}\"}}\n"
                        )
                        .as_bytes(),
                    );

                    if generation == 0 {
                        stream.sock.shutdown(std::net::Shutdown::Both).ok();
                    } else {
                        write_fixture_chunk(
                            reader.get_mut(),
                            b"{\"type\":\"request\",\"requestId\":\"room-request-1\",\"method\":\"moe_read_room\",\"params\":{\"roomId\":\"moe-dev-room\",\"afterMessageId\":\"welcome-2\",\"limit\":1}}\n",
                        );
                        let response = read_fixture_chunk(&mut reader);
                        response_sender
                            .send(serde_json::from_slice(&response).expect("Room response JSON"))
                            .expect("capture Room response");
                        let mut byte = [0_u8; 1];
                        let _ = reader.read(&mut byte);
                    }
                }
            });

            Self {
                endpoint: RelayHttpsEndpoint::parse(&format!(
                    "https://localhost:{port}/desktop-link"
                ))
                .expect("TLS fixture endpoint"),
                certificate_der: certificate.as_ref().to_vec(),
                request_receiver,
                response_receiver,
                worker,
            }
        }

        fn transport(&self) -> Arc<RelayHttpsTransport> {
            Arc::new(
                RelayHttpsTransport::with_test_root_certificate(
                    self.endpoint.clone(),
                    Duration::from_secs(1),
                    self.certificate_der.clone(),
                )
                .expect("fixture transport"),
            )
        }

        fn assert_authenticated_requests(&self, device_id: &RelayDeviceId) {
            let authorization = [b"Authorization: Bearer ".as_slice(), TEST_CREDENTIAL].concat();
            let hello_device = format!("\"deviceId\":\"{}\"", device_id.as_str());
            for _ in 0..2 {
                let request = self
                    .request_receiver
                    .recv_timeout(Duration::from_secs(2))
                    .expect("authenticated TLS request");
                assert!(
                    request
                        .windows(authorization.len())
                        .any(|value| value == authorization)
                );
                assert!(
                    request
                        .windows(hello_device.len())
                        .any(|value| value == hello_device.as_bytes())
                );
            }
        }

        fn assert_room_response(&self) {
            let response = self
                .response_receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("Room response");
            assert_eq!(response["type"], "response");
            assert_eq!(response["requestId"], "room-request-1");
            assert_eq!(response["result"]["ok"], true);
            assert_eq!(response["result"]["room"]["messages"][0]["id"], "welcome-3");
            assert_eq!(
                response["result"]["page"]["nextAfterMessageId"],
                "welcome-3"
            );
            assert!(response.get("error").is_none());
        }

        fn join(self) {
            self.worker.join().expect("TLS Relay fixture");
        }
    }

    fn write_fixture_chunk(stream: &mut impl Write, value: &[u8]) {
        write!(stream, "{:X}\r\n", value.len()).expect("chunk size");
        stream.write_all(value).expect("chunk value");
        stream.write_all(b"\r\n").expect("chunk terminator");
        stream.flush().expect("flush chunk");
    }

    fn read_fixture_chunk(reader: &mut impl BufRead) -> Vec<u8> {
        let mut size_line = Vec::new();
        reader
            .read_until(b'\n', &mut size_line)
            .expect("response chunk size");
        let size = usize::from_str_radix(
            std::str::from_utf8(&size_line)
                .expect("ASCII response chunk size")
                .trim(),
            16,
        )
        .expect("hex response chunk size");
        assert!(size <= 8 * 1024);
        let mut value = vec![0_u8; size];
        reader.read_exact(&mut value).expect("response chunk");
        let mut terminator = [0_u8; 2];
        reader
            .read_exact(&mut terminator)
            .expect("response chunk terminator");
        assert_eq!(terminator, *b"\r\n");
        value
    }

    fn exercise_product_tls_reconnect<S>(
        service: Arc<RelayClientService<S>>,
        account_id: RelayAccountId,
        device_id: RelayDeviceId,
    ) where
        S: CredentialStore + Send + Sync + 'static,
    {
        let fixture = ReconnectingTlsFixture::start();
        let runtime = desktop_relay_orchestrator();
        runtime
            .start(
                &account_id,
                relay_https_task_factory(
                    Arc::clone(&service),
                    account_id.clone(),
                    device_id.clone(),
                    fixture.transport(),
                    desktop_room_source(),
                ),
            )
            .expect("start product HTTPS task");

        let deadline = Instant::now() + Duration::from_secs(6);
        let mut connected_generations = Vec::new();
        let mut unexpected_disconnect = false;
        let mut retry_waiting = false;
        let mut retry_elapsed = false;
        let mut observed_events = Vec::new();
        while Instant::now() < deadline && connected_generations.len() < 2 {
            if let Some(update) = runtime.poll_next().expect("runtime update") {
                observed_events.push(update.event().kind());
                retry_waiting |= update.status().phase() == RelayRuntimePhase::RetryWaiting;
                match update.event().kind() {
                    DesktopRelayRuntimeEventKind::Connected => {
                        connected_generations.push(update.event().generation())
                    }
                    DesktopRelayRuntimeEventKind::UnexpectedDisconnect(
                        RelayConnectionErrorCode::RelayUnavailable,
                    ) => unexpected_disconnect = true,
                    DesktopRelayRuntimeEventKind::RetryElapsed => retry_elapsed = true,
                    DesktopRelayRuntimeEventKind::ConnectionFailed(_)
                    | DesktopRelayRuntimeEventKind::UnexpectedDisconnect(_) => {}
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(
            connected_generations.len(),
            2,
            "events={observed_events:?}, runtime={:?}, service={:?}",
            runtime.status(&account_id),
            service.status(&account_id)
        );
        assert_ne!(connected_generations[0], connected_generations[1]);
        assert!(unexpected_disconnect);
        assert!(retry_waiting);
        assert!(retry_elapsed);
        assert_eq!(
            service.status(&account_id).phase(),
            moe_relay_client::RelayConnectionPhase::Connected
        );
        fixture.assert_authenticated_requests(&device_id);
        fixture.assert_room_response();

        let stop_started = Instant::now();
        assert!(runtime.stop(&account_id).expect("stop product HTTPS task"));
        assert!(stop_started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            service.status(&account_id).phase(),
            moe_relay_client::RelayConnectionPhase::Offline
        );
        fixture.join();
    }

    #[test]
    fn bundled_config_is_absent_or_complete_and_never_accepts_http() {
        assert!(
            DesktopRelayProductConfig::from_values(None, None, None)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            DesktopRelayProductConfig::from_values(
                Some("https://relay.example.com/desktop-link"),
                None,
                Some("desktop-1"),
            )
            .unwrap_err(),
            DesktopRelayProductConfigError::Incomplete
        );
        assert_eq!(
            DesktopRelayProductConfig::from_values(
                Some("http://relay.example.com/desktop-link"),
                Some("primary"),
                Some("desktop-1"),
            )
            .unwrap_err(),
            DesktopRelayProductConfigError::InvalidEndpoint
        );
        assert_eq!(
            DesktopRelayProductConfig::from_values(
                Some("https://relay.example.com/desktop-link"),
                Some("../other"),
                Some("desktop-1"),
            )
            .unwrap_err(),
            DesktopRelayProductConfigError::InvalidIdentity
        );
        assert!(
            DesktopRelayProductConfig::from_values(
                Some("https://relay.example.com/desktop-link"),
                Some("primary"),
                Some("desktop-1"),
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn transport_errors_keep_the_existing_safe_runtime_codes() {
        use moe_relay_client::RelayTransportErrorKind;

        for (kind, expected) in [
            (
                RelayTransportErrorKind::Rejected,
                RelayConnectionErrorCode::RelayRejected,
            ),
            (
                RelayTransportErrorKind::Unavailable,
                RelayConnectionErrorCode::RelayUnavailable,
            ),
            (
                RelayTransportErrorKind::Protocol,
                RelayConnectionErrorCode::Protocol,
            ),
            (
                RelayTransportErrorKind::Cancelled,
                RelayConnectionErrorCode::Cancelled,
            ),
        ] {
            assert_eq!(
                transport_error_code(RelayTransportError::new(kind)),
                expected
            );
        }
    }

    #[test]
    fn room_router_rejects_unknown_invalid_and_duplicate_requests() {
        let source = desktop_room_source();
        let unknown = RelayRequestFrame::parse(serde_json::json!({
            "type":"request",
            "requestId":"request-unknown",
            "method":"moe_delete_room",
            "params":{}
        }))
        .unwrap();
        assert_eq!(
            route_request(&unknown, source.as_ref()).unwrap_err().code,
            RelayResponseErrorCode::UnsupportedMethod
        );

        let invalid = RelayRequestFrame::parse(serde_json::json!({
            "type":"request",
            "requestId":"request-invalid",
            "method":"moe_read_room",
            "params":{"limit":31}
        }))
        .unwrap();
        assert_eq!(
            route_request(&invalid, source.as_ref()).unwrap_err().code,
            RelayResponseErrorCode::InvalidRequest
        );

        let valid = RelayRequestFrame::parse(serde_json::json!({
            "type":"request",
            "requestId":"request-valid",
            "method":"moe_read_room",
            "params":{"roomId":"moe-dev-room","afterMessageId":"welcome-2","limit":1}
        }))
        .unwrap();
        let result = serde_json::to_value(route_request(&valid, source.as_ref()).unwrap()).unwrap();
        assert_eq!(result["room"]["messages"][0]["id"], "welcome-3");

        source
            .append_message(
                RoomMessageDraft::try_new(
                    "message-ui-write".to_owned(),
                    "moe-dev-room".to_owned(),
                    "owner".to_owned(),
                    vec!["codex".to_owned()],
                    "UI write reaches Relay read".to_owned(),
                    "2026-08-12T02:00:00Z".to_owned(),
                    Vec::new(),
                )
                .unwrap(),
            )
            .unwrap();
        let after_write = RelayRequestFrame::parse(serde_json::json!({
            "type":"request",
            "requestId":"request-after-write",
            "method":"moe_read_room",
            "params":{"roomId":"moe-dev-room","afterMessageId":"welcome-3","limit":1}
        }))
        .unwrap();
        let result =
            serde_json::to_value(route_request(&after_write, source.as_ref()).unwrap()).unwrap();
        assert_eq!(
            result["room"]["messages"][0]["body"],
            "UI write reaches Relay read"
        );

        let mut request_ids = HashSet::new();
        assert_eq!(
            register_request_id(&mut request_ids, "request-1"),
            RequestRegistration::Accepted
        );
        assert_eq!(
            register_request_id(&mut request_ids, "request-1"),
            RequestRegistration::Duplicate
        );
        for index in 2..=RELAY_MAXIMUM_REQUESTS_PER_CONNECTION {
            assert_eq!(
                register_request_id(&mut request_ids, &format!("request-{index}")),
                RequestRegistration::Accepted
            );
        }
        assert_eq!(
            register_request_id(&mut request_ids, "request-over-limit"),
            RequestRegistration::LimitReached
        );
    }

    #[test]
    fn product_https_task_authenticates_cancels_and_reconnects() {
        exercise_product_tls_reconnect(
            Arc::new(RelayClientService::new(TestCredentialStore)),
            RelayAccountId::new("tls-product-test").unwrap(),
            RelayDeviceId::new("desktop-test").unwrap(),
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "writes and removes one isolated Windows Credential Manager test target"]
    fn windows_credential_store_to_product_https_runtime() {
        use moe_credential_store::PlatformCredentialStore;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let account_id =
            RelayAccountId::new(format!("tls-product-{}-{unique}", std::process::id()))
                .expect("unique account ID");
        let credential_id = RelayCredentialId::new(account_id.as_str().to_owned()).unwrap();
        let store = PlatformCredentialStore;
        let credential = SecretBytes::new(TEST_CREDENTIAL.to_vec()).unwrap();
        store.store(&credential_id, &credential).unwrap();
        let cleanup = WindowsCredentialCleanup(credential_id.clone());

        exercise_product_tls_reconnect(
            Arc::new(RelayClientService::new(store)),
            account_id,
            RelayDeviceId::new("desktop-windows-test").unwrap(),
        );

        assert!(store.delete(&credential_id).unwrap());
        assert!(!store.contains(&credential_id).unwrap());
        drop(cleanup);
    }
}
