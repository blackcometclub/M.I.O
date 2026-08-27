#![forbid(unsafe_code)]

use moe_protocol::AdapterDescriptor;
use std::path::{Path, PathBuf};

pub trait AdapterMetadata {
    fn descriptor(&self) -> &AdapterDescriptor;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTurnRequest {
    dispatch_id: String,
    prompt: String,
    workspace: Option<TextTurnWorkspace>,
    continuity: Option<TextTurnContinuity>,
}

impl TextTurnRequest {
    pub fn new(dispatch_id: String, prompt: String) -> Self {
        Self {
            dispatch_id,
            prompt,
            workspace: None,
            continuity: None,
        }
    }

    pub fn with_workspace(mut self, workspace: TextTurnWorkspace) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn with_continuity(mut self, continuity: TextTurnContinuity) -> Self {
        self.continuity = Some(continuity);
        self
    }

    pub fn dispatch_id(&self) -> &str {
        &self.dispatch_id
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn workspace(&self) -> Option<&TextTurnWorkspace> {
        self.workspace.as_ref()
    }

    pub fn continuity(&self) -> Option<&TextTurnContinuity> {
        self.continuity.as_ref()
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
}

impl TextTurnResponse {
    pub fn new(text: String) -> Self {
        Self {
            text,
            session_id: None,
        }
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTurnError {
    Unavailable,
    WorkspaceSandboxUnavailable,
    TimedOut,
    Rejected,
    InvalidResponse,
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
            .with_workspace(TextTurnWorkspace::new(
                PathBuf::from("C:/isolated-workspace"),
                TextTurnWorkspaceAccess::ReadWrite,
            ));
        let workspace = request.workspace().unwrap();

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
}
