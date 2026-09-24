//! Kiku - offline dictation.
//!
//! Modules are organised by domain rather than by layer, so a change to "how audio is
//! captured" touches one directory instead of being spread across `services/`,
//! `utils/` and `helpers/`.

pub mod asr;
pub mod audio;
pub mod dictation;
pub mod error;
pub mod feedback;
pub mod history;
pub mod hotkeys;
pub mod models;
pub mod output;
pub mod overlay;
pub mod state;
pub mod update;

mod ipc;
pub mod runtime;
pub mod settings;
pub mod tray;

pub use error::{CommandResult, Error, ErrorPayload, Result};

use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, Builder};

use crate::state::AppState;

/// The single source of truth for the IPC surface.
///
/// Used twice: by `run` to register the handlers, and by the `bindings` test to
/// generate `src/lib/ipc/bindings.ts`. Because both read the same builder, a Rust
/// signature change is a TypeScript compile error rather than a runtime surprise.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            ipc::app_info,
            ipc::list_microphones,
            ipc::list_models,
            ipc::download_model,
            ipc::delete_model,
            ipc::use_model,
            ipc::verify_model,
            ipc::engine_status,
            ipc::hotkey_bindings,
            ipc::validate_hotkey,
            ipc::set_hotkeys,
            ipc::open_url,
            ipc::starts_with_computer,
            ipc::set_starts_with_computer,
            ipc::dictation_state,
            ipc::cancel_dictation,
            ipc::set_microphone,
            ipc::preferences,
            ipc::set_preferences,
            ipc::preview_sound,
            ipc::list_history,
            ipc::delete_history_entry,
            ipc::clear_history,
            ipc::purge_history,
            ipc::check_for_update,
        ])
        .events(collect_events![
            ipc::DownloadProgressed,
            ipc::EngineStatusChanged,
            ipc::DictationStateChanged,
            ipc::LevelMeasured,
            ipc::TranscriptProduced,
            ipc::DictationDiscarded,
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kiku=info,warn".into()),
        )
        .init();

    let builder = specta_builder();

    let app = tauri::Builder::default();

    // One Kiku at a time. Every copy watches the dictation key, so a second one records
    // the same speech and pastes it again: double text. Starting with the computer
    // launches a hidden copy, and opening Kiku from the menu later was enough to get
    // two. A second launch now shows the running window and exits instead - unless it
    // is itself a login launch, which has nothing to show.
    //
    // Not macOS: opening an application that is already running brings that one
    // forward rather than starting another, so there is nothing to guard against.
    // Registered first, as the plugin requires, so the second copy exits before
    // anything else starts.
    #[cfg(not(target_os = "macos"))]
    let app = app.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
        tracing::info!("Kiku is already running; not starting a second copy");
        if !args.iter().any(|argument| argument == "--hidden") {
            tray::reveal(app);
        }
    }));

    app.plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Starting with the computer is off until asked for. `--hidden` is what makes
        // it bearable: a dictation key that steals focus at login every morning would
        // be worse than one you have to start yourself.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            tracing::info!(path = %data_dir.display(), "application data directory");

            let state = AppState::new(data_dir);
            // Whatever the user last chose, or the shipped default on a first run.
            let bindings = state.hotkeys.current();
            app.manage(state);

            // The overlay window is created at launch and merely hidden, never created
            // on demand: creating an OS window costs 30-120 ms of visible lag, which is
            // most of the latency budget for the whole press-to-paint path.
            // A hotkey already owned by another application must not stop Kiku from
            // starting: the window opens, Settings shows the conflict, and the user
            // rebinds.
            if let Err(error) = runtime::install_hotkeys(app.handle(), bindings) {
                tracing::warn!(%error, "dictation hotkeys are unavailable");
            }

            apply_retention(app.handle());

            load_model_in_background(app.handle().clone());

            if let Err(error) = tray::install(app.handle()) {
                // Not fatal, but it does mean closing the window would leave no way
                // back, so the window is kept closable-to-quit instead.
                tracing::warn!(%error, "the tray is unavailable");
            }

            if let Some(main) = app.get_webview_window("main") {
                keep_running_when_closed(&main);
                // Started by the system at login, the window stays out of the way; the
                // tray says Kiku is there and the dictation key already works.
                if !started_hidden() {
                    main.show()?;
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Kiku")
        .run(|app, event| {
            // Clicking the dock icon of an application with no open windows. Without
            // this the window is gone for good: closing it hides it, macOS still shows
            // the application as running, and the one gesture for "come back" does
            // nothing at all.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                tray::reveal(app);
            }

            // Closing the last window must not end the process. The dictation key is
            // the product; the window is where its settings live.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }

            let _ = (app, event);
        });
}

