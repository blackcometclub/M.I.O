use crate::room_source::DesktopRoomSource;
use moe_core::{ConductorCapabilities, RoomMessageFindError, RoomParticipantKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

const FILE_VERSION: u8 = 1;
const FILE_NAME: &str = "room-conductor-settings-v1.json";
const MAXIMUM_FILE_BYTES: usize = 256 * 1024;
const MAXIMUM_ROOM_SETTINGS: usize = 1_000;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConductorSendMode {
    Direct,
    Conductor,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RoomConductorStatus {
    ok: bool,
    pub(crate) room_id: String,
    pub(crate) conductor_id: Option<String>,
    pub(crate) send_mode: ConductorSendMode,
}

impl RoomConductorStatus {
    fn direct(room_id: &str) -> Self {
        Self {
            ok: true,
            room_id: room_id.to_owned(),
            conductor_id: None,
            send_mode: ConductorSendMode::Direct,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedRoomConductor {
    room_id: String,
    conductor_id: String,
    send_mode: ConductorSendMode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedFile {
    file_version: u8,
    rooms: Vec<PersistedRoomConductor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomConductorSettingsError {
    code: &'static str,
    message: &'static str,
}

impl fmt::Display for RoomConductorSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for RoomConductorSettingsError {}

fn error(code: &'static str, message: &'static str) -> RoomConductorSettingsError {
    RoomConductorSettingsError { code, message }
}

fn invalid() -> RoomConductorSettingsError {
    error(
        "invalidRoomConductorSettings",
        "The Room conductor settings are invalid.",
    )
}

fn unavailable() -> RoomConductorSettingsError {
    error(
        "roomConductorSettingsUnavailable",
        "The Room conductor settings are temporarily unavailable.",
    )
}

struct SettingsPersistence {
    path: PathBuf,
}

impl SettingsPersistence {
    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.backup")
    }

    fn load(&self) -> Result<BTreeMap<String, PersistedRoomConductor>, RoomConductorSettingsError> {
        if !self.path.exists() && !self.backup_path().exists() {
            return Ok(BTreeMap::new());
        }
        let persisted = read_file(&self.path).or_else(|_| read_file(&self.backup_path()))?;
        if persisted.file_version != FILE_VERSION || persisted.rooms.len() > MAXIMUM_ROOM_SETTINGS {
            return Err(invalid());
        }
        let mut rooms = BTreeMap::new();
        for room in persisted.rooms {
            validate_room_setting(&room)?;
            if rooms.insert(room.room_id.clone(), room).is_some() {
                return Err(invalid());
            }
        }
        Ok(rooms)
    }

    fn persist(
        &self,
        rooms: &BTreeMap<String, PersistedRoomConductor>,
    ) -> Result<(), RoomConductorSettingsError> {
        if rooms.len() > MAXIMUM_ROOM_SETTINGS {
            return Err(invalid());
        }
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(invalid)?;
        fs::create_dir_all(parent).map_err(|_| unavailable())?;
        if !parent.is_dir() || (self.path.exists() && !self.path.is_file()) {
            return Err(invalid());
        }
        let body = serde_json::to_vec(&PersistedFile {
            file_version: FILE_VERSION,
            rooms: rooms.values().cloned().collect(),
        })
        .map_err(|_| invalid())?;
        if body.len() > MAXIMUM_FILE_BYTES {
            return Err(invalid());
        }

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{}.{}.{sequence}.tmp",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(invalid)?,
            std::process::id()
        ));
        let mut guard = TempGuard::new(temp.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| unavailable())?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|_| unavailable())?;
        drop(file);

        let backup = self.backup_path();
        if backup.exists() && !backup.is_file() {
            return Err(invalid());
        }
        if self.path.exists() {
            if !self.path.is_file() {
                return Err(invalid());
            }
            if read_file(&self.path).is_ok() {
                if backup.exists() {
                    fs::remove_file(&backup).map_err(|_| unavailable())?;
                }
                fs::rename(&self.path, &backup).map_err(|_| unavailable())?;
            } else {
                fs::remove_file(&self.path).map_err(|_| unavailable())?;
            }
        }
        if fs::rename(&temp, &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(unavailable());
        }
        guard.keep();
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<PersistedFile, RoomConductorSettingsError> {
    if !path.is_file() {
        return Err(invalid());
    }
    let file = File::open(path).map_err(|_| unavailable())?;
    let size = file.metadata().map_err(|_| unavailable())?.len();
    if size == 0 || size > MAXIMUM_FILE_BYTES as u64 {
        return Err(invalid());
    }
    let mut body = Vec::with_capacity(size as usize);
    file.take(MAXIMUM_FILE_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|_| unavailable())?;
    serde_json::from_slice(&body).map_err(|_| invalid())
}

struct TempGuard {
    path: PathBuf,
    keep: bool,
}

impl TempGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) struct DesktopRoomConductorSettings {
    rooms: Mutex<BTreeMap<String, PersistedRoomConductor>>,
    persistence: Option<SettingsPersistence>,
}

impl DesktopRoomConductorSettings {
    #[cfg(test)]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            rooms: Mutex::new(BTreeMap::new()),
            persistence: None,
        })
    }

    fn persistent(path: PathBuf) -> Result<Arc<Self>, RoomConductorSettingsError> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(invalid());
        }
        let persistence = SettingsPersistence { path };
        let rooms = persistence.load()?;
        Ok(Arc::new(Self {
            rooms: Mutex::new(rooms),
            persistence: Some(persistence),
        }))
    }

    fn get(&self, room_id: &str) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        self.rooms.lock().map_err(|_| unavailable()).map(|rooms| {
            rooms
                .get(room_id)
                .map(|room| RoomConductorStatus {
                    ok: true,
                    room_id: room.room_id.clone(),
                    conductor_id: Some(room.conductor_id.clone()),
                    send_mode: room.send_mode,
                })
                .unwrap_or_else(|| RoomConductorStatus::direct(room_id))
        })
    }

    pub(crate) fn set_conductor(
        &self,
        room_id: &str,
        conductor_id: &str,
    ) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
        if !valid_identifier(room_id) || !valid_identifier(conductor_id) {
            return Err(invalid());
        }
        let mut rooms = self.rooms.lock().map_err(|_| unavailable())?;
        if rooms
            .get(room_id)
            .is_some_and(|room| room.conductor_id == conductor_id)
        {
            return Ok(RoomConductorStatus {
                ok: true,
                room_id: room_id.to_owned(),
                conductor_id: Some(conductor_id.to_owned()),
                send_mode: rooms[room_id].send_mode,
            });
        }
        let mut next = rooms.clone();
        next.insert(
            room_id.to_owned(),
            PersistedRoomConductor {
                room_id: room_id.to_owned(),
                conductor_id: conductor_id.to_owned(),
                send_mode: ConductorSendMode::Conductor,
            },
        );
        self.persist(&next)?;
        *rooms = next;
        Ok(RoomConductorStatus {
            ok: true,
            room_id: room_id.to_owned(),
            conductor_id: Some(conductor_id.to_owned()),
            send_mode: ConductorSendMode::Conductor,
        })
    }

    pub(crate) fn set_mode(
        &self,
        room_id: &str,
        send_mode: ConductorSendMode,
    ) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let mut rooms = self.rooms.lock().map_err(|_| unavailable())?;
        let current = rooms.get(room_id).ok_or_else(|| {
            error(
                "roomConductorNotConfigured",
                "This Room does not have a configured conductor.",
            )
        })?;
        if current.send_mode == send_mode {
            return Ok(RoomConductorStatus {
                ok: true,
                room_id: current.room_id.clone(),
                conductor_id: Some(current.conductor_id.clone()),
                send_mode,
            });
        }
        let mut next = rooms.clone();
        let updated = next.get_mut(room_id).ok_or_else(invalid)?;
        updated.send_mode = send_mode;
        let status = RoomConductorStatus {
            ok: true,
            room_id: updated.room_id.clone(),
            conductor_id: Some(updated.conductor_id.clone()),
            send_mode,
        };
        self.persist(&next)?;
        *rooms = next;
        Ok(status)
    }

    fn clear(&self, room_id: &str) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
        if !valid_identifier(room_id) {
            return Err(invalid());
        }
        let mut rooms = self.rooms.lock().map_err(|_| unavailable())?;
        if !rooms.contains_key(room_id) {
            return Ok(RoomConductorStatus::direct(room_id));
        }
        let mut next = rooms.clone();
        next.remove(room_id);
        self.persist(&next)?;
        *rooms = next;
        Ok(RoomConductorStatus::direct(room_id))
    }

    fn persist(
        &self,
        rooms: &BTreeMap<String, PersistedRoomConductor>,
    ) -> Result<(), RoomConductorSettingsError> {
        if let Some(persistence) = &self.persistence {
            persistence.persist(rooms)?;
        }
        Ok(())
    }
}

