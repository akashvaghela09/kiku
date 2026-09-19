//! Thin wrapper over `sherpa-rs`'s offline transducer recogniser, holding the
//! Parakeet-specific configuration in one place.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

use crate::audio::TARGET_SAMPLE_RATE;

/// Files that make up a NeMo transducer export.
#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub joiner: PathBuf,
    pub tokens: PathBuf,
}

impl ModelPaths {
    /// Resolve the four files inside a sherpa-onnx model directory, accepting both the
    /// int8 and float export naming conventions.
    pub fn discover(dir: &Path) -> Result<Self> {
        let pick = |stem: &str| -> Result<PathBuf> {
            for name in [format!("{stem}.int8.onnx"), format!("{stem}.onnx")] {
                let candidate = dir.join(&name);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
            bail!("no {stem}.onnx or {stem}.int8.onnx in {}", dir.display());
        };

        let tokens = dir.join("tokens.txt");
        if !tokens.is_file() {
            bail!("no tokens.txt in {}", dir.display());
        }

        Ok(Self {
            encoder: pick("encoder")?,
            decoder: pick("decoder")?,
            joiner: pick("joiner")?,
            tokens,
        })
    }

    pub fn total_bytes(&self) -> u64 {
        [&self.encoder, &self.decoder, &self.joiner, &self.tokens]
            .iter()
            .filter_map(|p| p.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

pub struct Transcription {
    pub text: String,
    pub elapsed_secs: f64,
    pub audio_secs: f64,
}

impl Transcription {
    /// Real-time factor: processing time divided by audio duration. Below 1.0 means
    /// faster than real time; 0.1 means ten times faster.
    pub fn rtf(&self) -> f64 {
        if self.audio_secs <= 0.0 {
            return f64::NAN;
        }
        self.elapsed_secs / self.audio_secs
    }
}

pub struct Engine {
    recognizer: TransducerRecognizer,
    pub load_secs: f64,
}

impl Engine {
    pub fn load(paths: &ModelPaths, num_threads: i32, feature_dim: i32) -> Result<Self> {
        let to_str = |p: &Path| -> Result<String> {
            p.to_str()
                .map(str::to_owned)
                .with_context(|| format!("model path is not valid UTF-8: {}", p.display()))
        };

        let started = Instant::now();
        let recognizer = TransducerRecognizer::new(TransducerConfig {
            encoder: to_str(&paths.encoder)?,
            decoder: to_str(&paths.decoder)?,
            joiner: to_str(&paths.joiner)?,
            tokens: to_str(&paths.tokens)?,
            // NeMo transducers need their own decoding path; the generic "transducer"
            // default mis-handles Parakeet's TDT duration outputs.
            model_type: "nemo_transducer".to_owned(),
            decoding_method: "greedy_search".to_owned(),
            num_threads,
            sample_rate: TARGET_SAMPLE_RATE as i32,
            feature_dim,
            debug: false,
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("creating the sherpa-onnx offline recogniser")?;

        Ok(Self {
            recognizer,
            load_secs: started.elapsed().as_secs_f64(),
        })
    }

    /// Transcribe 16 kHz mono f32 samples.
    pub fn transcribe(&mut self, samples: &[f32]) -> Transcription {
        let started = Instant::now();
        let text = self.recognizer.transcribe(TARGET_SAMPLE_RATE, samples);
        Transcription {
            text: text.trim().to_owned(),
            elapsed_secs: started.elapsed().as_secs_f64(),
            audio_secs: samples.len() as f64 / f64::from(TARGET_SAMPLE_RATE),
        }
    }
}
