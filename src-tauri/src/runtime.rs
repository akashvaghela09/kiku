//! How the application reacts to a hotkey.
//!
//! This is the only place that knows about both a global shortcut and a Tauri window,
//! which keeps `dictation`, `audio` and `asr` free of any UI concern and testable on
//! their own.

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_specta::Event;

use crate::dictation::{DictationState, Outcome};
use crate::hotkeys::{HotkeyAction, HotkeyBindings, Interpreter};
use crate::ipc::{DictationDiscarded, DictationStateChanged, LevelMeasured, TranscriptProduced};
use crate::state::AppState;

/// Register the dictation hotkeys and route their events into a session.
pub fn install_hotkeys(app: &AppHandle, bindings: HotkeyBindings) -> crate::Result<()> {
    let interpreter = std::sync::Arc::new(Interpreter::new(&bindings)?);

    let handler_app = app.clone();
    let handler_interpreter = std::sync::Arc::clone(&interpreter);

    app.global_shortcut()
        .on_shortcuts(
            [bindings.hold.shortcut()?, bindings.toggle.shortcut()?],
            move |_, shortcut: &Shortcut, event| {
                let state: ShortcutState = event.state();
                if let Some(action) = handler_interpreter.interpret(shortcut, state) {
                    handle(&handler_app, action);
                }
            },
        )
        .map_err(|error| {
            tracing::warn!(%error, "could not attach the hotkey handler");
            crate::Error::HotkeyTaken(bindings.hold.display.clone())
        })?;

    app.state::<AppState>().hotkeys.adopt(bindings);
    Ok(())
}

fn handle(app: &AppHandle, action: HotkeyAction) {
    let state = app.state::<AppState>();

    match action {
        HotkeyAction::HoldStarted => start(app),
        HotkeyAction::HoldEnded => stop(app),
        // One key for both edges: start when idle, stop when already listening.
        HotkeyAction::Toggled => {
            if state.dictation.is_listening() {
                stop(app);
            } else {
                start(app);
            }
        }
    }
}

fn start(app: &AppHandle) {
    let state = app.state::<AppState>();

    if !state.asr.is_ready() {
        // Say so rather than appearing to ignore the key. The overlay cannot host an
        // actionable error because it is click-through, so this surfaces as an event
        // the main window can raise.
        tracing::info!("hotkey pressed before the model finished loading");
        let _ = DictationDiscarded(crate::dictation::Discarded::NoSpeech).emit(app);
        return;
    }

    let level_app = app.clone();
    let result = state.dictation.start(state.microphone(), move |level| {
        let _ = LevelMeasured(level).emit(&level_app);
    });

    match result {
        Ok(()) => {
            show_overlay(app);
            publish_state(app, DictationState::Listening);
        }
        Err(error) => {
            tracing::error!(%error, "could not start dictation");
            let _ = app.emit("dictation-error", error.to_string());
            hide_overlay(app);
        }
    }
}

fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.dictation.is_listening() {
        return;
    }

    publish_state(app, DictationState::Processing);

    // Decoding blocks for a few hundred milliseconds; keep it off the event loop so
    // the overlay keeps animating.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();

        match state.dictation.stop(&state.asr) {
            Ok(Outcome::Transcribed(transcript)) => {
                let _ = TranscriptProduced(*transcript).emit(&app);
            }
            Ok(Outcome::Discarded(reason)) => {
                tracing::debug!(?reason, "dictation produced no text");
                let _ = DictationDiscarded(reason).emit(&app);
            }
            Err(error) => {
                tracing::error!(%error, "dictation failed");
                let _ = app.emit("dictation-error", error.to_string());
            }
        }

        publish_state(&app, DictationState::Idle);
        hide_overlay(&app);
    });
}

fn publish_state<R: Runtime>(app: &AppHandle<R>, state: DictationState) {
    let _ = DictationStateChanged(state).emit(app);
}

fn show_overlay<R: Runtime>(app: &AppHandle<R>) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        // `show` rather than create: the window exists from launch, because creating
        // one costs 30-120 ms of visible lag on the press-to-paint path.
        let _ = overlay.show();
    }
}

fn hide_overlay<R: Runtime>(app: &AppHandle<R>) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
}
