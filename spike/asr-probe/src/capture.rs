//! Microphone capture via `cpal`, normalised to 16 kHz mono f32.
//!
//! This is the spike's rehearsal of what Kiku's chunk 2 audio layer has to do:
//! accept whatever format the device offers, downmix, resample, and hand back a
//! plain buffer. Kiku proper will stream this into a ring buffer instead of
//! collecting it, and will emit live levels for the overlay waveform.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use crate::audio::{resample, to_mono, TARGET_SAMPLE_RATE};

pub struct CaptureResult {
    pub samples: Vec<f32>,
    pub device_name: String,
    pub device_rate: u32,
    pub device_channels: u16,
}

pub fn list_input_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let default = host.default_input_device().and_then(|d| d.name().ok());
    let mut names = Vec::new();
    for device in host.input_devices().context("enumerating input devices")? {
        let name = device.name().unwrap_or_else(|_| "<unnamed>".into());
        let marker = if Some(&name) == default.as_ref() {
            " (default)"
        } else {
            ""
        };
        names.push(format!("{name}{marker}"));
    }
    Ok(names)
}

fn pick_device(requested: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    match requested {
        None => host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default input device; is a microphone connected?")),
        Some(wanted) => host
            .input_devices()?
            .find(|d| d.name().is_ok_and(|n| n.contains(wanted)))
            .ok_or_else(|| anyhow!("no input device matching {wanted:?}")),
    }
}

/// Record for `duration`, returning 16 kHz mono f32 samples.
///
/// `on_level` is invoked from the audio callback with the RMS of each buffer, which is
/// how the overlay's waveform will eventually be driven.
pub fn record(
    duration: Duration,
    device_name: Option<&str>,
    mut on_level: impl FnMut(f32) + Send + 'static,
) -> Result<CaptureResult> {
    let device = pick_device(device_name)?;
    let name = device.name().unwrap_or_else(|_| "<unnamed>".into());
    let config = device
        .default_input_config()
        .context("querying default input config")?;
    let sample_format = config.sample_format();
    let device_rate = config.sample_rate().0;
    let device_channels = config.channels();
    let stream_config: cpal::StreamConfig = config.into();

    let collected = Arc::new(Mutex::new(Vec::<f32>::new()));
    let sink = Arc::clone(&collected);

    let on_error = |err| eprintln!("audio stream error: {err}");

    // Accumulate raw interleaved frames; downmix and resample once at the end so the
    // callback stays cheap and never allocates more than the push.
    let mut push = move |data: &[f32]| {
        let sum_sq: f32 = data.iter().map(|s| s * s).sum();
        let rms = (sum_sq / data.len().max(1) as f32).sqrt();
        on_level(rms);
        if let Ok(mut buf) = sink.lock() {
            buf.extend_from_slice(data);
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| push(data),
            on_error,
            None,
        ),
        SampleFormat::I16 => {
            let scale = 1.0 / f32::from(i16::MAX);
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> = data.iter().map(|&s| f32::from(s) * scale).collect();
                    push(&converted);
                },
                on_error,
                None,
            )
        }
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                let converted: Vec<f32> = data
                    .iter()
                    .map(|&s| (f32::from(s) - 32768.0) / 32768.0)
                    .collect();
                push(&converted);
            },
            on_error,
            None,
        ),
        other => return Err(anyhow!("unsupported sample format {other:?}")),
    }
    .context("building the input stream")?;

    stream.play().context("starting the input stream")?;
    let started = Instant::now();
    while started.elapsed() < duration {
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(stream);

    let raw = collected
        .lock()
        .map_err(|_| anyhow!("audio buffer mutex was poisoned"))?
        .clone();

    let mono = to_mono(&raw, device_channels);
    Ok(CaptureResult {
        samples: resample(&mono, device_rate, TARGET_SAMPLE_RATE),
        device_name: name,
        device_rate,
        device_channels,
    })
}
