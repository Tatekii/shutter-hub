use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn open_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS folders (
            id         INTEGER PRIMARY KEY,
            path       TEXT NOT NULL UNIQUE,
            added_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS photos (
            id            INTEGER PRIMARY KEY,
            folder_id     INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            path          TEXT NOT NULL UNIQUE,
            filename      TEXT NOT NULL,
            size          INTEGER NOT NULL DEFAULT 0,
            mtime_ms      INTEGER NOT NULL DEFAULT 0,
            width         INTEGER,
            height        INTEGER,
            camera_make   TEXT,
            camera_model  TEXT,
            lens_model    TEXT,
            focal_length  REAL,
            aperture      REAL,
            shutter_speed TEXT,
            iso           INTEGER,
            taken_at      TEXT,
            has_thumb     INTEGER NOT NULL DEFAULT 0,
            indexed_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_photos_folder ON photos(folder_id);
        CREATE INDEX IF NOT EXISTS idx_photos_taken  ON photos(taken_at);
        "#,
    )?;
    Ok(conn)
}
