//! Where installed models live on disk, and what state they are in.
//!
//! The store deliberately separates a *cheap* check from an *expensive* one. Presence
//! and size are checked on every startup; hashing 630 MB takes a second or more and
//! runs only after a download or when the user asks for a repair. Treating them the
//! same would either make startup slow or make verification meaningless.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;

use super::registry::{ModelSpec, RemoteFile};
use crate::error::{Error, Result};

/// Suffix for a download still in progress. Kept beside the final file so a resumed
/// download finds it, and so a half-written file can never be mistaken for a complete
/// one.
pub const PART_SUFFIX: &str = ".part";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "state", content = "detail")]
pub enum InstallState {
    /// Nothing on disk.
    Missing,
    /// A download was interrupted; bytes already fetched. See `DownloadProgress` for
    /// why byte counts cross the boundary as `u32`.
    Partial { downloaded: u32 },
    /// All files present at the expected sizes.
    Installed,
}

pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    /// `root` is the application data directory; models live in `<root>/models`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into().join("models"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn dir_for(&self, spec: &ModelSpec) -> PathBuf {
        self.root.join(spec.id)
    }

    pub fn path_for(&self, spec: &ModelSpec, file: &RemoteFile) -> PathBuf {
        self.dir_for(spec).join(file.path)
    }

    /// Cheap check: are all files present at the right size?
    ///
    /// Size is a weak guarantee but a free one, and it catches the common failures —
    /// a truncated download, a file deleted by a disk cleaner. Content corruption is
    /// what `verify` is for.
    pub fn state(&self, spec: &ModelSpec) -> InstallState {
        let mut downloaded = 0u64;
        let mut complete = true;

        for file in spec.files {
            let path = self.path_for(spec, file);
            match path.metadata() {
                Ok(metadata) if metadata.len() == file.bytes => downloaded += file.bytes,
                Ok(metadata) => {
                    downloaded += metadata.len().min(file.bytes);
                    complete = false;
                }
                Err(_) => {
                    // A part file from an interrupted download still counts as progress.
                    let part = path.with_extension_appended(PART_SUFFIX);
                    if let Ok(metadata) = part.metadata() {
                        downloaded += metadata.len().min(file.bytes);
                    }
                    complete = false;
                }
            }
        }

        if complete {
            InstallState::Installed
        } else if downloaded == 0 {
            InstallState::Missing
        } else {
            InstallState::Partial {
                downloaded: downloaded as u32,
            }
        }
    }

    pub fn is_installed(&self, spec: &ModelSpec) -> bool {
        self.state(spec) == InstallState::Installed
    }

    /// Expensive check: hash every file against the pinned SHA-256.
    ///
    /// Run after a download and on an explicit repair, never on startup.
    pub fn verify(&self, spec: &ModelSpec) -> Result<()> {
        for file in spec.files {
            let path = self.path_for(spec, file);
            let actual = hash_file(&path)?;
            if actual != file.sha256 {
                return Err(Error::ModelLoad(format!(
                    "{} failed its integrity check — the download is corrupt.",
                    file.path
                )));
            }
        }
        Ok(())
    }

    /// Remove an installed model, including any interrupted download.
    pub fn delete(&self, spec: &ModelSpec) -> Result<()> {
        let dir = self.dir_for(spec);
        if !dir.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(&dir)
            .map_err(|e| Error::Internal(format!("could not remove {}: {e}", dir.display())))
    }

    pub fn ensure_dir(&self, spec: &ModelSpec) -> Result<PathBuf> {
        let dir = self.dir_for(spec);
        std::fs::create_dir_all(&dir)
            .map_err(|e| Error::Internal(format!("could not create {}: {e}", dir.display())))?;
        Ok(dir)
    }
}

/// SHA-256 of a file, streamed so a 630 MB model never lands in memory at once.
pub fn hash_file(path: &Path) -> Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|e| Error::ModelLoad(format!("could not read {}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| Error::ModelLoad(format!("could not read {}: {e}", path.display())))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// `Path::with_extension` replaces the existing extension; we want to append, so that
/// `encoder.int8.onnx` becomes `encoder.int8.onnx.part` rather than `encoder.int8.part`.
trait AppendExtension {
    fn with_extension_appended(&self, suffix: &str) -> PathBuf;
}

impl AppendExtension for Path {
    fn with_extension_appended(&self, suffix: &str) -> PathBuf {
        let mut name = self.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::registry::DEFAULT;

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kiku-store-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_empty_store_reports_missing() {
        let root = temp_root();
        let store = ModelStore::new(&root);
        assert_eq!(store.state(DEFAULT), InstallState::Missing);
        assert!(!store.is_installed(DEFAULT));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn appending_an_extension_keeps_the_existing_one() {
        let path = Path::new("/models/encoder.int8.onnx");
        assert_eq!(
            path.with_extension_appended(".part"),
            PathBuf::from("/models/encoder.int8.onnx.part")
        );
    }

    #[test]
    fn a_wrong_sized_file_is_partial_not_installed() {
        let root = temp_root();
        let store = ModelStore::new(&root);
        let dir = store.ensure_dir(DEFAULT).unwrap();
        // Write one file at the wrong length.
        std::fs::write(dir.join(DEFAULT.files[3].path), b"not the real tokens").unwrap();

        match store.state(DEFAULT) {
            InstallState::Partial { downloaded } => assert!(downloaded > 0),
            other => panic!("expected Partial, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hashing_matches_a_known_value() {
        let root = temp_root();
        let path = root.join("sample.txt");
        std::fs::write(&path, b"kiku").unwrap();
        // echo -n kiku | sha256sum
        assert_eq!(
            hash_file(&path).unwrap(),
            "8e46f1213aacc2026d80a30789341f9dc6d3dfcc0adabfa0c6ad6cbcba87f38e"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deleting_a_model_that_is_not_there_is_not_an_error() {
        let root = temp_root();
        let store = ModelStore::new(&root);
        assert!(store.delete(DEFAULT).is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }
}
