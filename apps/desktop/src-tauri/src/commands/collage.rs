use crate::collage;
use crate::db;
use crate::export::{self, ExportFormat};
use crate::models::{CollageRequest, CollageResult};
use crate::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

use super::library::to_str;

/// 渲染拼图并保存到目标目录（同步命令，图片量小、渲染快）。
#[tauri::command]
pub fn render_collage(
    state: State<'_, AppState>,
    req: CollageRequest,
) -> Result<CollageResult, String> {
    render_collage_inner(&state, &req)
}

pub fn render_collage_inner(
    state: &AppState,
    req: &CollageRequest,
) -> Result<CollageResult, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    std::fs::create_dir_all(&req.target_dir).map_err(to_str)?;
    let target = PathBuf::from(&req.target_dir);

    let mut images: Vec<Option<image::DynamicImage>> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for id in &req.photo_ids {
        let row: rusqlite::Result<(String, String, i64, String)> = conn.query_row(
            "SELECT path, media_type, has_thumb, filename FROM photos WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        );
        match row {
            Ok((path, media_type, has_thumb, filename)) => {
                let image_path: Option<PathBuf> = if media_type == "video" {
                    (has_thumb != 0).then(|| crate::thumbs::thumb_path_for(&state.cache_dir, *id))
                } else {
                    crate::preview::ensure(&state.preview_dir, *id, Path::new(&path))
                        .ok()
                        .flatten()
                };
                match image_path.and_then(|p| image::open(p).ok()) {
                    Some(img) => images.push(Some(img)),
                    None => {
                        failed.push(filename);
                        images.push(None);
                    }
                }
            }
            Err(_) => {
                failed.push(format!("#{id}"));
                images.push(None);
            }
        }
    }

    if images.iter().all(|i| i.is_none()) {
        return Err("没有可用于拼图的可解码照片".to_string());
    }

    let bg = collage::parse_hex_color(&req.background).unwrap_or([255, 255, 255]);
    let canvas = collage::render(
        &images,
        &req.layout,
        req.cols,
        req.gap,
        req.radius,
        bg,
        req.size,
    );
    let (width, height) = canvas.dimensions();

    let stem = format!(
        "collage_{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    );
    let out = export::write_image(
        image::DynamicImage::ImageRgb8(canvas),
        &target,
        &stem,
        ExportFormat::Jpeg,
        req.quality.clamp(1, 100),
    )
    .map_err(|e| format!("{e:#}"))?;

    Ok(CollageResult {
        path: out.to_string_lossy().into_owned(),
        width,
        height,
        failed,
    })
}
