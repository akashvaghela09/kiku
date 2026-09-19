//! Database schema and migrations.
//!
//! The migration runner exists from the very first release, and that is the point: the
//! scope requires history to survive an update, which only holds if there is a
//! versioning mechanism in place *before* the first schema change. Retrofitting one
//! after users have data is what breaks histories.
//!
//! Each entry in [`MIGRATIONS`] runs once, in order, inside a transaction, and
//! `PRAGMA user_version` records how far we got. Migrations are append-only: never
//! edit one that has shipped, add another.

use rusqlite::Connection;

use crate::error::{Error, Result};

/// Applied in order. Append only — editing a shipped migration desynchronises every
/// database that already ran it.
const MIGRATIONS: &[&str] = &[
    // 1 — transcripts, plus a full-text index kept in step by triggers.
    r#"
    CREATE TABLE transcripts (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        text        TEXT    NOT NULL,
        created_at  INTEGER NOT NULL,
        audio_ms    INTEGER NOT NULL DEFAULT 0,
        decode_ms   INTEGER NOT NULL DEFAULT 0,
        model_id    TEXT
    );

    CREATE INDEX idx_transcripts_created_at ON transcripts (created_at DESC);

    -- External-content FTS: the index stores no copy of the text, so a transcript
    -- deleted from `transcripts` cannot survive in the search index.
    CREATE VIRTUAL TABLE transcripts_fts USING fts5 (
        text,
        content='transcripts',
        content_rowid='id',
        tokenize='unicode61'
    );

    CREATE TRIGGER transcripts_ai AFTER INSERT ON transcripts BEGIN
        INSERT INTO transcripts_fts (rowid, text) VALUES (new.id, new.text);
    END;

    CREATE TRIGGER transcripts_ad AFTER DELETE ON transcripts BEGIN
        INSERT INTO transcripts_fts (transcripts_fts, rowid, text)
        VALUES ('delete', old.id, old.text);
    END;

    CREATE TRIGGER transcripts_au AFTER UPDATE ON transcripts BEGIN
        INSERT INTO transcripts_fts (transcripts_fts, rowid, text)
        VALUES ('delete', old.id, old.text);
        INSERT INTO transcripts_fts (rowid, text) VALUES (new.id, new.text);
    END;
    "#,
];

/// Bring `connection` up to the current schema, applying only what it is missing.
pub fn migrate(connection: &mut Connection) -> Result<()> {
    // Foreign keys off by default in SQLite; WAL keeps reads from blocking the write
    // that happens at the end of every dictation.
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(database)?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(database)?;

    let current: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(database)?;

    if current as usize > MIGRATIONS.len() {
        return Err(Error::Database(format!(
            "this history was written by a newer version of Kiku (schema {current}). \
             Update Kiku to open it."
        )));
    }

    for (index, migration) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let version = index + 1;
        tracing::info!(version, "applying history migration");

        let transaction = connection.transaction().map_err(database)?;
        transaction.execute_batch(migration).map_err(database)?;
        transaction
            .pragma_update(None, "user_version", version as i64)
            .map_err(database)?;
        transaction.commit().map_err(database)?;
    }

    Ok(())
}

pub(crate) fn database(error: impl std::fmt::Display) -> Error {
    Error::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
    }

    #[test]
    fn migrating_a_fresh_database_reaches_the_latest_version() {
        let connection = migrated();
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let mut connection = migrated();
        migrate(&mut connection).expect("a second run must be harmless");
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version as usize, MIGRATIONS.len());
    }

    #[test]
    fn a_database_from_a_newer_version_is_refused_rather_than_corrupted() {
        let mut connection = migrated();
        connection
            .pragma_update(None, "user_version", 999_i64)
            .unwrap();

        let error = migrate(&mut connection).unwrap_err().to_string();
        assert!(error.contains("newer version"), "{error}");
    }

    #[test]
    fn full_text_search_is_available() {
        // FTS5 is compiled into the bundled SQLite; if that ever changes, the failure
        // should be this test rather than a user's search silently returning nothing.
        let connection = migrated();
        connection
            .execute(
                "INSERT INTO transcripts (text, created_at) VALUES (?1, ?2)",
                rusqlite::params!["the quick brown fox", 0_i64],
            )
            .unwrap();

        let found: i64 = connection
            .query_row(
                "SELECT count(*) FROM transcripts_fts WHERE transcripts_fts MATCH 'brown'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(found, 1);
    }

    #[test]
    fn deleting_a_transcript_removes_it_from_the_search_index() {
        let connection = migrated();
        connection
            .execute(
                "INSERT INTO transcripts (text, created_at) VALUES ('secret plans', 0)",
                [],
            )
            .unwrap();
        connection.execute("DELETE FROM transcripts", []).unwrap();

        let found: i64 = connection
            .query_row(
                "SELECT count(*) FROM transcripts_fts WHERE transcripts_fts MATCH 'secret'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(found, 0, "deleted text must not survive in the index");
    }
}
