//! Getting the transcript into the application the user is actually typing in.
//!
//! The transcript is written to the clipboard and then a paste keystroke is
//! synthesised. That is deliberate rather than lazy: typing the text character by
//! character is slow, mangles non-ASCII on several keyboard layouts, and races with
//! autocomplete. A clipboard paste arrives as one atomic edit.
//!
//! Two details matter and are easy to get wrong:
//!
//! * **Modifiers may still be physically held.** Hold-to-talk fires on key *release*,
//!   and a user commonly lifts the space bar a moment before Alt. Sending Ctrl+V then
//!   would really send Ctrl+Alt+V. Every modifier is released first.
//! * **The clipboard needs a moment.** On X11 the clipboard is a negotiation between
//!   processes rather than a buffer, so a paste issued in the same breath as the write
//!   can fetch the previous contents.

use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tauri::AppHandle;

use crate::error::{Error, Result};

/// Pause between claiming the clipboard and asking for a paste.
///
/// Long enough for X11's ownership handshake, short enough to stay well inside the
/// latency budget a user would notice.
const CLIPBOARD_SETTLE: Duration = Duration::from_millis(40);

/// Modifiers that must not be down when the paste chord is sent.
const MODIFIERS: [Key; 4] = [Key::Alt, Key::Control, Key::Shift, Key::Meta];

/// The platform's paste modifier: Command on macOS, Control elsewhere.
#[cfg(target_os = "macos")]
const PASTE_MODIFIER: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const PASTE_MODIFIER: Key = Key::Control;

/// Synthesise a paste into whatever window currently has focus.
///
/// The caller must have put the text on the clipboard first. Returns an error when
/// the platform refuses to synthesise input - on macOS that means Accessibility
/// permission has not been granted, which the UI turns into an actionable prompt.
pub fn paste_into_focused_window(app: &AppHandle) -> Result<()> {
    // Waited for here rather than on the main thread: it is a pause for another
    // process to catch up, and stalling the interface for it would be visible.
    std::thread::sleep(CLIPBOARD_SETTLE);

    on_the_main_thread(app, synthesise_paste)
}

/// Run `work` on the main thread and hand back what it returned.
///
/// `run_on_main_thread` executes the closure inline when it is already on the main
/// thread, so the value is always sent before it is awaited and this cannot deadlock
/// against itself.
fn on_the_main_thread(app: &AppHandle, work: fn() -> Result<()>) -> Result<()> {
    let (sender, receiver) = std::sync::mpsc::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(work());
    })
    .map_err(|error| Error::Clipboard(format!("could not reach the main thread: {error}")))?;

    receiver
        .recv()
        .map_err(|_| Error::Clipboard("the paste keystroke never ran".into()))?
}

fn synthesise_paste() -> Result<()> {
    let mut enigo =
        Enigo::new(&settings()).map_err(|error| permission_denied(&error.to_string()))?;

    // Release anything the user may still be holding from the hotkey itself.
    for modifier in MODIFIERS {
        let _ = enigo.key(modifier, Direction::Release);
    }

    enigo
        .key(PASTE_MODIFIER, Direction::Press)
        .map_err(input_failed)?;
    let result = enigo.key(Key::Unicode('v'), Direction::Click);
    // Release the modifier whatever happened, so a failure cannot leave Ctrl stuck
    // down across the user's whole desktop.
    let release = enigo.key(PASTE_MODIFIER, Direction::Release);

    result.map_err(input_failed)?;
    release.map_err(input_failed)?;
    Ok(())
}

/// How the keyboard connection is opened.
///
/// `open_prompt_to_get_permissions` is `true` by default, and enigo checks the
/// permission by calling `AXIsProcessTrustedWithOptions` with the prompt option set -
/// which opens "Kiku would like to control this computer using accessibility features"
/// *every time it is called while the permission is missing*. A connection is opened
/// per paste, so an ungranted Kiku asked again on every single dictation.
///
/// Asking without the prompt turns that into a plain error, which the caller already
/// knows how to surface. The dialog is still offered, but once.
fn settings() -> Settings {
    Settings {
        open_prompt_to_get_permissions: false,
        ..Settings::default()
    }
}

/// Ask for Accessibility, at most once for the lifetime of the process.
///
/// Repeating the request on every paste is what made it feel broken. One dialog is a
/// permission request; one per dictation is a fault.
#[cfg(target_os = "macos")]
fn prompt_once() {
    use std::sync::Once;
    static ASKED: Once = Once::new();
    ASKED.call_once(|| {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
    });
}

#[cfg(not(target_os = "macos"))]
fn prompt_once() {}

fn permission_denied(error: &str) -> Error {
    prompt_once();
    Error::Clipboard(format!("could not reach the keyboard: {error}"))
}

fn input_failed(error: impl std::fmt::Display) -> Error {
    Error::Clipboard(format!("the paste keystroke was refused: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paste_modifier_matches_the_platform() {
        if cfg!(target_os = "macos") {
            assert_eq!(PASTE_MODIFIER, Key::Meta);
        } else {
            assert_eq!(PASTE_MODIFIER, Key::Control);
        }
    }

    #[test]
    fn every_modifier_is_cleared_before_pasting() {
        // The list must contain the paste modifier itself, or a user holding Ctrl
        // would send Ctrl+Ctrl+V.
        assert!(MODIFIERS.contains(&PASTE_MODIFIER));
        assert!(
            MODIFIERS.contains(&Key::Alt),
            "Alt+Space is the default hotkey"
        );
    }

    #[test]
    fn the_settle_delay_stays_within_the_latency_budget() {
        assert!(CLIPBOARD_SETTLE <= Duration::from_millis(100));
    }
}
