#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    pub id: String,
    pub display_name: String,
    pub capabilities: Vec<AdapterCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdapterCapability {
    TextInput,
    ImageInput,
    DocumentInput,
    Streaming,
    PersistentSession,
    ToolCalling,
    Approvals,
    Interrupt,
    InboundPush,
}

const MAXIMUM_RELAY_IDENTIFIER_BYTES: usize = 128;
pub const RELAY_READ_ROOM_METHOD: &str = "moe_read_room";
pub const RELAY_MAXIMUM_REQUESTS_PER_CONNECTION: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayFrameError {
    InvalidFrame,
    InvalidRequestId,
    InvalidReadRoomParams,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum RelayInboundFrame {
    #[serde(rename = "request")]
    Request {
        #[serde(rename = "requestId")]
        request_id: String,
        method: String,
        params: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelayRequestFrame {
    request_id: String,
    method: String,
    params: Value,
}

impl RelayRequestFrame {
    pub fn parse(value: Value) -> Result<Self, RelayFrameError> {
        let RelayInboundFrame::Request {
            request_id,
            method,
            params,
        } = serde_json::from_value(value).map_err(|_| RelayFrameError::InvalidFrame)?;
        if !valid_relay_identifier(&request_id) {
            return Err(RelayFrameError::InvalidRequestId);
        }
        Ok(Self {
            request_id,
            method,
            params,
        })
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn read_room_params(&self) -> Result<RelayReadRoomParams, RelayFrameError> {
        RelayReadRoomParams::parse(self.params.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelayReadRoomParams {
    #[serde(default = "default_room_id")]
    room_id: String,
    #[serde(default)]
    after_message_id: Option<String>,
    #[serde(default = "default_room_limit")]
    limit: u8,
}

impl RelayReadRoomParams {
    fn parse(value: Value) -> Result<Self, RelayFrameError> {
        let params: Self =
            serde_json::from_value(value).map_err(|_| RelayFrameError::InvalidReadRoomParams)?;
        if !valid_relay_identifier(&params.room_id)
            || params
                .after_message_id
                .as_deref()
                .is_some_and(|value| !valid_relay_identifier(value))
            || !(1..=30).contains(&params.limit)
        {
            return Err(RelayFrameError::InvalidReadRoomParams);
        }
        Ok(params)
    }

    pub fn room_id(&self) -> &str {
        &self.room_id
    }

    pub fn after_message_id(&self) -> Option<&str> {
        self.after_message_id.as_deref()
    }

    pub fn limit(&self) -> u8 {
        self.limit
    }
}

fn default_room_id() -> String {
    "moe-dev-room".to_owned()
}

fn default_room_limit() -> u8 {
    30
}

fn valid_relay_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAXIMUM_RELAY_IDENTIFIER_BYTES
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayResponseErrorCode {
    UnsupportedMethod,
    InvalidRequest,
    DuplicateRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RelayResponseError<'a> {
    code: RelayResponseErrorCode,
    message: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayResponseFrame<'a, T: Serialize> {
    #[serde(rename = "type")]
    frame_type: &'static str,
    request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RelayResponseError<'a>>,
}

impl<'a, T: Serialize> RelayResponseFrame<'a, T> {
    pub fn success(request_id: &'a str, result: &'a T) -> Self {
        Self {
            frame_type: "response",
            request_id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(request_id: &'a str, code: RelayResponseErrorCode, message: &'a str) -> Self {
        Self {
            frame_type: "response",
            request_id,
            result: None,
            error: Some(RelayResponseError { code, message }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterCapability, AdapterDescriptor, RELAY_READ_ROOM_METHOD, RelayFrameError,
        RelayRequestFrame, RelayResponseErrorCode, RelayResponseFrame,
    };
    use serde_json::json;

    #[test]
    fn descriptor_uses_provider_neutral_json() {
        let descriptor = AdapterDescriptor {
            id: "fake-local".into(),
            display_name: "Fake Local".into(),
            capabilities: vec![AdapterCapability::TextInput, AdapterCapability::Streaming],
        };

        let json = serde_json::to_string(&descriptor).expect("descriptor should serialize");

        assert!(json.contains("textInput"));
        assert!(json.contains("streaming"));
        assert!(!json.contains("codex"));
        assert!(!json.contains("claude"));
    }

    #[test]
    fn parses_strict_bounded_read_room_request() {
        let request = RelayRequestFrame::parse(json!({
            "type": "request",
            "requestId": "request-1",
            "method": RELAY_READ_ROOM_METHOD,
            "params": {"roomId": "moe-dev-room", "afterMessageId": "welcome-2", "limit": 1}
        }))
        .unwrap();
        let params = request.read_room_params().unwrap();
        assert_eq!(request.request_id(), "request-1");
        assert_eq!(params.room_id(), "moe-dev-room");
        assert_eq!(params.after_message_id(), Some("welcome-2"));
        assert_eq!(params.limit(), 1);

        for invalid in [
            json!({"type":"request","requestId":"request-1","method":RELAY_READ_ROOM_METHOD,"params":{"limit":0}}),
            json!({"type":"request","requestId":"request-1","method":RELAY_READ_ROOM_METHOD,"params":{"limit":31}}),
            json!({"type":"request","requestId":"request-1","method":RELAY_READ_ROOM_METHOD,"params":{"path":"C:/private"}}),
        ] {
            let request = RelayRequestFrame::parse(invalid).unwrap();
            assert_eq!(
                request.read_room_params(),
                Err(RelayFrameError::InvalidReadRoomParams)
            );
        }
    }

    #[test]
    fn rejects_malformed_frame_and_serializes_one_response_branch() {
        assert_eq!(
            RelayRequestFrame::parse(json!({
                "type":"request",
                "requestId":"../request",
                "method":RELAY_READ_ROOM_METHOD,
                "params":{},
            })),
            Err(RelayFrameError::InvalidRequestId)
        );
        assert!(
            RelayRequestFrame::parse(json!({
                "type":"request",
                "requestId":"request-1",
                "method":RELAY_READ_ROOM_METHOD,
                "params":{},
                "extra":true,
            }))
            .is_err()
        );

        let result = json!({"ok":true});
        let success =
            serde_json::to_value(RelayResponseFrame::success("request-1", &result)).unwrap();
        assert!(success.get("result").is_some());
        assert!(success.get("error").is_none());
        let error = serde_json::to_value(RelayResponseFrame::<serde_json::Value>::error(
            "request-1",
            RelayResponseErrorCode::UnsupportedMethod,
            "unsupported",
        ))
        .unwrap();
        assert!(error.get("result").is_none());
        assert_eq!(error["error"]["code"], "unsupported_method");
    }
}
