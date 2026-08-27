#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    error::Error,
    fmt,
    sync::atomic::{Ordering, compiler_fence},
};

const TARGET_PREFIX: &str = "M.O.E./relay-device/v1/";
const MAXIMUM_SECRET_BYTES: usize = 2_560;

fn clear_secret(value: &mut [u8]) {
    for byte in value {
        // SAFETY: byte is a valid unique pointer into the live slice.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    compiler_fence(Ordering::SeqCst);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStoreError {
    InvalidAccountId,
    InvalidSecret,
    UnsupportedPlatform,
    Platform { operation: &'static str, code: i32 },
}

impl fmt::Display for CredentialStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAccountId => formatter.write_str("relay account ID is invalid"),
            Self::InvalidSecret => formatter.write_str("device credential size is invalid"),
            Self::UnsupportedPlatform => {
                formatter.write_str("secure credential storage is unavailable on this platform")
            }
            Self::Platform { operation, code } => {
                write!(
                    formatter,
                    "credential store {operation} failed with OS code {code}"
                )
            }
        }
    }
}

impl Error for CredentialStoreError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayCredentialId {
    account_id: String,
}

impl RelayCredentialId {
    pub fn new(account_id: impl Into<String>) -> Result<Self, CredentialStoreError> {
        let account_id = account_id.into();
        if account_id.is_empty()
            || account_id.len() > 64
            || !account_id
                .bytes()
                .enumerate()
                .all(|(index, value)| match value {
                    b'a'..=b'z' | b'0'..=b'9' => true,
                    b'.' | b'_' | b'-' => index > 0,
                    _ => false,
                })
        {
            return Err(CredentialStoreError::InvalidAccountId);
        }
        Ok(Self { account_id })
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn target_name(&self) -> String {
        format!("{TARGET_PREFIX}{}", self.account_id)
    }
}

pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    pub fn new(mut value: Vec<u8>) -> Result<Self, CredentialStoreError> {
        if value.is_empty() || value.len() > MAXIMUM_SECRET_BYTES {
            clear_secret(&mut value);
            return Err(CredentialStoreError::InvalidSecret);
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretBytes([REDACTED])")
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        clear_secret(&mut self.0);
    }
}

pub trait CredentialStore {
    fn store(
        &self,
        id: &RelayCredentialId,
        credential: &SecretBytes,
    ) -> Result<(), CredentialStoreError>;

    fn load(&self, id: &RelayCredentialId) -> Result<Option<SecretBytes>, CredentialStoreError>;

    fn delete(&self, id: &RelayCredentialId) -> Result<bool, CredentialStoreError>;

    fn contains(&self, id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
        Ok(self.load(id)?.is_some())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformCredentialStore;

#[cfg(windows)]
mod platform {
    use super::{
        CredentialStoreError, MAXIMUM_SECRET_BYTES, RelayCredentialId, SecretBytes, clear_secret,
    };
    use std::{ptr, slice};
    use windows_sys::Win32::{
        Foundation::{ERROR_NOT_FOUND, GetLastError},
        Security::Credentials::{
            CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree,
            CredReadW, CredWriteW,
        },
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn platform_error(operation: &'static str) -> CredentialStoreError {
        CredentialStoreError::Platform {
            operation,
            // SAFETY: GetLastError has no preconditions and is called immediately after failure.
            code: unsafe { GetLastError() } as i32,
        }
    }

    pub fn store(
        id: &RelayCredentialId,
        credential: &SecretBytes,
    ) -> Result<(), CredentialStoreError> {
        let mut target = wide(&id.target_name());
        let mut username = wide("M.O.E. Relay Device");
        let mut blob = credential.expose().to_vec();
        let native = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: username.as_mut_ptr(),
            ..Default::default()
        };

        // SAFETY: pointers reference live buffers and all sizes match for the duration of the call.
        let written = unsafe { CredWriteW(&native, 0) };
        let error = (written == 0).then(|| platform_error("write"));
        clear_secret(&mut blob);
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    pub fn load(id: &RelayCredentialId) -> Result<Option<SecretBytes>, CredentialStoreError> {
        let target = wide(&id.target_name());
        let mut native: *mut CREDENTIALW = ptr::null_mut();
        // SAFETY: target is nul-terminated and native is a valid out pointer.
        let read = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut native) };
        if read == 0 {
            let error = platform_error("read");
            if matches!(
                error,
                CredentialStoreError::Platform { code, .. }
                    if code == ERROR_NOT_FOUND as i32
            ) {
                return Ok(None);
            }
            return Err(error);
        }

        // SAFETY: successful CredReadW returns a valid allocation released by CredFree.
        unsafe {
            let native_ref = &*native;
            let size = native_ref.CredentialBlobSize as usize;
            if size == 0 || size > MAXIMUM_SECRET_BYTES {
                CredFree(native.cast());
                return Err(CredentialStoreError::InvalidSecret);
            }
            let copied = slice::from_raw_parts(native_ref.CredentialBlob, size).to_vec();
            CredFree(native.cast());
            SecretBytes::new(copied).map(Some)
        }
    }

    pub fn contains(id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
        let target = wide(&id.target_name());
        let mut native: *mut CREDENTIALW = ptr::null_mut();
        // SAFETY: target is nul-terminated and native is a valid out pointer.
        let read = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut native) };
        if read != 0 {
            // SAFETY: successful CredReadW returns an allocation released by CredFree.
            unsafe { CredFree(native.cast()) };
            return Ok(true);
        }

