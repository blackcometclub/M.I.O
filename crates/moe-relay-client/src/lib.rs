#![forbid(unsafe_code)]

mod lifecycle;

pub use lifecycle::{
    RELAY_RETRY_DELAYS_MS, RelayLifecycle, RelayLifecycleAction, RelayLifecycleTransitionError,
    RelayRuntimePhase, RelayRuntimeStatus,
};

use moe_credential_store::{CredentialStore, CredentialStoreError, RelayCredentialId, SecretBytes};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

const MAXIMUM_DEVICE_ID_BYTES: usize = 128;
const PAIRING_CODE_BYTES: usize = 9;
const PAIRING_CODE_SEPARATOR_INDEX: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayModelError {
    InvalidAccountId,
    InvalidDeviceId,
    InvalidPairingCode,
    InvalidDeviceCredential,
}

impl fmt::Display for RelayModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAccountId => formatter.write_str("relay account ID is invalid"),
            Self::InvalidDeviceId => formatter.write_str("relay device ID is invalid"),
            Self::InvalidPairingCode => formatter.write_str("relay pairing code is invalid"),
            Self::InvalidDeviceCredential => {
                formatter.write_str("relay device credential is invalid")
            }
        }
    }
}

impl Error for RelayModelError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayAccountId(RelayCredentialId);

impl RelayAccountId {
    pub fn new(value: impl Into<String>) -> Result<Self, RelayModelError> {
        RelayCredentialId::new(value)
            .map(Self)
            .map_err(|_| RelayModelError::InvalidAccountId)
    }

    pub fn as_str(&self) -> &str {
        self.0.account_id()
    }

    fn credential_id(&self) -> &RelayCredentialId {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayDeviceId(String);

impl RelayDeviceId {
    pub fn new(value: impl Into<String>) -> Result<Self, RelayModelError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAXIMUM_DEVICE_ID_BYTES
            || !value.bytes().enumerate().all(|(index, byte)| match byte {
                b'a'..=b'z' | b'0'..=b'9' => true,
                b'.' | b'_' | b'-' => index > 0,
                _ => false,
            })
        {
            return Err(RelayModelError::InvalidDeviceId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct PairingCode(SecretBytes);

impl PairingCode {
    pub fn new(value: impl Into<String>) -> Result<Self, RelayModelError> {
        let value = SecretBytes::new(value.into().into_bytes())
            .map_err(|_| RelayModelError::InvalidPairingCode)?;
        let valid = value.expose().len() == PAIRING_CODE_BYTES
            && value.expose().iter().enumerate().all(|(index, byte)| {
                if index == PAIRING_CODE_SEPARATOR_INDEX {
                    *byte == b'-'
                } else {
                    matches!(
                        byte,
                        b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'2'..=b'9'
                    )
                }
            });
        if !valid {
            return Err(RelayModelError::InvalidPairingCode);
        }
        Ok(Self(value))
    }

    fn expose(&self) -> &str {
        std::str::from_utf8(self.0.expose()).expect("validated pairing code must be ASCII")
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingCode([REDACTED])")
    }
}

pub struct PairingResponse {
    device_id: RelayDeviceId,
    device_credential: SecretBytes,
}

impl PairingResponse {
    pub fn new(
        device_id: RelayDeviceId,
        device_credential: Vec<u8>,
    ) -> Result<Self, RelayModelError> {
        let device_credential = SecretBytes::new(device_credential)
            .map_err(|_| RelayModelError::InvalidDeviceCredential)?;
        Ok(Self {
            device_id,
            device_credential,
        })
    }
}

impl fmt::Debug for PairingResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingResponse")
            .field("device_id", &self.device_id)
            .field("device_credential", &"[REDACTED]")
            .finish()
    }
}

pub struct RelayPairingRequest<'a> {
    device_id: &'a RelayDeviceId,
    pairing_code: &'a PairingCode,
}

impl RelayPairingRequest<'_> {
    pub fn device_id(&self) -> &RelayDeviceId {
        self.device_id
    }

    pub fn pairing_code(&self) -> &str {
        self.pairing_code.expose()
    }
}

impl fmt::Debug for RelayPairingRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayPairingRequest")
            .field("device_id", self.device_id)
            .field("pairing_code", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayPairingTransportErrorKind {
    InvalidCode,
    Expired,
    Used,
    Locked,
    Unavailable,
    Protocol,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayPairingTransportError {
    kind: RelayPairingTransportErrorKind,
}

impl RelayPairingTransportError {
    pub fn new(kind: RelayPairingTransportErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> RelayPairingTransportErrorKind {
        self.kind
    }
}

impl fmt::Display for RelayPairingTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RelayPairingTransportErrorKind::InvalidCode => {
                formatter.write_str("relay pairing code was rejected")
            }
            RelayPairingTransportErrorKind::Expired => {
                formatter.write_str("relay pairing code expired")
            }
            RelayPairingTransportErrorKind::Used => {
                formatter.write_str("relay pairing code was already used")
            }
            RelayPairingTransportErrorKind::Locked => {
                formatter.write_str("relay pairing code is locked")
            }
            RelayPairingTransportErrorKind::Unavailable => {
                formatter.write_str("relay pairing service is unavailable")
            }
            RelayPairingTransportErrorKind::Protocol => {
                formatter.write_str("relay pairing protocol failed")
            }
            RelayPairingTransportErrorKind::Cancelled => {
                formatter.write_str("relay pairing was cancelled")
            }
        }
    }
}

impl Error for RelayPairingTransportError {}

pub trait RelayPairingTransport {
    fn exchange(
        &self,
        request: RelayPairingRequest<'_>,
    ) -> Result<PairingResponse, RelayPairingTransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingReceipt {
    account_id: RelayAccountId,
    device_id: RelayDeviceId,
}

impl PairingReceipt {
    pub fn account_id(&self) -> &RelayAccountId {
        &self.account_id
    }

