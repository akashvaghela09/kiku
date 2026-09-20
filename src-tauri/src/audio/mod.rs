//! Audio coming **in**: the microphone, and preparing what it hears for the recogniser.
//!
//! Everything leaving this module is 16 kHz mono f32, which is the only format the
//! rest of Kiku knows about. Sound going **out** — the cues the user hears — is
//! `crate::feedback`, which shares nothing with this but the cpal dependency.

mod capture;
mod device;
mod level;
mod resample;
mod speech;

pub use capture::{Capture, Recording, MAX_RECORDING};
pub use device::{list as list_microphones, MicrophoneInfo, TARGET_SAMPLE_RATE};
pub use level::Level;
pub use speech::SpeechLevel;
