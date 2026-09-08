use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const GIT_DIRECTORY: &str = "Git";
const GIT_EXECUTABLE_DIRECTORY: &str = "bin";
const GIT_EXECUTABLE_FILE_NAME: &str = "git.exe";
const MAXIMUM_GIT_EXECUTABLE_BYTES: u64 = 256 * 1_024 * 1_024;
const NODE_DIRECTORY: &str = "nodejs";
const NODE_EXECUTABLE_FILE_NAME: &str = "node.exe";
const MAXIMUM_NODE_EXECUTABLE_BYTES: u64 = 512 * 1_024 * 1_024;
const NPM_DIRECTORY: &str = "npm";
const NPM_CLI_RELATIVE_PATH: &str = "bin/npm-cli.js";
const NPM_PACKAGE_JSON_FILE_NAME: &str = "package.json";
const MAXIMUM_NPM_RUNTIME_FILES: usize = 4_096;
const MAXIMUM_NPM_RUNTIME_DIRECTORIES: usize = 1_024;
const MAXIMUM_NPM_RUNTIME_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
const MAXIMUM_NPM_RUNTIME_TOTAL_BYTES: u64 = 64 * 1_024 * 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandToolchainProductError {
    UnsafeInstallRoot,
    UnsafeGitExecutable,
    GitExecutableChanged,
    UnsafeNodeExecutable,
    NodeExecutableChanged,
    UnsafeNpmRuntime,
    NpmRuntimeChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedGitExecutable {
    path: PathBuf,
    identity: FileIdentity,
    hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedNodeExecutable {
    path: PathBuf,
    identity: FileIdentity,
    hash: [u8; 32],
    npm_runtime: Option<VerifiedNpmRuntime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedNpmRuntime {
    root: PathBuf,
    snapshot: NpmRuntimeSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NpmRuntimeSnapshot {
    file_count: usize,
    directory_count: usize,
    total_bytes: u64,
    tree_hash: [u8; 32],
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DesktopCommandToolchainProduct {
    GitUnavailable,
    GitReady(VerifiedGitExecutable),
    NodeReady(VerifiedNodeExecutable),
    GitAndNodeReady {
        git: VerifiedGitExecutable,
        node: VerifiedNodeExecutable,
    },
    Rejected,
}

impl DesktopCommandToolchainProduct {
    pub(crate) fn product() -> Self {
        let roots = product_install_roots();
        match (locate_git(&roots), locate_node(&roots)) {
            (Ok(git), Ok(node)) => Self::from_tools(git, node),
            (Err(_), _) | (_, Err(_)) => Self::Rejected,
        }
    }

    pub(crate) fn ready_git(&self) -> Option<&VerifiedGitExecutable> {
        match self {
            Self::GitReady(git) | Self::GitAndNodeReady { git, .. } => Some(git),
            Self::GitUnavailable | Self::NodeReady(_) | Self::Rejected => None,
        }
    }

    pub(crate) fn ready_node(&self) -> Option<&VerifiedNodeExecutable> {
        match self {
            Self::NodeReady(node) | Self::GitAndNodeReady { node, .. } => Some(node),
            Self::GitUnavailable | Self::GitReady(_) | Self::Rejected => None,
        }
    }

    pub(crate) fn ready_npm_runtime(&self) -> Option<&VerifiedNpmRuntime> {
        self.ready_node()
            .and_then(VerifiedNodeExecutable::npm_runtime)
    }

    fn from_tools(
        git: Option<VerifiedGitExecutable>,
        node: Option<VerifiedNodeExecutable>,
    ) -> Self {
        match (git, node) {
            (Some(git), Some(node)) => Self::GitAndNodeReady { git, node },
            (Some(git), None) => Self::GitReady(git),
            (None, Some(node)) => Self::NodeReady(node),
            (None, None) => Self::GitUnavailable,
        }
    }

    #[cfg(test)]
    pub(crate) fn locate_for_test(
        install_root: &Path,
    ) -> Result<Self, CommandToolchainProductError> {
        if !install_root.is_absolute() {
            return Err(CommandToolchainProductError::UnsafeInstallRoot);
        }
        let roots = [install_root.to_owned()];
        Ok(Self::from_tools(locate_git(&roots)?, locate_node(&roots)?))
    }
}

#[allow(dead_code)]
impl VerifiedGitExecutable {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandToolchainProductError> {
        let (identity, hash) = inspect_git(&self.path)?;
        if identity != self.identity || hash != self.hash {
            return Err(CommandToolchainProductError::GitExecutableChanged);
        }
        Ok(())
    }
}

#[allow(dead_code)]
impl VerifiedNodeExecutable {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandToolchainProductError> {
        let (identity, hash) = inspect_node(&self.path)?;
        if identity != self.identity || hash != self.hash {
            return Err(CommandToolchainProductError::NodeExecutableChanged);
        }
        Ok(())
    }

    pub(crate) fn npm_runtime(&self) -> Option<&VerifiedNpmRuntime> {
        self.npm_runtime.as_ref()
    }
}

#[allow(dead_code)]
impl VerifiedNpmRuntime {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn revalidate(&self) -> Result<(), CommandToolchainProductError> {
        let snapshot = inspect_npm_runtime(&self.root)?;
        if snapshot != self.snapshot {
            return Err(CommandToolchainProductError::NpmRuntimeChanged);
        }
        Ok(())
    }
}

fn product_install_roots() -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for variable in ["ProgramW6432", "ProgramFiles", "LOCALAPPDATA"] {
        let Some(root) = std::env::var_os(variable).map(PathBuf::from) else {
            continue;
        };
        if !root.is_absolute() {
            continue;
        }
        let root = if variable == "LOCALAPPDATA" {
            root.join("Programs")
        } else {
            root
        };
        roots.insert(root);
    }
    roots.into_iter().collect()
}

fn locate_git(
    install_roots: &[PathBuf],
) -> Result<Option<VerifiedGitExecutable>, CommandToolchainProductError> {
    let mut rejected = false;
    for install_root in install_roots {
        let candidate = install_root
            .join(GIT_DIRECTORY)
            .join(GIT_EXECUTABLE_DIRECTORY)
            .join(GIT_EXECUTABLE_FILE_NAME);
        if !candidate.exists() {
            continue;
        }
        match validate_candidate(install_root, &candidate) {
            Ok(git) => return Ok(Some(git)),
            Err(_) => rejected = true,
        }
    }
    if rejected {
        Err(CommandToolchainProductError::UnsafeGitExecutable)
    } else {
        Ok(None)
    }
}

fn locate_node(
    install_roots: &[PathBuf],
) -> Result<Option<VerifiedNodeExecutable>, CommandToolchainProductError> {
    let mut rejected = false;
    for install_root in install_roots {
        let candidate = install_root
            .join(NODE_DIRECTORY)
            .join(NODE_EXECUTABLE_FILE_NAME);
        if !candidate.exists() {
            continue;
        }
        match validate_node_candidate(install_root, &candidate) {
            Ok(node) => return Ok(Some(node)),
            Err(_) => rejected = true,
        }
    }
    if rejected {
        Err(CommandToolchainProductError::UnsafeNodeExecutable)
    } else {
        Ok(None)
    }
}

fn validate_candidate(
    install_root: &Path,
    candidate: &Path,
) -> Result<VerifiedGitExecutable, CommandToolchainProductError> {
    if !install_root.is_absolute() || !install_root.is_dir() || is_filesystem_link(install_root)? {
        return Err(CommandToolchainProductError::UnsafeInstallRoot);
    }
    let git_root = install_root.join(GIT_DIRECTORY);
    let executable_root = git_root.join(GIT_EXECUTABLE_DIRECTORY);
    if !git_root.is_dir()
        || !executable_root.is_dir()
        || is_filesystem_link(&git_root)?
        || is_filesystem_link(&executable_root)?
        || is_filesystem_link(candidate)?
    {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
    }
    let canonical_install_root = install_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeInstallRoot)?;
    let canonical_executable_root = executable_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    if !canonical_executable_root.starts_with(&canonical_install_root)
        || canonical_candidate.parent() != Some(canonical_executable_root.as_path())
        || !canonical_candidate
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(GIT_EXECUTABLE_FILE_NAME))
    {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
    }
    let (identity, hash) = inspect_git(&canonical_candidate)?;
    Ok(VerifiedGitExecutable {
        path: canonical_candidate,
        identity,
        hash,
    })
}

fn inspect_git(path: &Path) -> Result<(FileIdentity, [u8; 32]), CommandToolchainProductError> {
    if !path.is_absolute() || is_filesystem_link(path)? {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
    }
    let metadata =
        fs::metadata(path).map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAXIMUM_GIT_EXECUTABLE_BYTES {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
    }
    let (identity, link_count) = file_identity(path)?;
    if link_count != 1 {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
    }
    Ok((identity, hash_file(path)?))
}

fn validate_node_candidate(
    install_root: &Path,
    candidate: &Path,
) -> Result<VerifiedNodeExecutable, CommandToolchainProductError> {
    if !install_root.is_absolute()
        || !install_root.is_dir()
        || is_filesystem_link(install_root)
            .map_err(|_| CommandToolchainProductError::UnsafeInstallRoot)?
    {
        return Err(CommandToolchainProductError::UnsafeInstallRoot);
    }
    let executable_root = install_root.join(NODE_DIRECTORY);
    if !executable_root.is_dir()
        || is_filesystem_link(&executable_root)
            .map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?
        || is_filesystem_link(candidate)
            .map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?
    {
        return Err(CommandToolchainProductError::UnsafeNodeExecutable);
    }
    let canonical_install_root = install_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeInstallRoot)?;
    let canonical_executable_root = executable_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?;
    if !canonical_executable_root.starts_with(&canonical_install_root)
        || canonical_candidate.parent() != Some(canonical_executable_root.as_path())
        || !canonical_candidate
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(NODE_EXECUTABLE_FILE_NAME))
    {
        return Err(CommandToolchainProductError::UnsafeNodeExecutable);
    }
    let (identity, hash) = inspect_node(&canonical_candidate)?;
    let npm_root = canonical_executable_root
        .join("node_modules")
        .join(NPM_DIRECTORY);
    let npm_runtime = npm_root
        .exists()
        .then(|| validate_npm_runtime(&canonical_executable_root, &npm_root))
        .transpose()
        .ok()
        .flatten();
    Ok(VerifiedNodeExecutable {
        path: canonical_candidate,
        identity,
        hash,
        npm_runtime,
    })
}

