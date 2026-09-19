//! The commands the frontend may call, and the events it may listen to.
//!
//! Intentionally thin: commands validate input, delegate to a domain module, and map
//! the result. Logic lives in `audio`, `asr` and `models`, never here, so this file
//! stays readable as a list of what the application can do.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tauri_specta::Event;

use crate::asr::{EngineStatus, Transcript};
use crate::audio::Level;
use crate::audio::{self, MicrophoneInfo};
use crate::dictation::{DictationState, Discarded};
use crate::error::{CommandResult, Error};
use crate::history::Page;
use crate::hotkeys::{Hotkey, HotkeyBindings};
use crate::models::{self, DownloadProgress, InstallState};
use crate::sound::{self, Cue};
use crate::state::{AppState, Preferences};

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

// -------------------------------------------------------------------- hotkeys

/// The dictation hotkeys currently registered.
#[tauri::command]
#[specta::specta]
pub fn hotkey_bindings(state: State<'_, AppState>) -> CommandResult<HotkeyBindings> {
    Ok(state.hotkeys.current())
}

/// Validate a shortcut without registering it, so a rebinding UI can give immediate
/// feedback as the user types.
#[tauri::command]
#[specta::specta]
pub fn validate_hotkey(spec: String) -> CommandResult<Hotkey> {
    Ok(Hotkey::parse(&spec)?)
}

// ------------------------------------------------------------------ dictation

/// Whether Kiku is idle, listening or transcribing.
#[tauri::command]
#[specta::specta]
pub fn dictation_state(state: State<'_, AppState>) -> CommandResult<DictationState> {
    Ok(state.dictation.state())
}

/// Abandon the current recording without transcribing it.
#[tauri::command]
#[specta::specta]
pub fn cancel_dictation(state: State<'_, AppState>) -> CommandResult<()> {
    Ok(state.dictation.cancel()?)
}

/// Choose the microphone. `None` follows the system default.
#[tauri::command]
#[specta::specta]
pub fn set_microphone(state: State<'_, AppState>, device_id: Option<String>) -> CommandResult<()> {
    state.set_microphone(device_id);
    Ok(())
}

// ---------------------------------------------------------------- preferences

/// How transcripts are delivered and whether sounds play.
#[tauri::command]
#[specta::specta]
pub fn preferences(state: State<'_, AppState>) -> CommandResult<Preferences> {
    Ok(state.preferences())
}

#[tauri::command]
#[specta::specta]
pub fn set_preferences(state: State<'_, AppState>, preferences: Preferences) -> CommandResult<()> {
    state.set_preferences(preferences);
    Ok(())
}

// -------------------------------------------------------------------- history

/// A page of past dictations, newest first, optionally filtered by a search.
#[tauri::command]
#[specta::specta]
pub fn list_history(
    state: State<'_, AppState>,
    query: Option<String>,
    limit: u32,
    offset: u32,
) -> CommandResult<Page> {
    Ok(state.history()?.list(query.as_deref(), limit, offset)?)
}

/// Delete one entry.
#[tauri::command]
#[specta::specta]
pub fn delete_history_entry(state: State<'_, AppState>, id: f64) -> CommandResult<bool> {
    Ok(state.history()?.delete(id as i64)?)
}

/// Delete every entry. Returns how many went.
#[tauri::command]
#[specta::specta]
pub fn clear_history(state: State<'_, AppState>) -> CommandResult<u32> {
    Ok(state.history()?.clear()?)
}

/// Delete entries older than `days`. Returns how many went.
#[tauri::command]
#[specta::specta]
pub fn purge_history(state: State<'_, AppState>, days: u32) -> CommandResult<u32> {
    Ok(state.history()?.purge_older_than(days)?)
}

// --------------------------------------------------------------------- sounds

/// Which feedback cue to preview from Settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SoundCue {
    Start,
    Stop,
    Error,
}

/// Play a cue so the user can hear it while adjusting the setting.
///
/// Deliberately ignores the sounds preference: previewing is the one place a user
/// wants to hear a cue they have currently switched off.
#[tauri::command]
#[specta::specta]
pub fn preview_sound(cue: SoundCue) -> CommandResult<()> {
    let cue = match cue {
        SoundCue::Start => Cue::Start,
        SoundCue::Stop => Cue::Stop,
        SoundCue::Error => Cue::Error,
    };
    sound::play(cue, true);
    Ok(())
}

// ---------------------------------------------------------------------- events

/// Emitted repeatedly while a model downloads.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DownloadProgressed(pub DownloadProgress);

/// Emitted when the recogniser's state changes, so the UI never has to poll.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct EngineStatusChanged(pub EngineStatus);

/// Emitted as dictation moves between idle, listening and processing.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DictationStateChanged(pub DictationState);

/// Emitted about fourteen times a second while recording, to drive the waveform.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LevelMeasured(pub Level);

/// Emitted when a dictation produced text.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TranscriptProduced(pub Transcript);

/// Emitted when a dictation finished but produced nothing worth keeping.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DictationDiscarded(pub Discarded);
