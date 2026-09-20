//! Hotkey specifications: parsing, validation and display.
//!
//! A binding is stored as a portable string such as `Alt+Space`. It is parsed into the
//! platform shortcut type when registered, and rendered differently per platform when
//! shown - a macOS user expects `⌥Space`, not `Alt+Space`.
//!
//! Only modifier-plus-key combinations exist here, because that is all the operating
//! systems can register. Windows' `RegisterHotKey`, macOS' `RegisterEventHotKey` and
//! X11's `XGrabKey` each take a modifier mask plus exactly one key; a chord of two
//! ordinary keys has no representation and would in any case fire while typing.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_plugin_global_shortcut::{Code, Shortcut};

use super::watcher::SingleKey;
use crate::error::{Error, Result};

/// Hold to talk. Releasing the key ends the recording.
///
/// A single key, because one key is far easier to *hold* than a chord, and Right Ctrl
/// is never used alone by any operating system. It cannot be registered as a global
/// shortcut - no platform accepts a bare modifier - so it is watched instead; see
/// [`super::watcher`].
///
/// Not `Alt+Space`, which reads better but is genuinely contested: it opens the window
/// system menu on Windows and on GNOME and Cinnamon - verified bound to
/// `activate-window-menu` on the development machine.
///
/// Mac keyboards have no right Control key, so macOS watches Right Option instead.
#[cfg(target_os = "macos")]
pub const DEFAULT_HOLD: &str = "RightAlt";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_HOLD: &str = "RightControl";

/// Fallback for keyboards without a usable right-hand modifier, and for anyone who
/// would rather have a chord. Unclaimed at the OS level on all three platforms.
pub const FALLBACK_HOLD: &str = "Ctrl+Shift+Space";

/// Press once to start, again to stop. Same base key, so there is one thing to learn.
pub const DEFAULT_TOGGLE: &str = "Ctrl+Alt+Space";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Hotkey {
    /// Portable specification, for example `Ctrl+Alt+Space`.
    pub spec: String,
    /// Rendered for this platform, for example `⌃⌥Space` on macOS.
    pub display: String,
}

impl Hotkey {
    pub fn parse(spec: &str) -> Result<Self> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return Err(Error::Internal("A shortcut cannot be empty.".into()));
        }

        // Single keys are watched rather than registered, so they never reach the
        // platform's shortcut parser - which rejects bare modifiers outright.
        if let Some(single) = SingleKey::parse(trimmed) {
            return Ok(Self {
                spec: trimmed.to_owned(),
                display: single.display().to_owned(),
            });
        }

        // Reject anything the OS could not register, with a sentence that explains the
        // rule rather than echoing a parser error.
        let shortcut = Shortcut::from_str(trimmed).map_err(|_| {
            Error::Internal(format!(
                "{trimmed} isn't a shortcut Kiku can use. Combine at least one modifier \
                 (Ctrl, Alt, Shift or Cmd) with one other key."
            ))
        })?;

        if shortcut.mods.is_empty() && !is_safe_without_modifier(shortcut.key) {
            return Err(Error::Internal(format!(
                "{trimmed} has no modifier. A shortcut without Ctrl, Alt, Shift or Cmd \
                 would fire while you type."
            )));
        }

        Ok(Self {
            display: display_for(trimmed),
            spec: trimmed.to_owned(),
        })
    }

    /// The single key this binding watches, if it is one.
    pub fn single_key(&self) -> Option<SingleKey> {
        SingleKey::parse(&self.spec)
    }

    /// The registrable shortcut this binding is, if it is one.
    pub fn shortcut(&self) -> Result<Shortcut> {
        if self.single_key().is_some() {
            return Err(Error::Internal(format!(
                "{} is watched rather than registered.",
                self.spec
            )));
        }
        Shortcut::from_str(&self.spec)
            .map_err(|_| Error::Internal(format!("{} is no longer a valid shortcut.", self.spec)))
    }
}

/// Whether a key can be bound on its own.
///
/// A bare letter or digit would fire in the middle of a sentence, which is why
/// modifiers are normally required. Function keys never appear in prose, so binding
/// one alone is safe - and a single key is far easier to *hold* than a three-key
/// chord, which matters for push-to-talk.
fn is_safe_without_modifier(key: Code) -> bool {
    matches!(
        key,
        Code::F1
            | Code::F2
            | Code::F3
            | Code::F4
            | Code::F5
            | Code::F6
            | Code::F7
            | Code::F8
            | Code::F9
            | Code::F10
            | Code::F11
            | Code::F12
            | Code::F13
            | Code::F14
            | Code::F15
            | Code::F16
            | Code::F17
            | Code::F18
            | Code::F19
            | Code::F20
            | Code::Pause
            | Code::ScrollLock
    )
}

