use crate::room_source::{DesktopRoomPersistenceError, DesktopRoomSource};
use moe_core::RoomCatalogSource;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

const BACKUP_DIRECTORY_NAME: &str = "M.O.E Backups";
const BACKUP_PREFIX: &str = "moe-room-backup-";
const BACKUP_SUFFIX: &str = ".json";
const BACKUP_SETTINGS_FILE_NAME: &str = "room-backup-settings-v1.json";
const BACKUP_SETTINGS_FILE_VERSION: u8 = 1;
const MAXIMUM_BACKUP_SETTINGS_BYTES: usize = 64 * 1024;
const ROOM_BACKUP_DIRECTORY_CHOICE_EVENT: &str = "moe-room-backup-directory-choice";
static BACKUP_SETTINGS_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupSuccess {
    ok: bool,
    file_name: String,
    room_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupDirectoryStatus {
    ok: bool,
    directory_path: String,
    is_custom: bool,
    available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupPreview {
    ok: bool,
    file_name: String,
    room_count: usize,
    created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomBackupDirectoryChoiceEvent {
    changed: bool,
    status: Option<RoomBackupDirectoryStatus>,
    error_code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupCommandError {
    code: &'static str,
    message: &'static str,
}

impl fmt::Display for RoomBackupCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for RoomBackupCommandError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RoomBackupSettingsFile {
    file_version: u8,
    custom_directory: Option<String>,
}

pub(crate) struct DesktopRoomBackupSettings {
    path: PathBuf,
    custom_directory: Mutex<Option<PathBuf>>,
}

fn map_error(_: DesktopRoomPersistenceError) -> RoomBackupCommandError {
    RoomBackupCommandError {
        code: "roomBackupUnavailable",
        message: "The Room backup could not be accessed.",
    }
}

fn invalid_settings() -> RoomBackupCommandError {
    RoomBackupCommandError {
        code: "roomBackupSettingsInvalid",
        message: "The Room backup settings are invalid.",
    }
}

fn unavailable_settings() -> RoomBackupCommandError {
    RoomBackupCommandError {
        code: "roomBackupSettingsUnavailable",
        message: "The Room backup settings could not be accessed.",
    }
}

fn valid_directory_path(path: &Path) -> bool {
    path.is_absolute()
        && path.parent().is_some()
        && path.file_name().is_some()
        && path.to_str().is_some_and(|value| {
            !value.is_empty() && value.len() <= 32_768 && !value.chars().any(char::is_control)
        })
}

fn read_backup_settings(path: &Path) -> Result<Option<PathBuf>, RoomBackupCommandError> {
    let file = File::open(path).map_err(|_| unavailable_settings())?;
    let metadata = file.metadata().map_err(|_| unavailable_settings())?;
    if metadata.len() > MAXIMUM_BACKUP_SETTINGS_BYTES as u64 {
        return Err(invalid_settings());
    }
    let mut body = Vec::with_capacity(metadata.len() as usize);
    file.take((MAXIMUM_BACKUP_SETTINGS_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| unavailable_settings())?;
    if body.len() > MAXIMUM_BACKUP_SETTINGS_BYTES {
        return Err(invalid_settings());
    }
    let value: RoomBackupSettingsFile =
        serde_json::from_slice(&body).map_err(|_| invalid_settings())?;
    if value.file_version != BACKUP_SETTINGS_FILE_VERSION {
        return Err(invalid_settings());
    }
    value
        .custom_directory
        .map(PathBuf::from)
        .map(|path| {
            if valid_directory_path(&path) {
                Ok(path)
            } else {
                Err(invalid_settings())
            }
        })
        .transpose()
}

impl DesktopRoomBackupSettings {
    pub(crate) fn load(path: PathBuf) -> Result<Arc<Self>, RoomBackupCommandError> {
        let backup_path = path.with_extension("backup");
        let custom_directory = if path.is_file() {
            match read_backup_settings(&path) {
                Ok(directory) => directory,
                Err(primary_error) if backup_path.is_file() => {
                    read_backup_settings(&backup_path).map_err(|_| primary_error)?
                }
                Err(error) => return Err(error),
            }
        } else if path.exists() {
            return Err(invalid_settings());
        } else if backup_path.is_file() {
            read_backup_settings(&backup_path)?
        } else if backup_path.exists() {
            return Err(invalid_settings());
        } else {
            None
        };
        Ok(Arc::new(Self {
            path,
            custom_directory: Mutex::new(custom_directory),
        }))
    }

    fn persist(&self, custom_directory: &Option<PathBuf>) -> Result<(), RoomBackupCommandError> {
        let parent = self.path.parent().ok_or_else(invalid_settings)?;
        fs::create_dir_all(parent).map_err(|_| unavailable_settings())?;
        let body = serde_json::to_vec(&RoomBackupSettingsFile {
            file_version: BACKUP_SETTINGS_FILE_VERSION,
            custom_directory: custom_directory
                .as_ref()
                .and_then(|path| path.to_str())
                .map(str::to_owned),
        })
        .map_err(|_| invalid_settings())?;
        if body.len() > MAXIMUM_BACKUP_SETTINGS_BYTES {
            return Err(invalid_settings());
        }

        let sequence = BACKUP_SETTINGS_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = self
            .path
            .with_extension(format!("tmp-{}-{sequence}", std::process::id()));
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| unavailable_settings())?;
        if temp.write_all(&body).and_then(|_| temp.sync_all()).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable_settings());
        }
        drop(temp);

        let backup_path = self.path.with_extension("backup");
        if self.path.exists() {
            if backup_path.exists() {
                fs::remove_file(&backup_path).map_err(|_| unavailable_settings())?;
            }
            fs::rename(&self.path, &backup_path).map_err(|_| unavailable_settings())?;
        }
        if fs::rename(&temp_path, &self.path).is_err() {
            let _ = fs::remove_file(&temp_path);
            if !self.path.exists() && backup_path.is_file() {
                let _ = fs::rename(&backup_path, &self.path);
            }
            return Err(unavailable_settings());
        }
        Ok(())
    }

    fn custom_directory(&self) -> Result<Option<PathBuf>, RoomBackupCommandError> {
        self.custom_directory
            .lock()
            .map(|value| value.clone())
            .map_err(|_| unavailable_settings())
    }

    fn set_custom_directory(
        &self,
        directory: Option<PathBuf>,
    ) -> Result<(), RoomBackupCommandError> {
        if directory
            .as_ref()
            .is_some_and(|path| !valid_directory_path(path) || !path.is_dir())
        {
            return Err(invalid_settings());
        }
        let mut current = self
            .custom_directory
            .lock()
            .map_err(|_| unavailable_settings())?;
        self.persist(&directory)?;
        *current = directory;
        Ok(())
    }
}

fn default_backup_directory(app: &AppHandle) -> Result<PathBuf, RoomBackupCommandError> {
    app.path()
        .document_dir()
        .map(|path| path.join(BACKUP_DIRECTORY_NAME))
        .map_err(|_| map_error(DesktopRoomPersistenceError::Io))
}

fn configured_backup_directory(
    app: &AppHandle,
    settings: &DesktopRoomBackupSettings,
) -> Result<(PathBuf, bool), RoomBackupCommandError> {
    if let Some(path) = std::env::var_os("MOE_ROOM_BACKUP_DIR") {
        let path = PathBuf::from(path);
        if valid_directory_path(&path) {
            return Ok((path, true));
        }
        return Err(invalid_settings());
    }
    if let Some(path) = settings.custom_directory()? {
        return Ok((path, true));
    }
    default_backup_directory(app).map(|path| (path, false))
}

fn directory_status(
    directory: &Path,
    is_custom: bool,
) -> Result<RoomBackupDirectoryStatus, RoomBackupCommandError> {
    if !valid_directory_path(directory) {
        return Err(invalid_settings());
    }
    let directory_path = directory.to_str().ok_or_else(invalid_settings)?.to_owned();
    Ok(RoomBackupDirectoryStatus {
        ok: true,
        directory_path,
        is_custom,
        available: directory.is_dir(),
    })
}

fn current_backup_stamp() -> Result<u128, RoomBackupCommandError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .map_err(|_| map_error(DesktopRoomPersistenceError::Io))
}

fn backup_file(directory: &Path, stamp: u128) -> PathBuf {
    directory.join(format!("{BACKUP_PREFIX}{stamp:020}{BACKUP_SUFFIX}"))
}

fn valid_backup_file_name(name: &str) -> bool {
    name.strip_prefix(BACKUP_PREFIX)
        .and_then(|value| value.strip_suffix(BACKUP_SUFFIX))
        .is_some_and(|stamp| stamp.len() == 20 && stamp.bytes().all(|byte| byte.is_ascii_digit()))
}

fn backup_stamp_from_path(path: &Path) -> Result<u64, RoomBackupCommandError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(invalid_settings)?;
    let stamp = name
        .strip_prefix(BACKUP_PREFIX)
        .and_then(|value| value.strip_suffix(BACKUP_SUFFIX))
        .filter(|value| value.len() == 20 && value.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(invalid_settings)?;
    stamp.parse::<u64>().map_err(|_| invalid_settings())
}

fn export_rooms(
    source: &DesktopRoomSource,
    directory: &Path,
    stamp: u128,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    if !valid_directory_path(directory) {
        return Err(invalid_settings());
    }
    let destination = backup_file(directory, stamp);
    source
        .export_snapshot(destination.clone())
        .map_err(map_error)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(invalid_settings)?
        .to_owned();
    let room_count = source
        .list_rooms()
        .map_err(|_| RoomBackupCommandError {
            code: "roomBackupUnavailable",
            message: "The Room catalog could not be read.",
        })?
        .len();
    Ok(RoomBackupSuccess {
        ok: true,
        file_name,
        room_count,
    })
}

fn latest_backup(directory: &Path) -> Result<PathBuf, RoomBackupCommandError> {
    if !valid_directory_path(directory) || !directory.is_dir() {
        return Err(map_error(DesktopRoomPersistenceError::Io));
    }
    let entries =
        fs::read_dir(directory).map_err(|_| map_error(DesktopRoomPersistenceError::Io))?;
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            valid_backup_file_name(name).then(|| (name.to_owned(), entry.path()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, path)| path)
        .ok_or(RoomBackupCommandError {
            code: "roomBackupMissing",
            message: "No M.I.O. Room backup was found.",
        })
}

fn preview_latest_rooms(
    source: &DesktopRoomSource,
    directory: &Path,
) -> Result<RoomBackupPreview, RoomBackupCommandError> {
    let path = latest_backup(directory)?;
    let room_count = source.inspect_snapshot(path.clone()).map_err(map_error)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(invalid_settings)?
        .to_owned();
    Ok(RoomBackupPreview {
        ok: true,
        file_name,
        room_count,
        created_at_unix_ms: backup_stamp_from_path(&path)?,
    })
}

fn restore_selected_rooms(
    source: &DesktopRoomSource,
    directory: &Path,
    expected_file_name: &str,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    if !valid_backup_file_name(expected_file_name) {
        return Err(invalid_settings());
    }
    let path = latest_backup(directory)?;
    if path.file_name().and_then(|name| name.to_str()) != Some(expected_file_name) {
        return Err(RoomBackupCommandError {
            code: "roomBackupChanged",
            message: "The selected Room backup is no longer the latest backup.",
        });
    }
    source.restore_snapshot(path).map_err(map_error)?;
    let room_count = source
        .list_rooms()
        .map_err(|_| RoomBackupCommandError {
            code: "roomBackupUnavailable",
            message: "The restored Room catalog could not be read.",
        })?
        .len();
    Ok(RoomBackupSuccess {
        ok: true,
        file_name: expected_file_name.to_owned(),
        room_count,
    })
}

#[cfg(windows)]
fn open_directory(directory: &Path) -> Result<(), RoomBackupCommandError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    if !valid_directory_path(directory) || !directory.is_dir() {
        return Err(map_error(DesktopRoomPersistenceError::Io));
    }
    let operation = "open\0".encode_utf16().collect::<Vec<_>>();
    let target = directory
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(map_error(DesktopRoomPersistenceError::Io));
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_directory(_: &Path) -> Result<(), RoomBackupCommandError> {
    Err(RoomBackupCommandError {
        code: "roomBackupOpenUnsupported",
        message: "Opening the Room backup directory is not supported on this platform.",
    })
}

pub(crate) fn product_room_backup_settings_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(BACKUP_SETTINGS_FILE_NAME)
}