    pub fn device_id(&self) -> &RelayDeviceId {
        &self.device_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayTransportErrorKind {
    Rejected,
    Unavailable,
    Protocol,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayTransportError {
    kind: RelayTransportErrorKind,
}

impl RelayTransportError {
    pub fn new(kind: RelayTransportErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> RelayTransportErrorKind {
        self.kind
    }

    pub fn safe_error_code(&self) -> RelayConnectionErrorCode {
        match self.kind {
            RelayTransportErrorKind::Rejected => RelayConnectionErrorCode::RelayRejected,
            RelayTransportErrorKind::Unavailable => RelayConnectionErrorCode::RelayUnavailable,
            RelayTransportErrorKind::Protocol => RelayConnectionErrorCode::Protocol,
            RelayTransportErrorKind::Cancelled => RelayConnectionErrorCode::Cancelled,
        }
    }
}

impl fmt::Display for RelayTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RelayTransportErrorKind::Rejected => {
                formatter.write_str("relay rejected the device credential")
            }
            RelayTransportErrorKind::Unavailable => formatter.write_str("relay is unavailable"),
            RelayTransportErrorKind::Protocol => {
                formatter.write_str("relay protocol negotiation failed")
            }
            RelayTransportErrorKind::Cancelled => {
                formatter.write_str("relay connection was cancelled")
            }
        }
    }
}

impl Error for RelayTransportError {}

pub struct RelayConnectionRequest<'a> {
    account_id: &'a RelayAccountId,
    device_id: &'a RelayDeviceId,
    device_credential: &'a SecretBytes,
}

impl RelayConnectionRequest<'_> {
    pub fn account_id(&self) -> &RelayAccountId {
        self.account_id
    }

    pub fn device_id(&self) -> &RelayDeviceId {
        self.device_id
    }

    pub fn device_credential(&self) -> &[u8] {
        self.device_credential.expose()
    }
}

impl fmt::Debug for RelayConnectionRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayConnectionRequest")
            .field("account_id", self.account_id)
            .field("device_id", self.device_id)
            .field("device_credential", &"[REDACTED]")
            .finish()
    }
}

pub trait RelayTransport {
    type Connection;

