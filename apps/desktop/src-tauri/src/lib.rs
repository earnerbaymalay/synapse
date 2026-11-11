pub mod commands;
pub mod config;
pub mod dry_run;
pub mod error;
pub mod logger;
pub mod state;

use std::sync::Mutex;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init_logger();

    tauri::Builder::default()
        .manage(Mutex::new(AppState::new()))
        // No custom menu. On macOS Tauri installs the standard app menu
        // itself (enable_macos_default_menu defaults to true), which is what
        // makes Cmd+C / Cmd+V / Cmd+Q work in the webview. The hand-built menu
        // this replaced overrode that with a "Quit" item that had no event
        // handler (clicking it did nothing) and empty Edit/View/Tools/Help
        // submenus, and on Linux/Windows it put a native menu bar inside the
        // frameless window, above the custom title bar.
        .setup(|_app| {
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
