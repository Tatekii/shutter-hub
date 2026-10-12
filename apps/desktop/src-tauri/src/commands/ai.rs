use crate::ai;
use crate::db;
use crate::models::{AiDownloadProgress, AiJobDone, AiJobProgress, AiStatusDto, FaceDto, PhotoDto};
use crate::AppState;
use rusqlite::Connection;
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use super::library::{query_photos, to_str, PHOTO_COLUMNS};

/// 快速失败：已有 AI 任务在跑时立即报错。
fn check_not_busy(state: &AppState) -> Result<(), String> {
    if state.ai_busy.load(Ordering::SeqCst) {
        Err("AI 任务已在运行".to_string())
    } else {
        Ok(())
    }
}

/// 在后台任务内部抢锁（compare_exchange），Drop 时释放。
/// 同一时间只允许一个 AI 任务（下载 / 向量 / 人脸）。
struct BusyGuard {
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl BusyGuard {
    fn acquire(state: &AppState) -> Result<Self, String> {
        state
            .ai_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "AI 任务已在运行".to_string())?;
        Ok(Self {
            flag: state.ai_busy.clone(),
        })
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub fn ai_status(state: State<'_, AppState>) -> Result<AiStatusDto, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let (embeddings_done, photos_total, faces_done, faces_total) =
        status_counts(&conn).map_err(to_str)?;
    Ok(AiStatusDto {
        clip_ready: ai::clip_ready(&state.model_dir),
        face_ready: ai::face_ready(&state.model_dir),
        embeddings_done,
        embeddings_total: photos_total,
        faces_done,
        faces_total,
        busy: state.ai_busy.load(Ordering::SeqCst),
    })
}

fn status_counts(conn: &Connection) -> rusqlite::Result<(i64, i64, i64, i64)> {
    let embeddings_done: i64 =
        conn.query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))?;
    let photos_total: i64 = conn.query_row("SELECT COUNT(*) FROM photos", [], |r| r.get(0))?;
    let faces_done: i64 =
        conn.query_row("SELECT COUNT(*) FROM ai_markers WHERE kind = 'face'", [], |r| {
            r.get(0)
        })?;
    let faces_total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM photos WHERE media_type = 'photo'",
        [],
        |r| r.get(0),
    )?;
    Ok((embeddings_done, photos_total, faces_done, faces_total))
}

/// 下载模型（约 180MB，已有文件跳过），emit "ai://download"。
#[tauri::command]
pub async fn ai_download_models(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    check_not_busy(&state)?;
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = BusyGuard::acquire(&state)?;
        let emitter = app.clone();
        let result = ai::download_models(&state.model_dir, move |file, received, total| {
            let _ = emitter.emit(
                "ai://download",
                AiDownloadProgress {
                    file: file.to_string(),
                    received,
                    total,
                    done: false,
                    error: None,
                },
            );
        });
        let payload = match &result {
            Ok(()) => AiDownloadProgress {
                file: String::new(),
                received: 0,
                total: None,
                done: true,
                error: None,
            },
            Err(e) => AiDownloadProgress {
                file: String::new(),
                received: 0,
                total: None,
                done: true,
                error: Some(format!("{e:#}")),
            },
        };
        let _ = app.emit("ai://download", payload);
        result.map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 为所有照片（视频取缩略帧）生成 CLIP 语义向量，emit "ai://progress"。
#[tauri::command]
pub async fn ai_compute_embeddings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    check_not_busy(&state)?;
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = BusyGuard::acquire(&state)?;
        run_embedding_job(&app, &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn run_embedding_job(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if !ai::clip_ready(&state.model_dir) {
        return Err("模型未下载".into());
    }
    let conn = db::open_db(&state.db_path).map_err(to_str)?;

    let pending: Vec<(i64, String, String, i64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, path, media_type, has_thumb FROM photos
                 WHERE id NOT IN (SELECT photo_id FROM embeddings)",
            )
            .map_err(to_str)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(to_str)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(to_str)?
    };
    let total = pending.len() as u64;

    for (i, (id, path, media_type, has_thumb)) in pending.iter().enumerate() {
        let current = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let _ = app.emit(
            "ai://progress",
            AiJobProgress {
                kind: "embeddings".into(),
                processed: (i + 1) as u64,
                total,
                current: current.clone(),
            },
        );

        // 视频没有全幅预览：退而取缩略帧做向量
        let image_path = if media_type == "video" {
            if *has_thumb != 0 {
                Some(crate::thumbs::thumb_path_for(&state.cache_dir, *id))
            } else {
                None
            }
        } else {
            crate::preview::ensure(&state.preview_dir, *id, Path::new(path)).ok().flatten()
        };
        let Some(image_path) = image_path else { continue };
        let Ok(img) = image::open(&image_path) else { continue };
        let Ok(emb) = ai::embed_image(&state.model_dir, &img) else {
            continue;
        };
        let blob = f32_slice_to_blob(&emb);
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (photo_id, embedding) VALUES (?1, ?2)",
            rusqlite::params![id, blob],
        )
        .map_err(to_str)?;
    }

    let _ = app.emit(
        "ai://done",
        AiJobDone {
            kind: "embeddings".into(),
            ok: true,
            error: None,
        },
    );
    Ok(())
}

