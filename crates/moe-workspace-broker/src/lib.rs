#![forbid(unsafe_code)]

//! Host-owned boundary for Room workspace broker operations.
//!
//! This crate deliberately does not expose the resolved host path. Provider
//! adapters must not receive it. Bounded file I/O remains behind an open
//! directory capability. Command mediation is intentionally out of scope.

use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAXIMUM_RELATIVE_PATH_BYTES: usize = 4_096;
const MAXIMUM_RELATIVE_PATH_COMPONENTS: usize = 256;
pub const MAXIMUM_FILE_BYTES: usize = 1_048_576;
pub const MAXIMUM_BINARY_FILE_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_DIRECTORY_ENTRIES: usize = 256;
const MAXIMUM_DIRECTORY_LISTING_BYTES: usize = 64 * 1_024;

static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceBoundaryError {
    InvalidRoot,
    InvalidRelativePath,
    Unavailable,
    UnsafeLink,
    NotFile,
    NotDirectory,
    AlreadyExists,
    ContentTooLarge,
    DirectoryTooLarge,
}

impl fmt::Display for WorkspaceBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoot => "the workspace root is invalid",
            Self::InvalidRelativePath => "the workspace-relative path is invalid",
            Self::Unavailable => "the workspace target is unavailable",
            Self::UnsafeLink => "filesystem links are outside the workspace broker boundary",
            Self::NotFile => "the workspace target is not a regular file",
            Self::NotDirectory => "the workspace target is not a directory",
            Self::AlreadyExists => "the workspace target already exists",
            Self::ContentTooLarge => "the workspace file exceeds the broker size limit",
            Self::DirectoryTooLarge => "the workspace directory exceeds the broker listing limit",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceEntryKind {
    File,
    Directory,
    BlockedLink,
    Other,
}

impl WorkspaceEntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::BlockedLink => "blocked_link",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDirectoryEntry {
    name: String,
    kind: WorkspaceEntryKind,
}

impl WorkspaceDirectoryEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> WorkspaceEntryKind {
        self.kind
    }
}

#[derive(Debug)]
pub struct WorkspaceBroker {
    boundary: WorkspaceBoundary,
    root: Dir,
}

impl WorkspaceBroker {
    pub fn new(root: &Path) -> Result<Self, WorkspaceBoundaryError> {
        let boundary = WorkspaceBoundary::new(root)?;
        let root = Dir::open_ambient_dir(&boundary.canonical_root, ambient_authority())
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        Ok(Self { boundary, root })
    }

    pub fn read_file(&self, relative: &Path) -> Result<Vec<u8>, WorkspaceBoundaryError> {
        let target = self.boundary.existing_file(relative)?;
        let file = self
            .root
            .open(target.relative())
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        let metadata = file
            .metadata()
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if !metadata.is_file() {
            return Err(WorkspaceBoundaryError::NotFile);
        }
        if has_multiple_hard_links(&metadata) {
            return Err(WorkspaceBoundaryError::UnsafeLink);
        }
        if metadata.len() > MAXIMUM_FILE_BYTES as u64 {
            return Err(WorkspaceBoundaryError::ContentTooLarge);
        }

        let mut contents = Vec::with_capacity(metadata.len() as usize);
        file.take(MAXIMUM_FILE_BYTES as u64 + 1)
            .read_to_end(&mut contents)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if contents.len() > MAXIMUM_FILE_BYTES {
            Err(WorkspaceBoundaryError::ContentTooLarge)
        } else {
            Ok(contents)
        }
    }

