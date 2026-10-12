//! 文字渲染（水印 / 分享卡信息条）：fontdb 发现系统字体 + ab_glyph 光栅化。
//!
//! 找不到可用字体（极端精简系统）时所有绘制静默跳过，调用方按无文字降级。

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use std::sync::OnceLock;

/// 按文本内容选择字体：含 CJK 时优先中文字体，否则无衬线。
fn candidate_families(has_cjk: bool) -> Vec<&'static str> {
    if has_cjk {
        vec![
            "PingFang SC",
            "Hiragino Sans GB",
            "Microsoft YaHei",
            "SimHei",
            "Noto Sans CJK SC",
            "Noto Sans SC",
            "Source Han Sans SC",
            "WenQuanYi Micro Hei",
            "SansSerif",
        ]
    } else {
        vec![
            "Helvetica Neue",
            "Segoe UI",
            "DejaVu Sans",
            "Noto Sans",
            "SansSerif",
        ]
    }
}

fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            c as u32,
            0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF
        )
    })
}

/// 加载系统字体（按缓存复用）。返回能覆盖全部字符的字体。
fn font_for(text: &str) -> Option<&'static FontVec> {
    static LATIN: OnceLock<Option<FontVec>> = OnceLock::new();
    static CJK: OnceLock<Option<FontVec>> = OnceLock::new();

    let cell = if has_cjk(text) { &CJK } else { &LATIN };
    cell.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let families = candidate_families(has_cjk(text));
        for family in families {
            let query = fontdb::Query {
                families: &[fontdb::Family::Name(family)],
                ..Default::default()
            };
            let Some(id) = db.query(&query) else { continue };
            let Some((data, index)) = db.with_face_data(id, |data, index| (data.to_vec(), index))
            else {
                continue;
            };
            let Ok(font) = FontVec::try_from_vec_and_index(data, index) else {
                continue;
            };
            if text.chars().all(|c| c == ' ' || font.glyph_id(c).0 != 0) {
                return Some(font);
            }
        }
        None
    })
    .as_ref()
}

/// 文本的渲染尺寸（宽 × 行高），scale 为像素高度。
pub fn measure(text: &str, scale_px: f32) -> Option<(f32, f32)> {
    let font = font_for(text)?;
    let scaled = font.as_scaled(PxScale::from(scale_px));
    let mut width = 0.0f32;
    for c in text.chars() {
        width += scaled.h_advance(font.glyph_id(c));
    }
    Some((width, scale_px))
}

/// 在 (x, baseline_y) 处绘制文字；alpha 0-1。
/// 返回是否真正绘制（字体缺失 / 文本为空时为 false）。
pub fn draw_text(
    img: &mut image::RgbImage,
    text: &str,
    x: i32,
    baseline_y: i32,
    scale_px: f32,
    color: [u8; 3],
    alpha: f32,
) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    let Some(font) = font_for(text) else {
        return false;
    };
    let scale = PxScale::from(scale_px);
    let scaled = font.as_scaled(scale);
    let mut pen_x = x as f32;
    let alpha = alpha.clamp(0.0, 1.0);

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        if let Some(outlined) = font.outline_glyph(glyph_id.with_scale_and_position(
            scale,
            ab_glyph::point(pen_x, baseline_y as f32),
        )) {
            // draw() 回调坐标相对字形包围盒左上角，需加 px_bounds.min 偏移
            let bounds_min = outlined.px_bounds().min;
            let (bx, by) = (bounds_min.x as i32, bounds_min.y as i32);
            outlined.draw(|gx, gy, coverage| {
                let px = bx + gx as i32;
                let py = by + gy as i32;
                if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                    return;
                }
                let a = coverage * alpha;
                if a <= 0.001 {
                    return;
                }
                let dst = img.get_pixel_mut(px as u32, py as u32);
                for ch in 0..3 {
                    dst[ch] = (dst[ch] as f32 * (1.0 - a) + color[ch] as f32 * a)
                        .clamp(0.0, 255.0) as u8;
                }
            });
        }
        pen_x += scaled.h_advance(glyph_id);
    }
    true
}

/// 右下角水印：带 1px 深色阴影提高可读性。分别指定右 / 下边距。
pub fn draw_text_bottom_right(
    img: &mut image::RgbImage,
    text: &str,
    right_margin: u32,
    bottom_margin: u32,
    scale_px: f32,
    color: [u8; 3],
    alpha: f32,
) -> bool {
    let Some((width, height)) = measure(text, scale_px) else {
        return false;
    };
    let x = (img.width() as f32 - right_margin as f32 - width).max(4.0);
    let baseline =
        (img.height() as f32 - bottom_margin as f32 - (height - scale_px) * 0.5).max(scale_px);
    // 阴影（右下 1px）
    draw_text(img, text, x as i32 + 1, baseline as i32 + 1, scale_px, [0, 0, 0], alpha * 0.6);
    draw_text(img, text, x as i32, baseline as i32, scale_px, color, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_detection() {
        assert!(has_cjk("你好 world"));
        assert!(!has_cjk("hello world"));
    }

    /// 系统字体存在时应能绘制出像素；字体缺失时跳过（CI 极简环境）。
    #[test]
    fn draws_text_when_font_available() {
        let mut img = image::RgbImage::from_pixel(200, 60, image::Rgb([0, 0, 0]));
        let before = img.clone();
        let drew = draw_text(&mut img, "Shutter Hub", 10, 40, 28.0, [255, 255, 255], 1.0);
        if drew {
            assert_ne!(img, before, "text should change pixels");
        } else {
            eprintln!("no system font available, skipping pixel assertion");
        }
    }

    #[test]
    fn empty_text_noop() {
        let mut img = image::RgbImage::new(50, 50);
        let before = img.clone();
        assert!(!draw_text(&mut img, "  ", 0, 10, 16.0, [255, 255, 255], 1.0));
        assert_eq!(img, before);
    }
}
