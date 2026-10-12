use crate::db;
use crate::models::{AlbumDto, TagDto};
use crate::AppState;
use rusqlite::Connection;
use tauri::State;

use super::library::to_str;

#[tauri::command]
pub fn list_albums(state: State<'_, AppState>) -> Result<Vec<AlbumDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.name, COUNT(ap.photo_id) AS photo_count
             FROM albums a LEFT JOIN album_photos ap ON ap.album_id = a.id
             GROUP BY a.id ORDER BY a.created_at DESC",
        )
        .map_err(to_str)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(AlbumDto {
                id: row.get("id")?,
                name: row.get("name")?,
                photo_count: row.get("photo_count")?,
            })
        })
        .map_err(to_str)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_str)?;
    Ok(rows)
}

#[tauri::command]
pub fn create_album(state: State<'_, AppState>, name: String) -> Result<AlbumDto, String> {
    create_album_inner(&state, name)
}

pub fn create_album_inner(state: &AppState, name: String) -> Result<AlbumDto, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("相册名不能为空".to_string());
    }
    conn.execute(
        "INSERT OR IGNORE INTO albums (name) VALUES (?1)",
        rusqlite::params![name],
    )
    .map_err(to_str)?;
    album_by_name(&conn, &name).map_err(|_| "create album failed".to_string())
}

#[tauri::command]
pub fn delete_album(state: State<'_, AppState>, album_id: i64) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    conn.execute("DELETE FROM albums WHERE id = ?1", [album_id])
        .map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn add_photos_to_album(
    state: State<'_, AppState>,
    album_id: i64,
    photo_ids: Vec<i64>,
) -> Result<(), String> {
    add_photos_to_album_inner(&state, album_id, photo_ids)
}

pub fn add_photos_to_album_inner(
    state: &AppState,
    album_id: i64,
    photo_ids: Vec<i64>,
) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let tx = conn.unchecked_transaction().map_err(to_str)?;
    {
        let mut stmt = tx
            .prepare("INSERT OR IGNORE INTO album_photos (album_id, photo_id) VALUES (?1, ?2)")
            .map_err(to_str)?;
        for id in &photo_ids {
            stmt.execute(rusqlite::params![album_id, id]).map_err(to_str)?;
        }
    }
    tx.commit().map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn remove_photos_from_album(
    state: State<'_, AppState>,
    album_id: i64,
    photo_ids: Vec<i64>,
) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let tx = conn.unchecked_transaction().map_err(to_str)?;
    {
        let mut stmt = tx
            .prepare("DELETE FROM album_photos WHERE album_id = ?1 AND photo_id = ?2")
            .map_err(to_str)?;
        for id in &photo_ids {
            stmt.execute(rusqlite::params![album_id, id]).map_err(to_str)?;
        }
    }
    tx.commit().map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<TagDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, COUNT(pt.photo_id) AS photo_count
             FROM tags t LEFT JOIN photo_tags pt ON pt.tag_id = t.id
             GROUP BY t.id ORDER BY photo_count DESC, t.name ASC",
        )
        .map_err(to_str)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TagDto {
                id: row.get("id")?,
                name: row.get("name")?,
                photo_count: row.get("photo_count")?,
            })
        })
        .map_err(to_str)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_str)?;
    Ok(rows)
}

#[tauri::command]
pub fn add_tags(
    state: State<'_, AppState>,
    photo_ids: Vec<i64>,
    tags: Vec<String>,
) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let tx = conn.unchecked_transaction().map_err(to_str)?;
    {
        for tag in tags {
            let tag = tag.trim().to_string();
            if tag.is_empty() {
                continue;
            }
            tx.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                rusqlite::params![tag],
            )
            .map_err(to_str)?;
            let tag_id: i64 = tx
                .query_row(
                    "SELECT id FROM tags WHERE name = ?1",
                    rusqlite::params![tag],
                    |row| row.get(0),
                )
                .map_err(to_str)?;
            let mut stmt = tx
                .prepare("INSERT OR IGNORE INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)")
                .map_err(to_str)?;
            for id in &photo_ids {
                stmt.execute(rusqlite::params![id, tag_id]).map_err(to_str)?;
            }
        }
    }
    tx.commit().map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn remove_tag_from_photos(
    state: State<'_, AppState>,
    tag_id: i64,
    photo_ids: Vec<i64>,
) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let tx = conn.unchecked_transaction().map_err(to_str)?;
    {
        let mut stmt = tx
            .prepare("DELETE FROM photo_tags WHERE tag_id = ?1 AND photo_id = ?2")
            .map_err(to_str)?;
        for id in &photo_ids {
            stmt.execute(rusqlite::params![tag_id, id]).map_err(to_str)?;
        }
    }
    tx.commit().map_err(to_str)?;
    Ok(())
}

#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, tag_id: i64) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    conn.execute("DELETE FROM tags WHERE id = ?1", [tag_id])
        .map_err(to_str)?;
    Ok(())
}

fn album_by_name(conn: &Connection, name: &str) -> rusqlite::Result<AlbumDto> {
    conn.query_row(
        "SELECT id, name, 0 FROM albums WHERE name = ?1",
        rusqlite::params![name],
        |row| {
            Ok(AlbumDto {
                id: row.get(0)?,
                name: row.get(1)?,
                photo_count: row.get(2)?,
            })
        },
    )
}
