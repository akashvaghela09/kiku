//! Global hotkeys for starting and stopping dictation.

mod binding;
mod manager;

pub use binding::{Hotkey, DEFAULT_HOLD, DEFAULT_TOGGLE};
pub use manager::{HotkeyAction, HotkeyBindings, HotkeyManager, Interpreter};
