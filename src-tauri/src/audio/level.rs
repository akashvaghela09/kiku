//! Loudness metering.
//!
//! Levels are computed inside the audio callback, so everything here is allocation-free
//! and branch-light. The overlay's waveform is driven from these values, and the
//! clipping flag is what lets Settings tell a user their input gain is too high rather
//! than silently feeding distorted audio to the recogniser.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Samples at or above this magnitude are treated as clipped. Slightly below 1.0
/// because converters round, and a run of exact 1.0 samples is already distortion.
const CLIP_THRESHOLD: f32 = 0.99;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    /// Root mean square of the buffer, 0.0..=1.0. Perceptually the useful one.
    pub rms: f32,
    /// Largest absolute sample in the buffer, 0.0..~1.0.
    pub peak: f32,
    /// Whether any sample in the buffer reached full scale.
    pub clipped: bool,
}

impl Level {
    pub fn measure(samples: &[f32]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sum_squares = 0.0f32;
        let mut peak = 0.0f32;
        for &sample in samples {
            sum_squares += sample * sample;
            peak = peak.max(sample.abs());
        }

        Self {
            rms: (sum_squares / samples.len() as f32).sqrt(),
            peak,
            clipped: peak >= CLIP_THRESHOLD,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_silent_and_unclipped() {
        let level = Level::measure(&[]);
        assert_eq!(level.rms, 0.0);
        assert!(!level.clipped);
    }

    #[test]
    fn full_scale_square_wave_reads_as_clipped() {
        let level = Level::measure(&[1.0, -1.0, 1.0, -1.0]);
        assert!((level.rms - 1.0).abs() < f32::EPSILON);
        assert!((level.peak - 1.0).abs() < f32::EPSILON);
        assert!(level.clipped);
    }

    #[test]
    fn half_scale_is_not_clipped() {
        let level = Level::measure(&[0.5, -0.5]);
        assert!((level.rms - 0.5).abs() < 1e-6);
        assert!(!level.clipped);
    }

    #[test]
    fn rms_is_below_peak_for_a_sine() {
        let sine: Vec<f32> = (0..1000)
            .map(|i| (i as f32 * std::f32::consts::TAU / 50.0).sin())
            .collect();
        let level = Level::measure(&sine);
        // A sine's RMS is peak / sqrt(2).
        assert!((level.rms - level.peak / 2.0_f32.sqrt()).abs() < 0.01);
    }
}
