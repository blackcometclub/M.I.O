use super::RelayConnectionErrorCode;

pub const RELAY_RETRY_DELAYS_MS: [u64; 5] = [1_000, 2_000, 5_000, 10_000, 30_000];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayRuntimePhase {
    Offline,
    Connecting,
    Connected,
    RetryWaiting,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayLifecycleAction {
    None,
    StartConnection,
    CloseConnection,
    CancelRetry,
    ScheduleRetry { delay_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayLifecycleTransitionError {
    AlreadyActive,
    InvalidTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayRuntimeStatus {
    phase: RelayRuntimePhase,
    retry_attempt: u8,
    next_retry_delay_ms: Option<u64>,
    last_error_code: Option<RelayConnectionErrorCode>,
}

impl RelayRuntimeStatus {
    pub fn phase(self) -> RelayRuntimePhase {
        self.phase
    }

    pub fn retry_attempt(self) -> u8 {
        self.retry_attempt
    }

    pub fn next_retry_delay_ms(self) -> Option<u64> {
        self.next_retry_delay_ms
    }

    pub fn last_error_code(self) -> Option<RelayConnectionErrorCode> {
        self.last_error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayLifecycle {
    status: RelayRuntimeStatus,
}

impl Default for RelayLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayLifecycle {
    pub const fn new() -> Self {
        Self {
            status: RelayRuntimeStatus {
                phase: RelayRuntimePhase::Offline,
                retry_attempt: 0,
                next_retry_delay_ms: None,
                last_error_code: None,
            },
        }
    }

    pub fn status(&self) -> RelayRuntimeStatus {
        self.status
    }

    pub fn start(&mut self) -> Result<RelayLifecycleAction, RelayLifecycleTransitionError> {
        match self.status.phase {
            RelayRuntimePhase::Offline | RelayRuntimePhase::Error => {
                self.status = RelayRuntimeStatus {
                    phase: RelayRuntimePhase::Connecting,
                    retry_attempt: 0,
                    next_retry_delay_ms: None,
                    last_error_code: None,
                };
                Ok(RelayLifecycleAction::StartConnection)
            }
            RelayRuntimePhase::Connecting
            | RelayRuntimePhase::Connected
            | RelayRuntimePhase::RetryWaiting
            | RelayRuntimePhase::Stopping => Err(RelayLifecycleTransitionError::AlreadyActive),
        }
    }

    pub fn connected(&mut self) -> Result<(), RelayLifecycleTransitionError> {
        if self.status.phase != RelayRuntimePhase::Connecting {
            return Err(RelayLifecycleTransitionError::InvalidTransition);
        }

        self.status = RelayRuntimeStatus {
            phase: RelayRuntimePhase::Connected,
            retry_attempt: 0,
            next_retry_delay_ms: None,
            last_error_code: None,
        };
        Ok(())
    }

    pub fn connection_failed(
        &mut self,
        error_code: RelayConnectionErrorCode,
    ) -> Result<RelayLifecycleAction, RelayLifecycleTransitionError> {
        if self.status.phase != RelayRuntimePhase::Connecting {
            return Err(RelayLifecycleTransitionError::InvalidTransition);
        }

        Ok(self.after_connection_loss(error_code))
    }

    pub fn unexpected_disconnect(
        &mut self,
        error_code: RelayConnectionErrorCode,
    ) -> Result<RelayLifecycleAction, RelayLifecycleTransitionError> {
        if self.status.phase != RelayRuntimePhase::Connected {
            return Err(RelayLifecycleTransitionError::InvalidTransition);
        }

        Ok(self.after_connection_loss(error_code))
    }

    pub fn retry_elapsed(&mut self) -> Result<RelayLifecycleAction, RelayLifecycleTransitionError> {
        if self.status.phase != RelayRuntimePhase::RetryWaiting {
            return Err(RelayLifecycleTransitionError::InvalidTransition);
        }

        self.status.phase = RelayRuntimePhase::Connecting;
        self.status.next_retry_delay_ms = None;
        Ok(RelayLifecycleAction::StartConnection)
    }

    pub fn stop(&mut self) -> RelayLifecycleAction {
        let action = match self.status.phase {
            RelayRuntimePhase::Offline => RelayLifecycleAction::None,
            RelayRuntimePhase::Connecting | RelayRuntimePhase::Connected => {
                RelayLifecycleAction::CloseConnection
            }
            RelayRuntimePhase::RetryWaiting => RelayLifecycleAction::CancelRetry,
            RelayRuntimePhase::Stopping => return RelayLifecycleAction::None,
            RelayRuntimePhase::Error => RelayLifecycleAction::None,
        };

        match action {
            RelayLifecycleAction::CloseConnection => {
                self.status.phase = RelayRuntimePhase::Stopping;
                self.status.next_retry_delay_ms = None;
            }
            RelayLifecycleAction::None | RelayLifecycleAction::CancelRetry => {
                self.reset_offline();
            }
            RelayLifecycleAction::StartConnection | RelayLifecycleAction::ScheduleRetry { .. } => {
                unreachable!("stop never starts or schedules a connection")
            }
        }
        action
    }

    pub fn stopped(&mut self) -> Result<(), RelayLifecycleTransitionError> {
        if self.status.phase != RelayRuntimePhase::Stopping {
            return Err(RelayLifecycleTransitionError::InvalidTransition);
        }

        self.reset_offline();
        Ok(())
    }

    pub fn runtime_failed(&mut self) {
        self.status.phase = RelayRuntimePhase::Error;
        self.status.next_retry_delay_ms = None;
        self.status.last_error_code = Some(RelayConnectionErrorCode::RuntimeUnavailable);
    }

    fn after_connection_loss(
        &mut self,
        error_code: RelayConnectionErrorCode,
    ) -> RelayLifecycleAction {
        self.status.last_error_code = Some(error_code);
        self.status.next_retry_delay_ms = None;

        if is_retryable_connection_error(error_code)
            && let Some(delay_ms) = RELAY_RETRY_DELAYS_MS.get(self.status.retry_attempt as usize)
        {
            self.status.phase = RelayRuntimePhase::RetryWaiting;
            self.status.retry_attempt += 1;
            self.status.next_retry_delay_ms = Some(*delay_ms);
            return RelayLifecycleAction::ScheduleRetry {
                delay_ms: *delay_ms,
            };
        }

        self.status.phase = RelayRuntimePhase::Error;
        RelayLifecycleAction::None
    }

    fn reset_offline(&mut self) {
        self.status = RelayRuntimeStatus {
            phase: RelayRuntimePhase::Offline,
            retry_attempt: 0,
            next_retry_delay_ms: None,
            last_error_code: None,
        };
    }
}

fn is_retryable_connection_error(error_code: RelayConnectionErrorCode) -> bool {
    matches!(
        error_code,
        RelayConnectionErrorCode::RelayUnavailable | RelayConnectionErrorCode::Protocol
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_connect_stop_and_stopped_form_the_owned_handle_path() {
        let mut lifecycle = RelayLifecycle::new();
        assert_eq!(lifecycle.start(), Ok(RelayLifecycleAction::StartConnection));
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Connecting);
        lifecycle.connected().unwrap();
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Connected);

        assert_eq!(lifecycle.stop(), RelayLifecycleAction::CloseConnection);
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Stopping);
        lifecycle.stopped().unwrap();
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Offline);
    }

    #[test]
    fn active_lifecycle_rejects_duplicate_start() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();

        assert_eq!(
            lifecycle.start(),
            Err(RelayLifecycleTransitionError::AlreadyActive)
        );
    }

    #[test]
    fn retry_schedule_is_bounded_and_reaches_terminal_error() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();

        for (index, delay_ms) in RELAY_RETRY_DELAYS_MS.iter().copied().enumerate() {
            assert_eq!(
                lifecycle.connection_failed(RelayConnectionErrorCode::RelayUnavailable),
                Ok(RelayLifecycleAction::ScheduleRetry { delay_ms })
            );
            let status = lifecycle.status();
            assert_eq!(status.phase(), RelayRuntimePhase::RetryWaiting);
            assert_eq!(status.retry_attempt(), (index + 1) as u8);
            assert_eq!(status.next_retry_delay_ms(), Some(delay_ms));
            assert_eq!(
                lifecycle.retry_elapsed(),
                Ok(RelayLifecycleAction::StartConnection)
            );
        }

        assert_eq!(
            lifecycle.connection_failed(RelayConnectionErrorCode::RelayUnavailable),
            Ok(RelayLifecycleAction::None)
        );
        let status = lifecycle.status();
        assert_eq!(status.phase(), RelayRuntimePhase::Error);
        assert_eq!(status.retry_attempt(), RELAY_RETRY_DELAYS_MS.len() as u8);
        assert_eq!(status.next_retry_delay_ms(), None);
    }

    #[test]
    fn successful_retry_clears_retry_metadata() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();
        lifecycle
            .connection_failed(RelayConnectionErrorCode::Protocol)
            .unwrap();
        lifecycle.retry_elapsed().unwrap();
        lifecycle.connected().unwrap();

        let status = lifecycle.status();
        assert_eq!(status.phase(), RelayRuntimePhase::Connected);
        assert_eq!(status.retry_attempt(), 0);
        assert_eq!(status.next_retry_delay_ms(), None);
        assert_eq!(status.last_error_code(), None);
    }

    #[test]
    fn manual_stop_cancels_waiting_retry_and_resets_state() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();
        lifecycle
            .connection_failed(RelayConnectionErrorCode::RelayUnavailable)
            .unwrap();

        assert_eq!(lifecycle.stop(), RelayLifecycleAction::CancelRetry);
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::Offline);
        assert_eq!(lifecycle.status().retry_attempt(), 0);
        assert_eq!(lifecycle.status().last_error_code(), None);
    }

    #[test]
    fn authentication_and_credential_failures_never_retry() {
        for error_code in [
            RelayConnectionErrorCode::CredentialMissing,
            RelayConnectionErrorCode::SecureStorageUnavailable,
            RelayConnectionErrorCode::SecureStorageFailed,
            RelayConnectionErrorCode::RelayRejected,
            RelayConnectionErrorCode::Cancelled,
        ] {
            let mut lifecycle = RelayLifecycle::new();
            lifecycle.start().unwrap();

            assert_eq!(
                lifecycle.connection_failed(error_code),
                Ok(RelayLifecycleAction::None)
            );
            let status = lifecycle.status();
            assert_eq!(status.phase(), RelayRuntimePhase::Error);
            assert_eq!(status.retry_attempt(), 0);
            assert_eq!(status.next_retry_delay_ms(), None);
            assert_eq!(status.last_error_code(), Some(error_code));
        }
    }

    #[test]
    fn unexpected_disconnect_uses_the_same_bounded_retry_contract() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();
        lifecycle.connected().unwrap();

        assert_eq!(
            lifecycle.unexpected_disconnect(RelayConnectionErrorCode::RelayUnavailable),
            Ok(RelayLifecycleAction::ScheduleRetry { delay_ms: 1_000 })
        );
        assert_eq!(lifecycle.status().phase(), RelayRuntimePhase::RetryWaiting);
    }

    #[test]
    fn out_of_order_runtime_events_are_rejected() {
        let mut lifecycle = RelayLifecycle::new();

        assert_eq!(
            lifecycle.connected(),
            Err(RelayLifecycleTransitionError::InvalidTransition)
        );
        assert_eq!(
            lifecycle.retry_elapsed(),
            Err(RelayLifecycleTransitionError::InvalidTransition)
        );
        assert_eq!(
            lifecycle.unexpected_disconnect(RelayConnectionErrorCode::RelayUnavailable),
            Err(RelayLifecycleTransitionError::InvalidTransition)
        );
    }

    #[test]
    fn runtime_failure_is_terminal_and_safe_to_restart() {
        let mut lifecycle = RelayLifecycle::new();
        lifecycle.start().unwrap();
        lifecycle.runtime_failed();

        let status = lifecycle.status();
        assert_eq!(status.phase(), RelayRuntimePhase::Error);
        assert_eq!(status.next_retry_delay_ms(), None);
        assert_eq!(
            status.last_error_code(),
            Some(RelayConnectionErrorCode::RuntimeUnavailable)
        );
        assert_eq!(lifecycle.start(), Ok(RelayLifecycleAction::StartConnection));
    }
}
