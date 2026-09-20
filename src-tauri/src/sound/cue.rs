//! The feedback cues and where their audio comes from.
//!
//! All three are 48 kHz mono WAV files in `assets/sounds/`, embedded in the binary
//! with `include_bytes!`. Embedding rather than shipping files alongside means there
//! is no path to resolve, no difference between running from `cargo` and running from
//! an installed bundle, and no way for a cue to go missing.
//!
//! Two are recordings. The third, the failure cue, is generated — but generated
//! *once*, by `cargo test render_error_cue -- --ignored`, and committed like the
//! others. Keeping it a file rather than rendering it at runtime means every cue loads
//! by the same path, and it can be listened to without running the application.

use std::io::Cursor;
use std::sync::OnceLock;

/// Every cue is 48 kHz mono, so playback never has to branch on the source.
pub const SAMPLE_RATE: u32 = 48_000;

const LISTENING_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/listening.wav"
));
const PASTED_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/pasted.wav"
));
const ERROR_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/error.wav"
));

/// Which moment a cue marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Recording has begun and the microphone is open.
    Listening,
    /// The transcript has reached the user.
    Pasted,
    /// Nothing usable came of it.
    Error,
}

impl Cue {
    /// Mono f32 samples at [`SAMPLE_RATE`].
    ///
    /// Decoded once and kept: a cue plays several times a minute, and re-parsing a WAV
    /// header each time would be wasted work on the path where latency is most
    /// noticeable.
    pub fn samples(self) -> &'static [f32] {
        match self {
            Self::Listening => cached(&LISTENING, LISTENING_WAV, "listening"),
            Self::Pasted => cached(&PASTED, PASTED_WAV, "pasted"),
            Self::Error => cached(&ERROR, ERROR_WAV, "error"),
        }
    }

    /// The file this cue is loaded from, for logs and documentation.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Listening => "listening.wav",
            Self::Pasted => "pasted.wav",
            Self::Error => "error.wav",
        }
    }
}

static LISTENING: OnceLock<Vec<f32>> = OnceLock::new();
static PASTED: OnceLock<Vec<f32>> = OnceLock::new();
static ERROR: OnceLock<Vec<f32>> = OnceLock::new();

fn cached(slot: &'static OnceLock<Vec<f32>>, bytes: &'static [u8], name: &str) -> &'static [f32] {
    slot.get_or_init(|| decode(bytes, name))
}

/// Decode an embedded 16-bit mono WAV.
///
/// A malformed asset is a build mistake, not a runtime condition, so this degrades to
/// silence and logs rather than panicking — a broken cue must never take the
/// application down mid-dictation.
fn decode(bytes: &'static [u8], name: &str) -> Vec<f32> {
    let reader = match hound::WavReader::new(Cursor::new(bytes)) {
        Ok(reader) => reader,
        Err(error) => {
            tracing::error!(%error, cue = name, "embedded cue is not a readable WAV");
            return Vec::new();
        }
    };

    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE {
        tracing::error!(
            cue = name,
            channels = spec.channels,
            sample_rate = spec.sample_rate,
            "embedded cue is not 48 kHz mono"
        );
    }

    let scale = 1.0 / f32::from(i16::MAX);
    reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .map(|sample| f32::from(sample) * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Cue; 3] = [Cue::Listening, Cue::Pasted, Cue::Error];

    #[test]
    fn every_cue_has_audio() {
        for cue in ALL {
            assert!(!cue.samples().is_empty(), "{cue:?} decoded to nothing");
        }
    }

    #[test]
    fn every_cue_is_short_enough_not_to_delay_speaking() {
        for cue in ALL {
            let seconds = cue.samples().len() as f32 / SAMPLE_RATE as f32;
            assert!((0.05..=0.60).contains(&seconds), "{cue:?} lasts {seconds}s");
        }
    }

    #[test]
    fn no_cue_is_loud_enough_to_startle() {
        // These are heard dozens of times a day. The recorded assets were scaled at
        // conversion time to sit near the synthesised ones.
        for cue in ALL {
            let peak = cue.samples().iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= 0.5, "{cue:?} peaks at {peak}");
            assert!(peak > 0.05, "{cue:?} is inaudibly quiet at {peak}");
        }
    }

    #[test]
    fn every_cue_names_a_distinct_file() {
        let mut names: Vec<_> = ALL.iter().map(|cue| cue.file_name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two cues share a file");
    }

    #[test]
    fn no_cue_is_wildly_louder_than_the_others() {
        // Peak is a poor guide: a sustained tone at the same peak as a short transient
        // sounds several times louder, so this compares RMS.
        let rms = |cue: Cue| {
            let samples = cue.samples();
            (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
        };
        let levels: Vec<f32> = ALL.iter().map(|&cue| rms(cue)).collect();
        let quietest = levels.iter().cloned().fold(f32::INFINITY, f32::min);
        let loudest = levels.iter().cloned().fold(0.0f32, f32::max);
        assert!(
            loudest / quietest < 4.0,
            "cue levels are out of step: {levels:?}"
        );
    }

    #[test]
    fn decoding_is_cached_rather_than_repeated() {
        // The same slice both times means the OnceLock is doing its job.
        for cue in ALL {
            assert!(
                std::ptr::eq(cue.samples(), cue.samples()),
                "{cue:?} re-decoded"
            );
        }
    }

    #[test]
    fn a_corrupt_asset_degrades_to_silence_instead_of_panicking() {
        assert!(decode(b"not a wav file at all", "broken").is_empty());
    }
}
