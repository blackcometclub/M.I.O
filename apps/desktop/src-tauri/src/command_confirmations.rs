use moe_command_broker::{
    ActionTimeCommand, CommandAuthorization, CommandConfirmationRegistry,
    CommandConfirmationRequest, CommandConfirmationScope, ConfirmationContractError,
    ConfirmationDecision, ConfirmationDenial, ConfirmationRegisterOutcome,
    ConfirmationResolveOutcome,
};
use moe_core::RoomCatalogSource;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::State;

use crate::room_source::DesktopRoomSource;
use crate::room_workspace::{AvailableRoomWorkspace, DesktopRoomWorkspaces};

const COMMAND_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DesktopConfirmationDecision {
    AllowOnce,
    AllowRoomSession,
    Deny,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCommandConfirmationView {
    request_id: String,
    room_id: String,
    action: &'static str,
    reason: &'static str,
    target_label: String,
    room_session_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCommandConfirmationPending {
    ok: bool,
    room_id: String,
    requests: Vec<DesktopCommandConfirmationView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum DesktopConfirmationResolutionStatus {
    AuthorizationReady,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCommandConfirmationResolution {
    ok: bool,
    request_id: String,
    status: DesktopConfirmationResolutionStatus,
    denial: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCommandRoomSessionStatus {
    ok: bool,
    room_id: String,
    active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCommandConfirmationError {
    code: &'static str,
    message: &'static str,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopCommandAuthorizationOutcome {
    Authorized(CommandAuthorization),
    Denied(ConfirmationDenial),
    RoomSessionEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopCommandWaitResolution {
    Resolved(ConfirmationResolveOutcome),
    RoomSessionEnded,
}

#[derive(Debug, Default)]
struct DesktopCommandConfirmationState {
    registry: CommandConfirmationRegistry,
    room_sessions: BTreeMap<String, DesktopRoomCommandSession>,
    waiting_sessions: BTreeMap<String, String>,
    resolutions: BTreeMap<String, DesktopCommandWaitResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DesktopRoomCommandSession {
    room_id: String,
    session_id: String,
    workspace_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopRoomCommandContext {
    room_id: String,
    session_id: String,
    workspace_key: String,
}

impl DesktopRoomCommandContext {
    pub(crate) fn room_id(&self) -> &str {
        &self.room_id
    }

    pub(crate) fn workspace_key(&self) -> &str {
        &self.workspace_key
    }
}

impl DesktopRoomCommandSession {
    fn scope(
        &self,
        command: ActionTimeCommand,
        target_label: String,
    ) -> Result<CommandConfirmationScope, DesktopCommandConfirmationError> {
        CommandConfirmationScope::new(
            self.room_id.clone(),
            self.session_id.clone(),
            self.workspace_key.clone(),
            command,
            target_label,
        )
        .map_err(map_contract_error)
    }
}

#[derive(Debug)]
pub(crate) struct DesktopCommandConfirmations {
    state: Mutex<DesktopCommandConfirmationState>,
    changed: Condvar,
    next_session_sequence: AtomicU64,
    next_request_sequence: AtomicU64,
}

impl Default for DesktopCommandConfirmations {
    fn default() -> Self {
        Self {
            state: Mutex::new(DesktopCommandConfirmationState::default()),
            changed: Condvar::new(),
            next_session_sequence: AtomicU64::new(1),
            next_request_sequence: AtomicU64::new(1),
        }
    }
}

impl DesktopCommandConfirmations {
    fn lock(
        &self,
    ) -> Result<MutexGuard<'_, DesktopCommandConfirmationState>, DesktopCommandConfirmationError>
    {
        self.state.lock().map_err(|_| unavailable())
    }

    pub(crate) fn active_room_context(
        &self,
        room_id: &str,
        workspace: &AvailableRoomWorkspace,
    ) -> Result<Option<DesktopRoomCommandContext>, DesktopCommandConfirmationError> {
        Ok(self.lock()?.room_sessions.get(room_id).and_then(|session| {
            (session.workspace_key == workspace.identity_key()).then(|| DesktopRoomCommandContext {
                room_id: session.room_id.clone(),
                session_id: session.session_id.clone(),
                workspace_key: session.workspace_key.clone(),
            })
        }))
    }

    pub(crate) fn matches_active_context(
        &self,
        context: &DesktopRoomCommandContext,
    ) -> Result<bool, DesktopCommandConfirmationError> {
        Ok(self
            .lock()?
            .room_sessions
            .get(context.room_id())
            .is_some_and(|session| {
                session.room_id == context.room_id
                    && session.session_id == context.session_id
                    && session.workspace_key == context.workspace_key
            }))
    }

    fn ensure_room_session(
        &self,
        room_id: &str,
        workspace: &AvailableRoomWorkspace,
    ) -> Result<DesktopRoomCommandSession, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        if let Some(current) = self.lock()?.room_sessions.get(room_id).cloned() {
            if current.workspace_key == workspace.identity_key() {
                return Ok(current);
            }
        }
        self.begin_room_session(room_id, workspace)
    }

    fn begin_room_session(
        &self,
        room_id: &str,
        workspace: &AvailableRoomWorkspace,
    ) -> Result<DesktopRoomCommandSession, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let sequence = self
            .next_session_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| unavailable())?;
        let session = DesktopRoomCommandSession {
            room_id: room_id.to_owned(),
            session_id: format!("mio-command-session-{}-{sequence}", std::process::id()),
            workspace_key: workspace.identity_key().to_owned(),
        };
        let mut state = self.lock()?;
        let previous = state.room_sessions.remove(room_id);
        let woke_waiters = previous
            .as_ref()
            .is_some_and(|previous| end_session_in_state(&mut state, &previous.session_id).1);
        state
            .room_sessions
            .insert(room_id.to_owned(), session.clone());
        drop(state);
        if woke_waiters {
            self.changed.notify_all();
        }
        Ok(session)
    }

    #[cfg(test)]
    pub(crate) fn begin_room_session_for_test(
        &self,
        room_id: &str,
        workspace: &AvailableRoomWorkspace,
    ) -> Result<(), DesktopCommandConfirmationError> {
        self.begin_room_session(room_id, workspace).map(drop)
    }

    #[allow(dead_code)]
    pub(crate) fn wait_for_room_authorization(
        &self,
        workspaces: &DesktopRoomWorkspaces,
        room_id: &str,
        command: ActionTimeCommand,
        target_label: String,
    ) -> Result<DesktopCommandAuthorizationOutcome, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let session = match self.lock()?.room_sessions.get(room_id).cloned() {
            Some(session) => session,
            None => return Ok(DesktopCommandAuthorizationOutcome::RoomSessionEnded),
        };
        let current_workspace = match workspaces.available_workspace(room_id) {
            Ok(Some(workspace)) => workspace,
            Ok(None) | Err(_) => {
                self.end_room_session(room_id)?;
                return Ok(DesktopCommandAuthorizationOutcome::RoomSessionEnded);
            }
        };
        if current_workspace.identity_key() != session.workspace_key {
            self.end_room_session(room_id)?;
            return Ok(DesktopCommandAuthorizationOutcome::RoomSessionEnded);
        }
        let sequence = self
            .next_request_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| unavailable())?;
        let request = CommandConfirmationRequest::new(
            format!("mio-command-request-{}-{sequence}", std::process::id()),
            session.scope(command, target_label)?,
        )
        .map_err(map_contract_error)?;
        self.wait_for_authorization_for_session(request, COMMAND_CONFIRMATION_TIMEOUT, &session)
    }

    #[cfg(test)]
    pub(crate) fn register(
        &self,
        request: CommandConfirmationRequest,
    ) -> Result<ConfirmationRegisterOutcome, DesktopCommandConfirmationError> {
        self.lock()?
            .registry
            .register(request)
            .map_err(map_contract_error)
    }

    #[allow(dead_code)]
    pub(crate) fn wait_for_authorization(
        &self,
        request: CommandConfirmationRequest,
    ) -> Result<DesktopCommandAuthorizationOutcome, DesktopCommandConfirmationError> {
        self.wait_for_authorization_for(request, COMMAND_CONFIRMATION_TIMEOUT)
    }

    fn wait_for_authorization_for(
        &self,
        request: CommandConfirmationRequest,
        timeout: Duration,
    ) -> Result<DesktopCommandAuthorizationOutcome, DesktopCommandConfirmationError> {
        self.wait_for_authorization_inner(request, timeout, None)
    }

    fn wait_for_authorization_for_session(
        &self,
        request: CommandConfirmationRequest,
        timeout: Duration,
        session: &DesktopRoomCommandSession,
    ) -> Result<DesktopCommandAuthorizationOutcome, DesktopCommandConfirmationError> {
        self.wait_for_authorization_inner(request, timeout, Some(session))
    }

    fn wait_for_authorization_inner(
        &self,
        request: CommandConfirmationRequest,
        timeout: Duration,
        active_session: Option<&DesktopRoomCommandSession>,
    ) -> Result<DesktopCommandAuthorizationOutcome, DesktopCommandConfirmationError> {
        let request_id = request.request_id().to_owned();
        let scope = request.scope().clone();
        let session_id = scope.session_id().to_owned();
        let deadline = Instant::now().checked_add(timeout).ok_or_else(invalid)?;
        let mut state = self.lock()?;
        if active_session.is_some_and(|expected| {
            state.room_sessions.get(&expected.room_id) != Some(expected)
                || request.scope().room_id() != expected.room_id
                || request.scope().session_id() != expected.session_id
                || request.scope().workspace_key() != expected.workspace_key
        }) {
            return Ok(DesktopCommandAuthorizationOutcome::RoomSessionEnded);
        }
        if state.waiting_sessions.contains_key(&request_id) {
            return Err(unavailable());
        }
        match state
            .registry
            .register(request)
            .map_err(map_contract_error)?
        {
            ConfirmationRegisterOutcome::AuthorizedByRoomSession => {
                let authorization = state
                    .registry
                    .consume_authorization(&request_id, &scope)
                    .map_err(map_contract_error)?;
                return Ok(DesktopCommandAuthorizationOutcome::Authorized(
                    authorization,
                ));
            }
            ConfirmationRegisterOutcome::Pending => {
                state
                    .waiting_sessions
                    .insert(request_id.clone(), session_id);
            }
            ConfirmationRegisterOutcome::AlreadyPending => return Err(unavailable()),
        }

        loop {
            if let Some(resolution) = state.resolutions.remove(&request_id) {
                state.waiting_sessions.remove(&request_id);
                return match resolution {
                    DesktopCommandWaitResolution::Resolved(
                        ConfirmationResolveOutcome::AuthorizationReady,
                    ) => state
                        .registry
                        .consume_authorization(&request_id, &scope)
                        .map(DesktopCommandAuthorizationOutcome::Authorized)
                        .map_err(map_contract_error),
                    DesktopCommandWaitResolution::Resolved(ConfirmationResolveOutcome::Denied(
                        denial,
                    )) => Ok(DesktopCommandAuthorizationOutcome::Denied(denial)),
                    DesktopCommandWaitResolution::RoomSessionEnded => {
                        Ok(DesktopCommandAuthorizationOutcome::RoomSessionEnded)
                    }
                };
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                state.waiting_sessions.remove(&request_id);
                let outcome = state
                    .registry
                    .resolve(&request_id, ConfirmationDecision::TimedOut)
                    .map_err(map_contract_error)?;
                return match outcome {
                    ConfirmationResolveOutcome::Denied(denial) => {
                        Ok(DesktopCommandAuthorizationOutcome::Denied(denial))
                    }
                    ConfirmationResolveOutcome::AuthorizationReady => Err(unavailable()),
                };
            }

            let (next_state, _) = self
                .changed
                .wait_timeout(state, remaining)
                .map_err(|_| unavailable())?;
            state = next_state;
        }
    }

    fn pending_for_room(
        &self,
        room_id: &str,
    ) -> Result<DesktopCommandConfirmationPending, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let state = self.lock()?;
        let requests = state
            .registry
            .pending_requests()
            .filter(|request| request.scope().room_id() == room_id)
            .map(|request| view(&request))
            .collect();
        Ok(DesktopCommandConfirmationPending {
            ok: true,
            room_id: room_id.to_owned(),
            requests,
        })
    }

    #[cfg(test)]
    pub(crate) fn pending_request_id_for_room_for_test(
        &self,
        room_id: &str,
    ) -> Result<Option<String>, DesktopCommandConfirmationError> {
        Ok(self
            .pending_for_room(room_id)?
            .requests
            .first()
            .map(|request| request.request_id.clone()))
    }

    #[cfg(test)]
    pub(crate) fn resolve_for_room_for_test(
        &self,
        room_id: &str,
        request_id: &str,
        decision: DesktopConfirmationDecision,
    ) -> Result<(), DesktopCommandConfirmationError> {
        self.resolve_for_room(room_id, request_id, decision)
            .map(drop)
    }

    fn resolve_for_room(
        &self,
        room_id: &str,
        request_id: &str,
        decision: DesktopConfirmationDecision,
    ) -> Result<DesktopCommandConfirmationResolution, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) || !valid_identifier(request_id) {
            return Err(invalid());
        }
        let mut state = self.lock()?;
        let belongs_to_room = state.registry.pending_requests().any(|request| {
            request.request_id() == request_id && request.scope().room_id() == room_id
        });
        if !belongs_to_room {
            return Err(not_pending());
        }
        let outcome = state
            .registry
            .resolve(request_id, map_decision(decision))
            .map_err(map_contract_error)?;
        if state.waiting_sessions.contains_key(request_id) {
            state.resolutions.insert(
                request_id.to_owned(),
                DesktopCommandWaitResolution::Resolved(outcome),
            );
        }
        drop(state);
        self.changed.notify_all();
        Ok(resolution(request_id, outcome))
    }

    #[allow(dead_code)]
    pub(crate) fn timeout(
        &self,
        request_id: &str,
    ) -> Result<ConfirmationResolveOutcome, DesktopCommandConfirmationError> {
        let mut state = self.lock()?;
        let outcome = state
            .registry
            .resolve(request_id, ConfirmationDecision::TimedOut)
            .map_err(map_contract_error)?;
        if state.waiting_sessions.contains_key(request_id) {
            state.resolutions.insert(
                request_id.to_owned(),
                DesktopCommandWaitResolution::Resolved(outcome),
            );
        }
        drop(state);
        self.changed.notify_all();
        Ok(outcome)
    }

    #[allow(dead_code)]
    pub(crate) fn consume_authorization(
        &self,
        request_id: &str,
        expected_scope: &CommandConfirmationScope,
    ) -> Result<CommandAuthorization, DesktopCommandConfirmationError> {
        self.lock()?
            .registry
            .consume_authorization(request_id, expected_scope)
            .map_err(map_contract_error)
    }

    #[allow(dead_code)]
    pub(crate) fn end_room_session(
        &self,
        room_id: &str,
    ) -> Result<usize, DesktopCommandConfirmationError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let mut state = self.lock()?;
        let Some(session) = state.room_sessions.remove(room_id) else {
            return Ok(0);
        };
        let (removed, woke_waiters) = end_session_in_state(&mut state, &session.session_id);
        drop(state);
        if woke_waiters {
            self.changed.notify_all();
        }
        Ok(removed)
    }
}

