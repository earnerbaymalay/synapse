//! Git-backed snapshot/rollback — T4.3. Per MASTER_PROMPT.md's "Time
//! Machine": every sync is a git commit against the Brain directory, and
//! rolling back is itself a new commit that restores an old tree — never
//! a destructive rewrite of history. Also provides the crash-safe lock
//! that guards a snapshot/rollback in progress.
//!
//! Shells out to the real `git` binary via `std::process::Command` rather
//! than a libgit2 binding (`git2`) — this project's Tauri build already
//! hit missing-system-library failures (`libsoup`/`javascriptcoregtk`) in
//! sandboxed environments, and `git` is guaranteed present (this whole
//! project is developed inside a git repo).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum SnapshotError {
    Io(String),
    Command(String),
    Locked(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Io(msg) => write!(f, "io error: {msg}"),
            SnapshotError::Command(msg) => write!(f, "git command failed: {msg}"),
            SnapshotError::Locked(msg) => write!(f, "locked: {msg}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, SnapshotError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| SnapshotError::Io(format!("failed to run git {}: {}", args.join(" "), e)))?;
    if !output.status.success() {
        return Err(SnapshotError::Command(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Ensures `brain_root` is a git repository. Idempotent — calling this on
/// an already-initialized repo is a safe no-op. Configures a local commit
/// identity so `snapshot` works even when no global `user.name`/
/// `user.email` is set (common in fresh sandboxes/CI).
pub fn ensure_repo(brain_root: &Path) -> Result<(), SnapshotError> {
    fs::create_dir_all(brain_root).map_err(|e| {
        SnapshotError::Io(format!("failed to create {}: {}", brain_root.display(), e))
    })?;
    if !brain_root.join(".git").is_dir() {
        run_git(brain_root, &["init", "-q"])?;
        run_git(
            brain_root,
            &["config", "user.email", "brain@llm-neurosurgeon.local"],
        )?;
        run_git(
            brain_root,
            &["config", "user.name", "LLM Neurosurgeon Brain"],
        )?;
    }
    Ok(())
}

fn current_head(brain_root: &Path) -> Result<String, SnapshotError> {
    run_git(brain_root, &["rev-parse", "HEAD"])
}

/// Records a snapshot: stages every change under `brain_root` and commits
/// it. Returns the resulting commit's full sha. If nothing changed since
/// the last snapshot, returns the existing `HEAD` instead of creating an
/// empty commit — snapshotting a quiescent Brain must not spam the log.
pub fn snapshot(brain_root: &Path, message: &str) -> Result<String, SnapshotError> {
    ensure_repo(brain_root)?;
    run_git(brain_root, &["add", "-A"])?;

    let status = run_git(brain_root, &["status", "--porcelain"])?;
    if status.is_empty() {
        return current_head(brain_root);
    }

    run_git(brain_root, &["commit", "-q", "-m", message])?;
    current_head(brain_root)
}

/// Restores `brain_root`'s working tree to be byte-identical to `commit`
/// (any revision `git rev-parse` accepts — a full sha, a short sha, a
/// branch/tag name). Never rewrites history: this is itself recorded as a
/// new commit on top of whatever came after `commit`, so "undo the
/// rollback" is just another rollback to the commit that preceded it —
/// the Time Machine model from MASTER_PROMPT.md. Returns the new
/// rollback commit's full sha.
pub fn rollback(brain_root: &Path, commit: &str) -> Result<String, SnapshotError> {
    ensure_repo(brain_root)?;

    // Resolve up front so the recorded message and file-list diff are
    // pinned to one exact commit even if `commit` was a relative ref.
    let target = run_git(brain_root, &["rev-parse", commit])?;

    // Restores every path present in the target commit's tree.
    run_git(brain_root, &["checkout", &target, "--", "."])?;

    // `checkout -- .` only restores paths that exist in the target
    // commit; paths that exist now but did not exist at that commit must
    // be deleted for the working tree to actually match it byte-for-byte.
    let target_files: HashSet<String> = list_files_at(brain_root, &target)?;
    let current_files: HashSet<String> = run_git(brain_root, &["ls-files"])?
        .lines()
        .map(|s| s.to_string())
        .collect();

    for extra in current_files.difference(&target_files) {
        let path = brain_root.join(extra);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                SnapshotError::Io(format!("failed to remove {}: {}", path.display(), e))
            })?;
        }
    }

    let short = &target[..target.len().min(12)];
    snapshot(brain_root, &format!("rollback to {short}"))
}

fn list_files_at(brain_root: &Path, revision: &str) -> Result<HashSet<String>, SnapshotError> {
    Ok(
        run_git(brain_root, &["ls-tree", "-r", "--name-only", revision])?
            .lines()
            .map(|s| s.to_string())
            .collect(),
    )
}

/// A crash-safe advisory lock guarding one snapshot/rollback at a time.
/// Held by writing the current process's PID to `path`; released by
/// deleting `path` when the guard drops. If a prior lock-holder crashed
/// mid-operation without cleaning up, `acquire` detects that the recorded
/// PID is no longer running and reclaims the lock rather than blocking
/// forever on a dead process.
pub struct SnapshotLock {
    path: PathBuf,
}

impl SnapshotLock {
    pub fn acquire(path: &Path) -> Result<SnapshotLock, SnapshotError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SnapshotError::Io(format!("failed to create {}: {}", parent.display(), e))
            })?;
        }

        match Self::try_create(path) {
            Ok(()) => Ok(SnapshotLock {
                path: path.to_path_buf(),
            }),
            Err(_) if path.exists() => {
                if let Some(pid) = Self::read_pid(path) {
                    if is_process_alive(pid) {
                        return Err(SnapshotError::Locked(format!(
                            "already locked by running process {pid}"
                        )));
                    }
                }
                // Stale lock (holder's PID is no longer running, or the
                // lock file was unreadable/corrupt) — reclaim it.
                fs::remove_file(path)
                    .map_err(|e| SnapshotError::Io(format!("failed to remove stale lock: {e}")))?;
                Self::try_create(path)
                    .map_err(|e| SnapshotError::Io(format!("failed to reclaim lock: {e}")))?;
                Ok(SnapshotLock {
                    path: path.to_path_buf(),
                })
            }
            Err(e) => Err(SnapshotError::Io(format!("failed to acquire lock: {e}"))),
        }
    }

    fn try_create(path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        write!(f, "{}", std::process::id())
    }

    fn read_pid(path: &Path) -> Option<u32> {
        fs::read_to_string(path).ok()?.trim().parse().ok()
    }
}

