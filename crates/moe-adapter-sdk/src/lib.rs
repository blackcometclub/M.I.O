#![forbid(unsafe_code)]

use moe_protocol::AdapterDescriptor;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Default)]
pub struct TextTurnCancellation {
    cancelled: Arc<AtomicBool>,
}

impl TextTurnCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub trait AdapterMetadata {
    fn descriptor(&self) -> &AdapterDescriptor;
}

#[derive(Debug, Clone)]
pub struct TextTurnRequest {
    dispatch_id: String,
    prompt: String,
    model: Option<String>,
    room_id: Option<String>,
    workspace: Option<TextTurnWorkspace>,
    continuity: Option<TextTurnContinuity>,
    cancellation: TextTurnCancellation,
}

impl TextTurnRequest {
    pub fn new(dispatch_id: String, prompt: String) -> Self {
        Self {
            dispatch_id,
            prompt,
            model: None,
            room_id: None,
            workspace: None,
            continuity: None,
            cancellation: TextTurnCancellation::default(),
        }
    }

    pub fn with_workspace(mut self, workspace: TextTurnWorkspace) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_room_id(mut self, room_id: String) -> Self {
        self.room_id = Some(room_id);
        self
    }

    pub fn with_continuity(mut self, continuity: TextTurnContinuity) -> Self {
        self.continuity = Some(continuity);
        self
    }

    pub fn with_cancellation(mut self, cancellation: TextTurnCancellation) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub fn dispatch_id(&self) -> &str {
        &self.dispatch_id
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn room_id(&self) -> Option<&str> {
        self.room_id.as_deref()
    }

    pub fn workspace(&self) -> Option<&TextTurnWorkspace> {
        self.workspace.as_ref()
    }

    pub fn continuity(&self) -> Option<&TextTurnContinuity> {
        self.continuity.as_ref()
    }

    pub fn cancellation(&self) -> &TextTurnCancellation {
        &self.cancellation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextTurnContinuity {
    StartPersistent,
    Resume { session_id: String },
}

impl TextTurnContinuity {
    pub fn resume(session_id: String) -> Self {
        Self::Resume { session_id }
    }

    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::StartPersistent => None,
            Self::Resume { session_id } => Some(session_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTurnWorkspaceAccess {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTurnWorkspace {
    root: PathBuf,
    access: TextTurnWorkspaceAccess,
}

impl TextTurnWorkspace {
    pub fn new(root: PathBuf, access: TextTurnWorkspaceAccess) -> Self {
        Self { root, access }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn access(&self) -> TextTurnWorkspaceAccess {
        self.access
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTurnResponse {
    text: String,
    session_id: Option<String>,
    artifact_ids: Vec<String>,
}

impl TextTurnResponse {
    pub fn new(text: String) -> Self {
        Self {
            text,
            session_id: None,
            artifact_ids: Vec::new(),
        }
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_artifact_ids(mut self, artifact_ids: Vec<String>) -> Self {
        self.artifact_ids = artifact_ids;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn artifact_ids(&self) -> &[String] {
        &self.artifact_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTurnError {
    Unavailable,
    WorkspaceUnavailable,
    WorkspaceSandboxUnavailable,
    TimedOut,
    Rejected,
    ClientUpdateRequired,
    PreflightFailure,
    ConfirmedFailure,
    InvalidResponse,
    Cancelled,
}

pub trait TextTurnAdapter: AdapterMetadata + Send + Sync {
    fn run_text_turn(&self, request: &TextTurnRequest) -> Result<TextTurnResponse, TextTurnError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use moe_protocol::AdapterCapability;

    struct FakeAdapter {
        descriptor: AdapterDescriptor,
    }

    impl AdapterMetadata for FakeAdapter {
        fn descriptor(&self) -> &AdapterDescriptor {
            &self.descriptor
        }
    }

    impl TextTurnAdapter for FakeAdapter {
        fn run_text_turn(
            &self,
            request: &TextTurnRequest,
        ) -> Result<TextTurnResponse, TextTurnError> {
            Ok(TextTurnResponse::new(format!(
                "{}:{}",
                request.dispatch_id(),
                request.prompt()
            )))
        }
    }

    #[test]
    fn neutral_text_turn_contract_keeps_provider_types_out() {
        let adapter = FakeAdapter {
            descriptor: AdapterDescriptor {
                id: "fake-local".to_owned(),
                display_name: "Fake Local".to_owned(),
                capabilities: vec![AdapterCapability::TextInput],
            },
        };
        let response = adapter
            .run_text_turn(&TextTurnRequest::new(
                "dispatch-1".to_owned(),
                "hello".to_owned(),
            ))
            .unwrap();

        assert_eq!(adapter.descriptor().id, "fake-local");
        assert_eq!(response.text(), "dispatch-1:hello");
    }

    #[test]
    fn carries_a_provider_neutral_workspace_boundary() {
        let request = TextTurnRequest::new("dispatch-2".to_owned(), "inspect".to_owned())
            .with_room_id("room-1".to_owned())
            .with_workspace(TextTurnWorkspace::new(
                PathBuf::from("C:/isolated-workspace"),
                TextTurnWorkspaceAccess::ReadWrite,
            ));
        let workspace = request.workspace().unwrap();

        assert_eq!(request.room_id(), Some("room-1"));
        assert_eq!(workspace.root(), Path::new("C:/isolated-workspace"));
        assert_eq!(workspace.access(), TextTurnWorkspaceAccess::ReadWrite);
    }

    #[test]
    fn carries_provider_neutral_persistent_continuity() {
        let start = TextTurnRequest::new("dispatch-3".to_owned(), "remember".to_owned())
            .with_continuity(TextTurnContinuity::StartPersistent);
        assert_eq!(
            start.continuity(),
            Some(&TextTurnContinuity::StartPersistent)
        );

        let resumed = TextTurnRequest::new("dispatch-4".to_owned(), "continue".to_owned())
            .with_continuity(TextTurnContinuity::resume("session-1".to_owned()));
        assert_eq!(
            resumed.continuity().unwrap().session_id(),
            Some("session-1")
        );

        let response =
            TextTurnResponse::new("done".to_owned()).with_session_id("session-1".to_owned());
        assert_eq!(response.session_id(), Some("session-1"));
    }

    #[test]
    fn carries_an_optional_provider_model_without_defining_provider_catalogs() {
        let default_request =
            TextTurnRequest::new("dispatch-model-default".to_owned(), "hello".to_owned());
        assert_eq!(default_request.model(), None);

        let selected =
            TextTurnRequest::new("dispatch-model-selected".to_owned(), "hello".to_owned())
                .with_model("provider-model-id".to_owned());
        assert_eq!(selected.model(), Some("provider-model-id"));
    }

    #[test]
    fn shares_a_provider_neutral_turn_cancellation_signal() {
        let cancellation = TextTurnCancellation::default();
        let request = TextTurnRequest::new("dispatch-5".to_owned(), "wait".to_owned())
            .with_cancellation(cancellation.clone());

        assert!(!request.cancellation().is_cancelled());
        cancellation.cancel();
        assert!(request.cancellation().is_cancelled());
    }
}