fn end_session_in_state(
    state: &mut DesktopCommandConfirmationState,
    session_id: &str,
) -> (usize, bool) {
    let waiting_request_ids = state
        .waiting_sessions
        .iter()
        .filter(|(_, waiting_session_id)| waiting_session_id.as_str() == session_id)
        .map(|(request_id, _)| request_id.clone())
        .collect::<Vec<_>>();
    let removed = state.registry.end_room_session(session_id);
    for request_id in &waiting_request_ids {
        state.resolutions.insert(
            request_id.clone(),
            DesktopCommandWaitResolution::RoomSessionEnded,
        );
    }
    (removed, !waiting_request_ids.is_empty())
}

fn view(request: &CommandConfirmationRequest) -> DesktopCommandConfirmationView {
    let scope = request.scope();
    DesktopCommandConfirmationView {
        request_id: request.request_id().to_owned(),
        room_id: scope.room_id().to_owned(),
        action: action_label(scope.command()),
        reason: reason_label(scope.command()),
        target_label: scope.target_label().to_owned(),
        room_session_allowed: scope.allows_room_session(),
    }
}

fn resolution(
    request_id: &str,
    outcome: ConfirmationResolveOutcome,
) -> DesktopCommandConfirmationResolution {
    match outcome {
        ConfirmationResolveOutcome::AuthorizationReady => DesktopCommandConfirmationResolution {
            ok: true,
            request_id: request_id.to_owned(),
            status: DesktopConfirmationResolutionStatus::AuthorizationReady,
            denial: None,
        },
        ConfirmationResolveOutcome::Denied(denial) => DesktopCommandConfirmationResolution {
            ok: true,
            request_id: request_id.to_owned(),
            status: DesktopConfirmationResolutionStatus::Denied,
            denial: Some(denial_label(denial)),
        },
    }
}

