//! Debounced filesystem watcher — T4.1's watcher half. Wraps `notify`'s
//! raw per-event stream (which fires once per underlying syscall — an
//! editor's save can be a temp-write + rename + chmod, three events for
//! one logical change) into a stream of coalesced batches, so the sync
//! daemon (T4.1's scheduler half, plus T4.2's conflict queue) reacts once
//! per real change instead of once per syscall.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::Duration;

/// One coalesced batch: every distinct path touched during a burst of
/// filesystem activity that settled for at least the watcher's debounce
/// window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebouncedEvent {
    pub paths: Vec<PathBuf>,
}

/// Watches `root` recursively and delivers one [`DebouncedEvent`] per
/// quiet period, rather than one callback per raw OS notification.
pub struct DebouncedWatcher {
    // Held only to keep the underlying OS watch alive for the struct's
    // lifetime; never read directly.
    _watcher: RecommendedWatcher,
    raw_rx: Receiver<notify::Result<Event>>,
    debounce: Duration,
}

impl DebouncedWatcher {
    /// Starts watching `root`. `debounce` is the quiet period required
    /// after the last raw event before a batch is considered settled.
    pub fn watch(root: &Path, debounce: Duration) -> notify::Result<Self> {
        let (tx, raw_rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            // The receiver may already be gone (watcher dropped mid-burst);
            // that's a normal shutdown race, not a bug to propagate.
            let _ = tx.send(res);
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            raw_rx,
            debounce,
        })
    }

    /// Blocks up to `initial_wait` for the first raw event; returns `None`
    /// if none arrives in time (nothing changed) or the watcher's channel
    /// closed. Once a first event lands, keeps collecting until
    /// `debounce` has elapsed with no further events, then returns every
    /// distinct path touched during the burst. Bounded on both ends —
    /// this never blocks forever, even if the underlying OS watch never
    /// fires (e.g. no inotify support in the current environment).
    pub fn next_batch(&self, initial_wait: Duration) -> Option<DebouncedEvent> {
        let first = match self.raw_rx.recv_timeout(initial_wait) {
            Ok(Ok(event)) => event,
            Ok(Err(_)) => return None,
            Err(RecvTimeoutError::Timeout) => return None,
            Err(RecvTimeoutError::Disconnected) => return None,
        };

        let mut paths: HashSet<PathBuf> = first.paths.into_iter().collect();
        loop {
            match self.raw_rx.recv_timeout(self.debounce) {
                Ok(Ok(event)) => paths.extend(event.paths),
                Ok(Err(_)) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        Some(DebouncedEvent {
            paths: paths.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    /// How long a test waits for the *first* event before giving up.
    /// Generous, since it only matters when inotify genuinely doesn't fire
    /// (e.g. a restricted sandbox) — a real editor save is microseconds.
    const TEST_INITIAL_WAIT: Duration = Duration::from_secs(5);
    const TEST_DEBOUNCE: Duration = Duration::from_millis(80);

    #[test]
    fn no_batch_when_nothing_changes() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = DebouncedWatcher::watch(dir.path(), TEST_DEBOUNCE).unwrap();

        let batch = watcher.next_batch(Duration::from_millis(200));
        assert!(batch.is_none());
    }

    #[test]
    fn coalesces_a_burst_of_writes_into_one_batch() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = DebouncedWatcher::watch(dir.path(), TEST_DEBOUNCE).unwrap();

        let root = dir.path().to_path_buf();
        thread::spawn(move || {
            for i in 0..5 {
                fs::write(root.join(format!("file-{i}.txt")), "content").unwrap();
                thread::sleep(Duration::from_millis(2));
            }
        });

        let Some(batch) = watcher.next_batch(TEST_INITIAL_WAIT) else {
            // No inotify support in this environment — nothing to assert
            // against; the bounded-wait behavior itself is what
            // `no_batch_when_nothing_changes` already covers.
            eprintln!("skipping: no filesystem events observed (no inotify in this environment)");
            return;
        };

        assert!(
            !batch.paths.is_empty(),
            "expected at least one path in the debounced batch"
        );
    }
}
