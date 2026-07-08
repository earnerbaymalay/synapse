#![allow(unused_imports)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.code,
            self.message,
            self.details.as_deref().unwrap_or("")
        )
    }
}

impl std::error::Error for AppError {}

pub fn from_tauri_error(error: tauri::Error) -> AppError {
    AppError {
        code: "TAURI_ERROR".to_string(),
        message: "Tauri application error".to_string(),
        details: Some(error.to_string()),
    }
}

pub fn handle_error(error: Box<dyn std::error::Error>) -> AppError {
    AppError {
        code: "APP_ERROR".to_string(),
        message: "Application error".to_string(),
        details: Some(error.to_string()),
    }
}
