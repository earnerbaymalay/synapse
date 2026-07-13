//! OS scheduler registration — T4.1's other half. The sync daemon needs
//! the OS to relaunch it (on login, on a timer, across reboots) rather
//! than relying on a shell staying open forever. Each OS's native
//! mechanism is different — launchd on macOS, systemd user timers on
//! Linux, Task Scheduler on Windows — so this module renders the
//! OS-specific config for a [`ScheduledJob`] and can write it to disk.
//!
//! What this module deliberately does NOT do: call `launchctl load`,
//! `systemctl --user enable --now`, or `schtasks /create` against the
//! real machine. Those commands register a background service that
//! persists across reboots — a system-wide, hard-to-reverse action — and
//! are out of scope for anything but an explicit, human-approved install
//! step (mirrors T3.4's Gate 2: dry-run/generate first, a human approves
//! before anything is actually registered with the OS).

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Which OS's native scheduler a job is being generated for. Parameterized
/// rather than branching on `cfg(target_os)` so every OS's output can be
/// generated and unit-tested from a single test binary, regardless of
/// which OS actually runs the tests — same pattern as
/// `crate::projector`'s per-tool policy table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerOs {
    MacOs,
    Linux,
    Windows,
}

/// A recurring job the sync daemon needs the OS to relaunch: run `command`
/// (with `args`) every `interval`, kept alive across logins/reboots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledJob {
    /// Reverse-DNS-style identifier, e.g. `"com.llmneurosurgeon.sync"`.
    /// Used as the launchd label, the systemd unit name, and the Windows
    /// task name.
    pub label: String,
    pub command: PathBuf,
    pub args: Vec<String>,
    pub interval: Duration,
}

/// The native config content for one [`SchedulerOs`]. Windows has no
/// declarative unit-file format the way launchd/systemd do — the
/// idiomatic "install mechanism" *is* the `schtasks` command line, so its
/// variant carries a command rather than file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedJob {
    Plist(String),
    SystemdUnit { service: String, timer: String },
    SchTasksCommand(String),
}

/// Renders `job` as `os`'s native scheduler config.
pub fn render(job: &ScheduledJob, os: SchedulerOs) -> RenderedJob {
    match os {
        SchedulerOs::MacOs => RenderedJob::Plist(render_plist(job)),
        SchedulerOs::Linux => {
            let (service, timer) = render_systemd_unit(job);
            RenderedJob::SystemdUnit { service, timer }
        }
        SchedulerOs::Windows => RenderedJob::SchTasksCommand(render_schtasks_command(job)),
    }
}

fn render_plist(job: &ScheduledJob) -> String {
    let args_xml: String = std::iter::once(job.command.display().to_string())
        .chain(job.args.iter().cloned())
        .map(|a| format!("        <string>{}</string>\n", xml_escape(&a)))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
{args_xml}    </array>
    <key>StartInterval</key>
    <integer>{interval}</integer>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        label = xml_escape(&job.label),
        args_xml = args_xml,
        interval = job.interval.as_secs(),
    )
}

