use moe_core::{
    InMemoryRoomSource, Room, RoomCatalogError, RoomCatalogSource, RoomCreateDraft, RoomMessage,
    RoomMessageDraft, RoomMessageFindError, RoomMutationError, RoomMutationSuccess,
    RoomParticipant, RoomParticipantKind, RoomReadQuery, RoomReadResult, RoomSnapshot, RoomSource,
    RoomStore, RoomSummary, RoomWriteError, RoomWriteSuccess,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const ROOM_FILE_VERSION: u8 = 1;
const MAXIMUM_ROOM_FILE_BYTES: usize = 64 * 1024 * 1024;
const ROOM_FILE_NAME: &str = "room-snapshot-v1.json";
pub(crate) const OWNER_PARTICIPANT_ID: &str = "owner";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParticipantIdMigration {
    pub(crate) previous_id: String,
    pub(crate) current_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopRoomPersistenceError {
    Io,
    InvalidFile,
}

#[derive(Debug, Clone)]
pub(crate) struct DesktopRoomContext {
    pub(crate) room: Room,
    pub(crate) participant_names: BTreeMap<String, String>,
    pub(crate) participant_kinds: BTreeMap<String, RoomParticipantKind>,
}

impl fmt::Display for DesktopRoomPersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io => "the Room persistence file could not be accessed",
            Self::InvalidFile => "the Room persistence file is invalid",
        })
    }
}

impl Error for DesktopRoomPersistenceError {}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedRoomFile {
    file_version: u8,
    snapshot: RoomSnapshot,
}

struct RoomPersistence {
    path: PathBuf,
}

impl RoomPersistence {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn backup_path(&self) -> PathBuf {
        sibling_path(&self.path, "backup")
    }

    fn load(&self) -> Result<Option<RoomSnapshot>, DesktopRoomPersistenceError> {
        if self.path.is_file() {
            match load_room_file(&self.path) {
                Ok(snapshot) => return Ok(Some(snapshot)),
                Err(DesktopRoomPersistenceError::InvalidFile) => {
                    let backup = self.backup_path();
                    if !backup.is_file() {
                        return Err(DesktopRoomPersistenceError::InvalidFile);
                    }
                    let snapshot = load_room_file(&backup)?;
                    self.preserve_corrupted_primary()?;
                    self.persist(&snapshot)?;
                    return Ok(Some(snapshot));
                }
                Err(error) => return Err(error),
            }
        }
        if self.path.exists() {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }
        let backup = self.backup_path();
        if backup.is_file() {
            return load_room_file(&backup).map(Some);
        }
        if backup.exists() {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }
        Ok(None)
    }

    fn preserve_corrupted_primary(&self) -> Result<(), DesktopRoomPersistenceError> {
        for _ in 0..16 {
            let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let quarantine = sibling_path(
                &self.path,
                &format!("corrupt-{}-{sequence}", std::process::id()),
            );
            if quarantine.exists() {
                continue;
            }
            return fs::rename(&self.path, quarantine).map_err(|_| DesktopRoomPersistenceError::Io);
        }
        Err(DesktopRoomPersistenceError::Io)
    }

    fn persist(&self, snapshot: &RoomSnapshot) -> Result<(), DesktopRoomPersistenceError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(DesktopRoomPersistenceError::InvalidFile)?;
        fs::create_dir_all(parent).map_err(|_| DesktopRoomPersistenceError::Io)?;
        if !parent.is_dir()
            || (self.path.exists() && !self.path.is_file())
            || (self.backup_path().exists() && !self.backup_path().is_file())
        {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }

        let body = serde_json::to_vec(&PersistedRoomFile {
            file_version: ROOM_FILE_VERSION,
            snapshot: snapshot.clone(),
        })
        .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        if body.len() > MAXIMUM_ROOM_FILE_BYTES {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }

        let (temp_path, mut temp_file) = create_temp_file(&self.path)?;
        let mut temp_guard = TempFileGuard::new(temp_path);
        temp_file
            .write_all(&body)
            .and_then(|_| temp_file.sync_all())
            .map_err(|_| DesktopRoomPersistenceError::Io)?;
        drop(temp_file);

        let backup = self.backup_path();
        if self.path.exists() {
            if backup.exists() {
                fs::remove_file(&backup).map_err(|_| DesktopRoomPersistenceError::Io)?;
            }
            fs::rename(&self.path, &backup).map_err(|_| DesktopRoomPersistenceError::Io)?;
        }