    pub fn list_directory(
        &self,
        relative: &Path,
    ) -> Result<Vec<WorkspaceDirectoryEntry>, WorkspaceBoundaryError> {
        let target = self.boundary.listing_directory(relative)?;
        let read_dir = self
            .root
            .read_dir(target.relative())
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        let mut entries = Vec::new();
        let mut listing_bytes = 0usize;
        for entry in read_dir {
            if entries.len() >= MAXIMUM_DIRECTORY_ENTRIES {
                return Err(WorkspaceBoundaryError::DirectoryTooLarge);
            }
            let entry = entry.map_err(|_| WorkspaceBoundaryError::Unavailable)?;
            let file_name = entry.file_name();
            let name = file_name
                .to_str()
                .ok_or(WorkspaceBoundaryError::InvalidRelativePath)?
                .to_owned();
            listing_bytes = listing_bytes
                .checked_add(name.len())
                .ok_or(WorkspaceBoundaryError::DirectoryTooLarge)?;
            if listing_bytes > MAXIMUM_DIRECTORY_LISTING_BYTES {
                return Err(WorkspaceBoundaryError::DirectoryTooLarge);
            }
            let metadata = fs::symlink_metadata(target.resolved.join(&file_name))
                .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
            let kind = if is_filesystem_link(&metadata) {
                WorkspaceEntryKind::BlockedLink
            } else if metadata.is_file() {
                WorkspaceEntryKind::File
            } else if metadata.is_dir() {
                WorkspaceEntryKind::Directory
            } else {
                WorkspaceEntryKind::Other
            };
            entries.push(WorkspaceDirectoryEntry { name, kind });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    pub fn create_file(
        &self,
        relative: &Path,
        contents: &[u8],
    ) -> Result<(), WorkspaceBoundaryError> {
        validate_contents_size(contents)?;
        let target = self.boundary.new_file(relative)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file =
            self.root
                .open_with(target.relative(), &options)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::AlreadyExists => WorkspaceBoundaryError::AlreadyExists,
                    _ => WorkspaceBoundaryError::Unavailable,
                })?;
        if file
            .write_all(contents)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            drop(file);
            let _ = self.root.remove_file(target.relative());
            return Err(WorkspaceBoundaryError::Unavailable);
        }
        Ok(())
    }

    pub fn create_binary_file(
        &self,
        relative: &Path,
        contents: &[u8],
    ) -> Result<(), WorkspaceBoundaryError> {
        if contents.len() > MAXIMUM_BINARY_FILE_BYTES {
            return Err(WorkspaceBoundaryError::ContentTooLarge);
        }
        let target = self.boundary.new_file(relative)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file =
            self.root
                .open_with(target.relative(), &options)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::AlreadyExists => WorkspaceBoundaryError::AlreadyExists,
                    _ => WorkspaceBoundaryError::Unavailable,
                })?;
        if file
            .write_all(contents)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            drop(file);
            let _ = self.root.remove_file(target.relative());
            return Err(WorkspaceBoundaryError::Unavailable);
        }
        Ok(())
    }

    pub fn replace_file(
        &self,
        relative: &Path,
        contents: &[u8],
    ) -> Result<(), WorkspaceBoundaryError> {
        validate_contents_size(contents)?;
        let target = self.boundary.existing_file(relative)?;
        let current = self
            .root
            .open(target.relative())
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        let metadata = current
            .metadata()
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if !metadata.is_file() {
            return Err(WorkspaceBoundaryError::NotFile);
        }
        if has_multiple_hard_links(&metadata) {
            return Err(WorkspaceBoundaryError::UnsafeLink);
        }
        drop(current);
        let temporary = temporary_sibling(target.relative())?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = self
            .root
            .open_with(&temporary, &options)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;

        if file
            .write_all(contents)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            drop(file);
            let _ = self.root.remove_file(&temporary);
            return Err(WorkspaceBoundaryError::Unavailable);
        }
        drop(file);
        if self
            .root
            .rename(&temporary, &self.root, target.relative())
            .is_err()
        {
            let _ = self.root.remove_file(&temporary);
            return Err(WorkspaceBoundaryError::Unavailable);
        }
        Ok(())
    }
}

fn validate_contents_size(contents: &[u8]) -> Result<(), WorkspaceBoundaryError> {
    if contents.len() > MAXIMUM_FILE_BYTES {
        Err(WorkspaceBoundaryError::ContentTooLarge)
    } else {
        Ok(())
    }
}

fn temporary_sibling(relative: &Path) -> Result<PathBuf, WorkspaceBoundaryError> {
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let file_name = relative
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(WorkspaceBoundaryError::InvalidRelativePath)?;
    let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{file_name}.mio-{}-{sequence}.tmp",
        std::process::id()
    )))
}

impl Error for WorkspaceBoundaryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTarget {
    relative: PathBuf,
    resolved: PathBuf,
}

impl WorkspaceTarget {
    pub fn relative(&self) -> &Path {
        &self.relative
    }

