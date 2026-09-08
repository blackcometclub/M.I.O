//! Host-owned, read-only Git status boundary for a selected Room workspace.
//!
//! This crate never invokes `git.exe`, a shell, hooks, or project scripts. It
//! accepts only a normal repository whose `.git` directory is inside the
//! selected workspace and asks libgit2 for a non-refreshing status snapshot.

use git2::{
    BranchType, ConfigLevel, DiffFormat, DiffOptions, ErrorCode, Repository, RepositoryOpenFlags,
    Status, StatusOptions,
};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const MAXIMUM_STATUS_ENTRIES: usize = 4_096;
pub const MAXIMUM_STATUS_BYTES: usize = 256 * 1_024;
pub const MAXIMUM_REVIEW_FILES: usize = 128;
pub const MAXIMUM_REVIEW_BYTES: usize = 64 * 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitStatusBrokerError {
    WorkspaceUnavailable,
    UnsupportedRepositoryLayout,
    UnsupportedRepositoryConfiguration,
    RepositoryUnavailable,
    StatusUnavailable,
    NonUtf8Path,
    TooManyEntries,
    OutputTooLarge,
    DiffUnavailable,
    TooManyReviewFiles,
    ReviewOutputTooLarge,
    NonUtf8Diff,
}

impl fmt::Display for GitStatusBrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WorkspaceUnavailable => "the selected workspace is unavailable",
            Self::UnsupportedRepositoryLayout => {
                "only a normal repository with an in-workspace .git directory is supported"
            }
            Self::UnsupportedRepositoryConfiguration => {
                "the repository configuration can access resources outside the workspace boundary"
            }
            Self::RepositoryUnavailable => "the selected workspace repository cannot be opened",
            Self::StatusUnavailable => "the repository status cannot be read",
            Self::NonUtf8Path => "the repository contains a path that cannot be represented safely",
            Self::TooManyEntries => "the repository status contains too many entries",
            Self::OutputTooLarge => "the repository status exceeds the output limit",
            Self::DiffUnavailable => "the repository review diff cannot be read",
            Self::TooManyReviewFiles => "the repository review contains too many changed files",
            Self::ReviewOutputTooLarge => "the repository review diff exceeds the output limit",
            Self::NonUtf8Diff => "the repository review diff is not UTF-8 text",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitReviewSnapshot {
    status: GitStatusSnapshot,
    patch: String,
}

impl GitReviewSnapshot {
    pub fn status(&self) -> &GitStatusSnapshot {
        &self.status
    }

    pub fn patch(&self) -> &str {
        &self.patch
    }
}

impl std::error::Error for GitStatusBrokerError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusSnapshot {
    branch: String,
    entries: Vec<GitStatusEntry>,
}

impl GitStatusSnapshot {
    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn entries(&self) -> &[GitStatusEntry] {
        &self.entries
    }

