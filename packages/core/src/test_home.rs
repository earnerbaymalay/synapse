//! Test-only serialization for the process-global `HOME` variable.
//!
//! Several adapters deliberately consult `$HOME` as well as the scanned root,
//! because a user-level install counts as "present" (see e.g.
//! [`crate::adapters::zed`]). `std::env::set_var` mutates the whole process,
//! so a test that repoints `HOME` changes what every *other* test sees — and
//! cargo runs a crate's tests on parallel threads in one process. That made
//! the suite nondeterministic: whichever HOME-reading test happened to
//! overlap a HOME-writing one failed, and the failure moved between adapters
//! from run to run.
//!
//! [`HomeGuard`] fixes that by making `HOME` a locked resource. Acquiring one
//! blocks until every other guard is dropped, and dropping it restores the
//! previous value, so tests that touch `HOME` are serialized against each
//! other and leave the environment as they found it.

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

static HOME_LOCK: Mutex<()> = Mutex::new(());

/// Exclusive access to `$HOME` for the lifetime of the guard.
pub(crate) struct HomeGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

impl HomeGuard {
    /// Takes the lock and points `HOME` at `path`.
    pub(crate) fn set(path: &Path) -> Self {
        // A panicking test poisons the lock; the environment is still safe to
        // take over, so recover rather than cascading one failure into every
        // subsequent HOME-dependent test.
        let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var_os("HOME");
        std::env::set_var("HOME", path);
        Self {
            _lock: lock,
            previous,
        }
    }

    /// Repoints `HOME` without releasing the lock — for a test that imports
    /// from one home and projects into another.
    pub(crate) fn repoint(&self, path: &Path) {
        std::env::set_var("HOME", path);
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn guard_restores_the_previous_home() {
        let before = std::env::var_os("HOME");
        let dir = tempdir().unwrap();
        {
            let _guard = HomeGuard::set(dir.path());
            assert_eq!(
                std::env::var_os("HOME"),
                Some(dir.path().as_os_str().into())
            );
        }
        assert_eq!(std::env::var_os("HOME"), before);
    }

    #[test]
    fn repoint_moves_home_without_losing_the_restore() {
        let before = std::env::var_os("HOME");
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        {
            let guard = HomeGuard::set(first.path());
            guard.repoint(second.path());
            assert_eq!(
                std::env::var_os("HOME"),
                Some(second.path().as_os_str().into()),
            );
        }
        assert_eq!(std::env::var_os("HOME"), before);
    }
}
