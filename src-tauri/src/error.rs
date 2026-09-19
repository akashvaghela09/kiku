//! Errors, and the single shape they take when they cross to the frontend.
//!
//! `Error` is the rich internal enum. `ErrorPayload` is what the UI receives: a stable
//! machine-readable `kind` for branching, plus a sentence already fit to display.
//! Commands return `Result<T, ErrorPayload>` so `?` converts on the way out and no
//! hand-written `specta` implementation is needed.
//!
//! Variants describe what the *user* can do next, not which Rust call failed. That is
//! why "the microphone is in use by another application" is a variant and "io error"
//! is not.

use serde::Serialize;
use specta::Type;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The speech model isn't installed yet.")]
    ModelMissing,

    #[error("The speech model couldn't be loaded: {0}")]
    ModelLoad(String),

    #[error("No microphone is available.")]
    NoMicrophone,

    #[error("The microphone couldn't be opened: {0}")]
    Microphone(String),

    #[error("{0} is already in use by another application.")]
    HotkeyTaken(String),

    #[error("Couldn't reach the clipboard: {0}")]
    Clipboard(String),

    #[error("Couldn't save to the history database: {0}")]
    Database(String),

    #[error("{0}")]
    Internal(String),
}

impl Error {
    /// Stable identifier the frontend branches on. Never shown to a user, so these
    /// strings must not change once a release has shipped.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ModelMissing => "modelMissing",
            Self::ModelLoad(_) => "modelLoad",
            Self::NoMicrophone => "noMicrophone",
            Self::Microphone(_) => "microphone",
            Self::HotkeyTaken(_) => "hotkeyTaken",
            Self::Clipboard(_) => "clipboard",
            Self::Database(_) => "database",
            Self::Internal(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub kind: String,
    pub message: String,
}

impl<E: Into<Error>> From<E> for ErrorPayload {
    fn from(error: E) -> Self {
        let error = error.into();
        Self {
            kind: error.kind().to_owned(),
            message: error.to_string(),
        }
    }
}

/// Return type for every `#[tauri::command]`.
pub type Result<T> = std::result::Result<T, ErrorPayload>;