/// UltraFace 人脸检测（照片），emit "ai://progress"。
#[tauri::command]
pub async fn ai_detect_faces(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    check_not_busy(&state)?;
    let state = state.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = BusyGuard::acquire(&state)?;
        run_face_job(&app, &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn run_face_job(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if !ai::face_ready(&state.model_dir) {
        return Err("模型未下载".into());
    }
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut model = ai::FaceModel::load(&state.model_dir).map_err(|e| format!("{e:#}"))?;

    let pending: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, path FROM photos
                 WHERE media_type = 'photo'
                   AND id NOT IN (SELECT photo_id FROM ai_markers WHERE kind = 'face')",
            )
            .map_err(to_str)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
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
            "ai://progress",
            AiJobProgress {
                kind: "faces".into(),
                processed: (i + 1) as u64,
                total,
                current,
            },
        );

        let image_path =
            crate::preview::ensure(&state.preview_dir, *id, Path::new(path)).ok().flatten();
        let detected: Vec<[f32; 4]> = match image_path {
            Some(p) => match image::open(&p) {
                Ok(img) => model.detect(&img).unwrap_or_default(),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        };

        conn.execute("DELETE FROM faces WHERE photo_id = ?1", [id])
            .map_err(to_str)?;
        for face in &detected {
            conn.execute(
                "INSERT INTO faces (photo_id, x, y, w, h) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, face[0] as f64, face[1] as f64, face[2] as f64, face[3] as f64],
            )
            .map_err(to_str)?;
        }
        conn.execute(
            "INSERT OR REPLACE INTO ai_markers (photo_id, kind) VALUES (?1, 'face')",
            [id],
        )
        .map_err(to_str)?;
    }

    let _ = app.emit(
        "ai://done",
        AiJobDone {
            kind: "faces".into(),
            ok: true,
            error: None,
        },
    );
    Ok(())
}

/// 人脸框列表（归一化坐标）。
#[tauri::command]
pub fn list_faces(state: State<'_, AppState>, photo_id: i64) -> Result<Vec<FaceDto>, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let mut stmt = conn
        .prepare("SELECT x, y, w, h FROM faces WHERE photo_id = ?1 ORDER BY id")
        .map_err(to_str)?;
    let rows = stmt
        .query_map([photo_id], |r| {
            Ok(FaceDto {
                x: r.get(0)?,
                y: r.get(1)?,
                w: r.get(2)?,
                h: r.get(3)?,
            })
        })
        .map_err(to_str)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_str)
}

/// AI 语义搜索：文本向量 vs 图片向量，返回最相似的照片（保持相似度排序）。
#[tauri::command]
pub async fn ai_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<PhotoDto>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai_search_inner(&state, &query, limit.unwrap_or(100)))
        .await
        .map_err(|e| e.to_string())?
}

pub fn ai_search_inner(
    state: &AppState,
    query: &str,
    limit: u32,
) -> Result<Vec<PhotoDto>, String> {
    if !ai::clip_ready(&state.model_dir) {
        return Err("模型未下载".into());
    }
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let query_emb = ai::embed_text(&state.model_dir, query).map_err(|e| format!("{e:#}"))?;

    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let rows: Vec<(i64, Vec<f32>)> = {
        let mut stmt = conn
            .prepare("SELECT photo_id, embedding FROM embeddings")
            .map_err(to_str)?;
        let mapped = stmt
            .query_map([], |r| {
                let id: i64 = r.get(0)?;
                let blob: Vec<u8> = r.get(1)?;
                Ok((id, blob_to_f32_vec(&blob)))
            })
            .map_err(to_str)?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(to_str)?
            .into_iter()
            .filter(|(_, emb)| emb.len() == ai::CLIP_DIM)
            .collect()
    };

    let top = ai::cosine_topk(&query_emb, &rows, limit as usize);
    let sql = format!("SELECT {PHOTO_COLUMNS} FROM photos WHERE id = ?1");
    let mut result = Vec::new();
    for (id, _score) in top {
        let mut photos = query_photos(&conn, &sql, &[&id], &state.cache_dir).map_err(to_str)?;
        if let Some(photo) = photos.pop() {
            result.push(photo);
        }
    }
    Ok(result)
}

fn f32_slice_to_blob(data: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(data.len() * 4);
    for v in data {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

fn blob_to_f32_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_roundtrip() {
        let data = vec![0.5f32, -1.25, 3.0];
        let blob = f32_slice_to_blob(&data);
        assert_eq!(blob.len(), 12);
        assert_eq!(blob_to_f32_vec(&blob), data);
    }
}