        if fs::rename(temp_guard.path(), &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(DesktopRoomPersistenceError::Io);
        }
        temp_guard.keep();
        Ok(())
    }
}

pub(crate) struct DesktopRoomSource {
    inner: InMemoryRoomSource,
    persistence: Option<RoomPersistence>,
    transaction_guard: Mutex<()>,
}

pub(crate) struct PreparedDesktopRoomSource {
    snapshot: RoomSnapshot,
    persistence: RoomPersistence,
    needs_persist: bool,
    owner_migration: Option<ParticipantIdMigration>,
}

impl PreparedDesktopRoomSource {
    pub(crate) fn owner_migration(&self) -> Option<&ParticipantIdMigration> {
        self.owner_migration.as_ref()
    }

    pub(crate) fn commit(self) -> Result<Arc<DesktopRoomSource>, DesktopRoomPersistenceError> {
        if self.needs_persist {
            InMemoryRoomSource::new(self.snapshot.clone())
                .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
            self.persistence.persist(&self.snapshot)?;
        }
        Ok(Arc::new(DesktopRoomSource::persistent(
            self.snapshot,
            self.persistence,
        )?))
    }
}

impl DesktopRoomSource {
    pub(crate) fn owner_participant_id(&self) -> Option<String> {
        let _transaction = self.transaction_guard.lock().ok()?;
        let snapshot = self.inner.snapshot().ok()?;
        let mut humans = snapshot
            .participants
            .iter()
            .filter(|participant| participant.kind == RoomParticipantKind::Human);
        let owner = humans.next()?;
        humans.next().is_none().then(|| owner.id.clone())
    }

    pub(crate) fn is_human_participant(&self, participant_id: &str) -> bool {
        let Ok(_transaction) = self.transaction_guard.lock() else {
            return false;
        };
        self.inner.snapshot().ok().is_some_and(|snapshot| {
            snapshot.participants.iter().any(|participant| {
                participant.id == participant_id && participant.kind == RoomParticipantKind::Human
            })
        })
    }

    pub(crate) fn has_participant(&self, participant_id: &str) -> bool {
        let Ok(_transaction) = self.transaction_guard.lock() else {
            return false;
        };
        self.inner.snapshot().ok().is_some_and(|snapshot| {
            snapshot
                .participants
                .iter()
                .any(|participant| participant.id == participant_id)
        })
    }

    pub(crate) fn room_context(
        &self,
        room_id: &str,
    ) -> Result<DesktopRoomContext, RoomMessageFindError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMessageFindError::SourceUnavailable)?;
        let snapshot = self
            .inner
            .snapshot()
            .map_err(|_| RoomMessageFindError::SourceUnavailable)?;
        let room = snapshot
            .rooms
            .into_iter()
            .find(|room| room.id == room_id)
            .ok_or(RoomMessageFindError::RoomNotFound)?;
        let participant_names = snapshot
            .participants
            .iter()
            .map(|participant| (participant.id.clone(), participant.display_name.clone()))
            .collect();
        let participant_kinds = snapshot
            .participants
            .into_iter()
            .map(|participant| (participant.id, participant.kind))
            .collect();
        Ok(DesktopRoomContext {
            room,
            participant_names,
            participant_kinds,
        })
    }

    #[cfg(test)]
    fn in_memory(snapshot: RoomSnapshot) -> Result<Self, DesktopRoomPersistenceError> {
        let inner = InMemoryRoomSource::new(snapshot)
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        Ok(Self {
            inner,
            persistence: None,
            transaction_guard: Mutex::new(()),
        })
    }

    fn persistent(
        snapshot: RoomSnapshot,
        persistence: RoomPersistence,
    ) -> Result<Self, DesktopRoomPersistenceError> {
        let inner = InMemoryRoomSource::new(snapshot)
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        Ok(Self {
            inner,
            persistence: Some(persistence),
            transaction_guard: Mutex::new(()),
        })
    }

    pub(crate) fn export_snapshot(
        &self,
        destination: PathBuf,
    ) -> Result<(), DesktopRoomPersistenceError> {
        if !valid_external_snapshot_path(&destination) {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| DesktopRoomPersistenceError::Io)?;
        let snapshot = self
            .inner
            .snapshot()
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        RoomPersistence::new(destination).persist(&snapshot)
    }

    pub(crate) fn restore_snapshot(
        &self,
        source: PathBuf,
    ) -> Result<(), DesktopRoomPersistenceError> {
        if !valid_external_snapshot_path(&source) || !source.is_file() {
            return Err(DesktopRoomPersistenceError::InvalidFile);
        }
        let mut snapshot = load_room_file(&source)?;
        normalize_owner_participant(&mut snapshot)?;
        let (snapshot, _) = merge_bundled_catalog(snapshot);
        InMemoryRoomSource::new(snapshot.clone())
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;

        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| DesktopRoomPersistenceError::Io)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        self.inner
            .replace_snapshot(snapshot.clone())
            .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
        if let Some(persistence) = &self.persistence
            && persistence.persist(&snapshot).is_err()
        {
            self.inner
                .replace_snapshot(before)
                .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
            return Err(DesktopRoomPersistenceError::Io);
        }
        Ok(())
    }
}

