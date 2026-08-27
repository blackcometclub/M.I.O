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

const CONTINUITY_FILE_VERSION: u8 = 1;
const CONTINUITY_FILE_NAME: &str = "room-ai-continuity-v1.json";
const MAXIMUM_CONTINUITY_FILE_BYTES: usize = 256 * 1024;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoomAiContinuation {
    pub(crate) session_id: String,
    pub(crate) last_synced_message_id: String,
    pub(crate) environment_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoomAiContinuityError {
    Invalid,
    Io,
}

impl fmt::Display for RoomAiContinuityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "the Room AI continuity state is invalid",
            Self::Io => "the Room AI continuity state could not be accessed",
        })
    }
}

impl Error for RoomAiContinuityError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedBinding {
    room_id: String,
    participant_id: String,
    session_id: String,
    last_synced_message_id: String,
    environment_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedContinuityFile {
    file_version: u8,
    bindings: Vec<PersistedBinding>,
}

type BindingKey = (String, String);

struct ContinuityPersistence {
    path: PathBuf,
}

impl ContinuityPersistence {
    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.backup")
    }

    fn load(&self) -> Result<BTreeMap<BindingKey, RoomAiContinuation>, RoomAiContinuityError> {
        let backup = self.backup_path();
        let source = if self.path.is_file() {
            &self.path
        } else if !self.path.exists() && backup.is_file() {
            &backup
        } else if !self.path.exists() && !backup.exists() {
            return Ok(BTreeMap::new());
        } else {
            return Err(RoomAiContinuityError::Invalid);
        };
        let file = File::open(source).map_err(|_| RoomAiContinuityError::Io)?;
        let size = file
            .metadata()
            .map_err(|_| RoomAiContinuityError::Io)?
            .len();
        if size == 0 || size > MAXIMUM_CONTINUITY_FILE_BYTES as u64 {
            return Err(RoomAiContinuityError::Invalid);
        }
        let mut body = Vec::with_capacity(size as usize);
        file.take(MAXIMUM_CONTINUITY_FILE_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|_| RoomAiContinuityError::Io)?;
        let persisted: PersistedContinuityFile =
            serde_json::from_slice(&body).map_err(|_| RoomAiContinuityError::Invalid)?;
        if persisted.file_version != CONTINUITY_FILE_VERSION {
            return Err(RoomAiContinuityError::Invalid);
        }
        let mut bindings = BTreeMap::new();
        for binding in persisted.bindings {
            if !valid_identifier(&binding.room_id)
                || !valid_identifier(&binding.participant_id)
                || !valid_session_id(&binding.session_id)
                || !valid_identifier(&binding.last_synced_message_id)
                || !valid_environment_key(&binding.environment_key)
            {
                return Err(RoomAiContinuityError::Invalid);
            }
            let key = (binding.room_id, binding.participant_id);
            let value = RoomAiContinuation {
                session_id: binding.session_id,
                last_synced_message_id: binding.last_synced_message_id,
                environment_key: binding.environment_key,
            };
            if bindings.insert(key, value).is_some() {
                return Err(RoomAiContinuityError::Invalid);
            }
        }
        Ok(bindings)
    }

    fn persist(
        &self,
        bindings: &BTreeMap<BindingKey, RoomAiContinuation>,
    ) -> Result<(), RoomAiContinuityError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(RoomAiContinuityError::Invalid)?;
        fs::create_dir_all(parent).map_err(|_| RoomAiContinuityError::Io)?;
        if !parent.is_dir() || (self.path.exists() && !self.path.is_file()) {
            return Err(RoomAiContinuityError::Invalid);
        }
        let body = serde_json::to_vec(&PersistedContinuityFile {
            file_version: CONTINUITY_FILE_VERSION,
            bindings: bindings
                .iter()
                .map(
                    |((room_id, participant_id), continuation)| PersistedBinding {
                        room_id: room_id.clone(),
                        participant_id: participant_id.clone(),
                        session_id: continuation.session_id.clone(),
                        last_synced_message_id: continuation.last_synced_message_id.clone(),
                        environment_key: continuation.environment_key.clone(),
                    },
                )
                .collect(),
        })
        .map_err(|_| RoomAiContinuityError::Invalid)?;
        if body.len() > MAXIMUM_CONTINUITY_FILE_BYTES {
            return Err(RoomAiContinuityError::Invalid);
        }
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{}.{}.{sequence}.tmp",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(RoomAiContinuityError::Invalid)?,
            std::process::id()
        ));
        let mut guard = TempGuard::new(temp.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| RoomAiContinuityError::Io)?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|_| RoomAiContinuityError::Io)?;
        drop(file);
        let backup = self.backup_path();
        if backup.exists() {
            if !backup.is_file() {
                return Err(RoomAiContinuityError::Invalid);
            }
            fs::remove_file(&backup).map_err(|_| RoomAiContinuityError::Io)?;
        }
        if self.path.exists() {
            fs::rename(&self.path, &backup).map_err(|_| RoomAiContinuityError::Io)?;
        }
        if fs::rename(&temp, &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(RoomAiContinuityError::Io);
        }
        guard.keep();
        Ok(())
    }
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

pub(crate) struct DesktopRoomAiContinuity {
    bindings: Mutex<BTreeMap<BindingKey, RoomAiContinuation>>,
    persistence: Option<ContinuityPersistence>,
}

