//! NVIDIA Parakeet through sherpa-onnx.
//!
//! The configuration here is not guesswork — each value was established by the chunk 0
//! gate and is wrong in a way that fails quietly if changed:
//!
//! * `model_type` must be `nemo_transducer`. The generic `transducer` path mishandles
//!   Parakeet's TDT duration outputs and produces garbled text rather than an error.
//! * `feature_dim` is 128, not the more common 80. A mismatch yields empty transcripts.
//! * Loading costs about four seconds, so the engine is loaded once at startup and
//!   kept warm for the process lifetime. Loading per dictation would dominate the
//!   latency budget several times over.

use std::path::{Path, PathBuf};
use std::time::Instant;

use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

use super::engine::{transcript, Engine, Transcript};
use crate::audio::TARGET_SAMPLE_RATE;
use crate::error::{Error, Result};

/// Mel bands the Parakeet encoder expects.
const FEATURE_DIM: i32 = 128;

/// Decode threads.
///
/// Chunk 0 measured 6.1x real time on one thread, 15.9x on four and 18.4x on eight —
/// so returns flatten after four. Capping there leaves cores free for the UI drawing
/// the waveform, which is the thing a user would actually notice.
const MAX_THREADS: usize = 4;

/// The four files a sherpa-onnx NeMo transducer export is made of.
#[derive(Debug, Clone)]
pub struct ModelFiles {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
}

impl ModelFiles {
    /// Resolve the files inside a model directory, accepting both the int8 and float
    /// export naming conventions.
    pub fn discover(dir: &Path) -> Result<Self> {
        let pick = |stem: &str| -> Result<PathBuf> {
            [format!("{stem}.int8.onnx"), format!("{stem}.onnx")]
                .into_iter()
                .map(|name| dir.join(name))
                .find(|candidate| candidate.is_file())
                .ok_or_else(|| {
                    Error::ModelLoad(format!("{stem}.onnx is missing from {}", dir.display()))
                })
        };

        let tokens = dir.join("tokens.txt");
        if !tokens.is_file() {
            return Err(Error::ModelLoad(format!(
                "tokens.txt is missing from {}",
                dir.display()
            )));
        }

        Ok(Self {
            encoder: pick("encoder")?,
            decoder: pick("decoder")?,
            joiner: pick("joiner")?,
            tokens,
        })
    }
}

fn decode_threads() -> i32 {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    (cores / 2).clamp(1, MAX_THREADS) as i32
}

pub struct ParakeetEngine {
    recognizer: TransducerRecognizer,
    model_id: String,
}

impl ParakeetEngine {
    pub fn load(files: &ModelFiles, model_id: impl Into<String>) -> Result<Self> {
        let as_str = |path: &Path| -> Result<String> {
            path.to_str().map(str::to_owned).ok_or_else(|| {
                Error::ModelLoad(format!("path is not valid UTF-8: {}", path.display()))
            })
        };

        let threads = decode_threads();
        let started = Instant::now();

        let recognizer = TransducerRecognizer::new(TransducerConfig {
            encoder: as_str(&files.encoder)?,
            decoder: as_str(&files.decoder)?,
            joiner: as_str(&files.joiner)?,
            tokens: as_str(&files.tokens)?,
            model_type: "nemo_transducer".to_owned(),
            decoding_method: "greedy_search".to_owned(),
            num_threads: threads,
            sample_rate: TARGET_SAMPLE_RATE as i32,
            feature_dim: FEATURE_DIM,
            debug: false,
            ..Default::default()
        })
        .map_err(|e| Error::ModelLoad(e.to_string()))?;

        let model_id = model_id.into();
        tracing::info!(
            model = %model_id,
            threads,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "speech model loaded"
        );

        Ok(Self {
            recognizer,
            model_id,
        })
    }
}

impl Engine for ParakeetEngine {
    fn transcribe(&mut self, samples: &[f32]) -> Result<Transcript> {
        let started = Instant::now();
        let text = self.recognizer.transcribe(TARGET_SAMPLE_RATE, samples);
        Ok(transcript(text, samples.len(), started.elapsed()))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_threads_stays_within_the_useful_range() {
        let threads = decode_threads();
        assert!((1..=MAX_THREADS as i32).contains(&threads));
    }

    #[test]
    fn a_directory_without_a_model_is_rejected_by_name() {
        let error = ModelFiles::discover(Path::new("/nonexistent-kiku-model")).unwrap_err();
        assert!(matches!(error, Error::ModelLoad(_)));
        assert!(error.to_string().contains("tokens.txt"));
    }
}
