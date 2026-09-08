use crate::{ActionTimeCommand, ConfirmationReason};
use std::collections::{BTreeMap, BTreeSet};

const MAXIMUM_CONFIRMATION_ID_BYTES: usize = 128;
const MAXIMUM_SCOPE_ID_BYTES: usize = 256;
const MAXIMUM_TARGET_CHARACTERS: usize = 256;
const MAXIMUM_PENDING_CONFIRMATIONS: usize = 32;
const MAXIMUM_TERMINAL_CONFIRMATIONS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandConfirmationScope {
    room_id: String,
    session_id: String,
    workspace_key: String,
    command: ActionTimeCommand,
    target_label: String,
}

impl CommandConfirmationScope {
    pub fn new(
        room_id: String,
        session_id: String,
        workspace_key: String,
        command: ActionTimeCommand,
        target_label: String,
    ) -> Result<Self, ConfirmationContractError> {
        if !valid_identifier(&room_id, MAXIMUM_SCOPE_ID_BYTES)
            || !valid_identifier(&session_id, MAXIMUM_SCOPE_ID_BYTES)
            || !valid_identifier(&workspace_key, MAXIMUM_SCOPE_ID_BYTES)
            || !valid_target_label(&target_label)
        {
            return Err(ConfirmationContractError::InvalidScope);
        }
        Ok(Self {
            room_id,
            session_id,
            workspace_key,
            command,
            target_label,
        })
    }

