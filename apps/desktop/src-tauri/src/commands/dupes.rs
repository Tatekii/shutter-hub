use crate::db;
use crate::models::{DupeGroup, HashProgress, SimilarPair};
use crate::AppState;
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

use super::library::{query_photos, to_str, PHOTO_COLUMNS};

const MAX_DUPE_GROUPS: usize = 100;
const SIMILAR_THRESHOLD: u32 = 6;

/// 为未计算哈希的照片计算 blake3 内容哈希（phash 缺失时顺带补算）。
#[tauri::command]
pub async fn compute_hashes(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = db::open_db(&db_path).map_err(|e| format!("{e:#}"))?;

        let pending: Vec<(i64, String)> = {
            let mut stmt = conn
                .prepare("SELECT id, path FROM photos WHERE content_hash IS NULL")
                .map_err(to_str)?;
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(to_str)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(to_str)?
        };
        let total = pending.len() as u64;

        for (i, (id, path)) in pending.iter().enumerate() {
            let current = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let _ = app.emit(
                "hash://progress",
                HashProgress {
                    processed: (i + 1) as u64,
                    total,
                    current: current.clone(),
                },
            );

            let hash = match file_hash(Path::new(path)) {
                Ok(h) => h,
                Err(_) => continue,
            };
            conn.execute(
                "UPDATE photos SET content_hash = ?1 WHERE id = ?2",
                rusqlite::params![hash, id],
            )
            .map_err(to_str)?;

            let phash_missing: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM photos WHERE id = ?1 AND phash IS NULL",
                    rusqlite::params![id],
                    |row| row.get(0),
                )
                .map_err(to_str)?;
            if phash_missing > 0 {
                let phash = crate::thumbs::phash_of_thumb(&crate::thumbs::thumb_path_for(&cache_dir, *id));
                if let Some(phash) = phash {
                    conn.execute(
                        "UPDATE photos SET phash = ?1 WHERE id = ?2",
                        rusqlite::params![phash, id],
                    )
                    .map_err(to_str)?;
                }
            }
        }

        let _ = app.emit("hash://done", ());
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_dupes(state: State<'_, AppState>) -> Result<Vec<DupeGroup>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;

    let hashes: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT content_hash FROM photos
                 WHERE content_hash IS NOT NULL
                 GROUP BY content_hash HAVING COUNT(*) > 1
                 ORDER BY COUNT(*) DESC LIMIT ?1",
            )
            .map_err(to_str)?;
        let rows = stmt
            .query_map([MAX_DUPE_GROUPS as i64], |row| row.get(0))
            .map_err(to_str)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(to_str)?
    };

    let sql = format!("SELECT {PHOTO_COLUMNS} FROM photos WHERE content_hash = ?1 ORDER BY path");
    let mut groups = Vec::new();
    for hash in hashes {
        let photos = query_photos(&conn, &sql, &[&hash], &state.cache_dir).map_err(to_str)?;
        let total_size: i64 = photos.iter().map(|p| p.size).sum();
        groups.push(DupeGroup {
            hash,
            total_size,
            photos,
        });
    }
    Ok(groups)
}

#[tauri::command]
pub fn list_similar(state: State<'_, AppState>) -> Result<Vec<SimilarPair>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;

    let hashed: Vec<(i64, i64)> = {
        let mut stmt = conn
            .prepare("SELECT id, phash FROM photos WHERE phash IS NOT NULL")
            .map_err(to_str)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(to_str)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(to_str)?
    };

    let mut buckets: HashMap<i32, Vec<(i64, i64)>> = HashMap::new();
    for (id, phash) in hashed {
        let bucket = (phash >> 48) as i32;
        let bucket_list = buckets.entry(bucket).or_default();
        if bucket_list.len() < 256 {
            bucket_list.push((id, phash));
        }
    }

    let mut pairs: Vec<(u32, i64, i64)> = Vec::new();
    for bucket in buckets.values() {
        for i in 0..bucket.len() {
            for j in (i + 1)..bucket.len() {
                let (id_a, hash_a) = bucket[i];
                let (id_b, hash_b) = bucket[j];
                let distance = ((hash_a as u64) ^ (hash_b as u64)).count_ones();
                if distance <= SIMILAR_THRESHOLD {
                    pairs.push((distance, id_a.min(id_b), id_a.max(id_b)));
                }
            }
        }
    }
    pairs.sort_by_key(|(d, a, b)| (*d, *a, *b));
    pairs.truncate(100);

    let mut result = Vec::new();
    for (distance, id_a, id_b) in pairs {
        let sql = format!("SELECT {PHOTO_COLUMNS} FROM photos WHERE id IN (?1, ?2)");
        let photos = query_photos(&conn, &sql, &[&id_a, &id_b], &state.cache_dir)
            .map_err(to_str)?;
        if photos.len() == 2 {
            result.push(SimilarPair { distance, photos });
        }
    }
    Ok(result)
}

fn file_hash(path: &Path) -> anyhow::Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        use std::io::Read;
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
