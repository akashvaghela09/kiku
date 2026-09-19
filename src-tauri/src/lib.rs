//! Kiku — offline dictation.
//!
//! Modules are organised by domain rather than by layer, so a change to "how audio is
//! captured" touches one directory instead of being spread across `services/`,
//! `utils/` and `helpers/`.

pub mod asr;
pub mod audio;
pub mod dictation;
pub mod error;
pub mod hotkeys;
pub mod models;
pub mod state;

mod ipc;
mod runtime;

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
            ipc::verify_model,
            ipc::engine_status,
            ipc::hotkey_bindings,
            ipc::validate_hotkey,
            ipc::dictation_state,
            ipc::cancel_dictation,
            ipc::set_microphone,
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            tracing::info!(path = %data_dir.display(), "application data directory");

            app.manage(AppState::new(data_dir));

            // The overlay window is created at launch and merely hidden, never created
            // on demand: creating an OS window costs 30-120 ms of visible lag, which is
            // most of the latency budget for the whole press-to-paint path.
            if let Some(overlay) = app.get_webview_window("overlay") {
                overlay.set_ignore_cursor_events(true)?;
            }

            // A hotkey already owned by another application must not stop Kiku from
            // starting: the window opens, Settings shows the conflict, and the user
            // rebinds.
            if let Err(error) =
                runtime::install_hotkeys(app.handle(), crate::hotkeys::HotkeyBindings::default())
            {
                tracing::warn!(%error, "dictation hotkeys are unavailable");
            }

            load_model_in_background(app.handle().clone());

            if let Some(main) = app.get_webview_window("main") {
                main.show()?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Kiku");
}

/// Load the speech model without blocking startup.
///
/// Loading costs about four seconds. Doing it on the setup thread would mean a window
/// that does not paint until it finishes, so it runs on a blocking worker and the UI
/// follows `EngineStatusChanged` instead.
fn load_model_in_background(app: tauri::AppHandle) {
    use crate::asr::ModelFiles;
    use tauri_specta::Event;

    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();

        let Some(spec) = models::ALL
            .iter()
            .find(|spec| state.models.is_installed(spec))
        else {
            tracing::info!("no model installed yet; onboarding will download one");
            let _ = ipc::EngineStatusChanged(state.asr.status()).emit(&app);
            return;
        };

        let dir = state.models.dir_for(spec);
        let result = ModelFiles::discover(&dir).and_then(|files| state.asr.load(&files, spec.id));

        if let Err(error) = result {
            tracing::error!(%error, model = spec.id, "could not load the speech model");
        }

        let _ = ipc::EngineStatusChanged(state.asr.status()).emit(&app);
    });
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
