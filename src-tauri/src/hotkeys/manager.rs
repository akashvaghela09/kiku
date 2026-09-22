//! Interpreting what the dictation hotkeys mean.
//!
//! Hold versus toggle, and swallowing the key-repeat events some platforms emit while
//! a shortcut is held, both live in [`Interpreter`] - which is pure, and therefore
//! testable without an operating system. Registering the shortcuts with the platform
//! is `runtime`'s job, because that needs an `AppHandle` and this does not.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

use super::binding::Hotkey;
use super::defaults::{DEFAULT_HOLD, DEFAULT_TOGGLE};
use crate::error::Result;

/// What the user meant by a key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    /// Hold-to-talk began.
    HoldStarted,
    /// Hold-to-talk ended; the key came back up.
    HoldEnded,
    /// Toggle pressed - start if idle, stop if listening.
    Toggled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    pub hold: Hotkey,
    pub toggle: Hotkey,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            hold: Hotkey::parse(DEFAULT_HOLD).expect("the default hold hotkey must parse"),
            toggle: Hotkey::parse(DEFAULT_TOGGLE).expect("the default toggle hotkey must parse"),
        }
    }
}

/// Turns raw shortcut events into dictation actions.
///
/// Holding a shortcut produces repeated `Pressed` events on some platforms. Without
/// the `holding` latch each repeat would restart the recording, so a held key would
/// capture only the last few milliseconds before release.
///
/// Either binding may be a watched single key rather than a registered shortcut - a
/// bare modifier cannot be registered at all - in which case it is `None` here and
/// `watcher` reports it instead.
#[derive(Debug)]
pub struct Interpreter {
    hold: Option<Shortcut>,
    toggle: Option<Shortcut>,
    holding: AtomicBool,
}

impl Interpreter {
    pub fn new(bindings: &HotkeyBindings) -> Result<Self> {
        Ok(Self {
            hold: registrable(&bindings.hold)?,
            toggle: registrable(&bindings.toggle)?,
            holding: AtomicBool::new(false),
        })
    }

    pub fn interpret(&self, fired: &Shortcut, state: ShortcutState) -> Option<HotkeyAction> {
        if self.hold.as_ref() == Some(fired) {
            return match state {
                // `swap` makes the latch atomic: two repeats arriving together still
                // produce exactly one HoldStarted.
                ShortcutState::Pressed => (!self.holding.swap(true, Ordering::SeqCst))
                    .then_some(HotkeyAction::HoldStarted),
                ShortcutState::Released => self
                    .holding
                    .swap(false, Ordering::SeqCst)
                    .then_some(HotkeyAction::HoldEnded),
            };
        }

        if self.toggle.as_ref() == Some(fired) {
            // Only the press matters; the release of a toggle means nothing.
            return matches!(state, ShortcutState::Pressed).then_some(HotkeyAction::Toggled);
        }

        None
    }

    /// Release the latch - used when a recording is cancelled by something other than
    /// the key coming up, so the next press is not swallowed.
    pub fn reset(&self) {
        self.holding.store(false, Ordering::SeqCst);
    }
}

