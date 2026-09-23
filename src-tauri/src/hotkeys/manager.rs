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
use super::defaults::{DEFAULT_HANDS_FREE, DEFAULT_HOLD};
use super::tap::Gestures;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    /// Held down to record for as long as it is held.
    pub hold: Hotkey,
    /// Tapped twice to record hands-free until it is tapped again.
    pub hands_free: Hotkey,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            hold: Hotkey::parse(DEFAULT_HOLD).expect("the default dictation key must parse"),
            hands_free: Hotkey::parse(DEFAULT_HANDS_FREE)
                .expect("the default hands-free key must parse"),
        }
    }
}

impl HotkeyBindings {
    /// Which gestures `key` should answer to, or `None` if it is bound to neither.
    ///
    /// One key bound to both modes answers to both; two keys each answer to one. That
    /// separation is the whole point of having two settings - a hands-free key that
    /// also recorded while held would be a second hold key with extra steps.
    pub fn gestures_for(&self, key: &Hotkey) -> Option<Gestures> {
        let holds = self.hold.spec == key.spec;
        let latches = self.hands_free.spec == key.spec;

        match (holds, latches) {
            (true, true) => Some(Gestures::Both),
            (true, false) => Some(Gestures::Hold),
            (false, true) => Some(Gestures::HandsFree),
            (false, false) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::tap::Gestures;

    #[test]
    fn both_defaults_are_watched_keys() {
        let bindings = HotkeyBindings::default();
        assert!(bindings.hold.single_key().is_some());
        assert!(bindings.hands_free.single_key().is_some());
    }

    #[test]
    fn one_key_carries_both_modes_out_of_the_box() {
        let bindings = HotkeyBindings::default();
        assert_eq!(
            bindings.hold.spec, bindings.hands_free.spec,
            "a fresh install should need one key, not two"
        );
        assert_eq!(bindings.gestures_for(&bindings.hold), Some(Gestures::Both));
    }

    #[test]
    fn separate_keys_each_answer_to_their_own_gesture() {
        let bindings = HotkeyBindings {
            hold: Hotkey::parse("RightControl").unwrap(),
            hands_free: Hotkey::parse("RightAlt").unwrap(),
        };

        assert_eq!(bindings.gestures_for(&bindings.hold), Some(Gestures::Hold));
        assert_eq!(
            bindings.gestures_for(&bindings.hands_free),
            Some(Gestures::HandsFree)
        );
    }

    #[test]
    fn a_key_bound_to_neither_answers_to_nothing() {
        let bindings = HotkeyBindings::default();
        let stranger = Hotkey::parse("RightSuper").unwrap();
        assert_eq!(bindings.gestures_for(&stranger), None);
    }
}