    pub fn render_porcelain(&self) -> Result<String, GitStatusBrokerError> {
        let mut output = format!("## {}\n", self.branch);
        for entry in &self.entries {
            output.push_str(entry.code());
            output.push(' ');
            output.push_str(&quote_path(entry.path()));
            output.push('\n');
            if output.len() > MAXIMUM_STATUS_BYTES {
                return Err(GitStatusBrokerError::OutputTooLarge);
            }
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusEntry {
    code: &'static str,
    path: String,
}

impl GitStatusEntry {
    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

pub fn read_git_status(workspace_root: &Path) -> Result<GitStatusSnapshot, GitStatusBrokerError> {
    disable_external_git_configuration()?;
    let workspace_root = canonical_workspace(workspace_root)?;
    let git_directory = workspace_root.join(".git");
    let git_metadata = fs::symlink_metadata(&git_directory)
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
    if !git_metadata.is_dir() || git_metadata.file_type().is_symlink() {
        return Err(GitStatusBrokerError::UnsupportedRepositoryLayout);
    }
    let canonical_git = git_directory
        .canonicalize()
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
    if canonical_git.parent() != Some(workspace_root.as_path()) {
        return Err(GitStatusBrokerError::UnsupportedRepositoryLayout);
    }
    validate_repository_control_files(&canonical_git)?;
    validate_local_configuration(&canonical_git.join("config"))?;

    let repository = Repository::open_ext(
        &workspace_root,
        RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&OsStr],
    )
    .map_err(|_| GitStatusBrokerError::RepositoryUnavailable)?;
    validate_repository_paths(&repository, &workspace_root, &canonical_git)?;

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        // Match ordinary porcelain status: report an untracked directory as one
        // entry rather than traversing a possible directory reparse point.
        .recurse_untracked_dirs(false)
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(true)
        .renames_head_to_index(false)
        .renames_index_to_workdir(false)
        .no_refresh(true)
        .update_index(false);
    let statuses = repository
        .statuses(Some(&mut options))
        .map_err(|_| GitStatusBrokerError::StatusUnavailable)?;
    if statuses.len() > MAXIMUM_STATUS_ENTRIES {
        return Err(GitStatusBrokerError::TooManyEntries);
    }
    let mut entries = Vec::with_capacity(statuses.len());
    for entry in statuses.iter() {
        let path = entry
            .path()
            .map_err(|_| GitStatusBrokerError::NonUtf8Path)?
            .to_owned();
        entries.push(GitStatusEntry {
            code: status_code(entry.status()),
            path,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let snapshot = GitStatusSnapshot {
        branch: branch_summary(&repository),
        entries,
    };
    snapshot.render_porcelain()?;
    Ok(snapshot)
}

/// Produce one bounded, read-only review packet for tracked changes.
///
/// This uses the same repository boundary as `read_git_status`, never invokes
/// Git, hooks, filters, or project scripts, and deliberately omits untracked
/// file bodies. Their names remain visible in the status snapshot.
pub fn read_git_review(workspace_root: &Path) -> Result<GitReviewSnapshot, GitStatusBrokerError> {
    let status = read_git_status(workspace_root)?;
    let workspace_root = canonical_workspace(workspace_root)?;
    let git_directory = workspace_root.join(".git");
    let canonical_git = git_directory
        .canonicalize()
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
    validate_repository_control_files(&canonical_git)?;
    validate_local_configuration(&canonical_git.join("config"))?;

    let repository = Repository::open_ext(
        &workspace_root,
        RepositoryOpenFlags::NO_SEARCH,
        &[] as &[&OsStr],
    )
    .map_err(|_| GitStatusBrokerError::RepositoryUnavailable)?;
    validate_repository_paths(&repository, &workspace_root, &canonical_git)?;

    let head_tree = match repository.head() {
        Ok(head) => Some(
            head.peel_to_tree()
                .map_err(|_| GitStatusBrokerError::DiffUnavailable)?,
        ),
        Err(error) if error.code() == ErrorCode::UnbornBranch => None,
        Err(_) => return Err(GitStatusBrokerError::DiffUnavailable),
    };
    let mut options = DiffOptions::new();
    options
        .include_untracked(false)
        .recurse_untracked_dirs(false)
        .include_ignored(false)
        .include_unmodified(false)
        .ignore_submodules(true);
    let diff = repository
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut options))
        .map_err(|_| GitStatusBrokerError::DiffUnavailable)?;
    if diff.deltas().len() > MAXIMUM_REVIEW_FILES {
        return Err(GitStatusBrokerError::TooManyReviewFiles);
    }

    let mut bytes = Vec::new();
    let mut exceeded = false;
    let printed = diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        let include_origin = matches!(origin, '+' | '-' | ' ');
        let required = line.content().len() + usize::from(include_origin);
        if bytes.len().saturating_add(required) > MAXIMUM_REVIEW_BYTES {
            exceeded = true;
            return false;
        }
        if include_origin {
            bytes.push(origin as u8);
        }
        bytes.extend_from_slice(line.content());
        true
    });
    if exceeded {
        return Err(GitStatusBrokerError::ReviewOutputTooLarge);
    }
    printed.map_err(|_| GitStatusBrokerError::DiffUnavailable)?;
    let patch = String::from_utf8(bytes).map_err(|_| GitStatusBrokerError::NonUtf8Diff)?;
    Ok(GitReviewSnapshot { status, patch })
}

fn disable_external_git_configuration() -> Result<(), GitStatusBrokerError> {
    static RESULT: OnceLock<Result<(), ()>> = OnceLock::new();
    RESULT
        .get_or_init(|| {
            // This is the sole git2 consumer in the desktop process. The one-time
            // initialization completes before any repository is opened; later
            // callers wait on OnceLock and never mutate libgit2 global state.
            for level in [
                ConfigLevel::ProgramData,
                ConfigLevel::System,
                ConfigLevel::XDG,
                ConfigLevel::Global,
            ] {
                // SAFETY: synchronized by RESULT and performed before this crate
                // exposes any operation that accesses libgit2 global state.
                unsafe { git2::opts::set_search_path(level, "") }.map_err(|_| ())?;
            }
            Ok(())
        })
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryConfiguration)
}

fn validate_repository_control_files(git_directory: &Path) -> Result<(), GitStatusBrokerError> {
    for relative in [
        "config",
        "HEAD",
        "index",
        "packed-refs",
        "config.worktree",
        "commondir",
        "objects/info/alternates",
        "info/exclude",
    ] {
        let path = git_directory.join(relative);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(GitStatusBrokerError::UnsupportedRepositoryLayout);
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
        if !canonical.starts_with(git_directory) {
            return Err(GitStatusBrokerError::UnsupportedRepositoryLayout);
        }
        if matches!(
            relative,
            "config.worktree" | "commondir" | "objects/info/alternates"
        ) {
            return Err(GitStatusBrokerError::UnsupportedRepositoryConfiguration);
        }
    }
    Ok(())
}

fn validate_local_configuration(config_path: &Path) -> Result<(), GitStatusBrokerError> {
    let contents = fs::read(config_path)
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryConfiguration)?;
    let contents = std::str::from_utf8(&contents)
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryConfiguration)?;
    let mut section = String::new();
    for source_line in contents.lines() {
        let line = source_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            let Some(end) = line.find(']') else {
                return Err(GitStatusBrokerError::UnsupportedRepositoryConfiguration);
            };
            section = line[1..end]
                .split_ascii_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(section.as_str(), "include" | "includeif" | "filter") {
                return Err(GitStatusBrokerError::UnsupportedRepositoryConfiguration);
            }
            continue;
        }
        let key = line
            .split(|character: char| character == '=' || character.is_ascii_whitespace())
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let external_core_key = section == "core"
            && matches!(
                key.as_str(),
                "attributesfile" | "excludesfile" | "fsmonitor" | "hookspath" | "worktree"
            );
        let external_extension_key =
            section == "extensions" && matches!(key.as_str(), "worktreeconfig" | "partialclone");
        if external_core_key || external_extension_key {
            return Err(GitStatusBrokerError::UnsupportedRepositoryConfiguration);
        }
    }
    Ok(())
}

