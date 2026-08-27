use crate::room_source::DesktopRoomSource;
use crate::time::current_rfc3339_timestamp;
use moe_core::{
    RoomCatalogSource, RoomCreateDraft, RoomMessageDraft, RoomMutationError, RoomMutationSuccess,
    RoomReadQuery, RoomReadResult, RoomSource, RoomStore, RoomSummary, RoomWriteError,
    RoomWriteSuccess,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

const DEFAULT_ROOM_READ_LIMIT: u8 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomReadCommandError {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomWriteCommandError {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomCatalogSuccess {
    ok: bool,
    rooms: Vec<RoomSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomMutationCommandError {
    code: &'static str,
    message: &'static str,
}

fn read_room(
    source: &DesktopRoomSource,
    room_id: String,
    after_message_id: Option<String>,
    limit: Option<u8>,
) -> Result<RoomReadResult, RoomReadCommandError> {
    let query = RoomReadQuery::try_new(
        room_id,
        after_message_id,
        limit.unwrap_or(DEFAULT_ROOM_READ_LIMIT),
    )
    .map_err(|_| RoomReadCommandError {
        code: "invalidRoomRequest",
        message: "The Room read request is invalid.",
    })?;
    Ok(source.read_room(&query))
}

fn current_timestamp() -> Result<String, RoomWriteCommandError> {
    current_rfc3339_timestamp().ok_or(RoomWriteCommandError {
        code: "roomWriteUnavailable",
        message: "The Room message could not be timestamped.",
    })
}

fn mutation_timestamp() -> Result<String, RoomMutationCommandError> {
    current_rfc3339_timestamp().ok_or(RoomMutationCommandError {
        code: "roomMutationUnavailable",
        message: "The Room mutation could not be timestamped.",
    })
}

fn list_rooms(source: &DesktopRoomSource) -> Result<RoomCatalogSuccess, RoomMutationCommandError> {
    source
        .list_rooms()
        .map(|rooms| RoomCatalogSuccess { ok: true, rooms })
        .map_err(|_| RoomMutationCommandError {
            code: "roomCatalogUnavailable",
            message: "The Room catalog is temporarily unavailable.",
        })
}

fn map_mutation_error(error: RoomMutationError) -> RoomMutationCommandError {
    match error {
        RoomMutationError::InvalidMutation => RoomMutationCommandError {
            code: "invalidRoomMutation",
            message: "The Room mutation is invalid.",
        },
        RoomMutationError::RoomNotFound => RoomMutationCommandError {
            code: "roomNotFound",
            message: "The Room is not available.",
        },
        RoomMutationError::ParticipantNotFound => RoomMutationCommandError {
            code: "roomParticipantInvalid",
            message: "The participant is not available.",
        },
        RoomMutationError::ParticipantInUse => RoomMutationCommandError {
            code: "roomParticipantInUse",
            message: "A participant referenced by Room history cannot be removed.",
        },
        RoomMutationError::RequiredParticipant => RoomMutationCommandError {
            code: "roomParticipantRequired",
            message: "The Room must keep its owner and at least one AI participant.",
        },
        RoomMutationError::RequiredRoom => RoomMutationCommandError {
            code: "roomRequired",
            message: "The final Room cannot be deleted.",
        },
        RoomMutationError::IdempotencyConflict => RoomMutationCommandError {
            code: "roomConflict",
            message: "The Room ID was already used for different content.",
        },
        RoomMutationError::CapacityReached => RoomMutationCommandError {
            code: "roomCapacityReached",
            message: "The Room catalog has reached its capacity.",
        },
        RoomMutationError::SourceUnavailable => RoomMutationCommandError {
            code: "roomMutationUnavailable",
            message: "The Room is temporarily unavailable.",
        },
    }
}

fn create_room(
    source: &DesktopRoomSource,
    room_id: String,
    name: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    let owner_id = source
        .owner_participant_id()
        .ok_or(RoomMutationCommandError {
            code: "roomMutationUnavailable",
            message: "The Room owner is temporarily unavailable.",
        })?;
    let draft = RoomCreateDraft::try_new(
        room_id,
        name,
        vec![owner_id, "codex".to_owned()],
        mutation_timestamp()?,
    )
    .map_err(|_| RoomMutationCommandError {
        code: "invalidRoomMutation",
        message: "The Room creation request is invalid.",
    })?;
    source.create_room(draft).map_err(map_mutation_error)
}

fn add_participant(
    source: &DesktopRoomSource,
    room_id: String,
    participant_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    source
        .add_room_participant(&room_id, &participant_id, &mutation_timestamp()?)
        .map_err(map_mutation_error)
}

fn rename_room(
    source: &DesktopRoomSource,
    room_id: String,
    name: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    source
        .rename_room(&room_id, &name, &mutation_timestamp()?)
        .map_err(map_mutation_error)
}

fn remove_participant(
    source: &DesktopRoomSource,
    room_id: String,
    participant_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    if source.is_human_participant(&participant_id) {
        return Err(RoomMutationCommandError {
            code: "roomParticipantRequired",
            message: "The Room owner cannot be removed.",
        });
    }
    source
        .remove_room_participant(&room_id, &participant_id, &mutation_timestamp()?)
        .map_err(map_mutation_error)
}

fn delete_room(
    source: &DesktopRoomSource,
    room_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    if ["moe-dev-room", "comparison-room", "mcp-lab"].contains(&room_id.as_str()) {
        return Err(RoomMutationCommandError {
            code: "protectedRoom",
            message: "A bundled Room cannot be deleted.",
        });
    }
    source
        .delete_room(&room_id, &mutation_timestamp()?)
        .map_err(map_mutation_error)
}

fn map_write_error(error: RoomWriteError) -> RoomWriteCommandError {
    match error {
        RoomWriteError::RoomNotFound => RoomWriteCommandError {
            code: "roomNotFound",
            message: "The Room is not available.",
        },
        RoomWriteError::AuthorNotParticipant | RoomWriteError::RecipientNotParticipant => {
            RoomWriteCommandError {
                code: "roomParticipantInvalid",
                message: "A message participant is not available in the Room.",
            }
        }
        RoomWriteError::IdempotencyConflict => RoomWriteCommandError {
            code: "messageConflict",
            message: "The message ID was already used for different content.",
        },
        RoomWriteError::RoomCapacityReached => RoomWriteCommandError {
            code: "roomCapacityReached",
            message: "The Room cannot accept more in-memory messages.",
        },
        RoomWriteError::SourceUnavailable => RoomWriteCommandError {
            code: "roomWriteUnavailable",
            message: "The Room is temporarily unavailable.",
        },
    }
}

fn write_message(
    source: &DesktopRoomSource,
    room_id: String,
    message_id: String,
    recipient_ids: Vec<String>,
    body: String,
) -> Result<RoomWriteSuccess, RoomWriteCommandError> {
    let owner_id = source.owner_participant_id().ok_or(RoomWriteCommandError {
        code: "roomWriteUnavailable",
        message: "The Room owner is temporarily unavailable.",
    })?;
    let draft = RoomMessageDraft::try_new(
        message_id,
        room_id,
        owner_id,
        recipient_ids,
        body,
        current_timestamp()?,
        Vec::new(),
    )
    .map_err(|_| RoomWriteCommandError {
        code: "invalidRoomMessage",
        message: "The Room message is invalid.",
    })?;
    source.append_message(draft).map_err(map_write_error)
}

#[tauri::command]
pub(crate) fn desktop_room_read(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    after_message_id: Option<String>,
    limit: Option<u8>,
) -> Result<RoomReadResult, RoomReadCommandError> {
    read_room(source.as_ref(), room_id, after_message_id, limit)
}

#[tauri::command]
pub(crate) fn desktop_room_list(
    source: State<'_, Arc<DesktopRoomSource>>,
) -> Result<RoomCatalogSuccess, RoomMutationCommandError> {
    list_rooms(source.as_ref())
}

#[tauri::command]
pub(crate) fn desktop_room_create(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    name: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    create_room(source.as_ref(), room_id, name)
}

#[tauri::command]
pub(crate) fn desktop_room_add_participant(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    participant_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    add_participant(source.as_ref(), room_id, participant_id)
}

#[tauri::command]
pub(crate) fn desktop_room_rename(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    name: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    rename_room(source.as_ref(), room_id, name)
}

#[tauri::command]
pub(crate) fn desktop_room_remove_participant(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    participant_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    remove_participant(source.as_ref(), room_id, participant_id)
}

#[tauri::command]
pub(crate) fn desktop_room_delete(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
) -> Result<RoomMutationSuccess, RoomMutationCommandError> {
    delete_room(source.as_ref(), room_id)
}

#[tauri::command]
pub(crate) fn desktop_room_write_message(
    source: State<'_, Arc<DesktopRoomSource>>,
    room_id: String,
    message_id: String,
    recipient_ids: Vec<String>,
    body: String,
) -> Result<RoomWriteSuccess, RoomWriteCommandError> {
    write_message(source.as_ref(), room_id, message_id, recipient_ids, body)
}

#[cfg(test)]
mod tests {
    use super::{
        add_participant, create_room, delete_room, list_rooms, read_room, remove_participant,
        rename_room, write_message,
    };
    use crate::room_source::desktop_room_source;
    use moe_core::{RoomReadQuery, RoomSource};

    #[test]
    fn reads_bundled_room_without_exposing_an_unbounded_query() {
        let source = desktop_room_source();
        let result = serde_json::to_value(
            read_room(source.as_ref(), "moe-dev-room".to_owned(), None, None).unwrap(),
        )
        .unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["room"]["messages"].as_array().unwrap().len(), 3);

        let error = read_room(source.as_ref(), "../private".to_owned(), None, Some(30))
            .expect_err("path-like Room ID must be rejected");
        assert_eq!(error.code, "invalidRoomRequest");
        let error = read_room(source.as_ref(), "moe-dev-room".to_owned(), None, Some(31))
            .expect_err("unbounded Room limit must be rejected");
        assert_eq!(error.code, "invalidRoomRequest");
    }

    #[test]
    fn lists_creates_and_updates_rooms_idempotently() {
        let source = desktop_room_source();
        let catalog = list_rooms(source.as_ref()).unwrap();
        assert_eq!(catalog.rooms.len(), 3);

        let created = create_room(
            source.as_ref(),
            "room-client-1".to_owned(),
            "New Room".to_owned(),
        )
        .unwrap();
        let duplicate = create_room(
            source.as_ref(),
            "room-client-1".to_owned(),
            "New Room".to_owned(),
        )
        .unwrap();
        assert_eq!(created.status(), moe_core::RoomMutationStatus::Created);
        assert_eq!(duplicate.status(), moe_core::RoomMutationStatus::Duplicate);

        let added = add_participant(
            source.as_ref(),
            "room-client-1".to_owned(),
            "gemini".to_owned(),
        )
        .unwrap();
        let duplicate = add_participant(
            source.as_ref(),
            "room-client-1".to_owned(),
            "gemini".to_owned(),
        )
        .unwrap();
        assert_eq!(added.status(), moe_core::RoomMutationStatus::Added);
        assert_eq!(duplicate.status(), moe_core::RoomMutationStatus::Duplicate);

        let renamed = rename_room(
            source.as_ref(),
            "room-client-1".to_owned(),
            "Renamed Room".to_owned(),
        )
        .unwrap();
        assert_eq!(renamed.status(), moe_core::RoomMutationStatus::Renamed);
        let removed = remove_participant(
            source.as_ref(),
            "room-client-1".to_owned(),
            "gemini".to_owned(),
        )
        .unwrap();
        assert_eq!(removed.status(), moe_core::RoomMutationStatus::Removed);
        let deleted = delete_room(source.as_ref(), "room-client-1".to_owned()).unwrap();
        assert_eq!(deleted.status(), moe_core::RoomMutationStatus::Deleted);
        assert_eq!(list_rooms(source.as_ref()).unwrap().rooms.len(), 3);

        let protected = delete_room(source.as_ref(), "moe-dev-room".to_owned()).unwrap_err();
        assert_eq!(protected.code, "protectedRoom");
    }

    #[test]
    fn writes_once_and_returns_the_backend_message_for_a_retry() {
        let source = desktop_room_source();
        let first = write_message(
            source.as_ref(),
            "moe-dev-room".to_owned(),
            "message-client-1".to_owned(),
            vec!["codex".to_owned()],
            "Rust Room write".to_owned(),
        )
        .unwrap();
        let retry = write_message(
            source.as_ref(),
            "moe-dev-room".to_owned(),
            "message-client-1".to_owned(),
            vec!["codex".to_owned()],
            "Rust Room write".to_owned(),
        )
        .unwrap();
        assert_eq!(first.status(), moe_core::RoomWriteStatus::Appended);
        assert_eq!(retry.status(), moe_core::RoomWriteStatus::Duplicate);
        assert_eq!(retry.message(), first.message());

        let query =
            RoomReadQuery::try_new("moe-dev-room".to_owned(), Some("welcome-3".to_owned()), 30)
                .unwrap();
        let result = serde_json::to_value(source.read_room(&query)).unwrap();
        assert_eq!(result["room"]["messages"].as_array().unwrap().len(), 1);
        assert_eq!(result["room"]["messages"][0]["body"], "Rust Room write");
    }

    #[test]
    fn rejects_invalid_recipient_and_conflicting_retry() {
        let source = desktop_room_source();
        let error = write_message(
            source.as_ref(),
            "moe-dev-room".to_owned(),
            "message-client-2".to_owned(),
            vec!["not-in-room".to_owned()],
            "invalid recipient".to_owned(),
        )
        .unwrap_err();
        assert_eq!(error.code, "roomParticipantInvalid");

        write_message(
            source.as_ref(),
            "moe-dev-room".to_owned(),
            "message-client-3".to_owned(),
            vec!["codex".to_owned()],
            "first".to_owned(),
        )
        .unwrap();
        let conflict = write_message(
            source.as_ref(),
            "moe-dev-room".to_owned(),
            "message-client-3".to_owned(),
            vec!["codex".to_owned()],
            "changed".to_owned(),
        )
        .unwrap_err();
        assert_eq!(conflict.code, "messageConflict");
    }
}
