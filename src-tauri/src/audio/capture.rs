//! Microphone capture.
//!
//! `cpal`'s `Stream` is not `Send` on every platform, so it can never be stored in
//! shared application state. Instead each recording owns a dedicated thread: the
//! thread builds the stream, parks while audio accumulates, and tears the stream down
//! before handing the samples back. The rest of the app sees only `start` and `stop`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;

use super::device::{self, TARGET_SAMPLE_RATE};
use super::level::Level;
use super::resample::resample_mono;
use crate::error::{Error, Result};

/// Hard ceiling on a single recording, so a stuck key cannot grow the buffer without
/// bound. Ten minutes of 16 kHz mono f32 is about 38 MB.
pub const MAX_RECORDING: Duration = Duration::from_secs(600);

/// A finished recording, already 16 kHz mono.
#[derive(Debug)]
pub struct Recording {
    pub samples: Vec<f32>,
    pub device_name: String,
    /// True when the recording was cut short by `MAX_RECORDING`.
    pub truncated: bool,
    /// True when any buffer reached full scale - the input gain is too high.
    pub clipped: bool,
}

impl Recording {
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.samples.len() as f64 / f64::from(TARGET_SAMPLE_RATE))
    }

    pub fn is_silent(&self) -> bool {
        Level::measure(&self.samples).peak < 0.01
    }
}

/// A recording in progress.
pub struct Capture {
    stop: Arc<AtomicBool>,
    worker: thread::JoinHandle<Result<Recording>>,
}

impl Capture {
    /// Open the microphone and start accumulating audio.
    ///
    /// `on_level` runs on the audio thread for every buffer, which is what drives the
    /// overlay waveform. It must not block or allocate.
    pub fn start(
        preferred_device: Option<String>,
        on_level: impl Fn(Level) + Send + 'static,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("kiku-capture".into())
            .spawn(move || {
                run_capture(
                    preferred_device.as_deref(),
                    &worker_stop,
                    on_level,
                    &ready_tx,
                )
            })
            .map_err(|e| Error::Microphone(format!("could not start the audio thread: {e}")))?;

        // Surface device-open failures to the caller rather than at stop() time, so the
        // overlay never shows "listening" for a microphone that never opened.
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { stop, worker }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(_) => Err(Error::Microphone(
                "the audio thread stopped before it started".into(),
            )),
        }
    }

    /// Stop recording and return the captured audio at 16 kHz mono.
    pub fn stop(self) -> Result<Recording> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker
            .join()
            .map_err(|_| Error::Microphone("the audio thread panicked".into()))?
    }
}

fn run_capture(
    preferred_device: Option<&str>,
    stop: &AtomicBool,
    on_level: impl Fn(Level) + Send + 'static,
    ready: &mpsc::Sender<Result<()>>,
) -> Result<Recording> {
    let device = match device::resolve(preferred_device) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Err(Error::NoMicrophone);
        }
    };

    let device_name = device::name_of(&device);
    let config = match device::best_config(&device) {
        Ok(config) => config,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Err(Error::Microphone("no usable input configuration".into()));
        }
    };

    let sample_format = config.sample_format();
    let source_rate = config.sample_rate();
    let channels = config.channels();
    #[cfg(target_os = "linux")]
    let buffer_range = *config.buffer_size();
    let stream_config: cpal::StreamConfig = config.into();

    // Pre-allocate the whole ceiling once so the audio callback never triggers a
    // reallocation mid-sentence.
    let capacity =
        (source_rate as usize) * usize::from(channels) * MAX_RECORDING.as_secs() as usize;
    let buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(capacity)));
    let clipped = Arc::new(AtomicBool::new(false));

    #[cfg(target_os = "linux")]
    let stream = build_linux_stream(
        &device,
        stream_config,
        &buffer_range,
        sample_format,
        &buffer,
        &clipped,
        on_level,
    );

    // macOS (CoreAudio) and Windows (WASAPI shared mode) already deliver buffers of
    // about 10 ms by default, so they open the stream exactly as they always have.
    #[cfg(not(target_os = "linux"))]
    let stream = build_stream(
        &device,
        stream_config,
        sample_format,
        Arc::clone(&buffer),
        Arc::clone(&clipped),
        on_level,
    );

    let stream = match stream {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready.send(Err(error));
            return Err(Error::Microphone("could not open the microphone".into()));
        }
    };

    if let Err(error) = stream.play() {
        let error = Error::Microphone(error.to_string());
        let _ = ready.send(Err(error));
        return Err(Error::Microphone("could not start the microphone".into()));
    }

    let _ = ready.send(Ok(()));

    let started = Instant::now();
    let mut truncated = false;
    while !stop.load(Ordering::Relaxed) {
        if started.elapsed() >= MAX_RECORDING {
            tracing::warn!("recording hit the {MAX_RECORDING:?} ceiling");
            truncated = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    drop(stream);

    let raw = buffer
        .lock()
        .map_err(|_| Error::Microphone("the audio buffer was poisoned".into()))?
        .clone();

    let mono = downmix(&raw, channels);
    let samples = resample_mono(&mono, source_rate, TARGET_SAMPLE_RATE)?;

    Ok(Recording {
        samples,
        device_name,
        truncated,
        clipped: clipped.load(Ordering::Relaxed),
    })
}

/// How long one audio buffer should be on Linux.
///
/// Every buffer is one chance to publish a level, so this sets the ceiling on how
/// often the waveform can move. Left to the platform, CoreAudio and WASAPI deliver
/// about 10 ms, but ALSA - through PipeWire or PulseAudio - delivers 128 ms, which held
/// the overlay to eight updates a second: bars that stepped rather than moved, and
/// syllables averaged away inside a single buffer. Twenty milliseconds restores the
/// full [`LEVEL_INTERVAL`](crate::dictation::LEVEL_INTERVAL) rate with room to spare
/// before the audio thread risks an overrun.
///
/// Linux only. macOS and Windows already get short buffers by default and keep
/// opening the stream exactly as they always have.
#[cfg(target_os = "linux")]
const BUFFER_DURATION: Duration = Duration::from_millis(20);

/// The buffer size to ask for, in frames, or `None` to leave it to the device.
#[cfg(target_os = "linux")]
fn preferred_buffer_size(range: &cpal::SupportedBufferSize, sample_rate: u32) -> Option<u32> {
    match *range {
        cpal::SupportedBufferSize::Range { min, max } => {
            let frames =
                (u64::from(sample_rate) * BUFFER_DURATION.as_millis() as u64 / 1000) as u32;
            Some(frames.clamp(min, max))
        }
        cpal::SupportedBufferSize::Unknown => None,
    }
}

/// Open the stream with short buffers, falling back to the device default.
///
/// A device may advertise a range and still refuse a size inside it, and a refused
/// buffer size must not cost the user their microphone. The callback is shared rather
/// than moved so the second attempt can reuse it; the lock is uncontended.
#[cfg(target_os = "linux")]
fn build_linux_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    range: &cpal::SupportedBufferSize,
    format: SampleFormat,
    buffer: &Arc<Mutex<Vec<f32>>>,
    clipped: &Arc<AtomicBool>,
    on_level: impl Fn(Level) + Send + 'static,
) -> Result<cpal::Stream> {
    let on_level = Arc::new(Mutex::new(on_level));
    let open = |buffer_size| {
        let on_level = Arc::clone(&on_level);
        build_stream(
            device,
            cpal::StreamConfig {
                buffer_size,
                ..config
            },
            format,
            Arc::clone(buffer),
            Arc::clone(clipped),
            move |level| {
                if let Ok(on_level) = on_level.lock() {
                    on_level(level);
                }
            },
        )
    };

    match preferred_buffer_size(range, config.sample_rate) {
        Some(frames) => open(cpal::BufferSize::Fixed(frames)).or_else(|error| {
            tracing::debug!(%error, frames, "buffer size refused; using the device default");
            open(cpal::BufferSize::Default)
        }),
        None => open(cpal::BufferSize::Default),
    }
}

