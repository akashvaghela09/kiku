//! Interpreting what the dictation hotkeys mean.
//!
//! Hold versus toggle, and swallowing the key-repeat events some platforms emit while
//! a shortcut is held, both live in [`Interpreter`] — which is pure, and therefore
//! testable without an operating system. Registering the shortcuts with the platform
//! is `runtime`'s job, because that needs an `AppHandle` and this does not.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

use super::binding::{Hotkey, DEFAULT_HOLD, DEFAULT_TOGGLE};
use crate::error::Result;

/// What the user meant by a key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    /// Hold-to-talk began.
    HoldStarted,
    /// Hold-to-talk ended; the key came back up.
    HoldEnded,
    /// Toggle pressed — start if idle, stop if listening.
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
#[derive(Debug)]
pub struct Interpreter {
    hold: Shortcut,
    toggle: Shortcut,
    holding: AtomicBool,
}

impl Interpreter {
    pub fn new(bindings: &HotkeyBindings) -> Result<Self> {
        Ok(Self {
            hold: bindings.hold.shortcut()?,
            toggle: bindings.toggle.shortcut()?,
            holding: AtomicBool::new(false),
        })
    }

    pub fn interpret(&self, fired: &Shortcut, state: ShortcutState) -> Option<HotkeyAction> {
        if *fired == self.hold {
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

        if *fired == self.toggle {
            // Only the press matters; the release of a toggle means nothing.
            return matches!(state, ShortcutState::Pressed).then_some(HotkeyAction::Toggled);
        }

        None
    }

    /// Release the latch — used when a recording is cancelled by something other than
    /// the key coming up, so the next press is not swallowed.
    pub fn reset(&self) {
        self.holding.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpreter() -> Interpreter {
        Interpreter::new(&HotkeyBindings::default()).unwrap()
    }

    fn hold() -> Shortcut {
        HotkeyBindings::default().hold.shortcut().unwrap()
    }

    fn toggle() -> Shortcut {
        HotkeyBindings::default().toggle.shortcut().unwrap()
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
    fn the_defaults_are_the_documented_pair() {
        let bindings = HotkeyBindings::default();
        assert_eq!(bindings.hold.spec, "Ctrl+Shift+Space");
        assert_eq!(bindings.toggle.spec, "Ctrl+Alt+Space");
    }

    #[test]
    fn the_defaults_share_a_base_key_so_there_is_one_thing_to_learn() {
        let bindings = HotkeyBindings::default();
        let base = |spec: &str| spec.rsplit('+').next().unwrap_or_default().to_owned();
        assert_eq!(base(&bindings.hold.spec), base(&bindings.toggle.spec));
    }
}
