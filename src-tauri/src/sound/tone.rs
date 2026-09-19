//! Generating the feedback tones.
//!
//! Kiku synthesises its cues rather than shipping audio files. Three reasons: no
//! licence to carry, no bytes in the bundle, and the tones stay tunable without a
//! round trip through an audio editor.
//!
//! The important part is the envelope. A sine that starts at full amplitude produces
//! an audible click, because the speaker cone is asked to jump instantaneously. Each
//! cue is therefore faded in and out over a few milliseconds.

use std::f32::consts::TAU;

/// Output sample rate for generated cues.
pub const SAMPLE_RATE: u32 = 48_000;

/// Fade applied to each end of a cue. Long enough to remove the click, short enough
/// not to soften the attack into a mush.
const FADE: Duration = Duration::from_millis(6);

use std::time::Duration;

/// Which moment a cue marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Listening has begun. Rises, because something is starting.
    Start,
    /// Listening has finished and text is on its way. Falls, to close the gesture.
    Stop,
    /// Nothing usable came of it.
    Error,
}

impl Cue {
    /// Frequency sweep, in hertz, and how long the cue lasts.
    ///
    /// The pitches sit in the 400–900 Hz range where small laptop speakers are most
    /// efficient and where a short tone stays audible without being shrill.
    fn shape(self) -> (f32, f32, Duration) {
        match self {
            Self::Start => (620.0, 880.0, Duration::from_millis(90)),
            Self::Stop => (880.0, 620.0, Duration::from_millis(110)),
            Self::Error => (400.0, 300.0, Duration::from_millis(160)),
        }
    }

    /// Peak amplitude. Feedback should be noticeable, not startling — these are heard
    /// dozens of times a day.
    fn gain(self) -> f32 {
        match self {
            Self::Start | Self::Stop => 0.18,
            Self::Error => 0.22,
        }
    }
}

/// Render a cue as mono f32 samples at [`SAMPLE_RATE`].
pub fn render(cue: Cue) -> Vec<f32> {
    let (from_hz, to_hz, duration) = cue.shape();
    let gain = cue.gain();

    let frames = (duration.as_secs_f32() * SAMPLE_RATE as f32) as usize;
    let fade_frames = ((FADE.as_secs_f32() * SAMPLE_RATE as f32) as usize).min(frames / 2);

    let mut phase = 0.0f32;
    let mut samples = Vec::with_capacity(frames);

    for index in 0..frames {
        let progress = index as f32 / frames as f32;
        let hz = from_hz + (to_hz - from_hz) * progress;

        // Advance the phase by the current frequency rather than evaluating
        // sin(2*pi*f(t)*t): the latter produces a discontinuity whenever f changes,
        // which is exactly the click the envelope is there to avoid.
        phase = (phase + TAU * hz / SAMPLE_RATE as f32) % TAU;

        let envelope = if index < fade_frames {
            index as f32 / fade_frames as f32
        } else if index >= frames - fade_frames {
            (frames - index) as f32 / fade_frames as f32
        } else {
            1.0
        };

        samples.push(phase.sin() * envelope * gain);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Cue; 3] = [Cue::Start, Cue::Stop, Cue::Error];

    #[test]
    fn every_cue_is_short_enough_not_to_delay_speaking() {
        for cue in ALL {
            let seconds = render(cue).len() as f32 / SAMPLE_RATE as f32;
            assert!((0.05..=0.25).contains(&seconds), "{cue:?} lasts {seconds}s");
        }
    }

    #[test]
    fn every_cue_stays_well_below_full_scale() {
        for cue in ALL {
            let peak = render(cue).iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= 0.25, "{cue:?} peaks at {peak}");
            assert!(peak > 0.05, "{cue:?} is inaudibly quiet at {peak}");
        }
    }

    #[test]
    fn every_cue_starts_and_ends_at_silence() {
        // A non-zero first or last sample is an audible click on every playback.
        for cue in ALL {
            let samples = render(cue);
            assert!(samples[0].abs() < 1e-6, "{cue:?} clicks on the way in");
            assert!(
                samples[samples.len() - 1].abs() < 0.01,
                "{cue:?} clicks on the way out"
            );
        }
    }

    #[test]
    fn the_waveform_is_continuous() {
        // Step changes between neighbouring samples are what a click actually is.
        for cue in ALL {
            let samples = render(cue);
            let largest_step = samples
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .fold(0.0f32, f32::max);
            assert!(largest_step < 0.05, "{cue:?} jumps by {largest_step}");
        }
    }

    #[test]
    fn start_rises_and_stop_falls() {
        let (start_from, start_to, _) = Cue::Start.shape();
        let (stop_from, stop_to, _) = Cue::Stop.shape();
        assert!(start_to > start_from, "starting should rise");
        assert!(stop_to < stop_from, "stopping should fall");
    }
}

#[cfg(test)]
mod audition {
    use super::*;

    /// Writes the cues as WAV files so they can be listened to rather than only
    /// asserted about. Ignored by default; run with:
    /// `cargo test audition -- --ignored --nocapture`
    #[test]
    #[ignore = "writes files for manual listening"]
    fn write_cue_wavs() {
        let dir = std::env::var("KIKU_CUE_DIR").unwrap_or_else(|_| "/tmp".to_owned());

        for (cue, name) in [
            (Cue::Start, "start"),
            (Cue::Stop, "stop"),
            (Cue::Error, "error"),
        ] {
            let path = format!("{dir}/kiku-cue-{name}.wav");
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: SAMPLE_RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(&path, spec).expect("creating the WAV");
            for sample in render(cue) {
                let scaled = (sample * f32::from(i16::MAX)) as i16;
                writer.write_sample(scaled).expect("writing a sample");
            }
            writer.finalize().expect("finalising the WAV");
            println!("wrote {path}");
        }
    }
}
