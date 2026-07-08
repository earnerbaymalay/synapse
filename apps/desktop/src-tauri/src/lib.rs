pub mod commands;
pub mod config;
pub mod error;
pub mod logger;
pub mod state;

use std::sync::Mutex;
use tauri::{CustomMenuItem, Menu, Submenu, WindowBuilder, WindowUrl};

use state::AppState;

fn build_menu() -> Menu {
    let file_menu = Submenu::new(
        "File",
        Menu::new().add_item(CustomMenuItem::new("quit".to_string(), "Quit")),
    );
    let edit_menu = Submenu::new("Edit", Menu::new());
    let view_menu = Submenu::new("View", Menu::new());
    let tools_menu = Submenu::new("Tools", Menu::new());
    let help_menu = Submenu::new("Help", Menu::new());

    Menu::new()
        .add_submenu(file_menu)
        .add_submenu(edit_menu)
        .add_submenu(view_menu)
        .add_submenu(tools_menu)
        .add_submenu(help_menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init_logger();

    tauri::Builder::default()
        .menu(build_menu())
        .manage(Mutex::new(AppState::new()))
        .setup(|app| {
            let _window = WindowBuilder::new(app, "main", WindowUrl::App("index.html".into()))
                .title("LLM Neurosurgeon")
                .inner_size(1200.0, 800.0)
                .min_inner_size(800.0, 600.0)
                .build()?;

            let _config = config::load_config()?;

            log::info!("LLM Neurosurgeon Desktop v{} initialized", env!("CARGO_PKG_VERSION"));
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
