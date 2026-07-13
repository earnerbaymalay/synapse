//! The onboarding wizard's dry-run step (T5.2). Mirrors `apps/cli`'s
//! `report_scan`/`report_import_dry_run` exactly — same `detect()` +
//! `import()` calls against `packages/core`'s adapters, same "nothing is
//! written" guarantee — just returning structured data for the GUI
//! instead of printing a text report. Real (non-dry-run) import is not
//! implemented here for the same reason it isn't in `apps/cli`: no
//! Brain-write path exists in `packages/core` yet (Phase 4+ scope).

use neurosurgeon_core::adapters::all_adapters;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTool {
    pub tool_id: String,
    pub skills: usize,
    pub agents: usize,
    pub mcp_servers: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunReport {
    pub root: String,
    pub detected: Vec<DetectedTool>,
}

/// Detects every supported AI tool's config under the current directory
/// and reports what `import()` would bring in. Never writes anything.
#[tauri::command]
pub fn scan_dry_run() -> Result<DryRunReport, String> {
    let root = env::current_dir().map_err(|e| format!("failed to read current directory: {e}"))?;

    let mut detected = Vec::new();
    for adapter in all_adapters() {
        if !adapter.detect(&root) {
            continue;
        }

        match adapter.import(&root) {
            Ok(result) => detected.push(DetectedTool {
                tool_id: adapter.id().to_string(),
                skills: result.skills.len(),
                agents: result.agents.len(),
                mcp_servers: result.mcp_servers.len(),
                error: None,
            }),
            Err(e) => detected.push(DetectedTool {
                tool_id: adapter.id().to_string(),
                skills: 0,
                agents: 0,
                mcp_servers: 0,
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(DryRunReport {
        root: root.display().to_string(),
        detected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_dry_run_succeeds_against_the_real_current_directory() {
        // No adapter fixtures live in this crate's own directory, so this
        // just proves the command runs end-to-end without panicking or
        // writing anything (same guarantee `apps/cli`'s equivalent test
        // makes) — actual detection behavior is exercised by
        // `packages/core`'s own adapter test suite, not duplicated here.
        let report = scan_dry_run().unwrap();
        assert!(!report.root.is_empty());
    }
}
