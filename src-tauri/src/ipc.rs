//! The commands the frontend may call, and the events it may listen to.
//!
//! Intentionally thin: commands validate input, delegate to a domain module, and map
//! the result. Logic lives in `audio`, `asr` and `models`, never here, so this file
//! stays readable as a list of what the application can do.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State};
use tauri_specta::Event;

use crate::asr::{EngineStatus, Transcript};
use crate::audio::Level;
use crate::audio::{self, MicrophoneInfo};
use crate::dictation::{DictationState, Discarded};
use crate::error::{CommandResult, Error};
use crate::feedback::{self, Cue};
use crate::history::Page;
use crate::hotkeys::{Hotkey, HotkeyBindings};
use crate::models::{self, DownloadProgress, InstallState};
use crate::state::{AppState, Preferences};
use crate::update::{self, UpdateStatus};

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
    /// See `DownloadProgress` for why byte counts cross the boundary as `u32`.
    pub total_bytes: u32,
    pub install: InstallState,
    /// Whether this is the model the recogniser currently has loaded.
    pub active: bool,
}

/// Every model Kiku can install, with what is on disk right now.
#[tauri::command]
#[specta::specta]
pub fn list_models(state: State<'_, AppState>) -> CommandResult<Vec<ModelInfo>> {
    let loaded = match state.asr.status() {
        EngineStatus::Ready(id) => Some(id),
        _ => None,
    };

    Ok(models::ALL
        .iter()
        .map(|spec| ModelInfo {
            id: spec.id.to_owned(),
            name: spec.name.to_owned(),
            summary: spec.summary.to_owned(),
            total_bytes: spec.total_bytes().min(u64::from(u32::MAX)) as u32,
            install: state.models.state(spec),
            active: loaded.as_deref() == Some(spec.id),
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

    // Load it, unless a model is already loaded or on its way.
    //
    // `use_model` was the only path that ever loaded one, so onboarding ended with a
    // model on disk, a recogniser still empty, and no way to tell the difference from
    // the first screen: the download reported success and dictation stayed dead until
    // the user went to Settings and chose the model they had just downloaded. A
    // restart also fixed it, because startup loads whichever model is installed, which
    // is what made it look intermittent.
    //
    // Guarded rather than unconditional: downloading a second model while a first one
    // is working is a download, not a request to switch. That is what `use_model` is
    // for, and it is the only thing that writes the preference.
    let handle = app.clone();
    let id = model_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let busy = {
            let state = handle.state::<AppState>();
            matches!(
                state.asr.status(),
                EngineStatus::Ready(_) | EngineStatus::Loading
            )
        };
        if !busy {
            crate::load_model(&handle, Some(&id));
        }
    });

    Ok(())
}

/// Remove an installed model, freeing its disk space.
///
/// If it is the model currently loaded, the recogniser is unloaded first and another
/// installed model is loaded in its place - deleting one model must not leave
/// dictation broken when another is available.
#[tauri::command]
#[specta::specta]
pub async fn delete_model(app: tauri::AppHandle, model_id: String) -> CommandResult<()> {
    let spec = models::find(&model_id)
        .ok_or_else(|| Error::Internal(format!("unknown model {model_id}")))?;

    {
        let state = app.state::<AppState>();
        let in_use = matches!(state.asr.status(), EngineStatus::Ready(ref id) if *id == model_id);
        if in_use {
            state.asr.unload();
        }
        state.models.delete(spec)?;

        // Forget a preference that now names nothing.
        let mut preferences = state.preferences();
        if preferences.model_id.as_deref() == Some(model_id.as_str()) {
            preferences.model_id = None;
            state.set_preferences(preferences);
        }
    }

    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::load_model(&handle, None));
    Ok(())
}