        let error = platform_error("read");
        if matches!(
            error,
            CredentialStoreError::Platform { code, .. }
                if code == ERROR_NOT_FOUND as i32
        ) {
            return Ok(false);
        }
        Err(error)
    }

    pub fn delete(id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
        let target = wide(&id.target_name());
        // SAFETY: target is a valid nul-terminated UTF-16 string.
        let deleted = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if deleted != 0 {
            return Ok(true);
        }

        let error = platform_error("delete");
        if matches!(
            error,
            CredentialStoreError::Platform { code, .. }
                if code == ERROR_NOT_FOUND as i32
        ) {
            return Ok(false);
        }
        Err(error)
    }
}

impl CredentialStore for PlatformCredentialStore {
    fn store(
        &self,
        id: &RelayCredentialId,
        credential: &SecretBytes,
    ) -> Result<(), CredentialStoreError> {
        #[cfg(windows)]
        {
            platform::store(id, credential)
        }
        #[cfg(not(windows))]
        {
            let _ = (id, credential);
            Err(CredentialStoreError::UnsupportedPlatform)
        }
    }

    fn load(&self, id: &RelayCredentialId) -> Result<Option<SecretBytes>, CredentialStoreError> {
        #[cfg(windows)]
        {
            platform::load(id)
        }
        #[cfg(not(windows))]
        {
            let _ = id;
            Err(CredentialStoreError::UnsupportedPlatform)
        }
    }

    fn delete(&self, id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
        #[cfg(windows)]
        {
            platform::delete(id)
        }
        #[cfg(not(windows))]
        {
            let _ = id;
            Err(CredentialStoreError::UnsupportedPlatform)
        }
    }

    fn contains(&self, id: &RelayCredentialId) -> Result<bool, CredentialStoreError> {
        #[cfg(windows)]
        {
            platform::contains(id)
        }
        #[cfg(not(windows))]
        {
            let _ = id;
            Err(CredentialStoreError::UnsupportedPlatform)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialStoreError, RelayCredentialId, SecretBytes};

    #[test]
    fn builds_versioned_target_from_restricted_account_id() {
        let id = RelayCredentialId::new("primary-relay_1.prod").unwrap();
        assert_eq!(id.account_id(), "primary-relay_1.prod");
        assert_eq!(
            id.target_name(),
            "M.O.E./relay-device/v1/primary-relay_1.prod"
        );
    }

    #[test]
    fn rejects_target_injection_and_ambiguous_ids() {
        for invalid in ["", "../relay", "/relay", "Relay", "relay/account", " relay"] {
            assert_eq!(
                RelayCredentialId::new(invalid),
                Err(CredentialStoreError::InvalidAccountId)
            );
        }
    }

    #[test]
    fn redacts_secret_debug_output() {
        let secret = SecretBytes::new(b"not-for-debug".to_vec()).unwrap();
        assert_eq!(format!("{secret:?}"), "SecretBytes([REDACTED])");
        assert_eq!(secret.expose(), b"not-for-debug");
    }
}