    pub fn room_id(&self) -> &str {
        &self.room_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn workspace_key(&self) -> &str {
        &self.workspace_key
    }

    pub fn command(&self) -> ActionTimeCommand {
        self.command
    }

    pub fn target_label(&self) -> &str {
        &self.target_label
    }

    pub fn reason(&self) -> ConfirmationReason {
        self.command.reason()
    }

    pub fn allows_room_session(&self) -> bool {
        self.command.allows_room_session()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandConfirmationRequest {
    request_id: String,
    scope: CommandConfirmationScope,
}

impl CommandConfirmationRequest {
    pub fn new(
        request_id: String,
        scope: CommandConfirmationScope,
    ) -> Result<Self, ConfirmationContractError> {
        if !valid_identifier(&request_id, MAXIMUM_CONFIRMATION_ID_BYTES) {
            return Err(ConfirmationContractError::InvalidRequestId);
        }
        Ok(Self { request_id, scope })
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn scope(&self) -> &CommandConfirmationScope {
        &self.scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationDecision {
    AllowOnce,
    AllowRoomSession,
    Deny,
    Dismissed,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationDenial {
    OwnerDenied,
    Dismissed,
    TimedOut,
    SessionLifetimeNotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationLifetime {
    Once,
    RoomSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAuthorization {
    request_id: String,
    scope: CommandConfirmationScope,
    lifetime: AuthorizationLifetime,
}

impl CommandAuthorization {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn scope(&self) -> &CommandConfirmationScope {
        &self.scope
    }

    pub fn lifetime(&self) -> AuthorizationLifetime {
        self.lifetime
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationRegisterOutcome {
    Pending,
    AlreadyPending,
    AuthorizedByRoomSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResolveOutcome {
    AuthorizationReady,
    Denied(ConfirmationDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationContractError {
    InvalidRequestId,
    InvalidScope,
    RequestIdConflict,
    RequestAlreadyResolved,
    UnknownRequest,
    TooManyPending,
    TerminalRegistryFull,
    ScopeMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadyAuthorization {
    scope: CommandConfirmationScope,
    lifetime: AuthorizationLifetime,
}

#[derive(Debug, Default)]
pub struct CommandConfirmationRegistry {
    pending: BTreeMap<String, CommandConfirmationScope>,
    ready: BTreeMap<String, ReadyAuthorization>,
    terminal: BTreeMap<String, String>,
    room_session_grants: BTreeSet<CommandConfirmationScope>,
}

impl CommandConfirmationRegistry {
    pub fn register(
        &mut self,
        request: CommandConfirmationRequest,
    ) -> Result<ConfirmationRegisterOutcome, ConfirmationContractError> {
        if let Some(existing) = self.pending.get(request.request_id()) {
            return if existing == request.scope() {
                Ok(ConfirmationRegisterOutcome::AlreadyPending)
            } else {
                Err(ConfirmationContractError::RequestIdConflict)
            };
        }
        if self.ready.contains_key(request.request_id())
            || self.terminal.contains_key(request.request_id())
        {
            return Err(ConfirmationContractError::RequestAlreadyResolved);
        }
        if self.terminal.len() >= MAXIMUM_TERMINAL_CONFIRMATIONS {
            return Err(ConfirmationContractError::TerminalRegistryFull);
        }
        if self.pending.len() + self.ready.len() >= MAXIMUM_PENDING_CONFIRMATIONS {
            return Err(ConfirmationContractError::TooManyPending);
        }
        if self.room_session_grants.contains(request.scope()) {
            self.ready.insert(
                request.request_id.clone(),
                ReadyAuthorization {
                    scope: request.scope,
                    lifetime: AuthorizationLifetime::RoomSession,
                },
            );
            return Ok(ConfirmationRegisterOutcome::AuthorizedByRoomSession);
        }
        self.pending.insert(request.request_id, request.scope);
        Ok(ConfirmationRegisterOutcome::Pending)
    }

    pub fn resolve(
        &mut self,
        request_id: &str,
        decision: ConfirmationDecision,
    ) -> Result<ConfirmationResolveOutcome, ConfirmationContractError> {
        if self.terminal.len() >= MAXIMUM_TERMINAL_CONFIRMATIONS {
            return Err(ConfirmationContractError::TerminalRegistryFull);
        }
        if self.ready.contains_key(request_id) || self.terminal.contains_key(request_id) {
            return Err(ConfirmationContractError::RequestAlreadyResolved);
        }
        let scope = self
            .pending
            .remove(request_id)
            .ok_or(ConfirmationContractError::UnknownRequest)?;
        match decision {
            ConfirmationDecision::AllowOnce => {
                self.ready.insert(
                    request_id.to_owned(),
                    ReadyAuthorization {
                        scope,
                        lifetime: AuthorizationLifetime::Once,
                    },
                );
                Ok(ConfirmationResolveOutcome::AuthorizationReady)
            }
            ConfirmationDecision::AllowRoomSession if scope.allows_room_session() => {
                self.room_session_grants.insert(scope.clone());
                self.ready.insert(
                    request_id.to_owned(),
                    ReadyAuthorization {
                        scope,
                        lifetime: AuthorizationLifetime::RoomSession,
                    },
                );
                Ok(ConfirmationResolveOutcome::AuthorizationReady)
            }
            ConfirmationDecision::AllowRoomSession => {
                self.finish_terminal(request_id, scope.session_id())?;
                Ok(ConfirmationResolveOutcome::Denied(
                    ConfirmationDenial::SessionLifetimeNotAllowed,
                ))
            }
            ConfirmationDecision::Deny => {
                self.finish_terminal(request_id, scope.session_id())?;
                Ok(ConfirmationResolveOutcome::Denied(
                    ConfirmationDenial::OwnerDenied,
                ))
            }
            ConfirmationDecision::Dismissed => {
                self.finish_terminal(request_id, scope.session_id())?;
                Ok(ConfirmationResolveOutcome::Denied(
                    ConfirmationDenial::Dismissed,
                ))
            }
            ConfirmationDecision::TimedOut => {
                self.finish_terminal(request_id, scope.session_id())?;
                Ok(ConfirmationResolveOutcome::Denied(
                    ConfirmationDenial::TimedOut,
                ))
            }
        }
    }

    pub fn consume_authorization(
        &mut self,
        request_id: &str,
        expected_scope: &CommandConfirmationScope,
    ) -> Result<CommandAuthorization, ConfirmationContractError> {
        if self.terminal.len() >= MAXIMUM_TERMINAL_CONFIRMATIONS {
            return Err(ConfirmationContractError::TerminalRegistryFull);
        }
        let ready = self
            .ready
            .remove(request_id)
            .ok_or_else(|| self.missing_request_error(request_id))?;
        if &ready.scope != expected_scope {
            self.finish_terminal(request_id, ready.scope.session_id())?;
            return Err(ConfirmationContractError::ScopeMismatch);
        }
        self.finish_terminal(request_id, ready.scope.session_id())?;
        Ok(CommandAuthorization {
            request_id: request_id.to_owned(),
            scope: ready.scope,
            lifetime: ready.lifetime,
        })
    }

    pub fn pending_requests(&self) -> impl Iterator<Item = CommandConfirmationRequest> + '_ {
        self.pending
            .iter()
            .map(|(request_id, scope)| CommandConfirmationRequest {
                request_id: request_id.clone(),
                scope: scope.clone(),
            })
    }

    pub fn end_room_session(&mut self, session_id: &str) -> usize {
        let before = self.pending.len()
            + self.ready.len()
            + self.terminal.len()
            + self.room_session_grants.len();
        self.pending
            .retain(|_, scope| scope.session_id() != session_id);
        self.ready
            .retain(|_, authorization| authorization.scope.session_id() != session_id);
        self.terminal
            .retain(|_, terminal_session_id| terminal_session_id != session_id);
        self.room_session_grants
            .retain(|scope| scope.session_id() != session_id);
        before
            - (self.pending.len()
                + self.ready.len()
                + self.terminal.len()
                + self.room_session_grants.len())
    }

    fn finish_terminal(
        &mut self,
        request_id: &str,
        session_id: &str,
    ) -> Result<(), ConfirmationContractError> {
        if self.terminal.len() >= MAXIMUM_TERMINAL_CONFIRMATIONS {
            return Err(ConfirmationContractError::TerminalRegistryFull);
        }
        self.terminal
            .insert(request_id.to_owned(), session_id.to_owned());
        Ok(())
    }

    fn missing_request_error(&self, request_id: &str) -> ConfirmationContractError {
        if self.terminal.contains_key(request_id) {
            ConfirmationContractError::RequestAlreadyResolved
        } else {
            ConfirmationContractError::UnknownRequest
        }
    }
}

fn valid_identifier(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_target_label(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= MAXIMUM_TARGET_CHARACTERS
        && value.chars().all(|character| !character.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(
        session_id: &str,
        command: ActionTimeCommand,
        target: &str,
    ) -> CommandConfirmationScope {
        CommandConfirmationScope::new(
            "room-1".to_owned(),
            session_id.to_owned(),
            "workspace-a29".to_owned(),
            command,
            target.to_owned(),
        )
        .unwrap()
    }

    fn request(id: &str, scope: CommandConfirmationScope) -> CommandConfirmationRequest {
        CommandConfirmationRequest::new(id.to_owned(), scope).unwrap()
    }

    #[test]
    fn allow_once_requires_exact_scope_and_is_consumed_once() {
        let mut registry = CommandConfirmationRegistry::default();
        let scope = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        assert_eq!(
            registry.register(request("request-1", scope.clone())),
            Ok(ConfirmationRegisterOutcome::Pending)
        );
        assert_eq!(
            registry.resolve("request-1", ConfirmationDecision::AllowOnce),
            Ok(ConfirmationResolveOutcome::AuthorizationReady)
        );
        let authorization = registry.consume_authorization("request-1", &scope).unwrap();
        assert_eq!(authorization.request_id(), "request-1");
        assert_eq!(authorization.scope(), &scope);
        assert_eq!(authorization.lifetime(), AuthorizationLifetime::Once);
        assert_eq!(
            registry.consume_authorization("request-1", &scope),
            Err(ConfirmationContractError::RequestAlreadyResolved)
        );
    }

    #[test]
    fn scope_mismatch_revokes_the_ready_authorization() {
        let mut registry = CommandConfirmationRegistry::default();
        let first_scope = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        let other = scope("session-1", ActionTimeCommand::GitPush, "backup/main");
        registry
            .register(request("request-1", first_scope.clone()))
            .unwrap();
        registry
            .resolve("request-1", ConfirmationDecision::AllowOnce)
            .unwrap();
        assert_eq!(
            registry.consume_authorization("request-1", &other),
            Err(ConfirmationContractError::ScopeMismatch)
        );
        assert_eq!(
            registry.consume_authorization("request-1", &first_scope),
            Err(ConfirmationContractError::RequestAlreadyResolved)
        );
    }

    #[test]
    fn room_session_grant_applies_only_to_the_exact_scope_and_ends_with_the_session() {
        let mut registry = CommandConfirmationRegistry::default();
        let granted = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        registry
            .register(request("request-1", granted.clone()))
            .unwrap();
        registry
            .resolve("request-1", ConfirmationDecision::AllowRoomSession)
            .unwrap();
        registry
            .consume_authorization("request-1", &granted)
            .unwrap();

        assert_eq!(
            registry.register(request("request-2", granted.clone())),
            Ok(ConfirmationRegisterOutcome::AuthorizedByRoomSession)
        );
        assert_eq!(
            registry
                .consume_authorization("request-2", &granted)
                .unwrap()
                .lifetime(),
            AuthorizationLifetime::RoomSession
        );
        let other_target = scope("session-1", ActionTimeCommand::GitPush, "backup/main");
        assert_eq!(
            registry.register(request("request-3", other_target)),
            Ok(ConfirmationRegisterOutcome::Pending)
        );
        assert!(registry.end_room_session("session-1") >= 2);
        assert_eq!(
            registry.register(request("request-4", granted)),
            Ok(ConfirmationRegisterOutcome::Pending)
        );
    }

    #[test]
    fn deny_dismiss_and_timeout_never_create_an_authorization() {
        for (index, decision, denial) in [
            (
                1,
                ConfirmationDecision::Deny,
                ConfirmationDenial::OwnerDenied,
            ),
            (
                2,
                ConfirmationDecision::Dismissed,
                ConfirmationDenial::Dismissed,
            ),
            (
                3,
                ConfirmationDecision::TimedOut,
                ConfirmationDenial::TimedOut,
            ),
        ] {
            let mut registry = CommandConfirmationRegistry::default();
            let request_id = format!("request-{index}");
            let scope = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
            registry
                .register(request(&request_id, scope.clone()))
                .unwrap();
            assert_eq!(
                registry.resolve(&request_id, decision),
                Ok(ConfirmationResolveOutcome::Denied(denial))
            );
            assert_eq!(
                registry.consume_authorization(&request_id, &scope),
                Err(ConfirmationContractError::RequestAlreadyResolved)
            );
        }
    }

    #[test]
    fn high_risk_operations_cannot_receive_a_room_session_grant() {
        for command in [
            ActionTimeCommand::UnregisteredTool,
            ActionTimeCommand::CredentialUse,
            ActionTimeCommand::AdministratorOperation,
            ActionTimeCommand::DestructiveOperation,
        ] {
            let mut registry = CommandConfirmationRegistry::default();
            let scope = scope("session-1", command, "one operation");
            registry
                .register(request("request-1", scope.clone()))
                .unwrap();
            assert_eq!(
                registry.resolve("request-1", ConfirmationDecision::AllowRoomSession),
                Ok(ConfirmationResolveOutcome::Denied(
                    ConfirmationDenial::SessionLifetimeNotAllowed
                ))
            );
            assert_eq!(
                registry.consume_authorization("request-1", &scope),
                Err(ConfirmationContractError::RequestAlreadyResolved)
            );
        }
    }

    #[test]
    fn duplicate_ids_are_idempotent_only_for_the_same_pending_scope() {
        let mut registry = CommandConfirmationRegistry::default();
        let first = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        let conflicting = scope("session-1", ActionTimeCommand::GitPush, "backup/main");
        assert_eq!(
            registry.register(request("request-1", first.clone())),
            Ok(ConfirmationRegisterOutcome::Pending)
        );
        assert_eq!(
            registry.register(request("request-1", first)),
            Ok(ConfirmationRegisterOutcome::AlreadyPending)
        );
        assert_eq!(
            registry.register(request("request-1", conflicting)),
            Err(ConfirmationContractError::RequestIdConflict)
        );
        assert_eq!(registry.pending_requests().count(), 1);
    }

    #[test]
    fn rejects_unsafe_identifiers_and_target_labels() {
        let valid = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        assert_eq!(
            CommandConfirmationRequest::new("request 1".to_owned(), valid),
            Err(ConfirmationContractError::InvalidRequestId)
        );
        for target in ["", "origin/main\nsecret"] {
            assert_eq!(
                CommandConfirmationScope::new(
                    "room-1".to_owned(),
                    "session-1".to_owned(),
                    "workspace-a29".to_owned(),
                    ActionTimeCommand::GitPush,
                    target.to_owned(),
                ),
                Err(ConfirmationContractError::InvalidScope)
            );
        }
    }

    #[test]
    fn bounds_the_number_of_pending_or_ready_confirmations() {
        let mut registry = CommandConfirmationRegistry::default();
        let scope = scope("session-1", ActionTimeCommand::GitPush, "origin/main");
        for index in 0..MAXIMUM_PENDING_CONFIRMATIONS {
            assert_eq!(
                registry.register(request(&format!("request-{index}"), scope.clone())),
                Ok(ConfirmationRegisterOutcome::Pending)
            );
        }
        assert_eq!(
            registry.register(request("request-overflow", scope.clone())),
            Err(ConfirmationContractError::TooManyPending)
        );
        assert!(matches!(
            registry.resolve("request-0", ConfirmationDecision::Deny),
            Ok(ConfirmationResolveOutcome::Denied(_))
        ));
        assert_eq!(
            registry.register(request("request-after-deny", scope)),
            Ok(ConfirmationRegisterOutcome::Pending)
        );
    }
}
