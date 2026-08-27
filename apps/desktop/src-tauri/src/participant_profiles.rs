use crate::room_source::{DesktopRoomSource, ParticipantIdMigration};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

const FILE_VERSION: u8 = 1;
const FILE_NAME: &str = "participant-profiles-v1.json";
const MAXIMUM_FILE_BYTES: usize = 16 * 1024 * 1024;
const MAXIMUM_AVATAR_DATA_URL_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_DISPLAY_NAME_BYTES: usize = 200;
const MAXIMUM_AI_INSTRUCTIONS_CHARS: usize = 2_000;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AiAccessMode {
    #[default]
    ProviderDefault,
    ChatOnly,
    WorkspaceRead,
    WorkspaceWrite,
}

impl AiAccessMode {
    fn effective_for(self, participant_id: &str) -> Self {
        match (participant_id, self) {
            ("codex", Self::ProviderDefault) => Self::ChatOnly,
            (_, Self::ProviderDefault) => Self::ChatOnly,
            (_, mode) => mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AvatarProfile {
    data_url: String,
    scale: f64,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ParticipantProfile {
    participant_id: String,
    display_name: String,
    avatar: Option<AvatarProfile>,
    #[serde(default)]
    ai_instructions: String,
    #[serde(default)]
    ai_access_mode: AiAccessMode,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileFile {
    file_version: u8,
    profiles: BTreeMap<String, ParticipantProfile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantProfileError {
    code: &'static str,
    message: &'static str,
}

impl fmt::Display for ParticipantProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for ParticipantProfileError {}

pub(crate) struct DesktopParticipantProfiles {
    path: PathBuf,
    profiles: Mutex<BTreeMap<String, ParticipantProfile>>,
}

fn invalid_profile() -> ParticipantProfileError {
    ParticipantProfileError {
        code: "invalidParticipantProfile",
        message: "The participant profile is invalid.",
    }
}

fn unavailable() -> ParticipantProfileError {
    ParticipantProfileError {
        code: "participantProfileUnavailable",
        message: "The participant profile is temporarily unavailable.",
    }
}

fn valid_display_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed.len() <= MAXIMUM_DISPLAY_NAME_BYTES
        && !trimmed.chars().any(char::is_control)
}

fn valid_avatar(avatar: &AvatarProfile) -> bool {
    let prefix_is_valid = [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/webp;base64,",
    ]
    .iter()
    .any(|prefix| avatar.data_url.starts_with(prefix));
    let encoded = avatar.data_url.split_once(',').map(|(_, body)| body);
    prefix_is_valid
        && avatar.data_url.len() <= MAXIMUM_AVATAR_DATA_URL_BYTES
        && encoded.is_some_and(|body| {
            !body.is_empty()
                && body
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
        })
        && avatar.scale.is_finite()
        && (1.0..=6.0).contains(&avatar.scale)
        && avatar.x.is_finite()
        && (-1.0..=1.0).contains(&avatar.x)
        && avatar.y.is_finite()
        && (-1.0..=1.0).contains(&avatar.y)
}

fn valid_profile(profile: &ParticipantProfile) -> bool {
    !profile.participant_id.is_empty()
        && profile.participant_id.len() <= 128
        && profile
            .participant_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && valid_display_name(&profile.display_name)
        && profile.avatar.as_ref().is_none_or(valid_avatar)
        && profile.ai_instructions.chars().count() <= MAXIMUM_AI_INSTRUCTIONS_CHARS
        && !profile
            .ai_instructions
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        && (profile.participant_id == "codex"
            || matches!(
                profile.ai_access_mode,
                AiAccessMode::ProviderDefault | AiAccessMode::ChatOnly
            ))
}

fn read_file(path: &Path) -> Result<BTreeMap<String, ParticipantProfile>, ParticipantProfileError> {
    let mut file = File::open(path).map_err(|_| unavailable())?;
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if metadata.len() > MAXIMUM_FILE_BYTES as u64 {
        return Err(invalid_profile());
    }
    let mut body = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut body).map_err(|_| unavailable())?;
    let value: ProfileFile = serde_json::from_slice(&body).map_err(|_| invalid_profile())?;
    if value.file_version != FILE_VERSION
        || value
            .profiles
            .iter()
            .any(|(id, profile)| id != &profile.participant_id || !valid_profile(profile))
    {
        return Err(invalid_profile());
    }
    Ok(value.profiles)
}

impl DesktopParticipantProfiles {
    #[cfg(test)]
    pub(crate) fn load(path: PathBuf) -> Result<Arc<Self>, ParticipantProfileError> {
        Self::load_with_participant_migration(path, None)
    }

    pub(crate) fn load_with_participant_migration(
        path: PathBuf,
        migration: Option<&ParticipantIdMigration>,
    ) -> Result<Arc<Self>, ParticipantProfileError> {
        let mut profiles = if path.is_file() {
            read_file(&path)
                .or_else(|_| read_file(&path.with_extension("backup")))
                .unwrap_or_default()
        } else if path.exists() {
            return Err(invalid_profile());
        } else {
            BTreeMap::new()
        };
        let migrated = migration
            .map(|migration| migrate_participant_id(&mut profiles, migration))
            .transpose()?
            .unwrap_or(false);
        let store = Arc::new(Self {
            path,
            profiles: Mutex::new(profiles),
        });
        if migrated {
            let profiles = store.profiles.lock().map_err(|_| unavailable())?.clone();
            store.persist(&profiles)?;
        }
        Ok(store)
    }

    fn persist(
        &self,
        profiles: &BTreeMap<String, ParticipantProfile>,
    ) -> Result<(), ParticipantProfileError> {
        let parent = self.path.parent().ok_or_else(invalid_profile)?;
        fs::create_dir_all(parent).map_err(|_| unavailable())?;
        let body = serde_json::to_vec(&ProfileFile {
            file_version: FILE_VERSION,
            profiles: profiles.clone(),
        })
        .map_err(|_| invalid_profile())?;
        if body.len() > MAXIMUM_FILE_BYTES {
            return Err(invalid_profile());
        }

        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = self
            .path
            .with_extension(format!("tmp-{}-{sequence}", std::process::id()));
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| unavailable())?;
        if temp.write_all(&body).and_then(|_| temp.sync_all()).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable());
        }
        drop(temp);

        let backup_path = self.path.with_extension("backup");
        if self.path.exists() {
            if backup_path.exists() {
                fs::remove_file(&backup_path).map_err(|_| unavailable())?;
            }
            fs::rename(&self.path, &backup_path).map_err(|_| unavailable())?;
        }
        if fs::rename(&temp_path, &self.path).is_err() {
            let _ = fs::remove_file(&temp_path);
            if !self.path.exists() && backup_path.is_file() {
                let _ = fs::rename(&backup_path, &self.path);
            }
            return Err(unavailable());
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<ParticipantProfile>, ParticipantProfileError> {
        self.profiles
            .lock()
            .map(|profiles| profiles.values().cloned().collect())
            .map_err(|_| unavailable())
    }

    pub(crate) fn display_name(&self, participant_id: &str) -> Option<String> {
        self.profiles.lock().ok().and_then(|profiles| {
            profiles
                .get(participant_id)
                .map(|profile| profile.display_name.clone())
        })
    }

    pub(crate) fn ai_instructions(&self, participant_id: &str) -> Option<String> {
        self.profiles.lock().ok().and_then(|profiles| {
            profiles
                .get(participant_id)
                .map(|profile| profile.ai_instructions.trim().to_owned())
                .filter(|instructions| !instructions.is_empty())
        })
    }

    pub(crate) fn ai_access_mode(&self, participant_id: &str) -> AiAccessMode {
        self.profiles
            .lock()
            .ok()
            .and_then(|profiles| {
                profiles
                    .get(participant_id)
                    .map(|profile| profile.ai_access_mode)
            })
            .unwrap_or_default()
            .effective_for(participant_id)
    }

    #[cfg(test)]
    pub(crate) fn for_tests(display_names: &[(&str, &str)]) -> Arc<Self> {
        let profiles = display_names
            .iter()
            .map(|(participant_id, display_name)| {
                (
                    (*participant_id).to_owned(),
                    ParticipantProfile {
                        participant_id: (*participant_id).to_owned(),
                        display_name: (*display_name).to_owned(),
                        avatar: None,
                        ai_instructions: String::new(),
                        ai_access_mode: AiAccessMode::default(),
                    },
                )
            })
            .collect();
        Arc::new(Self {
            path: PathBuf::new(),
            profiles: Mutex::new(profiles),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_tests_with_instructions(profiles: &[(&str, &str, &str)]) -> Arc<Self> {
        let profiles = profiles
            .iter()
            .map(|(participant_id, display_name, ai_instructions)| {
                (
                    (*participant_id).to_owned(),
                    ParticipantProfile {
                        participant_id: (*participant_id).to_owned(),
                        display_name: (*display_name).to_owned(),
                        avatar: None,
                        ai_instructions: (*ai_instructions).to_owned(),
                        ai_access_mode: AiAccessMode::default(),
                    },
                )
            })
            .collect();
        Arc::new(Self {
            path: PathBuf::new(),
            profiles: Mutex::new(profiles),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_tests_with_access(profiles: &[(&str, &str, AiAccessMode)]) -> Arc<Self> {
        let profiles = profiles
            .iter()
            .map(|(participant_id, display_name, ai_access_mode)| {
                (
                    (*participant_id).to_owned(),
                    ParticipantProfile {
                        participant_id: (*participant_id).to_owned(),
                        display_name: (*display_name).to_owned(),
                        avatar: None,
                        ai_instructions: String::new(),
                        ai_access_mode: *ai_access_mode,
                    },
                )
            })
            .collect();
        Arc::new(Self {
            path: PathBuf::new(),
            profiles: Mutex::new(profiles),
        })
    }

    fn save(
        &self,
        profile: ParticipantProfile,
    ) -> Result<ParticipantProfile, ParticipantProfileError> {
        if !valid_profile(&profile) {
            return Err(invalid_profile());
        }
        let mut profiles = self.profiles.lock().map_err(|_| unavailable())?;
        let mut next = profiles.clone();
        next.insert(profile.participant_id.clone(), profile.clone());
        self.persist(&next)?;
        *profiles = next;
        Ok(profile)
    }
}

fn migrate_participant_id(
    profiles: &mut BTreeMap<String, ParticipantProfile>,
    migration: &ParticipantIdMigration,
) -> Result<bool, ParticipantProfileError> {
    if migration.previous_id == migration.current_id {
        return Ok(false);
    }
    let Some(mut previous) = profiles.remove(&migration.previous_id) else {
        return Ok(false);
    };
    previous.participant_id.clone_from(&migration.current_id);
    if let Some(current) = profiles.get(&migration.current_id) {
        if current != &previous {
            previous.participant_id.clone_from(&migration.previous_id);
            profiles.insert(migration.previous_id.clone(), previous);
            return Err(invalid_profile());
        }
        return Ok(true);
    }
    profiles.insert(migration.current_id.clone(), previous);
    Ok(true)
}

pub(crate) fn product_profile_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

#[tauri::command]
pub(crate) fn desktop_participant_profiles(
    profiles: State<'_, Arc<DesktopParticipantProfiles>>,
) -> Result<Vec<ParticipantProfile>, ParticipantProfileError> {
    profiles.list()
}

#[tauri::command]
pub(crate) fn desktop_participant_profile_save(
    source: State<'_, Arc<DesktopRoomSource>>,
    profiles: State<'_, Arc<DesktopParticipantProfiles>>,
    participant_id: String,
    display_name: String,
    avatar: Option<AvatarProfile>,
    ai_instructions: String,
    ai_access_mode: AiAccessMode,
) -> Result<ParticipantProfile, ParticipantProfileError> {
    if !source.has_participant(&participant_id) {
        return Err(invalid_profile());
    }
    profiles.save(ParticipantProfile {
        participant_id,
        display_name: display_name.trim().to_owned(),
        avatar,
        ai_instructions: ai_instructions.trim().to_owned(),
        ai_access_mode,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AiAccessMode, AvatarProfile, DesktopParticipantProfiles, FILE_VERSION,
        MAXIMUM_AI_INSTRUCTIONS_CHARS, ParticipantProfile, ProfileFile, read_file, valid_profile,
    };
    use crate::room_source::ParticipantIdMigration;
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("moe-participant-profile-{label}-{unique}"))
    }

    fn example_profile() -> ParticipantProfile {
        ParticipantProfile {
            participant_id: "codex".to_owned(),
            display_name: "コデちゃん".to_owned(),
            avatar: Some(AvatarProfile {
                data_url: "data:image/png;base64,QUJD".to_owned(),
                scale: 1.5,
                x: 0.1,
                y: -0.2,
            }),
            ai_instructions: "明るく、短めに答える。".to_owned(),
            ai_access_mode: AiAccessMode::WorkspaceRead,
        }
    }

    #[test]
    fn validates_bounded_profile_and_avatar_data() {
        let profile = example_profile();
        assert!(valid_profile(&profile));

        let mut unsafe_profile = profile;
        unsafe_profile.display_name = "bad\nname".to_owned();
        assert!(!valid_profile(&unsafe_profile));

        let mut oversized_profile = example_profile();
        oversized_profile.ai_instructions = "あ".repeat(MAXIMUM_AI_INSTRUCTIONS_CHARS + 1);
        assert!(!valid_profile(&oversized_profile));

        let mut unsupported_profile = example_profile();
        unsupported_profile.participant_id = "gemini".to_owned();
        unsupported_profile.ai_access_mode = AiAccessMode::WorkspaceWrite;
        assert!(!valid_profile(&unsupported_profile));
    }

    #[test]
    fn persists_profiles_and_recovers_a_valid_backup() {
        let directory = test_directory("persistence");
        fs::create_dir_all(&directory).expect("test directory should be created");
        let path = directory.join("profiles.json");
        let profile = example_profile();

        let store =
            DesktopParticipantProfiles::load(path.clone()).expect("empty store should load");
        store.save(profile.clone()).expect("profile should persist");
        drop(store);
        let reloaded =
            DesktopParticipantProfiles::load(path.clone()).expect("saved store should load");
        assert_eq!(
            reloaded.list().expect("profiles should list"),
            vec![profile.clone()]
        );
        drop(reloaded);

        let backup = ProfileFile {
            file_version: FILE_VERSION,
            profiles: BTreeMap::from([(profile.participant_id.clone(), profile.clone())]),
        };
        fs::write(
            path.with_extension("backup"),
            serde_json::to_vec(&backup).unwrap(),
        )
        .expect("backup should be written");
        fs::write(&path, b"not json").expect("corrupt primary should be written");
        let recovered = DesktopParticipantProfiles::load(path).expect("backup should recover");
        assert_eq!(
            recovered.list().expect("profiles should list"),
            vec![profile]
        );

        fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn migrates_a_saved_owner_profile_without_losing_customization() {
        let directory = test_directory("owner-id-migration");
        fs::create_dir_all(&directory).expect("test directory should be created");
        let path = directory.join("profiles.json");
        let previous_id = "local-user";
        let mut profile = example_profile();
        profile.participant_id = previous_id.to_owned();
        profile.display_name = "Sample Owner".to_owned();
        profile.ai_access_mode = AiAccessMode::ChatOnly;
        let file = ProfileFile {
            file_version: FILE_VERSION,
            profiles: BTreeMap::from([(previous_id.to_owned(), profile)]),
        };
        fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();

        let store = DesktopParticipantProfiles::load_with_participant_migration(
            path.clone(),
            Some(&ParticipantIdMigration {
                previous_id: previous_id.to_owned(),
                current_id: "owner".to_owned(),
            }),
        )
        .unwrap();
        assert_eq!(store.display_name("owner").as_deref(), Some("Sample Owner"));
        assert_eq!(store.display_name(previous_id), None);
        drop(store);

        let persisted = read_file(&path).unwrap();
        assert!(persisted.contains_key("owner"));
        assert!(!persisted.contains_key(previous_id));
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn loads_existing_v1_profiles_without_ai_instructions() {
        let directory = test_directory("v1-compatibility");
        fs::create_dir_all(&directory).expect("test directory should be created");
        let path = directory.join("profiles.json");
        let legacy = serde_json::json!({
            "fileVersion": 1,
            "profiles": {
                "codex": {
                    "participantId": "codex",
                    "displayName": "Codex",
                    "avatar": null
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&legacy).unwrap())
            .expect("legacy profile should be written");

        let store = DesktopParticipantProfiles::load(path).expect("legacy profile should load");
        assert_eq!(store.display_name("codex").as_deref(), Some("Codex"));
        assert_eq!(store.ai_instructions("codex"), None);
        assert_eq!(store.ai_access_mode("codex"), AiAccessMode::ChatOnly);

        fs::remove_dir_all(directory).expect("test directory should be removed");
    }
}
