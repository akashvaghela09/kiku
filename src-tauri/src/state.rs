//! Everything the application owns for its whole lifetime.
//!
//! Held in Tauri's managed state and reached from commands via `State<AppState>`.
//! Kept deliberately small: it holds services, never UI state, which lives in the
//! frontend where it belongs.

use std::path::PathBuf;

use crate::asr::AsrService;
use crate::models::ModelStore;

pub struct AppState {
    pub models: ModelStore,
    pub asr: AsrService,
    /// Application data directory — models, database and settings all live under it.
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            models: ModelStore::new(&data_dir),
            asr: AsrService::new(),
            data_dir,
        }
    }
}
