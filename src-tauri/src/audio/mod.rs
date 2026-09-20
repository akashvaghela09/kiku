//! Audio coming **in**: the microphone, and preparing what it hears for the recogniser.
//!
//! Everything leaving this module is 16 kHz mono f32, which is the only format the
//! rest of Kiku knows about. Sound going **out** - the cues the user hears - is
//! `crate::feedback`, which shares nothing with this but the cpal dependency.

mod capture;
mod device;
mod level;
mod resample;
// PipeWire source enumeration shells out to `pactl`, so it exists only where that
// does. Compiling it elsewhere left three functions nobody calls, which `-D warnings`
// correctly rejects on macOS and Windows.
#[cfg(target_os = "linux")]
mod sources;
mod speech;

pub use capture::{Capture, Recording, MAX_RECORDING};
pub use device::{list as list_microphones, MicrophoneInfo, TARGET_SAMPLE_RATE};
pub use level::Level;
#[cfg(target_os = "linux")]
pub use sources::Source;
pub use speech::SpeechLevel;
