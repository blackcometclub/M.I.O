use moe_credential_store::{
    CredentialStore, CredentialStoreError, PlatformCredentialStore, RelayCredentialId, SecretBytes,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelayCredentialStatus {
    account_id: String,
    stored: bool,
    backend: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CredentialCommandError {
    code: &'static str,
    message: &'static str,
}

impl From<CredentialStoreError> for CredentialCommandError {
    fn from(error: CredentialStoreError) -> Self {
        match error {
            CredentialStoreError::InvalidAccountId => Self {
                code: "invalidAccountId",
                message: "Relay account ID is invalid.",
            },
            CredentialStoreError::InvalidSecret => Self {
                code: "invalidCredential",
                message: "Device credential is invalid.",
            },
            CredentialStoreError::UnsupportedPlatform => Self {
                code: "secureStorageUnavailable",
                message: "Secure credential storage is unavailable on this platform.",
            },
            CredentialStoreError::Platform { .. } => Self {
                code: "secureStorageFailed",
                message: "Secure credential storage could not complete the operation.",
            },
        }
    }
}

fn credential_id(account_id: String) -> Result<RelayCredentialId, CredentialCommandError> {
    RelayCredentialId::new(account_id).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn relay_credential_status(
    account_id: String,
) -> Result<RelayCredentialStatus, CredentialCommandError> {
    let id = credential_id(account_id)?;
    let store = PlatformCredentialStore;
    let stored = store.contains(&id).map_err(CredentialCommandError::from)?;
    Ok(RelayCredentialStatus {
        account_id: id.account_id().to_owned(),
        stored,
        backend: if cfg!(windows) {
            "windowsCredentialManager"
        } else {
            "unavailable"
        },
    })
}

#[allow(dead_code)]
pub(crate) fn store_relay_device_credential(
    account_id: String,
    credential: Vec<u8>,
) -> Result<(), CredentialCommandError> {
    let id = credential_id(account_id)?;
    let credential = SecretBytes::new(credential).map_err(CredentialCommandError::from)?;
    PlatformCredentialStore
        .store(&id, &credential)
        .map_err(Into::into)
}

#[allow(dead_code)]
pub(crate) fn load_relay_device_credential(
    account_id: String,
) -> Result<Option<SecretBytes>, CredentialCommandError> {
    let id = credential_id(account_id)?;
    PlatformCredentialStore.load(&id).map_err(Into::into)
}

#[allow(dead_code)]
pub(crate) fn delete_relay_device_credential(
    account_id: String,
) -> Result<bool, CredentialCommandError> {
    let id = credential_id(account_id)?;
    PlatformCredentialStore.delete(&id).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::relay_credential_status;

    #[test]
    fn rejects_target_injection_before_touching_platform_store() {
        let error = relay_credential_status("../other-target".into()).unwrap_err();
        assert_eq!(error.code, "invalidAccountId");
    }
}
