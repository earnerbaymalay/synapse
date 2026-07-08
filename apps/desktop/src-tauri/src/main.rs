mod commands;
mod config;
mod error;
mod logger;
mod state;

use std::path::PathBuf;
use tauri::{Builder, Menu, MenuItem, WindowBuilder, WindowUrl};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    let menu = Menu::new()
        .add_item(MenuItem::new("File", true))
        .add_item(MenuItem::new("Edit", true))
        .add_item(MenuItem::new("View", true))
        .add_item(MenuItem::new("Tools", true))
        .add_item(MenuItem::new("Help", true));

    let app = Builder::default()
        .menu(menu)
        .setup(|app| {
            // Initialize application state
            let _state = state::AppState::new()?;

            // Create main window
            let window = WindowBuilder::new()
                .title("LLM Neurosurgeon")
                .inner_size(1200.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .build(app.handle())?;

            // Load initial configuration
            let config = config::load_config()?;

            println!("LLM Neurosurgeon Desktop v1.0.0 initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_version,
            commands::open_settings,
            commands::run_adapter_command,
            commands::import_config,
            commands::export_config,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(move |_app_handle| Box::new(move || {}));
}