fn inspect_node(path: &Path) -> Result<(FileIdentity, [u8; 32]), CommandToolchainProductError> {
    if !path.is_absolute()
        || is_filesystem_link(path)
            .map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?
    {
        return Err(CommandToolchainProductError::UnsafeNodeExecutable);
    }
    let metadata =
        fs::metadata(path).map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAXIMUM_NODE_EXECUTABLE_BYTES
    {
        return Err(CommandToolchainProductError::UnsafeNodeExecutable);
    }
    let (identity, link_count) =
        file_identity(path).map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?;
    if link_count != 1 {
        return Err(CommandToolchainProductError::UnsafeNodeExecutable);
    }
    let hash = hash_file(path).map_err(|_| CommandToolchainProductError::UnsafeNodeExecutable)?;
    Ok((identity, hash))
}

fn validate_npm_runtime(
    node_root: &Path,
    npm_root: &Path,
) -> Result<VerifiedNpmRuntime, CommandToolchainProductError> {
    if !node_root.is_absolute()
        || !node_root.is_dir()
        || !npm_root.is_dir()
        || is_filesystem_link(node_root)
            .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?
        || is_filesystem_link(&node_root.join("node_modules"))
            .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?
        || is_filesystem_link(npm_root)
            .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?
    {
        return Err(CommandToolchainProductError::UnsafeNpmRuntime);
    }
    let canonical_node_root = node_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
    let canonical_npm_root = npm_root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
    if !canonical_npm_root.starts_with(&canonical_node_root)
        || !canonical_npm_root.join(NPM_CLI_RELATIVE_PATH).is_file()
        || !canonical_npm_root
            .join(NPM_PACKAGE_JSON_FILE_NAME)
            .is_file()
    {
        return Err(CommandToolchainProductError::UnsafeNpmRuntime);
    }
    let snapshot = inspect_npm_runtime(&canonical_npm_root)?;
    Ok(VerifiedNpmRuntime {
        root: canonical_npm_root,
        snapshot,
    })
}

