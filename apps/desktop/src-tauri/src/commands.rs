use crate::db;
use crate::indexer;
use crate::models::{FolderDto, LibraryStats, PhotoDto, ScanStats};
use crate::AppState;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

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
                id: row.get(0)?,
                path: row.get(1)?,
                photo_count: row.get(2)?,
            })
        })
        .map_err(to_str)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_str)?;

    Ok(folders)
}

#[tauri::command]
pub fn list_photos(state: State<'_, AppState>) -> Result<Vec<PhotoDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, folder_id, path, filename, size, width, height,
                    camera_make, camera_model, lens_model, focal_length, aperture,
                    shutter_speed, iso, taken_at, has_thumb
             FROM photos ORDER BY taken_at DESC NULLS LAST, filename ASC",
        )
        .map_err(to_str)?;

    let photos = stmt
        .query_map([], |row| row_to_photo(row, &state.cache_dir))
        .map_err(to_str)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_str)?;

    Ok(photos)
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
    }

    conn.execute("DELETE FROM folders WHERE id = ?1", [folder_id])
        .map_err(to_str)?;
    Ok(())
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
        let result = indexer::scan_folder(&app, folder_id, &dir, &db_path, &cache_dir);
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

fn row_to_photo(
    row: &rusqlite::Row<'_>,
    cache_dir: &Path,
) -> rusqlite::Result<PhotoDto> {
    let id: i64 = row.get(0)?;
    let has_thumb: i64 = row.get(15)?;
    Ok(PhotoDto {
        id,
        folder_id: row.get(1)?,
        path: row.get(2)?,
        filename: row.get(3)?,
        size: row.get(4)?,
        width: row.get(5)?,
        height: row.get(6)?,
        camera_make: row.get(7)?,
        camera_model: row.get(8)?,
        lens_model: row.get(9)?,
        focal_length: row.get(10)?,
        aperture: row.get(11)?,
        shutter_speed: row.get(12)?,
        iso: row.get(13)?,
        taken_at: row.get(14)?,
        has_thumb: has_thumb != 0,
        thumb_path: (has_thumb != 0).then(|| {
            crate::thumbs::thumb_path_for(cache_dir, id)
                .to_string_lossy()
                .into_owned()
        }),
    })
}

fn to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}