impl DesktopRoomAiContinuity {
    #[cfg(test)]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            bindings: Mutex::new(BTreeMap::new()),
            persistence: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn persistent_for_tests(path: PathBuf) -> Result<Arc<Self>, RoomAiContinuityError> {
        Self::persistent(path)
    }

    fn persistent(path: PathBuf) -> Result<Arc<Self>, RoomAiContinuityError> {
        let persistence = ContinuityPersistence { path };
        let bindings = persistence.load()?;
        Ok(Arc::new(Self {
            bindings: Mutex::new(bindings),
            persistence: Some(persistence),
        }))
    }

    pub(crate) fn get(
        &self,
        room_id: &str,
        participant_id: &str,
    ) -> Result<Option<RoomAiContinuation>, RoomAiContinuityError> {
        if !valid_identifier(room_id) || !valid_identifier(participant_id) {
            return Err(RoomAiContinuityError::Invalid);
        }
        self.bindings
            .lock()
            .map_err(|_| RoomAiContinuityError::Io)
            .map(|bindings| {
                bindings
                    .get(&(room_id.to_owned(), participant_id.to_owned()))
                    .cloned()
            })
    }

    pub(crate) fn commit(
        &self,
        room_id: &str,
        participant_id: &str,
        continuation: RoomAiContinuation,
    ) -> Result<(), RoomAiContinuityError> {
        if !valid_identifier(room_id)
            || !valid_identifier(participant_id)
            || !valid_session_id(&continuation.session_id)
            || !valid_identifier(&continuation.last_synced_message_id)
            || !valid_environment_key(&continuation.environment_key)
        {
            return Err(RoomAiContinuityError::Invalid);
        }
        let mut bindings = self
            .bindings
            .lock()
            .map_err(|_| RoomAiContinuityError::Io)?;
        let mut next = bindings.clone();
        next.insert(
            (room_id.to_owned(), participant_id.to_owned()),
            continuation,
        );
        if let Some(persistence) = &self.persistence {
            persistence.persist(&next)?;
        }
        *bindings = next;
        Ok(())
    }

    pub(crate) fn clear(
        &self,
        room_id: &str,
        participant_id: &str,
    ) -> Result<bool, RoomAiContinuityError> {
        if !valid_identifier(room_id) || !valid_identifier(participant_id) {
            return Err(RoomAiContinuityError::Invalid);
        }
        let mut bindings = self
            .bindings
            .lock()
            .map_err(|_| RoomAiContinuityError::Io)?;
        let key = (room_id.to_owned(), participant_id.to_owned());
        if !bindings.contains_key(&key) {
            return Ok(false);
        }
        let mut next = bindings.clone();
        next.remove(&key);
        if let Some(persistence) = &self.persistence {
            persistence.persist(&next)?;
        }
        *bindings = next;
        Ok(true)
    }
}

pub(crate) fn product_ai_continuity_file(app_data_dir: &Path) -> PathBuf {
    env::var_os("MOE_ROOM_AI_CONTINUITY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(CONTINUITY_FILE_NAME))
}

pub(crate) fn persistent_room_ai_continuity(
    path: PathBuf,
) -> Result<Arc<DesktopRoomAiContinuity>, RoomAiContinuityError> {
    DesktopRoomAiContinuity::persistent(path)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_session_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_environment_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "moe-room-ai-continuity-{label}-{}-{}.json",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn persists_room_and_participant_scoped_continuation() {
        let path = temp_file("persist");
        let store = DesktopRoomAiContinuity::persistent(path.clone()).unwrap();
        store
            .commit(
                "room-1",
                "codex",
                RoomAiContinuation {
                    session_id: "thread-1".to_owned(),
                    last_synced_message_id: "message-1".to_owned(),
                    environment_key: "workspace-1234".to_owned(),
                },
            )
            .unwrap();

        let reloaded = DesktopRoomAiContinuity::persistent(path.clone()).unwrap();
        assert_eq!(
            reloaded.get("room-1", "codex").unwrap(),
            Some(RoomAiContinuation {
                session_id: "thread-1".to_owned(),
                last_synced_message_id: "message-1".to_owned(),
                environment_key: "workspace-1234".to_owned(),
            })
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("json.backup"));
    }

    #[test]
    fn rejects_unsafe_session_state() {
        let store = DesktopRoomAiContinuity::in_memory();
        assert_eq!(
            store.commit(
                "room-1",
                "codex",
                RoomAiContinuation {
                    session_id: "bad session".to_owned(),
                    last_synced_message_id: "message-1".to_owned(),
                    environment_key: "chat".to_owned(),
                },
            ),
            Err(RoomAiContinuityError::Invalid)
        );
    }

    #[test]
    fn clears_only_the_selected_room_participant_binding() {
        let store = DesktopRoomAiContinuity::in_memory();
        for (room_id, participant_id) in [("room-1", "codex"), ("room-1", "grok")] {
            store
                .commit(
                    room_id,
                    participant_id,
                    RoomAiContinuation {
                        session_id: format!("session-{participant_id}"),
                        last_synced_message_id: "message-1".to_owned(),
                        environment_key: "chat".to_owned(),
                    },
                )
                .unwrap();
        }

        assert_eq!(store.clear("room-1", "codex"), Ok(true));
        assert_eq!(store.clear("room-1", "codex"), Ok(false));
        assert_eq!(store.get("room-1", "codex").unwrap(), None);
        assert!(store.get("room-1", "grok").unwrap().is_some());
    }
}
