use crate::db;
use crate::develop::Adjustments;
use crate::xmp;
use crate::AppState;
use std::path::Path;
use tauri::State;

use super::library::to_str;

/// 读取照片的调色参数（无记录返回 identity）。
#[tauri::command]
pub fn get_edit(state: State<'_, AppState>, id: i64) -> Result<Adjustments, String> {
    get_edit_inner(&state, id)
}

pub fn get_edit_inner(state: &AppState, id: i64) -> Result<Adjustments, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let edit: Option<String> = conn
        .query_row(
            "SELECT edit FROM photos WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(to_str)?;
    Ok(edit
        .as_deref()
        .and_then(Adjustments::from_json)
        .unwrap_or_default())
}

/// 保存调色参数：DB（JSON）+ XMP sidecar（crs 字段，Lightroom 互通）。
#[tauri::command]
pub fn set_edit(state: State<'_, AppState>, id: i64, adj: Adjustments) -> Result<(), String> {
    set_edit_inner(&state, id, &adj)
}

pub fn set_edit_inner(state: &AppState, id: i64, adj: &Adjustments) -> Result<(), String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let path: String = conn
        .query_row(
            "SELECT path FROM photos WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(to_str)?;

    let json = if adj.is_identity() {
        String::new()
    } else {
        adj.to_json()
    };
    conn.execute(
        "UPDATE photos SET edit = ?1 WHERE id = ?2",
        rusqlite::params![json, id],
    )
    .map_err(to_str)?;

    xmp::write_develop(Path::new(&path), adj).map_err(to_str)?;
    Ok(())
}
