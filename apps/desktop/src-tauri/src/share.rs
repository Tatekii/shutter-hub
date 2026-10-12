//! 分享卡：照片 + EXIF 信息条的社交直发卡片（1:1 / 4:5 / 3:4）。
//!
//! 信息条包含相机、参数、时间；可选右下角水印文字。字体缺失时降级为
//! 只剩照片的纯图卡片。

use image::{Rgb, RgbImage};

#[derive(Debug, Clone)]
pub struct ShareStyle {
    pub width: u32,
    pub height: u32,
    pub margin: u32,
    pub background: [u8; 3],
    pub text_color: [u8; 3],
    pub show_info: bool,
    pub watermark: String,
}

impl ShareStyle {
    pub fn preset(aspect: &str, background: [u8; 3], text_color: [u8; 3]) -> Self {
        let (width, height) = match aspect {
            "1:1" => (1440, 1440),
            "3:4" => (1440, 1920),
            _ => (1440, 1800), // 4:5 默认
        };
        Self {
            width,
            height,
            margin: width / 20,
            background,
            text_color,
            show_info: true,
            watermark: String::new(),
        }
    }
}

/// cover 裁剪到目标宽高（先按比例裁剪再缩放）。
pub fn cover_crop(img: &image::DynamicImage, w: u32, h: u32) -> RgbImage {
    let (iw, ih) = (img.width().max(1), img.height().max(1));
    let target_ratio = w as f64 / h as f64;
    let src_ratio = iw as f64 / ih as f64;
    let (cw, ch) = if src_ratio > target_ratio {
        (((ih as f64) * target_ratio) as u32, ih)
    } else {
        (iw, ((iw as f64) / target_ratio) as u32)
    };
    let x = (iw - cw) / 2;
    let y = (ih - ch) / 2;
    img.crop_imm(x, y, cw, ch)
        .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        .to_rgb8()
}

/// 渲染分享卡。lines 为信息条逐行文本（空数组 = 不渲染信息条）。
pub fn render_card(photo: &image::DynamicImage, lines: &[String], style: &ShareStyle) -> RgbImage {
    let w = style.width;
    let h = style.height;
    let m = style.margin;

    let line_h = (w / 26) as i32;
    let font_scale = w as f32 / 40.0;
    let non_empty: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    let info_h = if style.show_info && !non_empty.is_empty() {
        (line_h as u32) * (non_empty.len() as u32) + m / 2
    } else {
        0
    };

    // 信息条放不下时（超小画布）退回纯图
    let photo_w = w - 2 * m;
    let photo_h = h.saturating_sub(2 * m + info_h).max(64);
    let info_h = h.saturating_sub(2 * m + photo_h);

    let mut canvas = RgbImage::from_pixel(w, h, Rgb(style.background));
    let photo_rgb = cover_crop(photo, photo_w, photo_h);

    // 贴照片
    for (x, y, p) in photo_rgb.enumerate_pixels() {
        canvas.put_pixel(m + x, m + y, *p);
    }

    // 水印（照片区右下角）
    if !style.watermark.trim().is_empty() {
        let wm_scale = w as f32 / 46.0;
        let wm_margin = m / 2;
        let right_margin = m + wm_margin;
        let bottom_margin = m + info_h + wm_margin;
        crate::text::draw_text_bottom_right(
            &mut canvas,
            &style.watermark,
            right_margin,
            bottom_margin,
            wm_scale,
            [255, 255, 255],
            0.8,
        );
    }

    // 信息条
    if info_h > 0 {
        let x = m as i32;
        let mut baseline = (m + photo_h + m / 2) as i32 + line_h;
        for line in non_empty {
            crate::text::draw_text(
                &mut canvas,
                line,
                x,
                baseline,
                font_scale,
                style.text_color,
                1.0,
            );
            baseline += line_h;
        }
    }

    canvas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_crop_matches_target_size() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            600,
            400,
            Rgb([120, 80, 40]),
        ));
        let out = cover_crop(&img, 200, 200);
        assert_eq!(out.dimensions(), (200, 200));
        let tall = cover_crop(&img, 200, 300);
        assert_eq!(tall.dimensions(), (200, 300));
    }

    #[test]
    fn card_preset_dimensions() {
        let s = ShareStyle::preset("1:1", [255, 255, 255], [30, 30, 30]);
        assert_eq!((s.width, s.height), (1440, 1440));
        let s = ShareStyle::preset("4:5", [255, 255, 255], [30, 30, 30]);
        assert_eq!((s.width, s.height), (1440, 1800));
        let s = ShareStyle::preset("3:4", [255, 255, 255], [30, 30, 30]);
        assert_eq!((s.width, s.height), (1440, 1920));
    }

    #[test]
    fn render_card_has_background_border() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            300,
            300,
            Rgb([200, 30, 30]),
        ));
        let mut style = ShareStyle::preset("1:1", [255, 255, 255], [30, 30, 30]);
        style.show_info = false;
        let card = render_card(&img, &[], &style);
        assert_eq!(card.dimensions(), (1440, 1440));
        // 角落是背景色
        assert_eq!(card.get_pixel(0, 0), &Rgb([255, 255, 255]));
        // 照片区域中心是照片
        let center = card.get_pixel(720, 720);
        assert!(center[0] > 150 && center[1] < 80, "photo center: {center:?}");
    }
}
