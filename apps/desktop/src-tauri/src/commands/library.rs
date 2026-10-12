use crate::db;
use crate::indexer;
use crate::models::{FacetDto, FacetValue, FolderDto, LibraryStats, PhotoDto, PhotoQuery, ScanStats};
use crate::xmp;
use crate::AppState;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

pub(crate) const PHOTO_COLUMNS: &str = "id, folder_id, path, filename, size, width, height, camera_make,
    camera_model, lens_model, focal_length, aperture, shutter_speed, iso, taken_at, has_thumb,
    rating, color_label, flag, media_type, duration_ms";

#[tauri::command]
pub async fn add_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ScanStats>, String> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
    })
    .await
    .map_err(|e| e.to_string())?;

    let Some(dir) = picked else {
        return Ok(None);
    };

    insert_and_scan(&app, &state, dir).await.map(Some)
}

#[tauri::command]
pub async fn rescan_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_id: i64,
) -> Result<ScanStats, String> {
    let path = {
        let conn = db::open_db(&state.db_path).map_err(to_str)?;
        conn.query_row(
            "SELECT path FROM folders WHERE id = ?1",
            rusqlite::params![folder_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(to_str)?
    };

    insert_and_scan(&app, &state, PathBuf::from(path)).await
}

#[tauri::command]
pub fn list_folders(state: State<'_, AppState>) -> Result<Vec<FolderDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.path, COUNT(p.id) AS photo_count
             FROM folders f LEFT JOIN photos p ON p.folder_id = f.id
             GROUP BY f.id ORDER BY f.added_at DESC",
        )
        .map_err(to_str)?;

    let folders = stmt
        .query_map([], |row| {
            Ok(FolderDto {
                id: row.get("id")?,
                path: row.get("path")?,
                photo_count: row.get("photo_count")?,
            })
        })
        .map_err(to_str)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_str)?;

    Ok(folders)
}

#[tauri::command]
pub fn list_photos(
    state: State<'_, AppState>,
    query: PhotoQuery,
) -> Result<Vec<PhotoDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let (sql, params) = build_photo_query(&query);
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    query_photos(&conn, &sql, &refs, &state.cache_dir).map_err(to_str)
}

#[tauri::command]
pub fn get_photo(state: State<'_, AppState>, id: i64) -> Result<PhotoDto, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let sql = format!("SELECT {PHOTO_COLUMNS} FROM photos WHERE id = ?1");
    query_photos(&conn, &sql, &[&id], &state.cache_dir)
        .map_err(to_str)?
        .into_iter()
        .next()
        .ok_or_else(|| "photo not found".to_string())
}

#[tauri::command]
pub async fn get_viewer_preview(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || get_viewer_preview_inner(&state, id))
        .await
        .map_err(|e| e.to_string())?
}

/// 返回看图模式用的全尺寸预览路径（懒生成，缓存在 previews 目录）。
pub fn get_viewer_preview_inner(state: &AppState, id: i64) -> Result<Option<String>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let path: String = conn
        .query_row(
            "SELECT path FROM photos WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(to_str)?;

    crate::preview::ensure(&state.preview_dir, id, Path::new(&path))
        .map_err(|e| format!("{e:#}"))
        .map(|p| p.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> Result<LibraryStats, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let photo_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM photos", [], |row| row.get(0))
        .map_err(to_str)?;
    let folder_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM folders", [], |row| row.get(0))
        .map_err(to_str)?;
    Ok(LibraryStats {
        photo_count,
        folder_count,
    })
}

#[tauri::command]
pub fn remove_folder(state: State<'_, AppState>, folder_id: i64) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;

    let photo_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM photos WHERE folder_id = ?1")
            .map_err(to_str)?;
        let ids = stmt
            .query_map([folder_id], |row| row.get(0))
            .map_err(to_str)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_str)?;
        ids
    };

    for id in &photo_ids {
        let _ = std::fs::remove_file(crate::thumbs::thumb_path_for(&state.cache_dir, *id));
        let _ = std::fs::remove_file(crate::preview::path_for(&state.preview_dir, *id));
    }

    conn.execute("DELETE FROM photos_fts WHERE rowid IN (SELECT id FROM photos WHERE folder_id = ?1)", [folder_id])
        .map_err(to_str)?;
    conn.execute("DELETE FROM folders WHERE id = ?1", [folder_id])
        .map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn set_rating(state: State<'_, AppState>, id: i64, rating: i64) -> Result<(), String> {
    set_rating_inner(&state, id, rating)
}

