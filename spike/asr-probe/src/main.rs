//! Chunk 0 gate for Kiku.
//!
//! Proves, before any application code exists, that:
//!   1. the Parakeet ONNX exports load through the Rust sherpa-onnx bindings,
//!   2. CPU-only inference is fast enough to feel instant, and
//!   3. accuracy holds up on the owner's own voice.
//!
//! If any of those fail, the Whisper decision reopens.

mod audio;
mod capture;
mod engine;

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use engine::{Engine, ModelPaths};

const DEFAULT_MODEL_DIR: &str = ".cache/kiku/models/parakeet-tdt-0.6b-v2-int8";

#[derive(Parser)]
#[command(name = "asr-probe", about = "Kiku chunk 0 - Parakeet engine gate")]
struct Cli {
    /// Directory holding encoder/decoder/joiner/tokens.
    #[arg(long, global = true)]
    model_dir: Option<PathBuf>,

    /// Inference threads. 0 picks half the available cores.
    #[arg(long, global = true, default_value_t = 0)]
    threads: i32,

    /// Mel feature dimension expected by the encoder.
    #[arg(long, global = true, default_value_t = 128)]
    feature_dim: i32,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the available microphones.
    Devices,
    /// Transcribe a WAV file.
    File { path: PathBuf },
    /// Record from the microphone, then transcribe.
    Mic {
        #[arg(short, long, default_value_t = 5)]
        seconds: u64,
        #[arg(short, long)]
        device: Option<String>,
    },
    /// Measure the real-time factor across clip lengths.
    Bench {
        /// Source clip, repeated and trimmed to hit each target length.
        path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', default_value = "2,5,15,60")]
        lengths: Vec<u64>,
    },
}

fn resolve_model_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir);
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(DEFAULT_MODEL_DIR))
}

fn resolve_threads(requested: i32) -> i32 {
    if requested > 0 {
        return requested;
    }
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    (cores / 2).max(1)
}

fn load_engine(cli: &Cli) -> Result<Engine> {
    let dir = resolve_model_dir(cli.model_dir.clone())?;
    let paths = ModelPaths::discover(&dir)?;
    let threads = resolve_threads(cli.threads);

    println!("model     {}", dir.display());
    println!(
        "size      {:.0} MB",
        paths.total_bytes() as f64 / 1_048_576.0
    );
    println!("threads   {threads}   feature_dim {}", cli.feature_dim);

    let engine = Engine::load(&paths, threads, cli.feature_dim)?;
    println!("loaded    {:.2}s\n", engine.load_secs);
    Ok(engine)
}

fn report(label: &str, t: &engine::Transcription) {
    println!("--- {label} ---");
    println!(
        "audio {:.2}s   decode {:.2}s   RTF {:.3}  ({:.1}x real time)",
        t.audio_secs,
        t.elapsed_secs,
        t.rtf(),
        1.0 / t.rtf()
    );
    if t.text.is_empty() {
        println!("text  <empty - silence or a failed decode>");
    } else {
        println!("text  {}", t.text);
    }
    println!();
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Devices => {
            for name in capture::list_input_devices()? {
                println!("  {name}");
            }
        }

        Command::File { path } => {
            let mut engine = load_engine(&cli)?;
            let samples = audio::load_wav_16k_mono(
                path.to_str().context("WAV path is not valid UTF-8")?,
            )?;
            let result = engine.transcribe(&samples);
            report(&path.display().to_string(), &result);
        }

        Command::Mic { seconds, device } => {
            let mut engine = load_engine(&cli)?;
            println!("recording {seconds}s - speak now");

            let captured = capture::record(
                Duration::from_secs(*seconds),
                device.as_deref(),
                |rms| {
                    // Twenty-column meter, so a silent microphone is obvious.
                    let bars = ((rms * 60.0).min(1.0) * 20.0) as usize;
                    print!("\r  [{:<20}]", "#".repeat(bars));
                    let _ = std::io::stdout().flush();
                },
            )?;
            println!("\r  {:<24}", "");

            let (peak, rms) = audio::levels(&captured.samples);
            println!(
                "device    {} @ {} Hz, {} ch",
                captured.device_name, captured.device_rate, captured.device_channels
            );
            println!("levels    peak {peak:.3}  rms {rms:.4}");
            if peak < 0.01 {
                println!("warning   the microphone captured near-silence");
            }
            println!();

            let result = engine.transcribe(&captured.samples);
            report("microphone", &result);
        }

        Command::Bench { path, lengths } => {
            let mut engine = load_engine(&cli)?;
            let source = match path {
                Some(p) => p.clone(),
                None => resolve_model_dir(cli.model_dir.clone())?.join("test_wavs/0.wav"),
            };
            let base = audio::load_wav_16k_mono(
                source.to_str().context("WAV path is not valid UTF-8")?,
            )?;

            // Warm up first: the first decode pays one-off allocation costs that would
            // otherwise be blamed on the shortest clip.
            let _ = engine.transcribe(&base);

            println!("{:>8}  {:>9}  {:>7}  {:>12}", "clip", "decode", "RTF", "vs realtime");
            for seconds in lengths {
                let wanted = (*seconds as usize) * audio::TARGET_SAMPLE_RATE as usize;
                let padded: Vec<f32> = base.iter().copied().cycle().take(wanted).collect();
                let result = engine.transcribe(&padded);
                println!(
                    "{:>7}s  {:>8.2}s  {:>7.3}  {:>11.1}x",
                    seconds,
                    result.elapsed_secs,
                    result.rtf(),
                    1.0 / result.rtf()
                );
            }
        }
    }

    Ok(())
}
