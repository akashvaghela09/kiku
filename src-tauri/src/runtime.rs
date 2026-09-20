//! How the application reacts to a hotkey.
//!
//! This is the only place that knows about both a global shortcut and a Tauri window,
//! which keeps `dictation`, `audio` and `asr` free of any UI concern and testable on
//! their own.

use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_specta::Event;

use crate::dictation::{DictationState, Outcome};
use crate::hotkeys::{
    HotkeyAction, HotkeyBindings, Interpreter, KeyWatcher, TapOutcome, Thresholds,
};
use crate::ipc::{DictationDiscarded, DictationStateChanged, LevelMeasured, TranscriptProduced};
use crate::output::{self, Delivery};
use crate::overlay;
use crate::sound::{self, Cue};
use crate::state::AppState;

/// Install the dictation hotkeys and route them into a session.
///
/// A binding takes one of two paths depending on what it is. A chord is registered
/// with the operating system. A bare modifier such as Right Ctrl cannot be registered
/// by any platform, so it is watched by polling its key state instead.
pub fn install_hotkeys(app: &AppHandle, bindings: HotkeyBindings) -> crate::Result<()> {
    let interpreter = std::sync::Arc::new(Interpreter::new(&bindings)?);

    let registered: Vec<Shortcut> = [&bindings.hold, &bindings.toggle]
        .iter()
        .filter(|hotkey| hotkey.single_key().is_none())
        .map(|hotkey| hotkey.shortcut())
        .collect::<crate::Result<_>>()?;

    if !registered.is_empty() {
        let handler_app = app.clone();
        let handler_interpreter = std::sync::Arc::clone(&interpreter);

        app.global_shortcut()
            .on_shortcuts(registered, move |_, shortcut: &Shortcut, event| {
                let state: ShortcutState = event.state();
                if let Some(action) = handler_interpreter.interpret(shortcut, state) {
                    handle(&handler_app, action);
                }
            })
            .map_err(|error| {
                tracing::warn!(%error, "could not attach the hotkey handler");
                crate::Error::HotkeyTaken(bindings.hold.display.clone())
            })?;
    }

    install_watcher(app, &bindings);

    tracing::info!(
        hold = %bindings.hold.spec,
        toggle = %bindings.toggle.spec,
        "dictation hotkeys installed"
    );
    app.state::<AppState>().hotkeys.adopt(bindings);
    Ok(())
}

/// Start polling a single-key binding, or stop any watcher if neither is one.
fn install_watcher(app: &AppHandle, bindings: &HotkeyBindings) {
    let Some(key) = bindings.hold.single_key() else {
        app.state::<AppState>().hotkeys.watch(None);
        return;
    };

    let watcher_app = app.clone();
    let watcher = KeyWatcher::start(key, Thresholds::default(), move |outcome| match outcome {
        TapOutcome::Start => start(&watcher_app),
        TapOutcome::Finish => stop(&watcher_app),
        // A press too brief to be speech, or the key being used as a modifier. The
        // audio is thrown away rather than transcribed.
        TapOutcome::Discard => cancel(&watcher_app),
    });

    app.state::<AppState>().hotkeys.watch(Some(watcher));
}

/// Escape, registered only while a session is running.
///
/// The overlay is click-through by design, so it can never receive a click and the
/// keyboard is the only channel through which a recording can be abandoned. Escape is
/// grabbed for the duration of the session and released immediately afterwards —
/// holding it globally would break Escape everywhere else on the system.
fn escape_shortcut() -> Option<Shortcut> {
    Shortcut::from_str("Escape").ok()
}

fn grab_escape(app: &AppHandle) {
    let Some(escape) = escape_shortcut() else {
        return;
    };

    let handler_app = app.clone();
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut(escape, move |_, _, event| {
            if matches!(event.state(), ShortcutState::Pressed) {
                cancel(&handler_app);
            }
        })
    {
        // Something else owns Escape. Dictation still works; only the cancel
        // gesture is unavailable, which is not worth interrupting anyone about.
        tracing::debug!(%error, "could not grab Escape for cancelling");
    }
}

fn release_escape(app: &AppHandle) {
    if let Some(escape) = escape_shortcut() {
        let _ = app.global_shortcut().unregister(escape);
    }
}

fn cancel(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.dictation.is_listening() {
        return;
    }

    if let Err(error) = state.dictation.cancel() {
        tracing::warn!(%error, "could not cancel the recording");
    }

    release_escape(app);
    let _ = DictationDiscarded(crate::dictation::Discarded::TooShort).emit(app);
    publish_state(app, DictationState::Idle);
    hide_overlay(app);
}

