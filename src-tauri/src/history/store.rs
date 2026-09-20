//! Reading and writing the dictation history.
//!
//! Text only — no audio is ever stored. That is a scope decision with a privacy
//! consequence worth stating: everything a user dictates ends up here in plain text,
//! including things they did not mean to keep. The store therefore treats deletion and
//! pausing as first-class operations rather than afterthoughts.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::schema::{database, migrate};
use crate::error::{Error, Result};

/// A stored dictation.
///
/// The wire types are chosen so nothing crosses the boundary as a float. Specta maps
/// every Rust float to `number | null`, because NaN and infinity serialise as null,
/// and threading that null through the whole interface would be a poor trade for
/// values that are conceptually integers and instants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// SQLite rowid. `u32` allows four billion dictations, which at a thousand a day
    /// is eleven thousand years.
    pub id: u32,
    pub text: String,
    /// RFC 3339 in UTC — self-describing, exact, and parsed natively by `Date`.
    pub created_at: String,
    pub audio_ms: u32,
    pub decode_ms: u32,
    pub model_id: Option<String>,
}

/// A page of history.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub entries: Vec<Entry>,
    /// Total matching rows, so the UI can say "showing 50 of 1,204".
    pub total: u32,
}

pub struct History {
    connection: Mutex<Connection>,
}

impl History {
    /// Open (or create) the history database at `path`, migrating it to the current
    /// schema.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(database)?;
        }

        let mut connection = Connection::open(path).map_err(database)?;
        migrate(&mut connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let mut connection = Connection::open_in_memory().map_err(database)?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Record a dictation, returning its id.
    pub fn insert(
        &self,
        text: &str,
        audio_ms: u32,
        decode_ms: u32,
        model_id: Option<&str>,
    ) -> Result<i64> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO transcripts (text, created_at, audio_ms, decode_ms, model_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![text, now_millis(), audio_ms, decode_ms, model_id],
            )
            .map_err(database)?;
        Ok(connection.last_insert_rowid())
    }

    /// Most recent entries first, optionally filtered by a full-text query.
    pub fn list(&self, query: Option<&str>, limit: u32, offset: u32) -> Result<Page> {
        let connection = self.lock()?;
        let limit = limit.clamp(1, 500);

        // Sanitise first: a search box containing only punctuation reduces to nothing,
        // and an empty FTS5 MATCH is a syntax error rather than a match-everything.
        let pattern = query.map(fts_query).filter(|pattern| !pattern.is_empty());

        let Some(pattern) = pattern else {
            let total: u32 = connection
                .query_row("SELECT count(*) FROM transcripts", [], |row| row.get(0))
                .map_err(database)?;

            let mut statement = connection
                .prepare(
                    "SELECT id, text, created_at, audio_ms, decode_ms, model_id
                     FROM transcripts ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2",
                )
                .map_err(database)?;
            let entries = statement
                .query_map(params![limit, offset], read_entry)
                .map_err(database)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database)?;

            return Ok(Page { entries, total });
        };

        let total: u32 = connection
            .query_row(
                "SELECT count(*) FROM transcripts_fts WHERE transcripts_fts MATCH ?1",
                params![&pattern],
                |row| row.get(0),
            )
            .map_err(database)?;

        let mut statement = connection
            .prepare(
                "SELECT t.id, t.text, t.created_at, t.audio_ms, t.decode_ms, t.model_id
                 FROM transcripts_fts f
                 JOIN transcripts t ON t.id = f.rowid
                 WHERE f.transcripts_fts MATCH ?1
                 ORDER BY t.created_at DESC, t.id DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(database)?;
        let entries = statement
            .query_map(params![&pattern, limit, offset], read_entry)
            .map_err(database)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database)?;

        Ok(Page { entries, total })
    }

    pub fn get(&self, id: i64) -> Result<Option<Entry>> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id, text, created_at, audio_ms, decode_ms, model_id
                 FROM transcripts WHERE id = ?1",
                params![id],
                read_entry,
            )
            .optional()
            .map_err(database)
    }

    /// Delete one entry. Returns whether anything was removed.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let connection = self.lock()?;
        let removed = connection
            .execute("DELETE FROM transcripts WHERE id = ?1", params![id])
            .map_err(database)?;
        Ok(removed > 0)
    }

    /// Delete everything, returning how many entries went.
    pub fn clear(&self) -> Result<u32> {
        let connection = self.lock()?;
        let removed = connection
            .execute("DELETE FROM transcripts", [])
            .map_err(database)?;
        Ok(removed as u32)
    }

    /// Delete entries older than `days`, returning how many went.
    ///
    /// The auto-purge: a retention limit is the difference between history being a
    /// convenience and being an indefinite record of everything ever said near the
    /// microphone.
    pub fn purge_older_than(&self, days: u32) -> Result<u32> {
        let cutoff = now_millis() - i64::from(days) * 86_400_000;
        let connection = self.lock()?;
        let removed = connection
            .execute(
                "DELETE FROM transcripts WHERE created_at < ?1",
                params![cutoff],
            )
            .map_err(database)?;
        Ok(removed as u32)
    }

    pub fn count(&self) -> Result<u32> {
        let connection = self.lock()?;
        connection
            .query_row("SELECT count(*) FROM transcripts", [], |row| row.get(0))
            .map_err(database)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| Error::Database("the history database stopped responding".into()))
    }
}

