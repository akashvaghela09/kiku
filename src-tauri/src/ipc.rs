//! The commands the frontend may call, and the events it may listen to.
//!
//! Intentionally thin: commands validate input, delegate to a domain module, and map
//! the result. Logic lives in `audio`, `asr` and `models`, never here, so this file
//! stays readable as a list of what the application can do.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tauri_specta::Event;

use crate::asr::EngineStatus;
use crate::audio::{self, MicrophoneInfo};
use crate::error::{CommandResult, Error};
use crate::models::{self, DownloadProgress, InstallState};
use crate::state::AppState;

// ---------------------------------------------------------------- application

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

// ----------------------------------------------------------------- microphones

/// Microphones available right now, default first.
///
/// cpal has no device-change notification, so the frontend re-queries rather than
/// caching.
#[tauri::command]
#[specta::specta]
pub fn list_microphones() -> CommandResult<Vec<MicrophoneInfo>> {
    Ok(audio::list_microphones()?)
}

// ---------------------------------------------------------------------- models

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub summary: String,
    /// See `DownloadProgress` for why byte counts cross the boundary as `f64`.
    pub total_bytes: f64,
    pub install: InstallState,
}

/// Every model Kiku can install, with what is on disk right now.
#[tauri::command]
#[specta::specta]
pub fn list_models(state: State<'_, AppState>) -> CommandResult<Vec<ModelInfo>> {
    Ok(models::ALL
        .iter()
        .map(|spec| ModelInfo {
            id: spec.id.to_owned(),
            name: spec.name.to_owned(),
            summary: spec.summary.to_owned(),
            total_bytes: spec.total_bytes() as f64,
            install: state.models.state(spec),
        })
        .collect())
}

/// Download and install a model, emitting `DownloadProgressed` as it goes.
///
/// Awaited by the caller so onboarding can show a completion state, while progress
/// arrives out of band.
#[tauri::command]
#[specta::specta]
pub async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> CommandResult<()> {
    let spec = models::find(&model_id)
        .ok_or_else(|| Error::Internal(format!("unknown model {model_id}")))?;

    models::download(&state.models, spec, |progress| {
        // A failed emit means the window went away mid-download; the download itself
        // is still worth finishing.
        let _ = DownloadProgressed(progress).emit(&app);
    })
    .await?;

    Ok(())
}

/// Remove an installed model, including any interrupted download.
#[tauri::command]
#[specta::specta]
pub fn delete_model(state: State<'_, AppState>, model_id: String) -> CommandResult<()> {
    let spec = models::find(&model_id)
        .ok_or_else(|| Error::Internal(format!("unknown model {model_id}")))?;
    Ok(state.models.delete(spec)?)
}

/// Re-hash an installed model against its pinned checksums.
///
/// Slow by design — this is the repair path, not a startup check.
#[tauri::command]
#[specta::specta]
pub async fn verify_model(state: State<'_, AppState>, model_id: String) -> CommandResult<()> {
    let spec = models::find(&model_id)
        .ok_or_else(|| Error::Internal(format!("unknown model {model_id}")))?;
    Ok(state.models.verify(spec)?)
}

// ---------------------------------------------------------------------- engine

/// Whether the recogniser is ready, loading, or failed.
#[tauri::command]
#[specta::specta]
pub fn engine_status(state: State<'_, AppState>) -> CommandResult<EngineStatus> {
    Ok(state.asr.status())
}

// ---------------------------------------------------------------------- events

/// Emitted repeatedly while a model downloads.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DownloadProgressed(pub DownloadProgress);

/// Emitted when the recogniser's state changes, so the UI never has to poll.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct EngineStatusChanged(pub EngineStatus);
