#![forbid(unsafe_code)]

use moe_relay_client::{
    RelayConnectionRequest, RelayTransport, RelayTransportError, RelayTransportErrorKind,
};
use rustls::{ClientConfig, ClientConnection, StreamOwned, pki_types::ServerName};
use rustls_platform_verifier::BuilderVerifierExt;
use serde::Serialize;
use serde_json::Value;
use std::{
    fmt,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const MAXIMUM_STREAM_HEADER_BYTES: usize = 8 * 1024;
pub const MAXIMUM_STREAM_FRAME_BYTES: usize = 8 * 1024;
const MAXIMUM_CHUNK_SIZE_LINE_BYTES: usize = 64;
const MAXIMUM_DNS_NAME_BYTES: usize = 253;
const DESKTOP_LINK_PATH: &str = "/desktop-link";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayHttpsEndpointError {
    HttpsRequired,
    InvalidAuthority,
    InvalidHostname,
    InvalidPort,
    InvalidPath,
}

impl fmt::Display for RelayHttpsEndpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::HttpsRequired => "relay endpoint must use HTTPS",
            Self::InvalidAuthority => "relay endpoint authority is invalid",
            Self::InvalidHostname => "relay endpoint hostname is invalid",
            Self::InvalidPort => "relay endpoint port is invalid",
            Self::InvalidPath => "relay endpoint path must be /desktop-link",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RelayHttpsEndpointError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayHttpsEndpoint {
    hostname: String,
    port: u16,
}

impl RelayHttpsEndpoint {
    pub fn parse(value: &str) -> Result<Self, RelayHttpsEndpointError> {
        let remainder = value
            .strip_prefix("https://")
            .ok_or(RelayHttpsEndpointError::HttpsRequired)?;
        if remainder.contains(['?', '#']) {
            return Err(RelayHttpsEndpointError::InvalidPath);
        }
        let (authority, path) = remainder
            .split_once('/')
            .ok_or(RelayHttpsEndpointError::InvalidPath)?;
        if path != DESKTOP_LINK_PATH.trim_start_matches('/') {
            return Err(RelayHttpsEndpointError::InvalidPath);
        }
        if authority.is_empty() || authority.contains('@') || authority.contains(['[', ']']) {
            return Err(RelayHttpsEndpointError::InvalidAuthority);
        }

        let (hostname, port) = match authority.rsplit_once(':') {
            Some((hostname, port)) => {
                if hostname.contains(':') {
                    return Err(RelayHttpsEndpointError::InvalidAuthority);
                }
                let port = port
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or(RelayHttpsEndpointError::InvalidPort)?;
                (hostname, port)
            }
            None => (authority, 443),
        };
        if !is_valid_dns_hostname(hostname) {
            return Err(RelayHttpsEndpointError::InvalidHostname);
        }

        Ok(Self {
            hostname: hostname.to_owned(),
            port,
        })
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    fn host_header(&self) -> String {
        if self.port == 443 {
            self.hostname.clone()
        } else {
            format!("{}:{}", self.hostname, self.port)
        }
    }
}

fn is_valid_dns_hostname(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= MAXIMUM_DNS_NAME_BYTES
        && hostname.is_ascii()
        && hostname.parse::<std::net::IpAddr>().is_err()
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

pub struct RelayHttpsTransport {
    endpoint: RelayHttpsEndpoint,
    timeout: Duration,
    client_config: Arc<ClientConfig>,
}

impl RelayHttpsTransport {
    pub fn new(
        endpoint: RelayHttpsEndpoint,
        timeout: Duration,
    ) -> Result<Self, RelayTransportError> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut client_config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| protocol_error())?
            .with_platform_verifier()
            .map_err(|_| protocol_error())?
            .with_no_client_auth();
        client_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self::with_client_config(
            endpoint,
            timeout,
            Arc::new(client_config),
        ))
    }

    fn with_client_config(
        endpoint: RelayHttpsEndpoint,
        timeout: Duration,
        client_config: Arc<ClientConfig>,
    ) -> Self {
        Self {
            endpoint,
            timeout,
            client_config,
        }
    }

    #[cfg(any(test, feature = "test-root-certificate"))]
    #[doc(hidden)]
    pub fn with_test_root_certificate(
        endpoint: RelayHttpsEndpoint,
        timeout: Duration,
        certificate_der: Vec<u8>,
    ) -> Result<Self, RelayTransportError> {
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(certificate_der))
            .map_err(|_| protocol_error())?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut client_config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| protocol_error())?
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self::with_client_config(
            endpoint,
            timeout,
            Arc::new(client_config),
        ))
    }
}

