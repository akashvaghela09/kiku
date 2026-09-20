//! Stage the sherpa-onnx shared libraries before Tauri validates its resources.
//!
//! Those libraries are produced by `sherpa-rs-sys` into cargo's output directory,
//! whose path depends on the profile being built. Pointing `tauri.conf.json` straight
//! at `target/release/` therefore breaks every debug command: `tauri_build::build`
//! validates resource paths on *every* cargo invocation, so `cargo test` on a clean
//! checkout fails with "resource path doesn't exist" before a release build has ever
//! run.
//!
//! Copying them to a fixed directory first makes the resource path stable across
//! profiles and platforms, and lets one configuration cover all three operating
//! systems instead of three files listing different extensions.
//!
//! The copy also goes the other way. CI caches `target/`, and the cache prunes these
//! libraries while leaving cargo convinced the build script is fresh - so a cache hit
//! produced `unable to find library -lonnxruntime` at link time. Restoring them from
//! the staging directory when the profile directory has lost them makes a cached
//! build behave like a clean one, and the staging directory is itself cached.
//!
//! They are also copied next to the test binaries. Windows has no rpath: the loader
//! resolves a DLL from the directory of the executable that needs it, and a test
//! binary lives in `target/<profile>/deps/`. Without a copy there, `cargo test` on
//! Windows died before the harness produced a single line of output, which reads as
//! "test failed" with nothing to go on.

use std::path::{Path, PathBuf};

/// Where the staged copies live, relative to this manifest.
const STAGE_DIR: &str = "resources/lib";

/// Libraries worth carrying. Matched by prefix so each platform's own naming and
/// extension are handled without listing them.
const WANTED_PREFIXES: [&str; 3] = ["libsherpa-onnx", "sherpa-onnx", "libonnxruntime"];
const WANTED_EXACT: [&str; 1] = ["onnxruntime.dll"];

fn main() {
    if let Err(error) = stage_libraries() {
        // Not fatal: a build with no libraries to stage still compiles and runs from
        // the cargo output directory, which is what `cargo run` does anyway. Only
        // bundling needs them, and that failure would be loud.
        println!("cargo:warning=could not stage native libraries: {error}");
    }

    tauri_build::build();
}

fn stage_libraries() -> std::io::Result<()> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let stage = manifest.join(STAGE_DIR);
    std::fs::create_dir_all(&stage)?;

    // Tauri refuses to bundle a resource directory that matches nothing, so keep a
    // placeholder to guarantee the glob always has something to find.
    let placeholder = stage.join(".keep");
    if !placeholder.exists() {
        std::fs::write(&placeholder, b"")?;
    }

    // Always offer the staging directory to the linker. On a cached build the profile
    // directory may no longer hold these libraries, and this is where they survive.
    println!("cargo:rustc-link-search=native={}", stage.display());

    let Some(profile_dir) = cargo_profile_dir() else {
        return Ok(());
    };

    // Gather from everywhere they might be. On Linux they land beside the profile
    // binaries; on Windows they can stay inside the producing crate's OUT_DIR.
    let mut staged = copy_libraries(&profile_dir, &stage)?;
    if staged == 0 {
        for out_dir in dependency_out_dirs(&profile_dir) {
            staged += copy_libraries(&out_dir, &stage)?;
        }
    }
    if staged == 0 {
        // A cached build that pruned them, or one where sherpa-rs-sys has not run.
        // Whatever was staged earlier is the source of truth.
        staged = copy_libraries(&stage, &profile_dir)?;
        println!("cargo:warning=restored {staged} native libraries from {STAGE_DIR}");
    }

    // Windows has been failing with a test binary that dies before the harness runs,
    // which is what a missing DLL looks like, and there is no way to inspect the
    // runner. Report what was found and where so the next CI log answers it.
    println!(
        "cargo:warning=native libraries staged: {staged} (profile dir {})",
        profile_dir.display()
    );
    if let Ok(entries) = std::fs::read_dir(&stage) {
        for entry in entries.filter_map(Result::ok) {
            println!(
                "cargo:warning=  staged: {}",
                entry.file_name().to_string_lossy()
            );
        }
    }

    // Put them everywhere an executable might look. `profile_dir` serves the app
    // binary, `deps` serves the test binaries, and neither platform charges anything
    // meaningful for the duplication.
    copy_libraries(&stage, &profile_dir)?;
    copy_libraries(&stage, &profile_dir.join("deps"))?;

    Ok(())
}

/// `OUT_DIR` of every dependency that has one, under `target/<profile>/build/`.
///
/// Searched because a crate is free to leave its native libraries there rather than
/// copying them beside the profile binaries, and on Windows sherpa-onnx does.
fn dependency_out_dirs(profile_dir: &Path) -> Vec<PathBuf> {
    let build_dir = profile_dir.join("build");
    let Ok(entries) = std::fs::read_dir(&build_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("out"))
        .filter(|path| path.is_dir())
        .collect()
}

/// Copy the native libraries from one directory to another, returning how many moved.
fn copy_libraries(from: &Path, to: &Path) -> std::io::Result<usize> {
    if !from.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(to)?;

    let mut copied = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        let wanted = WANTED_EXACT.contains(&name.as_str())
            || (WANTED_PREFIXES.iter().any(|p| name.starts_with(p))
                && (name.contains(".so") || name.ends_with(".dylib") || name.ends_with(".dll")));

        if wanted && entry.path().is_file() {
            std::fs::copy(entry.path(), to.join(&name))?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// `target/<profile>/`, derived from `OUT_DIR`.
///
/// `OUT_DIR` is `target/<profile>/build/<crate>-<hash>/out`, so the profile directory
/// is three levels up. Deriving it this way keeps debug and release correct without
/// hardcoding either.
fn cargo_profile_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").ok()?);
    let profile = out_dir.parent()?.parent()?.parent()?;
    profile.is_dir().then(|| profile.to_path_buf())
}