fn map_decision(decision: DesktopConfirmationDecision) -> ConfirmationDecision {
    match decision {
        DesktopConfirmationDecision::AllowOnce => ConfirmationDecision::AllowOnce,
        DesktopConfirmationDecision::AllowRoomSession => ConfirmationDecision::AllowRoomSession,
        DesktopConfirmationDecision::Deny => ConfirmationDecision::Deny,
        DesktopConfirmationDecision::Dismissed => ConfirmationDecision::Dismissed,
    }
}

fn action_label(command: ActionTimeCommand) -> &'static str {
    match command {
        ActionTimeCommand::GitPush => "gitPush",
        ActionTimeCommand::PackageInstall => "packageInstall",
        ActionTimeCommand::CargoFetch => "cargoFetch",
        ActionTimeCommand::UnregisteredTool => "unregisteredTool",
        ActionTimeCommand::CredentialUse => "credentialUse",
        ActionTimeCommand::AdministratorOperation => "administratorOperation",
        ActionTimeCommand::DestructiveOperation => "destructiveOperation",
    }
}

fn reason_label(command: ActionTimeCommand) -> &'static str {
    match command.reason() {
        moe_command_broker::ConfirmationReason::ExternalMutation => "externalMutation",
        moe_command_broker::ConfirmationReason::NetworkDownload => "networkDownload",
        moe_command_broker::ConfirmationReason::UnregisteredTool => "unregisteredTool",
        moe_command_broker::ConfirmationReason::CredentialUse => "credentialUse",
        moe_command_broker::ConfirmationReason::PrivilegeExpansion => "privilegeExpansion",
        moe_command_broker::ConfirmationReason::DestructiveChange => "destructiveChange",
    }
}