fn render_systemd_unit(job: &ScheduledJob) -> (String, String) {
    let exec_start = std::iter::once(job.command.display().to_string())
        .chain(job.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");

    let service = format!(
        "[Unit]\nDescription={label}\n\n[Service]\nType=oneshot\nExecStart={exec_start}\n",
        label = job.label,
        exec_start = exec_start,
    );

    let timer = format!(
        "[Unit]\nDescription={label} timer\n\n[Timer]\nOnBootSec={interval}s\nOnUnitActiveSec={interval}s\n\n[Install]\nWantedBy=timers.target\n",
        label = job.label,
        interval = job.interval.as_secs(),
    );

    (service, timer)
}

fn render_schtasks_command(job: &ScheduledJob) -> String {
    let full_command = std::iter::once(job.command.display().to_string())
        .chain(job.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    let minutes = (job.interval.as_secs() / 60).max(1);
    format!(
        "schtasks /create /tn \"{label}\" /tr \"{command}\" /sc minute /mo {minutes} /f",
        label = job.label,
        command = full_command,
        minutes = minutes,
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Where `render(job, os)`'s output would be installed on a real machine,
/// relative to `home` (the user's home directory). Takes `home` as a
/// parameter — rather than reading `$HOME`/`%APPDATA%` itself — so tests
/// can point it at a tempdir instead of the real machine; the real
/// default lives in [`default_home`].
pub fn install_paths(job: &ScheduledJob, os: SchedulerOs, home: &Path) -> Vec<PathBuf> {
    match os {
        SchedulerOs::MacOs => vec![home
            .join("Library/LaunchAgents")
            .join(format!("{}.plist", job.label))],
        SchedulerOs::Linux => {
            let dir = home.join(".config/systemd/user");
            vec![
                dir.join(format!("{}.service", job.label)),
                dir.join(format!("{}.timer", job.label)),
            ]
        }
        // Windows Task Scheduler has no on-disk unit file to place; the
        // `schtasks` command itself is the "install path" — there is
        // nothing to write.
        SchedulerOs::Windows => vec![],
    }
}

/// The real home directory on this machine, for callers building the real
/// (non-test) install path. Returns `None` if it can't be determined
/// (matches `crate::adapters::get_home_dir`'s fallback behavior).
pub fn default_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[derive(Debug)]
pub enum SchedulerError {
    Io(String),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for SchedulerError {}

/// Writes `render(job, os)`'s content to `install_paths(job, os, home)`,
/// creating parent directories as needed. Returns the paths written.
///
/// This only writes files — it does NOT run `launchctl load`,
/// `systemctl --user enable --now`, or the `schtasks` command, so nothing
/// is actually registered with the OS's scheduler yet. Activating a
/// written job is a separate, explicit step outside this function on
/// purpose: registering a persistent background service is a system-wide
/// action a human should approve, not something an automated pass does
/// silently.
pub fn write_job_files(
    job: &ScheduledJob,
    os: SchedulerOs,
    home: &Path,
) -> Result<Vec<PathBuf>, SchedulerError> {
    let rendered = render(job, os);
    let paths = install_paths(job, os, home);

    match rendered {
        RenderedJob::Plist(content) => {
            let path = paths
                .first()
                .expect("install_paths always returns one entry for MacOs");
            write_with_parents(path, &content)?;
            Ok(vec![path.clone()])
        }
        RenderedJob::SystemdUnit { service, timer } => {
            let service_path = &paths[0];
            let timer_path = &paths[1];
            write_with_parents(service_path, &service)?;
            write_with_parents(timer_path, &timer)?;
            Ok(vec![service_path.clone(), timer_path.clone()])
        }
        RenderedJob::SchTasksCommand(_) => {
            // Nothing to write: see `install_paths`'s Windows branch.
            Ok(vec![])
        }
    }
}

fn write_with_parents(path: &Path, content: &str) -> Result<(), SchedulerError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            SchedulerError::Io(format!("failed to create {}: {}", parent.display(), e))
        })?;
    }
    std::fs::write(path, content)
        .map_err(|e| SchedulerError::Io(format!("failed to write {}: {}", path.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_job() -> ScheduledJob {
        ScheduledJob {
            label: "com.llmneurosurgeon.sync".to_string(),
            command: PathBuf::from("/usr/local/bin/neurosurgeon"),
            args: vec!["sync".to_string(), "--once".to_string()],
            interval: Duration::from_secs(300),
        }
    }

    #[test]
    fn renders_a_valid_looking_plist_for_macos() {
        let content = match render(&sample_job(), SchedulerOs::MacOs) {
            RenderedJob::Plist(c) => c,
            other => panic!("expected Plist, got {other:?}"),
        };
        assert!(content.contains("<key>Label</key>"));
        assert!(content.contains("com.llmneurosurgeon.sync"));
        assert!(content.contains("<integer>300</integer>"));
        assert!(content.contains("sync"));
        assert!(content.contains("--once"));
    }

    #[test]
    fn renders_a_systemd_service_and_timer_for_linux() {
        let (service, timer) = match render(&sample_job(), SchedulerOs::Linux) {
            RenderedJob::SystemdUnit { service, timer } => (service, timer),
            other => panic!("expected SystemdUnit, got {other:?}"),
        };
        assert!(service.contains("ExecStart=/usr/local/bin/neurosurgeon sync --once"));
        assert!(timer.contains("OnUnitActiveSec=300s"));
        assert!(timer.contains("WantedBy=timers.target"));
    }

    #[test]
    fn renders_an_schtasks_command_for_windows() {
        let command = match render(&sample_job(), SchedulerOs::Windows) {
            RenderedJob::SchTasksCommand(c) => c,
            other => panic!("expected SchTasksCommand, got {other:?}"),
        };
        assert!(command.starts_with("schtasks /create"));
        assert!(command.contains("com.llmneurosurgeon.sync"));
        assert!(command.contains("/sc minute /mo 5"));
    }

    #[test]
    fn xml_escapes_special_characters_in_the_plist() {
        let mut job = sample_job();
        job.label = "com.test & <co>".to_string();
        let content = match render(&job, SchedulerOs::MacOs) {
            RenderedJob::Plist(c) => c,
            other => panic!("expected Plist, got {other:?}"),
        };
        assert!(content.contains("com.test &amp; &lt;co&gt;"));
        assert!(!content.contains("com.test & <co>"));
    }

    #[test]
    fn write_job_files_writes_plist_under_home_on_macos() {
        let home = tempfile::tempdir().unwrap();
        let job = sample_job();

        let written = write_job_files(&job, SchedulerOs::MacOs, home.path()).unwrap();

        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("Library/LaunchAgents/com.llmneurosurgeon.sync.plist"));
        assert!(written[0].exists());
        let content = std::fs::read_to_string(&written[0]).unwrap();
        assert!(content.contains("<key>Label</key>"));
    }

    #[test]
    fn write_job_files_writes_service_and_timer_on_linux() {
        let home = tempfile::tempdir().unwrap();
        let job = sample_job();

        let written = write_job_files(&job, SchedulerOs::Linux, home.path()).unwrap();

        assert_eq!(written.len(), 2);
        for path in &written {
            assert!(path.exists());
        }
        assert!(written[0].to_string_lossy().ends_with(".service"));
        assert!(written[1].to_string_lossy().ends_with(".timer"));
    }

    #[test]
    fn write_job_files_writes_nothing_on_windows() {
        let home = tempfile::tempdir().unwrap();
        let job = sample_job();

        let written = write_job_files(&job, SchedulerOs::Windows, home.path()).unwrap();

        assert!(written.is_empty());
    }
}
