//! Detects which AI tools are installed and which config paths they own.
//! Phase 3 wires this to the real adapter list; for now it just defines the
//! result shape the CLI's `scan` verb and the Tauri `scan` command share.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub tool_id: String,
    pub config_paths: Vec<PathBuf>,
}

impl ScanResult {
    pub fn new(tool_id: impl Into<String>, config_paths: Vec<PathBuf>) -> Self {
        Self {
            tool_id: tool_id.into(),
            config_paths,
        }
    }

    pub fn is_present(&self) -> bool {
        !self.config_paths.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_tool_has_no_config_paths() {
        let result = ScanResult::new("cursor", vec![]);
        assert!(!result.is_present());
    }

    #[test]
    fn present_tool_reports_its_config_paths() {
        let result = ScanResult::new("claude-code", vec![PathBuf::from(".claude/CLAUDE.md")]);
        assert!(result.is_present());
        assert_eq!(result.config_paths.len(), 1);
    }
}