impl RelayTransport for RelayHttpsTransport {
    type Connection = RelayHttpsConnection;

    fn connect(
        &self,
        request: RelayConnectionRequest<'_>,
    ) -> Result<Self::Connection, RelayTransportError> {
        if self.timeout.is_zero() || !is_valid_bearer_credential(request.device_credential()) {
            return Err(protocol_error());
        }

        let tcp = Arc::new(connect_tcp(&self.endpoint, self.timeout)?);
        tcp.set_nonblocking(true).map_err(|_| unavailable_error())?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let shutdown = RelayShutdownHandle {
            stream: Arc::clone(&tcp),
            cancelled: Arc::clone(&cancelled),
        };
        let server_name =
            ServerName::try_from(self.endpoint.hostname.clone()).map_err(|_| protocol_error())?;
        let connection = ClientConnection::new(Arc::clone(&self.client_config), server_name)
            .map_err(|_| protocol_error())?;
        let mut stream = StreamOwned::new(
            connection,
            SharedTcpStream {
                stream: tcp,
                cancelled,
                timeout: self.timeout,
                deadline: Instant::now() + self.timeout,
            },
        );

        write_request_headers(&mut stream, &self.endpoint, request.device_credential())?;
        let hello = HelloFrame {
            frame_type: "hello",
            device_id: request.device_id().as_str(),
            protocol_version: moe_protocol::PROTOCOL_VERSION,
            capabilities: ["moe_read_room"],
        };
        write_json_chunk(&mut stream, &hello)?;

        let mut response = ChunkedHttpResponse::open(stream)?;
        let acknowledgement = response.read_frame()?;
        let valid_acknowledgement = acknowledgement
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|value| value == "hello_ack")
            && acknowledgement
                .get("connectionId")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty());
        if !valid_acknowledgement {
            return Err(protocol_error());
        }

        Ok(RelayHttpsConnection { response, shutdown })
    }
}

fn connect_tcp(
    endpoint: &RelayHttpsEndpoint,
    timeout: Duration,
) -> Result<TcpStream, RelayTransportError> {
    let addresses = (endpoint.hostname.as_str(), endpoint.port)
        .to_socket_addrs()
        .map_err(|_| unavailable_error())?;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect_timeout(&address, timeout) {
            return Ok(stream);
        }
    }
    Err(unavailable_error())
}

fn is_valid_bearer_credential(credential: &[u8]) -> bool {
    !credential.is_empty()
        && credential.iter().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z'
                    | b'A'..=b'Z'
                    | b'0'..=b'9'
                    | b'-'
                    | b'.'
                    | b'_'
                    | b'~'
                    | b'+'
                    | b'/'
                    | b'='
            )
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HelloFrame<'a> {
    #[serde(rename = "type")]
    frame_type: &'static str,
    device_id: &'a str,
    protocol_version: &'static str,
    capabilities: [&'static str; 1],
}

