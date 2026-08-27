#![forbid(unsafe_code)]

use moe_core::{
    RoomCatalogSource, RoomMessageDraft, RoomMessageProvenance, RoomParticipantKind, RoomReadQuery,
    RoomReadResult, RoomSource, RoomStore, RoomSummary, RoomWriteError, RoomWriteSuccess,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const DEFAULT_ROOM_READ_LIMIT: u8 = 30;
const OWNER_PROXY_MESSAGE_ID_PREFIX: &str = "mcp-owner-";
pub const MCP_API_VERSION: &str = "mio.mcp.v1";

pub const MIO_STATUS_TOOL: &str = "mio_status";
pub const MIO_ROOM_LIST_TOOL: &str = "mio_room_list";
pub const MIO_ROOM_READ_TOOL: &str = "mio_room_read";
pub const MIO_ROOM_POST_AS_OWNER_TOOL: &str = "mio_room_post_as_owner";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MioStatusOutput {
    pub ok: bool,
    pub server_name: &'static str,
    pub api_version: &'static str,
    pub ready: bool,
    pub capabilities: [&'static str; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MioRoomListOutput {
    pub ok: bool,
    pub rooms: Vec<RoomSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MioReadToolError {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MioWriteToolError {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MioRoomReadInput {
    pub room_id: String,
    #[serde(default)]
    pub after_message_id: Option<String>,
    #[serde(default)]
    pub limit: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MioRoomPostAsOwnerInput {
    pub request_id: String,
    pub room_id: String,
    pub recipient_ids: Vec<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MioToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub read_only: bool,
}

pub struct MioTools<R> {
    source: Arc<R>,
}

impl<R> MioTools<R> {
    pub fn new(source: Arc<R>) -> Self {
        Self { source }
    }
}

impl<R> MioTools<R>
where
    R: RoomSource + RoomCatalogSource,
{
    pub fn status(&self) -> MioStatusOutput {
        let ready = self.source.list_rooms().is_ok();
        MioStatusOutput {
            ok: ready,
            server_name: "mio",
            api_version: MCP_API_VERSION,
            ready,
            capabilities: [
                MIO_STATUS_TOOL,
                MIO_ROOM_LIST_TOOL,
                MIO_ROOM_READ_TOOL,
                MIO_ROOM_POST_AS_OWNER_TOOL,
            ],
        }
    }

    pub fn room_list(&self) -> Result<MioRoomListOutput, MioReadToolError> {
        self.source
            .list_rooms()
            .map(|rooms| MioRoomListOutput { ok: true, rooms })
            .map_err(|_| MioReadToolError {
                code: "roomCatalogUnavailable",
                message: "The Room catalog is temporarily unavailable.",
            })
    }

    pub fn room_read(&self, input: MioRoomReadInput) -> Result<RoomReadResult, MioReadToolError> {
        let query = RoomReadQuery::try_new(
            input.room_id,
            input.after_message_id,
            input.limit.unwrap_or(DEFAULT_ROOM_READ_LIMIT),
        )
        .map_err(|_| MioReadToolError {
            code: "invalidRoomRequest",
            message: "The Room read request is invalid.",
        })?;
        Ok(self.source.read_room(&query))
    }
}

impl<R> MioTools<R>
where
    R: RoomSource + RoomStore,
{
    pub fn room_post_as_owner(
        &self,
        input: MioRoomPostAsOwnerInput,
        created_at: String,
    ) -> Result<RoomWriteSuccess, MioWriteToolError> {
        if !valid_request_id(&input.request_id) {
            return Err(invalid_owner_proxy_message());
        }
        let owner_id = resolve_room_owner(
            self.source.read_room(
                &RoomReadQuery::try_new(input.room_id.clone(), None, 1)
                    .map_err(|_| invalid_owner_proxy_message())?,
            ),
        )?;
        let draft = RoomMessageDraft::try_new_with_provenance(
            format!("{OWNER_PROXY_MESSAGE_ID_PREFIX}{}", input.request_id),
            input.room_id,
            owner_id,
            input.recipient_ids,
            input.body,
            created_at,
            Vec::new(),
            Some(RoomMessageProvenance::CodexOwnerProxy),
        )
        .map_err(|_| invalid_owner_proxy_message())?;
        self.source.append_message(draft).map_err(map_write_error)
    }
}

fn resolve_room_owner(result: RoomReadResult) -> Result<String, MioWriteToolError> {
    let mut owners = result
        .participants()
        .ok_or_else(room_owner_unavailable)?
        .iter()
        .filter(|participant| participant.kind == RoomParticipantKind::Human)
        .map(|participant| participant.id.clone());
    let owner = owners.next().ok_or_else(room_owner_unavailable)?;
    if owners.next().is_some() {
        return Err(room_owner_unavailable());
    }
    Ok(owner)
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 118
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

fn invalid_owner_proxy_message() -> MioWriteToolError {
    MioWriteToolError {
        code: "invalidRoomMessage",
        message: "The Owner-proxy Room message is invalid.",
    }
}

fn room_owner_unavailable() -> MioWriteToolError {
    MioWriteToolError {
        code: "roomOwnerUnavailable",
        message: "The Room does not have exactly one human Owner.",
    }
}

fn map_write_error(error: RoomWriteError) -> MioWriteToolError {
    match error {
        RoomWriteError::RoomNotFound => MioWriteToolError {
            code: "roomNotFound",
            message: "The Room is not available.",
        },
        RoomWriteError::AuthorNotParticipant | RoomWriteError::RecipientNotParticipant => {
            MioWriteToolError {
                code: "roomParticipantInvalid",
                message: "A message participant is not available in the Room.",
            }
        }
        RoomWriteError::IdempotencyConflict => MioWriteToolError {
            code: "messageConflict",
            message: "The request ID was already used for different content.",
        },
        RoomWriteError::RoomCapacityReached => MioWriteToolError {
            code: "roomCapacityReached",
            message: "The Room cannot accept more messages.",
        },
        RoomWriteError::SourceUnavailable => MioWriteToolError {
            code: "roomWriteUnavailable",
            message: "The Room is temporarily unavailable.",
        },
    }
}

pub fn tool_descriptors() -> [MioToolDescriptor; 4] {
    [
        MioToolDescriptor {
            name: MIO_STATUS_TOOL,
            description: "Return the local M.I.O. MCP readiness and available capabilities.",
            input_schema: empty_input_schema(),
            read_only: true,
        },
        MioToolDescriptor {
            name: MIO_ROOM_LIST_TOOL,
            description: "List the Rooms available in the running M.I.O. desktop app.",
            input_schema: empty_input_schema(),
            read_only: true,
        },
        MioToolDescriptor {
            name: MIO_ROOM_READ_TOOL,
            description: "Read one bounded page of messages and participants from a Room.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "roomId": { "type": "string", "minLength": 1 },
                    "afterMessageId": { "type": "string", "minLength": 1 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 30 }
                },
                "required": ["roomId"],
                "additionalProperties": false
            }),
            read_only: true,
        },
        MioToolDescriptor {
            name: MIO_ROOM_POST_AS_OWNER_TOOL,
            description: "Save one Owner-proxy message with immutable via-Codex provenance. This does not dispatch AI replies or start a conductor.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "requestId": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 118,
                        "pattern": "^[A-Za-z0-9][A-Za-z0-9._-]*$"
                    },
                    "roomId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "recipientIds": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 100,
                        "uniqueItems": true,
                        "items": { "type": "string", "minLength": 1, "maxLength": 128 }
                    },
                    "body": { "type": "string", "minLength": 1, "maxLength": 4000 }
                },
                "required": ["requestId", "roomId", "recipientIds", "body"],
                "additionalProperties": false
            }),
            read_only: false,
        },
    ]
}