pub fn set_rating_inner(state: &AppState, id: i64, rating: i64) -> Result<(), String> {
    update_rating_field(state, id, |path| xmp::write_rating(path, rating), rating, |conn| {
        conn.execute(
            "UPDATE photos SET rating = ?1 WHERE id = ?2",
            rusqlite::params![rating, id],
        )
        .map(|_| ())
    })
}

#[tauri::command]
pub fn set_color_label(state: State<'_, AppState>, id: i64, label: String) -> Result<(), String> {
    let label = normalize_label(&label);
    update_rating_field(
        &state,
        id,
        |path| xmp::write_label(path, &label),
        &label,
        |conn| {
            conn.execute(
                "UPDATE photos SET color_label = ?1 WHERE id = ?2",
                rusqlite::params![label, id],
            )
            .map(|_| ())
        },
    )
}

#[tauri::command]
pub fn set_flag(state: State<'_, AppState>, id: i64, flag: String) -> Result<(), String> {
    set_flag_inner(&state, id, flag)
}

pub fn set_flag_inner(state: &AppState, id: i64, flag: String) -> Result<(), String> {
    let flag = if flag == "pick" || flag == "reject" {
        flag
    } else {
        String::new()
    };
    update_rating_field(
        state,
        id,
        |path| xmp::write_flag(path, &flag),
        &flag,
        |conn| {
            conn.execute(
                "UPDATE photos SET flag = ?1 WHERE id = ?2",
                rusqlite::params![flag, id],
            )
            .map(|_| ())
        },
    )
}

