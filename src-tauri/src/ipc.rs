//! The commands the frontend may call.
//!
//! This module is intentionally thin: commands validate input, delegate to a domain
//! module, and map the result. Logic lives in `audio`, `asr`, `models`, and so on —
//! never here, so the IPC surface stays readable as a list of capabilities.

use serde::Serialize;
use specta::Type;

use crate::error::Result;

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub identifier: String,
    pub platform: String,
}

/// Identity of the running build, shown in Settings → About and used by the update
/// check to decide whether a newer release exists.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        identifier: "io.github.akashvaghela09.kiku".to_owned(),
        platform: std::env::consts::OS.to_owned(),
    })
}