impl RoomSource for DesktopRoomSource {
    fn read_room(&self, query: &RoomReadQuery) -> RoomReadResult {
        let _transaction = self
            .transaction_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.inner.read_room(query)
    }

    fn find_message(
        &self,
        room_id: &str,
        message_id: &str,
    ) -> Result<RoomMessage, RoomMessageFindError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMessageFindError::SourceUnavailable)?;
        self.inner.find_message(room_id, message_id)
    }
}

impl RoomCatalogSource for DesktopRoomSource {
    fn list_rooms(&self) -> Result<Vec<RoomSummary>, RoomCatalogError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomCatalogError::SourceUnavailable)?;
        self.inner.list_rooms()
    }
}

impl RoomStore for DesktopRoomSource {
    fn append_message(&self, draft: RoomMessageDraft) -> Result<RoomWriteSuccess, RoomWriteError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomWriteError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomWriteError::SourceUnavailable)?;
        let result = self.inner.append_message(draft)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomWriteError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomWriteError::SourceUnavailable)?;
                return Err(RoomWriteError::SourceUnavailable);
            }
        }
        Ok(result)
    }

    fn create_room(
        &self,
        draft: RoomCreateDraft,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let result = self.inner.create_room(draft)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomMutationError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomMutationError::SourceUnavailable)?;
                return Err(RoomMutationError::SourceUnavailable);
            }
        }
        Ok(result)
    }

    fn add_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let result = self
            .inner
            .add_room_participant(room_id, participant_id, updated_at)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomMutationError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomMutationError::SourceUnavailable)?;
                return Err(RoomMutationError::SourceUnavailable);
            }
        }
        Ok(result)
    }

    fn rename_room(
        &self,
        room_id: &str,
        name: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let result = self.inner.rename_room(room_id, name, updated_at)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomMutationError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomMutationError::SourceUnavailable)?;
                return Err(RoomMutationError::SourceUnavailable);
            }
        }
        Ok(result)
    }

    fn remove_room_participant(
        &self,
        room_id: &str,
        participant_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let result = self
            .inner
            .remove_room_participant(room_id, participant_id, updated_at)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomMutationError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomMutationError::SourceUnavailable)?;
                return Err(RoomMutationError::SourceUnavailable);
            }
        }
        Ok(result)
    }

    fn delete_room(
        &self,
        room_id: &str,
        updated_at: &str,
    ) -> Result<RoomMutationSuccess, RoomMutationError> {
        let _transaction = self
            .transaction_guard
            .lock()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let before = self
            .inner
            .snapshot()
            .map_err(|_| RoomMutationError::SourceUnavailable)?;
        let result = self.inner.delete_room(room_id, updated_at)?;
        if let Some(persistence) = &self.persistence {
            let snapshot = self
                .inner
                .snapshot()
                .map_err(|_| RoomMutationError::SourceUnavailable)?;
            if persistence.persist(&snapshot).is_err() {
                self.inner
                    .replace_snapshot(before)
                    .map_err(|_| RoomMutationError::SourceUnavailable)?;
                return Err(RoomMutationError::SourceUnavailable);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
pub(crate) fn desktop_room_source() -> Arc<DesktopRoomSource> {
    Arc::new(
        DesktopRoomSource::in_memory(bundled_snapshot())
            .expect("bundled Desktop Room snapshot must be valid"),
    )
}

#[cfg(test)]
pub(crate) fn persistent_desktop_room_source(
    path: PathBuf,
) -> Result<Arc<DesktopRoomSource>, DesktopRoomPersistenceError> {
    prepare_persistent_desktop_room_source(path)?.commit()
}

pub(crate) fn prepare_persistent_desktop_room_source(
    path: PathBuf,
) -> Result<PreparedDesktopRoomSource, DesktopRoomPersistenceError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }
    let persistence = RoomPersistence::new(path);
    let loaded = persistence.load()?;
    let (snapshot, upgraded, owner_migration) = match loaded {
        Some(mut snapshot) => {
            let (owner_migration, owner_upgraded) = normalize_owner_participant(&mut snapshot)?;
            let (snapshot, catalog_upgraded) = merge_bundled_catalog(snapshot);
            (
                snapshot,
                catalog_upgraded || owner_upgraded,
                owner_migration,
            )
        }
        None => (bundled_snapshot(), false, None),
    };
    InMemoryRoomSource::new(snapshot.clone())
        .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
    Ok(PreparedDesktopRoomSource {
        snapshot,
        persistence,
        needs_persist: upgraded,
        owner_migration,
    })
}

pub(crate) fn product_room_file(app_data_dir: &Path) -> PathBuf {
    std::env::var_os("MOE_ROOM_DATA_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(ROOM_FILE_NAME))
}

fn bundled_snapshot() -> RoomSnapshot {
    RoomSnapshot {
        schema_version: moe_protocol::PROTOCOL_VERSION.to_owned(),
        generated_at: "2026-08-12T00:00:00+09:00".to_owned(),
        participants: vec![
            participant(OWNER_PARTICIPANT_ID, "Owner", RoomParticipantKind::Human),
            participant("claude-web", "Claude Web", RoomParticipantKind::Ai),
            participant("claude-code", "Claude Code", RoomParticipantKind::Ai),
            participant("codex", "Codex", RoomParticipantKind::Ai),
            participant("chatgpt", "ChatGPT", RoomParticipantKind::Ai),
            participant("grok", "Grok", RoomParticipantKind::Ai),
            participant("gemini", "Gemini", RoomParticipantKind::Ai),
            participant("openai-api", "OpenAI API", RoomParticipantKind::Ai),
            participant("generic-mcp", "Generic MCP", RoomParticipantKind::Ai),
            participant("other", "その他", RoomParticipantKind::Ai),
        ],
        rooms: vec![
            Room {
                id: "moe-dev-room".to_owned(),
                name: "M.I.O.開発室".to_owned(),
                participant_ids: vec![
                    OWNER_PARTICIPANT_ID.to_owned(),
                    "claude-web".to_owned(),
                    "codex".to_owned(),
                    "gemini".to_owned(),
                ],
                messages: vec![
                    message(
                        "moe-dev-room",
                        "welcome-1",
                        OWNER_PARTICIPANT_ID,
                        &["claude-web", "codex", "gemini"],
                        "みんな、M.I.O.のトークルームUIを作っていきましょうー。",
                        "2026-08-11T19:42:00+09:00",
                    ),
                    message(
                        "moe-dev-room",
                        "welcome-2",
                        "codex",
                        &[OWNER_PARTICIPANT_ID],
                        "了解です。ルーム参加者と、メッセージごとの宛先を分けておきますね。",
                        "2026-08-11T19:42:30+09:00",
                    ),
                    message(
                        "moe-dev-room",
                        "welcome-3",
                        "claude-web",
                        &[OWNER_PARTICIPANT_ID],
                        "背景やアイコンを自分好みにできる余白も、最初から確保しておきます。",
                        "2026-08-11T19:43:00+09:00",
                    ),
                ],
            },
            Room {
                id: "comparison-room".to_owned(),
                name: "回答くらべ部屋".to_owned(),
                participant_ids: vec![
                    OWNER_PARTICIPANT_ID.to_owned(),
                    "chatgpt".to_owned(),
                    "grok".to_owned(),
                    "gemini".to_owned(),
                ],
                messages: vec![message(
                    "comparison-room",
                    "comparison-1",
                    OWNER_PARTICIPANT_ID,
                    &["chatgpt", "grok", "gemini"],
                    "同じ質問を三人に投げて、答えを見てみたい。",
                    "2026-08-11T19:30:00+09:00",
                )],
            },
            Room {
                id: "mcp-lab".to_owned(),
                name: "MCP実験室".to_owned(),
                participant_ids: vec![
                    OWNER_PARTICIPANT_ID.to_owned(),
                    "generic-mcp".to_owned(),
                    "openai-api".to_owned(),
                ],
                messages: Vec::new(),
            },
        ],
    }
}

fn normalize_owner_participant(
    snapshot: &mut RoomSnapshot,
) -> Result<(Option<ParticipantIdMigration>, bool), DesktopRoomPersistenceError> {
    let human_indexes = snapshot
        .participants
        .iter()
        .enumerate()
        .filter_map(|(index, participant)| {
            (participant.kind == RoomParticipantKind::Human).then_some(index)
        })
        .collect::<Vec<_>>();
    if human_indexes.len() != 1 {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }
    let human_index = human_indexes[0];
    let mut upgraded = false;
    if snapshot.participants[human_index].display_name != "Owner" {
        snapshot.participants[human_index].display_name = "Owner".to_owned();
        upgraded = true;
    }
    let previous_id = snapshot.participants[human_index].id.clone();
    if previous_id == OWNER_PARTICIPANT_ID {
        return Ok((None, upgraded));
    }
    if snapshot
        .participants
        .iter()
        .any(|participant| participant.id == OWNER_PARTICIPANT_ID)
    {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }

    snapshot.participants[human_index].id = OWNER_PARTICIPANT_ID.to_owned();
    upgraded = true;
    for room in &mut snapshot.rooms {
        for participant_id in &mut room.participant_ids {
            if participant_id == &previous_id {
                *participant_id = OWNER_PARTICIPANT_ID.to_owned();
            }
        }
        for message in &mut room.messages {
            if message.author_id == previous_id {
                message.author_id = OWNER_PARTICIPANT_ID.to_owned();
            }
            for recipient in &mut message.recipients {
                if recipient == &previous_id {
                    *recipient = OWNER_PARTICIPANT_ID.to_owned();
                }
            }
        }
    }
    Ok((
        Some(ParticipantIdMigration {
            previous_id,
            current_id: OWNER_PARTICIPANT_ID.to_owned(),
        }),
        upgraded,
    ))
}

fn merge_bundled_catalog(mut snapshot: RoomSnapshot) -> (RoomSnapshot, bool) {
    let bundled = bundled_snapshot();
    let mut upgraded = false;
    for participant in bundled.participants {
        if !snapshot
            .participants
            .iter()
            .any(|existing| existing.id == participant.id)
        {
            snapshot.participants.push(participant);
            upgraded = true;
        }
    }
    for room in bundled.rooms {
        if !snapshot.rooms.iter().any(|existing| existing.id == room.id) {
            snapshot.rooms.push(room);
            upgraded = true;
        }
    }
    (snapshot, upgraded)
}

fn load_room_file(path: &Path) -> Result<RoomSnapshot, DesktopRoomPersistenceError> {
    let file = File::open(path).map_err(|_| DesktopRoomPersistenceError::Io)?;
    let metadata = file
        .metadata()
        .map_err(|_| DesktopRoomPersistenceError::Io)?;
    if metadata.len() > MAXIMUM_ROOM_FILE_BYTES as u64 {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }
    let mut body = Vec::with_capacity(metadata.len() as usize);
    file.take((MAXIMUM_ROOM_FILE_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| DesktopRoomPersistenceError::Io)?;
    if body.len() > MAXIMUM_ROOM_FILE_BYTES {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }
    let persisted: PersistedRoomFile =
        serde_json::from_slice(&body).map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
    if persisted.file_version != ROOM_FILE_VERSION {
        return Err(DesktopRoomPersistenceError::InvalidFile);
    }
    InMemoryRoomSource::new(persisted.snapshot.clone())
        .map_err(|_| DesktopRoomPersistenceError::InvalidFile)?;
    Ok(persisted.snapshot)
}

fn create_temp_file(path: &Path) -> Result<(PathBuf, File), DesktopRoomPersistenceError> {
    for _ in 0..16 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let suffix = format!("{}.{}.tmp", std::process::id(), sequence);
        let temp_path = sibling_path(path, &suffix);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(DesktopRoomPersistenceError::Io),
        }
    }
    Err(DesktopRoomPersistenceError::Io)
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!("{file_name}.{suffix}"))
}