/// Whether this launch should keep the window to itself.
///
/// Passed by the login item registered for "start with the computer", so the first
/// thing a user sees in the morning is not a window they did not ask for.
fn started_hidden() -> bool {
    std::env::args().any(|argument| argument == "--hidden")
}

/// Hide the window when it is closed, rather than destroying it.
///
/// Kiku is a dictation key that happens to have a window, so closing that window means
/// "I have finished with the settings", not "stop listening". Destroying it also left
/// no way back: the process stayed alive because the overlay is still loaded, macOS
/// went on showing the application as running, and nothing recreated the window - so
/// the only way to get it back was to quit and launch again.
///
/// Quitting is the tray's Quit item, or the usual quit shortcut.
fn keep_running_when_closed(window: &tauri::WebviewWindow) {
    let closing = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(error) = closing.hide() {
                tracing::warn!(%error, "could not hide the window");
            }
        }
    });
}

/// Delete history older than the user's retention setting.
///
/// Runs once at startup rather than on a timer: Kiku is not a long-running service,
/// and a retention rule that only takes effect when the application is open is both
/// sufficient and easier to reason about than a background job.
fn apply_retention(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let Some(days) = state.preferences().retention_days else {
        return;
    };

    match state
        .history()
        .and_then(|history| history.purge_older_than(days))
    {
        Ok(0) => {}
        Ok(removed) => tracing::info!(removed, days, "purged old history"),
        Err(error) => tracing::warn!(%error, "could not apply the history retention rule"),
    }
}

/// Load the speech model without blocking startup.
///
/// Loading costs about four seconds. Doing it on the setup thread would mean a window
/// that does not paint until it finishes, so it runs on a blocking worker and the UI
/// follows `EngineStatusChanged` instead.
fn load_model_in_background(app: tauri::AppHandle) {
    tauri::async_runtime::spawn_blocking(move || {
        let preferred = app.state::<AppState>().preferences().model_id;
        load_model(&app, preferred.as_deref());
    });
}

/// Load a model into the recogniser, blocking until it is ready.
///
/// `preferred` picks a specific model; without one, whichever is installed wins. A
/// preference naming a model that is no longer installed falls back rather than
/// failing, because deleting a model must not leave dictation broken when another one
/// is still there.
pub(crate) fn load_model(app: &tauri::AppHandle, preferred: Option<&str>) {
    use crate::asr::ModelFiles;
    use tauri_specta::Event;

    let state = app.state::<AppState>();

    let chosen = preferred
        .and_then(models::find)
        .filter(|spec| state.models.is_installed(spec))
        .or_else(|| {
            models::ALL
                .iter()
                .find(|spec| state.models.is_installed(spec))
        });

    let Some(spec) = chosen else {
        tracing::info!("no model installed yet; onboarding will download one");
        state.asr.unload();
        let _ = ipc::EngineStatusChanged(state.asr.status()).emit(app);
        return;
    };

    let dir = state.models.dir_for(spec);
    let result = ModelFiles::discover(&dir).and_then(|files| state.asr.load(&files, spec.id));

    if let Err(error) = result {
        tracing::error!(%error, model = spec.id, "could not load the speech model");
    }

    let _ = ipc::EngineStatusChanged(state.asr.status()).emit(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regenerates the TypeScript bindings. Run with `cargo test export_bindings`.
    /// Kept as a test so type generation can never be linked into a release binary.
    #[test]
    fn export_bindings() {
        specta_builder()
            .export(
                specta_typescript::Typescript::default()
                    .header("// Generated by tauri-specta. Do not edit.\n"),
                "../src/lib/ipc/bindings.ts",
            )
            .expect("failed to export TypeScript bindings");
    }
}
