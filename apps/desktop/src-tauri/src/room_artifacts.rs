use base64::{Engine as _, engine::general_purpose::STANDARD};
use moe_core::{RoomMessageFindError, RoomSource};
use moe_workspace_broker::{MAXIMUM_BINARY_FILE_BYTES, WorkspaceBoundaryError, WorkspaceBroker};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

use crate::room_source::DesktopRoomSource;
use crate::room_workspace::DesktopRoomWorkspaces;

const ARTIFACT_ID_PREFIX: &str = "artifact-image-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoomArtifactError {
    Invalid,
    NotFound,
    TooLarge,
    UnsafeSource,
    Unavailable,
    AlreadyExists,
    WorkspaceUnavailable,
}

impl fmt::Display for RoomArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "the Room image artifact is invalid",
            Self::NotFound => "the Room image artifact was not found",
            Self::TooLarge => "the Room image artifact is too large",
            Self::UnsafeSource => {
                "the Room image artifact source is outside the trusted image directory"
            }
            Self::Unavailable => "the Room image artifact is unavailable",
            Self::AlreadyExists => "the workspace file already exists",
            Self::WorkspaceUnavailable => "the Room workspace is unavailable",
        })
    }
}

impl Error for RoomArtifactError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageKind {
    Png,
    Jpeg,
    Webp,
}

impl ImageKind {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }
}

fn image_kind(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(ImageKind::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ImageKind::Jpeg)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageKind::Webp)
    } else {
        None
    }
}

fn artifact_id(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(ARTIFACT_ID_PREFIX.len() + digest.len() * 2);
    encoded.push_str(ARTIFACT_ID_PREFIX);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

pub(crate) struct RoomArtifactStore {
    root: PathBuf,
    trusted_generated_images_root: PathBuf,
}

impl RoomArtifactStore {
    pub(crate) fn product(app_data_dir: &Path) -> Result<Arc<Self>, RoomArtifactError> {
        let trusted_root = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex")))
            .ok_or(RoomArtifactError::Unavailable)?
            .join("generated_images");
        fs::create_dir_all(&trusted_root).map_err(|_| RoomArtifactError::Unavailable)?;
        Self::new(app_data_dir.join("room-artifacts-v1"), trusted_root).map(Arc::new)
    }

    fn new(
        root: PathBuf,
        trusted_generated_images_root: PathBuf,
    ) -> Result<Self, RoomArtifactError> {
        fs::create_dir_all(&root).map_err(|_| RoomArtifactError::Unavailable)?;
        let root = root
            .canonicalize()
            .map_err(|_| RoomArtifactError::Unavailable)?;
        let trusted_generated_images_root = trusted_generated_images_root
            .canonicalize()
            .map_err(|_| RoomArtifactError::Unavailable)?;
        Ok(Self {
            root,
            trusted_generated_images_root,
        })
    }

    pub(crate) fn ingest_generated_image(
        &self,
        source: &Path,
    ) -> Result<String, RoomArtifactError> {
        if !source.is_absolute() {
            return Err(RoomArtifactError::UnsafeSource);
        }
        let source = source
            .canonicalize()
            .map_err(|_| RoomArtifactError::NotFound)?;
        if !source.starts_with(&self.trusted_generated_images_root) {
            return Err(RoomArtifactError::UnsafeSource);
        }
        let metadata = fs::symlink_metadata(&source).map_err(|_| RoomArtifactError::NotFound)?;
        if !metadata.is_file() {
            return Err(RoomArtifactError::Invalid);
        }
        if metadata.len() > MAXIMUM_BINARY_FILE_BYTES as u64 {
            return Err(RoomArtifactError::TooLarge);
        }
        let bytes = fs::read(&source).map_err(|_| RoomArtifactError::Unavailable)?;
        if bytes.len() > MAXIMUM_BINARY_FILE_BYTES {
            return Err(RoomArtifactError::TooLarge);
        }
        let kind = image_kind(&bytes).ok_or(RoomArtifactError::Invalid)?;
        let id = artifact_id(&bytes);
        let destination = self.root.join(format!("{id}.{}", kind.extension()));
        if destination.exists() {
            return (fs::read(&destination).ok().as_deref() == Some(bytes.as_slice()))
                .then_some(id)
                .ok_or(RoomArtifactError::Invalid);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|_| RoomArtifactError::Unavailable)?;
        if file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            drop(file);
            let _ = fs::remove_file(destination);
            return Err(RoomArtifactError::Unavailable);
        }
        Ok(id)
    }

    fn read(&self, id: &str) -> Result<(ImageKind, Vec<u8>), RoomArtifactError> {
        if !valid_artifact_id(id) {
            return Err(RoomArtifactError::Invalid);
        }
        for kind in [ImageKind::Png, ImageKind::Jpeg, ImageKind::Webp] {
            let path = self.root.join(format!("{id}.{}", kind.extension()));
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(path).map_err(|_| RoomArtifactError::Unavailable)?;
            if bytes.len() > MAXIMUM_BINARY_FILE_BYTES
                || image_kind(&bytes) != Some(kind)
                || artifact_id(&bytes) != id
            {
                return Err(RoomArtifactError::Invalid);
            }
            return Ok((kind, bytes));
        }
        Err(RoomArtifactError::NotFound)
    }
}

