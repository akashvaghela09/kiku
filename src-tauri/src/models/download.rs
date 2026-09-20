//! Downloading a model, resumably.
//!
//! A 630 MB download over a domestic connection will be interrupted sooner or later,
//! so each file streams into a `.part` beside its destination and a retry resumes with
//! a Range request rather than starting again. The file is only renamed into place
//! after its SHA-256 matches, which means a complete-looking model on disk is always a
//! correct one.

use std::path::Path;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use tokio::io::AsyncWriteExt;

use super::registry::{ModelSpec, RemoteFile};
use super::store::{ModelStore, PART_SUFFIX};
use crate::error::{Error, Result};

/// Progress for the model as a whole, not the individual file, because that is what a
/// progress bar should show.
///
/// Byte counts cross the boundary as `u32`. Specta rejects `u64` there to prevent
/// silent precision loss, and maps every float to `number | null` because NaN cannot
/// be represented in JSON — so a float would push a null into every call site. `u32`
/// caps a model at four gigabytes, which is far beyond anything a lightweight
/// dictation tool would ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded_bytes: u32,
    pub total_bytes: u32,
}

impl DownloadProgress {
    pub fn new(downloaded: u64, total: u64) -> Self {
        Self {
            downloaded_bytes: downloaded.min(u64::from(u32::MAX)) as u32,
            total_bytes: total.min(u64::from(u32::MAX)) as u32,
        }
    }

    pub fn fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        f64::from(self.downloaded_bytes) as f32 / f64::from(self.total_bytes) as f32
    }
}

/// Download every file of `spec` that is not already present and correct.
///
/// `on_progress` is called as bytes arrive; it should be cheap, since it fires often.
pub async fn download(
    store: &ModelStore,
    spec: &ModelSpec,
    on_progress: impl Fn(DownloadProgress) + Send + Sync,
) -> Result<()> {
    let dir = store.ensure_dir(spec)?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("kiku/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| Error::Internal(format!("could not create the HTTP client: {e}")))?;

    let total_bytes = spec.total_bytes();
    let mut completed_bytes = 0u64;

    for file in spec.files {
        let destination = dir.join(file.path);

        // Already there and the right size: count it and move on. Re-hashing every
        // file on every resume would cost seconds for no benefit.
        if destination.metadata().is_ok_and(|m| m.len() == file.bytes) {
            completed_bytes += file.bytes;
            on_progress(DownloadProgress::new(completed_bytes, total_bytes));
            continue;
        }

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Internal(format!("could not create {}: {e}", parent.display()))
            })?;
        }

        fetch_file(
            &client,
            spec,
            file,
            &destination,
            completed_bytes,
            total_bytes,
            &on_progress,
        )
        .await?;

        completed_bytes += file.bytes;
    }

    // Verify only once everything is present, so a corrupt file is reported against
    // the model rather than interrupting mid-download.
    store.verify(spec)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn fetch_file(
    client: &reqwest::Client,
    spec: &ModelSpec,
    file: &RemoteFile,
    destination: &Path,
    bytes_before: u64,
    total_bytes: u64,
    on_progress: &(impl Fn(DownloadProgress) + Send + Sync),
) -> Result<()> {
    let mut part = destination.as_os_str().to_owned();
    part.push(PART_SUFFIX);
    let part = std::path::PathBuf::from(part);

    // Resume from whatever a previous attempt managed to write.
    let resume_from = part.metadata().map(|m| m.len()).unwrap_or(0);
    let resume_from = if resume_from >= file.bytes {
        0
    } else {
        resume_from
    };

    let url = spec.url_for(file);
    let mut request = client.get(&url);
    if resume_from > 0 {
        tracing::info!(file = file.path, resume_from, "resuming download");
        request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }

    let response = request
        .send()
        .await
        .map_err(|e| Error::Internal(format!("could not reach the model host: {e}")))?;

    if !response.status().is_success() {
        return Err(Error::Internal(format!(
            "the model host refused the download ({})",
            response.status()
        )));
    }

    // A server that ignores our Range header sends 200 and the whole file; writing
    // that onto the end of the part file would silently corrupt it.
    let appending = resume_from > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut written = if appending { resume_from } else { 0 };

    let mut handle = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(appending)
        .truncate(!appending)
        .open(&part)
        .await
        .map_err(|e| Error::Internal(format!("could not write {}: {e}", part.display())))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| Error::Internal(format!("the download was interrupted: {e}")))?;
        handle
            .write_all(&chunk)
            .await
            .map_err(|e| Error::Internal(format!("could not write {}: {e}", part.display())))?;

        written += chunk.len() as u64;
        on_progress(DownloadProgress::new(
            bytes_before + written.min(file.bytes),
            total_bytes,
        ));
    }

    handle.flush().await.map_err(|e| {
        Error::Internal(format!("could not finish writing {}: {e}", part.display()))
    })?;
    drop(handle);

    verify_part(&part, file)?;

    std::fs::rename(&part, destination).map_err(|e| {
        Error::Internal(format!("could not install {}: {e}", destination.display()))
    })?;

    Ok(())
}

/// Hash the completed part file before it is allowed to become the real one.
fn verify_part(part: &Path, file: &RemoteFile) -> Result<()> {
    use std::io::Read;

    let mut handle = std::fs::File::open(part)
        .map_err(|e| Error::Internal(format!("could not read {}: {e}", part.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];

    loop {
        let read = handle
            .read(&mut buffer)
            .map_err(|e| Error::Internal(format!("could not read {}: {e}", part.display())))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
            out
        });

    if actual != file.sha256 {
        // Remove it, so the next attempt starts clean rather than resuming from
        // bytes we already know are wrong.
        let _ = std::fs::remove_file(part);
        return Err(Error::ModelLoad(format!(
            "{} arrived corrupted and was discarded. Try downloading again.",
            file.path
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_fraction_is_zero_rather_than_nan_before_a_total_is_known() {
        let progress = DownloadProgress::new(0, 0);
        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn progress_fraction_tracks_the_ratio() {
        let progress = DownloadProgress::new(315_000_000, 630_000_000);
        assert!((progress.fraction() - 0.5).abs() < 1e-6);
    }
}