fn valid_external_snapshot_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.strip_prefix("moe-room-backup-")
                    .and_then(|value| value.strip_suffix(".json"))
                    .is_some_and(|stamp| {
                        stamp.len() == 20 && stamp.bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
}

struct TempFileGuard {
    path: PathBuf,
    remove_on_drop: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            remove_on_drop: true,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn keep(&mut self) {
        self.remove_on_drop = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn participant(id: &str, display_name: &str, kind: RoomParticipantKind) -> RoomParticipant {
    RoomParticipant {
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        kind,
    }
}

fn message(
    room_id: &str,
    id: &str,
    author_id: &str,
    recipients: &[&str],
    body: &str,
    created_at: &str,
) -> RoomMessage {
    RoomMessage {
        id: id.to_owned(),
        room_id: room_id.to_owned(),
        author_id: author_id.to_owned(),
        recipients: recipients.iter().map(|value| (*value).to_owned()).collect(),
        body: body.to_owned(),
        created_at: created_at.to_owned(),
        artifact_ids: Vec::new(),
        provenance: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn draft(id: &str, body: &str) -> RoomMessageDraft {
        RoomMessageDraft::try_new(
            id.to_owned(),
            "moe-dev-room".to_owned(),
            OWNER_PARTICIPANT_ID.to_owned(),
            vec!["codex".to_owned()],
            body.to_owned(),
            "2026-08-12T15:00:00Z".to_owned(),
            Vec::new(),
        )
        .unwrap()
    }

    fn ai_draft(id: &str, body: &str) -> RoomMessageDraft {
        RoomMessageDraft::try_new(
            id.to_owned(),
            "moe-dev-room".to_owned(),
            "codex".to_owned(),
            vec![OWNER_PARTICIPANT_ID.to_owned()],
            body.to_owned(),
            "2026-08-12T15:00:01Z".to_owned(),
            Vec::new(),
        )
        .unwrap()
    }

    fn isolated_file(label: &str) -> (PathBuf, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "moe-room-persistence-{label}-{}-{nonce}",
            std::process::id()
        ));
        (root.join(ROOM_FILE_NAME), root)
    }

    fn cleanup(root: &Path, file: &Path) {
        assert!(root.starts_with(std::env::temp_dir()));
        let _ = fs::remove_file(file);
        let _ = fs::remove_file(sibling_path(file, "backup"));
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let _ = fs::remove_file(path);
                }
            }
        }
        let _ = fs::remove_dir(root);
    }

    #[test]
    fn bundled_source_matches_the_current_development_room_snapshot() {
        let catalog = desktop_room_source().list_rooms().unwrap();
        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog[1].id, "comparison-room");
        assert_eq!(catalog[2].id, "mcp-lab");
        let query =
            RoomReadQuery::try_new("moe-dev-room".to_owned(), Some("welcome-2".to_owned()), 1)
                .unwrap();
        let result = serde_json::to_value(desktop_room_source().read_room(&query)).unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["room"]["name"], "M.I.O.開発室");
        assert_eq!(result["room"]["messages"][0]["id"], "welcome-3");
        assert_eq!(result["page"]["nextAfterMessageId"], "welcome-3");
    }

    #[test]
    fn persists_created_rooms_and_added_participants() {
        let (file, root) = isolated_file("room-mutations");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source
            .create_room(
                RoomCreateDraft::try_new(
                    "persisted-room".to_owned(),
                    "Persisted Room".to_owned(),
                    vec![OWNER_PARTICIPANT_ID.to_owned(), "codex".to_owned()],
                    "2026-08-12T15:00:02Z".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();
        source
            .add_room_participant("persisted-room", "gemini", "2026-08-12T15:00:03Z")
            .unwrap();
        drop(source);

        let reloaded = persistent_desktop_room_source(file.clone()).unwrap();
        let room = reloaded
            .list_rooms()
            .unwrap()
            .into_iter()
            .find(|room| room.id == "persisted-room")
            .unwrap();
        assert_eq!(room.name, "Persisted Room");
        assert_eq!(
            room.participant_ids,
            [OWNER_PARTICIPANT_ID, "codex", "gemini"]
        );

        drop(reloaded);
        cleanup(&root, &file);
    }

    #[test]
    fn persists_room_rename_participant_removal_and_deletion() {
        let (file, root) = isolated_file("room-management");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source
            .create_room(
                RoomCreateDraft::try_new(
                    "managed-room".to_owned(),
                    "Managed Room".to_owned(),
                    vec![
                        OWNER_PARTICIPANT_ID.to_owned(),
                        "codex".to_owned(),
                        "gemini".to_owned(),
                    ],
                    "2026-08-12T15:10:00Z".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();
        source
            .rename_room("managed-room", "Renamed Room", "2026-08-12T15:10:01Z")
            .unwrap();
        source
            .remove_room_participant("managed-room", "gemini", "2026-08-12T15:10:02Z")
            .unwrap();
        drop(source);

        let reloaded = persistent_desktop_room_source(file.clone()).unwrap();
        let room = reloaded
            .list_rooms()
            .unwrap()
            .into_iter()
            .find(|room| room.id == "managed-room")
            .unwrap();
        assert_eq!(room.name, "Renamed Room");
        assert_eq!(room.participant_ids, [OWNER_PARTICIPANT_ID, "codex"]);
        reloaded
            .delete_room("managed-room", "2026-08-12T15:10:03Z")
            .unwrap();
        drop(reloaded);

        let deleted = persistent_desktop_room_source(file.clone()).unwrap();
        assert!(
            deleted
                .list_rooms()
                .unwrap()
                .iter()
                .all(|room| room.id != "managed-room")
        );
        drop(deleted);
        cleanup(&root, &file);
    }

    #[test]
    fn upgrades_an_existing_single_room_file_without_losing_messages() {
        let (file, root) = isolated_file("catalog-upgrade");
        let mut legacy = bundled_snapshot();
        legacy.rooms.truncate(1);
        legacy.participants.retain(|participant| {
            [OWNER_PARTICIPANT_ID, "claude-web", "codex", "gemini"]
                .contains(&participant.id.as_str())
        });
        legacy.rooms[0].messages.push(message(
            "moe-dev-room",
            "existing-message",
            OWNER_PARTICIPANT_ID,
            &["codex"],
            "preserve me",
            "2026-08-12T15:00:04Z",
        ));
        RoomPersistence::new(file.clone()).persist(&legacy).unwrap();

        let upgraded = persistent_desktop_room_source(file.clone()).unwrap();
        assert_eq!(upgraded.list_rooms().unwrap().len(), 3);
        assert_eq!(
            upgraded
                .find_message("moe-dev-room", "existing-message")
                .unwrap()
                .body,
            "preserve me"
        );
        drop(upgraded);

        let persisted = load_room_file(&file).unwrap();
        assert_eq!(persisted.rooms.len(), 3);
        assert_eq!(persisted.participants.len(), 10);
        cleanup(&root, &file);
    }

    #[test]
    fn migrates_an_arbitrary_human_id_without_losing_room_history() {
        let (file, root) = isolated_file("owner-id-migration");
        let previous_owner_id = "local-user";
        let mut snapshot = bundled_snapshot();
        let human = snapshot
            .participants
            .iter_mut()
            .find(|participant| participant.kind == RoomParticipantKind::Human)
            .unwrap();
        human.id = previous_owner_id.to_owned();
        human.display_name = "Personal alias".to_owned();
        for room in &mut snapshot.rooms {
            for participant_id in &mut room.participant_ids {
                if participant_id == OWNER_PARTICIPANT_ID {
                    *participant_id = previous_owner_id.to_owned();
                }
            }
            for message in &mut room.messages {
                if message.author_id == OWNER_PARTICIPANT_ID {
                    message.author_id = previous_owner_id.to_owned();
                }
                for recipient in &mut message.recipients {
                    if recipient == OWNER_PARTICIPANT_ID {
                        *recipient = previous_owner_id.to_owned();
                    }
                }
            }
        }
        RoomPersistence::new(file.clone())
            .persist(&snapshot)
            .unwrap();

        let prepared = prepare_persistent_desktop_room_source(file.clone()).unwrap();
        assert_eq!(
            prepared.owner_migration(),
            Some(&ParticipantIdMigration {
                previous_id: previous_owner_id.to_owned(),
                current_id: OWNER_PARTICIPANT_ID.to_owned(),
            })
        );
        let source = prepared.commit().unwrap();
        assert_eq!(
            source.owner_participant_id().as_deref(),
            Some(OWNER_PARTICIPANT_ID)
        );
        let welcome = source.find_message("moe-dev-room", "welcome-1").unwrap();
        assert_eq!(welcome.author_id, OWNER_PARTICIPANT_ID);
        let reply = source.find_message("moe-dev-room", "welcome-2").unwrap();
        assert_eq!(reply.recipients, [OWNER_PARTICIPANT_ID]);
        drop(source);

        let persisted = load_room_file(&file).unwrap();
        assert!(
            persisted
                .participants
                .iter()
                .all(|participant| participant.id != previous_owner_id)
        );
        assert_eq!(
            persisted
                .participants
                .iter()
                .find(|participant| participant.kind == RoomParticipantKind::Human)
                .unwrap()
                .display_name,
            "Owner"
        );
        cleanup(&root, &file);
    }

    #[test]
    fn persists_and_reloads_an_idempotent_room_message() {
        let (file, root) = isolated_file("reload");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source
            .append_message(draft("persisted-message-1", "再起動後も残ります"))
            .unwrap();
        source
            .append_message(ai_draft("persisted-reply-1", "AI応答も残ります"))
            .unwrap();
        drop(source);

        let reloaded = persistent_desktop_room_source(file.clone()).unwrap();
        let message = reloaded
            .find_message("moe-dev-room", "persisted-message-1")
            .unwrap();
        assert_eq!(message.body, "再起動後も残ります");
        let retry = reloaded
            .append_message(draft("persisted-message-1", "再起動後も残ります"))
            .unwrap();
        assert_eq!(retry.status(), moe_core::RoomWriteStatus::Duplicate);
        let reply = reloaded
            .find_message("moe-dev-room", "persisted-reply-1")
            .unwrap();
        assert_eq!(reply.author_id, "codex");
        assert_eq!(reply.body, "AI応答も残ります");

        drop(reloaded);
        cleanup(&root, &file);
    }

    #[test]
    fn loads_the_backup_if_a_crash_left_the_primary_missing() {
        let (file, root) = isolated_file("backup");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source
            .append_message(draft("persisted-message-1", "first"))
            .unwrap();
        source
            .append_message(draft("persisted-message-2", "second"))
            .unwrap();
        drop(source);

        fs::remove_file(&file).unwrap();
        let recovered = persistent_desktop_room_source(file.clone()).unwrap();
        assert!(
            recovered
                .find_message("moe-dev-room", "persisted-message-1")
                .is_ok()
        );
        assert_eq!(
            recovered.find_message("moe-dev-room", "persisted-message-2"),
            Err(RoomMessageFindError::MessageNotFound)
        );

        drop(recovered);
        cleanup(&root, &file);
    }

    #[test]
    fn quarantines_a_corrupted_primary_and_recovers_the_valid_backup() {
        let (file, root) = isolated_file("corrupt");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source
            .append_message(draft("persisted-message-1", "first"))
            .unwrap();
        source
            .append_message(draft("persisted-message-2", "second"))
            .unwrap();
        drop(source);

        fs::write(&file, b"not-json").unwrap();
        let recovered = persistent_desktop_room_source(file.clone()).unwrap();
        assert!(
            recovered
                .find_message("moe-dev-room", "persisted-message-1")
                .is_ok()
        );
        assert_eq!(
            recovered.find_message("moe-dev-room", "persisted-message-2"),
            Err(RoomMessageFindError::MessageNotFound)
        );
        assert!(load_room_file(&file).is_ok());
        let quarantined = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("room-snapshot-v1.json.corrupt-")
            })
            .count();
        assert_eq!(quarantined, 1);

        cleanup(&root, &file);
    }

    #[test]
    fn export_and_restore_are_validated_and_persisted() {
        let (file, root) = isolated_file("export-restore");
        let export = root.join("moe-room-backup-00000000000000000001.json");
        let source = persistent_desktop_room_source(file.clone()).unwrap();
        source.export_snapshot(export.clone()).unwrap();
        source
            .rename_room("moe-dev-room", "Changed", "2026-08-12T16:30:00Z")
            .unwrap();
        source.restore_snapshot(export).unwrap();
        assert_eq!(source.list_rooms().unwrap()[0].name, "M.I.O.開発室");
        drop(source);

        let reloaded = persistent_desktop_room_source(file.clone()).unwrap();
        assert_eq!(reloaded.list_rooms().unwrap()[0].name, "M.I.O.開発室");
        assert_eq!(
            reloaded.export_snapshot(root.join("not-a-moe-backup.json")),
            Err(DesktopRoomPersistenceError::InvalidFile)
        );

        drop(reloaded);
        cleanup(&root, &file);
    }

    #[test]
    fn rolls_memory_back_when_the_persistent_write_fails() {
        let (file, root) = isolated_file("rollback");
        fs::create_dir_all(&root).unwrap();
        let blocked_parent = root.join("not-a-directory");
        fs::write(&blocked_parent, b"block").unwrap();
        let blocked_file = blocked_parent.join(ROOM_FILE_NAME);
        let source = persistent_desktop_room_source(blocked_file).unwrap();

        assert_eq!(
            source.append_message(draft("persisted-message-1", "must roll back")),
            Err(RoomWriteError::SourceUnavailable)
        );
        assert_eq!(
            source.find_message("moe-dev-room", "persisted-message-1"),
            Err(RoomMessageFindError::MessageNotFound)
        );

        drop(source);
        fs::remove_file(blocked_parent).unwrap();
        cleanup(&root, &file);
    }
}
