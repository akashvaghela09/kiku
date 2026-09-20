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

    let Some(profile_dir) = cargo_profile_dir() else {
        return Ok(());
    };
    println!("cargo:rerun-if-changed={}", profile_dir.display());

    for entry in std::fs::read_dir(&profile_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        let wanted = WANTED_EXACT.contains(&name.as_str())
            || (WANTED_PREFIXES.iter().any(|p| name.starts_with(p))
                && (name.contains(".so") || name.ends_with(".dylib") || name.ends_with(".dll")));

        if wanted && entry.path().is_file() {
            std::fs::copy(entry.path(), stage.join(&name))?;
        }
    }

    Ok(())
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

#[allow(dead_code)]
fn unused(_: &Path) {}