fn valid_artifact_id(id: &str) -> bool {
    id.len() == ARTIFACT_ID_PREFIX.len() + 64
        && id.starts_with(ARTIFACT_ID_PREFIX)
        && id[ARTIFACT_ID_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn authorize_artifact(
    source: &DesktopRoomSource,
    room_id: &str,
    message_id: &str,
    artifact_id: &str,
) -> Result<(), RoomArtifactError> {
    let message = source
        .find_message(room_id, message_id)
        .map_err(|error| match error {
            RoomMessageFindError::InvalidLookup
            | RoomMessageFindError::RoomNotFound
            | RoomMessageFindError::MessageNotFound => RoomArtifactError::NotFound,
            RoomMessageFindError::SourceUnavailable => RoomArtifactError::Unavailable,
        })?;
    message
        .artifact_ids
        .iter()
        .any(|id| id == artifact_id)
        .then_some(())
        .ok_or(RoomArtifactError::NotFound)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomArtifactReadSuccess {
    ok: bool,
    id: String,
    media_type: String,
    file_name: String,
    byte_length: usize,
    data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomArtifactSaveSuccess {
    ok: bool,
    relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomArtifactCommandError {
    code: &'static str,
    message: &'static str,
}

fn command_error(error: RoomArtifactError) -> RoomArtifactCommandError {
    let (code, message) = match error {
        RoomArtifactError::Invalid => ("artifactInvalid", "The Room image is invalid."),
        RoomArtifactError::NotFound => ("artifactNotFound", "The Room image was not found."),
        RoomArtifactError::TooLarge => ("artifactTooLarge", "The Room image is too large."),
        RoomArtifactError::UnsafeSource => {
            ("artifactSourceDenied", "The image source is not trusted.")
        }
        RoomArtifactError::Unavailable => ("artifactUnavailable", "The Room image is unavailable."),
        RoomArtifactError::AlreadyExists => (
            "workspaceFileExists",
            "A file with that name already exists.",
        ),
        RoomArtifactError::WorkspaceUnavailable => (
            "workspaceUnavailable",
            "The selected editing folder is unavailable.",
        ),
    };
    RoomArtifactCommandError { code, message }
}

#[tauri::command]
pub(crate) fn desktop_room_artifact_read(
    source: State<'_, Arc<DesktopRoomSource>>,
    artifacts: State<'_, Arc<RoomArtifactStore>>,
    room_id: String,
    message_id: String,
    artifact_id: String,
) -> Result<RoomArtifactReadSuccess, RoomArtifactCommandError> {
    authorize_artifact(source.as_ref(), &room_id, &message_id, &artifact_id)
        .map_err(command_error)?;
    let (kind, bytes) = artifacts.read(&artifact_id).map_err(command_error)?;
    Ok(RoomArtifactReadSuccess {
        ok: true,
        id: artifact_id.clone(),
        media_type: kind.media_type().to_owned(),
        file_name: format!("{artifact_id}.{}", kind.extension()),
        byte_length: bytes.len(),
        data_base64: STANDARD.encode(bytes),
    })
}

#[tauri::command]
pub(crate) fn desktop_room_artifact_save_to_workspace(
    source: State<'_, Arc<DesktopRoomSource>>,
    artifacts: State<'_, Arc<RoomArtifactStore>>,
    workspaces: State<'_, Arc<DesktopRoomWorkspaces>>,
    room_id: String,
    message_id: String,
    artifact_id: String,
    relative_path: String,
) -> Result<RoomArtifactSaveSuccess, RoomArtifactCommandError> {
    authorize_artifact(source.as_ref(), &room_id, &message_id, &artifact_id)
        .map_err(command_error)?;
    let (kind, bytes) = artifacts.read(&artifact_id).map_err(command_error)?;
    let relative = Path::new(&relative_path);
    let extension_matches = relative
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case(kind.extension())
                || (kind == ImageKind::Jpeg && extension.eq_ignore_ascii_case("jpeg"))
        });
    if !extension_matches {
        return Err(command_error(RoomArtifactError::Invalid));
    }
    let root = workspaces
        .available_root(&room_id)
        .map_err(|_| command_error(RoomArtifactError::WorkspaceUnavailable))?
        .ok_or_else(|| command_error(RoomArtifactError::WorkspaceUnavailable))?;
    WorkspaceBroker::new(&root)
        .and_then(|broker| broker.create_binary_file(relative, &bytes))
        .map_err(|error| {
            command_error(match error {
                WorkspaceBoundaryError::AlreadyExists => RoomArtifactError::AlreadyExists,
                WorkspaceBoundaryError::ContentTooLarge => RoomArtifactError::TooLarge,
                WorkspaceBoundaryError::InvalidRoot | WorkspaceBoundaryError::Unavailable => {
                    RoomArtifactError::WorkspaceUnavailable
                }
                _ => RoomArtifactError::Invalid,
            })
        })?;
    Ok(RoomArtifactSaveSuccess {
        ok: true,
        relative_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn roots() -> (PathBuf, PathBuf) {
        let base = env::temp_dir().join(format!(
            "moe-room-artifact-test-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let store = base.join("store");
        let trusted = base.join("generated_images");
        fs::create_dir_all(&trusted).unwrap();
        (store, trusted)
    }

    #[test]
    fn ingests_only_supported_images_from_trusted_root() {
        let (store, trusted) = roots();
        let source = trusted.join("image.png");
        let bytes = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3];
        fs::write(&source, bytes).unwrap();
        let artifacts = RoomArtifactStore::new(store, trusted).unwrap();
        let id = artifacts.ingest_generated_image(&source).unwrap();
        assert!(id.starts_with(ARTIFACT_ID_PREFIX));
        assert_eq!(
            artifacts.read(&id).unwrap(),
            (ImageKind::Png, bytes.to_vec())
        );
    }

    #[test]
    fn rejects_an_image_outside_the_trusted_root() {
        let (store, trusted) = roots();
        let source = trusted.parent().unwrap().join("outside.png");
        fs::write(&source, [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).unwrap();
        let artifacts = RoomArtifactStore::new(store, trusted).unwrap();
        assert_eq!(
            artifacts.ingest_generated_image(&source),
            Err(RoomArtifactError::UnsafeSource)
        );
    }

    #[test]
    fn rejects_non_image_bytes() {
        let (store, trusted) = roots();
        let source = trusted.join("not-an-image.png");
        fs::write(&source, b"not an image").unwrap();
        let artifacts = RoomArtifactStore::new(store, trusted).unwrap();
        assert_eq!(
            artifacts.ingest_generated_image(&source),
            Err(RoomArtifactError::Invalid)
        );
    }
}
