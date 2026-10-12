use crate::db;
use crate::export::{self, ExportFormat};
use crate::models::{ShareCardRequest, ShareCardResult};
use crate::share::ShareStyle;
use crate::AppState;
use std::path::Path;
use tauri::State;

use super::library::to_str;

/// 批量渲染分享卡（选中照片各出一张）。
#[tauri::command]
pub async fn render_share_cards(
    state: State<'_, AppState>,
    req: ShareCardRequest,
) -> Result<ShareCardResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || render_share_cards_inner(&state, &req))
        .await
        .map_err(|e| e.to_string())?
}

pub fn render_share_cards_inner(
    state: &AppState,
    req: &ShareCardRequest,
) -> Result<ShareCardResult, String> {
    let conn = db::open_db(&state.db_path).map_err(to_str)?;
    std::fs::create_dir_all(&req.target_dir).map_err(to_str)?;
    let target = std::path::PathBuf::from(&req.target_dir);

    let background = crate::collage::parse_hex_color(&req.background).unwrap_or([255, 255, 255]);
    let text_color = crate::collage::parse_hex_color(&req.text_color).unwrap_or([30, 30, 30]);

    let mut result = ShareCardResult::default();

    for id in &req.photo_ids {
        let row: rusqlite::Result<(
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<String>,
        )> = conn.query_row(
            "SELECT path, filename, has_thumb, camera_make, camera_model, lens_model,
                    focal_length, aperture, shutter_speed, iso, taken_at, edit
             FROM photos WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                ))
            },
        );

        let (path, filename, has_thumb, make, model, lens, focal, aperture, shutter, iso, taken_at, edit) =
            match row {
                Ok(v) => v,
                Err(_) => {
                    result.failed.push(format!("#{id}"));
                    continue;
                }
            };

        // 解码源：照片走预览管线（视频用缩略帧），并应用调色
        let media_is_video = path
            .to_ascii_lowercase()
            .ends_with(".mov")
            || path.to_ascii_lowercase().ends_with(".mp4")
            || path.to_ascii_lowercase().ends_with(".mkv");
        let image_path = if media_is_video {
            (has_thumb != 0).then(|| crate::thumbs::thumb_path_for(&state.cache_dir, *id))
        } else {
            crate::preview::ensure(&state.preview_dir, *id, Path::new(&path))
                .ok()
                .flatten()
        };
        let Some(image_path) = image_path else {
            result.failed.push(filename);
            continue;
        };
        let Ok(img) = image::open(&image_path) else {
            result.failed.push(filename);
            continue;
        };
        let img = match edit
            .as_deref()
            .and_then(crate::develop::Adjustments::from_json)
            .filter(|a| !a.is_identity())
        {
            Some(adj) => image::DynamicImage::ImageRgb8(crate::develop::render(img, &adj, None)),
            None => img,
        };

        let mut style = ShareStyle::preset(&req.aspect, background, text_color);
        style.show_info = req.show_info;
        style.watermark = req.watermark.clone();

        // 信息条文本
        let mut lines: Vec<String> = Vec::new();
        let camera = [make.as_deref(), model.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(if camera.is_empty() {
            filename.clone()
        } else {
            camera
        });
        let mut params: Vec<String> = Vec::new();
        if let Some(v) = focal {
            params.push(format!("{v}mm"));
        }
        if let Some(v) = aperture {
            params.push(format!("f/{v}"));
        }
        if let Some(v) = &shutter {
            params.push(v.clone());
        }
        if let Some(v) = iso {
            params.push(format!("ISO {v}"));
        }
        if !params.is_empty() {
            lines.push(params.join("  ·  "));
        }
        if let Some(v) = &taken_at {
            lines.push(v.chars().take(10).collect());
        }
        let _ = lens;

        let card = crate::share::render_card(&img, &lines, &style);
        let stem = format!("{}_card", export::file_stem(Path::new(&filename)));
        match export::write_image(
            image::DynamicImage::ImageRgb8(card),
            &target,
            &stem,
            ExportFormat::Jpeg,
            92,
        ) {
            Ok(out) => result.paths.push(out.to_string_lossy().into_owned()),
            Err(e) => result.failed.push(format!("{filename}: {e:#}")),
        }
    }

    if result.paths.is_empty() && !result.failed.is_empty() {
        return Err(format!("全部失败：{}", result.failed.join("；")));
    }
    Ok(result)
}