/// Replace the registered hotkeys.
///
/// Unregisters everything first, then installs the new pair. If the new pair cannot be
/// registered — because something else already owns it — the previous pair is put
/// back, so a failed rebinding never leaves the user without a working shortcut.
pub fn rebind(app: &AppHandle, next: HotkeyBindings) -> crate::Result<()> {
    let previous = app.state::<AppState>().hotkeys.current();
    app.global_shortcut().unregister_all().ok();
    app.state::<AppState>().hotkeys.watch(None);

    match install_hotkeys(app, next) {
        Ok(()) => Ok(()),
        Err(error) => {
            app.global_shortcut().unregister_all().ok();
            if let Err(rollback) = install_hotkeys(app, previous) {
                tracing::error!(%rollback, "could not restore the previous hotkeys");
            }
            Err(error)
        }
    }
}

fn handle(app: &AppHandle, action: HotkeyAction) {
    tracing::debug!(?action, "hotkey action");
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
            sound::play(Cue::Listening, state.preferences().sounds);
            grab_escape(app);
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

    release_escape(app);
    publish_state(app, DictationState::Processing);

    // Decoding blocks for a few hundred milliseconds; keep it off the event loop so
    // the overlay keeps animating.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();

        match state.dictation.stop(&state.asr) {
            Ok(Outcome::Transcribed(transcript)) => {
                deliver(&app, *transcript);
            }
            Ok(Outcome::Discarded(reason)) => {
                tracing::debug!(?reason, "dictation produced no text");
                sound::play(Cue::Error, state.preferences().sounds);
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

/// Put the transcript where the user is typing, then tell the UI what happened.
///
/// The clipboard copy is unconditional, so even a refused paste leaves the text
/// somewhere reachable — which is why a delivery failure is reported rather than
/// treated as a lost dictation.
fn deliver(app: &AppHandle, transcript: crate::asr::Transcript) {
    let state = app.state::<AppState>();
    let preferences = state.preferences();

    let text = output::prepare(&transcript.text, preferences.trailing_space);

    match output::deliver(app, &text, preferences.auto_paste) {
        // The cue marks the text arriving, so it plays on delivery rather than on the
        // key release. Reaching the clipboard counts: the text arrived either way, and
        // a refused paste is not a failed dictation.
        Ok(Delivery::Pasted) => {
            tracing::debug!("transcript pasted");
            sound::play(Cue::Pasted, preferences.sounds);
        }
        Ok(Delivery::CopiedOnly) => {
            tracing::info!("transcript copied but not pasted");
            sound::play(Cue::Pasted, preferences.sounds);
        }
        Err(error) => {
            tracing::error!(%error, "could not deliver the transcript");
            sound::play(Cue::Error, preferences.sounds);
            let _ = app.emit("dictation-error", error.to_string());
        }
    }

    record(app, &transcript);

    // The event carries the model's text, not the whitespace-adjusted copy: history
    // should record what was said, not how it was spliced into another application.
    let _ = TranscriptProduced(transcript).emit(app);
}

/// Save a transcript to history, unless the user has paused recording.
///
/// A history failure is logged and nothing else: the text has already reached the
/// user, and losing the record of a dictation must never look like losing the
/// dictation itself.
fn record(app: &AppHandle, transcript: &crate::asr::Transcript) {
    let state = app.state::<AppState>();
    if state.preferences().history_paused {
        return;
    }

    let model_id = match state.asr.status() {
        crate::asr::EngineStatus::Ready(model) => Some(model),
        _ => None,
    };

    let saved = state.history().and_then(|history| {
        history.insert(
            &transcript.text,
            transcript.audio_ms,
            transcript.decode_ms,
            model_id.as_deref(),
        )
    });

    if let Err(error) = saved {
        tracing::error!(%error, "could not save the dictation to history");
    }
}

fn publish_state<R: Runtime>(app: &AppHandle<R>, state: DictationState) {
    let _ = DictationStateChanged(state).emit(app);
}

fn show_overlay(app: &AppHandle) {
    // `show` rather than create: the window exists from launch, because creating one
    // costs 30-120 ms of visible lag on the press-to-paint path. Its position is
    // recomputed every time, never cached.
    overlay::show(app);
}

fn hide_overlay(app: &AppHandle) {
    overlay::hide(app);
}
