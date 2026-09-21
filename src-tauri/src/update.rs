//! Noticing that a newer Kiku exists.
//!
//! Kiku does not update itself. This asks GitHub once a day whether a newer release
//! is published; the window then shows a badge beside the name and a row in settings,
//! both of which do nothing but open the release page. Nothing here downloads,
//! installs, or runs anything. The result of that fetch is remote
//! data used for exactly one purpose: comparing a version string and displaying a URL.
//!
//! The check is the only network traffic Kiku makes after the model is installed, so
//! it is cached to once a day, fails silently when offline, and can be switched off
//! entirely. An offline-first product that pings a server on every launch is not
//! offline-first.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{Error, Result};

const RELEASES_API: &str = "https://api.github.com/repos/akashvaghela09/kiku/releases/latest";

/// How long a check is trusted before asking again.
const CACHE_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Ceiling on the request, so a hanging server cannot hold a background task open.
const TIMEOUT: Duration = Duration::from_secs(10);

/// What the UI shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "state", content = "detail")]
pub enum UpdateStatus {
    /// Running the newest published release, or a build ahead of it.
    UpToDate,
    /// A newer release exists.
    Available { version: String, url: String },
    /// Nothing known - offline, switched off, or not yet checked.
    Unknown,
}

/// The subset of GitHub's release payload we use. Everything else is ignored.
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedCheck {
    checked_at_secs: u64,
    status: UpdateStatus,
}

fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("update-check.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0)
}

/// Read a cached result if it is still fresh.
pub fn cached(data_dir: &Path) -> Option<UpdateStatus> {
    let raw = std::fs::read_to_string(cache_path(data_dir)).ok()?;
    let cached: CachedCheck = serde_json::from_str(&raw).ok()?;

    let age = now_secs().saturating_sub(cached.checked_at_secs);
    (age < CACHE_FOR.as_secs()).then_some(cached.status)
}

fn store(data_dir: &Path, status: &UpdateStatus) {
    let cached = CachedCheck {
        checked_at_secs: now_secs(),
        status: status.clone(),
    };
    if let Ok(encoded) = serde_json::to_string(&cached) {
        // A cache that cannot be written only means checking again tomorrow.
        let _ = std::fs::write(cache_path(data_dir), encoded);
    }
}

/// Check GitHub for a newer release, using the cache unless `force` is set.
pub async fn check(data_dir: &Path, current: &str, force: bool) -> Result<UpdateStatus> {
    if !force {
        if let Some(status) = cached(data_dir) {
            return Ok(status);
        }
    }

    let status = fetch(current).await?;
    store(data_dir, &status);
    Ok(status)
}

async fn fetch(current: &str) -> Result<UpdateStatus> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("kiku/", env!("CARGO_PKG_VERSION")))
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| Error::Internal(error.to_string()))?;

    let response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| Error::Internal(format!("could not reach GitHub: {error}")))?;

    if !response.status().is_success() {
        return Err(Error::Internal(format!(
            "GitHub answered {}",
            response.status()
        )));
    }

    let release: GithubRelease = response
        .json()
        .await
        .map_err(|error| Error::Internal(format!("could not read the release: {error}")))?;

    Ok(compare(current, &release))
}

/// Decide what a release means for the running build.
///
/// Separated from the fetch so the version logic is testable without a network.
fn compare(current: &str, release: &GithubRelease) -> UpdateStatus {
    if release.draft || release.prerelease {
        return UpdateStatus::UpToDate;
    }

    let Ok(current) = Version::parse(current.trim_start_matches('v')) else {
        return UpdateStatus::Unknown;
    };
    let Ok(latest) = Version::parse(release.tag_name.trim_start_matches('v')) else {
        // A tag that is not semver tells us nothing; saying so beats guessing.
        return UpdateStatus::Unknown;
    };

    if latest > current {
        UpdateStatus::Available {
            version: latest.to_string(),
            url: release.html_url.clone(),
        }
    } else {
        UpdateStatus::UpToDate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_owned(),
            html_url: "https://github.com/akashvaghela09/kiku/releases/tag/x".to_owned(),
            draft: false,
            prerelease: false,
        }
    }

    #[test]
    fn a_newer_release_is_offered() {
        match compare("0.1.0", &release("v0.2.0")) {
            UpdateStatus::Available { version, url } => {
                assert_eq!(version, "0.2.0");
                assert!(url.starts_with("https://github.com/"));
            }
            other => panic!("expected an update, got {other:?}"),
        }
    }

    #[test]
    fn the_same_version_is_up_to_date() {
        assert_eq!(compare("0.1.0", &release("v0.1.0")), UpdateStatus::UpToDate);
    }

    #[test]
    fn a_local_build_ahead_of_the_release_is_not_asked_to_downgrade() {
        assert_eq!(compare("0.3.0", &release("v0.2.0")), UpdateStatus::UpToDate);
    }

    #[test]
    fn tags_with_and_without_a_v_prefix_both_work() {
        assert!(matches!(
            compare("0.1.0", &release("0.2.0")),
            UpdateStatus::Available { .. }
        ));
    }

    #[test]
    fn drafts_and_prereleases_are_ignored() {
        let mut draft = release("v9.9.9");
        draft.draft = true;
        assert_eq!(compare("0.1.0", &draft), UpdateStatus::UpToDate);

        let mut prerelease = release("v9.9.9");
        prerelease.prerelease = true;
        assert_eq!(compare("0.1.0", &prerelease), UpdateStatus::UpToDate);
    }

    #[test]
    fn a_tag_that_is_not_semver_yields_unknown_rather_than_a_guess() {
        assert_eq!(compare("0.1.0", &release("nightly")), UpdateStatus::Unknown);
    }

    #[test]
    fn a_patch_release_still_counts() {
        assert!(matches!(
            compare("0.1.0", &release("v0.1.1")),
            UpdateStatus::Available { .. }
        ));
    }

    #[test]
    fn a_fresh_cache_is_returned_and_a_stale_one_is_not() {
        let dir = std::env::temp_dir().join(format!("kiku-update-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        store(&dir, &UpdateStatus::UpToDate);
        assert_eq!(cached(&dir), Some(UpdateStatus::UpToDate));

        // Backdate the check beyond the cache window.
        let stale = CachedCheck {
            checked_at_secs: now_secs() - CACHE_FOR.as_secs() - 1,
            status: UpdateStatus::UpToDate,
        };
        std::fs::write(cache_path(&dir), serde_json::to_string(&stale).unwrap()).unwrap();
        assert_eq!(cached(&dir), None, "a stale cache must not be trusted");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_corrupt_cache_is_simply_absent() {
        let dir = std::env::temp_dir().join(format!("kiku-update-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(cached(&dir), None);

        std::fs::write(cache_path(&dir), b"{ not json").unwrap();
        assert_eq!(cached(&dir), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
