//! Watching a single key that the operating system will not let us register.
//!
//! A bare modifier such as Right Ctrl cannot be a global shortcut: every platform's
//! hotkey API takes modifiers plus one *ordinary* key, and the Tauri plugin will not
//! even parse `ControlRight`. Such a key has to be observed instead.
//!
//! **Key state is polled, not hooked.** Reading whether a key is currently down -
//! `XQueryKeymap` on X11, `GetAsyncKeyState` on Windows, `CGEventSourceKeyState` on
//! macOS - is deliberately chosen over a low-level keyboard hook:
//!
//! * A hook intercepts every keystroke in the system. Polling reads state and consumes
//!   nothing, so Right Ctrl keeps working as Ctrl everywhere else.
//! * On Windows a hook means `WH_KEYBOARD_LL`, the pattern antivirus flags on unsigned
//!   binaries - and Kiku ships unsigned.
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

    /// The `device_query` keycode this key is reported as.
    ///
    /// Platform-specific, because the crate reports Mac modifiers under their Mac
    /// names: its macOS backend maps Right Option to `ROption` and Right Command to
    /// `RCommand`, and emits `RAlt` or `RMeta` nowhere at all. Those are separate
    /// variants of the same enum rather than aliases, so watching for `RAlt` on a Mac
    /// watches for a key the backend never reports - which is exactly what made the
    /// shipped macOS default, Right Option, impossible to press. Right Control was
    /// unaffected, and Right Control is what Linux defaults to, so nothing on the
    /// development machine could see it.
    fn keycode(self) -> Keycode {
        #[cfg(target_os = "macos")]
        {
            match self {
                Self::RightControl => Keycode::RControl,
                Self::RightAlt => Keycode::ROption,
                Self::RightSuper => Keycode::RCommand,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            match self {
                Self::RightControl => Keycode::RControl,
                Self::RightAlt => Keycode::RAlt,
                Self::RightSuper => Keycode::RMeta,
            }
        }
    }

    /// The left-hand counterpart of this key, which must not count as "another key".
    ///
    /// Same naming problem as [`Self::keycode`], with one extra wrinkle: the crate's
    /// macOS backend calls the *left* command key `Command` and the right one
    /// `RCommand`, so the pair here is not the `L`/`R` symmetry every other entry has.
    fn twin(self) -> Keycode {
        #[cfg(target_os = "macos")]
        {
            match self {
                Self::RightControl => Keycode::LControl,
                Self::RightAlt => Keycode::LOption,
                Self::RightSuper => Keycode::Command,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            match self {
                Self::RightControl => Keycode::LControl,
                Self::RightAlt => Keycode::LAlt,
                Self::RightSuper => Keycode::LMeta,
            }
        }
    }

    /// How this key should be written for the user.
    pub fn display(self) -> &'static str {
        // Each branch is a block expression rather than a bare match with `return`
        // statements: the latter compiles, but clippy rejects it on the platform where
        // the branch is live, which is exactly the platform it is never tested on
        // locally.
        #[cfg(target_os = "macos")]
        {
            match self {
                Self::RightControl => "Right ⌃",
                Self::RightAlt => "Right ⌥",
                Self::RightSuper => "Right ⌘",
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            match self {
                Self::RightControl => "Right Ctrl",
                Self::RightAlt => "Right Alt",
                Self::RightSuper => "Right Win",
            }
        }
    }
}

/// Keys whose presence means "the user is doing something else".
///
/// Deliberately not every key: the watched key's own left-hand twin is excluded,
/// because on X11 pressing the right-hand key reports the left one as well - an
/// artefact of how the modifier state is read, not a second key being pressed.
fn is_other_key(pressed: Keycode, watched: SingleKey) -> bool {
    if pressed == watched.keycode() {
        return false;
    }

    // The left-hand counterpart is reported alongside the right-hand key on X11, so
    // treating it as "another key" would cancel every recording immediately.
    pressed != watched.twin()
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

/// How long to wait between checks for the Accessibility grant.
///
/// Slow on purpose. Nothing can be watched until the user has been through System
/// Settings, which takes as long as it takes.
const ACCESS_RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// Block until the operating system will let us read key state, or the watcher stops.
///
/// `DeviceState::new()` is not used anywhere in Kiku: on macOS it is
/// `assert!(has_accessibility(), ..)`, which panics this thread when the grant is
/// missing. A panic here is silent - the thread dies, the process carries on, and the
/// hotkey simply never fires again for the rest of the session, including after the
/// user grants the permission it just asked for. `checked_new` reports the same
/// condition as `None` and lets us wait for it instead.
///
/// The prompt is shown once. `application_is_trusted_with_prompt` re-opens that dialog
/// every time it is called while the grant is missing, so polling with it would put a
/// system dialog on screen once a second; the silent check is what the loop uses.
fn await_device(running: &AtomicBool) -> Option<DeviceState> {
    if let Some(device) = DeviceState::checked_new() {
        return Some(device);
    }

    tracing::warn!("waiting for Accessibility before watching for the dictation key");

    #[cfg(target_os = "macos")]
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt();

    while running.load(Ordering::Relaxed) {
        thread::sleep(ACCESS_RETRY_INTERVAL);

        #[cfg(target_os = "macos")]
        if !macos_accessibility_client::accessibility::application_is_trusted() {
            continue;
        }

        if let Some(device) = DeviceState::checked_new() {
            tracing::info!("Accessibility granted; the dictation key is now watched");
            return Some(device);
        }
    }

    None
}

fn watch(
    key: SingleKey,
    thresholds: Thresholds,
    running: &AtomicBool,
    on_outcome: impl Fn(Outcome) + Send + 'static,
) {
    let Some(device) = await_device(running) else {
        return;
    };
    let mut machine = TapMachine::new(thresholds);
    let mut was_down = false;
    let mut last_seen: Vec<Keycode> = Vec::new();

    tracing::info!(key = key.spec(), "watching a single-key shortcut");

    while running.load(Ordering::Relaxed) {
        let pressed = device.get_keys();
        let now = Instant::now();

        let is_down = pressed.contains(&key.keycode());
        let others = pressed.iter().any(|&code| is_other_key(code, key));

        // What the platform actually reported, whenever it changes. The watched key
        // being absent from a set that is plainly not empty is the signature of a
        // keycode that does not match what this platform emits, which is a fault that
        // is otherwise invisible: the key simply does nothing, with nothing logged.
        if pressed != last_seen {
            tracing::debug!(?pressed, watching = ?key.keycode(), is_down, "keys changed");
            last_seen.clone_from(&pressed);
        }

        // Edges first, so a hold is one Down rather than a stream of them.
        let input = match (was_down, is_down) {
            (false, true) => Input::Down,
            (true, false) => Input::Up,
            (true, true) if others => Input::OtherKey,
            _ => Input::Tick,
        };
        was_down = is_down;

        if let Some(outcome) = machine.advance(input, now) {
            tracing::debug!(?input, ?outcome, "tap machine");
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
        // Asked through `twin` rather than by naming keycodes, because which variants
        // those are differs by platform.
        for key in [
            SingleKey::RightControl,
            SingleKey::RightAlt,
            SingleKey::RightSuper,
        ] {
            assert!(!is_other_key(key.twin(), key));
        }
    }

    #[test]
    fn a_watched_key_and_its_twin_are_different_keys() {
        for key in [
            SingleKey::RightControl,
            SingleKey::RightAlt,
            SingleKey::RightSuper,
        ] {
            assert_ne!(
                key.keycode(),
                key.twin(),
                "{key:?} would cancel itself on every poll"
            );
        }
    }

    /// The bug that made the shipped macOS default impossible to press.
    ///
    /// `device_query`'s macOS backend maps Right Option to `ROption` and Right Command
    /// to `RCommand`. `RAlt` and `RMeta` are separate variants that only Linux and
    /// Windows ever emit, so watching for one on a Mac waits forever.
    #[cfg(target_os = "macos")]
    #[test]
    fn mac_modifiers_are_watched_under_their_mac_keycodes() {
        assert_eq!(SingleKey::RightAlt.keycode(), Keycode::ROption);
        assert_eq!(SingleKey::RightSuper.keycode(), Keycode::RCommand);
        assert_eq!(SingleKey::RightAlt.twin(), Keycode::LOption);

        for key in [SingleKey::RightAlt, SingleKey::RightSuper] {
            assert!(
                !matches!(key.keycode(), Keycode::RAlt | Keycode::RMeta),
                "{key:?} is watched for a keycode macOS never reports"
            );
        }
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
