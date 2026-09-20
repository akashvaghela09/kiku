//! The generator behind `assets/sounds/error.wav`.
//!
//! The failure cue has no recording, so it is synthesised — but only once. Running
//! `cargo test render_error_cue -- --ignored` writes this out to the asset, which is
//! then committed and loaded exactly like the two recorded cues. Nothing here runs
//! during a dictation.
//!
//! The envelope is the part that matters. A sine starting at full amplitude produces
//! an audible click, because the speaker cone is asked to jump instantaneously, and
//! evaluating `sin(2*pi*f(t)*t)` for a sweep introduces a discontinuity every time the
//! frequency changes. The phase is accumulated instead, and both ends are faded.

use std::f32::consts::TAU;
use std::time::Duration;

/// Output sample rate for the generated cue.
pub const SAMPLE_RATE: u32 = 48_000;

/// Fade applied to each end. Long enough to remove the click, short enough not to
/// soften the attack into a mush.
const FADE: Duration = Duration::from_millis(6);

/// A falling two-tone blip: low, and slower than the others, so it reads as "that did
/// not work" without being alarming.
const FROM_HZ: f32 = 400.0;
const TO_HZ: f32 = 300.0;
const LENGTH: Duration = Duration::from_millis(160);

/// Peak amplitude.
///
/// Set by measuring rather than by eye: a sustained tone reads far louder than a short
/// transient at the same peak, and at 0.22 this cue was about four times the recorded
/// cues' RMS. This puts it at roughly twice theirs — more noticeable than routine
/// feedback, since it reports a problem, without being the loudest thing Kiku does.
const GAIN: f32 = 0.09;

/// Render the failure cue as mono f32 samples at [`SAMPLE_RATE`].
pub fn render_error() -> Vec<f32> {
    let frames = (LENGTH.as_secs_f32() * SAMPLE_RATE as f32) as usize;
    let fade_frames = ((FADE.as_secs_f32() * SAMPLE_RATE as f32) as usize).min(frames / 2);

    let mut phase = 0.0f32;
    let mut samples = Vec::with_capacity(frames);

    for index in 0..frames {
        let progress = index as f32 / frames as f32;
        let hz = FROM_HZ + (TO_HZ - FROM_HZ) * progress;

        // Advance the phase by the current frequency rather than evaluating
        // sin(2*pi*f(t)*t): the latter is discontinuous whenever f changes, which is
        // exactly the click the envelope exists to avoid.
        phase = (phase + TAU * hz / SAMPLE_RATE as f32) % TAU;

        let envelope = if index < fade_frames {
            index as f32 / fade_frames as f32
        } else if index >= frames - fade_frames {
            (frames - index) as f32 / fade_frames as f32
        } else {
            1.0
        };

        samples.push(phase.sin() * envelope * GAIN);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_failure_cue_is_short() {
        let seconds = render_error().len() as f32 / SAMPLE_RATE as f32;
        assert!((0.05..=0.25).contains(&seconds), "lasts {seconds}s");
    }

    #[test]
    fn the_failure_cue_stays_well_below_full_scale() {
        let peak = render_error().iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak <= 0.25, "peaks at {peak}");
        assert!(peak > 0.02, "inaudibly quiet at {peak}");
    }

    #[test]
    fn the_failure_cue_is_not_much_louder_than_the_recorded_ones() {
        // Peak is a poor guide here: a sustained tone at the same peak as a short
        // transient sounds several times louder, so this compares RMS.
        let samples = render_error();
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(
            (0.02..=0.10).contains(&rms),
            "failure cue RMS {rms} is out of step with the recorded cues (~0.03)"
        );
    }

    #[test]
    fn the_failure_cue_starts_and_ends_at_silence() {
        // A non-zero first or last sample is an audible click on every playback.
        let samples = render_error();
        assert!(samples[0].abs() < 1e-6, "clicks on the way in");
        assert!(
            samples[samples.len() - 1].abs() < 0.01,
            "clicks on the way out"
        );
    }

    #[test]
    fn the_waveform_is_continuous() {
        // A step between neighbouring samples is what a click actually is.
        let samples = render_error();
        let largest_step = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(largest_step < 0.05, "jumps by {largest_step}");
    }

    #[test]
    fn the_failure_cue_falls_rather_than_rises() {
        // Measured from the rendered audio rather than from the constants: a falling
        // pitch crosses zero less often as it goes, so the sweep is actually audible
        // in the output and not just declared in a pair of numbers.
        let samples = render_error();
        let third = samples.len() / 3;

        let crossings = |window: &[f32]| {
            window
                .windows(2)
                .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
                .count()
        };

        let opening = crossings(&samples[..third]);
        let closing = crossings(&samples[samples.len() - third..]);
        assert!(
            closing < opening,
            "a failure should not rise in pitch: {opening} crossings then {closing}"
        );
    }
}
