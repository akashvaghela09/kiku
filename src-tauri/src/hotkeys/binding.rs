//! The dictation key: parsing, validation and display.
//!
//! A binding is stored as a portable string such as `RightControl`, and rendered for
//! the platform it is shown on - a macOS user expects `Right ⌥`, not `RightAlt`.
//!
//! Only a bare right-hand modifier can be bound. That is the whole vocabulary, and it
//! is a deliberate narrowing: one key carries both modes, holding it to talk and
//! tapping it twice to keep listening, so a second binding would be a second way to do
//! something the first key already does. Chords used to be bindable and are not any
//! more - they cost a delivery path of their own, and every one of them was a worse
//! version of a key you can simply hold.
//!
//! A key that is bound here is *watched*, never registered: no platform accepts a bare
//! modifier as a global shortcut, and watching is what lets the key keep working as an
//! ordinary modifier everywhere else.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::watcher::SingleKey;
use crate::error::{Error, Result};

// The shipped bindings live in `defaults`, so there is one file to edit when they
// change and no second copy to fall out of step with it.

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

        let Some(single) = SingleKey::parse(trimmed) else {
            return Err(Error::Internal(format!(
                "{trimmed} isn't a key Kiku can use. Pick a right-hand modifier - \
                 Right Ctrl, Right Alt or Right Cmd - and hold it to talk, or tap it \
                 twice to keep listening."
            )));
        };

        Ok(Self {
            spec: trimmed.to_owned(),
            display: single.display().to_owned(),
        })
    }

    /// The key this binding watches.
    ///
    /// Always `Some` for a binding that came through [`Self::parse`]; it stays an
    /// `Option` because a specification can also arrive from a settings file written by
    /// something else.
    pub fn single_key(&self) -> Option<SingleKey> {
        SingleKey::parse(&self.spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::defaults::DEFAULT_HOLD;

    #[test]
    fn the_default_parses_and_is_watched() {
        let hold = Hotkey::parse(DEFAULT_HOLD).unwrap();
        assert!(hold.single_key().is_some());
    }

    #[test]
    fn every_right_hand_modifier_can_be_bound() {
        for spec in ["RightControl", "RightAlt", "RightSuper"] {
            let hotkey = Hotkey::parse(spec).unwrap();
            assert_eq!(hotkey.spec, spec);
            assert!(hotkey.single_key().is_some(), "{spec} should be watched");
        }
    }

    #[test]
    fn a_binding_is_written_for_a_person() {
        let hold = Hotkey::parse(DEFAULT_HOLD).unwrap();
        assert!(!hold.display.is_empty());
        assert_ne!(
            hold.display, hold.spec,
            "the label should not be the specification"
        );
    }

    /// Chords were bindable until 1.2.4 and are not any more. A stored one must be
    /// refused rather than silently becoming something else.
    #[test]
    fn a_chord_is_no_longer_a_binding() {
        for spec in ["Ctrl+Alt+Space", "Ctrl+Shift+Space", "Alt+Space"] {
            assert!(Hotkey::parse(spec).is_err(), "{spec} should be refused");
        }
    }

    #[test]
    fn an_ordinary_key_is_not_a_binding() {
        // Holding a letter would fire mid-sentence; a function key has no second mode,
        // since only a watched key can be tapped twice.
        for spec in ["F9", "A", "Space", "Escape"] {
            assert!(Hotkey::parse(spec).is_err(), "{spec} should be refused");
        }
    }

    #[test]
    fn nonsense_is_refused_with_something_readable() {
        let error = Hotkey::parse("RightPinky").unwrap_err().to_string();
        assert!(
            error.contains("Right Ctrl"),
            "the message should say what can be bound, got {error}"
        );
    }

    #[test]
    fn an_empty_binding_is_refused() {
        assert!(Hotkey::parse("").is_err());
        assert!(Hotkey::parse("   ").is_err());
    }

    #[test]
    fn surrounding_space_is_ignored() {
        let hotkey = Hotkey::parse("  RightControl  ").unwrap();
        assert_eq!(hotkey.spec, "RightControl");
    }
}