fn canonical_workspace(path: &Path) -> Result<PathBuf, GitStatusBrokerError> {
    if !path.is_absolute() {
        return Err(GitStatusBrokerError::WorkspaceUnavailable);
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| GitStatusBrokerError::WorkspaceUnavailable)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GitStatusBrokerError::WorkspaceUnavailable);
    }
    path.canonicalize()
        .map_err(|_| GitStatusBrokerError::WorkspaceUnavailable)
}

fn validate_repository_paths(
    repository: &Repository,
    workspace_root: &Path,
    git_directory: &Path,
) -> Result<(), GitStatusBrokerError> {
    let workdir = repository
        .workdir()
        .ok_or(GitStatusBrokerError::UnsupportedRepositoryLayout)?
        .canonicalize()
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
    let repository_path = repository
        .path()
        .canonicalize()
        .map_err(|_| GitStatusBrokerError::UnsupportedRepositoryLayout)?;
    if workdir != workspace_root || repository_path != git_directory {
        return Err(GitStatusBrokerError::UnsupportedRepositoryLayout);
    }
    Ok(())
}

fn branch_summary(repository: &Repository) -> String {
    match repository.head() {
        Ok(head) if head.is_branch() => {
            let name = head.shorthand().unwrap_or("HEAD").to_owned();
            let Ok(local) = repository.find_branch(&name, BranchType::Local) else {
                return name;
            };
            let Ok(upstream) = local.upstream() else {
                return name;
            };
            let Ok(Some(upstream_name)) = upstream.name() else {
                return name;
            };
            let Some(local_target) = local.get().target() else {
                return format!("{name}...{upstream_name}");
            };
            let Some(upstream_target) = upstream.get().target() else {
                return format!("{name}...{upstream_name}");
            };
            match repository.graph_ahead_behind(local_target, upstream_target) {
                Ok((0, 0)) => format!("{name}...{upstream_name}"),
                Ok((ahead, 0)) => format!("{name}...{upstream_name} [ahead {ahead}]"),
                Ok((0, behind)) => format!("{name}...{upstream_name} [behind {behind}]"),
                Ok((ahead, behind)) => {
                    format!("{name}...{upstream_name} [ahead {ahead}, behind {behind}]")
                }
                Err(_) => format!("{name}...{upstream_name}"),
            }
        }
        Ok(_) => "HEAD (detached)".to_owned(),
        Err(error) if error.code() == ErrorCode::UnbornBranch => repository
            .find_reference("HEAD")
            .ok()
            .and_then(|head| head.symbolic_target().ok().flatten().map(str::to_owned))
            .and_then(|target| target.strip_prefix("refs/heads/").map(str::to_owned))
            .map(|name| format!("No commits yet on {name}"))
            .unwrap_or_else(|| "No commits yet".to_owned()),
        Err(_) => "HEAD unavailable".to_owned(),
    }
}

