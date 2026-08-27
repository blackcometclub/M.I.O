use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;

const MAXIMUM_IDENTIFIER_BYTES: usize = 128;
const MAXIMUM_DISPLAY_NAME_BYTES: usize = 200;
const MAXIMUM_MESSAGE_BODY_BYTES: usize = 100_000;
const MAXIMUM_ROOM_WRITE_BODY_BYTES: usize = 4_000;
const MAXIMUM_PARTICIPANTS: usize = 1_000;
const MAXIMUM_ROOMS: usize = 1_000;
const MAXIMUM_ROOM_PARTICIPANTS: usize = 100;
const MAXIMUM_ROOM_MESSAGES: usize = 10_000;
const MAXIMUM_MESSAGE_LINKS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomParticipantKind {
    Human,
    Ai,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomParticipant {
    pub id: String,
    pub display_name: String,
    pub kind: RoomParticipantKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomMessage {
    pub id: String,
    pub room_id: String,
    pub author_id: String,
    pub recipients: Vec<String>,
    pub body: String,
    pub created_at: String,
    pub artifact_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<RoomMessageProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomMessageProvenance {
    CodexOwnerProxy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub participant_ids: Vec<String>,
    pub messages: Vec<RoomMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomSnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub participants: Vec<RoomParticipant>,
    pub rooms: Vec<Room>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomSnapshotError {
    InvalidSnapshot,
    SourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSummary {
    pub id: String,
    pub name: String,
    pub participant_ids: Vec<String>,
    pub latest_message_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomCatalogError {
    SourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCreateDraft {
    room: Room,
    created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomCreateDraftError {
    InvalidRoom,
}

impl RoomCreateDraft {
    pub fn try_new(
        id: String,
        name: String,
        participant_ids: Vec<String>,
        created_at: String,
    ) -> Result<Self, RoomCreateDraftError> {
        if !valid_identifier(&id)
            || !valid_text(&name, MAXIMUM_DISPLAY_NAME_BYTES)
            || participant_ids.is_empty()
            || participant_ids.len() > MAXIMUM_ROOM_PARTICIPANTS
            || !unique_valid_identifiers(&participant_ids)
            || !valid_text(&created_at, 64)
        {
            return Err(RoomCreateDraftError::InvalidRoom);
        }
        Ok(Self {
            room: Room {
                id,
                name,
                participant_ids,
                messages: Vec::new(),
            },
            created_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomMutationStatus {
    Created,
    Added,
    Renamed,
    Removed,
    Deleted,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomMutationSuccess {
    ok: bool,
    status: RoomMutationStatus,
    room: RoomSummary,
}

impl RoomMutationSuccess {
    fn new(status: RoomMutationStatus, room: &Room) -> Self {
        Self {
            ok: true,
            status,
            room: room_summary(room),
        }
    }

    pub fn status(&self) -> RoomMutationStatus {
        self.status
    }

    pub fn room(&self) -> &RoomSummary {
        &self.room
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomMutationError {
    InvalidMutation,
    RoomNotFound,
    ParticipantNotFound,
    ParticipantInUse,
    RequiredParticipant,
    RequiredRoom,
    IdempotencyConflict,
    CapacityReached,
    SourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomReadQuery {
    room_id: String,
    after_message_id: Option<String>,
    limit: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomReadQueryError {
    InvalidQuery,
}

impl RoomReadQuery {
    pub fn try_new(
        room_id: String,
        after_message_id: Option<String>,
        limit: u8,
    ) -> Result<Self, RoomReadQueryError> {
        if !valid_identifier(&room_id)
            || after_message_id
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || !(1..=30).contains(&limit)
        {
            return Err(RoomReadQueryError::InvalidQuery);
        }
        Ok(Self {
            room_id,
            after_message_id,
            limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RoomReadResult {
    Success(RoomReadSuccess),
    Failure(RoomReadFailure),
}

impl RoomReadResult {
    pub fn participants(&self) -> Option<&[RoomParticipant]> {
        match self {
            Self::Success(result) => Some(&result.room.participants),
            Self::Failure(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomReadSuccess {
    ok: bool,
    snapshot_generated_at: String,
    room: RoomReadView,
    page: RoomReadPage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomReadView {
    id: String,
    name: String,
    participants: Vec<RoomParticipant>,
    messages: Vec<RoomMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomReadPage {
    after_message_id: Option<String>,
    limit: u8,
    returned: usize,
    has_more_before: bool,
    has_more_after: bool,
    next_after_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomReadFailure {
    ok: bool,
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMessageDraft {
    message: RoomMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomMessageDraftError {
    InvalidMessage,
}

impl RoomMessageDraft {
    pub fn try_new(
        id: String,
        room_id: String,
        author_id: String,
        recipients: Vec<String>,
        body: String,
        created_at: String,
        artifact_ids: Vec<String>,
    ) -> Result<Self, RoomMessageDraftError> {
        Self::try_new_with_provenance(
            id,
            room_id,
            author_id,
            recipients,
            body,
            created_at,
            artifact_ids,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new_with_provenance(
        id: String,
        room_id: String,
        author_id: String,
        recipients: Vec<String>,
        body: String,
        created_at: String,
        artifact_ids: Vec<String>,
        provenance: Option<RoomMessageProvenance>,
    ) -> Result<Self, RoomMessageDraftError> {
        let message = RoomMessage {
            id,
            room_id,
            author_id,
            recipients,
            body,
            created_at,
            artifact_ids,
            provenance,
        };
        if !valid_message_shape(&message, MAXIMUM_ROOM_WRITE_BODY_BYTES)
            || message.body.trim().is_empty()
            || message.recipients.is_empty()
        {
            return Err(RoomMessageDraftError::InvalidMessage);
        }
        Ok(Self { message })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomWriteStatus {
    Appended,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomWriteSuccess {
    ok: bool,
    status: RoomWriteStatus,
    message: RoomMessage,
}

impl RoomWriteSuccess {
    fn new(status: RoomWriteStatus, message: RoomMessage) -> Self {
        Self {
            ok: true,
            status,
            message,
        }
    }

    pub fn status(&self) -> RoomWriteStatus {
        self.status
    }

    pub fn message(&self) -> &RoomMessage {
        &self.message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomWriteError {
    RoomNotFound,
    AuthorNotParticipant,
    RecipientNotParticipant,
    IdempotencyConflict,
    RoomCapacityReached,
    SourceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomMessageFindError {
    InvalidLookup,
    RoomNotFound,
    MessageNotFound,
    SourceUnavailable,
}

pub trait RoomSource {
    fn read_room(&self, query: &RoomReadQuery) -> RoomReadResult;

    fn find_message(
        &self,
        room_id: &str,
        message_id: &str,
    ) -> Result<RoomMessage, RoomMessageFindError>;
}

pub trait RoomCatalogSource {
    fn list_rooms(&self) -> Result<Vec<RoomSummary>, RoomCatalogError>;
}

pub trait RoomStore: RoomSource + RoomCatalogSource {
    fn append_message(&self, draft: RoomMessageDraft) -> Result<RoomWriteSuccess, RoomWriteError>;

    fn create_room(&self, draft: RoomCreateDraft)
    -> Result<RoomMutationSuccess, RoomMutationError>;

    fn add_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError>;

    fn rename_room(
        &self,
        room_id: &str,
        name: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError>;

    fn remove_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError>;

    fn delete_room(
        &self,
        room_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError>;
}

pub struct InMemoryRoomSource {
    snapshot: RwLock<RoomSnapshot>,
}

impl InMemoryRoomSource {
    pub fn new(snapshot: RoomSnapshot) -> Result<Self, RoomSnapshotError> {
        validate_snapshot(&snapshot)?;
        Ok(Self {
            snapshot: RwLock::new(snapshot),
        })
    }

    pub fn snapshot(&self) -> Result<RoomSnapshot, RoomSnapshotError> {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| RoomSnapshotError::SourceUnavailable)
    }

    pub fn replace_snapshot(&self, snapshot: RoomSnapshot) -> Result<(), RoomSnapshotError> {
        validate_snapshot(&snapshot)?;
        let mut current = self
            .snapshot
            .write()
            .map_err(|_| RoomSnapshotError::SourceUnavailable)?;
        *current = snapshot;
        Ok(())
    }
}

impl RoomSource for InMemoryRoomSource {
    fn read_room(&self, query: &RoomReadQuery) -> RoomReadResult {
        let Ok(snapshot) = self.snapshot.read() else {
            return RoomReadResult::Failure(RoomReadFailure {
                ok: false,
                code: "room_unavailable",
                message: "The Room source is temporarily unavailable.",
            });
        };
        let Some(room) = snapshot.rooms.iter().find(|room| room.id == query.room_id) else {
            return RoomReadResult::Failure(RoomReadFailure {
                ok: false,
                code: "room_not_found",
                message: "The requested room is not available.",
            });
        };

        let start_index = if let Some(cursor) = query.after_message_id.as_deref() {
            let Some(cursor_index) = room
                .messages
                .iter()
                .position(|message| message.id == cursor)
            else {
                return RoomReadResult::Failure(RoomReadFailure {
                    ok: false,
                    code: "cursor_not_found",
                    message: "The requested message cursor is not available.",
                });
            };
            cursor_index + 1
        } else {
            room.messages.len().saturating_sub(query.limit as usize)
        };
        let messages: Vec<_> = room
            .messages
            .iter()
            .skip(start_index)
            .take(query.limit as usize)
            .cloned()
            .collect();
        let participants = room
            .participant_ids
            .iter()
            .filter_map(|participant_id| {
                snapshot
                    .participants
                    .iter()
                    .find(|participant| participant.id == *participant_id)
                    .cloned()
            })
            .collect();
        let next_after_message_id = messages
            .last()
            .map(|message| message.id.clone())
            .or_else(|| query.after_message_id.clone());

        RoomReadResult::Success(RoomReadSuccess {
            ok: true,
            snapshot_generated_at: snapshot.generated_at.clone(),
            room: RoomReadView {
                id: room.id.clone(),
                name: room.name.clone(),
                participants,
                messages,
            },
            page: RoomReadPage {
                after_message_id: query.after_message_id.clone(),
                limit: query.limit,
                returned: room
                    .messages
                    .len()
                    .saturating_sub(start_index)
                    .min(query.limit as usize),
                has_more_before: query.after_message_id.is_none() && start_index > 0,
                has_more_after: start_index.saturating_add(query.limit as usize)
                    < room.messages.len(),
                next_after_message_id,
            },
        })
    }

    fn find_message(
        &self,
        room_id: &str,
        message_id: &str,
    ) -> Result<RoomMessage, RoomMessageFindError> {
        if !valid_identifier(room_id) || !valid_identifier(message_id) {
            return Err(RoomMessageFindError::InvalidLookup);
        }
        let snapshot = self
            .snapshot
            .read()
            .map_err(|_| RoomMessageFindError::SourceUnavailable)?;
        let room = snapshot
            .rooms
            .iter()
            .find(|room| room.id == room_id)
            .ok_or(RoomMessageFindError::RoomNotFound)?;
        room.messages
            .iter()
            .find(|message| message.id == message_id)
            .cloned()
            .ok_or(RoomMessageFindError::MessageNotFound)
    }
}

impl RoomCatalogSource for InMemoryRoomSource {
    fn list_rooms(&self) -> Result<Vec<RoomSummary>, RoomCatalogError> {
        let snapshot = self
            .snapshot
            .read()
            .map_err(|_| RoomCatalogError::SourceUnavailable)?;
        Ok(snapshot.rooms.iter().map(room_summary).collect())
    }
}

impl RoomStore for InMemoryRoomSource {
    fn append_message(&self, draft: RoomMessageDraft) -> Result<RoomWriteSuccess, RoomWriteError> {
        let message = draft.message;
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomWriteError::SourceUnavailable)?;

        if let Some(existing) = snapshot
            .rooms
            .iter()
            .flat_map(|room| room.messages.iter())
            .find(|existing| existing.id == message.id)
        {
            return if same_idempotent_payload(existing, &message) {
                Ok(RoomWriteSuccess::new(
                    RoomWriteStatus::Duplicate,
                    existing.clone(),
                ))
            } else {
                Err(RoomWriteError::IdempotencyConflict)
            };
        }

        {
            let room = snapshot
                .rooms
                .iter_mut()
                .find(|room| room.id == message.room_id)
                .ok_or(RoomWriteError::RoomNotFound)?;
            if room.messages.len() >= MAXIMUM_ROOM_MESSAGES {
                return Err(RoomWriteError::RoomCapacityReached);
            }
            if !room
                .participant_ids
                .iter()
                .any(|participant_id| participant_id == &message.author_id)
            {
                return Err(RoomWriteError::AuthorNotParticipant);
            }
            if message.recipients.iter().any(|recipient| {
                !room
                    .participant_ids
                    .iter()
                    .any(|participant_id| participant_id == recipient)
            }) {
                return Err(RoomWriteError::RecipientNotParticipant);
            }
            room.messages.push(message.clone());
        }
        snapshot.generated_at.clone_from(&message.created_at);
        Ok(RoomWriteSuccess::new(RoomWriteStatus::Appended, message))
    }

    fn create_room(
        &self,
        draft: RoomCreateDraft,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        if let Some(existing) = snapshot.rooms.iter().find(|room| room.id == draft.room.id) {
            return if existing.name == draft.room.name
                && existing.participant_ids == draft.room.participant_ids
            {
                Ok(RoomMutationSuccess::new(
                    RoomMutationStatus::Duplicate,
                    existing,
                ))
            } else {
                Err(RoomMutationError::IdempotencyConflict)
            };
        }
        if snapshot.rooms.len() >= MAXIMUM_ROOMS {
            return Err(RoomMutationError::CapacityReached);
        }
        if draft.room.participant_ids.iter().any(|participant_id| {
            !snapshot
                .participants
                .iter()
                .any(|participant| participant.id == *participant_id)
        }) {
            return Err(RoomMutationError::ParticipantNotFound);
        }
        snapshot.generated_at.clone_from(&draft.created_at);
        snapshot.rooms.push(draft.room);
        let room = snapshot.rooms.last().expect("a Room was just appended");
        Ok(RoomMutationSuccess::new(RoomMutationStatus::Created, room))
    }

    fn add_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        if !valid_identifier(room_id)
            || !valid_identifier(participant_id)
            || !valid_text(updated_at, 64)
        {
            return Err(RoomMutationError::InvalidMutation);
        }
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        if !snapshot
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return Err(RoomMutationError::ParticipantNotFound);
        }
        let room = snapshot
            .rooms
            .iter_mut()
            .find(|room| room.id == room_id)
            .ok_or(RoomMutationError::RoomNotFound)?;
        if room
            .participant_ids
            .iter()
            .any(|existing| existing == participant_id)
        {
            return Ok(RoomMutationSuccess::new(
                RoomMutationStatus::Duplicate,
                room,
            ));
        }
        if room.participant_ids.len() >= MAXIMUM_ROOM_PARTICIPANTS {
            return Err(RoomMutationError::CapacityReached);
        }
        room.participant_ids.push(participant_id.to_owned());
        let result = RoomMutationSuccess::new(RoomMutationStatus::Added, room);
        snapshot.generated_at = updated_at.to_owned();
        Ok(result)
    }

    fn rename_room(
        &self,
        room_id: &str,
        name: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        if !valid_identifier(room_id)
            || !valid_text(name, MAXIMUM_DISPLAY_NAME_BYTES)
            || !valid_text(updated_at, 64)
        {
            return Err(RoomMutationError::InvalidMutation);
        }
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let room = snapshot
            .rooms
            .iter_mut()
            .find(|room| room.id == room_id)
            .ok_or(RoomMutationError::RoomNotFound)?;
        if room.name == name {
            return Ok(RoomMutationSuccess::new(
                RoomMutationStatus::Duplicate,
                room,
            ));
        }
        room.name = name.to_owned();
        let result = RoomMutationSuccess::new(RoomMutationStatus::Renamed, room);
        snapshot.generated_at = updated_at.to_owned();
        Ok(result)
    }

    fn remove_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        if !valid_identifier(room_id)
            || !valid_identifier(participant_id)
            || !valid_text(updated_at, 64)
        {
            return Err(RoomMutationError::InvalidMutation);
        }
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        if !snapshot
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return Err(RoomMutationError::ParticipantNotFound);
        }
        let room = snapshot
            .rooms
            .iter_mut()
            .find(|room| room.id == room_id)
            .ok_or(RoomMutationError::RoomNotFound)?;
        let Some(index) = room
            .participant_ids
            .iter()
            .position(|existing| existing == participant_id)
        else {
            return Ok(RoomMutationSuccess::new(
                RoomMutationStatus::Duplicate,
                room,
            ));
        };
        if room.participant_ids.len() <= 2 {
            return Err(RoomMutationError::RequiredParticipant);
        }
        if room.messages.iter().any(|message| {
            message.author_id == participant_id
                || message
                    .recipients
                    .iter()
                    .any(|recipient| recipient == participant_id)
        }) {
            return Err(RoomMutationError::ParticipantInUse);
        }
        room.participant_ids.remove(index);
        let result = RoomMutationSuccess::new(RoomMutationStatus::Removed, room);
        snapshot.generated_at = updated_at.to_owned();
        Ok(result)
    }

    fn delete_room(
        &self,
        room_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        if !valid_identifier(room_id) || !valid_text(updated_at, 64) {
            return Err(RoomMutationError::InvalidMutation);
        }
        let mut snapshot = self
            .snapshot
            .write()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        if snapshot.rooms.len() <= 1 {
            return Err(RoomMutationError::RequiredRoom);
        }
        let index = snapshot
            .rooms
            .iter()
            .position(|room| room.id == room_id)
            .ok_or(RoomMutationError::RoomNotFound)?;
        let result = RoomMutationSuccess::new(RoomMutationStatus::Deleted, &snapshot.rooms[index]);
        snapshot.rooms.remove(index);
        snapshot.generated_at = updated_at.to_owned();
        Ok(result)
    }
}

fn room_summary(room: &Room) -> RoomSummary {
    RoomSummary {
        id: room.id.clone(),
        name: room.name.clone(),
        participant_ids: room.participant_ids.clone(),
        latest_message_at: room
            .messages
            .last()
            .map(|message| message.created_at.clone()),
    }
}

fn same_idempotent_payload(existing: &RoomMessage, retried: &RoomMessage) -> bool {
    existing.id == retried.id
        && existing.room_id == retried.room_id
        && existing.author_id == retried.author_id
        && existing.recipients == retried.recipients
        && existing.body == retried.body
        && existing.artifact_ids == retried.artifact_ids
        && existing.provenance == retried.provenance
}

fn validate_snapshot(snapshot: &RoomSnapshot) -> Result<(), RoomSnapshotError> {
    if snapshot.schema_version != moe_protocol::PROTOCOL_VERSION
        || !valid_text(&snapshot.generated_at, 64)
        || snapshot.participants.len() > MAXIMUM_PARTICIPANTS
        || snapshot.rooms.len() > MAXIMUM_ROOMS
    {
        return Err(RoomSnapshotError::InvalidSnapshot);
    }

    let mut participant_ids = HashSet::new();
    for participant in &snapshot.participants {
        if !valid_identifier(&participant.id)
            || !valid_text(&participant.display_name, MAXIMUM_DISPLAY_NAME_BYTES)
            || !participant_ids.insert(participant.id.as_str())
        {
            return Err(RoomSnapshotError::InvalidSnapshot);
        }
    }

    let mut room_ids = HashSet::new();
    let mut message_ids = HashSet::new();
    for room in &snapshot.rooms {
        if !valid_identifier(&room.id)
            || !valid_text(&room.name, MAXIMUM_DISPLAY_NAME_BYTES)
            || room.participant_ids.len() > MAXIMUM_ROOM_PARTICIPANTS
            || room.messages.len() > MAXIMUM_ROOM_MESSAGES
            || !room_ids.insert(room.id.as_str())
        {
            return Err(RoomSnapshotError::InvalidSnapshot);
        }
        let room_participants: HashSet<_> =
            room.participant_ids.iter().map(String::as_str).collect();
        if room_participants.len() != room.participant_ids.len()
            || room_participants
                .iter()
                .any(|participant_id| !participant_ids.contains(participant_id))
        {
            return Err(RoomSnapshotError::InvalidSnapshot);
        }

        for message in &room.messages {
            if !valid_message_shape(message, MAXIMUM_MESSAGE_BODY_BYTES)
                || message.room_id != room.id
                || !room_participants.contains(message.author_id.as_str())
                || message
                    .recipients
                    .iter()
                    .any(|recipient| !room_participants.contains(recipient.as_str()))
                || !message_ids.insert(message.id.as_str())
            {
                return Err(RoomSnapshotError::InvalidSnapshot);
            }
        }
    }
    Ok(())
}

fn valid_message_shape(message: &RoomMessage, maximum_body_bytes: usize) -> bool {
    valid_identifier(&message.id)
        && valid_identifier(&message.room_id)
        && valid_identifier(&message.author_id)
        && message.recipients.len() <= MAXIMUM_MESSAGE_LINKS
        && unique_valid_identifiers(&message.recipients)
        && valid_text(&message.body, maximum_body_bytes)
        && valid_text(&message.created_at, 64)
        && message.artifact_ids.len() <= MAXIMUM_MESSAGE_LINKS
        && unique_valid_identifiers(&message.artifact_ids)
}

fn unique_valid_identifiers(values: &[String]) -> bool {
    let mut unique = HashSet::new();
    values
        .iter()
        .all(|value| valid_identifier(value) && unique.insert(value.as_str()))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAXIMUM_IDENTIFIER_BYTES
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> InMemoryRoomSource {
        InMemoryRoomSource::new(RoomSnapshot {
            schema_version: moe_protocol::PROTOCOL_VERSION.to_owned(),
            generated_at: "2026-08-12T00:00:00+09:00".to_owned(),
            participants: vec![
                RoomParticipant {
                    id: "owner".to_owned(),
                    display_name: "Owner".to_owned(),
                    kind: RoomParticipantKind::Human,
                },
                RoomParticipant {
                    id: "codex".to_owned(),
                    display_name: "Codex".to_owned(),
                    kind: RoomParticipantKind::Ai,
                },
                RoomParticipant {
                    id: "gemini".to_owned(),
                    display_name: "Gemini".to_owned(),
                    kind: RoomParticipantKind::Ai,
                },
            ],
            rooms: vec![Room {
                id: "room-1".to_owned(),
                name: "Room 1".to_owned(),
                participant_ids: vec!["owner".to_owned()],
                messages: (1..=3)
                    .map(|index| RoomMessage {
                        id: format!("message-{index}"),
                        room_id: "room-1".to_owned(),
                        author_id: "owner".to_owned(),
                        recipients: Vec::new(),
                        body: format!("body {index}"),
                        created_at: format!("2026-08-12T00:00:0{index}+09:00"),
                        artifact_ids: Vec::new(),
                        provenance: None,
                    })
                    .collect(),
            }],
        })
        .unwrap()
    }

    #[test]
    fn reads_latest_page_and_cursor_page() {
        let source = source();
        let latest = serde_json::to_value(
            source.read_room(&RoomReadQuery::try_new("room-1".to_owned(), None, 2).unwrap()),
        )
        .unwrap();
        assert_eq!(latest["room"]["messages"][0]["id"], "message-2");
        assert_eq!(latest["page"]["hasMoreBefore"], true);

        let after = serde_json::to_value(source.read_room(
            &RoomReadQuery::try_new("room-1".to_owned(), Some("message-1".to_owned()), 1).unwrap(),
        ))
        .unwrap();
        assert_eq!(after["room"]["messages"][0]["id"], "message-2");
        assert_eq!(after["page"]["hasMoreAfter"], true);
        assert_eq!(after["page"]["nextAfterMessageId"], "message-2");
    }

    #[test]
    fn reports_room_and_cursor_not_found_without_snapshot_detail() {
        let source = source();
        let missing_room = serde_json::to_value(
            source.read_room(&RoomReadQuery::try_new("missing".to_owned(), None, 30).unwrap()),
        )
        .unwrap();
        assert_eq!(missing_room["code"], "room_not_found");
        let missing_cursor = serde_json::to_value(source.read_room(
            &RoomReadQuery::try_new("room-1".to_owned(), Some("missing".to_owned()), 30).unwrap(),
        ))
        .unwrap();
        assert_eq!(missing_cursor["code"], "cursor_not_found");
    }

    #[test]
    fn rejects_dangling_participant_and_duplicate_message() {
        let first_source = source();
        let mut snapshot = first_source.snapshot.into_inner().unwrap();
        snapshot.rooms[0].participant_ids.push("missing".to_owned());
        assert!(InMemoryRoomSource::new(snapshot).is_err());

        let second_source = source();
        let mut snapshot = second_source.snapshot.into_inner().unwrap();
        snapshot.rooms[0].messages[1].id = "message-1".to_owned();
        assert!(InMemoryRoomSource::new(snapshot).is_err());

        let third_source = source();
        let mut snapshot = third_source.snapshot.into_inner().unwrap();
        let mut second_room = snapshot.rooms[0].clone();
        second_room.id = "room-2".to_owned();
        for message in &mut second_room.messages {
            message.room_id.clone_from(&second_room.id);
        }
        snapshot.rooms.push(second_room);
        assert!(InMemoryRoomSource::new(snapshot).is_err());
    }

    #[test]
    fn rejects_unbounded_or_malformed_read_query() {
        for (room_id, cursor, limit) in [
            ("../room", None, 1),
            ("room-1", Some("../message"), 1),
            ("room-1", None, 0),
            ("room-1", None, 31),
        ] {
            assert_eq!(
                RoomReadQuery::try_new(room_id.to_owned(), cursor.map(str::to_owned), limit,),
                Err(RoomReadQueryError::InvalidQuery)
            );
        }
    }

    fn draft(id: &str, body: &str) -> RoomMessageDraft {
        RoomMessageDraft::try_new(
            id.to_owned(),
            "room-1".to_owned(),
            "owner".to_owned(),
            vec!["owner".to_owned()],
            body.to_owned(),
            "2026-08-12T01:00:00Z".to_owned(),
            Vec::new(),
        )
        .unwrap()
    }

    fn codex_owner_proxy_draft(id: &str, body: &str) -> RoomMessageDraft {
        RoomMessageDraft::try_new_with_provenance(
            id.to_owned(),
            "room-1".to_owned(),
            "owner".to_owned(),
            vec!["owner".to_owned()],
            body.to_owned(),
            "2026-08-12T01:00:00Z".to_owned(),
            Vec::new(),
            Some(RoomMessageProvenance::CodexOwnerProxy),
        )
        .unwrap()
    }

    #[test]
    fn appends_once_and_returns_the_same_message_for_an_exact_retry() {
        let source = source();
        let first = source
            .append_message(draft("message-client-1", "new body"))
            .unwrap();
        assert_eq!(first.status(), RoomWriteStatus::Appended);
        let retry = source
            .append_message(draft("message-client-1", "new body"))
            .unwrap();
        assert_eq!(retry.status(), RoomWriteStatus::Duplicate);
        assert_eq!(retry.message(), first.message());

        let result = serde_json::to_value(source.read_room(
            &RoomReadQuery::try_new("room-1".to_owned(), Some("message-3".to_owned()), 30).unwrap(),
        ))
        .unwrap();
        assert_eq!(result["room"]["messages"].as_array().unwrap().len(), 1);
        assert_eq!(result["room"]["messages"][0]["body"], "new body");
    }

    #[test]
    fn rejects_idempotency_conflict_and_invalid_write_shape() {
        let source = source();
        source
            .append_message(draft("message-client-1", "first"))
            .unwrap();
        assert_eq!(
            source.append_message(draft("message-client-1", "changed")),
            Err(RoomWriteError::IdempotencyConflict)
        );
        assert_eq!(
            source.append_message(codex_owner_proxy_draft("message-client-1", "first")),
            Err(RoomWriteError::IdempotencyConflict)
        );
        assert_eq!(
            RoomMessageDraft::try_new(
                "message-client-2".to_owned(),
                "room-1".to_owned(),
                "owner".to_owned(),
                Vec::new(),
                "   ".to_owned(),
                "2026-08-12T01:00:00Z".to_owned(),
                Vec::new(),
            ),
            Err(RoomMessageDraftError::InvalidMessage)
        );
    }

    #[test]
    fn loads_legacy_messages_without_provenance_and_omits_empty_provenance() {
        let snapshot = source().snapshot().unwrap();
        let value = serde_json::to_value(&snapshot).unwrap();
        assert!(value["rooms"][0]["messages"][0].get("provenance").is_none());
        let loaded = serde_json::from_value::<RoomSnapshot>(value).unwrap();
        assert_eq!(loaded.rooms[0].messages[0].provenance, None);
    }

    #[test]
    fn preserves_owner_proxy_provenance_in_snapshot_round_trip() {
        let source = source();
        source
            .append_message(codex_owner_proxy_draft("mcp-owner-request-1", "body"))
            .unwrap();
        let value = serde_json::to_value(source.snapshot().unwrap()).unwrap();
        let loaded = serde_json::from_value::<RoomSnapshot>(value).unwrap();
        assert_eq!(
            loaded.rooms[0].messages.last().unwrap().provenance,
            Some(RoomMessageProvenance::CodexOwnerProxy)
        );
    }

    #[test]
    fn lists_creates_and_updates_rooms_idempotently() {
        let source = source();
        let catalog = source.list_rooms().unwrap();
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].id, "room-1");
        assert_eq!(
            catalog[0].latest_message_at.as_deref(),
            Some("2026-08-12T00:00:03+09:00")
        );

        let create = || {
            RoomCreateDraft::try_new(
                "room-2".to_owned(),
                "Room 2".to_owned(),
                vec!["owner".to_owned()],
                "2026-08-12T02:00:00Z".to_owned(),
            )
            .unwrap()
        };
        let created = source.create_room(create()).unwrap();
        let duplicate = source.create_room(create()).unwrap();
        assert_eq!(created.status(), RoomMutationStatus::Created);
        assert_eq!(duplicate.status(), RoomMutationStatus::Duplicate);
        assert_eq!(created.room(), duplicate.room());

        let added = source
            .add_room_participant("room-2", "codex", "2026-08-12T02:00:01Z")
            .unwrap();
        let duplicate_participant = source
            .add_room_participant("room-2", "codex", "2026-08-12T02:00:02Z")
            .unwrap();
        assert_eq!(added.status(), RoomMutationStatus::Added);
        assert_eq!(
            duplicate_participant.status(),
            RoomMutationStatus::Duplicate
        );
        assert_eq!(source.list_rooms().unwrap().len(), 2);
    }

    #[test]
    fn rejects_conflicting_room_creation_and_unknown_participant() {
        let source = source();
        source
            .create_room(
                RoomCreateDraft::try_new(
                    "room-2".to_owned(),
                    "Room 2".to_owned(),
                    vec!["owner".to_owned()],
                    "2026-08-12T02:00:00Z".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            source.create_room(
                RoomCreateDraft::try_new(
                    "room-2".to_owned(),
                    "Changed".to_owned(),
                    vec!["owner".to_owned()],
                    "2026-08-12T02:00:00Z".to_owned(),
                )
                .unwrap(),
            ),
            Err(RoomMutationError::IdempotencyConflict)
        );
        assert_eq!(
            source.add_room_participant("room-2", "missing", "2026-08-12T02:00:01Z"),
            Err(RoomMutationError::ParticipantNotFound)
        );
    }

    #[test]
    fn renames_removes_and_deletes_rooms_with_safety_guards() {
        let source = source();
        source
            .create_room(
                RoomCreateDraft::try_new(
                    "room-2".to_owned(),
                    "Room 2".to_owned(),
                    vec!["owner".to_owned(), "codex".to_owned(), "gemini".to_owned()],
                    "2026-08-12T03:00:00Z".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();

        let renamed = source
            .rename_room("room-2", "Renamed", "2026-08-12T03:00:01Z")
            .unwrap();
        let duplicate_name = source
            .rename_room("room-2", "Renamed", "2026-08-12T03:00:02Z")
            .unwrap();
        assert_eq!(renamed.status(), RoomMutationStatus::Renamed);
        assert_eq!(duplicate_name.status(), RoomMutationStatus::Duplicate);

        let removed = source
            .remove_room_participant("room-2", "gemini", "2026-08-12T03:00:03Z")
            .unwrap();
        assert_eq!(removed.status(), RoomMutationStatus::Removed);
        assert_eq!(removed.room().participant_ids, ["owner", "codex"]);
        assert_eq!(
            source.remove_room_participant("room-2", "codex", "2026-08-12T03:00:04Z"),
            Err(RoomMutationError::RequiredParticipant)
        );

        let deleted = source
            .delete_room("room-2", "2026-08-12T03:00:05Z")
            .unwrap();
        assert_eq!(deleted.status(), RoomMutationStatus::Deleted);
        assert_eq!(source.list_rooms().unwrap().len(), 1);
        assert_eq!(
            source.delete_room("room-1", "2026-08-12T03:00:06Z"),
            Err(RoomMutationError::RequiredRoom)
        );
    }

    #[test]
    fn refuses_to_remove_a_participant_referenced_by_history() {
        let source = source();
        source
            .add_room_participant("room-1", "codex", "2026-08-12T04:00:00Z")
            .unwrap();
        source
            .add_room_participant("room-1", "gemini", "2026-08-12T04:00:01Z")
            .unwrap();
        source
            .append_message(
                RoomMessageDraft::try_new(
                    "message-client-history".to_owned(),
                    "room-1".to_owned(),
                    "owner".to_owned(),
                    vec!["gemini".to_owned()],
                    "history".to_owned(),
                    "2026-08-12T04:00:02Z".to_owned(),
                    Vec::new(),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            source.remove_room_participant("room-1", "gemini", "2026-08-12T04:00:03Z"),
            Err(RoomMutationError::ParticipantInUse)
        );
    }
}
