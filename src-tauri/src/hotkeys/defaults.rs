//! Every shortcut Kiku ships, in one place.
//!
//! Changing what a fresh install is bound to means editing this file and nothing else.
//! The specifications are the portable form - `Alt+Space`, `RightControl` - which
//! [`super::binding::Hotkey::parse`] validates and renders for the platform it is
//! running on, so `Alt+Space` reaches a Mac user as `⌥Space` without a second
//! definition existing anywhere.
//!
//! The frontend keeps a matching list in `src/features/settings/shortcuts.ts` for the
//! preset buttons in Settings. There is no IPC for it because presets are offered
//! before any binding is applied, and a round trip to draw three buttons is not worth
//! the machinery; the comment in that file points back here.
//!
//! A stored binding always wins over anything here. These are what a fresh install
//! gets, and what the presets in Settings restore.

/// Hold to talk. Releasing the key ends the recording.
///
/// A single key, because one key is far easier to *hold* than a chord, and no operating
/// system claims a bare right-hand modifier on its own. It cannot be registered as a
/// global shortcut - no platform accepts a bare modifier - so it is watched instead;
/// see [`super::watcher`].
///
/// Mac keyboards have no right Control key, so macOS watches Right Option instead.
#[cfg(target_os = "macos")]
pub const DEFAULT_HOLD: &str = "RightAlt";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_HOLD: &str = "RightControl";

/// Press once to start, again to stop - the chord route into hands-free recording.
///
/// The *primary* route needs nothing from this file: tapping [`DEFAULT_HOLD`] twice
/// latches hands-free recording, and a third tap ends it. That gesture is the same on
/// every platform because it is decided by [`super::tap::TapMachine`] from the watched
/// key alone - double-tapping Right Option on a Mac and Right Ctrl everywhere else is
/// one implementation, not three. Its timings are
/// [`super::tap::DEFAULT_HOLD_THRESHOLD`] and
/// [`super::tap::DEFAULT_DOUBLE_TAP_WINDOW`].
///
/// This chord exists for keyboards whose right-hand modifier is awkward, and for anyone
/// who would rather press two keys once than one key twice.
///
/// Not `Alt+Space`, which reads better and is genuinely contested: Windows, GNOME and
/// Cinnamon bind it to `activate-window-menu`, so registering it there either fails or
/// takes the window menu away from the desktop.
pub const DEFAULT_TOGGLE: &str = "Ctrl+Alt+Space";

/// Fallback for keyboards without a usable right-hand modifier, and for anyone who
/// would rather have a chord. Unclaimed at the OS level on all three platforms.
pub const FALLBACK_HOLD: &str = "Ctrl+Shift+Space";

/// The toggle offered alongside [`FALLBACK_HOLD`] as a matched chord pair.
///
/// The same specification as [`DEFAULT_TOGGLE`]: the chord preset changes the hold key
/// and leaves the toggle where it is, because there is nothing wrong with it.
pub const FALLBACK_TOGGLE: &str = "Ctrl+Alt+Space";

/// A single-key pair for keyboards whose right-hand modifiers are awkward.
pub const FUNCTION_HOLD: &str = "F9";
pub const FUNCTION_TOGGLE: &str = "F10";

/// Abandon a recording in progress.
///
/// Registered only while a session is running and released immediately afterwards:
/// holding it globally would break Escape everywhere else on the system.
pub const CANCEL: &str = "Escape";
