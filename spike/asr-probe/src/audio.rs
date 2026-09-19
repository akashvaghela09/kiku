//! Audio helpers for the spike: WAV loading, mono downmixing and resampling to the
//! 16 kHz mono f32 that Parakeet expects.
//!
//! The resampler here is deliberately naive (linear interpolation). It is good enough
//! to prove the pipeline; Kiku proper should use a windowed-sinc resampler such as
//! `rubato`, because linear interpolation aliases audibly when downsampling 48k -> 16k.

use anyhow::{Context, Result};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Average interleaved frames down to a single channel.
pub fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    let channels = usize::from(channels);
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Linear-interpolation resample of mono audio.
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = f64::from(from_rate) / f64::from(to_rate);
    let out_len = ((samples.len() as f64) / ratio).floor() as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos.floor() as usize;
            let frac = (pos - pos.floor()) as f32;
            let a = samples[idx];
            let b = *samples.get(idx + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

/// Load any WAV file as 16 kHz mono f32, converting sample format and rate as needed.
pub fn load_wav_16k_mono(path: &str) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("opening WAV file {path}"))?;
    let spec = reader.spec();

    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / f32::from(i16::MAX);
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()?
        }
    };

    let mono = to_mono(&raw, spec.channels);
    Ok(resample(&mono, spec.sample_rate, TARGET_SAMPLE_RATE))
}

/// Peak and RMS of a buffer, used to sanity-check that the microphone actually
/// captured something rather than silence.
pub fn levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    (peak, rms)
}
