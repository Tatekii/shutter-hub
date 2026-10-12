use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

/// 有序迁移列表：每个条目是一个 SQL 批次，索引 i 代表 user_version i -> i+1。
const MIGRATIONS: &[&str] = &[
    // v1: 基础结构（文件夹 + 照片索引）
    r#"
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
    // v2: 评分 / 色标 / 旗标
    r#"
    ALTER TABLE photos ADD COLUMN rating      INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE photos ADD COLUMN color_label TEXT    NOT NULL DEFAULT '';
    ALTER TABLE photos ADD COLUMN flag        TEXT    NOT NULL DEFAULT '';
    "#,
    // v3: FTS5 全文索引（文件名 / 路径 / 相机 / 镜头），并回填已有数据
    r#"
    CREATE VIRTUAL TABLE IF NOT EXISTS photos_fts USING fts5(
        filename, path, camera_make, camera_model, lens_model,
        tokenize = 'unicode61'
    );
    INSERT INTO photos_fts(rowid, filename, path, camera_make, camera_model, lens_model)
        SELECT id, filename, path, coalesce(camera_make, ''),
                      coalesce(camera_model, ''), coalesce(lens_model, '')
        FROM photos;
    "#,
    // v4: 相册 + 标签
    r#"
    CREATE TABLE IF NOT EXISTS albums (
        id         INTEGER PRIMARY KEY,
        name       TEXT NOT NULL UNIQUE,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS album_photos (
        album_id INTEGER NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
        photo_id INTEGER NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
        PRIMARY KEY (album_id, photo_id)
    );

    CREATE TABLE IF NOT EXISTS tags (
        id   INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE
    );

    CREATE TABLE IF NOT EXISTS photo_tags (
        photo_id INTEGER NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
        tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
        PRIMARY KEY (photo_id, tag_id)
    );
    "#,
    // v5: 内容哈希（重复检测）
    r#"
    ALTER TABLE photos ADD COLUMN content_hash TEXT;
    ALTER TABLE photos ADD COLUMN phash        INTEGER;
    "#,
    // v6: 视频素材（媒体类型 + 时长）
    r#"
    ALTER TABLE photos ADD COLUMN media_type  TEXT NOT NULL DEFAULT 'photo';
    ALTER TABLE photos ADD COLUMN duration_ms INTEGER;
    "#,
];

pub fn open_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        "#,
    )?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (i, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration)?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}
