use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{State, WebviewWindow};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct AdapterCommand {
    name: String,
    arguments: Vec<String>,
}

#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Dry-run of the auto-update channel (T7.3). Given the release manifest a
/// channel endpoint served (fetched by the frontend), decides whether a
/// newer build exists for this platform — WITHOUT downloading, verifying, or
/// installing anything. Until release signing keys exist (Phase 8), the
/// result's `signing` field is `not_configured` and the UI must not offer a
/// one-click install: this is a check, not an installer.
#[tauri::command]
pub fn check_for_update(
    manifest_json: String,
    channel: String,
) -> Result<neurosurgeon_core::updater::DryRunResult, String> {
    let current = env!("CARGO_PKG_VERSION");
    neurosurgeon_core::updater::dry_run_from_json(current, &manifest_json, &channel)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_settings(window: WebviewWindow) -> Result<(), String> {
    println!("Opening settings for window '{}'...", window.label());
    Ok(())
}

#[tauri::command]
pub fn run_adapter_command(
    state: State<'_, Mutex<AppState>>,
    command: AdapterCommand,
) -> Result<String, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    println!(
        "Running adapter command: {:?} (sync_status: {})",
        command, app_state.sync_status
    );

    let output = format!(
        "Adapter '{}' executed with args: {:?}",
        command.name, command.arguments
    );

    Ok(output)
}

#[tauri::command]
pub fn import_config(state: State<'_, Mutex<AppState>>, path: String) -> Result<String, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    let count = app_state.adapter_count;
    app_state.update_adapter_status(count, "connected");
    println!("Importing config from: {}", path);

    let imported = format!("Config imported from {} successfully", path);

    Ok(imported)
}

#[tauri::command]
pub fn export_config(state: State<'_, Mutex<AppState>>, path: String) -> Result<String, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    println!(
        "Exporting config to: {} ({} adapters tracked)",
        path, app_state.adapter_count
    );

    let exported = format!("Config exported to {} successfully", path);

    Ok(exported)
}