    fn connect(
        &self,
        request: RelayConnectionRequest<'_>,
    ) -> Result<Self::Connection, RelayTransportError>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum RelayClientError {
    CredentialStore(CredentialStoreError),
    PairingDeviceMismatch,
    CredentialNotStored,
    PairingTransport(RelayPairingTransportError),
    Transport(RelayTransportError),
}

impl fmt::Display for RelayClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CredentialStore(error) => error.fmt(formatter),
            Self::PairingDeviceMismatch => {
                formatter.write_str("pairing response belongs to another relay device")
            }
            Self::CredentialNotStored => {
                formatter.write_str("relay device credential is not stored")
            }
            Self::PairingTransport(error) => error.fmt(formatter),
            Self::Transport(error) => error.fmt(formatter),
        }
    }
}

impl Error for RelayClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CredentialStore(error) => Some(error),
            Self::PairingTransport(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::PairingDeviceMismatch | Self::CredentialNotStored => None,
        }
    }
}

impl From<CredentialStoreError> for RelayClientError {
    fn from(error: CredentialStoreError) -> Self {
        Self::CredentialStore(error)
    }
}

pub struct RelayConnectionManager<S> {
    credential_store: S,
}

impl<S> RelayConnectionManager<S> {
    pub fn new(credential_store: S) -> Self {
        Self { credential_store }
    }
}

impl<S: CredentialStore> RelayConnectionManager<S> {
    pub fn has_credential(&self, account_id: &RelayAccountId) -> Result<bool, RelayClientError> {
        self.credential_store
            .contains(account_id.credential_id())
            .map_err(Into::into)
    }

    pub fn pair<T: RelayPairingTransport>(
        &self,
        account_id: &RelayAccountId,
        device_id: &RelayDeviceId,
        pairing_code: PairingCode,
        transport: &T,
    ) -> Result<PairingReceipt, RelayClientError> {
        let response = transport
            .exchange(RelayPairingRequest {
                device_id,
                pairing_code: &pairing_code,
            })
            .map_err(RelayClientError::PairingTransport)?;
        self.accept_pairing(account_id, device_id, response)
    }

    pub fn accept_pairing(
        &self,
        account_id: &RelayAccountId,
        expected_device_id: &RelayDeviceId,
        response: PairingResponse,
    ) -> Result<PairingReceipt, RelayClientError> {
        if response.device_id != *expected_device_id {
            return Err(RelayClientError::PairingDeviceMismatch);
        }

        self.credential_store
            .store(account_id.credential_id(), &response.device_credential)?;

        Ok(PairingReceipt {
            account_id: account_id.clone(),
            device_id: response.device_id,
        })
    }

    pub fn connect<T: RelayTransport>(
        &self,
        account_id: &RelayAccountId,
        device_id: &RelayDeviceId,
        transport: &T,
    ) -> Result<T::Connection, RelayClientError> {
        let device_credential = self
            .credential_store
            .load(account_id.credential_id())?
            .ok_or(RelayClientError::CredentialNotStored)?;

        transport
            .connect(RelayConnectionRequest {
                account_id,
                device_id,
                device_credential: &device_credential,
            })
            .map_err(RelayClientError::Transport)
    }

