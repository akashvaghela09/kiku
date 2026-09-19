//! The catalogue of speech models Kiku knows how to install.
//!
//! Every entry pins a specific Hugging Face revision rather than a branch, so an
//! upstream change can never silently alter what gets downloaded, and every file
//! carries the SHA-256 it must hash to. The hashes below are Hugging Face's own LFS
//! object ids, verified against a local download.
//!
//! There is one entry today. Chunk 0 measured the 0.6B model at six times real time on
//! a *single* thread, so the lighter 110M variant was deferred: a machine too slow for
//! this is too slow for a webview. The shape stays plural because adding a second entry
//! should be data, not code.

/// One file within a model export.
#[derive(Debug, Clone, Copy)]
pub struct RemoteFile {
    /// Path within the repository, and the name it keeps on disk.
    pub path: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    /// Stable identifier, used as the on-disk directory name and in settings.
    pub id: &'static str,
    /// Shown in the UI. Deliberately not a size or a parameter count — those ask the
    /// user a question they have no way to answer.
    pub name: &'static str,
    pub summary: &'static str,
    pub repo: &'static str,
    /// Pinned commit. Never a branch name.
    pub revision: &'static str,
    pub files: &'static [RemoteFile],
}

impl ModelSpec {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.bytes).sum()
    }

    /// Direct download URL for a file at the pinned revision.
    pub fn url_for(&self, file: &RemoteFile) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.revision, file.path
        )
    }
}

pub const PARAKEET_060B_V2: ModelSpec = ModelSpec {
    id: "parakeet-tdt-0.6b-v2",
    name: "Standard",
    summary: "English. Accurate, and fast enough to feel instant on any recent machine.",
    repo: "csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8",
    revision: "1ab9323565ddb038682214b292f588070a538ce2",
    files: &[
        RemoteFile {
            path: "encoder.int8.onnx",
            sha256: "a32b12d17bbbc309d0686fbbcc2987b5e9b8333a7da83fa6b089f0a2acd651ab",
            bytes: 652_184_296,
        },
        RemoteFile {
            path: "decoder.int8.onnx",
            sha256: "b6bb64963457237b900e496ee9994b59294526439fbcc1fecf705b31a15c6b4e",
            bytes: 7_257_753,
        },
        RemoteFile {
            path: "joiner.int8.onnx",
            sha256: "7946164367946e7f9f29a122407c3252b680dbae9a51343eb2488d057c3c43d2",
            bytes: 1_739_080,
        },
        RemoteFile {
            path: "tokens.txt",
            sha256: "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
            bytes: 9_384,
        },
    ],
};

/// Everything installable, in the order Settings should present it.
pub const ALL: &[ModelSpec] = &[PARAKEET_060B_V2];

/// The model a fresh install gets.
pub const DEFAULT: &ModelSpec = &PARAKEET_060B_V2;

pub fn find(id: &str) -> Option<&'static ModelSpec> {
    ALL.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_model_is_in_the_catalogue() {
        assert!(find(DEFAULT.id).is_some());
    }

    #[test]
    fn identifiers_are_unique() {
        let mut ids: Vec<_> = ALL.iter().map(|spec| spec.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate model id");
    }

    #[test]
    fn every_file_pins_a_full_sha256() {
        for spec in ALL {
            for file in spec.files {
                assert_eq!(file.sha256.len(), 64, "{} {}", spec.id, file.path);
                assert!(
                    file.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                    "{} {} is not hex",
                    spec.id,
                    file.path
                );
                assert!(file.bytes > 0, "{} {} has no size", spec.id, file.path);
            }
        }
    }

    #[test]
    fn revisions_are_commit_hashes_not_branch_names() {
        for spec in ALL {
            assert_eq!(spec.revision.len(), 40, "{} is not pinned", spec.id);
            assert!(spec.revision.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn urls_point_at_the_pinned_revision() {
        let spec = DEFAULT;
        let url = spec.url_for(&spec.files[0]);
        assert!(url.contains(spec.revision), "{url}");
        assert!(url.starts_with("https://huggingface.co/"), "{url}");
    }

    #[test]
    fn the_default_model_declares_a_plausible_total_size() {
        // Around 630 MB. A wildly different number means an entry was edited by hand.
        let mb = DEFAULT.total_bytes() / 1_048_576;
        assert!((600..700).contains(&mb), "unexpected total: {mb} MB");
    }
}