#[tauri::command]
pub fn list_facets(state: State<'_, AppState>) -> Result<FacetDto, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;

    let camera_models = facet_values(
        &conn,
        "SELECT camera_model, COUNT(*) FROM photos
         WHERE camera_model IS NOT NULL AND camera_model != ''
         GROUP BY camera_model ORDER BY COUNT(*) DESC LIMIT 40",
    )
    .map_err(to_str)?;
    let lens_models = facet_values(
        &conn,
        "SELECT lens_model, COUNT(*) FROM photos
         WHERE lens_model IS NOT NULL AND lens_model != ''
         GROUP BY lens_model ORDER BY COUNT(*) DESC LIMIT 40",
    )
    .map_err(to_str)?;

    let iso_range: (Option<i64>, Option<i64>) = conn
        .query_row("SELECT MIN(iso), MAX(iso) FROM photos WHERE iso > 0", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(to_str)?;
    let focal_range: (Option<f64>, Option<f64>) = conn
        .query_row(
            "SELECT MIN(focal_length), MAX(focal_length) FROM photos WHERE focal_length > 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(to_str)?;
    let date_range: (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT MIN(taken_at), MAX(taken_at) FROM photos WHERE taken_at IS NOT NULL",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(to_str)?;

    Ok(FacetDto {
        camera_models,
        lens_models,
        iso_range,
        focal_range,
        date_range,
    })
}

fn facet_values(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<FacetValue>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(FacetValue {
            value: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect()
}

fn normalize_label(label: &str) -> String {
    if xmp::COLOR_LABELS.contains(&label) {
        label.to_string()
    } else {
        String::new()
    }
}

fn update_rating_field<T, F, U>(
    state: &AppState,
    id: i64,
    write_xmp: F,
    _value: T,
    update_db: U,
) -> Result<(), String>
where
    F: FnOnce(&Path) -> anyhow::Result<()>,
    U: FnOnce(&Connection) -> rusqlite::Result<()>,
{
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let path: String = conn
        .query_row(
            "SELECT path FROM photos WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(to_str)?;

    update_db(&conn).map_err(to_str)?;
    write_xmp(Path::new(&path)).map_err(to_str)?;
    Ok(())
}

fn build_photo_query(q: &PhotoQuery) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = format!("SELECT {PHOTO_COLUMNS} FROM photos");
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(qstr) = &q.q {
        let qstr = qstr.trim();
        if !qstr.is_empty() {
            clauses.push(
                "id IN (SELECT rowid FROM photos_fts WHERE photos_fts MATCH ?)".to_string(),
            );
            params.push(Box::new(escape_fts(qstr)));
        }
    }
    if let Some(c) = &q.camera {
        clauses.push("camera_model = ?".to_string());
        params.push(Box::new(c.clone()));
    }
    if let Some(l) = &q.lens {
        clauses.push("lens_model = ?".to_string());
        params.push(Box::new(l.clone()));
    }
    if let Some(v) = q.iso_min {
        clauses.push("iso >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = q.iso_max {
        clauses.push("iso <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = q.focal_min {
        clauses.push("focal_length >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = q.focal_max {
        clauses.push("focal_length <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = q.aperture_min {
        clauses.push("aperture >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = q.aperture_max {
        clauses.push("aperture <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = &q.date_from {
        if !v.is_empty() {
            clauses.push("taken_at >= ?".to_string());
            params.push(Box::new(v.clone()));
        }
    }
    if let Some(v) = &q.date_to {
        if !v.is_empty() {
            clauses.push("taken_at <= ?".to_string());
            params.push(Box::new(v.clone()));
        }
    }
    if let Some(v) = q.min_rating {
        if v > 0 {
            clauses.push("rating >= ?".to_string());
            params.push(Box::new(v));
        }
    }
    if let Some(v) = &q.color_label {
        if !v.is_empty() {
            clauses.push("color_label = ?".to_string());
            params.push(Box::new(v.clone()));
        }
    }
    if let Some(v) = &q.flag {
        if !v.is_empty() {
            clauses.push("flag = ?".to_string());
            params.push(Box::new(v.clone()));
        }
    }
    if let Some(v) = &q.media_type {
        if v == "photo" || v == "video" {
            clauses.push("media_type = ?".to_string());
            params.push(Box::new(v.clone()));
        }
    }
    if let Some(id) = q.album_id {
        clauses.push(
            "id IN (SELECT photo_id FROM album_photos WHERE album_id = ?)".to_string(),
        );
        params.push(Box::new(id));
    }
    if let Some(id) = q.tag_id {
        clauses.push("id IN (SELECT photo_id FROM photo_tags WHERE tag_id = ?)".to_string());
        params.push(Box::new(id));
    }

    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY taken_at DESC NULLS LAST, filename ASC");

    (sql, params)
}

fn escape_fts(q: &str) -> String {
    let tokens: Vec<&str> = q.split_whitespace().collect();
    tokens
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let t = t.replace('"', "\"\"");
            if i == tokens.len() - 1 {
                format!("\"{t}\"*")
            } else {
                format!("\"{t}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn query_photos(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
    cache_dir: &Path,
) -> rusqlite::Result<Vec<PhotoDto>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, |row| row_to_photo(row, cache_dir))?;
    rows.collect()
}

pub fn row_to_photo(
    row: &rusqlite::Row<'_>,
    cache_dir: &Path,
) -> rusqlite::Result<PhotoDto> {
    let id: i64 = row.get("id")?;
    let has_thumb: i64 = row.get("has_thumb")?;
    Ok(PhotoDto {
        id,
        folder_id: row.get("folder_id")?,
        path: row.get("path")?,
        filename: row.get("filename")?,
        size: row.get("size")?,
        width: row.get("width")?,
        height: row.get("height")?,
        camera_make: row.get("camera_make")?,
        camera_model: row.get("camera_model")?,
        lens_model: row.get("lens_model")?,
        focal_length: row.get("focal_length")?,
        aperture: row.get("aperture")?,
        shutter_speed: row.get("shutter_speed")?,
        iso: row.get("iso")?,
        taken_at: row.get("taken_at")?,
        has_thumb: has_thumb != 0,
        thumb_path: (has_thumb != 0).then(|| {
            crate::thumbs::thumb_path_for(cache_dir, id)
                .to_string_lossy()
                .into_owned()
        }),
        rating: row.get("rating")?,
        color_label: row.get("color_label")?,
        flag: row.get("flag")?,
        media_type: row.get("media_type")?,
        duration_ms: row.get("duration_ms")?,
    })
}

pub(crate) fn to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

async fn insert_and_scan(
    app: &AppHandle,
    state: &AppState,
    dir: PathBuf,
) -> Result<ScanStats, String> {
    let folder_id = {
        let conn = db::open_db(&state.db_path).map_err(to_str)?;
        conn.execute(
            "INSERT OR IGNORE INTO folders (path) VALUES (?1)",
            [dir.to_string_lossy().into_owned()],
        )
        .map_err(to_str)?;
        conn.query_row(
            "SELECT id FROM folders WHERE path = ?1",
            [dir.to_string_lossy().into_owned()],
            |row| row.get(0),
        )
        .map_err(to_str)?
    };

    let app = app.clone();
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        let result = indexer::scan_folder(folder_id, &dir, &db_path, &cache_dir, move |p| {
            let _ = emitter.emit("scan://progress", p);
        });
        match &result {
            Ok(stats) => {
                let _ = app.emit("scan://done", stats);
            }
            Err(e) => {
                let _ = app.emit("scan://error", format!("{e:#}"));
            }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
