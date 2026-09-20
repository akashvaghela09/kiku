//! One-off: renders the generated failure cue to the asset it ships as.
//!
//! Run with `cargo test render_error_cue -- --ignored` after changing the generator
//! in `sound::tone`.
#[test]
#[ignore = "rewrites a committed asset"]
fn write_error_wav() {
    let samples = kiku_lib::sound::render_error_cue();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/sounds/error.wav");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: kiku_lib::sound::SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("creating the WAV");
    for sample in samples {
        writer
            .write_sample((sample * f32::from(i16::MAX)) as i16)
            .expect("writing a sample");
    }
    writer.finalize().expect("finalising the WAV");
    println!("wrote {path}");
}
