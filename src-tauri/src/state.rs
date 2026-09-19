//! Everything the application owns for its whole lifetime.
//!
//! Held in Tauri's managed state and reached from commands via `State<AppState>`.
//! Kept deliberately small: it holds services, never UI state, which lives in the
//! frontend where it belongs.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::asr::AsrService;
use crate::dictation::Dictation;
use crate::hotkeys::HotkeyBindings;
use crate::models::ModelStore;

pub struct AppState {
    pub models: ModelStore,
    pub asr: AsrService,
    pub dictation: Dictation,
    pub hotkeys: HotkeyState,
    /// Application data directory — models, database and settings all live under it.
    pub data_dir: PathBuf,
    /// Device id of the chosen microphone; `None` means follow the system default.
    microphone: Mutex<Option<String>>,
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
