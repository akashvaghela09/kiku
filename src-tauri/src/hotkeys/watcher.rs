//! Watching a single key that the operating system will not let us register.
//!
//! A bare modifier such as Right Ctrl cannot be a global shortcut: every platform's
//! hotkey API takes modifiers plus one *ordinary* key, and the Tauri plugin will not
//! even parse `ControlRight`. Such a key has to be observed instead.
//!
//! **Key state is polled, not hooked.** Reading whether a key is currently down —
//! `XQueryKeymap` on X11, `GetAsyncKeyState` on Windows, `CGEventSourceKeyState` on
//! macOS — is deliberately chosen over a low-level keyboard hook:
//!
//! * A hook intercepts every keystroke in the system. Polling reads state and consumes
//!   nothing, so Right Ctrl keeps working as Ctrl everywhere else.
//! * On Windows a hook means `WH_KEYBOARD_LL`, the pattern antivirus flags on unsigned
//!   binaries — and Kiku ships unsigned.
//! * On macOS a hook needs Input Monitoring, a second permission on top of the
//!   Accessibility grant pasting already requires. Polling reuses Accessibility.
//!
//! The cost is a poll loop. At 120 Hz a keymap read is microseconds, and the thread
//! sleeps the rest.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use device_query::{DeviceQuery, DeviceState, Keycode};

use super::tap::{Input, Outcome, TapMachine, Thresholds};

/// How often key state is sampled.
///
/// Fast enough that the delay before recording starts is imperceptible, slow enough
/// that the loop costs nothing measurable.
const POLL_INTERVAL: Duration = Duration::from_millis(8);

/// A key that can be watched but not registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleKey {
    RightControl,
    RightAlt,
    RightSuper,
}

impl SingleKey {
    /// The specification string this key is stored as.
    pub fn spec(self) -> &'static str {
        match self {
            Self::RightControl => "RightControl",
            Self::RightAlt => "RightAlt",
            Self::RightSuper => "RightSuper",
        }
    }

    pub fn parse(spec: &str) -> Option<Self> {
        match spec.trim() {
            "RightControl" => Some(Self::RightControl),
            "RightAlt" => Some(Self::RightAlt),
            "RightSuper" => Some(Self::RightSuper),
            _ => None,
        }
    }

    fn keycode(self) -> Keycode {
        match self {
            Self::RightControl => Keycode::RControl,
            Self::RightAlt => Keycode::RAlt,
            Self::RightSuper => Keycode::RMeta,
        }
    }

    /// How this key should be written for the user.
    pub fn display(self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            Self::RightControl => return "Right ⌃",
            Self::RightAlt => return "Right ⌥",
            Self::RightSuper => return "Right ⌘",
        }

        #[cfg(not(target_os = "macos"))]
        match self {
            Self::RightControl => "Right Ctrl",
            Self::RightAlt => "Right Alt",
            Self::RightSuper => "Right Win",
        }
    }
}

/// Keys whose presence means "the user is doing something else".
///
/// Deliberately not every key: the watched key's own left-hand twin is excluded,
/// because on X11 pressing the right-hand key reports the left one as well — an
/// artefact of how the modifier state is read, not a second key being pressed.
fn is_other_key(pressed: Keycode, watched: SingleKey) -> bool {
    if pressed == watched.keycode() {
        return false;
    }

    // The left-hand counterpart is reported alongside the right-hand key on X11, so
    // treating it as "another key" would cancel every recording immediately.
    let twin = match watched {
        SingleKey::RightControl => Keycode::LControl,
        SingleKey::RightAlt => Keycode::LAlt,
        SingleKey::RightSuper => Keycode::LMeta,
    };
    pressed != twin
}

/// Runs a poll loop and reports what the user meant.
pub struct KeyWatcher {
    running: Arc<AtomicBool>,
}

impl KeyWatcher {
    /// Start watching `key`, calling `on_outcome` when the recording should change.
    ///
    /// The callback runs on the watcher thread and must not block.
    pub fn start(
        key: SingleKey,
        thresholds: Thresholds,
        on_outcome: impl Fn(Outcome) + Send + 'static,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);

        thread::Builder::new()
            .name("kiku-keywatch".into())
            .spawn(move || watch(key, thresholds, &worker_running, on_outcome))
            .map(|_| ())
            .unwrap_or_else(|error| tracing::error!(%error, "could not start the key watcher"));

        Self { running }
    }
}

impl Drop for KeyWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn watch(
    key: SingleKey,
    thresholds: Thresholds,
    running: &AtomicBool,
    on_outcome: impl Fn(Outcome) + Send + 'static,
) {
    let device = DeviceState::new();
    let mut machine = TapMachine::new(thresholds);
    let mut was_down = false;

    tracing::info!(key = key.spec(), "watching a single-key shortcut");

    while running.load(Ordering::Relaxed) {
        let pressed = device.get_keys();
        let now = Instant::now();

        let is_down = pressed.contains(&key.keycode());
        let others = pressed.iter().any(|&code| is_other_key(code, key));

        // Edges first, so a hold is one Down rather than a stream of them.
        let input = match (was_down, is_down) {
            (false, true) => Input::Down,
            (true, false) => Input::Up,
            (true, true) if others => Input::OtherKey,
            _ => Input::Tick,
        };
        was_down = is_down;

        if let Some(outcome) = machine.advance(input, now) {
            on_outcome(outcome);
        }

        thread::sleep(POLL_INTERVAL);
    }

    tracing::debug!(key = key.spec(), "stopped watching");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_round_trips_through_its_specification() {
        for key in [
            SingleKey::RightControl,
            SingleKey::RightAlt,
            SingleKey::RightSuper,
        ] {
            assert_eq!(SingleKey::parse(key.spec()), Some(key));
        }
    }

    #[test]
    fn an_unknown_specification_is_not_a_single_key() {
        for spec in ["Ctrl+Shift+Space", "F9", "", "RightPinky"] {
            assert_eq!(SingleKey::parse(spec), None);
        }
    }

    #[test]
    fn the_watched_key_is_not_treated_as_another_key() {
        assert!(!is_other_key(Keycode::RControl, SingleKey::RightControl));
    }

    #[test]
    fn the_left_hand_twin_is_not_treated_as_another_key() {
        // Verified on X11: pressing Right Ctrl reports [LControl, RControl]. Counting
        // the left one as "another key" would cancel every recording on the next poll.
        assert!(!is_other_key(Keycode::LControl, SingleKey::RightControl));
        assert!(!is_other_key(Keycode::LAlt, SingleKey::RightAlt));
        assert!(!is_other_key(Keycode::LMeta, SingleKey::RightSuper));
    }

    #[test]
    fn an_ordinary_key_does_count_as_another_key() {
        // This is what makes Right Ctrl + C copy without also dictating.
        assert!(is_other_key(Keycode::C, SingleKey::RightControl));
        assert!(is_other_key(Keycode::Space, SingleKey::RightControl));
    }

    #[test]
    fn a_different_modifier_counts_as_another_key() {
        assert!(is_other_key(Keycode::LShift, SingleKey::RightControl));
        assert!(is_other_key(Keycode::RAlt, SingleKey::RightControl));
    }

    #[test]
    fn polling_is_frequent_enough_to_feel_immediate() {
        assert!(POLL_INTERVAL <= Duration::from_millis(16));
    }
}
