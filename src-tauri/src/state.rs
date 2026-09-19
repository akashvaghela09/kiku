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
use crate::hotkeys::HotkeyBindings;
use crate::models::ModelStore;

/// User preferences that affect how a transcript is delivered.
///
/// Persisted to disk in chunk 10; held here so the rest of the application can already
/// read them from one place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    /// Paste into the focused window after copying. The copy always happens.
    pub auto_paste: bool,
    /// Append one space, so consecutive dictations do not run together.
    pub trailing_space: bool,
    /// Play the short tones that mark listening starting and finishing.
    pub sounds: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_paste: true,
            trailing_space: true,
            sounds: true,
        }
    }
}

pub struct AppState {
    pub models: ModelStore,
    pub asr: AsrService,
    pub dictation: Dictation,
    pub hotkeys: HotkeyState,
    /// Application data directory — models, database and settings all live under it.
    pub data_dir: PathBuf,
    /// Device id of the chosen microphone; `None` means follow the system default.
    microphone: Mutex<Option<String>>,
    preferences: Mutex<Preferences>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            models: ModelStore::new(&data_dir),
            asr: AsrService::new(),
            dictation: Dictation::new(),
            hotkeys: HotkeyState::default(),
            data_dir,
            microphone: Mutex::new(None),
            preferences: Mutex::new(Preferences::default()),
        }
    }

    pub fn preferences(&self) -> Preferences {
        self.preferences
            .lock()
            .map(|preferences| preferences.clone())
            .unwrap_or_default()
    }

    pub fn set_preferences(&self, next: Preferences) {
        if let Ok(mut current) = self.preferences.lock() {
            *current = next;
        }
    }

    pub fn microphone(&self) -> Option<String> {
        self.microphone.lock().ok().and_then(|id| id.clone())
    }

    pub fn set_microphone(&self, id: Option<String>) {
        if let Ok(mut current) = self.microphone.lock() {
            *current = id;
        }
    }
}

/// The bindings currently registered with the operating system.
#[derive(Default)]
pub struct HotkeyState {
    bindings: Mutex<Option<HotkeyBindings>>,
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
}
