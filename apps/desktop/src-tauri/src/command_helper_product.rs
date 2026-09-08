use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const COMMAND_HELPER_FILE_NAME: &str = "mio-command-helper.exe";
const MAXIMUM_COMMAND_HELPER_BYTES: u64 = 32 * 1_024 * 1_024;
const BUNDLED_COMMAND_HELPER_SHA256: Option<&str> =
    option_env!("MOE_BUNDLED_COMMAND_HELPER_SHA256");

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandHelperProductError {
    InvalidBuildHash,
    InvalidDesktopExecutable,
    HelperUnavailable,
    UnsafeHelper,
    HelperChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    index: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BundledCommandHelper {
    path: PathBuf,
    expected_hash: [u8; 32],
    identity: FileIdentity,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum DesktopCommandHelperProduct {
    NotBundled,
    Ready(BundledCommandHelper),
    Rejected,
}

impl DesktopCommandHelperProduct {
    pub(crate) fn product() -> Self {
        match BundledCommandHelper::product() {
            Ok(Some(helper)) => Self::Ready(helper),
            Ok(None) => Self::NotBundled,
            Err(_) => Self::Rejected,
        }
    }

    pub(crate) fn ready(&self) -> Option<&BundledCommandHelper> {
        match self {
            Self::Ready(helper) => Some(helper),
            Self::NotBundled | Self::Rejected => None,
        }
    }
}

#[allow(dead_code)]
impl BundledCommandHelper {
    pub(crate) fn product() -> Result<Option<Self>, CommandHelperProductError> {
        let Some(expected_hash) = BUNDLED_COMMAND_HELPER_SHA256 else {
            return Ok(None);
        };
        let current_executable = std::env::current_exe()
            .map_err(|_| CommandHelperProductError::InvalidDesktopExecutable)?;
        Self::locate(&current_executable, expected_hash).map(Some)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandHelperProductError> {
        let inspected = inspect_helper(&self.path, self.expected_hash)?;
        if inspected != self.identity {
            return Err(CommandHelperProductError::HelperChanged);
        }
        Ok(())
    }

    fn locate(
        current_executable: &Path,
        expected_hash: &str,
    ) -> Result<Self, CommandHelperProductError> {
        let expected_hash = parse_sha256(expected_hash)?;
        if !current_executable.is_absolute() || is_filesystem_link(current_executable)? {
            return Err(CommandHelperProductError::InvalidDesktopExecutable);
        }
        let current_executable = current_executable
            .canonicalize()
            .map_err(|_| CommandHelperProductError::InvalidDesktopExecutable)?;
        if !current_executable.is_file() {
            return Err(CommandHelperProductError::InvalidDesktopExecutable);
        }
        let executable_directory = current_executable
            .parent()
            .filter(|parent| parent.is_absolute())
            .ok_or(CommandHelperProductError::InvalidDesktopExecutable)?;
        let helper = executable_directory.join(COMMAND_HELPER_FILE_NAME);
        let canonical_helper = helper
            .canonicalize()
            .map_err(|_| CommandHelperProductError::HelperUnavailable)?;
        if canonical_helper.parent() != Some(executable_directory) {
            return Err(CommandHelperProductError::UnsafeHelper);
        }
        let identity = inspect_helper(&canonical_helper, expected_hash)?;
        Ok(Self {
            path: canonical_helper,
            expected_hash,
            identity,
        })
    }

    #[cfg(test)]
    pub(crate) fn locate_for_test(
        current_executable: &Path,
        expected_hash: &str,
    ) -> Result<Self, CommandHelperProductError> {
        Self::locate(current_executable, expected_hash)
    }
}

fn inspect_helper(
    path: &Path,
    expected_hash: [u8; 32],
) -> Result<FileIdentity, CommandHelperProductError> {
    if !path.is_absolute() || is_filesystem_link(path)? {
        return Err(CommandHelperProductError::UnsafeHelper);
    }
    let metadata = fs::metadata(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAXIMUM_COMMAND_HELPER_BYTES {
        return Err(CommandHelperProductError::UnsafeHelper);
    }
    let (identity, link_count) = file_identity(path)?;
    if link_count != 1 {
        return Err(CommandHelperProductError::UnsafeHelper);
    }
    if hash_file(path)? != expected_hash {
        return Err(CommandHelperProductError::HelperChanged);
    }
    Ok(identity)
}

fn parse_sha256(value: &str) -> Result<[u8; 32], CommandHelperProductError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CommandHelperProductError::InvalidBuildHash);
    }
    let mut parsed = [0_u8; 32];
    for (index, output) in parsed.iter_mut().enumerate() {
        let offset = index * 2;
        *output = u8::from_str_radix(&value[offset..offset + 2], 16)
            .map_err(|_| CommandHelperProductError::InvalidBuildHash)?;
    }
    Ok(parsed)
}

fn hash_file(path: &Path) -> Result<[u8; 32], CommandHelperProductError> {
    let mut file = File::open(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16 * 1_024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| CommandHelperProductError::HelperUnavailable)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

#[cfg(windows)]
fn file_identity(path: &Path) -> Result<(FileIdentity, u64), CommandHelperProductError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let file = File::open(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe {
        GetFileInformationByHandle(
            file.as_raw_handle().cast(),
            std::ptr::addr_of_mut!(information),
        )
    } == 0
    {
        return Err(CommandHelperProductError::HelperUnavailable);
    }
    Ok((
        FileIdentity {
            volume: u64::from(information.dwVolumeSerialNumber),
            index: (u64::from(information.nFileIndexHigh) << 32)
                | u64::from(information.nFileIndexLow),
        },
        u64::from(information.nNumberOfLinks),
    ))
}

#[cfg(unix)]
fn file_identity(path: &Path) -> Result<(FileIdentity, u64), CommandHelperProductError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    Ok((
        FileIdentity {
            volume: metadata.dev(),
            index: metadata.ino(),
        },
        metadata.nlink(),
    ))
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_path: &Path) -> Result<(FileIdentity, u64), CommandHelperProductError> {
    Err(CommandHelperProductError::HelperUnavailable)
}

#[cfg(windows)]
fn is_filesystem_link(path: &Path) -> Result<bool, CommandHelperProductError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata =
        fs::symlink_metadata(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_filesystem_link(path: &Path) -> Result<bool, CommandHelperProductError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| CommandHelperProductError::HelperUnavailable)?;
    Ok(metadata.file_type().is_symlink())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "moe-command-helper-product-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn hash(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn fixture(label: &str) -> (TestDirectory, PathBuf, PathBuf, String) {
        let root = TestDirectory::new(label);
        let desktop = root.0.join("mio-desktop.exe");
        let helper = root.0.join(COMMAND_HELPER_FILE_NAME);
        fs::write(&desktop, b"desktop").unwrap();
        fs::write(&helper, b"helper").unwrap();
        let expected = hash(b"helper");
        (root, desktop, helper, expected)
    }

    #[test]
    fn accepts_only_the_exact_sibling_with_the_embedded_hash() {
        let (_root, desktop, helper, expected) = fixture("accept");
        let located = BundledCommandHelper::locate(&desktop, &expected).unwrap();
        assert_eq!(located.path(), helper.canonicalize().unwrap());
        located.revalidate().unwrap();
    }

    #[test]
    fn rejects_missing_wrong_hash_relative_and_extra_hard_link() {
        let (root, desktop, helper, expected) = fixture("reject");
        assert_eq!(
            BundledCommandHelper::locate(Path::new("mio-desktop.exe"), &expected),
            Err(CommandHelperProductError::InvalidDesktopExecutable)
        );
        assert_eq!(
            BundledCommandHelper::locate(&desktop, &hash(b"different")),
            Err(CommandHelperProductError::HelperChanged)
        );
        let extra_link = root.0.join("extra-helper.exe");
        fs::hard_link(&helper, &extra_link).unwrap();
        assert_eq!(
            BundledCommandHelper::locate(&desktop, &expected),
            Err(CommandHelperProductError::UnsafeHelper)
        );
        fs::remove_file(extra_link).unwrap();
        fs::remove_file(helper).unwrap();
        assert_eq!(
            BundledCommandHelper::locate(&desktop, &expected),
            Err(CommandHelperProductError::HelperUnavailable)
        );
    }

    #[test]
    fn revalidation_rejects_a_replaced_helper() {
        let (_root, desktop, helper, expected) = fixture("replace");
        let located = BundledCommandHelper::locate(&desktop, &expected).unwrap();
        fs::remove_file(&helper).unwrap();
        fs::write(&helper, b"replacement").unwrap();
        assert_eq!(
            located.revalidate(),
            Err(CommandHelperProductError::HelperChanged)
        );
    }

    #[test]
    fn rejects_invalid_build_hashes() {
        let invalid_hex = "g".repeat(64);
        for value in ["", "00", invalid_hex.as_str()] {
            assert_eq!(
                parse_sha256(value),
                Err(CommandHelperProductError::InvalidBuildHash)
            );
        }
    }
}