    pub fn delete_credential(&self, account_id: &RelayAccountId) -> Result<bool, RelayClientError> {
        self.credential_store
            .delete(account_id.credential_id())
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayConnectionPhase {
    Offline,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayConnectionErrorCode {
    CredentialMissing,
    SecureStorageUnavailable,
    SecureStorageFailed,
    PairingRejected,
    PairingExpired,
    PairingUsed,
    PairingLocked,
    RelayRejected,
    RelayUnavailable,
    Protocol,
    Cancelled,
    AlreadyActive,
    RuntimeUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayConnectionStatus {
    account_id: RelayAccountId,
    phase: RelayConnectionPhase,
    credential_stored: bool,
    last_error_code: Option<RelayConnectionErrorCode>,
}

impl RelayConnectionStatus {
    pub fn account_id(&self) -> &RelayAccountId {
        &self.account_id
    }

    pub fn phase(&self) -> RelayConnectionPhase {
        self.phase
    }

    pub fn credential_stored(&self) -> bool {
        self.credential_stored
    }

    pub fn last_error_code(&self) -> Option<RelayConnectionErrorCode> {
        self.last_error_code
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RelayServiceError {
    AlreadyActive,
    Client(RelayClientError),
}

impl RelayServiceError {
    pub fn safe_error_code(&self) -> RelayConnectionErrorCode {
        match self {
            Self::AlreadyActive => RelayConnectionErrorCode::AlreadyActive,
            Self::Client(error) => safe_error_code(error),
        }
    }
}

impl fmt::Display for RelayServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive => {
                formatter.write_str("relay account connection is already active")
            }
            Self::Client(error) => error.fmt(formatter),
        }
    }
}

impl Error for RelayServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AlreadyActive => None,
            Self::Client(error) => Some(error),
        }
    }
}

