pub mod commands;
pub mod config;
pub mod error;
pub mod logger;
pub mod state;

use std::sync::Mutex;
use tauri::{Builder, Window, WindowBuilder, WindowUrl};

#[derive(Debug)]
pub struct AppState {
    pub config_loaded: bool,
    pub last_adapter_check: String,
    pub adapter_count: usize,
    pub sync_status: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config_loaded: false,
            last_adapter_check: "2026-07-07T14:23:12Z".to_string(),
            adapter_count: 12,
            sync_status: "disconnected".to_string(),
        }
    }

    pub fn update_adapter_status(&mut self, count: usize, status: &str) {
        self.adapter_count = count;
        self.last_adapter_check = chrono::offset::Utc::now().to_rfc3339();
        self.sync_status = status.to_string();
    }
}

#[cfg_attr(not(debug_assertions), tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .setup(|app| {
            println!("LLM Neurosurgeon Desktop v1.0.0 initialized");

            // Create main window
            let window = WindowBuilder::new()
                .title("LLM Neurosurgeon")
                .inner_size(1200.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .build(app.handle())?;

            println!("Window created successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_version,
            commands::open_settings,
            commands::run_adapter_command,
            commands::import_config,
            commands::export_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