fn status_code(status: Status) -> &'static str {
    if status.is_conflicted() {
        return "UU";
    }
    if status == Status::WT_NEW {
        return "??";
    }
    let index = if status.is_index_new() {
        'A'
    } else if status.is_index_modified() {
        'M'
    } else if status.is_index_deleted() {
        'D'
    } else if status.is_index_renamed() {
        'R'
    } else if status.is_index_typechange() {
        'T'
    } else {
        ' '
    };
    let worktree = if status.is_wt_modified() {
        'M'
    } else if status.is_wt_deleted() {
        'D'
    } else if status.is_wt_renamed() {
        'R'
    } else if status.is_wt_typechange() {
        'T'
    } else if status.contains(Status::WT_UNREADABLE) {
        '!'
    } else if status.is_wt_new() {
        '?'
    } else {
        ' '
    };
    match (index, worktree) {
        ('A', ' ') => "A ",
        ('M', ' ') => "M ",
        ('D', ' ') => "D ",
        ('R', ' ') => "R ",
        ('T', ' ') => "T ",
        (' ', 'M') => " M",
        (' ', 'D') => " D",
        (' ', 'R') => " R",
        (' ', 'T') => " T",
        (' ', '!') => " !",
        (' ', '?') => "??",
        ('M', 'M') => "MM",
        ('M', 'D') => "MD",
        ('A', 'M') => "AM",
        ('A', 'D') => "AD",
        ('D', 'M') => "DM",
        _ => "!!",
    }
}