impl From<RelayClientError> for RelayServiceError {
    fn from(error: RelayClientError) -> Self {
        Self::Client(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccountConnectionState {
    phase: RelayConnectionPhase,
    last_error_code: Option<RelayConnectionErrorCode>,
}

impl Default for AccountConnectionState {
    fn default() -> Self {
        Self {
            phase: RelayConnectionPhase::Offline,
            last_error_code: None,
        }
    }
}

type ConnectionStates = Arc<Mutex<HashMap<String, AccountConnectionState>>>;

fn lock_states(
    states: &ConnectionStates,
) -> MutexGuard<'_, HashMap<String, AccountConnectionState>> {
    states
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn safe_error_code(error: &RelayClientError) -> RelayConnectionErrorCode {
    match error {
        RelayClientError::CredentialStore(CredentialStoreError::UnsupportedPlatform) => {
            RelayConnectionErrorCode::SecureStorageUnavailable
        }
        RelayClientError::CredentialStore(_) => RelayConnectionErrorCode::SecureStorageFailed,
        RelayClientError::PairingDeviceMismatch => RelayConnectionErrorCode::Protocol,
        RelayClientError::CredentialNotStored => RelayConnectionErrorCode::CredentialMissing,
        RelayClientError::PairingTransport(error) => match error.kind() {
            RelayPairingTransportErrorKind::InvalidCode => {
                RelayConnectionErrorCode::PairingRejected
            }
            RelayPairingTransportErrorKind::Expired => RelayConnectionErrorCode::PairingExpired,
            RelayPairingTransportErrorKind::Used => RelayConnectionErrorCode::PairingUsed,
            RelayPairingTransportErrorKind::Locked => RelayConnectionErrorCode::PairingLocked,
            RelayPairingTransportErrorKind::Unavailable => {
                RelayConnectionErrorCode::RelayUnavailable
            }
            RelayPairingTransportErrorKind::Protocol => RelayConnectionErrorCode::Protocol,
            RelayPairingTransportErrorKind::Cancelled => RelayConnectionErrorCode::Cancelled,
        },
        RelayClientError::Transport(error) => error.safe_error_code(),
    }
}

pub struct RelayClientService<S> {
    manager: RelayConnectionManager<S>,
    states: ConnectionStates,
}

impl<S> RelayClientService<S> {
    pub fn new(credential_store: S) -> Self {
        Self {
            manager: RelayConnectionManager::new(credential_store),
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<S: CredentialStore> RelayClientService<S> {
    pub fn status(&self, account_id: &RelayAccountId) -> RelayConnectionStatus {
        let credential_result = self.manager.has_credential(account_id);
        let mut states = lock_states(&self.states);
        let state = states.entry(account_id.as_str().to_owned()).or_default();
        let credential_stored = match credential_result {
            Ok(stored) => stored,
            Err(error) => {
                state.phase = RelayConnectionPhase::Error;
                state.last_error_code = Some(safe_error_code(&error));
                false
            }
        };
        RelayConnectionStatus {
            account_id: account_id.clone(),
            phase: state.phase,
            credential_stored,
            last_error_code: state.last_error_code,
        }
    }

    pub fn connect<T: RelayTransport>(
        &self,
        account_id: &RelayAccountId,
        device_id: &RelayDeviceId,
        transport: &T,
    ) -> Result<ManagedRelayConnection<T::Connection>, RelayServiceError> {
        {
            let mut states = lock_states(&self.states);
            let state = states.entry(account_id.as_str().to_owned()).or_default();
            if matches!(
                state.phase,
                RelayConnectionPhase::Connecting | RelayConnectionPhase::Connected
            ) {
                return Err(RelayServiceError::AlreadyActive);
            }
            state.phase = RelayConnectionPhase::Connecting;
            state.last_error_code = None;
        }

        match self.manager.connect(account_id, device_id, transport) {
            Ok(connection) => {
                let mut states = lock_states(&self.states);
                let state = states.entry(account_id.as_str().to_owned()).or_default();
                state.phase = RelayConnectionPhase::Connected;
                state.last_error_code = None;
                Ok(ManagedRelayConnection {
                    connection: Some(connection),
                    account_id: account_id.as_str().to_owned(),
                    states: Arc::clone(&self.states),
                })
            }
            Err(error) => {
                let code = safe_error_code(&error);
                let mut states = lock_states(&self.states);
                let state = states.entry(account_id.as_str().to_owned()).or_default();
                state.phase = RelayConnectionPhase::Error;
                state.last_error_code = Some(code);
                Err(RelayServiceError::Client(error))
            }
        }
    }
}

pub struct ManagedRelayConnection<C> {
    connection: Option<C>,
    account_id: String,
    states: ConnectionStates,
}

impl<C> ManagedRelayConnection<C> {
    pub fn connection(&self) -> &C {
        self.connection
            .as_ref()
            .expect("managed relay connection is available before drop")
    }

    pub fn connection_mut(&mut self) -> &mut C {
        self.connection
            .as_mut()
            .expect("managed relay connection is available before drop")
    }
}

impl<C> Drop for ManagedRelayConnection<C> {
    fn drop(&mut self) {
        self.connection.take();
        let mut states = lock_states(&self.states);
        states.insert(self.account_id.clone(), AccountConnectionState::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, cell::RefCell};

    #[derive(Default)]
    struct FakeCredentialStore {
        credential: RefCell<Option<Vec<u8>>>,
        fail_store: Cell<bool>,
        fail_load: Cell<bool>,
    }

    impl CredentialStore for FakeCredentialStore {
        fn store(
            &self,
            _id: &RelayCredentialId,
            credential: &SecretBytes,
        ) -> Result<(), CredentialStoreError> {
            if self.fail_store.get() {
                return Err(CredentialStoreError::Platform {
                    operation: "fake-write",
                    code: 1,
                });
            }
            self.credential.replace(Some(credential.expose().to_vec()));
            Ok(())
        }

        fn load(
            &self,
            _id: &RelayCredentialId,
        ) -> Result<Option<SecretBytes>, CredentialStoreError> {
            if self.fail_load.get() {
                return Err(CredentialStoreError::Platform {
                    operation: "fake-read",
                    code: 2,
                });
            }
            self.credential
                .borrow()
                .as_ref()
                .map(|credential| SecretBytes::new(credential.clone()))
                .transpose()
        }

        fn delete(&self, _id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
            Ok(self.credential.replace(None).is_some())
        }
    }

    #[derive(Default)]
    struct FakeTransport {
        received_credential: RefCell<Option<Vec<u8>>>,
        failure: Cell<Option<RelayTransportErrorKind>>,
    }

    struct FakePairingTransport {
        result: RefCell<Option<Result<PairingResponse, RelayPairingTransportError>>>,
        expected_code: &'static str,
        called: Cell<bool>,
    }

    impl FakePairingTransport {
        fn new(
            expected_code: &'static str,
            result: Result<PairingResponse, RelayPairingTransportError>,
        ) -> Self {
            Self {
                result: RefCell::new(Some(result)),
                expected_code,
                called: Cell::new(false),
            }
        }
    }

    impl RelayPairingTransport for FakePairingTransport {
        fn exchange(
            &self,
            request: RelayPairingRequest<'_>,
        ) -> Result<PairingResponse, RelayPairingTransportError> {
            assert_eq!(request.device_id(), &device_id());
            assert_eq!(request.pairing_code(), self.expected_code);
            self.called.set(true);
            self.result
                .borrow_mut()
                .take()
                .expect("fake pairing transport must be called once")
        }
    }

    impl RelayTransport for FakeTransport {
        type Connection = &'static str;

        fn connect(
            &self,
            request: RelayConnectionRequest<'_>,
        ) -> Result<Self::Connection, RelayTransportError> {
            self.received_credential
                .replace(Some(request.device_credential().to_vec()));
            if let Some(kind) = self.failure.get() {
                return Err(RelayTransportError::new(kind));
            }
            Ok("connected")
        }
    }

    fn account_id() -> RelayAccountId {
        RelayAccountId::new("primary-relay").unwrap()
    }

    fn device_id() -> RelayDeviceId {
        RelayDeviceId::new("moe-desktop").unwrap()
    }

    fn pairing_response(credential: &[u8]) -> PairingResponse {
        PairingResponse::new(device_id(), credential.to_vec()).unwrap()
    }

    #[test]
    fn validates_relay_identity_without_building_targets_in_this_crate() {
        assert_eq!(account_id().as_str(), "primary-relay");
        assert_eq!(device_id().as_str(), "moe-desktop");

        assert_eq!(
            RelayAccountId::new("../other"),
            Err(RelayModelError::InvalidAccountId)
        );
        assert_eq!(
            RelayDeviceId::new("Other/Device"),
            Err(RelayModelError::InvalidDeviceId)
        );
    }

    #[test]
    fn validates_and_redacts_canonical_pairing_code() {
        let code = PairingCode::new("ABCD-EFGH").unwrap();
        assert_eq!(code.expose(), "ABCD-EFGH");
        assert_eq!(format!("{code:?}"), "PairingCode([REDACTED])");

        for invalid in ["", "ABCD-EFGI", "abcd-efgh", "ABCD0EFGH", "ABCD-EFGH2"] {
            assert!(matches!(
                PairingCode::new(invalid),
                Err(RelayModelError::InvalidPairingCode)
            ));
        }
    }

    #[test]
    fn exchanges_pairing_code_and_stores_response_inside_rust() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();
        let transport = FakePairingTransport::new(
            "ABCD-EFGH",
            Ok(PairingResponse::new(device.clone(), b"paired-secret".to_vec()).unwrap()),
        );

        let receipt = manager
            .pair(
                &account,
                &device,
                PairingCode::new("ABCD-EFGH").unwrap(),
                &transport,
            )
            .unwrap();

        assert!(transport.called.get());
        assert_eq!(receipt.account_id(), &account);
        assert_eq!(receipt.device_id(), &device);
        assert_eq!(
            manager.credential_store.credential.borrow().as_deref(),
            Some(b"paired-secret".as_slice())
        );
    }

    #[test]
    fn pairing_transport_failure_does_not_touch_credential_store() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let error = RelayPairingTransportError::new(RelayPairingTransportErrorKind::Expired);
        let transport = FakePairingTransport::new("ABCD-EFGH", Err(error));

        let result = manager.pair(
            &account_id(),
            &device_id(),
            PairingCode::new("ABCD-EFGH").unwrap(),
            &transport,
        );

        assert_eq!(result, Err(RelayClientError::PairingTransport(error)));
        assert!(manager.credential_store.credential.borrow().is_none());
    }

    #[test]
    fn pairing_exchange_rejects_response_for_another_device() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let other_device = RelayDeviceId::new("other-device").unwrap();
        let transport = FakePairingTransport::new(
            "ABCD-EFGH",
            Ok(PairingResponse::new(other_device, b"wrong-device".to_vec()).unwrap()),
        );

        let result = manager.pair(
            &account_id(),
            &device_id(),
            PairingCode::new("ABCD-EFGH").unwrap(),
            &transport,
        );

        assert_eq!(result, Err(RelayClientError::PairingDeviceMismatch));
        assert!(manager.credential_store.credential.borrow().is_none());
    }

    #[test]
    fn redacts_pairing_and_connection_debug_output() {
        let account = account_id();
        let device = device_id();
        let response = pairing_response(b"do-not-log-this");
        let response_debug = format!("{response:?}");
        assert!(response_debug.contains("[REDACTED]"));
        assert!(!response_debug.contains("do-not-log-this"));

        let secret = SecretBytes::new(b"also-private".to_vec()).unwrap();
        let request = RelayConnectionRequest {
            account_id: &account,
            device_id: &device,
            device_credential: &secret,
        };
        let request_debug = format!("{request:?}");
        assert!(request_debug.contains("[REDACTED]"));
        assert!(!request_debug.contains("also-private"));
    }

    #[test]
    fn stores_pairing_and_returns_metadata_only() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();

        let receipt = manager
            .accept_pairing(&account, &device, pairing_response(b"first-credential"))
            .unwrap();

        assert_eq!(receipt.account_id(), &account);
        assert_eq!(receipt.device_id(), &device);
        assert_eq!(
            manager.credential_store.credential.borrow().as_deref(),
            Some(b"first-credential".as_slice())
        );
    }

    #[test]
    fn rejects_pairing_for_another_device_before_storage() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let other_device = RelayDeviceId::new("other-device").unwrap();

        let error = manager
            .accept_pairing(
                &account_id(),
                &device_id(),
                PairingResponse::new(other_device, b"private".to_vec()).unwrap(),
            )
            .unwrap_err();

        assert_eq!(error, RelayClientError::PairingDeviceMismatch);
        assert!(manager.credential_store.credential.borrow().is_none());
    }

    #[test]
    fn failed_repairing_preserves_the_previous_credential() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();
        manager
            .accept_pairing(&account, &device, pairing_response(b"previous"))
            .unwrap();
        manager.credential_store.fail_store.set(true);

        let error = manager
            .accept_pairing(&account, &device, pairing_response(b"replacement"))
            .unwrap_err();

        assert!(matches!(error, RelayClientError::CredentialStore(_)));
        assert_eq!(
            manager.credential_store.credential.borrow().as_deref(),
            Some(b"previous".as_slice())
        );
    }

    #[test]
    fn refuses_connection_when_credential_is_not_stored() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let transport = FakeTransport::default();

        let error = manager
            .connect(&account_id(), &device_id(), &transport)
            .unwrap_err();

        assert_eq!(error, RelayClientError::CredentialNotStored);
        assert!(transport.received_credential.borrow().is_none());
    }