/// Render a specification the way this platform's users expect to read it.
#[cfg(target_os = "macos")]
fn display_for(spec: &str) -> String {
    spec.split('+')
        .map(|part| match part.trim().to_ascii_lowercase().as_str() {
            "cmd" | "command" | "super" | "meta" => "⌘",
            "ctrl" | "control" => "⌃",
            "alt" | "option" => "⌥",
            "shift" => "⇧",
            _ => part.trim(),
        })
        .collect::<Vec<_>>()
        .concat()
}

#[cfg(not(target_os = "macos"))]
fn display_for(spec: &str) -> String {
    spec.split('+')
        .map(|part| {
            let part = part.trim();
            match part.to_ascii_lowercase().as_str() {
                "control" => "Ctrl".to_owned(),
                "option" => "Alt".to_owned(),
                "super" | "meta" | "cmd" | "command" => "Win".to_owned(),
                // Title-case, so `alt+space` and `Alt+Space` render identically.
                lowered => {
                    let mut chars = lowered.chars();
                    match chars.next() {
                        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                        None => String::new(),
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_valid() {
        for spec in [DEFAULT_HOLD, DEFAULT_TOGGLE, FALLBACK_HOLD] {
            assert!(Hotkey::parse(spec).is_ok(), "{spec} should parse");
        }
    }

    #[test]
    fn the_default_hold_is_a_watched_single_key() {
        let hold = Hotkey::parse(DEFAULT_HOLD).unwrap();
        assert!(
            hold.single_key().is_some(),
            "the default should be one key to hold"
        );
        assert!(
            hold.shortcut().is_err(),
            "a watched key must not be handed to the shortcut registrar"
        );
    }

    #[test]
    fn a_chord_is_registrable_and_not_a_watched_key() {
        let chord = Hotkey::parse(FALLBACK_HOLD).unwrap();
        assert!(chord.single_key().is_none());
        assert!(chord.shortcut().is_ok());
    }

    #[test]
    fn a_single_key_gets_a_readable_display_name() {
        let hold = Hotkey::parse("RightControl").unwrap();
        assert!(!hold.display.is_empty());
        assert_ne!(
            hold.display, "RightControl",
            "it should be written for a person"
        );
    }

    #[test]
    fn a_function_key_may_be_bound_on_its_own() {
        // The whole point of offering F9: one key is far easier to hold than three,
        // and a function key cannot fire while typing.
        for spec in ["F8", "F9", "F10"] {
            assert!(Hotkey::parse(spec).is_ok(), "{spec} should be allowed");
        }
    }

    #[test]
    fn a_bare_letter_or_digit_is_still_rejected() {
        for spec in ["A", "K", "1", "Space", "Enter"] {
            assert!(
                Hotkey::parse(spec).is_err(),
                "{spec} should need a modifier"
            );
        }
    }

    #[test]
    fn a_bare_key_is_rejected_with_an_explanation() {
        let error = Hotkey::parse("Space").unwrap_err().to_string();
        assert!(
            error.contains("modifier"),
            "the message should explain the rule: {error}"
        );
    }

    #[test]
    fn an_empty_specification_is_rejected() {
        assert!(Hotkey::parse("   ").is_err());
    }

    #[test]
    fn nonsense_is_rejected() {
        assert!(Hotkey::parse("Alt+NotAKey").is_err());
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        let hotkey = Hotkey::parse("  Alt+Space  ").unwrap();
        assert_eq!(hotkey.spec, "Alt+Space");
    }

    #[test]
    fn a_parsed_binding_round_trips_to_a_shortcut() {
        assert!(Hotkey::parse(DEFAULT_TOGGLE).unwrap().shortcut().is_ok());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn display_is_normalised_on_windows_and_linux() {
        assert_eq!(Hotkey::parse("alt+space").unwrap().display, "Alt+Space");
        assert_eq!(
            Hotkey::parse("Ctrl+Alt+Space").unwrap().display,
            "Ctrl+Alt+Space"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn display_uses_symbols_on_macos() {
        assert_eq!(Hotkey::parse("Alt+Space").unwrap().display, "⌥Space");
        assert_eq!(Hotkey::parse("Ctrl+Alt+Space").unwrap().display, "⌃⌥Space");
    }
}