fn empty_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::{
        MIO_ROOM_LIST_TOOL, MIO_ROOM_POST_AS_OWNER_TOOL, MIO_ROOM_READ_TOOL, MIO_STATUS_TOOL,
        MioRoomPostAsOwnerInput, MioRoomReadInput, MioTools, tool_descriptors,
    };
    use moe_core::{
        InMemoryRoomSource, Room, RoomMessage, RoomMessageProvenance, RoomParticipant,
        RoomParticipantKind, RoomSnapshot, RoomWriteStatus,
    };
    use std::sync::Arc;

    fn source() -> Arc<InMemoryRoomSource> {
        Arc::new(
            InMemoryRoomSource::new(RoomSnapshot {
                schema_version: moe_protocol::PROTOCOL_VERSION.to_owned(),
                generated_at: "2026-08-14T00:00:00Z".to_owned(),
                participants: vec![
                    RoomParticipant {
                        id: "local-owner".to_owned(),
                        display_name: "Sample Owner".to_owned(),
                        kind: RoomParticipantKind::Human,
                    },
                    RoomParticipant {
                        id: "codex".to_owned(),
                        display_name: "Codex".to_owned(),
                        kind: RoomParticipantKind::Ai,
                    },
                ],
                rooms: vec![Room {
                    id: "dev-room".to_owned(),
                    name: "M.I.O. Dev Test".to_owned(),
                    participant_ids: vec!["local-owner".to_owned(), "codex".to_owned()],
                    messages: vec![
                        message("message-1", "first"),
                        message("message-2", "second"),
                        message("message-3", "third"),
                    ],
                }],
            })
            .unwrap(),
        )
    }

    fn message(id: &str, body: &str) -> RoomMessage {
        RoomMessage {
            id: id.to_owned(),
            room_id: "dev-room".to_owned(),
            author_id: "local-owner".to_owned(),
            recipients: vec!["codex".to_owned()],
            body: body.to_owned(),
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            artifact_ids: Vec::new(),
            provenance: None,
        }
    }

    #[test]
    fn advertises_three_reads_and_one_owner_proxy_write() {
        let descriptors = serde_json::to_value(tool_descriptors()).unwrap();
        let names = descriptors
            .as_array()
            .unwrap()
            .iter()
            .map(|descriptor| descriptor["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                MIO_STATUS_TOOL,
                MIO_ROOM_LIST_TOOL,
                MIO_ROOM_READ_TOOL,
                MIO_ROOM_POST_AS_OWNER_TOOL,
            ]
        );
        assert_eq!(
            descriptors[2]["inputSchema"]["properties"]["limit"]["maximum"],
            30
        );
        assert_eq!(descriptors[2]["inputSchema"]["additionalProperties"], false);
        assert_eq!(descriptors[2]["readOnly"], true);
        assert_eq!(descriptors[3]["readOnly"], false);
        assert_eq!(
            descriptors[3]["inputSchema"]["required"],
            serde_json::json!(["requestId", "roomId", "recipientIds", "body"])
        );
    }

    #[test]
    fn reports_readiness_and_lists_the_bounded_room_catalog() {
        let tools = MioTools::new(source());
        let status = serde_json::to_value(tools.status()).unwrap();
        assert_eq!(status["ok"], true);
        assert_eq!(status["serverName"], "mio");
        assert_eq!(status["apiVersion"], "mio.mcp.v1");
        assert_eq!(status["capabilities"].as_array().unwrap().len(), 4);

        let catalog = serde_json::to_value(tools.room_list().unwrap()).unwrap();
        assert_eq!(catalog["ok"], true);
        assert_eq!(catalog["rooms"].as_array().unwrap().len(), 1);
        assert_eq!(catalog["rooms"][0]["id"], "dev-room");
    }

    #[test]
    fn reads_a_room_with_the_existing_bounded_core_query() {
        let tools = MioTools::new(source());
        let result = tools
            .room_read(MioRoomReadInput {
                room_id: "dev-room".to_owned(),
                after_message_id: None,
                limit: Some(2),
            })
            .unwrap();
        let result = serde_json::to_value(result).unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["room"]["id"], "dev-room");
        assert_eq!(result["room"]["messages"].as_array().unwrap().len(), 2);
        assert_eq!(result["page"]["limit"], 2);
        assert_eq!(result["page"]["hasMoreBefore"], true);
    }

    #[test]
    fn rejects_unbounded_or_shape_invalid_room_reads() {
        let tools = MioTools::new(source());
        let error = tools
            .room_read(MioRoomReadInput {
                room_id: "dev-room".to_owned(),
                after_message_id: None,
                limit: Some(31),
            })
            .unwrap_err();
        let error = serde_json::to_value(error).unwrap();
        assert_eq!(error["code"], "invalidRoomRequest");

        let error = serde_json::from_value::<MioRoomReadInput>(serde_json::json!({
            "roomId": "dev-room",
            "unexpected": true
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn returns_a_safe_structured_failure_for_an_unknown_room() {
        let tools = MioTools::new(source());
        let result = tools
            .room_read(MioRoomReadInput {
                room_id: "missing-room".to_owned(),
                after_message_id: None,
                limit: None,
            })
            .unwrap();
        let result = serde_json::to_value(result).unwrap();
        assert_eq!(result["ok"], false);
        assert_eq!(result["code"], "room_not_found");
        assert_eq!(result["message"], "The requested room is not available.");
    }

    #[test]
    fn posts_one_owner_proxy_message_idempotently_with_immutable_provenance() {
        let source = source();
        let tools = MioTools::new(source.clone());
        let input = || MioRoomPostAsOwnerInput {
            request_id: "request-1".to_owned(),
            room_id: "dev-room".to_owned(),
            recipient_ids: vec!["codex".to_owned()],
            body: "Please inspect this Room.".to_owned(),
        };

        let appended = tools
            .room_post_as_owner(input(), "2026-08-17T00:00:00Z".to_owned())
            .unwrap();
        assert_eq!(appended.status(), RoomWriteStatus::Appended);
        assert_eq!(appended.message().id, "mcp-owner-request-1");
        assert_eq!(appended.message().author_id, "local-owner");
        assert_eq!(appended.message().recipients, ["codex"]);
        assert_eq!(
            appended.message().provenance,
            Some(RoomMessageProvenance::CodexOwnerProxy)
        );

        let duplicate = tools
            .room_post_as_owner(input(), "2026-08-17T00:01:00Z".to_owned())
            .unwrap();
        assert_eq!(duplicate.status(), RoomWriteStatus::Duplicate);
        assert_eq!(duplicate.message().created_at, "2026-08-17T00:00:00Z");

        let conflict = tools
            .room_post_as_owner(
                MioRoomPostAsOwnerInput {
                    body: "Different content.".to_owned(),
                    ..input()
                },
                "2026-08-17T00:02:00Z".to_owned(),
            )
            .unwrap_err();
        assert_eq!(conflict.code, "messageConflict");

        let snapshot = source.snapshot().unwrap();
        assert_eq!(snapshot.rooms[0].messages.len(), 4);
    }

    #[test]
    fn refuses_rooms_without_exactly_one_human_owner() {
        let input = || MioRoomPostAsOwnerInput {
            request_id: "owner-check".to_owned(),
            room_id: "dev-room".to_owned(),
            recipient_ids: vec!["codex".to_owned()],
            body: "body".to_owned(),
        };

        let mut no_owner = source().snapshot().unwrap();
        no_owner.participants[0].kind = RoomParticipantKind::Ai;
        let tools = MioTools::new(Arc::new(InMemoryRoomSource::new(no_owner).unwrap()));
        assert_eq!(
            tools
                .room_post_as_owner(input(), "2026-08-17T00:00:00Z".to_owned())
                .unwrap_err()
                .code,
            "roomOwnerUnavailable"
        );

        let mut ambiguous_owner = source().snapshot().unwrap();
        ambiguous_owner.participants.push(RoomParticipant {
            id: "second-owner".to_owned(),
            display_name: "Second Owner".to_owned(),
            kind: RoomParticipantKind::Human,
        });
        ambiguous_owner.rooms[0]
            .participant_ids
            .push("second-owner".to_owned());
        let tools = MioTools::new(Arc::new(InMemoryRoomSource::new(ambiguous_owner).unwrap()));
        assert_eq!(
            tools
                .room_post_as_owner(input(), "2026-08-17T00:00:00Z".to_owned())
                .unwrap_err()
                .code,
            "roomOwnerUnavailable"
        );
    }

    #[test]
    fn rejects_invalid_owner_proxy_shapes_and_participants() {
        let tools = MioTools::new(source());
        let invalid_request = MioRoomPostAsOwnerInput {
            request_id: "unsafe/request".to_owned(),
            room_id: "dev-room".to_owned(),
            recipient_ids: vec!["codex".to_owned()],
            body: "body".to_owned(),
        };
        assert_eq!(
            tools
                .room_post_as_owner(invalid_request, "2026-08-17T00:00:00Z".to_owned())
                .unwrap_err()
                .code,
            "invalidRoomMessage"
        );

        let invalid_leading_character = MioRoomPostAsOwnerInput {
            request_id: ".unsafe".to_owned(),
            room_id: "dev-room".to_owned(),
            recipient_ids: vec!["codex".to_owned()],
            body: "body".to_owned(),
        };
        assert_eq!(
            tools
                .room_post_as_owner(invalid_leading_character, "2026-08-17T00:00:00Z".to_owned())
                .unwrap_err()
                .code,
            "invalidRoomMessage"
        );

        let missing_recipient = MioRoomPostAsOwnerInput {
            request_id: "request-2".to_owned(),
            room_id: "dev-room".to_owned(),
            recipient_ids: vec!["missing".to_owned()],
            body: "body".to_owned(),
        };
        assert_eq!(
            tools
                .room_post_as_owner(missing_recipient, "2026-08-17T00:00:00Z".to_owned())
                .unwrap_err()
                .code,
            "roomParticipantInvalid"
        );

        let unknown_field = serde_json::from_value::<MioRoomPostAsOwnerInput>(serde_json::json!({
            "requestId": "request-3",
            "roomId": "dev-room",
            "recipientIds": ["codex"],
            "body": "body",
            "authorId": "codex"
        }))
        .unwrap_err();
        assert!(unknown_field.to_string().contains("unknown field"));
    }
}
