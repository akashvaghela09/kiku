//! End-to-end check of the real recogniser.
//!
//! Skipped automatically when the model is not on this machine, so `cargo test` stays
//! fast and works on a fresh checkout. Point `KIKU_TEST_MODEL_DIR` at a sherpa-onnx
//! Parakeet export to run it, or let it find the development default.

use std::path::PathBuf;

use kiku_lib::asr::{AsrService, ModelFiles};

const DEV_MODEL_DIR: &str = ".cache/kiku/models/parakeet-tdt-0.6b-v2-int8";

fn model_dir() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("KIKU_TEST_MODEL_DIR") {
        let path = PathBuf::from(explicit);
        return path.is_dir().then_some(path);
    }
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home).join(DEV_MODEL_DIR);
    path.is_dir().then_some(path)
}

/// Read a 16 kHz mono WAV as f32.
fn read_wav(path: &std::path::Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("opening the reference clip");
    assert_eq!(reader.spec().sample_rate, 16_000, "clip must be 16 kHz");
    assert_eq!(reader.spec().channels, 1, "clip must be mono");

    let scale = 1.0 / f32::from(i16::MAX);
    reader
        .samples::<i16>()
        .map(|sample| sample.expect("reading a sample") as f32 * scale)
        .collect()
}

#[test]
fn the_reference_clip_transcribes_through_the_service() {
    let Some(dir) = model_dir() else {
        eprintln!("skipping: no model installed (set KIKU_TEST_MODEL_DIR to run)");
        return;
    };

    let clip = dir.join("test_wavs/0.wav");
    if !clip.is_file() {
        eprintln!("skipping: the export has no test_wavs/0.wav");
        return;
    }

    let files = ModelFiles::discover(&dir).expect("discovering the model files");
    let service = AsrService::new();
    service
        .load(&files, "parakeet-tdt-0.6b-v2")
        .expect("loading the model");
    assert!(service.is_ready());

    let samples = read_wav(&clip);
    let transcript = service.transcribe(&samples).expect("transcribing");

    println!(
        "text: {}\naudio {} ms, decode {} ms, RTF {:.3}",
        transcript.text,
        transcript.audio_ms,
        transcript.decode_ms,
        transcript.real_time_factor(),
    );

    assert!(!transcript.is_empty(), "the recogniser returned nothing");

    // The clip is a line of narrated prose. Checking a couple of distinctive words
    // rather than the whole string keeps this from breaking on a model update while
    // still failing loudly if the decode path regresses to garbage.
    let lowered = transcript.text.to_lowercase();
    assert!(
        lowered.contains("portrait") && lowered.contains("wish"),
        "unexpected transcription: {}",
        transcript.text
    );

    // Chunk 0 measured RTF around 0.05. Anything approaching real time means the
    // configuration has regressed - most likely threads or the decoding method.
    assert!(
        transcript.real_time_factor() < 0.5,
        "decoding got drastically slower: RTF {:.3}",
        transcript.real_time_factor()
    );
}
