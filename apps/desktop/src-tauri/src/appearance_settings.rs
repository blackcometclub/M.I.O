use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

const FILE_VERSION: u8 = 1;
const FILE_NAME: &str = "appearance-settings-v1.json";
const MAXIMUM_FILE_BYTES: usize = 26 * 1024 * 1024;
const MAXIMUM_IMAGE_DATA_URL_BYTES: usize = 12 * 1024 * 1024;
const MAXIMUM_FILE_NAME_BYTES: usize = 512;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppearanceImage {
    data_url: String,
    file_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppearancePlacement {
    scale: f64,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppearanceArtwork {
    data_url: String,
    file_name: String,
    placement: AppearancePlacement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppearanceSettings {
    background_color: String,
    background_image: Option<AppearanceImage>,
    artwork: Option<AppearanceArtwork>,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            background_color: "#ffc126".to_owned(),
            background_image: None,
            artwork: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppearanceFile {
    file_version: u8,
    settings: AppearanceSettings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppearanceSettingsError {
    code: &'static str,
    message: &'static str,
}

impl fmt::Display for AppearanceSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for AppearanceSettingsError {}

pub(crate) struct DesktopAppearanceSettings {
    path: PathBuf,
    settings: Mutex<AppearanceSettings>,
}

fn invalid_settings() -> AppearanceSettingsError {
    AppearanceSettingsError {
        code: "invalidAppearanceSettings",
        message: "The appearance settings are invalid.",
    }
}

fn unavailable() -> AppearanceSettingsError {
    AppearanceSettingsError {
        code: "appearanceSettingsUnavailable",
        message: "The appearance settings are temporarily unavailable.",
    }
}

fn valid_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name.len() <= MAXIMUM_FILE_NAME_BYTES
        && !file_name.chars().any(char::is_control)
}

fn valid_data_url(data_url: &str) -> bool {
    let valid_prefix = [
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/webp;base64,",
    ]
    .iter()
    .any(|prefix| data_url.starts_with(prefix));
    let encoded = data_url.split_once(',').map(|(_, body)| body);
    valid_prefix
        && data_url.len() <= MAXIMUM_IMAGE_DATA_URL_BYTES
        && encoded.is_some_and(|body| {
            !body.is_empty()
                && body
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
        })
}

fn valid_settings(settings: &AppearanceSettings) -> bool {
    let valid_color = settings.background_color.len() == 7
        && settings.background_color.starts_with('#')
        && settings.background_color[1..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    let valid_background = settings
        .background_image
        .as_ref()
        .is_none_or(|image| valid_file_name(&image.file_name) && valid_data_url(&image.data_url));
    let valid_artwork = settings.artwork.as_ref().is_none_or(|artwork| {
        valid_file_name(&artwork.file_name)
            && valid_data_url(&artwork.data_url)
            && artwork.placement.scale.is_finite()
            && (0.05..=2.0).contains(&artwork.placement.scale)
            && artwork.placement.x.is_finite()
            && (-0.5..=1.5).contains(&artwork.placement.x)
            && artwork.placement.y.is_finite()
            && (-0.5..=1.5).contains(&artwork.placement.y)
    });
    valid_color && valid_background && valid_artwork
}

fn read_file(path: &Path) -> Result<AppearanceSettings, AppearanceSettingsError> {
    let mut file = File::open(path).map_err(|_| unavailable())?;
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if metadata.len() > MAXIMUM_FILE_BYTES as u64 {
        return Err(invalid_settings());
    }
    let mut body = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut body).map_err(|_| unavailable())?;
    let value: AppearanceFile = serde_json::from_slice(&body).map_err(|_| invalid_settings())?;
    if value.file_version != FILE_VERSION || !valid_settings(&value.settings) {
        return Err(invalid_settings());
    }
    Ok(value.settings)
}

impl DesktopAppearanceSettings {
    pub(crate) fn load(path: PathBuf) -> Result<Arc<Self>, AppearanceSettingsError> {
        let settings = if path.is_file() {
            read_file(&path)
                .or_else(|_| read_file(&path.with_extension("backup")))
                .unwrap_or_default()
        } else if path.exists() {
            return Err(invalid_settings());
        } else {
            AppearanceSettings::default()
        };
        Ok(Arc::new(Self {
            path,
            settings: Mutex::new(settings),
        }))
    }

    fn persist(&self, settings: &AppearanceSettings) -> Result<(), AppearanceSettingsError> {
        let parent = self.path.parent().ok_or_else(invalid_settings)?;
        fs::create_dir_all(parent).map_err(|_| unavailable())?;
        let body = serde_json::to_vec(&AppearanceFile {
            file_version: FILE_VERSION,
            settings: settings.clone(),
        })
        .map_err(|_| invalid_settings())?;
        if body.len() > MAXIMUM_FILE_BYTES {
            return Err(invalid_settings());
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

    fn get(&self) -> Result<AppearanceSettings, AppearanceSettingsError> {
        self.settings
            .lock()
            .map(|settings| settings.clone())
            .map_err(|_| unavailable())
    }

    fn save(
        &self,
        settings: AppearanceSettings,
    ) -> Result<AppearanceSettings, AppearanceSettingsError> {
        if !valid_settings(&settings) {
            return Err(invalid_settings());
        }
        let mut current = self.settings.lock().map_err(|_| unavailable())?;
        self.persist(&settings)?;
        *current = settings.clone();
        Ok(settings)
    }
}

pub(crate) fn product_appearance_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

#[tauri::command]
pub(crate) fn desktop_appearance_settings(
    settings: State<'_, Arc<DesktopAppearanceSettings>>,
) -> Result<AppearanceSettings, AppearanceSettingsError> {
    settings.get()
}

#[tauri::command]
pub(crate) fn desktop_appearance_settings_save(
    settings: State<'_, Arc<DesktopAppearanceSettings>>,
    appearance: AppearanceSettings,
) -> Result<AppearanceSettings, AppearanceSettingsError> {
    settings.save(appearance)
}

#[cfg(test)]
mod tests {
    use super::{
        AppearanceArtwork, AppearanceImage, AppearancePlacement, AppearanceSettings,
        DesktopAppearanceSettings, valid_settings,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("moe-appearance-{unique}"))
    }

    fn example_settings() -> AppearanceSettings {
        AppearanceSettings {
            background_color: "#80b9cf".to_owned(),
            background_image: Some(AppearanceImage {
                data_url: "data:image/png;base64,QUJD".to_owned(),
                file_name: "background.png".to_owned(),
            }),
            artwork: Some(AppearanceArtwork {
                data_url: "data:image/webp;base64,QUJD".to_owned(),
                file_name: "artwork.webp".to_owned(),
                placement: AppearancePlacement {
                    scale: 1.25,
                    x: 0.4,
                    y: 0.6,
                },
            }),
        }
    }

    #[test]
    fn validates_bounded_appearance() {
        assert!(valid_settings(&example_settings()));
        let mut invalid = example_settings();
        invalid.background_color = "yellow".to_owned();
        assert!(!valid_settings(&invalid));
    }

    #[test]
    fn persists_reloads_and_recovers_from_backup() {
        let directory = test_directory();
        fs::create_dir_all(&directory).expect("test directory should be created");
        let path = directory.join("appearance.json");
        let first = example_settings();
        let mut second = first.clone();
        second.background_color = "#f08da8".to_owned();

        let store = DesktopAppearanceSettings::load(path.clone()).expect("store should load");
        store
            .save(first.clone())
            .expect("first appearance should save");
        store.save(second).expect("second appearance should save");
        drop(store);
        let reloaded = DesktopAppearanceSettings::load(path.clone()).expect("store should reload");
        assert_eq!(
            reloaded
                .get()
                .expect("appearance should read")
                .background_color,
            "#f08da8"
        );
        drop(reloaded);

        fs::write(&path, b"not json").expect("primary should be corrupted");
        let recovered = DesktopAppearanceSettings::load(path).expect("backup should recover");
        assert_eq!(recovered.get().expect("appearance should read"), first);
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }
}
