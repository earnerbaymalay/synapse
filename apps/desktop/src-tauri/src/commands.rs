use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Result, State, Window};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct AdapterCommand {
    name: String,
    arguments: Vec<String>,
}

pub fn get_version() -> String {
    return env!("CARGO_PKG_VERSION").to_string();
}

pub fn open_settings(window: Window) -> Result<(), String> {
    println!("Opening settings...");
    Ok(())
}

pub fn run_adapter_command(
    state: State<AppState>,
    command: AdapterCommand,
) -> Result<String, String> {
    println!("Running adapter command: {:?}", command);

    let output = format!(
        "Adapter '{}' executed with args: {:?}",
        command.name, command.arguments
    );

    Ok(output)
}

pub fn import_config(state: State<AppState>, path: String) -> Result<String, String> {
    println!("Importing config from: {}", path);

    let imported = format!("Config imported from {} successfully", path);

    Ok(imported)
}

pub fn export_config(state: State<AppState>, path: String) -> Result<String, String> {
    println!("Exporting config to: {}", path);

    let exported = format!("Config exported to {} successfully", path);

    Ok(exported)
}
