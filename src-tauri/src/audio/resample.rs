//! Sample-rate conversion to the 16 kHz the recogniser expects.
//!
//! All of rubato's generality is contained here behind one function, so the rest of
//! the app never sees an adapter, a chunk size or a channel layout. Conversion runs
//! once when a recording stops, never inside the audio callback.
//!
//! Correct anti-aliasing matters even though the destination is "only" speech: naive
//! linear interpolation folds everything above 8 kHz back down into the band the
//! acoustic model is listening to, which shows up as degraded accuracy rather than as
//! anything audible.

use rubato::audioadapter_buffers::direct::SequentialSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::error::{Error, Result};

/// Frames handed to the resampler at a time. 1024 keeps the FFT blocks small enough to
/// stay in cache while amortising per-call overhead.
const CHUNK: usize = 1024;

/// Resample mono audio, returning it unchanged when the rate already matches.
pub fn resample_mono(input: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    if from_rate == to_rate || input.is_empty() {
        return Ok(input.to_vec());
    }

    let mut resampler = Fft::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        CHUNK,
        1,
        FixedSync::Both,
    )
    .map_err(|e| Error::Internal(format!("could not build the resampler: {e}")))?;

    let source = SequentialSlice::new(input, 1, input.len())
        .map_err(|e| Error::Internal(format!("resampler input rejected: {e}")))?;

    // Sized for the worst case: rubato reports exactly how much room it may need,
    // including the filter delay it trims off for us.
    let capacity = resampler.process_all_needed_output_len(input.len());
    let mut buffer = vec![0.0f32; capacity];
    let mut sink = SequentialSlice::new_mut(buffer.as_mut_slice(), 1, capacity)
        .map_err(|e| Error::Internal(format!("resampler output rejected: {e}")))?;

    let (_consumed, produced) = resampler
        .process_all_into_buffer(&source, &mut sink, input.len(), None)
        .map_err(|e| Error::Internal(format!("resampling failed: {e}")))?;

    buffer.truncate(produced);
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frames: usize, rate: u32, hz: f32) -> Vec<f32> {
        (0..frames)
            .map(|i| (i as f32 * std::f32::consts::TAU * hz / rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn matching_rates_are_returned_untouched() {
        let input = sine(1000, 16_000, 440.0);
        let output = resample_mono(&input, 16_000, 16_000).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn empty_input_stays_empty() {
        assert!(resample_mono(&[], 48_000, 16_000).unwrap().is_empty());
    }

    #[test]
    fn output_length_tracks_the_rate_ratio() {
        let input = sine(48_000, 48_000, 440.0);
        let output = resample_mono(&input, 48_000, 16_000).unwrap();
        // One second in, one second out, within a chunk's tolerance.
        assert!(
            (output.len() as i64 - 16_000).abs() < CHUNK as i64,
            "expected ~16000 frames, got {}",
            output.len()
        );
    }

    #[test]
    fn a_tone_inside_the_passband_survives_downsampling() {
        // 440 Hz is far below the 8 kHz Nyquist limit of the destination rate, so it
        // must come through with its amplitude essentially intact.
        let input = sine(48_000, 48_000, 440.0);
        let output = resample_mono(&input, 48_000, 16_000).unwrap();

        let settled = &output[1000..output.len() - 1000];
        let peak = settled.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.45 && peak < 0.55, "amplitude drifted: peak {peak}");
    }

    #[test]
    fn a_tone_above_nyquist_is_filtered_out_rather_than_aliased() {
        // 15 kHz cannot exist at 16 kHz and must be attenuated, not folded down to
        // 1 kHz where the acoustic model would hear it as speech.
        let input = sine(48_000, 48_000, 15_000.0);
        let output = resample_mono(&input, 48_000, 16_000).unwrap();

        let settled = &output[2000..output.len() - 2000];
        let rms = (settled.iter().map(|s| s * s).sum::<f32>() / settled.len() as f32).sqrt();
        assert!(rms < 0.02, "aliasing leaked through: rms {rms}");
    }
}
