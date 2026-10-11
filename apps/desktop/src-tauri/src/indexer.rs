use crate::db;
use crate::metadata;
use crate::models::{ScanProgress, ScanStats};
use crate::thumbs;
use crate::xmp;
use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "heic", "heif", "psd",
    "nef", "cr2", "cr3", "arw", "raf", "dng", "orf", "rw2", "pef", "srw", "x3f",
];

struct Counters {
    added: AtomicU64,
    updated: AtomicU64,
    skipped: AtomicU64,
}

pub enum IndexOutcome {
    Added(i64),
    Updated(i64),
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
    let counters = Counters {
        added: AtomicU64::new(0),
        updated: AtomicU64::new(0),
        skipped: AtomicU64::new(0),
    };

    let mut need_thumbs: Vec<(i64, std::path::PathBuf)> = Vec::new();

    for (i, path) in paths.iter().enumerate() {
        match index_one(&conn, folder_id, path)? {
            IndexOutcome::Added(photo_id) => {
                counters.added.fetch_add(1, Ordering::Relaxed);
                need_thumbs.push((photo_id, path.clone()));
            }
            IndexOutcome::Updated(photo_id) => {
                counters.updated.fetch_add(1, Ordering::Relaxed);
                need_thumbs.push((photo_id, path.clone()));
            }
            IndexOutcome::Skipped => {
                counters.skipped.fetch_add(1, Ordering::Relaxed);
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

    let thumb_results: Vec<(i64, Option<i64>)> = need_thumbs
        .par_iter()
        .map(|(photo_id, src)| {
            let thumb_ok = matches!(thumbs::generate(*photo_id, src, cache_dir), Ok(Some(_)));
            let phash = thumb_ok
                .then(|| thumbs::phash_of_thumb(&thumbs::thumb_path_for(cache_dir, *photo_id)))
                .flatten();
            (*photo_id, phash)
        })
        .collect();

    for (photo_id, phash) in thumb_results {
        conn.execute(
            "UPDATE photos SET has_thumb = 1, phash = COALESCE(phash, ?2) WHERE id = ?1",
            rusqlite::params![photo_id, phash],
        )?;
    }

    Ok(ScanStats {
        folder_id,
        added: counters.added.load(Ordering::Relaxed),
        updated: counters.updated.load(Ordering::Relaxed),
        skipped: counters.skipped.load(Ordering::Relaxed),
        total,
    })
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
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let existing: Option<(i64, i64, i64, i64, String, String)> = conn
        .query_row(
            "SELECT id, size, mtime_ms, rating, color_label, flag FROM photos WHERE path = ?1",
            rusqlite::params![path_str],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .ok();

    let photo = metadata::extract(path);
    let sidecar = xmp::read(path);

    let outcome = if let Some((id, old_size, old_mtime, old_rating, old_label, old_flag)) =
        existing
    {
        if old_size == size && old_mtime == mtime_ms {
            IndexOutcome::Skipped
        } else {
            let rating = sidecar
                .as_ref()
                .and_then(|s| s.rating)
                .unwrap_or(old_rating);
            let label = sidecar
                .as_ref()
                .and_then(|s| s.label.clone())
                .unwrap_or(old_label);
            let flag = sidecar
                .as_ref()
                .and_then(|s| s.flag.clone())
                .unwrap_or(old_flag);

            conn.execute(
                "UPDATE photos SET size = ?1, mtime_ms = ?2, width = ?3, height = ?4,
                 camera_make = ?5, camera_model = ?6, lens_model = ?7, focal_length = ?8,
                 aperture = ?9, shutter_speed = ?10, iso = ?11, taken_at = ?12,
                 rating = ?13, color_label = ?14, flag = ?15,
                 indexed_at = datetime('now') WHERE id = ?16",
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
                    rating,
                    label,
                    flag,
                    id
                ],
            )?;
            IndexOutcome::Updated(id)
        }
    } else {
        let rating = sidecar.as_ref().and_then(|s| s.rating).unwrap_or(0);
        let label = sidecar
            .as_ref()
            .and_then(|s| s.label.clone())
            .unwrap_or_default();
        let flag = sidecar
            .as_ref()
            .and_then(|s| s.flag.clone())
            .unwrap_or_default();

        conn.execute(
            "INSERT INTO photos (folder_id, path, filename, size, mtime_ms, width, height,
             camera_make, camera_model, lens_model, focal_length, aperture, shutter_speed,
             iso, taken_at, rating, color_label, flag)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            rusqlite::params![
                folder_id,
                path_str,
                filename,
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
                rating,
                label,
                flag,
            ],
        )?;
        IndexOutcome::Added(conn.last_insert_rowid())
    };

    if let IndexOutcome::Added(id) | IndexOutcome::Updated(id) = outcome {
        sync_fts(conn, id)?;
    }

    Ok(outcome)
}

fn sync_fts(conn: &rusqlite::Connection, photo_id: i64) -> Result<()> {
    conn.execute("DELETE FROM photos_fts WHERE rowid = ?1", [photo_id])?;
    conn.execute(
        "INSERT INTO photos_fts(rowid, filename, path, camera_make, camera_model, lens_model)
         SELECT id, filename, path, COALESCE(camera_make, ''), COALESCE(camera_model, ''),
                COALESCE(lens_model, '') FROM photos WHERE id = ?1",
        [photo_id],
    )?;
    Ok(())
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
