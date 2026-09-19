//! The commands the frontend may call.
//!
//! This module is intentionally thin: commands validate input, delegate to a domain
//! module, and map the result. Logic lives in `audio`, `asr`, `models`, and so on —
//! never here, so the IPC surface stays readable as a list of capabilities.

use serde::Serialize;
use specta::Type;

use crate::audio::{self, MicrophoneInfo};
use crate::error::CommandResult;

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
pub fn app_info() -> CommandResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        identifier: "io.github.akashvaghela09.kiku".to_owned(),
        platform: std::env::consts::OS.to_owned(),
    })
}

/// Microphones available right now, default first.
///
/// Called by Settings, and again whenever the device list may have changed — cpal has
/// no change notification, so the frontend re-queries rather than caching.
#[tauri::command]
#[specta::specta]
pub fn list_microphones() -> CommandResult<Vec<MicrophoneInfo>> {
    Ok(audio::list_microphones()?)
}