fn write_request_headers(
    stream: &mut impl Write,
    endpoint: &RelayHttpsEndpoint,
    credential: &[u8],
) -> Result<(), RelayTransportError> {
    stream
        .write_all(b"POST /desktop-link HTTP/1.1\r\nHost: ")
        .and_then(|_| stream.write_all(endpoint.host_header().as_bytes()))
        .and_then(|_| stream.write_all(b"\r\nAuthorization: Bearer "))
        .and_then(|_| stream.write_all(credential))
        .and_then(|_| {
            stream.write_all(
                b"\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
        })
        .map_err(|_| unavailable_error())
}

fn write_json_chunk(
    stream: &mut impl Write,
    value: &impl Serialize,
) -> Result<(), RelayTransportError> {
    let mut encoded = serde_json::to_vec(value).map_err(|_| protocol_error())?;
    encoded.push(b'\n');
    write_http_chunk(stream, &encoded)
}

fn write_http_chunk(stream: &mut impl Write, value: &[u8]) -> Result<(), RelayTransportError> {
    if value.is_empty() || value.len() > MAXIMUM_STREAM_FRAME_BYTES {
        return Err(protocol_error());
    }
    write!(stream, "{:X}\r\n", value.len())
        .and_then(|_| stream.write_all(value))
        .and_then(|_| stream.write_all(b"\r\n"))
        .and_then(|_| stream.flush())
        .map_err(|_| unavailable_error())
}

struct SharedTcpStream {
    stream: Arc<TcpStream>,
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
    deadline: Instant,
}

impl SharedTcpStream {
    fn begin_operation(&mut self) {
        self.deadline = Instant::now() + self.timeout;
    }
}

impl Read for SharedTcpStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(cancelled_io_error());
            }
            match self.stream.as_ref().read(buffer) {
                Ok(read) => return Ok(read),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    if Instant::now() >= self.deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "relay TLS read timed out",
                        ));
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl Write for SharedTcpStream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(cancelled_io_error());
            }
            match self.stream.as_ref().write(buffer) {
                Ok(written) => return Ok(written),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    if Instant::now() >= self.deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "relay TLS write timed out",
                        ));
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn cancelled_io_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        "relay TLS I/O was cancelled",
    )
}

type RelayTlsStream = StreamOwned<ClientConnection, SharedTcpStream>;

pub struct RelayHttpsConnection {
    response: ChunkedHttpResponse<RelayTlsStream>,
    shutdown: RelayShutdownHandle,
}

impl RelayHttpsConnection {
    pub fn read_frame(&mut self) -> Result<Value, RelayTransportError> {
        self.response.reader.get_mut().sock.begin_operation();
        self.response.read_frame().map_err(|error| {
            if self.shutdown.is_cancelled() {
                RelayTransportError::new(RelayTransportErrorKind::Cancelled)
            } else {
                error
            }
        })
    }

    pub fn write_frame(&mut self, value: &impl Serialize) -> Result<(), RelayTransportError> {
        self.response.reader.get_mut().sock.begin_operation();
        write_json_chunk(self.response.reader.get_mut(), value).map_err(|error| {
            if self.shutdown.is_cancelled() {
                RelayTransportError::new(RelayTransportErrorKind::Cancelled)
            } else {
                error
            }
        })
    }

    pub fn shutdown_handle(&self) -> RelayShutdownHandle {
        self.shutdown.clone()
    }
}

#[derive(Clone)]
pub struct RelayShutdownHandle {
    stream: Arc<TcpStream>,
    cancelled: Arc<AtomicBool>,
}

