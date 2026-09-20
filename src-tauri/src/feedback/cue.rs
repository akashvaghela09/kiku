//! The two feedback cues and where their audio comes from.
//!
//! Both are 48 kHz mono WAV files in `assets/sounds/`, embedded in the binary with
//! `include_bytes!`. Embedding rather than shipping files alongside means there is no
//! path to resolve, no difference between running from `cargo` and running from an
//! installed bundle, and no way for a cue to go missing.

use std::io::Cursor;
use std::sync::OnceLock;

/// Both cues are 48 kHz mono, so playback never has to branch on the source.
pub const SAMPLE_RATE: u32 = 48_000;

const START_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/start.wav"
));
const STOP_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/stop.wav"
));

/// Which moment a cue marks.
///
/// Exactly two, and both are about *listening* rather than about the result. A
/// dictation that produces no text makes no sound: the overlay says so, and a failure
/// chime is one more noise in a tool used dozens of times a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Kiku has started listening - on either the hold or the toggle shortcut.
    Start,
    /// Kiku has stopped listening.
    Stop,
}

impl Cue {
    /// Mono f32 samples at [`SAMPLE_RATE`].
    ///
    /// Decoded once and kept: a cue plays several times a minute, and re-parsing a WAV
    /// header each time would be wasted work on the path where latency is most
    /// noticeable.
    pub fn samples(self) -> &'static [f32] {
        match self {
            Self::Start => cached(&START, START_WAV, "start"),
            Self::Stop => cached(&STOP, STOP_WAV, "stop"),
        }
    }

    /// The file this cue is loaded from, for logs and documentation.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Start => "start.wav",
            Self::Stop => "stop.wav",
        }
    }
}

static START: OnceLock<Vec<f32>> = OnceLock::new();
static STOP: OnceLock<Vec<f32>> = OnceLock::new();

fn cached(slot: &'static OnceLock<Vec<f32>>, bytes: &'static [u8], name: &str) -> &'static [f32] {
    slot.get_or_init(|| decode(bytes, name))
}

/// Decode an embedded 16-bit mono WAV.
///
/// A malformed asset is a build mistake, not a runtime condition, so this degrades to
/// silence and logs rather than panicking - a broken cue must never take the
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

    const ALL: [Cue; 2] = [Cue::Start, Cue::Stop];

    #[test]
    fn both_cues_have_audio() {
        for cue in ALL {
            assert!(!cue.samples().is_empty(), "{cue:?} decoded to nothing");
        }
    }

    #[test]
    fn both_cues_are_short_enough_not_to_delay_speaking() {
        for cue in ALL {
            let seconds = cue.samples().len() as f32 / SAMPLE_RATE as f32;
            assert!((0.05..=0.60).contains(&seconds), "{cue:?} lasts {seconds}s");
        }
    }

    #[test]
    fn neither_cue_is_loud_enough_to_startle() {
        // These are heard dozens of times a day.
        for cue in ALL {
            let peak = cue.samples().iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= 0.5, "{cue:?} peaks at {peak}");
            assert!(peak > 0.02, "{cue:?} is inaudibly quiet at {peak}");
        }
    }

    #[test]
    fn the_two_cues_sound_distinct_from_each_other() {
        // Start and stop must not be mistakable for one another, or the feedback says
        // nothing. Different lengths are the cheapest guarantee of that.
        let length = |cue: Cue| cue.samples().len();
        assert_ne!(length(Cue::Start), length(Cue::Stop));
    }

    #[test]
    fn each_cue_names_its_own_file() {
        assert_ne!(Cue::Start.file_name(), Cue::Stop.file_name());
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
