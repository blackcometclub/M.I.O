use moe_core::RoomCatalogSource;
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
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::room_source::DesktopRoomSource;

const WORKSPACE_FILE_VERSION: u8 = 1;
const WORKSPACE_FILE_NAME: &str = "room-workspaces-v1.json";
const MAXIMUM_WORKSPACE_FILE_BYTES: usize = 64 * 1024;
const ROOM_WORKSPACE_CHOICE_EVENT: &str = "moe-room-workspace-choice";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoomWorkspaceError {
    Io,
    Invalid,
    RoomNotFound,
    Unavailable,
    UnsafeLink,
}

impl fmt::Display for RoomWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io => "the Room workspace settings could not be accessed",
            Self::Invalid => "the Room workspace settings are invalid",
            Self::RoomNotFound => "the Room does not exist",
            Self::Unavailable => "the selected workspace is unavailable",
            Self::UnsafeLink => "the selected workspace is a filesystem link",
        })
    }
}

impl Error for RoomWorkspaceError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedWorkspaceBinding {
    room_id: String,
    root: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedWorkspaceFile {
    file_version: u8,
    bindings: Vec<PersistedWorkspaceBinding>,
}

struct WorkspacePersistence {
    path: PathBuf,
}

impl WorkspacePersistence {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.backup")
    }

    fn load(&self) -> Result<BTreeMap<String, PathBuf>, RoomWorkspaceError> {
        let backup = self.backup_path();
        let source = if self.path.is_file() {
            &self.path
        } else if !self.path.exists() && backup.is_file() {
            &backup
        } else if !self.path.exists() && !backup.exists() {
            return Ok(BTreeMap::new());
        } else {
            return Err(RoomWorkspaceError::Invalid);
        };
        let file = File::open(source).map_err(|_| RoomWorkspaceError::Io)?;
        let size = file.metadata().map_err(|_| RoomWorkspaceError::Io)?.len();
        if size == 0 || size > MAXIMUM_WORKSPACE_FILE_BYTES as u64 {
            return Err(RoomWorkspaceError::Invalid);
        }
        let mut body = Vec::with_capacity(size as usize);
        file.take(MAXIMUM_WORKSPACE_FILE_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|_| RoomWorkspaceError::Io)?;
        let persisted: PersistedWorkspaceFile =
            serde_json::from_slice(&body).map_err(|_| RoomWorkspaceError::Invalid)?;
        if persisted.file_version != WORKSPACE_FILE_VERSION {
            return Err(RoomWorkspaceError::Invalid);
        }
        let mut bindings = BTreeMap::new();
        for binding in persisted.bindings {
            if !valid_room_id(&binding.room_id)
                || !binding.root.is_absolute()
                || bindings.insert(binding.room_id, binding.root).is_some()
            {
                return Err(RoomWorkspaceError::Invalid);
            }
        }
        Ok(bindings)
    }

