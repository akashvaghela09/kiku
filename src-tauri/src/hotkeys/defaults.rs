//! Every shortcut Kiku ships, in one place.
//!
//! Changing what a fresh install is bound to means editing this file and nothing else.
//! The specification is the portable form - `RightControl`, `RightAlt` - which
//! [`super::binding::Hotkey::parse`] validates and renders for the platform it is
//! running on, so `RightAlt` reaches a Mac user as `Right ⌥` without a second
//! definition existing anywhere.
//!
//! The frontend keeps a matching copy in `src/features/settings/shortcuts.ts` for the
//! preset button in Settings; the comment there points back here.
//!
//! A stored binding wins over this. It is what a fresh install gets, and what the
//! preset in Settings restores.

/// The dictation key.
///
/// One key carries both modes. Holding it records for as long as it is held; tapping
/// it twice records hands-free until it is tapped again. That second gesture is
/// [`super::tap::TapMachine`]'s doing and needs no binding of its own, which is why
/// there is exactly one shortcut here.
///
/// It is a bare modifier, which no operating system will register as a global
/// shortcut, since they all want modifiers plus an ordinary key. So it is watched
/// instead; see [`super::watcher`]. Watching is what lets the key keep working as an
/// ordinary modifier everywhere else, and what makes pressing any other key while
/// holding it cancel rather than dictate.
///
/// Mac keyboards have no right Control key, so macOS watches Right Option instead.
#[cfg(target_os = "macos")]
pub const DEFAULT_HOLD: &str = "RightAlt";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_HOLD: &str = "RightControl";

/// The hands-free key, double-tapped.
///
/// The same key as [`DEFAULT_HOLD`] by default, because one key doing both is the
/// simplest thing that works: hold it to talk, tap it twice to keep listening. They
/// are separate settings so that anyone who wants a key each can have one.
pub const DEFAULT_HANDS_FREE: &str = DEFAULT_HOLD;

/// Abandon a recording in progress.
///
/// Registered only while a session is running and released immediately afterwards:
/// holding it globally would break Escape everywhere else on the system. It is not
/// bindable, so it is not a [`super::binding::Hotkey`].
pub const CANCEL: &str = "Escape";