pub(crate) struct DesktopConductorCapabilities {
    participants: BTreeMap<String, ConductorCapabilities>,
}

impl DesktopConductorCapabilities {
    fn product() -> Arc<Self> {
        Arc::new(Self {
            participants: BTreeMap::from([(
                "codex".to_owned(),
                ConductorCapabilities {
                    conductor_plan_v1: true,
                },
            )]),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_conductor(participant_id: &str) -> Arc<Self> {
        Arc::new(Self {
            participants: BTreeMap::from([(
                participant_id.to_owned(),
                ConductorCapabilities {
                    conductor_plan_v1: true,
                },
            )]),
        })
    }

    #[cfg(test)]
    fn without_conductors() -> Arc<Self> {
        Arc::new(Self {
            participants: BTreeMap::new(),
        })
    }

    fn supports_conductor(&self, participant_id: &str) -> bool {
        self.participants
            .get(participant_id)
            .is_some_and(|capabilities| capabilities.conductor_plan_v1)
    }
}

pub(crate) fn room_status(
    source: &DesktopRoomSource,
    settings: &DesktopRoomConductorSettings,
    capabilities: &DesktopConductorCapabilities,
    room_id: &str,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    let context = match source.room_context(room_id) {
        Ok(context) => context,
        Err(RoomMessageFindError::RoomNotFound) => {
            settings.clear(room_id)?;
            return Err(map_room_error(RoomMessageFindError::RoomNotFound));
        }
        Err(source_error) => return Err(map_room_error(source_error)),
    };
    let status = settings.get(room_id)?;
    let Some(conductor_id) = status.conductor_id.as_deref() else {
        return Ok(status);
    };
    let valid_member = context
        .room
        .participant_ids
        .iter()
        .any(|participant_id| participant_id == conductor_id);
    let is_ai = context.participant_kinds.get(conductor_id) == Some(&RoomParticipantKind::Ai);
    if valid_member && is_ai && capabilities.supports_conductor(conductor_id) {
        Ok(status)
    } else {
        settings.clear(room_id)
    }
}

fn configure_room_conductor(
    source: &DesktopRoomSource,
    settings: &DesktopRoomConductorSettings,
    capabilities: &DesktopConductorCapabilities,
    room_id: &str,
    conductor_id: &str,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    let context = source.room_context(room_id).map_err(map_room_error)?;
    if !context
        .room
        .participant_ids
        .iter()
        .any(|participant_id| participant_id == conductor_id)
    {
        return Err(error(
            "roomConductorParticipantNotFound",
            "The selected conductor is not participating in this Room.",
        ));
    }
    if context.participant_kinds.get(conductor_id) != Some(&RoomParticipantKind::Ai) {
        return Err(error(
            "roomConductorRequiresAi",
            "Only an AI participant can be selected as conductor.",
        ));
    }
    if !capabilities.supports_conductor(conductor_id) {
        return Err(error(
            "roomConductorUnsupported",
            "This AI does not support Room conductor orchestration.",
        ));
    }
    settings.set_conductor(room_id, conductor_id)
}

fn map_room_error(source_error: RoomMessageFindError) -> RoomConductorSettingsError {
    match source_error {
        RoomMessageFindError::RoomNotFound => error(
            "roomConductorRoomNotFound",
            "The requested Room is not available.",
        ),
        _ => unavailable(),
    }
}

fn validate_room_setting(
    setting: &PersistedRoomConductor,
) -> Result<(), RoomConductorSettingsError> {
    if !valid_identifier(&setting.room_id) || !valid_identifier(&setting.conductor_id) {
        return Err(invalid());
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => true,
            b'.' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

pub(crate) fn product_room_conductor_settings_file(app_data_dir: &Path) -> PathBuf {
    env::var_os("MOE_ROOM_CONDUCTOR_SETTINGS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(FILE_NAME))
}

pub(crate) fn persistent_room_conductor_settings(
    path: PathBuf,
) -> Result<Arc<DesktopRoomConductorSettings>, RoomConductorSettingsError> {
    DesktopRoomConductorSettings::persistent(path)
}

pub(crate) fn product_conductor_capabilities() -> Arc<DesktopConductorCapabilities> {
    DesktopConductorCapabilities::product()
}

#[tauri::command]
pub(crate) fn desktop_room_conductor_status(
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomConductorSettings>>,
    capabilities: State<'_, Arc<DesktopConductorCapabilities>>,
    room_id: String,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    room_status(
        source.as_ref(),
        settings.as_ref(),
        capabilities.as_ref(),
        &room_id,
    )
}

#[tauri::command]
pub(crate) fn desktop_room_conductor_set(
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomConductorSettings>>,
    capabilities: State<'_, Arc<DesktopConductorCapabilities>>,
    room_id: String,
    conductor_id: String,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    configure_room_conductor(
        source.as_ref(),
        settings.as_ref(),
        capabilities.as_ref(),
        &room_id,
        &conductor_id,
    )
}

#[tauri::command]
pub(crate) fn desktop_room_conductor_mode_save(
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomConductorSettings>>,
    capabilities: State<'_, Arc<DesktopConductorCapabilities>>,
    room_id: String,
    send_mode: ConductorSendMode,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    room_status(
        source.as_ref(),
        settings.as_ref(),
        capabilities.as_ref(),
        &room_id,
    )?;
    settings.set_mode(&room_id, send_mode)
}

#[tauri::command]
pub(crate) fn desktop_room_conductor_clear(
    settings: State<'_, Arc<DesktopRoomConductorSettings>>,
    room_id: String,
) -> Result<RoomConductorStatus, RoomConductorSettingsError> {
    settings.clear(&room_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room_source::desktop_room_source;

    fn temp_file(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "mio-room-conductor-{label}-{}-{}.json",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn configures_switches_and_clears_a_supported_ai() {
        let source = desktop_room_source();
        let settings = DesktopRoomConductorSettings::in_memory();
        let capabilities = DesktopConductorCapabilities::with_conductor("codex");

        let configured = configure_room_conductor(
            source.as_ref(),
            settings.as_ref(),
            capabilities.as_ref(),
            "moe-dev-room",
            "codex",
        )
        .unwrap();
        assert_eq!(configured.conductor_id.as_deref(), Some("codex"));
        assert_eq!(configured.send_mode, ConductorSendMode::Conductor);

        let direct = settings
            .set_mode("moe-dev-room", ConductorSendMode::Direct)
            .unwrap();
        assert_eq!(direct.send_mode, ConductorSendMode::Direct);
        assert_eq!(
            room_status(
                source.as_ref(),
                settings.as_ref(),
                capabilities.as_ref(),
                "moe-dev-room"
            )
            .unwrap(),
            direct
        );

        let cleared = settings.clear("moe-dev-room").unwrap();
        assert_eq!(cleared, RoomConductorStatus::direct("moe-dev-room"));
    }

    #[test]
    fn product_capabilities_enable_codex_only() {
        let capabilities = DesktopConductorCapabilities::product();
        assert!(capabilities.supports_conductor("codex"));
        assert!(!capabilities.supports_conductor("gemini"));
        assert!(!capabilities.supports_conductor("grok"));
        assert!(!capabilities.supports_conductor("claude-code"));
    }

    #[test]
    fn command_status_serializes_as_a_valid_success_envelope() {
        let value = serde_json::to_value(RoomConductorStatus {
            ok: true,
            room_id: "moe-dev-room".to_owned(),
            conductor_id: Some("codex".to_owned()),
            send_mode: ConductorSendMode::Conductor,
        })
        .unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["roomId"], "moe-dev-room");
        assert_eq!(value["conductorId"], "codex");
        assert_eq!(value["sendMode"], "conductor");
    }

    #[test]
    fn rejects_owner_nonmember_and_unsupported_ai() {
        let source = desktop_room_source();
        let settings = DesktopRoomConductorSettings::in_memory();
        let codex = DesktopConductorCapabilities::with_conductor("codex");
        assert_eq!(
            configure_room_conductor(
                source.as_ref(),
                settings.as_ref(),
                codex.as_ref(),
                "moe-dev-room",
                "owner"
            )
            .unwrap_err()
            .code,
            "roomConductorRequiresAi"
        );
        assert_eq!(
            configure_room_conductor(
                source.as_ref(),
                settings.as_ref(),
                codex.as_ref(),
                "moe-dev-room",
                "missing"
            )
            .unwrap_err()
            .code,
            "roomConductorParticipantNotFound"
        );
        assert_eq!(
            configure_room_conductor(
                source.as_ref(),
                settings.as_ref(),
                DesktopConductorCapabilities::without_conductors().as_ref(),
                "moe-dev-room",
                "codex"
            )
            .unwrap_err()
            .code,
            "roomConductorUnsupported"
        );
    }

    #[test]
    fn persists_device_local_selection_and_last_mode() {
        let path = temp_file("persist");
        let store = DesktopRoomConductorSettings::persistent(path.clone()).unwrap();
        store.set_conductor("moe-dev-room", "codex").unwrap();
        store
            .set_mode("moe-dev-room", ConductorSendMode::Direct)
            .unwrap();
        drop(store);

        let reloaded = DesktopRoomConductorSettings::persistent(path.clone()).unwrap();
        assert_eq!(
            reloaded.get("moe-dev-room").unwrap(),
            RoomConductorStatus {
                ok: true,
                room_id: "moe-dev-room".to_owned(),
                conductor_id: Some("codex".to_owned()),
                send_mode: ConductorSendMode::Direct,
            }
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn recovers_settings_from_backup_and_heals_on_the_next_save() {
        let path = temp_file("recover");
        let store = DesktopRoomConductorSettings::persistent(path.clone()).unwrap();
        store.set_conductor("moe-dev-room", "codex").unwrap();
        store
            .set_mode("moe-dev-room", ConductorSendMode::Direct)
            .unwrap();
        drop(store);

        fs::write(&path, b"not json").unwrap();
        let recovered = DesktopRoomConductorSettings::persistent(path.clone()).unwrap();
        assert_eq!(
            recovered.get("moe-dev-room").unwrap().send_mode,
            ConductorSendMode::Conductor
        );
        recovered
            .set_mode("moe-dev-room", ConductorSendMode::Direct)
            .unwrap();
        drop(recovered);

        let healed = DesktopRoomConductorSettings::persistent(path.clone()).unwrap();
        assert_eq!(
            healed.get("moe-dev-room").unwrap().send_mode,
            ConductorSendMode::Direct
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn clears_an_effective_selection_when_capability_is_lost() {
        let source = desktop_room_source();
        let settings = DesktopRoomConductorSettings::in_memory();
        settings.set_conductor("moe-dev-room", "codex").unwrap();

        let status = room_status(
            source.as_ref(),
            settings.as_ref(),
            DesktopConductorCapabilities::without_conductors().as_ref(),
            "moe-dev-room",
        )
        .unwrap();
        assert_eq!(status, RoomConductorStatus::direct("moe-dev-room"));
        assert_eq!(settings.get("moe-dev-room").unwrap(), status);
    }

    #[test]
    fn clears_stale_settings_for_a_room_that_no_longer_exists() {
        let source = desktop_room_source();
        let settings = DesktopRoomConductorSettings::in_memory();
        settings.set_conductor("deleted-room", "codex").unwrap();

        assert_eq!(
            room_status(
                source.as_ref(),
                settings.as_ref(),
                DesktopConductorCapabilities::with_conductor("codex").as_ref(),
                "deleted-room",
            )
            .unwrap_err()
            .code,
            "roomConductorRoomNotFound"
        );
        assert_eq!(
            settings.get("deleted-room").unwrap(),
            RoomConductorStatus::direct("deleted-room")
        );
    }
}
