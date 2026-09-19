//! Capturing the microphone and preparing audio for the recogniser.
//!
//! Everything leaving this module is 16 kHz mono f32, which is the only format the
//! rest of Kiku knows about.

mod capture;
mod device;
mod level;
mod resample;

pub use capture::{Capture, Recording, MAX_RECORDING};
pub use device::{list as list_microphones, MicrophoneInfo, TARGET_SAMPLE_RATE};
pub use level::Level;
