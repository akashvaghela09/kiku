//! Everything the application owns for its whole lifetime.
//!
//! Held in Tauri's managed state and reached from commands via `State<AppState>`.
//! Kept deliberately small: it holds services, never UI state, which lives in the
//! frontend where it belongs.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::asr::AsrService;
use crate::dictation::Dictation;
use crate::error::{Error, Result};
use crate::history::History;
use crate::hotkeys::{HotkeyBindings, KeyWatcher};
use crate::models::ModelStore;
use crate::settings::Settings;

/// User preferences that affect how a transcript is delivered.
///
/// Persisted by [`crate::settings`], which is also what seeds them at launch.
/// Which palette the main window uses.
///
/// The overlay is deliberately not covered by this. It floats over an arbitrary
/// application rather than over a Kiku window, so matching the desktop theme would be
/// a promise about a surface the app does not own; it is always dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the desktop, and keep following it when it changes.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    /// Light, dark, or whatever the desktop is set to.
    pub theme: Theme,
    /// Paste into the focused window after copying. The copy always happens.
    pub auto_paste: bool,
    /// Append one space, so consecutive dictations do not run together.
    pub trailing_space: bool,
    /// Play the short tones that mark listening starting and finishing.
    pub sounds: bool,
    /// Keep a record of each dictation. On by default; turning it off leaves the
    /// existing history alone and simply stops adding to it.
    pub record_history: bool,
    /// Delete history entries older than this many days. `None` keeps everything.
    pub retention_days: Option<u32>,
    /// Ask GitHub once a day whether a newer release exists. Never downloads.
    pub check_for_updates: bool,
    /// Which model to load. `None` means "whichever is installed", which is what a
    /// fresh install and a single-model setup both want.
    pub model_id: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            auto_paste: true,
            trailing_space: true,
            sounds: true,
            record_history: true,
            retention_days: None,
            check_for_updates: true,
            model_id: None,
        }
    }
}

pub struct AppState {
    pub models: ModelStore,
    pub asr: AsrService,
    pub dictation: Dictation,
    pub hotkeys: HotkeyState,
    /// Application data directory - models, database and settings all live under it.
    pub data_dir: PathBuf,
    /// Device id of the chosen microphone; `None` means follow the system default.
    microphone: Mutex<Option<String>>,
    preferences: Mutex<Preferences>,
    /// `None` when the database could not be opened. Dictation still works without
    /// history - refusing to launch over a history problem would be the worse failure.
    history: Option<History>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let history = match History::open(&data_dir.join("history.sqlite3")) {
            Ok(history) => Some(history),
            Err(error) => {
                tracing::error!(%error, "history is unavailable; dictation will still work");
                None
            }
        };

        let stored = Settings::load(&data_dir);

        let hotkeys = HotkeyState::default();
        // Adopted before anything is registered, so `setup` can read back what the
        // user last chose and install that instead of the shipped default.
        hotkeys.adopt(stored.hotkeys);

        Self {
            models: ModelStore::new(&data_dir),
            asr: AsrService::new(),
            dictation: Dictation::new(),
            hotkeys,
            data_dir,
            microphone: Mutex::new(stored.microphone),
            preferences: Mutex::new(stored.preferences),
            history,
        }
    }

    /// Write the current settings to disk.
    ///
    /// Called by every setter rather than on a timer or at shutdown: a preference
    /// changed and then lost to a crash - or to a quit the application never sees,
    /// which on macOS is most of them - is indistinguishable from one that was never
    /// saved. Failing to write is logged and otherwise ignored, because a settings
    /// file that cannot be written is not a reason to refuse the setting.
    pub fn save(&self) {
        let settings = Settings {
            preferences: self.preferences(),
            hotkeys: self.hotkeys.current(),
            microphone: self.microphone(),
        };

        if let Err(error) = settings.save(&self.data_dir) {
            tracing::warn!(%error, "settings could not be saved");
        }
    }

    pub fn history(&self) -> Result<&History> {
        self.history
            .as_ref()
            .ok_or_else(|| Error::Database("The history database could not be opened.".into()))
    }

    pub fn preferences(&self) -> Preferences {
        self.preferences
            .lock()
            .map(|preferences| preferences.clone())
            .unwrap_or_default()
    }

    pub fn set_preferences(&self, next: Preferences) {
        // The guard is dropped before `save`, which reads the same mutex back. A
        // std::sync::Mutex is not reentrant, so holding it across that call would
        // deadlock the command thread.
        if let Ok(mut current) = self.preferences.lock() {
            *current = next;
        }
        self.save();
    }

    pub fn microphone(&self) -> Option<String> {
        self.microphone.lock().ok().and_then(|id| id.clone())
    }

    pub fn set_microphone(&self, id: Option<String>) {
        if let Ok(mut current) = self.microphone.lock() {
            *current = id;
        }
        self.save();
    }
}

/// The bindings currently in force.
///
/// Also owns the key watcher, because a bare modifier cannot be registered with the
/// operating system and has to be polled instead. Keeping the watcher here is what
/// keeps it alive - dropping it stops the poll thread.
#[derive(Default)]
pub struct HotkeyState {
    bindings: Mutex<Option<HotkeyBindings>>,
    watcher: Mutex<Vec<KeyWatcher>>,
}

impl HotkeyState {
    pub fn current(&self) -> HotkeyBindings {
        self.bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.clone())
            .unwrap_or_default()
    }

    /// Record what was successfully registered.
    pub fn adopt(&self, bindings: HotkeyBindings) {
        if let Ok(mut current) = self.bindings.lock() {
            *current = Some(bindings);
        }
    }

    /// Install a key watcher, stopping whichever one was running.
    /// Install the watchers, stopping whichever were running.
    ///
    /// A list because the two modes can be bound to different keys, and each key needs
    /// a thread of its own. Bound to the same key - which is what a fresh install does
    /// - there is one watcher answering to both gestures.
    pub fn watch(&self, watchers: Vec<KeyWatcher>) {
        if let Ok(mut current) = self.watcher.lock() {
            // Dropping the previous watchers stops their threads.
            *current = watchers;
        }
    }
}