fn inspect_npm_runtime(root: &Path) -> Result<NpmRuntimeSnapshot, CommandToolchainProductError> {
    if !root.is_absolute()
        || !root.is_dir()
        || is_filesystem_link(root).map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?
    {
        return Err(CommandToolchainProductError::UnsafeNpmRuntime);
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
    if canonical_root != root {
        return Err(CommandToolchainProductError::UnsafeNpmRuntime);
    }

    let mut pending = vec![canonical_root.clone()];
    let mut files = Vec::new();
    let mut directory_count = 0usize;
    let mut total_bytes = 0u64;
    while let Some(directory) = pending.pop() {
        directory_count = directory_count
            .checked_add(1)
            .ok_or(CommandToolchainProductError::UnsafeNpmRuntime)?;
        if directory_count > MAXIMUM_NPM_RUNTIME_DIRECTORIES {
            return Err(CommandToolchainProductError::UnsafeNpmRuntime);
        }
        let entries =
            fs::read_dir(&directory).map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
        for entry in entries {
            let entry = entry.map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            let path = entry.path();
            if is_filesystem_link(&path)
                .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?
            {
                return Err(CommandToolchainProductError::UnsafeNpmRuntime);
            }
            let file_type = entry
                .file_type()
                .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file() || files.len() >= MAXIMUM_NPM_RUNTIME_FILES {
                return Err(CommandToolchainProductError::UnsafeNpmRuntime);
            }
            let relative = path
                .strip_prefix(&canonical_root)
                .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            let relative = relative
                .to_str()
                .filter(|value| !value.is_empty())
                .ok_or(CommandToolchainProductError::UnsafeNpmRuntime)?
                .replace('\\', "/");
            let metadata = entry
                .metadata()
                .map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            let length = metadata.len();
            if length > MAXIMUM_NPM_RUNTIME_FILE_BYTES {
                return Err(CommandToolchainProductError::UnsafeNpmRuntime);
            }
            total_bytes = total_bytes
                .checked_add(length)
                .filter(|total| *total <= MAXIMUM_NPM_RUNTIME_TOTAL_BYTES)
                .ok_or(CommandToolchainProductError::UnsafeNpmRuntime)?;
            let (_, link_count) =
                file_identity(&path).map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            if link_count != 1 {
                return Err(CommandToolchainProductError::UnsafeNpmRuntime);
            }
            let hash =
                hash_file(&path).map_err(|_| CommandToolchainProductError::UnsafeNpmRuntime)?;
            files.push((relative, length, hash));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty()
        || !files
            .iter()
            .any(|(path, _, _)| path == NPM_CLI_RELATIVE_PATH)
        || !files
            .iter()
            .any(|(path, _, _)| path == NPM_PACKAGE_JSON_FILE_NAME)
    {
        return Err(CommandToolchainProductError::UnsafeNpmRuntime);
    }
    let mut digest = Sha256::new();
    for (relative, length, hash) in &files {
        digest.update((relative.len() as u64).to_le_bytes());
        digest.update(relative.as_bytes());
        digest.update(length.to_le_bytes());
        digest.update(hash);
    }
    Ok(NpmRuntimeSnapshot {
        file_count: files.len(),
        directory_count,
        total_bytes,
        tree_hash: digest.finalize().into(),
    })
}

fn hash_file(path: &Path) -> Result<[u8; 32], CommandToolchainProductError> {
    let mut file =
        File::open(path).map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16 * 1_024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

#[cfg(windows)]
fn file_identity(path: &Path) -> Result<(FileIdentity, u64), CommandToolchainProductError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let file = File::open(path).map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe {
        GetFileInformationByHandle(
            file.as_raw_handle().cast(),
            std::ptr::addr_of_mut!(information),
        )
    } == 0
    {
        return Err(CommandToolchainProductError::UnsafeGitExecutable);
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
fn file_identity(path: &Path) -> Result<(FileIdentity, u64), CommandToolchainProductError> {
    use std::os::unix::fs::MetadataExt;

    let metadata =
        fs::metadata(path).map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    Ok((
        FileIdentity {
            volume: metadata.dev(),
            index: metadata.ino(),
        },
        metadata.nlink(),
    ))
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_path: &Path) -> Result<(FileIdentity, u64), CommandToolchainProductError> {
    Err(CommandToolchainProductError::UnsafeGitExecutable)
}

#[cfg(windows)]
fn is_filesystem_link(path: &Path) -> Result<bool, CommandToolchainProductError> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
    Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_filesystem_link(path: &Path) -> Result<bool, CommandToolchainProductError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| CommandToolchainProductError::UnsafeGitExecutable)?;
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
                "moe-command-toolchain-product-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn git(&self) -> PathBuf {
            self.0
                .join(GIT_DIRECTORY)
                .join(GIT_EXECUTABLE_DIRECTORY)
                .join(GIT_EXECUTABLE_FILE_NAME)
        }

        fn create_git(&self, body: &[u8]) -> PathBuf {
            let git = self.git();
            fs::create_dir_all(git.parent().unwrap()).unwrap();
            fs::write(&git, body).unwrap();
            git
        }

        fn node(&self) -> PathBuf {
            self.0.join(NODE_DIRECTORY).join(NODE_EXECUTABLE_FILE_NAME)
        }

        fn create_node(&self, body: &[u8]) -> PathBuf {
            let node = self.node();
            fs::create_dir_all(node.parent().unwrap()).unwrap();
            fs::write(&node, body).unwrap();
            node
        }

        fn npm_root(&self) -> PathBuf {
            self.0
                .join(NODE_DIRECTORY)
                .join("node_modules")
                .join(NPM_DIRECTORY)
        }

        fn create_npm_runtime(&self) -> PathBuf {
            let root = self.npm_root();
            fs::create_dir_all(root.join("bin")).unwrap();
            fs::create_dir_all(root.join("lib")).unwrap();
            fs::write(root.join(NPM_CLI_RELATIVE_PATH), b"npm cli").unwrap();
            fs::write(
                root.join(NPM_PACKAGE_JSON_FILE_NAME),
                br#"{"name":"npm","version":"11.0.0"}"#,
            )
            .unwrap();
            fs::write(root.join("lib").join("entry.js"), b"module.exports = 1;").unwrap();
            root
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn accepts_only_the_exact_git_for_windows_layout() {
        let root = TestDirectory::new("accept");
        let git = root.create_git(b"git executable");
        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        let verified = product.ready_git().unwrap();
        assert_eq!(verified.path(), git.canonicalize().unwrap());
        verified.revalidate().unwrap();
    }

    #[test]
    fn accepts_only_the_exact_node_for_windows_layout() {
        let root = TestDirectory::new("node-accept");
        let node = root.create_node(b"node executable");
        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        let verified = product.ready_node().unwrap();
        assert_eq!(verified.path(), node.canonicalize().unwrap());
        verified.revalidate().unwrap();
    }

    #[test]
    fn keeps_both_verified_tools_when_both_are_available() {
        let root = TestDirectory::new("both");
        root.create_git(b"git executable");
        root.create_node(b"node executable");
        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        assert!(product.ready_git().is_some());
        assert!(product.ready_node().is_some());
    }

    #[test]
    fn accepts_and_revalidates_a_bounded_link_free_npm_runtime() {
        let root = TestDirectory::new("npm-accept");
        root.create_node(b"node executable");
        let npm = root.create_npm_runtime();
        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        let runtime = product.ready_npm_runtime().unwrap().clone();
        assert_eq!(runtime.root(), npm.canonicalize().unwrap());
        runtime.revalidate().unwrap();

        fs::write(npm.join("lib").join("entry.js"), b"changed").unwrap();
        assert_eq!(
            runtime.revalidate(),
            Err(CommandToolchainProductError::NpmRuntimeChanged)
        );
        assert!(product.ready_node().unwrap().revalidate().is_ok());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a locally installed product Node.js and npm runtime"]
    fn live_product_detects_the_installed_npm_runtime() {
        let product = DesktopCommandToolchainProduct::product();
        assert!(
            product.ready_node().is_some(),
            "installed Node.js was not accepted: {product:?}"
        );
        assert!(
            product.ready_npm_runtime().is_some(),
            "installed npm runtime was not accepted: {product:?}"
        );
    }

    #[test]
    fn an_unsafe_or_incomplete_npm_runtime_does_not_disable_verified_node() {
        let root = TestDirectory::new("npm-rejected");
        root.create_node(b"node executable");
        let npm = root.npm_root();
        fs::create_dir_all(npm.join("bin")).unwrap();
        fs::write(npm.join(NPM_CLI_RELATIVE_PATH), b"npm cli").unwrap();

        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        assert!(product.ready_node().is_some());
        assert!(product.ready_npm_runtime().is_none());
    }

    #[test]
    fn ignores_unregistered_layouts_and_rejects_relative_roots() {
        let root = TestDirectory::new("layout");
        fs::write(root.0.join(GIT_EXECUTABLE_FILE_NAME), b"wrong location").unwrap();
        assert!(matches!(
            DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap(),
            DesktopCommandToolchainProduct::GitUnavailable
        ));
        assert_eq!(
            DesktopCommandToolchainProduct::locate_for_test(Path::new("Program Files")),
            Err(CommandToolchainProductError::UnsafeInstallRoot)
        );
    }

    #[test]
    fn rejects_extra_hard_links_and_replaced_executables() {
        let root = TestDirectory::new("replace");
        let git = root.create_git(b"git executable");
        let extra = root.0.join("extra-git.exe");
        fs::hard_link(&git, &extra).unwrap();
        assert_eq!(
            DesktopCommandToolchainProduct::locate_for_test(&root.0),
            Err(CommandToolchainProductError::UnsafeGitExecutable)
        );
        fs::remove_file(extra).unwrap();
        let product = DesktopCommandToolchainProduct::locate_for_test(&root.0).unwrap();
        let verified = product.ready_git().unwrap().clone();
        fs::remove_file(&git).unwrap();
        fs::write(&git, b"replacement").unwrap();
        assert_eq!(
            verified.revalidate(),
            Err(CommandToolchainProductError::GitExecutableChanged)
        );
    }
}