    /// Returns the host-only canonical target selected by this boundary.
    /// Provider adapters must never serialize or expose this path.
    pub fn resolved_for_host(&self) -> &Path {
        &self.resolved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceBoundary {
    canonical_root: PathBuf,
}

impl WorkspaceBoundary {
    pub fn new(root: &Path) -> Result<Self, WorkspaceBoundaryError> {
        if !root.is_absolute() {
            return Err(WorkspaceBoundaryError::InvalidRoot);
        }
        let metadata =
            fs::symlink_metadata(root).map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if is_filesystem_link(&metadata) {
            return Err(WorkspaceBoundaryError::UnsafeLink);
        }
        if !metadata.is_dir() {
            return Err(WorkspaceBoundaryError::InvalidRoot);
        }
        let canonical_root = root
            .canonicalize()
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        let canonical_metadata = fs::symlink_metadata(&canonical_root)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if is_filesystem_link(&canonical_metadata) {
            return Err(WorkspaceBoundaryError::UnsafeLink);
        }
        if !canonical_metadata.is_dir() {
            return Err(WorkspaceBoundaryError::InvalidRoot);
        }
        Ok(Self { canonical_root })
    }

    pub fn existing_file(
        &self,
        relative: &Path,
    ) -> Result<WorkspaceTarget, WorkspaceBoundaryError> {
        let target = self.existing_target(relative)?;
        let metadata = fs::symlink_metadata(&target.resolved)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if metadata.is_file() {
            Ok(target)
        } else {
            Err(WorkspaceBoundaryError::NotFile)
        }
    }

    pub fn existing_directory(
        &self,
        relative: &Path,
    ) -> Result<WorkspaceTarget, WorkspaceBoundaryError> {
        let target = self.existing_target(relative)?;
        let metadata = fs::symlink_metadata(&target.resolved)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if metadata.is_dir() {
            Ok(target)
        } else {
            Err(WorkspaceBoundaryError::NotDirectory)
        }
    }

    fn listing_directory(
        &self,
        relative: &Path,
    ) -> Result<WorkspaceTarget, WorkspaceBoundaryError> {
        if relative == Path::new(".") {
            return Ok(WorkspaceTarget {
                relative: PathBuf::from("."),
                resolved: self.canonical_root.clone(),
            });
        }
        self.existing_directory(relative)
    }

    pub fn new_file(&self, relative: &Path) -> Result<WorkspaceTarget, WorkspaceBoundaryError> {
        let relative = workspace_relative_path(relative)?;
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let resolved_parent = self.walk_existing(parent)?;
        let parent_metadata = fs::symlink_metadata(&resolved_parent)
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if !parent_metadata.is_dir() {
            return Err(WorkspaceBoundaryError::NotDirectory);
        }
        let file_name = relative
            .file_name()
            .ok_or(WorkspaceBoundaryError::InvalidRelativePath)?;
        let resolved = resolved_parent.join(file_name);
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) if is_filesystem_link(&metadata) => {
                return Err(WorkspaceBoundaryError::UnsafeLink);
            }
            Ok(_) => return Err(WorkspaceBoundaryError::AlreadyExists),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(WorkspaceBoundaryError::Unavailable),
        }
        Ok(WorkspaceTarget { relative, resolved })
    }

    fn existing_target(&self, relative: &Path) -> Result<WorkspaceTarget, WorkspaceBoundaryError> {
        let relative = workspace_relative_path(relative)?;
        let resolved = self.walk_existing(&relative)?;
        Ok(WorkspaceTarget { relative, resolved })
    }

    fn walk_existing(&self, relative: &Path) -> Result<PathBuf, WorkspaceBoundaryError> {
        let mut current = self.canonical_root.clone();
        for component in relative.components() {
            let Component::Normal(segment) = component else {
                return Err(WorkspaceBoundaryError::InvalidRelativePath);
            };
            current.push(segment);
            let metadata =
                fs::symlink_metadata(&current).map_err(|_| WorkspaceBoundaryError::Unavailable)?;
            if is_filesystem_link(&metadata) {
                return Err(WorkspaceBoundaryError::UnsafeLink);
            }
        }
        let canonical = current
            .canonicalize()
            .map_err(|_| WorkspaceBoundaryError::Unavailable)?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(WorkspaceBoundaryError::UnsafeLink);
        }
        Ok(canonical)
    }
}

