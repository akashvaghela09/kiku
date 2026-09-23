//! What the dictation key is bound to.
//!
//! One binding, because one key carries both modes: holding it records while it is
//! held, and tapping it twice records hands-free until the next tap. The second
//! gesture is [`super::tap::TapMachine`]'s work, not a second shortcut, so there is
//! nothing here to bind it to.
//!
//! There is no interpreter beside it either. A bare modifier cannot be registered as a
//! global shortcut on any platform, so the binding is always watched rather than
//! delivered by the operating system, and the watcher reports what the user meant
//! directly.

use serde::{Deserialize, Serialize};
use specta::Type;

use super::binding::Hotkey;
use super::defaults::DEFAULT_HOLD;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    pub hold: Hotkey,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            hold: Hotkey::parse(DEFAULT_HOLD).expect("the default dictation key must parse"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_watched_key() {
        let bindings = HotkeyBindings::default();
        assert!(
            bindings.hold.single_key().is_some(),
            "the dictation key is watched, never registered"
        );
    }

    #[test]
    fn the_default_round_trips_through_its_specification() {
        let bindings = HotkeyBindings::default();
        assert_eq!(bindings.hold.spec, DEFAULT_HOLD);
    }

    #[test]
    fn the_default_is_written_for_a_person() {
        let bindings = HotkeyBindings::default();
        assert!(!bindings.hold.display.is_empty());
        assert_ne!(
            bindings.hold.display, bindings.hold.spec,
            "the label should not be the specification"
        );
    }
}
