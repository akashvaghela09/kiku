//! Delivering a transcript to the user.
//!
//! Always to the clipboard, and then - unless the user turned it off - pasted into
//! whatever has focus. The clipboard copy happens first and unconditionally, so a
//! refused paste still leaves the text somewhere the user can reach it.

mod paste;
mod text;

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{Error, Result};

pub use text::prepare;

/// What happened to a transcript on its way to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// On the clipboard and pasted into the focused window.
    Pasted,
    /// On the clipboard only - either the user turned pasting off, or the platform
    /// refused to synthesise input.
    CopiedOnly,
}

/// Put `text` on the clipboard and, when `auto_paste` is set, paste it.
///
/// A failed paste is not an error: the text is already on the clipboard, so the user
/// can press paste themselves. The caller tells them which happened.
pub fn deliver(app: &AppHandle, text: &str, auto_paste: bool) -> Result<Delivery> {
    if text.is_empty() {
        return Ok(Delivery::CopiedOnly);
    }

    app.clipboard()
        .write_text(text.to_owned())
        .map_err(|error| Error::Clipboard(error.to_string()))?;

    if !auto_paste {
        return Ok(Delivery::CopiedOnly);
    }

    match paste::paste_into_focused_window(app) {
        Ok(()) => Ok(Delivery::Pasted),
        Err(error) => {
            // Password fields, some terminals and a few Electron apps refuse
            // synthesised input; on macOS this is also what a missing Accessibility
            // grant looks like. The clipboard copy already succeeded.
            tracing::warn!(%error, "could not paste; the text is on the clipboard");
            Ok(Delivery::CopiedOnly)
        }
    }
}