fn build_stream(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: SampleFormat,
    buffer: Arc<Mutex<Vec<f32>>>,
    clipped: Arc<AtomicBool>,
    on_level: impl Fn(Level) + Send + 'static,
) -> Result<cpal::Stream> {
    let on_error = |error| tracing::error!(%error, "microphone stream error");

    // One closure body for every sample format: convert to f32 up front, then share
    // the metering and accumulation path.
    let consume = move |samples: &[f32]| {
        let level = Level::measure(samples);
        if level.clipped {
            clipped.store(true, Ordering::Relaxed);
        }
        on_level(level);

        if let Ok(mut buffer) = buffer.lock() {
            buffer.extend_from_slice(samples);
        }
    };

    let stream = match format {
        SampleFormat::F32 => {
            device.build_input_stream(config, move |data: &[f32], _| consume(data), on_error, None)
        }
        SampleFormat::I16 => {
            let scale = 1.0 / f32::from(i16::MAX);
            device.build_input_stream(
                config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> = data.iter().map(|&s| f32::from(s) * scale).collect();
                    consume(&converted);
                },
                on_error,
                None,
            )
        }
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                let converted: Vec<f32> = data
                    .iter()
                    .map(|&s| (f32::from(s) - 32_768.0) / 32_768.0)
                    .collect();
                consume(&converted);
            },
            on_error,
            None,
        ),
        other => {
            return Err(Error::Microphone(format!(
                "unsupported sample format {other:?}"
            )))
        }
    };

    stream.map_err(|e| Error::Microphone(e.to_string()))
}

/// Average interleaved frames down to one channel.
fn downmix(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    let channels = usize::from(channels);
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn a_buffer_is_twenty_milliseconds_where_the_device_allows_it() {
        let open = cpal::SupportedBufferSize::Range {
            min: 64,
            max: 1_048_576,
        };
        assert_eq!(preferred_buffer_size(&open, 16_000), Some(320));
        assert_eq!(preferred_buffer_size(&open, 48_000), Some(960));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_buffer_size_outside_the_device_range_is_clamped_into_it() {
        let narrow = cpal::SupportedBufferSize::Range {
            min: 1024,
            max: 4096,
        };
        assert_eq!(preferred_buffer_size(&narrow, 16_000), Some(1024));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_unknown_range_is_left_to_the_device() {
        assert_eq!(
            preferred_buffer_size(&cpal::SupportedBufferSize::Unknown, 16_000),
            None
        );
    }

    #[test]
    fn mono_passes_through_untouched() {
        let input = vec![0.1, -0.2, 0.3];
        assert_eq!(downmix(&input, 1), input);
    }

    #[test]
    fn stereo_frames_are_averaged() {
        // Two frames: (1.0, 0.0) and (0.5, -0.5).
        let input = vec![1.0, 0.0, 0.5, -0.5];
        assert_eq!(downmix(&input, 2), vec![0.5, 0.0]);
    }

    #[test]
    fn a_trailing_partial_frame_is_discarded_rather_than_skewing_the_mix() {
        let input = vec![1.0, 1.0, 0.5];
        assert_eq!(downmix(&input, 2), vec![1.0]);
    }
}