fn denial_label(denial: ConfirmationDenial) -> &'static str {
    match denial {
        ConfirmationDenial::OwnerDenied => "ownerDenied",
        ConfirmationDenial::Dismissed => "dismissed",
        ConfirmationDenial::TimedOut => "timedOut",
        ConfirmationDenial::SessionLifetimeNotAllowed => "sessionLifetimeNotAllowed",
    }
}

fn map_contract_error(error: ConfirmationContractError) -> DesktopCommandConfirmationError {
    match error {
        ConfirmationContractError::InvalidRequestId | ConfirmationContractError::InvalidScope => {
            invalid()
        }
        ConfirmationContractError::UnknownRequest
        | ConfirmationContractError::RequestAlreadyResolved => not_pending(),
        ConfirmationContractError::RequestIdConflict
        | ConfirmationContractError::ScopeMismatch
        | ConfirmationContractError::TooManyPending
        | ConfirmationContractError::TerminalRegistryFull => unavailable(),
    }
}

fn invalid() -> DesktopCommandConfirmationError {
    DesktopCommandConfirmationError {
        code: "invalidCommandConfirmation",
        message: "The command confirmation request is invalid.",
    }
}

fn not_pending() -> DesktopCommandConfirmationError {
    DesktopCommandConfirmationError {
        code: "commandConfirmationNotPending",
        message: "The command confirmation is no longer pending.",
    }
}

