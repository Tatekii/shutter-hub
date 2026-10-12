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
        let row: rusqlite::Result<(String, String, String, Option<String>)> = conn.query_row(
            "SELECT path, filename, media_type, edit FROM photos WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        );

        let current = match &row {
            Ok((_, filename, _, _)) => filename.clone(),
            Err(_) => format!("#{id}"),
        };

        match row {
            Ok((path, filename, media_type, edit)) => {
                let adj = edit
                    .as_deref()
                    .and_then(crate::develop::Adjustments::from_json)
                    .filter(|a| !a.is_identity());
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
                    adj.as_ref(),
                    req.watermark.as_deref(),
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

/// 导出 Live 图的运动片段：GIF 动图（需 ffmpeg）或原样拷贝。
#[tauri::command]
pub async fn export_live_motion(
    state: State<'_, AppState>,
    photo_id: i64,
    target_dir: String,
    format: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        export_live_motion_inner(&state, photo_id, &target_dir, &format)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn export_live_motion_inner(
    state: &AppState,
    photo_id: i64,
    target_dir: &str,
    format: &str,
) -> Result<String, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    let motion: Option<String> = conn
        .query_row(
            "SELECT live_motion FROM photos WHERE id = ?1",
            [photo_id],
            |r| r.get(0),
        )
        .map_err(to_str)?;
    let motion = motion.ok_or_else(|| "该照片没有配对实况片段".to_string())?;
    let motion = PathBuf::from(motion);
    if !motion.exists() {
        return Err("实况片段文件不存在（可能已被移动）".to_string());
    }

    std::fs::create_dir_all(target_dir).map_err(to_str)?;
    let target = Path::new(target_dir);
    let stem = export::file_stem(&motion);

    if format == "copy" {
        let ext = motion
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mov")
            .to_ascii_lowercase();
        let out = export::copy_file(&motion, target, &stem, &ext).map_err(|e| format!("{e:#}"))?;
        return Ok(out.to_string_lossy().into_owned());
    }

    let out = export::unique_path(target, &stem, "gif");
    if crate::video::make_gif(&motion, &out) {
        Ok(out.to_string_lossy().into_owned())
    } else {
        Err("导出 GIF 需要安装 ffmpeg（如 brew install ffmpeg）".to_string())
    }
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
    adj: Option<&crate::develop::Adjustments>,
    watermark: Option<&str>,
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
    // 非破坏性调色：导出时按全分辨率应用
    let img = match adj {
        Some(adj) => image::DynamicImage::ImageRgb8(crate::develop::render(img, adj, None)),
        None => img,
    };
    let img = export::fit_long_edge(img, max_dimension);
    // 水印（右下角，带阴影）
    let img = match watermark {
        Some(text) if !text.trim().is_empty() => {
            let mut rgb = crate::preview::flatten_to_rgb(img).to_rgb8();
            let margin = (rgb.width() / 40).max(8);
            let scale = (rgb.width() as f32 / 42.0).max(12.0);
            crate::text::draw_text_bottom_right(
                &mut rgb,
                text,
                margin,
                margin,
                scale,
                [255, 255, 255],
                0.85,
            );
            image::DynamicImage::ImageRgb8(rgb)
        }
        _ => img,
    };
    export::write_image(img, target, &stem, format, quality)
}