fn workspace_relative_path(path: &Path) -> Result<PathBuf, WorkspaceBoundaryError> {
    if path.as_os_str().is_empty()
        || path.as_os_str().as_encoded_bytes().len() > MAXIMUM_RELATIVE_PATH_BYTES
        || path.is_absolute()
    {
        return Err(WorkspaceBoundaryError::InvalidRelativePath);
    }
    let mut relative = PathBuf::new();
    let mut component_count = 0;
    for component in path.components() {
        match component {
            Component::Normal(segment) if valid_path_segment(segment) => {
                component_count += 1;
                if component_count > MAXIMUM_RELATIVE_PATH_COMPONENTS {
                    return Err(WorkspaceBoundaryError::InvalidRelativePath);
                }
                relative.push(segment);
            }
            Component::Normal(_) => return Err(WorkspaceBoundaryError::InvalidRelativePath),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => {
                return Err(WorkspaceBoundaryError::InvalidRelativePath);
            }
        }
    }
    if relative.as_os_str().is_empty() {
        Err(WorkspaceBoundaryError::InvalidRelativePath)
    } else {
        Ok(relative)
    }
}

#[cfg(windows)]
fn valid_path_segment(segment: &OsStr) -> bool {
    let Some(segment) = segment.to_str() else {
        return false;
    };
    if segment.is_empty() || segment.ends_with([' ', '.']) || segment.contains([':', '\0']) {
        return false;
    }
    let device_name = segment
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !matches!(
        device_name.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) && !windows_numbered_device_name(&device_name)
}

#[cfg(windows)]
fn windows_numbered_device_name(device_name: &str) -> bool {
    let Some(number) = device_name
        .strip_prefix("COM")
        .or_else(|| device_name.strip_prefix("LPT"))
    else {
        return false;
    };
    matches!(
        number,
        "0" | "1"
            | "2"
            | "3"
            | "4"
            | "5"
            | "6"
            | "7"
            | "8"
            | "9"
            | "⁰"
            | "¹"
            | "²"
            | "³"
            | "⁴"
            | "⁵"
            | "⁶"
            | "⁷"
            | "⁸"
            | "⁹"
    )
}

#[cfg(not(windows))]
fn valid_path_segment(segment: &OsStr) -> bool {
    !segment.as_encoded_bytes().contains(&0)
}

