//! Does giving the loader more threads make session creation faster?
use kiku_lib::asr::{ModelFiles, ParakeetEngine};
use std::path::PathBuf;
use std::time::Instant;

#[test]
#[ignore = "needs a model installed"]
fn thread_count_versus_load_time() {
    let home = std::env::var_os("HOME").unwrap();
    let dir = PathBuf::from(home)
        .join(".local/share/io.github.akashvaghela09.kiku/models/parakeet-tdt-0.6b-v2");
    if !dir.is_dir() {
        eprintln!("skipping: model not installed");
        return;
    }
    let files = ModelFiles::discover(&dir).unwrap();

    for threads in [1, 2, 4, 8] {
        let t = Instant::now();
        let engine = ParakeetEngine::load_with_threads(&files, "bench", threads).unwrap();
        println!(
            "  threads {threads:>2}: load {:>5} ms",
            t.elapsed().as_millis()
        );
        drop(engine);
    }
}