/// Switch to a different installed model.
///
/// Returns once the model is loaded, since the caller wants to know when dictation is
/// usable again rather than when the request was accepted.
#[tauri::command]
#[specta::specta]
pub async fn use_model(app: tauri::AppHandle, model_id: String) -> CommandResult<EngineStatus> {
    let spec = models::find(&model_id)
        .ok_or_else(|| Error::Internal(format!("unknown model {model_id}")))?;

    {
        let state = app.state::<AppState>();
        if !state.models.is_installed(spec) {
            return Err(Error::ModelMissing.into());
        }
        let mut preferences = state.preferences();
        preferences.model_id = Some(model_id.clone());
        state.set_preferences(preferences);
    }

    let handle = app.clone();
    let id = model_id.clone();
    tauri::async_runtime::spawn_blocking(move || crate::load_model(&handle, Some(&id)))
        .await
        .map_err(|error| Error::Internal(error.to_string()))?;

    Ok(app.state::<AppState>().asr.status())
}

/// Re-hash an installed model against its pinned checksums.
///
/// Slow by design - this is the repair path, not a startup check.
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

/// Rebind the dictation key.
///
/// Validated before anything is torn down, so a key Kiku cannot watch never takes the
/// working one away.
#[tauri::command]
#[specta::specta]
pub fn set_hotkeys(
    app: tauri::AppHandle,
    bindings: HotkeyBindings,
) -> CommandResult<HotkeyBindings> {
    let validated = HotkeyBindings {
        hold: Hotkey::parse(&bindings.hold.spec)?,
        hands_free: Hotkey::parse(&bindings.hands_free.spec)?,
    };

    crate::runtime::rebind(&app, validated.clone())?;
    Ok(validated)
}

// ------------------------------------------------------------------- startup

/// Whether Kiku is registered to start with the computer.
///
/// Asked of the operating system rather than remembered, because the login item can be
/// removed from System Settings without Kiku ever knowing. A preference would go on
/// claiming a thing that is no longer true.
#[tauri::command]
#[specta::specta]
pub fn starts_with_computer(app: tauri::AppHandle) -> CommandResult<bool> {
    use tauri_plugin_autostart::ManagerExt;
    Ok(app.autolaunch().is_enabled().unwrap_or(false))
}

/// Register or remove the login item.
#[tauri::command]
#[specta::specta]
pub fn set_starts_with_computer(app: tauri::AppHandle, enabled: bool) -> CommandResult<bool> {
    use tauri_plugin_autostart::ManagerExt;

    let launcher = app.autolaunch();
    let outcome = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };

    if let Err(error) = outcome {
        return Err(Error::Internal(format!(
            "Kiku could not change whether it starts with your computer: {error}"
        ))
        .into());
    }

    // Report what the system says, not what was asked for.
    Ok(launcher.is_enabled().unwrap_or(false))
}

/// Open a link in the user's browser.
///
/// The only outbound link in the application is the GitHub release page, which is why
/// this exists at all.
#[tauri::command]
#[specta::specta]
pub fn open_url(app: tauri::AppHandle, url: String) -> CommandResult<()> {
    use tauri_plugin_opener::OpenerExt;

    // Refuse anything that is not plainly a web link: `open` hands the string to the
    // platform, where a `file:` or custom scheme would launch something local.
    if !url.starts_with("https://") {
        return Err(Error::Internal("Only https links can be opened.".into()).into());
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| Error::Internal(error.to_string()))?;
    Ok(())
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
pub fn delete_history_entry(state: State<'_, AppState>, id: u32) -> CommandResult<bool> {
    Ok(state.history()?.delete(i64::from(id))?)
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
    /// Kiku started listening.
    Start,
    /// Kiku stopped listening.
    Stop,
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
    };
    feedback::play(cue, true);
    Ok(())
}

// --------------------------------------------------------------------- updates

/// Whether a newer Kiku has been published.
///
/// Uses a result cached for a day unless `force` is set, and reports `Unknown` rather
/// than an error when offline - a failed update check is not something to interrupt
/// someone about.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(
    state: State<'_, AppState>,
    force: bool,
) -> CommandResult<UpdateStatus> {
    if !state.preferences().check_for_updates {
        return Ok(UpdateStatus::Unknown);
    }

    let current = env!("CARGO_PKG_VERSION");
    match update::check(&state.data_dir, current, force).await {
        Ok(status) => Ok(status),
        Err(error) => {
            tracing::debug!(%error, "update check did not complete");
            Ok(UpdateStatus::Unknown)
        }
    }
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