#[cfg(windows)]
fn is_filesystem_link(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_filesystem_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn has_multiple_hard_links(metadata: &cap_std::fs::Metadata) -> bool {
    use cap_fs_ext::MetadataExt;

    metadata.nlink() > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "moe-workspace-broker-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(windows)]
    fn create_directory_link(link: &Path, target: &Path) {
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "New-Item -ItemType Junction -Path $env:MOE_BROKER_LINK -Target $env:MOE_BROKER_TARGET | Out-Null",
            ])
            .env("MOE_BROKER_LINK", link)
            .env("MOE_BROKER_TARGET", target)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[cfg(unix)]
    fn create_directory_link(link: &Path, target: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[test]
    fn accepts_only_normal_workspace_relative_paths() {
        let fixture = TestRoot::new("relative");
        let workspace = fixture.0.join("workspace");
        let nested = workspace.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("input.txt"), b"safe").unwrap();
        let boundary = WorkspaceBoundary::new(&workspace).unwrap();

        assert_eq!(
            boundary
                .existing_file(Path::new("nested/input.txt"))
                .unwrap()
                .relative(),
            Path::new("nested/input.txt")
        );
        for invalid in [
            Path::new(""),
            Path::new("."),
            Path::new("../outside.txt"),
            Path::new("nested/../input.txt"),
            Path::new("/outside.txt"),
        ] {
            assert_eq!(
                boundary.existing_file(invalid),
                Err(WorkspaceBoundaryError::InvalidRelativePath)
            );
        }
        let oversized = PathBuf::from("a".repeat(MAXIMUM_RELATIVE_PATH_BYTES + 1));
        assert_eq!(
            boundary.existing_file(&oversized),
            Err(WorkspaceBoundaryError::InvalidRelativePath)
        );
        let mut too_deep = PathBuf::new();
        for _ in 0..=MAXIMUM_RELATIVE_PATH_COMPONENTS {
            too_deep.push("a");
        }
        assert_eq!(
            boundary.existing_file(&too_deep),
            Err(WorkspaceBoundaryError::InvalidRelativePath)
        );
        #[cfg(windows)]
        for invalid in [
            Path::new("C:outside.txt"),
            Path::new("nested/input.txt:stream"),
            Path::new("nested/input.txt."),
            Path::new("nested/input.txt "),
            Path::new("NUL"),
            Path::new("COM1.log"),
            Path::new("COM¹.log"),
            Path::new("LPT⁹"),
        ] {
            assert_eq!(
                boundary.existing_file(invalid),
                Err(WorkspaceBoundaryError::InvalidRelativePath)
            );
        }
    }

    #[test]
    fn performs_only_bounded_broker_owned_file_io() {
        let fixture = TestRoot::new("io");
        let workspace = fixture.0.join("workspace");
        let nested = workspace.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("input.txt"), b"before").unwrap();
        let broker = WorkspaceBroker::new(&workspace).unwrap();

        assert_eq!(
            broker.read_file(Path::new("nested/input.txt")).unwrap(),
            b"before"
        );
        broker
            .create_file(Path::new("nested/output.txt"), b"created")
            .unwrap();
        assert_eq!(fs::read(nested.join("output.txt")).unwrap(), b"created");
        assert_eq!(
            broker.create_file(Path::new("nested/output.txt"), b"duplicate"),
            Err(WorkspaceBoundaryError::AlreadyExists)
        );
        broker
            .replace_file(Path::new("nested/input.txt"), b"after")
            .unwrap();
        assert_eq!(fs::read(nested.join("input.txt")).unwrap(), b"after");

        let binary = vec![0xabu8; MAXIMUM_FILE_BYTES + 1];
        broker
            .create_binary_file(Path::new("nested/image.bin"), &binary)
            .unwrap();
        assert_eq!(fs::read(nested.join("image.bin")).unwrap(), binary);
        assert_eq!(
            broker.create_binary_file(Path::new("nested/image.bin"), b"duplicate"),
            Err(WorkspaceBoundaryError::AlreadyExists)
        );

        let oversized = vec![b'x'; MAXIMUM_FILE_BYTES + 1];
        assert_eq!(
            broker.create_file(Path::new("nested/too-large.txt"), &oversized),
            Err(WorkspaceBoundaryError::ContentTooLarge)
        );
        fs::write(nested.join("oversized.txt"), &oversized).unwrap();
        assert_eq!(
            broker.read_file(Path::new("nested/oversized.txt")),
            Err(WorkspaceBoundaryError::ContentTooLarge)
        );
    }

    #[test]
    fn lists_one_directory_level_with_bounded_sorted_metadata() {
        let fixture = TestRoot::new("listing");
        let workspace = fixture.0.join("workspace");
        let nested = workspace.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(workspace.join("zeta.txt"), b"z").unwrap();
        fs::write(workspace.join("alpha.txt"), b"a").unwrap();
        fs::write(nested.join("child.txt"), b"child").unwrap();
        let broker = WorkspaceBroker::new(&workspace).unwrap();

        let root_entries = broker.list_directory(Path::new(".")).unwrap();
        assert_eq!(
            root_entries
                .iter()
                .map(|entry| (entry.name(), entry.kind()))
                .collect::<Vec<_>>(),
            vec![
                ("alpha.txt", WorkspaceEntryKind::File),
                ("nested", WorkspaceEntryKind::Directory),
                ("zeta.txt", WorkspaceEntryKind::File),
            ]
        );
        assert_eq!(
            broker
                .list_directory(Path::new("nested"))
                .unwrap()
                .iter()
                .map(|entry| (entry.name(), entry.kind()))
                .collect::<Vec<_>>(),
            vec![("child.txt", WorkspaceEntryKind::File)]
        );
        assert_eq!(
            broker.list_directory(Path::new("alpha.txt")),
            Err(WorkspaceBoundaryError::NotDirectory)
        );
        assert_eq!(
            broker.list_directory(Path::new("./nested")),
            Err(WorkspaceBoundaryError::InvalidRelativePath)
        );
    }

    #[test]
    fn rejects_directory_listings_above_the_entry_limit() {
        let fixture = TestRoot::new("large-listing");
        let workspace = fixture.0.join("workspace");
        fs::create_dir(&workspace).unwrap();
        for index in 0..=MAXIMUM_DIRECTORY_ENTRIES {
            fs::write(workspace.join(format!("entry-{index:03}.txt")), b"x").unwrap();
        }
        let broker = WorkspaceBroker::new(&workspace).unwrap();

        assert_eq!(
            broker.list_directory(Path::new(".")),
            Err(WorkspaceBoundaryError::DirectoryTooLarge)
        );
    }

    #[test]
    fn distinguishes_files_directories_and_new_file_targets() {
        let fixture = TestRoot::new("kinds");
        let workspace = fixture.0.join("workspace");
        let nested = workspace.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("input.txt"), b"safe").unwrap();
        let boundary = WorkspaceBoundary::new(&workspace).unwrap();

        assert!(boundary.existing_directory(Path::new("nested")).is_ok());
        assert_eq!(
            boundary.existing_file(Path::new("nested")),
            Err(WorkspaceBoundaryError::NotFile)
        );
        assert_eq!(
            boundary.existing_directory(Path::new("nested/input.txt")),
            Err(WorkspaceBoundaryError::NotDirectory)
        );
        assert!(boundary.new_file(Path::new("nested/output.txt")).is_ok());
        assert_eq!(
            boundary.new_file(Path::new("nested/input.txt")),
            Err(WorkspaceBoundaryError::AlreadyExists)
        );
        assert_eq!(
            boundary.new_file(Path::new("missing/output.txt")),
            Err(WorkspaceBoundaryError::Unavailable)
        );
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn rejects_a_link_root_and_nested_link_targets() {
        let fixture = TestRoot::new("links");
        let workspace = fixture.0.join("workspace");
        let outside = fixture.0.join("outside");
        let linked_root = fixture.0.join("linked-root");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"outside").unwrap();
        create_directory_link(&linked_root, &outside);

        assert_eq!(
            WorkspaceBoundary::new(&linked_root),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );

        let escape = workspace.join("escape");
        create_directory_link(&escape, &outside);
        let boundary = WorkspaceBoundary::new(&workspace).unwrap();
        assert_eq!(
            boundary.existing_file(Path::new("escape/secret.txt")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(
            boundary.new_file(Path::new("escape/escaped.txt")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(
            boundary.new_file(Path::new("escape")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );

        let broker = WorkspaceBroker::new(&workspace).unwrap();
        assert_eq!(
            broker
                .list_directory(Path::new("."))
                .unwrap()
                .iter()
                .map(|entry| (entry.name(), entry.kind()))
                .collect::<Vec<_>>(),
            vec![("escape", WorkspaceEntryKind::BlockedLink)]
        );
        assert_eq!(
            broker.list_directory(Path::new("escape")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(
            broker.read_file(Path::new("escape/secret.txt")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(
            broker.create_file(Path::new("escape/escaped.txt"), b"blocked"),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert!(!outside.join("escaped.txt").exists());

        let capability = Dir::open_ambient_dir(&workspace, ambient_authority()).unwrap();
        assert!(capability.open(Path::new("escape/secret.txt")).is_err());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        assert!(
            capability
                .open_with(Path::new("escape/capability-escaped.txt"), &options)
                .is_err()
        );
        assert!(!outside.join("capability-escaped.txt").exists());

        fs::remove_dir(&escape).unwrap();
        fs::remove_dir(&linked_root).unwrap();
    }

    #[test]
    fn rejects_a_file_hard_linked_to_an_outside_name() {
        let fixture = TestRoot::new("hard-links");
        let workspace = fixture.0.join("workspace");
        let outside = fixture.0.join("outside");
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&outside).unwrap();
        let outside_file = outside.join("secret.txt");
        let workspace_link = workspace.join("linked.txt");
        fs::write(&outside_file, b"outside").unwrap();
        fs::hard_link(&outside_file, &workspace_link).unwrap();

        let broker = WorkspaceBroker::new(&workspace).unwrap();
        assert_eq!(
            broker.read_file(Path::new("linked.txt")),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(
            broker.replace_file(Path::new("linked.txt"), b"blocked"),
            Err(WorkspaceBoundaryError::UnsafeLink)
        );
        assert_eq!(fs::read(&outside_file).unwrap(), b"outside");
    }
}
