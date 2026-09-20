//! Measures what a model switch actually costs, so the fix targets the real cost.
use kiku_lib::asr::{AsrService, ModelFiles};
use std::path::PathBuf;
use std::time::Instant;

fn dir(id: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let p = PathBuf::from(home)
        .join(".local/share/io.github.akashvaghela09.kiku/models")
        .join(id);
    p.is_dir().then_some(p)
}

#[test]
#[ignore = "needs both models installed"]
fn time_a_model_switch() {
    let (Some(big), Some(small)) = (
        dir("parakeet-tdt-0.6b-v2"),
        dir("parakeet-tdt-transducer-110m"),
    ) else {
        eprintln!("skipping: both models must be installed");
        return;
    };

    let service = AsrService::new();

    let t = Instant::now();
    let files = ModelFiles::discover(&big).unwrap();
    println!("  discover (Standard):  {:>6} ms", t.elapsed().as_millis());

    let t = Instant::now();
    service.load(&files, "parakeet-tdt-0.6b-v2").unwrap();
    println!("  load    (Standard):   {:>6} ms", t.elapsed().as_millis());

    // The switch: release the loaded model, then load the other.
    let t = Instant::now();
    let small_files = ModelFiles::discover(&small).unwrap();
    let discover_ms = t.elapsed().as_millis();

    let t = Instant::now();
    service
        .load(&small_files, "parakeet-tdt-transducer-110m")
        .unwrap();
    let switch_ms = t.elapsed().as_millis();

    println!("  discover (Compact):   {:>6} ms", discover_ms);
    println!(
        "  SWITCH to Compact:    {:>6} ms   <- what the user waits for",
        switch_ms
    );

    // And back.
    let t = Instant::now();
    service.load(&files, "parakeet-tdt-0.6b-v2").unwrap();
    println!("  SWITCH to Standard:   {:>6} ms", t.elapsed().as_millis());
}
