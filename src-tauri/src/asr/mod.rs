//! Speech recognition.
//!
//! Audio goes in as 16 kHz mono f32; text comes out punctuated and capitalised by the
//! model itself. One warm engine serves the whole process — see `service`.

mod engine;
mod parakeet;
mod service;

pub use engine::{Engine, Transcript};
pub use parakeet::{ModelFiles, ParakeetEngine};
pub use service::{AsrService, EngineStatus};