impl Drop for SnapshotLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    // No cheap liveness check without an extra dependency; assume alive
    // so a live lock on a non-Unix OS is never silently stolen. Means a
    // truly-crashed lock only self-heals on Unix for now.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;

    fn hash_tree(root: &Path) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let mut queue = vec![root.to_path_buf()];
        while let Some(dir) = queue.pop() {
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                    continue;
                }
                if path.is_dir() {
                    queue.push(path);
                } else {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    let content = fs::read(&path).unwrap();
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    out.insert(rel, hex::encode(hasher.finalize()));
                }
            }
        }
        out
    }

    #[test]
    fn ensure_repo_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        ensure_repo(dir.path()).unwrap();
        ensure_repo(dir.path()).unwrap();
        assert!(dir.path().join(".git").is_dir());
    }

    #[test]
    fn snapshot_commits_changes_and_returns_a_full_sha() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("skill.yaml"), "id: repo-conventions").unwrap();

        let sha = snapshot(dir.path(), "initial import").unwrap();

        assert_eq!(sha.len(), 40);
        let log = run_git(dir.path(), &["log", "--oneline"]).unwrap();
        assert!(log.contains("initial import"));
    }

    #[test]
    fn snapshot_is_a_noop_when_nothing_changed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("skill.yaml"), "id: repo-conventions").unwrap();

        let first = snapshot(dir.path(), "initial import").unwrap();
        let second = snapshot(dir.path(), "should not create a new commit").unwrap();

        assert_eq!(first, second);
        let log = run_git(dir.path(), &["log", "--oneline"]).unwrap();
        assert_eq!(log.lines().count(), 1);
    }

    #[test]
    fn rollback_restores_working_tree_byte_identical_and_preserves_history() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join("a.yaml"), "version: 1").unwrap();
        fs::write(dir.path().join("b.yaml"), "version: 1").unwrap();
        let snapshot_one = snapshot(dir.path(), "snapshot one").unwrap();
        let tree_at_one = hash_tree(dir.path());

        // Diverge: edit a.yaml, delete b.yaml, add c.yaml.
        fs::write(dir.path().join("a.yaml"), "version: 2").unwrap();
        fs::remove_file(dir.path().join("b.yaml")).unwrap();
        fs::write(dir.path().join("c.yaml"), "new file").unwrap();
        snapshot(dir.path(), "snapshot two").unwrap();

        let rollback_sha = rollback(dir.path(), &snapshot_one).unwrap();

        assert_eq!(
            hash_tree(dir.path()),
            tree_at_one,
            "working tree must be byte-identical to snapshot one after rollback"
        );
        assert!(!dir.path().join("c.yaml").exists());

        // History is preserved, not rewritten: 3 commits (one, two, and
        // the rollback itself), not 1.
        let log = run_git(dir.path(), &["log", "--oneline"]).unwrap();
        assert_eq!(log.lines().count(), 3);
        assert_ne!(rollback_sha, snapshot_one);
    }

    #[test]
    fn lock_prevents_concurrent_acquire_while_held() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join(".lock");

        let _held = SnapshotLock::acquire(&lock_path).unwrap();
        let second = SnapshotLock::acquire(&lock_path);

        assert!(matches!(second, Err(SnapshotError::Locked(_))));
    }

    #[test]
    fn lock_releases_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join(".lock");

        {
            let _held = SnapshotLock::acquire(&lock_path).unwrap();
            assert!(lock_path.exists());
        }

        assert!(!lock_path.exists());
        // Acquiring again after the guard dropped must succeed cleanly.
        let _reacquired = SnapshotLock::acquire(&lock_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn lock_reclaims_a_stale_lock_left_by_a_dead_process() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join(".lock");

        // A PID that is essentially guaranteed not to be running: PID 1
        // exists (init), so pick something absurdly high instead — Linux
        // PIDs don't reach this by default (pid_max is typically <= 4M).
        fs::write(&lock_path, "999999999").unwrap();

        // Should reclaim the stale lock rather than error.
        let guard = SnapshotLock::acquire(&lock_path).unwrap();
        drop(guard);
    }
}
