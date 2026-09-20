//! Playing the feedback cues.
//!
//! Each cue opens an output stream, plays, and closes it. A persistent stream would
//! shave perhaps twenty milliseconds off the start cue, at the cost of holding the
//! audio device open for the entire time Kiku is running — which shows up in every
//! system mixer and can keep a laptop's audio hardware awake. For a hundred-millisecond
//! blip a few times an hour, that trade is not worth making.
//!
//! Playback never blocks the caller: a cue that cannot be played is a warning in the
//! log, never an interruption to dictation.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use super::cue::{Cue, SAMPLE_RATE};

/// Play a cue on the default output device, returning immediately.
pub fn play(cue: Cue) {
    std::thread::Builder::new()
        .name("kiku-cue".into())
        .spawn(move || {
            if let Err(error) = play_blocking(cue) {
                tracing::warn!(?cue, %error, "could not play the feedback tone");
            }
        })
        .map(|_| ())
        .unwrap_or_else(|error| tracing::warn!(%error, "could not start the cue thread"));
}

fn play_blocking(cue: Cue) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no audio output device".to_owned())?;
    let config = device
        .default_output_config()
        .map_err(|error| error.to_string())?;

    let channels = usize::from(config.channels());
    let samples = Arc::new(resample_to(
        cue.samples().to_vec(),
        SAMPLE_RATE,
        config.sample_rate(),
    ));
    let cursor = Arc::new(AtomicUsize::new(0));
    let total = samples.len();

    let stream = build_stream(
        &device,
        config.into(),
        config.sample_format(),
        Arc::clone(&samples),
        Arc::clone(&cursor),
        channels,
    )?;

    stream.play().map_err(|error| error.to_string())?;

    // Wait for the cue to drain, with a ceiling so a stalled device cannot leak the
    // thread. The tail allows the audio buffer to flush before the stream is dropped.
    let expected = Duration::from_secs_f32(total as f32 / config.sample_rate() as f32);
    let deadline = std::time::Instant::now() + expected + Duration::from_millis(250);
    while cursor.load(Ordering::Relaxed) < total && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(30));

    Ok(())
}

fn build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: SampleFormat,
    samples: Arc<Vec<f32>>,
    cursor: Arc<AtomicUsize>,
    channels: usize,
) -> Result<cpal::Stream, String> {
    let on_error = |error| tracing::warn!(%error, "cue output stream error");

    // One mono cue fanned out to however many channels the device has.
    let fill = move |output: &mut [f32]| {
        let mut position = cursor.load(Ordering::Relaxed);
        for frame in output.chunks_mut(channels) {
            let value = samples.get(position).copied().unwrap_or(0.0);
            position += 1;
            for slot in frame.iter_mut() {
                *slot = value;
            }
        }
        cursor.store(position, Ordering::Relaxed);
    };

    match format {
        SampleFormat::F32 => device.build_output_stream(
            config,
            move |data: &mut [f32], _| fill(data),
            on_error,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            config,
            move |data: &mut [i16], _| {
                let mut scratch = vec![0.0f32; data.len()];
                fill(&mut scratch);
                for (out, value) in data.iter_mut().zip(scratch) {
                    *out = (value * f32::from(i16::MAX)) as i16;
                }
            },
            on_error,
            None,
        ),
        other => return Err(format!("unsupported output format {other:?}")),
    }
    .map_err(|error| error.to_string())
}

/// Nearest-neighbour rate conversion for a cue.
///
/// Good enough here and nowhere else: a few hundred milliseconds of blip has no
/// content worth protecting from the aliasing this introduces, unlike captured
/// speech, which goes through rubato.
fn resample_to(samples: Vec<f32>, from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples;
    }
    let ratio = f64::from(from_rate) / f64::from(to_rate);
    let frames = (samples.len() as f64 / ratio) as usize;
    (0..frames)
        .map(|index| {
            let source = (index as f64 * ratio) as usize;
            samples.get(source).copied().unwrap_or(0.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_rate_is_passed_through_untouched() {
        let samples = Cue::Start.samples().to_vec();
        let rate = SAMPLE_RATE;
        assert_eq!(resample_to(samples.clone(), rate, rate), samples);
    }

    #[test]
    fn resampling_scales_the_length_by_the_rate_ratio() {
        let samples = Cue::Start.samples().to_vec();
        let from = SAMPLE_RATE;
        let converted = resample_to(samples.clone(), from, 44_100);
        let expected = samples.len() as f64 * 44_100.0 / f64::from(from);
        assert!((converted.len() as f64 - expected).abs() < 2.0);
    }

    #[test]
    fn resampling_preserves_the_duration_in_seconds() {
        for cue in [Cue::Start, Cue::Stop] {
            let from = SAMPLE_RATE;
            let original = cue.samples().to_vec();
            let original_seconds = original.len() as f32 / from as f32;

            for rate in [22_050, 44_100, 48_000, 96_000] {
                let converted = resample_to(original.clone(), from, rate);
                let seconds = converted.len() as f32 / rate as f32;
                assert!(
                    (seconds - original_seconds).abs() < 0.01,
                    "{cue:?} at {rate} Hz gave {seconds}s, expected {original_seconds}s"
                );
            }
        }
    }
}
