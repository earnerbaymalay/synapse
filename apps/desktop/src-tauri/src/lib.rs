pub mod commands;
pub mod config;
pub mod dry_run;
pub mod error;
pub mod logger;
pub mod state;

use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, Submenu};

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

            // A missing or malformed settings.json must not stop the window
            // from opening: `setup` returning Err aborts the whole app with
            // "error while running tauri application". Nothing reads the
            // config yet, so log and carry on with defaults.
            if let Err(e) = config::load_config() {
                log::warn!("could not load settings, using defaults: {e}");
            }

            log::info!("Synapse Desktop v{} initialized", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_version,
            commands::check_for_update,
            commands::intake,
            commands::examine,
            commands::heal_vitals,
            commands::get_memory_graph,
            commands::get_cockpit_telemetry,
            commands::spool_list,
            commands::spool_show,
            dry_run::scan_dry_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