    #[test]
    fn repairing_replaces_the_credential_used_by_transport() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();
        manager
            .accept_pairing(&account, &device, pairing_response(b"previous"))
            .unwrap();
        manager
            .accept_pairing(&account, &device, pairing_response(b"replacement"))
            .unwrap();
        let transport = FakeTransport::default();

        assert_eq!(
            manager.connect(&account, &device, &transport).unwrap(),
            "connected"
        );
        assert_eq!(
            transport.received_credential.borrow().as_deref(),
            Some(b"replacement".as_slice())
        );
    }

    #[test]
    fn reports_safe_transport_failure_without_deleting_credential() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();
        manager
            .accept_pairing(&account, &device, pairing_response(b"still-stored"))
            .unwrap();
        let transport = FakeTransport::default();
        transport
            .failure
            .set(Some(RelayTransportErrorKind::Unavailable));

        let error = manager.connect(&account, &device, &transport).unwrap_err();

        assert_eq!(
            error,
            RelayClientError::Transport(RelayTransportError::new(
                RelayTransportErrorKind::Unavailable
            ))
        );
        assert_eq!(
            manager.credential_store.credential.borrow().as_deref(),
            Some(b"still-stored".as_slice())
        );
    }

    #[test]
    fn deletion_removes_credential_and_is_idempotent() {
        let manager = RelayConnectionManager::new(FakeCredentialStore::default());
        let account = account_id();
        let device = device_id();
        manager
            .accept_pairing(&account, &device, pairing_response(b"temporary"))
            .unwrap();

        assert!(manager.delete_credential(&account).unwrap());
        assert!(!manager.delete_credential(&account).unwrap());
        assert_eq!(
            manager
                .connect(&account, &device, &FakeTransport::default())
                .unwrap_err(),
            RelayClientError::CredentialNotStored
        );
    }

    fn service_with_credential(credential: &[u8]) -> RelayClientService<FakeCredentialStore> {
        let store = FakeCredentialStore::default();
        store.credential.replace(Some(credential.to_vec()));
        RelayClientService::new(store)
    }

    #[test]
    fn service_reports_offline_metadata_without_exposing_credential() {
        let service = service_with_credential(b"service-secret");
        let account = account_id();

        let status = service.status(&account);

        assert_eq!(status.account_id(), &account);
        assert_eq!(status.phase(), RelayConnectionPhase::Offline);
        assert!(status.credential_stored());
        assert_eq!(status.last_error_code(), None);
        assert!(!format!("{status:?}").contains("service-secret"));
    }

    #[test]
    fn managed_connection_updates_lifecycle_and_returns_offline_on_drop() {
        let service = service_with_credential(b"service-secret");
        let account = account_id();
        let device = device_id();
        let transport = FakeTransport::default();

        let mut connection = service.connect(&account, &device, &transport).unwrap();
        assert_eq!(*connection.connection(), "connected");
        assert_eq!(*connection.connection_mut(), "connected");
        assert_eq!(
            service.status(&account).phase(),
            RelayConnectionPhase::Connected
        );

        drop(connection);
        let status = service.status(&account);
        assert_eq!(status.phase(), RelayConnectionPhase::Offline);
        assert!(status.credential_stored());
        assert_eq!(status.last_error_code(), None);
    }

    #[test]
    fn service_rejects_second_connection_for_the_same_account() {
        let service = service_with_credential(b"service-secret");
        let account = account_id();
        let device = device_id();
        let transport = FakeTransport::default();
        let _connection = service.connect(&account, &device, &transport).unwrap();

        let error = match service.connect(&account, &device, &transport) {
            Ok(_) => panic!("second connection unexpectedly succeeded"),
            Err(error) => error,
        };

        assert_eq!(error, RelayServiceError::AlreadyActive);
        assert_eq!(
            service.status(&account).phase(),
            RelayConnectionPhase::Connected
        );
    }

    #[test]
    fn service_reports_missing_credential_as_safe_metadata_error() {
        let service = RelayClientService::new(FakeCredentialStore::default());
        let account = account_id();

        let error = match service.connect(&account, &device_id(), &FakeTransport::default()) {
            Ok(_) => panic!("connection without credential unexpectedly succeeded"),
            Err(error) => error,
        };
        let status = service.status(&account);

        assert_eq!(
            error,
            RelayServiceError::Client(RelayClientError::CredentialNotStored)
        );
        assert_eq!(status.phase(), RelayConnectionPhase::Error);
        assert!(!status.credential_stored());
        assert_eq!(
            status.last_error_code(),
            Some(RelayConnectionErrorCode::CredentialMissing)
        );
    }

    #[test]
    fn service_maps_transport_and_storage_failures_without_platform_detail() {
        let service = service_with_credential(b"service-secret");
        let account = account_id();
        let transport = FakeTransport::default();
        transport
            .failure
            .set(Some(RelayTransportErrorKind::Unavailable));

        let _ = service.connect(&account, &device_id(), &transport);
        assert_eq!(
            service.status(&account).last_error_code(),
            Some(RelayConnectionErrorCode::RelayUnavailable)
        );

        let failing_store = FakeCredentialStore::default();
        failing_store.fail_load.set(true);
        let failing_service = RelayClientService::new(failing_store);
        let status = failing_service.status(&account);
        assert_eq!(status.phase(), RelayConnectionPhase::Error);
        assert_eq!(
            status.last_error_code(),
            Some(RelayConnectionErrorCode::SecureStorageFailed)
        );
    }
}
