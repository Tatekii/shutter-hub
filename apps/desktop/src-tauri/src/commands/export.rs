use crate::db;
use crate::export::{self, ExportFormat};
use crate::models::{ExportProgress, ExportRequest, ExportResult};
use crate::AppState;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

use super::library::to_str;

/// 批量导出选中照片：emit "export://progress" / "export://done"。
#[tauri::command]
pub async fn export_photos(
    app: AppHandle,
    state: State<'_, AppState>,
    req: ExportRequest,
) -> Result<ExportResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        let result = export_photos_inner(&state, &req, move |p| {
            let _ = emitter.emit("export://progress", p);
        });
        if let Ok(res) = &result {
            let _ = app.emit("export://done", res);
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn export_photos_inner(
    state: &AppState,
    req: &ExportRequest,
    mut on_progress: impl FnMut(ExportProgress),
) -> Result<ExportResult, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    std::fs::create_dir_all(&req.target_dir).map_err(to_str)?;
    let target = PathBuf::from(&req.target_dir);
    let format = ExportFormat::parse(&req.format);
    let quality = req.quality.clamp(1, 100);
    let suffix = req.filename_suffix.trim();

    let mut result = ExportResult {
        target_dir: req.target_dir.clone(),
        ..Default::default()
    };
    let total = req.photo_ids.len() as u64;

    for (i, id) in req.photo_ids.iter().enumerate() {
        let row: rusqlite::Result<(String, String, String)> = conn.query_row(
            "SELECT path, filename, media_type FROM photos WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        );

        let current = match &row {
            Ok((_, filename, _)) => filename.clone(),
            Err(_) => format!("#{id}"),
        };

        match row {
            Ok((path, filename, media_type)) => {
                match export_one(
                    state,
                    *id,
                    Path::new(&path),
                    &media_type,
                    &target,
                    format,
                    quality,
                    req.max_dimension,
                    suffix,
                ) {
                    Ok(out) => {
                        result.exported += 1;
                        result.total_bytes +=
                            std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
                    }
                    Err(e) => result.failed.push(format!("{filename}: {e:#}")),
                }
            }
            Err(_) => result.failed.push(format!("#{id}: 记录不存在")),
        }

        on_progress(ExportProgress {
            processed: (i + 1) as u64,
            total,
            current,
        });
    }

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn export_one(
    state: &AppState,
    id: i64,
    src: &Path,
    media_type: &str,
    target: &Path,
    format: ExportFormat,
    quality: u8,
    max_dimension: Option<u32>,
    suffix: &str,
) -> anyhow::Result<PathBuf> {
    let stem = format!("{}{}", export::file_stem(src), suffix);

    if media_type == "video" {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_ascii_lowercase();
        return export::copy_file(src, target, &stem, &ext);
    }

    let preview = crate::preview::ensure(&state.preview_dir, id, src)
        .map_err(|e| anyhow::anyhow!("{e:#}"))?
        .ok_or_else(|| anyhow::anyhow!("无法解码（该格式暂不支持导出）"))?;
    let img = image::open(&preview)?;
    let img = export::fit_long_edge(img, max_dimension);
    export::write_image(img, target, &stem, format, quality)
}
