use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_root: String,
    pub adapter_paths: Vec<String>,
    pub workspace_mode: bool,
    pub dark_mode: bool,
    pub auto_save: bool,
    pub sync_enabled: bool,
}

impl AppConfig {
    pub fn default() -> Self {
        Self {
            project_root: "~/AIBrain".to_string(),
            adapter_paths: vec![
                ".github/copilot-instructions.md".to_string(),
                ".claude/CLAUDE.md".to_string(),
                "~/.config/zed/AGENTS.md".to_string(),
            ],
            workspace_mode: true,
            dark_mode: true,
            auto_save: true,
            sync_enabled: false,
        }
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Expands a leading `~` to the user's home directory. Adapter paths are
/// per-tool literals (see docs/CODE_REVIEW.md §2.2) sourced from each
/// adapter's own config, so this only needs to handle the common `~/...`
/// shorthand rather than full shell tilde semantics.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(user_dirs) = UserDirs::new() {
            return user_dirs.home_dir().join(rest);
        }
    }
    PathBuf::from(path)
}

/// Resolves the OS-appropriate config file path for the app, e.g.
/// `~/.config/llm-neurosurgeon/settings.json` on Linux. This is distinct
/// from the repo's own `.claude/` directory, which is Claude Code's config
/// and must not be written to by this app (docs/CODE_REVIEW.md §2.1).
pub fn config_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let project_dirs = ProjectDirs::from("dev", "llmneurosurgeon", "LLM Neurosurgeon")
        .ok_or("could not determine an OS config directory for this platform")?;
    Ok(project_dirs.config_dir().join("settings.json"))
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = config_file_path()?;

    if config_path.exists() {
        AppConfig::load(&config_path)
    } else {
        let default_config = AppConfig::default();
        default_config.save(&config_path)?;
        Ok(default_config)
    }
}
