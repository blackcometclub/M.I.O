use crate::room_source::{DesktopRoomPersistenceError, DesktopRoomSource};
use moe_core::RoomCatalogSource;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

const BACKUP_DIRECTORY_NAME: &str = "M.O.E Backups";
const BACKUP_PREFIX: &str = "moe-room-backup-";
const BACKUP_SUFFIX: &str = ".json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupSuccess {
    ok: bool,
    file_name: String,
    room_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomBackupCommandError {
    code: &'static str,
    message: &'static str,
}

fn map_error(_: DesktopRoomPersistenceError) -> RoomBackupCommandError {
    RoomBackupCommandError {
        code: "roomBackupUnavailable",
        message: "The Room backup could not be accessed.",
    }
}

fn backup_directory(app: &tauri::AppHandle) -> Result<PathBuf, RoomBackupCommandError> {
    if let Some(path) = std::env::var_os("MOE_ROOM_BACKUP_DIR") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return Ok(path);
        }
        return Err(map_error(DesktopRoomPersistenceError::InvalidFile));
    }
    app.path()
        .document_dir()
        .map(|path| path.join(BACKUP_DIRECTORY_NAME))
        .map_err(|_| map_error(DesktopRoomPersistenceError::Io))
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

fn export_rooms(
    source: &DesktopRoomSource,
    directory: &Path,
    stamp: u128,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    if !directory.is_absolute() {
        return Err(map_error(DesktopRoomPersistenceError::InvalidFile));
    }
    let destination = backup_file(directory, stamp);
    source
        .export_snapshot(destination.clone())
        .map_err(map_error)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| map_error(DesktopRoomPersistenceError::InvalidFile))?
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
    let entries =
        fs::read_dir(directory).map_err(|_| map_error(DesktopRoomPersistenceError::Io))?;
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let stamp = name
                .strip_prefix(BACKUP_PREFIX)?
                .strip_suffix(BACKUP_SUFFIX)?;
            if stamp.len() != 20 || !stamp.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            Some((name.to_owned(), entry.path()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, path)| path)
        .ok_or(RoomBackupCommandError {
            code: "roomBackupMissing",
            message: "No M.I.O. Room backup was found.",
        })
}

fn restore_latest_rooms(
    source: &DesktopRoomSource,
    directory: &Path,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    if !directory.is_absolute() {
        return Err(map_error(DesktopRoomPersistenceError::InvalidFile));
    }
    let path = latest_backup(directory)?;
    source.restore_snapshot(path.clone()).map_err(map_error)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| map_error(DesktopRoomPersistenceError::InvalidFile))?
        .to_owned();
    let room_count = source
        .list_rooms()
        .map_err(|_| RoomBackupCommandError {
            code: "roomBackupUnavailable",
            message: "The restored Room catalog could not be read.",
        })?
        .len();
    Ok(RoomBackupSuccess {
        ok: true,
        file_name,
        room_count,
    })
}

#[tauri::command]
pub(crate) fn desktop_room_backup(
    app: tauri::AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    export_rooms(
        source.as_ref(),
        &backup_directory(&app)?,
        current_backup_stamp()?,
    )
}

#[tauri::command]
pub(crate) fn desktop_room_restore_latest_backup(
    app: tauri::AppHandle,
    source: State<'_, Arc<DesktopRoomSource>>,
) -> Result<RoomBackupSuccess, RoomBackupCommandError> {
    restore_latest_rooms(source.as_ref(), &backup_directory(&app)?)
}

#[cfg(test)]
mod tests {
    use super::{export_rooms, restore_latest_rooms};
    use crate::room_source::desktop_room_source;
    use moe_core::{RoomCatalogSource, RoomStore};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn isolated_directory() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "moe-room-backup-command-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn exports_and_restores_only_the_latest_valid_moe_backup() {
        let directory = isolated_directory();
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

        let restored = restore_latest_rooms(source.as_ref(), &directory).unwrap();
        assert_eq!(
            restored.file_name,
            "moe-room-backup-00000000000000000002.json"
        );
        assert_eq!(restored.room_count, 3);
        assert_eq!(source.list_rooms().unwrap()[0].name, "Changed");

        fs::remove_dir_all(directory).unwrap();
    }
}
