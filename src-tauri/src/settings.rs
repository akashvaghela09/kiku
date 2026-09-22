//! The settings that survive a restart.
//!
//! One JSON file beside the history database. Everything the user can change from the
//! interface lives in it: preferences, the two hotkey bindings, and the chosen
//! microphone.
//!
//! Until this existed, none of that was written anywhere. `AppState` seeded itself
//! from `Preferences::default()` and the setters only ever updated a mutex, so every
//! setting - theme, hotkeys, the lot - came back to its default on the next launch,
//! and the data directory held a history database and a downloaded model but no record
//! that any choice had been made.
//!
//! Reading is deliberately forgiving. A settings file is not worth failing a launch
//! over, and one written by a newer version must not stop an older one from starting,
//! so anything unreadable falls back to defaults section by section: a preferences
//! block this version cannot parse costs the preferences, not the hotkeys.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::hotkeys::HotkeyBindings;
use crate::state::Preferences;

/// Name of the file inside the application data directory.
const FILE: &str = "settings.json";

/// Everything that is written out.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub preferences: Preferences,
    pub hotkeys: HotkeyBindings,
    /// Device id of the chosen microphone; `None` follows the system default.
    pub microphone: Option<String>,
}

impl Settings {
    /// Where the file lives.
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join(FILE)
    }

    /// Read the settings, falling back to defaults for anything unreadable.
    ///
    /// A missing file is the ordinary first-run case and is not logged as a problem.
    pub fn load(data_dir: &Path) -> Self {
        let path = Self::path(data_dir);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "settings could not be read");
                return Self::default();
            }
        };

        // Parsed section by section rather than all at once. A single `from_str` makes
        // one unparseable field discard every setting in the file, including the ones
        // this version understands perfectly well.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            tracing::warn!(path = %path.display(), "settings are not valid JSON; using defaults");
            return Self::default();
        };

        Self {
            preferences: section(&value, "preferences"),
            hotkeys: section(&value, "hotkeys"),
            microphone: section(&value, "microphone"),
        }
    }

    /// Write the settings out, replacing whatever was there.
    ///
    /// Written to a temporary file and renamed over the old one, so a crash or a full
    /// disk midway leaves the previous settings intact rather than a half-written file
    /// that the next launch would discard entirely.
    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        let path = Self::path(data_dir);
        let staging = path.with_extension("json.tmp");

        let text = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

        std::fs::write(&staging, text)?;
        std::fs::rename(&staging, &path)
    }
}

/// One section of the file, or its default if it is absent or unreadable.
fn section<T: Default + serde::de::DeserializeOwned>(value: &serde_json::Value, key: &str) -> T {
    let Some(found) = value.get(key) else {
        return T::default();
    };

    serde_json::from_value(found.clone()).unwrap_or_else(|error| {
        tracing::warn!(%error, key, "this setting could not be read; using its default");
        T::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kiku-settings-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_first_run_has_no_file_and_gets_the_defaults() {
        let dir = temp_dir("first-run");
        let settings = Settings::load(&dir);
        assert_eq!(settings.preferences, Preferences::default());
        assert_eq!(settings.hotkeys, HotkeyBindings::default());
        assert!(settings.microphone.is_none());
    }

    #[test]
    fn what_is_written_is_what_comes_back() {
        let dir = temp_dir("round-trip");

        let mut settings = Settings::default();
        settings.preferences.auto_paste = false;
        settings.preferences.retention_days = Some(30);
        settings.microphone = Some("a-particular-microphone".into());
        settings.save(&dir).unwrap();

        let reloaded = Settings::load(&dir);
        assert_eq!(reloaded.preferences, settings.preferences);
        assert_eq!(reloaded.hotkeys, settings.hotkeys);
        assert_eq!(
            reloaded.microphone.as_deref(),
            Some("a-particular-microphone")
        );
    }

    #[test]
    fn an_unreadable_section_costs_only_that_section() {
        let dir = temp_dir("partial");

        // Valid JSON, but `preferences` is the wrong shape entirely.
        std::fs::write(
            Settings::path(&dir),
            r#"{"preferences": "not an object", "microphone": "kept"}"#,
        )
        .unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(
            settings.preferences,
            Preferences::default(),
            "an unreadable section falls back"
        );
        assert_eq!(
            settings.microphone.as_deref(),
            Some("kept"),
            "a readable one beside it survives"
        );
    }

    #[test]
    fn a_file_that_is_not_json_at_all_is_survivable() {
        let dir = temp_dir("corrupt");
        std::fs::write(Settings::path(&dir), "{{ not json").unwrap();
        assert_eq!(Settings::load(&dir).preferences, Preferences::default());
    }

    #[test]
    fn saving_leaves_no_temporary_file_behind() {
        let dir = temp_dir("staging");
        Settings::default().save(&dir).unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();

        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
    }
}
