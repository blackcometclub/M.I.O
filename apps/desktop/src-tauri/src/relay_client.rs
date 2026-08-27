use moe_credential_store::PlatformCredentialStore;
use moe_relay_client::{
    RelayAccountId, RelayClientService, RelayConnectionErrorCode, RelayConnectionPhase,
    RelayConnectionStatus, RelayRuntimePhase, RelayRuntimeStatus,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

use crate::relay_runtime::DesktopRelayOrchestrator;

pub(crate) type DesktopRelayService = RelayClientService<PlatformCredentialStore>;

pub(crate) fn desktop_relay_service() -> Arc<DesktopRelayService> {
    Arc::new(RelayClientService::new(PlatformCredentialStore))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelayConnectionStatusView {
    account_id: String,
    phase: &'static str,
    credential_stored: bool,
    last_error_code: Option<&'static str>,
    retry_attempt: u8,
    next_retry_delay_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelayConnectionCommandError {
    code: &'static str,
    message: &'static str,
}

fn phase_name(phase: RelayConnectionPhase) -> &'static str {
    match phase {
        RelayConnectionPhase::Offline => "offline",
        RelayConnectionPhase::Connecting => "connecting",
        RelayConnectionPhase::Connected => "connected",
        RelayConnectionPhase::Error => "error",
    }
}

fn runtime_phase_name(phase: RelayRuntimePhase) -> &'static str {
    match phase {
        RelayRuntimePhase::Offline => "offline",
        RelayRuntimePhase::Connecting => "connecting",
        RelayRuntimePhase::Connected => "connected",
        RelayRuntimePhase::RetryWaiting => "retryWaiting",
        RelayRuntimePhase::Stopping => "stopping",
        RelayRuntimePhase::Error => "error",
    }
}

fn error_code_name(code: RelayConnectionErrorCode) -> &'static str {
    match code {
        RelayConnectionErrorCode::CredentialMissing => "credentialMissing",
        RelayConnectionErrorCode::SecureStorageUnavailable => "secureStorageUnavailable",
        RelayConnectionErrorCode::SecureStorageFailed => "secureStorageFailed",
        RelayConnectionErrorCode::PairingRejected => "pairingRejected",
        RelayConnectionErrorCode::PairingExpired => "pairingExpired",
        RelayConnectionErrorCode::PairingUsed => "pairingUsed",
        RelayConnectionErrorCode::PairingLocked => "pairingLocked",
        RelayConnectionErrorCode::RelayRejected => "relayRejected",
        RelayConnectionErrorCode::RelayUnavailable => "relayUnavailable",
        RelayConnectionErrorCode::Protocol => "protocolFailed",
        RelayConnectionErrorCode::Cancelled => "cancelled",
        RelayConnectionErrorCode::AlreadyActive => "alreadyActive",
        RelayConnectionErrorCode::RuntimeUnavailable => "runtimeUnavailable",
    }
}

fn status_view(
    status: RelayConnectionStatus,
    runtime_status: RelayRuntimeStatus,
) -> RelayConnectionStatusView {
    let runtime_is_authoritative = runtime_status.phase() != RelayRuntimePhase::Offline;
    RelayConnectionStatusView {
        account_id: status.account_id().as_str().to_owned(),
        phase: if runtime_is_authoritative {
            runtime_phase_name(runtime_status.phase())
        } else {
            phase_name(status.phase())
        },
        credential_stored: status.credential_stored(),
        last_error_code: if runtime_is_authoritative {
            runtime_status.last_error_code().map(error_code_name)
        } else {
            status.last_error_code().map(error_code_name)
        },
        retry_attempt: runtime_status.retry_attempt(),
        next_retry_delay_ms: runtime_status.next_retry_delay_ms(),
    }
}

fn connection_status(
    service: &DesktopRelayService,
    runtime: &DesktopRelayOrchestrator,
    account_id: String,
) -> Result<RelayConnectionStatusView, RelayConnectionCommandError> {
    let account_id = RelayAccountId::new(account_id).map_err(|_| RelayConnectionCommandError {
        code: "invalidAccountId",
        message: "Relay account ID is invalid.",
    })?;
    Ok(status_view(
        service.status(&account_id),
        runtime.status(&account_id),
    ))
}

#[tauri::command]
pub(crate) fn relay_connection_status(
    service: State<'_, Arc<DesktopRelayService>>,
    runtime: State<'_, DesktopRelayOrchestrator>,
    account_id: String,
) -> Result<RelayConnectionStatusView, RelayConnectionCommandError> {
    connection_status(service.as_ref(), &runtime, account_id)
}

#[cfg(test)]
mod tests {
    use super::{
        connection_status, desktop_relay_service, error_code_name, phase_name, runtime_phase_name,
    };
    use crate::relay_runtime::{DesktopRelayConnectionTaskFactory, desktop_relay_orchestrator};
    use moe_relay_client::{
        RelayAccountId, RelayConnectionErrorCode, RelayConnectionPhase, RelayRuntimePhase,
    };
    use std::sync::Arc;

    #[test]
    fn rejects_invalid_account_before_touching_secure_storage() {
        let error = connection_status(
            &desktop_relay_service(),
            &desktop_relay_orchestrator(),
            "../other-target".into(),
        )
        .unwrap_err();
        assert_eq!(error.code, "invalidAccountId");
    }

    #[test]
    fn exposes_only_stable_metadata_names() {
        assert_eq!(phase_name(RelayConnectionPhase::Connected), "connected");
        assert_eq!(
            runtime_phase_name(RelayRuntimePhase::RetryWaiting),
            "retryWaiting"
        );
        assert_eq!(
            error_code_name(RelayConnectionErrorCode::SecureStorageFailed),
            "secureStorageFailed"
        );
        assert_eq!(
            error_code_name(RelayConnectionErrorCode::RelayUnavailable),
            "relayUnavailable"
        );
    }

    #[test]
    fn runtime_retry_metadata_overrides_idle_service_phase() {
        let service = desktop_relay_service();
        let runtime = desktop_relay_orchestrator();
        let account = RelayAccountId::new("status-runtime-probe").unwrap();
        let factory: DesktopRelayConnectionTaskFactory =
            Arc::new(|| Box::new(|_| RelayConnectionErrorCode::RelayUnavailable));
        runtime.start(&account, factory).unwrap();
        while runtime.poll_next().unwrap().is_none() {}

        let status = connection_status(&service, &runtime, account.as_str().to_owned()).unwrap();

        assert_eq!(status.phase, "retryWaiting");
        assert_eq!(status.retry_attempt, 1);
        assert_eq!(status.next_retry_delay_ms, Some(1_000));
        assert_eq!(status.last_error_code, Some("relayUnavailable"));
        runtime.stop(&account).unwrap();
    }
}
