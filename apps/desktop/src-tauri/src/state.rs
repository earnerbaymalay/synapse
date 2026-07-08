use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub config_loaded: bool,
    pub last_adapter_check: String,
    pub adapter_count: usize,
    pub sync_status: String,
}

impl AppState {
    pub fn new() -> Result<Mutex<AppState>, Box<dyn std::error::Error>> {
        let state = Mutex::new(AppState {
            config_loaded: false,
            last_adapter_check: chrono::offset::Utc::now().to_rfc3339(),
            adapter_count: 12,
            sync_status: "disconnected".to_string(),
        });
        Ok(state)
    }

    pub fn update_adapter_status(&mut self, count: usize, status: &str) {
        self.adapter_count = count;
        self.last_adapter_check = chrono::offset::Utc::now().to_rfc3339();
        self.sync_status = status.to_string();
    }
}