impl RelayShutdownHandle {
    pub fn shutdown(&self) {
        self.cancelled.store(true, Ordering::Release);
        let _ = self.stream.shutdown(Shutdown::Both);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

struct ChunkedHttpResponse<S> {
    reader: BufReader<S>,
    decoded: Vec<u8>,
    finished: bool,
}

impl<S: Read> ChunkedHttpResponse<S> {
    fn open(stream: S) -> Result<Self, RelayTransportError> {
        let mut reader = BufReader::new(stream);
        let status_line = read_bounded_line(&mut reader, 256)?;
        let mut status_parts = std::str::from_utf8(&status_line)
            .map_err(|_| protocol_error())?
            .split_ascii_whitespace();
        if status_parts.next() != Some("HTTP/1.1") {
            return Err(protocol_error());
        }
        let status = status_parts
            .next()
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(protocol_error)?;

        let mut header_bytes = status_line.len();
        let mut chunked = false;
        let mut ndjson = false;
        loop {
            let remaining = MAXIMUM_STREAM_HEADER_BYTES
                .checked_sub(header_bytes)
                .ok_or_else(protocol_error)?;
            let line = read_bounded_line(&mut reader, remaining)?;
            header_bytes += line.len();
            if line == b"\r\n" || line == b"\n" {
                break;
            }
            if let Ok(line) = std::str::from_utf8(&line)
                && let Some((name, value)) = line.split_once(':')
            {
                if name.eq_ignore_ascii_case("transfer-encoding")
                    && value.trim().eq_ignore_ascii_case("chunked")
                {
                    chunked = true;
                }
                if name.eq_ignore_ascii_case("content-type")
                    && value
                        .trim()
                        .split(';')
                        .next()
                        .is_some_and(|value| value.eq_ignore_ascii_case("application/x-ndjson"))
                {
                    ndjson = true;
                }
            }
        }

        if status == 401 || status == 403 {
            return Err(RelayTransportError::new(RelayTransportErrorKind::Rejected));
        }
        if status != 200 || !chunked || !ndjson {
            return Err(protocol_error());
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
                return serde_json::from_slice(&line).map_err(|_| protocol_error());
            }
            if self.finished {
                return Err(protocol_error());
            }

            let size_line = read_bounded_line(&mut self.reader, MAXIMUM_CHUNK_SIZE_LINE_BYTES)?;
            let size_text = std::str::from_utf8(&size_line)
                .map_err(|_| protocol_error())?
                .trim();
            let size = usize::from_str_radix(
                size_text
                    .split_once(';')
                    .map_or(size_text, |(size, _)| size),
                16,
            )
            .map_err(|_| protocol_error())?;
            if size == 0 {
                let mut trailer_bytes = 0;
                loop {
                    let remaining = MAXIMUM_STREAM_HEADER_BYTES
                        .checked_sub(trailer_bytes)
                        .ok_or_else(protocol_error)?;
                    let trailer = read_bounded_line(&mut self.reader, remaining)?;
                    trailer_bytes += trailer.len();
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
                return Err(protocol_error());
            }

            let start = self.decoded.len();
            self.decoded.resize(start + size, 0);
            self.reader
                .read_exact(&mut self.decoded[start..])
                .map_err(|_| unavailable_error())?;
            let mut terminator = [0_u8; 2];
            self.reader
                .read_exact(&mut terminator)
                .map_err(|_| unavailable_error())?;
            if terminator != *b"\r\n" {
                return Err(protocol_error());
            }
        }
    }
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    maximum: usize,
) -> Result<Vec<u8>, RelayTransportError> {
    if maximum == 0 {
        return Err(protocol_error());
    }
    let mut line = Vec::new();
    reader
        .take((maximum + 1) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|_| unavailable_error())?;
    if line.is_empty() {
        return Err(unavailable_error());
    }
    if line.len() > maximum || !line.ends_with(b"\n") {
        return Err(protocol_error());
    }
    Ok(line)
}

fn protocol_error() -> RelayTransportError {
    RelayTransportError::new(RelayTransportErrorKind::Protocol)
}

fn unavailable_error() -> RelayTransportError {
    RelayTransportError::new(RelayTransportErrorKind::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_credential_store::{
        CredentialStore, CredentialStoreError, RelayCredentialId, SecretBytes,
    };
    use moe_relay_client::{RelayAccountId, RelayConnectionManager, RelayDeviceId};
    use rcgen::{CertifiedKey, generate_simple_self_signed};
    use rustls::{
        RootCertStore, ServerConfig, ServerConnection,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    };
    use std::{
        net::TcpListener,
        sync::mpsc,
        thread::{self, JoinHandle},
        time::Instant,
    };

    const TEST_CREDENTIAL: &[u8] = b"test-device-credential";

    struct TestCredentialStore(&'static [u8]);

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
            SecretBytes::new(self.0.to_vec()).map(Some)
        }

        fn delete(&self, _id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
            Ok(true)
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum FixtureResponse {
        AckAndClose,
        AckAndHold,
        HoldWithoutResponse,
        Unauthorized,
        OversizedHeader,
        OversizedFrame,
    }

    struct TlsFixture {
        endpoint: RelayHttpsEndpoint,
        certificate: CertificateDer<'static>,
        request_receiver: mpsc::Receiver<Vec<u8>>,
        release_sender: mpsc::Sender<()>,
        worker: JoinHandle<()>,
    }

    impl TlsFixture {
        fn start(certificate_hostname: &str, response: FixtureResponse) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind TLS fixture");
            let port = listener.local_addr().expect("fixture address").port();
            let CertifiedKey { cert, signing_key } =
                generate_simple_self_signed(vec![certificate_hostname.to_owned()])
                    .expect("generate fixture certificate");
            let certificate = cert.der().clone();
            let private_key =
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(signing_key.serialize_der()));
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let server_config = ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("safe TLS versions")
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], private_key)
                .expect("fixture certificate and key");
            let (request_sender, request_receiver) = mpsc::channel();
            let (release_sender, release_receiver) = mpsc::channel();
            let worker = thread::spawn(move || {
                let (tcp, _) = listener.accept().expect("accept TLS fixture client");
                tcp.set_read_timeout(Some(Duration::from_secs(3)))
                    .and_then(|_| tcp.set_write_timeout(Some(Duration::from_secs(3))))
                    .expect("fixture timeouts");
                let connection =
                    ServerConnection::new(Arc::new(server_config)).expect("TLS server connection");
                let stream = StreamOwned::new(connection, tcp);
                let mut reader = BufReader::new(stream);
                let mut request = Vec::new();
                loop {
                    let mut line = Vec::new();
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    request.extend_from_slice(&line);
                    if line == b"\r\n" || line == b"\n" || request.len() > 16 * 1024 {
                        break;
                    }
                }
                let _ = request_sender.send(request);

                let mut chunk_size_line = Vec::new();
                if reader.read_until(b'\n', &mut chunk_size_line).is_err() {
                    return;
                }
                let Some(chunk_size) = std::str::from_utf8(&chunk_size_line)
                    .ok()
                    .and_then(|line| usize::from_str_radix(line.trim(), 16).ok())
                else {
                    return;
                };
                let mut hello_and_terminator = vec![0_u8; chunk_size + 2];
                if reader.read_exact(&mut hello_and_terminator).is_err() {
                    return;
                }

                let stream = reader.get_mut();
                match response {
                    FixtureResponse::Unauthorized => {
                        stream
                            .write_all(
                                b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n",
                            )
                            .expect("write rejection");
                    }
                    FixtureResponse::OversizedHeader => {
                        stream
                            .write_all(b"HTTP/1.1 200 OK\r\nX-Oversized: ")
                            .and_then(|_| {
                                stream.write_all(&vec![b'a'; MAXIMUM_STREAM_HEADER_BYTES + 1])
                            })
                            .and_then(|_| stream.write_all(b"\r\n\r\n"))
                            .expect("write oversized header");
                    }
                    FixtureResponse::OversizedFrame => {
                        stream
                            .write_all(
                                b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n",
                            )
                            .and_then(|_| {
                                write!(stream, "{:X}\r\n", MAXIMUM_STREAM_FRAME_BYTES + 1)
                            })
                            .expect("write oversized frame declaration");
                    }
                    FixtureResponse::AckAndClose | FixtureResponse::AckAndHold => {
                        stream
                            .write_all(
                                b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n",
                            )
                            .expect("write successful headers");
                        write_http_chunk(
                            stream,
                            b"{\"type\":\"hello_ack\",\"connectionId\":\"fixture-1\"}\n",
                        )
                        .expect("write acknowledgement");
                        if matches!(response, FixtureResponse::AckAndHold) {
                            let _ = release_receiver.recv_timeout(Duration::from_secs(3));
                        }
                    }
                    FixtureResponse::HoldWithoutResponse => {
                        let _ = release_receiver.recv_timeout(Duration::from_secs(3));
                    }
                }
                let _ = stream.flush();
            });

            Self {
                endpoint: RelayHttpsEndpoint::parse(&format!(
                    "https://localhost:{port}/desktop-link"
                ))
                .expect("fixture endpoint"),
                certificate,
                request_receiver,
                release_sender,
                worker,
            }
        }

        fn trusted_transport(&self, timeout: Duration) -> RelayHttpsTransport {
            let mut roots = RootCertStore::empty();
            roots
                .add(self.certificate.clone())
                .expect("trust fixture certificate");
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let mut config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("safe TLS versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            config.alpn_protocols = vec![b"http/1.1".to_vec()];
            RelayHttpsTransport::with_client_config(
                self.endpoint.clone(),
                timeout,
                Arc::new(config),
            )
        }

        fn untrusted_transport(&self, timeout: Duration) -> RelayHttpsTransport {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let mut config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("safe TLS versions")
                .with_root_certificates(RootCertStore::empty())
                .with_no_client_auth();
            config.alpn_protocols = vec![b"http/1.1".to_vec()];
            RelayHttpsTransport::with_client_config(
                self.endpoint.clone(),
                timeout,
                Arc::new(config),
            )
        }

        fn join(self) {
            let _ = self.release_sender.send(());
            self.worker.join().expect("TLS fixture worker");
        }
    }

    fn connect(
        transport: &RelayHttpsTransport,
    ) -> Result<RelayHttpsConnection, RelayTransportError> {
        connect_with_credential(transport, TEST_CREDENTIAL)
    }

    fn connect_with_credential(
        transport: &RelayHttpsTransport,
        credential: &'static [u8],
    ) -> Result<RelayHttpsConnection, RelayTransportError> {
        let manager = RelayConnectionManager::new(TestCredentialStore(credential));
        let account = RelayAccountId::new("fixture-account").expect("account ID");
        let device = RelayDeviceId::new("fixture-device").expect("device ID");
        manager
            .connect(&account, &device, transport)
            .map_err(|error| match error {
                moe_relay_client::RelayClientError::Transport(error) => error,
                other => panic!("unexpected relay client error: {other}"),
            })
    }

    fn connect_error(transport: &RelayHttpsTransport, message: &str) -> RelayTransportError {
        match connect(transport) {
            Ok(_) => panic!("{message}"),
            Err(error) => error,
        }
    }

    #[test]
    fn endpoint_accepts_only_https_dns_desktop_link() {
        let endpoint = RelayHttpsEndpoint::parse("https://relay.example.com/desktop-link")
            .expect("valid endpoint");
        assert_eq!(endpoint.hostname(), "relay.example.com");
        assert_eq!(endpoint.port(), 443);

        let endpoint = RelayHttpsEndpoint::parse("https://localhost:8443/desktop-link")
            .expect("valid local fixture endpoint");
        assert_eq!(endpoint.port(), 8443);

        for invalid in [
            "http://relay.example.com/desktop-link",
            "https://127.0.0.1/desktop-link",
            "https://[::1]/desktop-link",
            "https://user@relay.example.com/desktop-link",
            "https://relay.example.com/other",
            "https://relay.example.com/desktop-link?debug=true",
            "https://relay.example.com:0/desktop-link",
            "https://-relay.example.com/desktop-link",
        ] {
            assert!(RelayHttpsEndpoint::parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn bounded_line_does_not_read_past_the_limit() {
        let mut reader = BufReader::new(&b"12345\nnext\n"[..]);
        assert_eq!(read_bounded_line(&mut reader, 4), Err(protocol_error()));
    }

    #[test]
    fn bearer_credential_rejects_header_unsafe_bytes_before_network_io() {
        let endpoint =
            RelayHttpsEndpoint::parse("https://localhost:9/desktop-link").expect("local endpoint");
        let transport = RelayHttpsTransport::with_client_config(
            endpoint,
            Duration::from_secs(1),
            Arc::new(
                ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .expect("safe TLS versions")
                .with_root_certificates(RootCertStore::empty())
                .with_no_client_auth(),
            ),
        );
        let error = match connect_with_credential(&transport, b"unsafe\r\nheader") {
            Ok(_) => panic!("header-unsafe credential must fail"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), RelayTransportErrorKind::Protocol);
    }

    #[test]
    fn trusted_local_tls_fixture_completes_authenticated_handshake() {
        let fixture = TlsFixture::start("localhost", FixtureResponse::AckAndClose);
        let transport = fixture.trusted_transport(Duration::from_secs(2));
        let _connection = connect(&transport).expect("trusted TLS handshake");
        let request = fixture
            .request_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("fixture request");
        let expected = [b"Authorization: Bearer ".as_slice(), TEST_CREDENTIAL].concat();
        assert!(
            request
                .windows(expected.len())
                .any(|value| value == expected)
        );
        fixture.join();
    }

    #[test]
    fn tls_rejects_untrusted_certificate() {
        let fixture = TlsFixture::start("localhost", FixtureResponse::AckAndClose);
        let transport = fixture.untrusted_transport(Duration::from_secs(1));
        assert_eq!(
            connect_error(&transport, "untrusted certificate must fail").kind(),
            RelayTransportErrorKind::Unavailable
        );
        fixture.join();
    }

    #[test]
    fn tls_rejects_hostname_mismatch() {
        let fixture = TlsFixture::start("not-localhost.example", FixtureResponse::AckAndClose);
        let transport = fixture.trusted_transport(Duration::from_secs(1));
        assert_eq!(
            connect_error(&transport, "hostname mismatch must fail").kind(),
            RelayTransportErrorKind::Unavailable
        );
        fixture.join();
    }

    #[test]
    fn http_rejection_maps_to_rejected_without_server_body() {
        let fixture = TlsFixture::start("localhost", FixtureResponse::Unauthorized);
        let transport = fixture.trusted_transport(Duration::from_secs(1));
        assert_eq!(
            connect_error(&transport, "401 must fail").kind(),
            RelayTransportErrorKind::Rejected
        );
        fixture.join();
    }

    #[test]
    fn oversized_response_header_and_frame_are_rejected() {
        for response in [
            FixtureResponse::OversizedHeader,
            FixtureResponse::OversizedFrame,
        ] {
            let fixture = TlsFixture::start("localhost", response);
            let transport = fixture.trusted_transport(Duration::from_secs(1));
            let error = connect_error(&transport, "oversized response must fail");
            fixture.join();
            assert_eq!(
                error.kind(),
                RelayTransportErrorKind::Protocol,
                "{response:?}"
            );
        }
    }

    #[test]
    fn shutdown_handle_interrupts_blocking_tls_read() {
        let fixture = TlsFixture::start("localhost", FixtureResponse::AckAndHold);
        let transport = fixture.trusted_transport(Duration::from_secs(5));
        let mut connection = connect(&transport).expect("trusted TLS handshake");
        let shutdown = connection.shutdown_handle();
        let (result_sender, result_receiver) = mpsc::channel();
        let started = Instant::now();
        let reader = thread::spawn(move || {
            let _ = result_sender.send(connection.read_frame());
        });
        thread::sleep(Duration::from_millis(50));
        shutdown.shutdown();
        let result = result_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("shutdown must release blocking read");
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
        reader.join().expect("reader worker");
        fixture.join();
    }

    #[test]
    fn stalled_tls_response_obeys_read_timeout() {
        let fixture = TlsFixture::start("localhost", FixtureResponse::HoldWithoutResponse);
        let transport = fixture.trusted_transport(Duration::from_millis(100));
        let started = Instant::now();
        assert_eq!(
            connect_error(&transport, "stalled response must time out").kind(),
            RelayTransportErrorKind::Unavailable
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        fixture.join();
    }
}
