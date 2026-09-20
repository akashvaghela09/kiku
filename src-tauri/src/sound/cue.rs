//! The feedback cues and where their audio comes from.
//!
//! The two cues a user hears constantly — listening started, text delivered — are
//! recorded assets, embedded in the binary with `include_bytes!`. Embedding rather
//! than shipping files alongside means there is no path to resolve, no difference
//! between running from `cargo` and running from an installed bundle, and no way for
//! a cue to go missing.
//!
//! The failure cue is still synthesised. It is a different kind of event and is heard
//! rarely, so it does not need to be designed, and generating it keeps the bundle
//! smaller for a sound nobody wants to hear twice.

use std::borrow::Cow;
use std::io::Cursor;
use std::sync::OnceLock;

use super::tone;

/// 48 kHz mono, matching the synthesised cues so both share one playback path.
const ASSET_SAMPLE_RATE: u32 = 48_000;

const LISTENING_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/listening.wav"
));
const PASTED_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/sounds/pasted.wav"
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
    /// Sample rate the cue's audio is in.
    pub fn sample_rate(self) -> u32 {
        match self {
            Self::Listening | Self::Pasted => ASSET_SAMPLE_RATE,
            Self::Error => tone::SAMPLE_RATE,
        }
    }

    /// Mono f32 samples for the cue.
    ///
    /// Assets are decoded once and kept: a cue plays several times a minute, and
    /// re-parsing a WAV header each time would be wasted work on the path where
    /// latency is most noticeable.
    pub fn samples(self) -> Cow<'static, [f32]> {
        match self {
            Self::Listening => Cow::Borrowed(listening()),
            Self::Pasted => Cow::Borrowed(pasted()),
            Self::Error => Cow::Owned(tone::render_error()),
        }
    }
}

fn listening() -> &'static [f32] {
    static CACHE: OnceLock<Vec<f32>> = OnceLock::new();
    CACHE.get_or_init(|| decode(LISTENING_WAV, "listening"))
}

fn pasted() -> &'static [f32] {
    static CACHE: OnceLock<Vec<f32>> = OnceLock::new();
    CACHE.get_or_init(|| decode(PASTED_WAV, "pasted"))
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
    if spec.channels != 1 || spec.sample_rate != ASSET_SAMPLE_RATE {
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
            let seconds = cue.samples().len() as f32 / cue.sample_rate() as f32;
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
    fn the_recorded_assets_are_at_the_expected_rate() {
        assert_eq!(Cue::Listening.sample_rate(), ASSET_SAMPLE_RATE);
        assert_eq!(Cue::Pasted.sample_rate(), ASSET_SAMPLE_RATE);
    }

    #[test]
    fn decoding_is_cached_rather_than_repeated() {
        // Same slice both times means the OnceLock is doing its job.
        assert!(std::ptr::eq(listening(), listening()));
        assert!(std::ptr::eq(pasted(), pasted()));
    }

    #[test]
    fn a_corrupt_asset_degrades_to_silence_instead_of_panicking() {
        assert!(decode(b"not a wav file at all", "broken").is_empty());
    }
}
