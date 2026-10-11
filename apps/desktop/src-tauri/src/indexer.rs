use crate::db;
use crate::metadata;
use crate::models::{ScanProgress, ScanStats};
use crate::thumbs;
use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "heic", "heif", "psd",
    "nef", "cr2", "cr3", "arw", "raf", "dng", "orf", "rw2", "pef", "srw", "x3f",
];

pub struct ScanStatsAcc {
    pub added: AtomicU64,
    pub updated: AtomicU64,
    pub skipped: AtomicU64,
}

impl ScanStatsAcc {
    fn into_stats(self, folder_id: i64, total: u64) -> ScanStats {
        ScanStats {
            folder_id,
            added: self.added.load(Ordering::Relaxed),
            updated: self.updated.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            total,
        }
    }
}

pub enum IndexOutcome {
    Added(i64),
    Updated,
    Skipped,
}

pub fn scan_folder(
    app: &AppHandle,
    folder_id: i64,
    folder_path: &Path,
    db_path: &Path,
    cache_dir: &Path,
) -> Result<ScanStats> {
    let paths = collect_images(folder_path);
    let total = paths.len() as u64;

    let conn = db::open_db(db_path)?;
    let stats = ScanStatsAcc {
        added: AtomicU64::new(0),
        updated: AtomicU64::new(0),
        skipped: AtomicU64::new(0),
    };

    let mut need_thumbs: Vec<(i64, std::path::PathBuf)> = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        match index_one(&conn, folder_id, path)? {
            IndexOutcome::Added(photo_id) => {
                stats.added.fetch_add(1, Ordering::Relaxed);
                need_thumbs.push((photo_id, path.clone()));
            }
            IndexOutcome::Updated => {
                stats.updated.fetch_add(1, Ordering::Relaxed);
                if let Some(id) = photo_id_of(&conn, path)? {
                    need_thumbs.push((id, path.clone()));
                }
            }
            IndexOutcome::Skipped => {
                stats.skipped.fetch_add(1, Ordering::Relaxed);
            }
        }

        let processed = (i + 1) as u64;
        if processed % 10 == 0 || processed == total {
            let _ = app.emit(
                "scan://progress",
                ScanProgress {
                    folder_id,
                    processed,
                    total,
                    current: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            );
        }
    }

    let ok_ids: Vec<i64> = need_thumbs
        .par_iter()
        .filter_map(|(photo_id, src)| {
            thumbs::generate(*photo_id, src, cache_dir)
                .ok()
                .flatten()
                .map(|_| *photo_id)
        })
        .collect();

    for id in ok_ids {
        conn.execute(
            "UPDATE photos SET has_thumb = 1 WHERE id = ?1",
            rusqlite::params![id],
        )?;
    }

    Ok(stats.into_stats(folder_id, total))
}

/// 索引单个文件，返回索引结果。
fn index_one(
    conn: &rusqlite::Connection,
    folder_id: i64,
    path: &Path,
) -> Result<IndexOutcome> {
    let meta = std::fs::metadata(path)?;
    let size = meta.len() as i64;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let path_str = path.to_string_lossy().into_owned();

    let existing: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT id, size, mtime_ms FROM photos WHERE path = ?1",
            rusqlite::params![path_str],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    let photo = metadata::extract(path);

    if let Some((id, old_size, old_mtime)) = existing {
        if old_size == size && old_mtime == mtime_ms {
            return Ok(IndexOutcome::Skipped);
        }
        conn.execute(
            "UPDATE photos SET size = ?1, mtime_ms = ?2, width = ?3, height = ?4,
             camera_make = ?5, camera_model = ?6, lens_model = ?7, focal_length = ?8,
             aperture = ?9, shutter_speed = ?10, iso = ?11, taken_at = ?12,
             indexed_at = datetime('now') WHERE id = ?13",
            rusqlite::params![
                size,
                mtime_ms,
                photo.width,
                photo.height,
                photo.camera_make,
                photo.camera_model,
                photo.lens_model,
                photo.focal_length,
                photo.aperture,
                photo.shutter_speed,
                photo.iso,
                photo.taken_at,
                id
            ],
        )?;
        Ok(IndexOutcome::Updated)
    } else {
        conn.execute(
            "INSERT INTO photos (folder_id, path, filename, size, mtime_ms, width, height,
             camera_make, camera_model, lens_model, focal_length, aperture, shutter_speed,
             iso, taken_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                folder_id,
                path_str,
                path.file_name().map(|n| n.to_string_lossy().into_owned()),
                size,
                mtime_ms,
                photo.width,
                photo.height,
                photo.camera_make,
                photo.camera_model,
                photo.lens_model,
                photo.focal_length,
                photo.aperture,
                photo.shutter_speed,
                photo.iso,
                photo.taken_at,
            ],
        )?;
        Ok(IndexOutcome::Added(conn.last_insert_rowid()))
    }
}

fn photo_id_of(conn: &rusqlite::Connection, path: &Path) -> Result<Option<i64>> {
    let path_str = path.to_string_lossy().into_owned();
    let id = conn
        .query_row(
            "SELECT id FROM photos WHERE path = ?1",
            rusqlite::params![path_str],
            |row| row.get(0),
        )
        .ok();
    Ok(id)
}

fn collect_images(root: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            paths.push(path.to_path_buf());
        }
    }
    paths
}
