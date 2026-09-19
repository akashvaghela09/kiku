//! What a speech recogniser is, from the rest of Kiku's point of view.
//!
//! One trait, one implementation. The trait exists because Kiku ships a single model
//! today but the scope explicitly leaves the door open to a second (a lighter Parakeet
//! for old hardware, or Whisper if multilingual ever enters scope). It costs a handful
//! of lines and keeps that door from becoming surgery.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    /// Recognised text, already punctuated and capitalised by the model.
    pub text: String,
    /// Length of the audio that produced it.
    pub audio_ms: u32,
    /// Wall-clock time the decode took.
    pub decode_ms: u32,
}

impl Transcript {
    /// Real-time factor. Below 1.0 is faster than real time; 0.05 is twenty times
    /// faster. Recorded per dictation so Settings can show a measured latency for this
    /// machine rather than a number from a benchmark on someone else's.
    pub fn real_time_factor(&self) -> f32 {
        if self.audio_ms == 0 {
            return f32::NAN;
        }
        self.decode_ms as f32 / self.audio_ms as f32
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

pub trait Engine: Send {
    /// Transcribe 16 kHz mono f32 audio.
    fn transcribe(&mut self, samples: &[f32]) -> Result<Transcript>;

    /// Short identifier of the loaded model, for logs and the About screen.
    fn model_id(&self) -> &str;
}

/// Helper shared by implementations so the timing fields are populated one way only.
pub(crate) fn transcript(text: String, samples: usize, decode: Duration) -> Transcript {
    let audio_ms = (samples as f64 / f64::from(crate::audio::TARGET_SAMPLE_RATE) * 1000.0) as u32;
    Transcript {
        text: text.trim().to_owned(),
        audio_ms,
        decode_ms: decode.as_millis() as u32,
    }
}