/// The shortcut a binding registers as, or `None` when it is a watched single key.
fn registrable(hotkey: &Hotkey) -> Result<Option<Shortcut>> {
    if hotkey.single_key().is_some() {
        return Ok(None);
    }
    hotkey.shortcut().map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bindings where both halves are registrable chords, which is what the
    /// Interpreter is responsible for. The shipped default hold is a watched single
    /// key and never reaches this code.
    fn chord_bindings() -> HotkeyBindings {
        HotkeyBindings {
            hold: Hotkey::parse(crate::hotkeys::defaults::FALLBACK_HOLD).unwrap(),
            toggle: Hotkey::parse(DEFAULT_TOGGLE).unwrap(),
        }
    }

    fn interpreter() -> Interpreter {
        Interpreter::new(&chord_bindings()).unwrap()
    }

    fn hold() -> Shortcut {
        chord_bindings().hold.shortcut().unwrap()
    }

    fn toggle() -> Shortcut {
        chord_bindings().toggle.shortcut().unwrap()
    }

    #[test]
    fn holding_starts_once_and_ends_once() {
        let interpreter = interpreter();
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Pressed),
            Some(HotkeyAction::HoldStarted)
        );
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            Some(HotkeyAction::HoldEnded)
        );
    }

    #[test]
    fn key_repeat_while_held_is_swallowed() {
        let interpreter = interpreter();
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Pressed),
            Some(HotkeyAction::HoldStarted)
        );
        // Everything the platform repeats while the key stays down must be ignored,
        // or each repeat would restart the recording mid-sentence.
        for _ in 0..25 {
            assert_eq!(interpreter.interpret(&hold(), ShortcutState::Pressed), None);
        }
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            Some(HotkeyAction::HoldEnded)
        );
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let interpreter = interpreter();
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            None
        );
    }

    #[test]
    fn repeated_releases_end_the_hold_only_once() {
        let interpreter = interpreter();
        interpreter.interpret(&hold(), ShortcutState::Pressed);
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            Some(HotkeyAction::HoldEnded)
        );
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            None
        );
    }

    #[test]
    fn toggle_fires_on_press_and_ignores_release() {
        let interpreter = interpreter();
        assert_eq!(
            interpreter.interpret(&toggle(), ShortcutState::Pressed),
            Some(HotkeyAction::Toggled)
        );
        assert_eq!(
            interpreter.interpret(&toggle(), ShortcutState::Released),
            None
        );
    }

    #[test]
    fn the_toggle_key_does_not_disturb_the_hold_latch() {
        let interpreter = interpreter();
        interpreter.interpret(&hold(), ShortcutState::Pressed);
        interpreter.interpret(&toggle(), ShortcutState::Pressed);
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Released),
            Some(HotkeyAction::HoldEnded)
        );
    }

    #[test]
    fn an_unrelated_shortcut_is_ignored() {
        let interpreter = interpreter();
        let other = Hotkey::parse("Ctrl+Shift+K").unwrap().shortcut().unwrap();
        assert_eq!(interpreter.interpret(&other, ShortcutState::Pressed), None);
    }

    #[test]
    fn resetting_lets_the_next_press_start_again() {
        let interpreter = interpreter();
        interpreter.interpret(&hold(), ShortcutState::Pressed);
        interpreter.reset();
        assert_eq!(
            interpreter.interpret(&hold(), ShortcutState::Pressed),
            Some(HotkeyAction::HoldStarted)
        );
    }

    #[test]
    fn the_default_hold_is_a_single_key_and_the_toggle_is_a_chord() {
        let bindings = HotkeyBindings::default();
        assert!(
            bindings.hold.single_key().is_some(),
            "holding should need one key, not a chord"
        );
        assert_eq!(bindings.toggle.spec, DEFAULT_TOGGLE);
        assert!(
            bindings.toggle.single_key().is_none(),
            "the toggle is a chord, so it registers rather than being watched"
        );
    }

    /// The defaults have to be usable as what they are, not merely parseable: the hold
    /// key is watched and must be refused by the shortcut registrar, and the toggle is
    /// registered and must be accepted by it.
    #[test]
    fn each_default_suits_the_path_that_delivers_it() {
        let bindings = HotkeyBindings::default();
        assert!(
            bindings.hold.shortcut().is_err(),
            "a bare modifier cannot register"
        );
        assert!(
            bindings.toggle.shortcut().is_ok(),
            "the toggle must register"
        );
    }

    #[test]
    fn a_watched_hold_key_leaves_the_interpreter_only_the_toggle() {
        // The watcher reports the single key; the Interpreter must not expect it to
        // arrive as a registered shortcut.
        let interpreter = Interpreter::new(&HotkeyBindings::default()).unwrap();
        assert!(interpreter.hold.is_none());
        assert!(interpreter.toggle.is_some());
    }
}
