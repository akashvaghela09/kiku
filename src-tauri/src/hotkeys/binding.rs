//! Hotkey specifications: parsing, validation and display.
//!
//! A binding is stored as a portable string such as `Alt+Space`. It is parsed into the
//! platform shortcut type when registered, and rendered differently per platform when
//! shown — a macOS user expects `⌥Space`, not `Alt+Space`.
//!
//! Only modifier-plus-key combinations exist here, because that is all the operating
//! systems can register. Windows' `RegisterHotKey`, macOS' `RegisterEventHotKey` and
//! X11's `XGrabKey` each take a modifier mask plus exactly one key; a chord of two
//! ordinary keys has no representation and would in any case fire while typing.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_plugin_global_shortcut::Shortcut;

use crate::error::{Error, Result};

/// Hold to talk. Released ends the recording.
pub const DEFAULT_HOLD: &str = "Alt+Space";

/// Press once to start, again to stop.
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

        // Reject anything the OS could not register, with a sentence that explains the
        // rule rather than echoing a parser error.
        let shortcut = Shortcut::from_str(trimmed).map_err(|_| {
            Error::Internal(format!(
                "{trimmed} isn't a shortcut Kiku can use. Combine at least one modifier \
                 (Ctrl, Alt, Shift or Cmd) with one other key."
            ))
        })?;

        if shortcut.mods.is_empty() {
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

    pub fn shortcut(&self) -> Result<Shortcut> {
        Shortcut::from_str(&self.spec)
            .map_err(|_| Error::Internal(format!("{} is no longer a valid shortcut.", self.spec)))
    }
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
    fn both_defaults_are_valid() {
        assert!(Hotkey::parse(DEFAULT_HOLD).is_ok());
        assert!(Hotkey::parse(DEFAULT_TOGGLE).is_ok());
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
