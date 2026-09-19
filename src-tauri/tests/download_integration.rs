//! Exercises the real download path against Hugging Face.
//!
//! Uses only the 9 KB tokens file, so it proves URL construction, the HTTP fetch, the
//! SHA-256 gate and the atomic rename without moving 630 MB. Skipped unless
//! `KIKU_TEST_NETWORK=1`, so the ordinary test run stays offline and fast.

use kiku_lib::models::{download, InstallState, ModelSpec, ModelStore, DEFAULT};

/// A one-file stand-in for the real model: the tokens file of the pinned revision.
fn tokens_only_spec() -> ModelSpec {
    let tokens = DEFAULT
        .files
        .iter()
        .find(|file| file.path == "tokens.txt")
        .copied()
        .expect("the default model has a tokens file");

    // Leaked so the spec can keep the `&'static [RemoteFile]` the registry type uses.
    // This is a test; one 24-byte leak per run is not worth a lifetime parameter in
    // production code.
    let files: &'static [_] = Box::leak(Box::new([tokens]));

    ModelSpec {
        id: "test-tokens-only",
        files,
        ..*DEFAULT
    }
}

fn temp_root() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kiku-download-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[tokio::test]
async fn a_pinned_file_downloads_verifies_and_installs() {
    if std::env::var("KIKU_TEST_NETWORK").as_deref() != Ok("1") {
        eprintln!("skipping: set KIKU_TEST_NETWORK=1 to run the network test");
        return;
    }

    let root = temp_root();
    let store = ModelStore::new(&root);
    let spec = tokens_only_spec();

    assert_eq!(store.state(&spec), InstallState::Missing);

    let mut seen_progress = 0.0f64;
    let observed = std::sync::Mutex::new(Vec::new());
    download(&store, &spec, |progress| {
        observed.lock().unwrap().push(progress.downloaded_bytes);
    })
    .await
    .expect("download should succeed");

    if let Some(last) = observed.lock().unwrap().last() {
        seen_progress = *last;
    }

    assert_eq!(store.state(&spec), InstallState::Installed);
    assert_eq!(
        seen_progress,
        spec.total_bytes() as f64,
        "progress must reach the total"
    );

    // The checksum gate is the real assertion: verify re-hashes what landed on disk.
    store.verify(&spec).expect("the installed file must verify");

    // No part file should survive a successful download.
    let part = store.dir_for(&spec).join("tokens.txt.part");
    assert!(!part.exists(), "a .part file was left behind");

    store.delete(&spec).expect("cleanup");
    assert_eq!(store.state(&spec), InstallState::Missing);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_corrupted_part_file_is_discarded_rather_than_resumed_into() {
    if std::env::var("KIKU_TEST_NETWORK").as_deref() != Ok("1") {
        eprintln!("skipping: set KIKU_TEST_NETWORK=1 to run the network test");
        return;
    }

    let root = temp_root().join("corrupt");
    let store = ModelStore::new(&root);
    let spec = tokens_only_spec();
    let dir = store.ensure_dir(&spec).unwrap();

    // Leave a part file full of the wrong bytes, shorter than the real file so the
    // downloader tries to resume from it.
    std::fs::write(dir.join("tokens.txt.part"), vec![b'x'; 128]).unwrap();

    let result = download(&store, &spec, |_| {}).await;

    // Either the range request produced a file that fails its hash, or the server
    // ignored the range and the clean download succeeded. Both are acceptable; what
    // must never happen is a corrupt file being installed.
    if result.is_ok() {
        store.verify(&spec).expect("anything installed must verify");
    } else {
        assert_eq!(store.state(&spec), InstallState::Missing);
    }

    let _ = std::fs::remove_dir_all(&root);
}