    fn persist(&self, bindings: &BTreeMap<String, PathBuf>) -> Result<(), RoomWorkspaceError> {
        let parent = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(RoomWorkspaceError::Invalid)?;
        fs::create_dir_all(parent).map_err(|_| RoomWorkspaceError::Io)?;
        if !parent.is_dir() || (self.path.exists() && !self.path.is_file()) {
            return Err(RoomWorkspaceError::Invalid);
        }
        let body = serde_json::to_vec(&PersistedWorkspaceFile {
            file_version: WORKSPACE_FILE_VERSION,
            bindings: bindings
                .iter()
                .map(|(room_id, root)| PersistedWorkspaceBinding {
                    room_id: room_id.clone(),
                    root: root.clone(),
                })
                .collect(),
        })
        .map_err(|_| RoomWorkspaceError::Invalid)?;
        if body.len() > MAXIMUM_WORKSPACE_FILE_BYTES {
            return Err(RoomWorkspaceError::Invalid);
        }
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{}.{}.{sequence}.tmp",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(RoomWorkspaceError::Invalid)?,
            std::process::id()
        ));
        let mut guard = TempGuard::new(temp.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| RoomWorkspaceError::Io)?;
        file.write_all(&body)
            .and_then(|_| file.sync_all())
            .map_err(|_| RoomWorkspaceError::Io)?;
        drop(file);
        let backup = self.backup_path();
        if backup.exists() {
            if !backup.is_file() {
                return Err(RoomWorkspaceError::Invalid);
            }
            fs::remove_file(&backup).map_err(|_| RoomWorkspaceError::Io)?;
        }
        if self.path.exists() {
            fs::rename(&self.path, &backup).map_err(|_| RoomWorkspaceError::Io)?;
        }
        if fs::rename(&temp, &self.path).is_err() {
            if !self.path.exists() && backup.is_file() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(RoomWorkspaceError::Io);
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

pub(crate) struct DesktopRoomWorkspaces {
    bindings: Mutex<BTreeMap<String, PathBuf>>,
    persistence: Option<WorkspacePersistence>,
}

impl DesktopRoomWorkspaces {
    #[cfg(test)]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            bindings: Mutex::new(BTreeMap::new()),
            persistence: None,
        })
    }

    fn persistent(path: PathBuf) -> Result<Arc<Self>, RoomWorkspaceError> {
        let persistence = WorkspacePersistence::new(path);
        let bindings = persistence.load()?;
        Ok(Arc::new(Self {
            bindings: Mutex::new(bindings),
            persistence: Some(persistence),
        }))
    }

    pub(crate) fn bind(&self, room_id: &str, root: PathBuf) -> Result<(), RoomWorkspaceError> {
        if !valid_room_id(room_id) || !root.is_dir() {
            return Err(RoomWorkspaceError::Invalid);
        }
        let canonical = safe_workspace_root(&root)?;
        let mut bindings = self.bindings.lock().map_err(|_| RoomWorkspaceError::Io)?;
        let mut next = bindings.clone();
        next.insert(room_id.to_owned(), canonical);
        if let Some(persistence) = &self.persistence {
            persistence.persist(&next)?;
        }
        *bindings = next;
        Ok(())
    }

    pub(crate) fn clear(&self, room_id: &str) -> Result<(), RoomWorkspaceError> {
        if !valid_room_id(room_id) {
            return Err(RoomWorkspaceError::Invalid);
        }
        let mut bindings = self.bindings.lock().map_err(|_| RoomWorkspaceError::Io)?;
        let mut next = bindings.clone();
        next.remove(room_id);
        if let Some(persistence) = &self.persistence {
            persistence.persist(&next)?;
        }
        *bindings = next;
        Ok(())
    }

    pub(crate) fn configured_root(
        &self,
        room_id: &str,
    ) -> Result<Option<PathBuf>, RoomWorkspaceError> {
        let bindings = self.bindings.lock().map_err(|_| RoomWorkspaceError::Io)?;
        Ok(bindings.get(room_id).cloned())
    }

    pub(crate) fn available_root(
        &self,
        room_id: &str,
    ) -> Result<Option<PathBuf>, RoomWorkspaceError> {
        let Some(root) = self.configured_root(room_id)? else {
            return Ok(None);
        };
        safe_workspace_root(&root).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RoomWorkspaceMode {
    ChatOnly,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomWorkspaceStatus {
    ok: bool,
    room_id: String,
    mode: RoomWorkspaceMode,
    folder_name: Option<String>,
    available: bool,
    changed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomWorkspaceChoiceEvent {
    room_id: String,
    changed: bool,
    error_code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomWorkspaceCommandError {
    code: &'static str,
    message: &'static str,
}

fn status(
    source: &DesktopRoomSource,
    workspaces: &DesktopRoomWorkspaces,
    room_id: &str,
    changed: bool,
) -> Result<RoomWorkspaceStatus, RoomWorkspaceCommandError> {
    ensure_room_exists(source, room_id)?;
    let root = workspaces.configured_root(room_id).map_err(map_error)?;
    let folder_name = root.as_ref().map(|root| {
        root.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned())
    });
    let available = match root.as_ref() {
        Some(root) => safe_workspace_root(root).is_ok(),
        None => true,
    };
    Ok(RoomWorkspaceStatus {
        ok: true,
        room_id: room_id.to_owned(),
        mode: if root.is_some() {
            RoomWorkspaceMode::Workspace
        } else {
            RoomWorkspaceMode::ChatOnly
        },
        folder_name,
        available,
        changed,
    })
}

fn ensure_room_exists(
    source: &DesktopRoomSource,
    room_id: &str,
) -> Result<(), RoomWorkspaceCommandError> {
    if !valid_room_id(room_id) {
        return Err(map_error(RoomWorkspaceError::Invalid));
    }
    let rooms = source
        .list_rooms()
        .map_err(|_| map_error(RoomWorkspaceError::Io))?;
    if rooms.iter().any(|room| room.id == room_id) {
        Ok(())
    } else {
        Err(map_error(RoomWorkspaceError::RoomNotFound))
    }
}

fn valid_room_id(room_id: &str) -> bool {
    !room_id.is_empty()
        && room_id.len() <= 128
        && room_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn safe_workspace_root(root: &Path) -> Result<PathBuf, RoomWorkspaceError> {
    if !root.is_absolute() {
        return Err(RoomWorkspaceError::Invalid);
    }
    if is_filesystem_link(root)? {
        return Err(RoomWorkspaceError::UnsafeLink);
    }
    let canonical = root
        .canonicalize()
        .map_err(|_| RoomWorkspaceError::Unavailable)?;
    if !canonical.is_dir() || is_filesystem_link(&canonical)? {
        return Err(RoomWorkspaceError::UnsafeLink);
    }
    Ok(canonical)
}

#[cfg(windows)]
fn is_filesystem_link(path: &Path) -> Result<bool, RoomWorkspaceError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata = fs::symlink_metadata(path).map_err(|_| RoomWorkspaceError::Unavailable)?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_filesystem_link(path: &Path) -> Result<bool, RoomWorkspaceError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| RoomWorkspaceError::Unavailable)?;
    Ok(metadata.file_type().is_symlink())
}

fn map_error(error: RoomWorkspaceError) -> RoomWorkspaceCommandError {
    match error {
        RoomWorkspaceError::Invalid => RoomWorkspaceCommandError {
            code: "roomWorkspaceInvalid",
            message: "The Room workspace selection is invalid.",
        },
        RoomWorkspaceError::RoomNotFound => RoomWorkspaceCommandError {
            code: "roomNotFound",
            message: "The Room does not exist.",
        },
        RoomWorkspaceError::Unavailable => RoomWorkspaceCommandError {
            code: "roomWorkspaceUnavailable",
            message: "The selected Room workspace is unavailable.",
        },
        RoomWorkspaceError::UnsafeLink => RoomWorkspaceCommandError {
            code: "roomWorkspaceUnsafeLink",
            message: "Linked folders cannot be used as a Room workspace.",
        },
        RoomWorkspaceError::Io => RoomWorkspaceCommandError {
            code: "roomWorkspaceUnavailable",
            message: "The Room workspace settings could not be accessed.",
        },
    }
}

#[tauri::command]
pub(crate) fn desktop_room_workspace_status(
    source: State<'_, Arc<DesktopRoomSource>>,
    workspaces: State<'_, Arc<DesktopRoomWorkspaces>>,
    room_id: String,
) -> Result<RoomWorkspaceStatus, RoomWorkspaceCommandError> {
    status(source.inner(), workspaces.inner(), &room_id, false)
}

#[tauri::command]
pub(crate) fn desktop_room_workspace_choose(
    app: AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
    workspaces: State<'_, Arc<DesktopRoomWorkspaces>>,
    room_id: String,
) -> Result<RoomWorkspaceStatus, RoomWorkspaceCommandError> {
    ensure_room_exists(source.inner(), &room_id)?;
    let mut dialog = app
        .dialog()
        .file()
        .set_title("M.I.O.の作業フォルダーを選択");
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    let event_app = app.clone();
    let event_room_id = room_id.clone();
    let event_workspaces = workspaces.inner().clone();
    dialog.pick_folder(move |selected| {
        let (changed, error_code) = match selected {
            Some(selected) => match selected.into_path() {
                Ok(root) => match event_workspaces.bind(&event_room_id, root) {
                    Ok(()) => (true, None),
                    Err(error) => (false, Some(map_error(error).code)),
                },
                Err(_) => (false, Some(map_error(RoomWorkspaceError::Invalid).code)),
            },
            None => (false, None),
        };
        let _ = event_app.emit(
            ROOM_WORKSPACE_CHOICE_EVENT,
            RoomWorkspaceChoiceEvent {
                room_id: event_room_id,
                changed,
                error_code,
            },
        );
    });
    status(source.inner(), workspaces.inner(), &room_id, false)
}

#[tauri::command]
pub(crate) fn desktop_room_workspace_clear(
    source: State<'_, Arc<DesktopRoomSource>>,
    workspaces: State<'_, Arc<DesktopRoomWorkspaces>>,
    room_id: String,
) -> Result<RoomWorkspaceStatus, RoomWorkspaceCommandError> {
    ensure_room_exists(source.inner(), &room_id)?;
    workspaces.clear(&room_id).map_err(map_error)?;
    status(source.inner(), workspaces.inner(), &room_id, true)
}

pub(crate) fn product_workspace_file(app_data_dir: &Path) -> PathBuf {
    env::var_os("MOE_ROOM_WORKSPACE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_data_dir.join(WORKSPACE_FILE_NAME))
}

pub(crate) fn persistent_room_workspaces(
    path: PathBuf,
) -> Result<Arc<DesktopRoomWorkspaces>, RoomWorkspaceError> {
    DesktopRoomWorkspaces::persistent(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn create_junction(link: &Path, target: &Path) {
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "New-Item -ItemType Junction -Path $env:MOE_JUNCTION_LINK -Target $env:MOE_JUNCTION_TARGET | Out-Null",
            ])
            .env("MOE_JUNCTION_LINK", link)
            .env("MOE_JUNCTION_TARGET", target)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn isolated_root(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "moe-room-workspace-{label}-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn persists_only_the_canonical_room_workspace_binding() {
        let root = isolated_root("persist");
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let settings_file = root.join("settings.json");
        let settings = DesktopRoomWorkspaces::persistent(settings_file.clone()).unwrap();

        settings.bind("moe-dev-room", workspace.clone()).unwrap();
        let reloaded = DesktopRoomWorkspaces::persistent(settings_file).unwrap();

        assert_eq!(
            reloaded.available_root("moe-dev-room").unwrap(),
            Some(workspace.canonicalize().unwrap())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn chat_only_clear_removes_the_persisted_binding() {
        let root = isolated_root("clear");
        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        let settings_file = root.join("settings.json");
        let settings = DesktopRoomWorkspaces::persistent(settings_file.clone()).unwrap();
        settings.bind("moe-dev-room", workspace).unwrap();

        settings.clear("moe-dev-room").unwrap();
        let reloaded = DesktopRoomWorkspaces::persistent(settings_file).unwrap();

        assert_eq!(reloaded.available_root("moe-dev-room").unwrap(), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn rejects_a_selected_or_replaced_junction_root() {
        let root = isolated_root("junction-root");
        let outside = root.join("outside");
        let linked_workspace = root.join("linked-workspace");
        fs::create_dir(&outside).unwrap();
        create_junction(&linked_workspace, &outside);
        let settings = DesktopRoomWorkspaces::in_memory();

        assert_eq!(
            settings.bind("moe-dev-room", linked_workspace.clone()),
            Err(RoomWorkspaceError::UnsafeLink)
        );
        fs::remove_dir(&linked_workspace).unwrap();

        let workspace = root.join("workspace");
        fs::create_dir(&workspace).unwrap();
        settings.bind("moe-dev-room", workspace.clone()).unwrap();
        fs::remove_dir(&workspace).unwrap();
        create_junction(&workspace, &outside);
        assert_eq!(
            settings.available_root("moe-dev-room"),
            Err(RoomWorkspaceError::UnsafeLink)
        );

        fs::remove_dir(&workspace).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