#[tauri::command]
pub(crate) fn desktop_room_backup_status(
    app: AppHandle,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupDirectoryStatus, RoomBackupCommandError> {
    let (directory, is_custom) = configured_backup_directory(&app, settings.inner())?;
    directory_status(&directory, is_custom)
}

#[tauri::command]
pub(crate) fn desktop_room_backup_choose_directory(
    app: AppHandle,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupDirectoryStatus, RoomBackupCommandError> {
    let (current_directory, current_is_custom) =
        configured_backup_directory(&app, settings.inner())?;
    let current_status = directory_status(&current_directory, current_is_custom)?;
    let default_directory = default_backup_directory(&app)?;
    let event_app = app.clone();
    let event_settings = settings.inner().clone();
    let mut dialog = app
        .dialog()
        .file()
        .set_title("M.I.O.のバックアップ保存先を選択");
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    dialog.pick_folder(move |selected| {
        let (changed, status, error_code) = match selected {
            Some(selected) => match selected.into_path() {
                Ok(directory) => match event_settings.set_custom_directory(Some(directory)) {
                    Ok(()) => {
                        let selected = event_settings
                            .custom_directory()
                            .ok()
                            .flatten()
                            .and_then(|path| directory_status(&path, true).ok());
                        (selected.is_some(), selected, None)
                    }
                    Err(error) => (false, None, Some(error.code)),
                },
                Err(_) => (false, None, Some("roomBackupSettingsInvalid")),
            },
            None => {
                let status = event_settings
                    .custom_directory()
                    .ok()
                    .flatten()
                    .map(|path| (path, true))
                    .unwrap_or((default_directory, false));
                (false, directory_status(&status.0, status.1).ok(), None)
            }
        };
        let _ = event_app.emit(
            ROOM_BACKUP_DIRECTORY_CHOICE_EVENT,
            RoomBackupDirectoryChoiceEvent {
                changed,
                status,
                error_code,
            },
        );
    });
    Ok(current_status)
}

#[tauri::command]
pub(crate) fn desktop_room_backup_use_default_directory(
    app: AppHandle,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupDirectoryStatus, RoomBackupCommandError> {
    settings.set_custom_directory(None)?;
    let directory = default_backup_directory(&app)?;
    directory_status(&directory, false)
}

#[tauri::command]
pub(crate) fn desktop_room_backup_open_directory(
    app: AppHandle,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupDirectoryStatus, RoomBackupCommandError> {
    let (directory, is_custom) = configured_backup_directory(&app, settings.inner())?;
    open_directory(&directory)?;
    directory_status(&directory, is_custom)
}

#[tauri::command]
pub(crate) fn desktop_room_backup(
    app: AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    let (directory, _) = configured_backup_directory(&app, settings.inner())?;
    if settings.custom_directory()?.is_some() && !directory.is_dir() {
        return Err(map_error(DesktopRoomPersistenceError::Io));
    }
    export_rooms(source.as_ref(), &directory, current_backup_stamp()?)
}

#[tauri::command]
pub(crate) fn desktop_room_backup_preview_latest(
    app: AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
) -> Result<RoomBackupPreview, RoomBackupCommandError> {
    let (directory, _) = configured_backup_directory(&app, settings.inner())?;
    preview_latest_rooms(source.as_ref(), &directory)
}

#[tauri::command]
pub(crate) fn desktop_room_restore_backup(
    app: AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
    settings: State<'_, Arc<DesktopRoomBackupSettings>>,
    file_name: String,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    let (directory, _) = configured_backup_directory(&app, settings.inner())?;
    restore_selected_rooms(source.as_ref(), &directory, &file_name)
}

#[cfg(test)]
mod tests {
    use super::{
        DesktopRoomBackupSettings, export_rooms, preview_latest_rooms, restore_selected_rooms,
    };
    use crate::room_source::desktop_room_source;
    use moe_core::{RoomCatalogSource, RoomStore};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn isolated_directory(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "moe-room-backup-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn exports_previews_and_restores_only_the_confirmed_latest_backup() {
        let directory = isolated_directory("restore");
        let source = desktop_room_source();
        export_rooms(source.as_ref(), &directory, 1).unwrap();
        source
            .rename_room("moe-dev-room", "Changed", "2026-08-12T16:00:00Z")
            .unwrap();
        export_rooms(source.as_ref(), &directory, 2).unwrap();
        source
            .rename_room("moe-dev-room", "Changed again", "2026-08-12T16:00:01Z")
            .unwrap();
        fs::write(directory.join("untrusted.json"), b"not a backup").unwrap();

        let preview = preview_latest_rooms(source.as_ref(), &directory).unwrap();
        assert_eq!(
            preview.file_name,
            "moe-room-backup-00000000000000000002.json"
        );
        assert_eq!(preview.room_count, 3);
        assert_eq!(preview.created_at_unix_ms, 2);
        let restored =
            restore_selected_rooms(source.as_ref(), &directory, &preview.file_name).unwrap();
        assert_eq!(restored.room_count, 3);
        assert_eq!(source.list_rooms().unwrap()[0].name, "Changed");

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn refuses_restore_when_latest_backup_changed_after_preview() {
        let directory = isolated_directory("race");
        let source = desktop_room_source();
        export_rooms(source.as_ref(), &directory, 1).unwrap();
        let preview = preview_latest_rooms(source.as_ref(), &directory).unwrap();
        export_rooms(source.as_ref(), &directory, 2).unwrap();

        let error =
            restore_selected_rooms(source.as_ref(), &directory, &preview.file_name).unwrap_err();
        assert_eq!(error.code, "roomBackupChanged");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_corrupt_latest_backup_during_preview_without_restoring() {
        let directory = isolated_directory("corrupt");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("moe-room-backup-00000000000000000001.json"),
            b"not valid Room JSON",
        )
        .unwrap();
        let source = desktop_room_source();
        let before = source.list_rooms().unwrap()[0].name.clone();

        assert!(preview_latest_rooms(source.as_ref(), &directory).is_err());
        assert_eq!(source.list_rooms().unwrap()[0].name, before);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn persists_custom_backup_directory_without_moving_existing_files() {
        let root = isolated_directory("settings");
        let custom = root.join("chosen");
        fs::create_dir_all(&custom).unwrap();
        let settings_file = root.join("room-backup-settings-v1.json");
        let settings = DesktopRoomBackupSettings::load(settings_file.clone()).unwrap();
        settings.set_custom_directory(Some(custom.clone())).unwrap();
        fs::write(custom.join("keep.txt"), b"keep").unwrap();

        let reloaded = DesktopRoomBackupSettings::load(settings_file).unwrap();
        assert_eq!(reloaded.custom_directory().unwrap(), Some(custom.clone()));
        reloaded.set_custom_directory(None).unwrap();
        assert_eq!(reloaded.custom_directory().unwrap(), None);
        assert_eq!(fs::read(custom.join("keep.txt")).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovers_settings_from_the_previous_file_without_defaulting_silently() {
        let root = isolated_directory("settings-recovery");
        let custom = root.join("chosen");
        fs::create_dir_all(&custom).unwrap();
        let settings_file = root.join("room-backup-settings-v1.json");
        let settings = DesktopRoomBackupSettings::load(settings_file.clone()).unwrap();
        settings.set_custom_directory(Some(custom.clone())).unwrap();
        settings.set_custom_directory(None).unwrap();
        fs::remove_file(&settings_file).unwrap();

        let recovered = DesktopRoomBackupSettings::load(settings_file).unwrap();
        assert_eq!(recovered.custom_directory().unwrap(), Some(custom));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_corrupt_settings_instead_of_silently_using_the_default() {
        let root = isolated_directory("settings-corrupt");
        fs::create_dir_all(&root).unwrap();
        let settings_file = root.join("room-backup-settings-v1.json");
        fs::write(&settings_file, b"not valid settings").unwrap();

        let error = match DesktopRoomBackupSettings::load(settings_file) {
            Ok(_) => panic!("corrupt settings must not fall back to the default directory"),
            Err(error) => error,
        };
        assert_eq!(error.code, "roomBackupSettingsInvalid");
        fs::remove_dir_all(root).unwrap();
    }
}