fn unavailable() -> DesktopCommandConfirmationError {
    DesktopCommandConfirmationError {
        code: "commandConfirmationUnavailable",
        message: "Command confirmation is temporarily unavailable.",
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[tauri::command]
pub(crate) fn desktop_command_confirmation_pending(
    confirmations: State<'_, Arc<DesktopCommandConfirmations>>,
    room_id: String,
) -> Result<DesktopCommandConfirmationPending, DesktopCommandConfirmationError> {
    confirmations.pending_for_room(&room_id)
}

#[tauri::command]
pub(crate) fn desktop_command_confirmation_resolve(
    confirmations: State<'_, Arc<DesktopCommandConfirmations>>,
    room_id: String,
    request_id: String,
    decision: DesktopConfirmationDecision,
) -> Result<DesktopCommandConfirmationResolution, DesktopCommandConfirmationError> {
    confirmations.resolve_for_room(&room_id, &request_id, decision)
}

#[tauri::command]
pub(crate) fn desktop_command_room_session_activate(
    source: State<'_, Arc<DesktopRoomSource>>,
    workspaces: State<'_, Arc<DesktopRoomWorkspaces>>,
    confirmations: State<'_, Arc<DesktopCommandConfirmations>>,
    room_id: String,
) -> Result<DesktopCommandRoomSessionStatus, DesktopCommandConfirmationError> {
    if !valid_identifier(&room_id) {
        return Err(invalid());
    }
    let rooms = source.list_rooms().map_err(|_| unavailable())?;
    if !rooms.iter().any(|room| room.id == room_id) {
        return Err(invalid());
    }
    let active = match workspaces
        .available_workspace(&room_id)
        .map_err(|_| unavailable())?
    {
        Some(workspace) => {
            confirmations.ensure_room_session(&room_id, &workspace)?;
            true
        }
        None => {
            confirmations.end_room_session(&room_id)?;
            false
        }
    };
    Ok(DesktopCommandRoomSessionStatus {
        ok: true,
        room_id,
        active,
    })
}

#[tauri::command]
pub(crate) fn desktop_command_room_session_deactivate(
    confirmations: State<'_, Arc<DesktopCommandConfirmations>>,
    room_id: String,
) -> Result<DesktopCommandRoomSessionStatus, DesktopCommandConfirmationError> {
    if !valid_identifier(&room_id) {
        return Err(invalid());
    }
    confirmations.end_room_session(&room_id)?;
    Ok(DesktopCommandRoomSessionStatus {
        ok: true,
        room_id,
        active: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_command_broker::AuthorizationLifetime;
    use std::{env, fs, thread};

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn isolated_root(label: &str) -> std::path::PathBuf {
        let root = env::temp_dir().join(format!(
            "moe-command-confirmation-{label}-{}-{}",
            std::process::id(),
            TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn scope(room_id: &str, command: ActionTimeCommand) -> CommandConfirmationScope {
        CommandConfirmationScope::new(
            room_id.to_owned(),
            "session-1".to_owned(),
            "workspace-secret-key".to_owned(),
            command,
            "origin/main".to_owned(),
        )
        .unwrap()
    }

    fn request(request_id: &str, scope: CommandConfirmationScope) -> CommandConfirmationRequest {
        CommandConfirmationRequest::new(request_id.to_owned(), scope).unwrap()
    }

    fn wait_until_pending(confirmations: &DesktopCommandConfirmations, room_id: &str) {
        for _ in 0..200 {
            if !confirmations
                .pending_for_room(room_id)
                .unwrap()
                .requests
                .is_empty()
            {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("confirmation did not become pending");
    }

    fn pending_request_id(confirmations: &DesktopCommandConfirmations, room_id: &str) -> String {
        for _ in 0..200 {
            let pending = confirmations.pending_for_room(room_id).unwrap();
            if let Some(request) = pending.requests.first() {
                return request.request_id.clone();
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("confirmation did not become pending");
    }

    #[test]
    fn exposes_only_safe_display_fields_for_the_selected_room() {
        let confirmations = DesktopCommandConfirmations::default();
        confirmations
            .register(request(
                "request-1",
                scope("room-1", ActionTimeCommand::GitPush),
            ))
            .unwrap();
        confirmations
            .register(request(
                "request-2",
                scope("room-2", ActionTimeCommand::PackageInstall),
            ))
            .unwrap();

        let pending = confirmations.pending_for_room("room-1").unwrap();
        assert_eq!(pending.requests.len(), 1);
        assert_eq!(pending.requests[0].request_id, "request-1");
        assert_eq!(pending.requests[0].action, "gitPush");
        assert_eq!(pending.requests[0].reason, "externalMutation");
        assert!(pending.requests[0].room_session_allowed);
        let json = serde_json::to_value(pending).unwrap();
        let visible = json.to_string();
        assert!(!visible.contains("session-1"));
        assert!(!visible.contains("workspace-secret-key"));
    }

    #[test]
    fn refuses_to_resolve_a_request_through_another_room() {
        let confirmations = DesktopCommandConfirmations::default();
        confirmations
            .register(request(
                "request-1",
                scope("room-1", ActionTimeCommand::GitPush),
            ))
            .unwrap();

        let error = confirmations
            .resolve_for_room(
                "room-2",
                "request-1",
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap_err();
        assert_eq!(error.code, "commandConfirmationNotPending");
        assert_eq!(
            confirmations
                .pending_for_room("room-1")
                .unwrap()
                .requests
                .len(),
            1
        );
    }

    #[test]
    fn owner_allow_once_still_requires_internal_exact_scope_consumption() {
        let confirmations = DesktopCommandConfirmations::default();
        let scope = scope("room-1", ActionTimeCommand::GitPush);
        confirmations
            .register(request("request-1", scope.clone()))
            .unwrap();

        let resolved = confirmations
            .resolve_for_room(
                "room-1",
                "request-1",
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap();
        assert_eq!(
            resolved.status,
            DesktopConfirmationResolutionStatus::AuthorizationReady
        );
        let authorization = confirmations
            .consume_authorization("request-1", &scope)
            .unwrap();
        assert_eq!(authorization.scope(), &scope);
        assert!(
            confirmations
                .consume_authorization("request-1", &scope)
                .is_err()
        );
    }

    #[test]
    fn high_risk_session_choice_is_returned_as_a_denial() {
        let confirmations = DesktopCommandConfirmations::default();
        confirmations
            .register(request(
                "request-1",
                scope("room-1", ActionTimeCommand::CredentialUse),
            ))
            .unwrap();

        let resolved = confirmations
            .resolve_for_room(
                "room-1",
                "request-1",
                DesktopConfirmationDecision::AllowRoomSession,
            )
            .unwrap();
        assert_eq!(resolved.status, DesktopConfirmationResolutionStatus::Denied);
        assert_eq!(resolved.denial, Some("sessionLifetimeNotAllowed"));
    }

    #[test]
    fn timeout_is_host_owned_and_never_creates_authorization() {
        let confirmations = DesktopCommandConfirmations::default();
        let scope = scope("room-1", ActionTimeCommand::GitPush);
        confirmations
            .register(request("request-1", scope.clone()))
            .unwrap();

        assert_eq!(
            confirmations.timeout("request-1"),
            Ok(ConfirmationResolveOutcome::Denied(
                ConfirmationDenial::TimedOut
            ))
        );
        assert!(
            confirmations
                .consume_authorization("request-1", &scope)
                .is_err()
        );
    }

    #[test]
    fn waiting_host_wakes_and_consumes_the_exact_authorization_once() {
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        let expected_scope = scope("room-1", ActionTimeCommand::GitPush);
        let waiter_confirmations = confirmations.clone();
        let waiter_scope = expected_scope.clone();
        let waiter = thread::spawn(move || {
            waiter_confirmations.wait_for_authorization_for(
                request("request-wait-1", waiter_scope),
                Duration::from_secs(2),
            )
        });
        wait_until_pending(confirmations.as_ref(), "room-1");

        confirmations
            .resolve_for_room(
                "room-1",
                "request-wait-1",
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap();

        let outcome = waiter.join().unwrap().unwrap();
        let DesktopCommandAuthorizationOutcome::Authorized(authorization) = outcome else {
            panic!("expected an authorization");
        };
        assert_eq!(authorization.scope(), &expected_scope);
        assert_eq!(authorization.lifetime(), AuthorizationLifetime::Once);
        assert!(
            confirmations
                .consume_authorization("request-wait-1", &expected_scope)
                .is_err()
        );
    }

    #[test]
    fn waiting_host_returns_owner_denial_without_an_authorization() {
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        let waiter_confirmations = confirmations.clone();
        let waiter = thread::spawn(move || {
            waiter_confirmations.wait_for_authorization_for(
                request(
                    "request-wait-denied",
                    scope("room-1", ActionTimeCommand::GitPush),
                ),
                Duration::from_secs(2),
            )
        });
        wait_until_pending(confirmations.as_ref(), "room-1");

        confirmations
            .resolve_for_room(
                "room-1",
                "request-wait-denied",
                DesktopConfirmationDecision::Deny,
            )
            .unwrap();

        assert_eq!(
            waiter.join().unwrap().unwrap(),
            DesktopCommandAuthorizationOutcome::Denied(ConfirmationDenial::OwnerDenied)
        );
    }

    #[test]
    fn waiting_host_owns_timeout_and_fails_closed() {
        let confirmations = DesktopCommandConfirmations::default();

        assert_eq!(
            confirmations
                .wait_for_authorization_for(
                    request(
                        "request-wait-timeout",
                        scope("room-1", ActionTimeCommand::GitPush),
                    ),
                    Duration::ZERO,
                )
                .unwrap(),
            DesktopCommandAuthorizationOutcome::Denied(ConfirmationDenial::TimedOut)
        );
        assert!(
            confirmations
                .pending_for_room("room-1")
                .unwrap()
                .requests
                .is_empty()
        );
    }

    #[test]
    fn ending_the_room_session_wakes_waiters_without_authorizing() {
        let root = isolated_root("command-session-end");
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let available = workspaces.available_workspace("room-1").unwrap().unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        let session = confirmations
            .begin_room_session("room-1", &available)
            .unwrap();
        let waiter_confirmations = confirmations.clone();
        let waiter = thread::spawn(move || {
            waiter_confirmations.wait_for_authorization_for(
                request(
                    "request-wait-session-end",
                    session
                        .scope(ActionTimeCommand::GitPush, "origin/main".to_owned())
                        .unwrap(),
                ),
                Duration::from_secs(2),
            )
        });
        wait_until_pending(confirmations.as_ref(), "room-1");

        assert!(confirmations.end_room_session("room-1").unwrap() >= 1);

        assert_eq!(
            waiter.join().unwrap().unwrap(),
            DesktopCommandAuthorizationOutcome::RoomSessionEnded
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn another_room_session_does_not_end_the_current_room_session() {
        let root = isolated_root("parallel-room-sessions");
        let first_workspace = root.join("first-workspace");
        let second_workspace = root.join("second-workspace");
        fs::create_dir(&first_workspace).unwrap();
        fs::create_dir(&second_workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", first_workspace).unwrap();
        workspaces.bind("room-2", second_workspace).unwrap();
        let confirmations = DesktopCommandConfirmations::default();

        let first = confirmations
            .begin_room_session(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let second = confirmations
            .begin_room_session(
                "room-2",
                &workspaces.available_workspace("room-2").unwrap().unwrap(),
            )
            .unwrap();

        assert!(
            confirmations
                .matches_active_context(&DesktopRoomCommandContext {
                    room_id: first.room_id,
                    session_id: first.session_id,
                    workspace_key: first.workspace_key,
                })
                .unwrap()
        );
        assert!(
            confirmations
                .matches_active_context(&DesktopRoomCommandContext {
                    room_id: second.room_id,
                    session_id: second.session_id,
                    workspace_key: second.workspace_key,
                })
                .unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reopening_the_same_room_and_workspace_keeps_its_session() {
        let root = isolated_root("stable-room-session");
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let available = workspaces.available_workspace("room-1").unwrap().unwrap();
        let confirmations = DesktopCommandConfirmations::default();

        let first = confirmations
            .ensure_room_session("room-1", &available)
            .unwrap();
        let reopened = confirmations
            .ensure_room_session("room-1", &available)
            .unwrap();

        assert_eq!(reopened, first);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacing_the_workspace_rotates_the_host_owned_room_session() {
        let root = isolated_root("command-session-identity");
        let workspace = root.join("workspace");
        let old_workspace = root.join("old-workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace.clone()).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        let first = confirmations
            .begin_room_session(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let waiter_confirmations = confirmations.clone();
        let first_scope = first
            .scope(ActionTimeCommand::GitPush, "origin/main".to_owned())
            .unwrap();
        let waiter = thread::spawn(move || {
            waiter_confirmations.wait_for_authorization_for(
                request("request-replaced-workspace", first_scope),
                Duration::from_secs(2),
            )
        });
        wait_until_pending(confirmations.as_ref(), "room-1");

        fs::rename(&workspace, &old_workspace).unwrap();
        fs::create_dir(&workspace).unwrap();
        let second = confirmations
            .begin_room_session(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();

        assert_ne!(first.session_id, second.session_id);
        assert_ne!(first.workspace_key, second.workspace_key);
        assert_eq!(first.room_id, second.room_id);
        assert_eq!(
            waiter.join().unwrap().unwrap(),
            DesktopCommandAuthorizationOutcome::RoomSessionEnded
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_room_session_owns_the_request_id_and_exact_scope() {
        let root = isolated_root("host-owned-request");
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace).unwrap();
        let confirmations = Arc::new(DesktopCommandConfirmations::default());
        let session = confirmations
            .begin_room_session(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        let expected_session = session.clone();
        let waiter_confirmations = confirmations.clone();
        let waiter_workspaces = workspaces.clone();
        let waiter = thread::spawn(move || {
            waiter_confirmations.wait_for_room_authorization(
                waiter_workspaces.as_ref(),
                "room-1",
                ActionTimeCommand::GitPush,
                "origin/main".to_owned(),
            )
        });
        let request_id = pending_request_id(confirmations.as_ref(), "room-1");
        assert!(request_id.starts_with("mio-command-request-"));

        confirmations
            .resolve_for_room(
                "room-1",
                &request_id,
                DesktopConfirmationDecision::AllowOnce,
            )
            .unwrap();
        let DesktopCommandAuthorizationOutcome::Authorized(authorization) =
            waiter.join().unwrap().unwrap()
        else {
            panic!("expected an authorization");
        };
        assert_eq!(authorization.scope().room_id(), expected_session.room_id);
        assert_eq!(
            authorization.scope().session_id(),
            expected_session.session_id
        );
        assert_eq!(
            authorization.scope().workspace_key(),
            expected_session.workspace_key
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replaced_workspace_cannot_register_a_new_request_without_reactivation() {
        let root = isolated_root("replaced-workspace-request");
        let workspace = root.join("workspace");
        let old_workspace = root.join("old-workspace");
        fs::create_dir(&workspace).unwrap();
        let workspaces = DesktopRoomWorkspaces::in_memory();
        workspaces.bind("room-1", workspace.clone()).unwrap();
        let confirmations = DesktopCommandConfirmations::default();
        confirmations
            .begin_room_session(
                "room-1",
                &workspaces.available_workspace("room-1").unwrap().unwrap(),
            )
            .unwrap();
        fs::rename(&workspace, &old_workspace).unwrap();
        fs::create_dir(&workspace).unwrap();

        assert_eq!(
            confirmations
                .wait_for_room_authorization(
                    workspaces.as_ref(),
                    "room-1",
                    ActionTimeCommand::GitPush,
                    "origin/main".to_owned(),
                )
                .unwrap(),
            DesktopCommandAuthorizationOutcome::RoomSessionEnded
        );
        assert!(
            confirmations
                .pending_for_room("room-1")
                .unwrap()
                .requests
                .is_empty()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
