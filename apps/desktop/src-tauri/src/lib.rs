pub mod commands;
pub mod config;
pub mod dry_run;
pub mod error;
pub mod logger;
pub mod state;

use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::{WebviewUrl, WebviewWindowBuilder};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init_logger();

    tauri::Builder::default()
        .manage(Mutex::new(AppState::new()))
        .setup(|app| {
            let handle = app.handle();

            let quit_item = MenuItem::new(handle, "Quit", true, None::<&str>)?;
            let file_menu = Submenu::with_items(handle, "File", true, &[&quit_item])?;
            let edit_menu = Submenu::with_items(handle, "Edit", true, &[])?;
            let view_menu = Submenu::with_items(handle, "View", true, &[])?;
            let tools_menu = Submenu::with_items(handle, "Tools", true, &[])?;
            let help_menu = Submenu::with_items(handle, "Help", true, &[])?;
            let menu = Menu::with_items(
                handle,
                &[&file_menu, &edit_menu, &view_menu, &tools_menu, &help_menu],
            )?;
            app.set_menu(menu)?;

            let _window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("LLM Neurosurgeon")
                    .inner_size(1200.0, 800.0)
                    .min_inner_size(800.0, 600.0)
                    .build()?;

            let _config = config::load_config()?;

            log::info!(
                "LLM Neurosurgeon Desktop v{} initialized",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_version,
            commands::open_settings,
            commands::run_adapter_command,
            commands::import_config,
            commands::export_config,
            dry_run::scan_dry_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
