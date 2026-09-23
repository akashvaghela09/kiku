//! Global hotkeys for starting and stopping dictation.

mod binding;
pub mod defaults;
mod manager;
mod tap;
mod watcher;

pub use binding::Hotkey;
pub use defaults::DEFAULT_HOLD;
pub use manager::HotkeyBindings;
pub use tap::{Outcome as TapOutcome, Thresholds};
pub use watcher::{KeyWatcher, SingleKey};