fn read_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get::<_, i64>(0)? as u32,
        text: row.get(1)?,
        created_at: to_rfc3339(row.get::<_, i64>(2)?),
        audio_ms: row.get(3)?,
        decode_ms: row.get(4)?,
        model_id: row.get(5)?,
    })
}

/// Unix milliseconds as an RFC 3339 timestamp in UTC.
fn to_rfc3339(millis: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .ok()
        .and_then(|moment| {
            moment
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_default()
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

/// Turn what someone typed into a search box into a safe FTS5 query.
///
/// FTS5's syntax has operators — `AND`, `NEAR`, `*`, `"` — and a raw string containing
/// them is either a syntax error or a query the user did not ask for. Each word is
/// quoted as a literal and a prefix wildcard is added to the last one, so results
/// narrow as the user is still typing.
fn fts_query(input: &str) -> String {
    let words: Vec<String> = input
        .split_whitespace()
        .map(|word| word.replace('"', ""))
        .filter(|word| !word.is_empty())
        .collect();

    if words.is_empty() {
        return String::new();
    }

    let last = words.len() - 1;
    words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if index == last {
                format!("\"{word}\"*")
            } else {
                format!("\"{word}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_with(texts: &[&str]) -> History {
        let history = History::in_memory().unwrap();
        for text in texts {
            history.insert(text, 1000, 60, Some("parakeet")).unwrap();
        }
        history
    }

    #[test]
    fn an_inserted_entry_can_be_read_back() {
        let history = history_with(&[]);
        let id = history
            .insert("Hello there.", 1500, 90, Some("parakeet"))
            .unwrap();

        let entry = history.get(id).unwrap().expect("the entry should exist");
        assert_eq!(entry.text, "Hello there.");
        assert_eq!(entry.audio_ms, 1500);
        assert_eq!(entry.decode_ms, 90);
        assert_eq!(entry.model_id.as_deref(), Some("parakeet"));
        assert!(
            entry.created_at.contains('T') && entry.created_at.ends_with('Z'),
            "expected RFC 3339 UTC, got {}",
            entry.created_at
        );
    }

    #[test]
    fn listing_returns_newest_first() {
        let history = history_with(&["first", "second", "third"]);
        let page = history.list(None, 10, 0).unwrap();

        assert_eq!(page.total, 3);
        let texts: Vec<_> = page.entries.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, vec!["third", "second", "first"]);
    }

    #[test]
    fn paging_walks_the_whole_history() {
        let history = history_with(&["a", "b", "c", "d", "e"]);

        let first = history.list(None, 2, 0).unwrap();
        let second = history.list(None, 2, 2).unwrap();

        assert_eq!(first.entries.len(), 2);
        assert_eq!(second.entries.len(), 2);
        assert_eq!(first.total, 5);
        assert_ne!(first.entries[0].id, second.entries[0].id);
    }

    #[test]
    fn search_finds_a_word_in_the_middle_of_an_entry() {
        let history = history_with(&["remember to buy milk", "call the dentist"]);
        let page = history.list(Some("milk"), 10, 0).unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].text, "remember to buy milk");
    }

    #[test]
    fn search_matches_a_prefix_so_results_narrow_while_typing() {
        let history = history_with(&["dentist appointment"]);
        for partial in ["den", "denti", "dentist"] {
            let page = history.list(Some(partial), 10, 0).unwrap();
            assert_eq!(page.total, 1, "searching for {partial:?} should match");
        }
    }

    #[test]
    fn search_punctuation_does_not_become_a_syntax_error() {
        let history = history_with(&["the quick brown fox"]);
        // Every one of these contains FTS5 operator syntax.
        for hostile in ["\"", "fox\" OR \"", "NEAR(a b)", "*", "AND", "quick AND"] {
            let result = history.list(Some(hostile), 10, 0);
            assert!(result.is_ok(), "{hostile:?} should not error");
        }
    }

    #[test]
    fn an_empty_or_blank_search_lists_everything() {
        let history = history_with(&["one", "two"]);
        assert_eq!(history.list(Some(""), 10, 0).unwrap().total, 2);
        assert_eq!(history.list(Some("   "), 10, 0).unwrap().total, 2);
    }

    #[test]
    fn deleting_an_entry_removes_it_from_listings_and_search() {
        let history = history_with(&["confidential note"]);
        let id = i64::from(history.list(None, 10, 0).unwrap().entries[0].id);

        assert!(history.delete(id).unwrap());
        assert_eq!(history.count().unwrap(), 0);
        assert_eq!(history.list(Some("confidential"), 10, 0).unwrap().total, 0);
    }

    #[test]
    fn deleting_something_that_is_not_there_reports_false() {
        let history = history_with(&[]);
        assert!(!history.delete(4242).unwrap());
    }

    #[test]
    fn clearing_removes_everything() {
        let history = history_with(&["a", "b", "c"]);
        assert_eq!(history.clear().unwrap(), 3);
        assert_eq!(history.count().unwrap(), 0);
    }

    #[test]
    fn purging_keeps_recent_entries_and_drops_old_ones() {
        let history = history_with(&["recent"]);

        // Backdate one entry by 40 days.
        {
            let connection = history.lock().unwrap();
            let old = now_millis() - 40 * 86_400_000;
            connection
                .execute(
                    "INSERT INTO transcripts (text, created_at) VALUES ('ancient', ?1)",
                    params![old],
                )
                .unwrap();
        }

        assert_eq!(history.purge_older_than(30).unwrap(), 1);
        let remaining = history.list(None, 10, 0).unwrap();
        assert_eq!(remaining.total, 1);
        assert_eq!(remaining.entries[0].text, "recent");
    }

    #[test]
    fn the_page_limit_is_clamped_to_something_sane() {
        let history = history_with(&["only one"]);
        // Zero would return nothing at all; a million would be a denial of service
        // against the UI.
        assert_eq!(history.list(None, 0, 0).unwrap().entries.len(), 1);
        assert_eq!(history.list(None, u32::MAX, 0).unwrap().entries.len(), 1);
    }

    #[test]
    fn a_quote_in_the_search_box_is_stripped_rather_than_escaping_the_literal() {
        assert_eq!(fts_query("say \"hello\""), "\"say\" \"hello\"*");
    }
}
