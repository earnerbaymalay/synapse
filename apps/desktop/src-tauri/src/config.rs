use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = ".claude/settings.json";

    if Path::new(config_path).exists() {
        AppConfig::load(config_path)
    } else {
        let default_config = AppConfig::default();
        default_config.save(config_path)?;
        Ok(default_config)
    }
}