fn quote_path(path: &str) -> String {
    if path
        .chars()
        .any(|character| character.is_control() || matches!(character, '"' | '\\'))
    {
        format!("{path:?}")
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let parent = std::env::current_dir()
                .unwrap()
                .join("target")
                .join("moe-git-status-broker-tests");
            fs::create_dir_all(&parent).unwrap();
            let root = parent.join(format!(
                "moe-git-status-broker-{label}-{}-{}",
                std::process::id(),
                TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reads_untracked_files_without_starting_an_external_git_process() {
        let fixture = Fixture::new("untracked");
        let mut init = git2::RepositoryInitOptions::new();
        init.initial_head("main");
        Repository::init_opts(&fixture.root, &init).unwrap();
        fs::create_dir_all(fixture.root.join("nested")).unwrap();
        fs::write(fixture.root.join("nested").join("note.txt"), "hello\n").unwrap();

        let snapshot = read_git_status(&fixture.root).unwrap();

        assert_eq!(snapshot.branch(), "No commits yet on main");
        assert_eq!(
            snapshot.entries(),
            &[GitStatusEntry {
                code: "??",
                path: "nested/".to_owned(),
            }]
        );
        assert!(snapshot.render_porcelain().unwrap().contains("?? nested/"));
    }

    #[test]
    fn renders_only_bounded_tracked_changes_for_review() {
        let fixture = Fixture::new("review");
        let repository = Repository::init(&fixture.root).unwrap();
        fs::write(fixture.root.join("tracked.rs"), "fn value() -> u8 { 1 }\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("tracked.rs")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repository.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("M.I.O. test", "mio@example.invalid").unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
            .unwrap();
        drop(tree);

        fs::write(fixture.root.join("tracked.rs"), "fn value() -> u8 { 2 }\n").unwrap();
        fs::write(fixture.root.join("untracked-secret.txt"), "do-not-send\n").unwrap();

        let review = read_git_review(&fixture.root).unwrap();
        assert!(
            review
                .status()
                .render_porcelain()
                .unwrap()
                .contains("tracked.rs")
        );
        assert!(review.patch().contains("fn value() -> u8 { 2 }"));
        assert!(!review.patch().contains("do-not-send"));
        assert!(review.patch().len() <= MAXIMUM_REVIEW_BYTES);
    }

    #[test]
    fn rejects_a_gitfile_that_can_point_outside_the_workspace() {
        let fixture = Fixture::new("gitfile");
        fs::write(fixture.root.join(".git"), "gitdir: C:/outside\n").unwrap();
        assert_eq!(
            read_git_status(&fixture.root),
            Err(GitStatusBrokerError::UnsupportedRepositoryLayout)
        );
    }

    #[test]
    fn rejects_local_configuration_that_can_read_outside_the_workspace() {
        for dangerous in [
            "\n[include]\npath = C:/outside/gitconfig\n",
            "\n[includeIf \"gitdir:C:/outside/\"]\npath = C:/outside/gitconfig\n",
            "\n[core]\nexcludesFile = C:/outside/ignore\n",
            "\n[core]\nattributesFile = C:/outside/attributes\n",
            "\n[core]\nfsmonitor = C:/outside/monitor.exe\n",
            "\n[filter \"unsafe\"]\nclean = C:/outside/filter.exe\n",
        ] {
            let fixture = Fixture::new("external-config");
            Repository::init(&fixture.root).unwrap();
            let config = fixture.root.join(".git").join("config");
            let mut contents = fs::read_to_string(&config).unwrap();
            contents.push_str(dangerous);
            fs::write(config, contents).unwrap();
            assert_eq!(
                read_git_status(&fixture.root),
                Err(GitStatusBrokerError::UnsupportedRepositoryConfiguration)
            );
        }
    }

    #[test]
    fn reports_an_untracked_directory_without_descending_into_it() {
        let fixture = Fixture::new("untracked-directory");
        Repository::init(&fixture.root).unwrap();
        fs::create_dir_all(fixture.root.join("nested").join("deep")).unwrap();
        fs::write(
            fixture.root.join("nested").join("deep").join("note.txt"),
            "hello\n",
        )
        .unwrap();

        let snapshot = read_git_status(&fixture.root).unwrap();

        assert_eq!(snapshot.entries().len(), 1);
        assert!(snapshot.entries()[0].path().starts_with("nested"));
        assert!(!snapshot.entries()[0].path().contains("note.txt"));
    }

    #[test]
    #[ignore = "reads an explicitly configured real workspace without refreshing its Git index"]
    fn live_real_workspace_status_does_not_change_the_index() {
        let workspace = std::env::var_os("MOE_TEST_REAL_WORKSPACE")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .expect("MOE_TEST_REAL_WORKSPACE must name an existing absolute directory");
        let index = workspace.join(".git").join("index");
        let before = fs::read(&index).unwrap();

        let started = std::time::Instant::now();
        let snapshot = read_git_status(&workspace).unwrap();
        let elapsed = started.elapsed();

        assert_eq!(fs::read(&index).unwrap(), before);
        assert!(snapshot.branch().starts_with("main"));
        assert!(elapsed < std::time::Duration::from_secs(30));
        eprintln!(
            "host-owned Git status completed in {elapsed:?}\n{}",
            snapshot.render_porcelain().unwrap()
        );
    }
}
