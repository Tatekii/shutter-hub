//! 暗房 v1：非破坏性基础调色（CPU 管线）。
//!
//! 参数口径与 Lightroom 一致（写入 XMP crs 命名空间）：
//! exposure 为 EV（-5..5），其余为 -100..100 的滑块值。
//! 渲染顺序：曝光 → 白平衡 → 高光/阴影 → 对比 → 饱和度。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Adjustments {
    /// EV，-5..5
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    /// 相对色温 -100..100（JPEG 口径，Lightroom 非 RAW 同款）
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
}

impl Default for Adjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
            saturation: 0.0,
        }
    }
}

impl Adjustments {
    pub fn is_identity(&self) -> bool {
        let a = [
            self.exposure,
            self.contrast,
            self.highlights,
            self.shadows,
            self.temperature,
            self.tint,
            self.saturation,
        ];
        a.iter().all(|v| v.abs() < 0.001)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

/// 应用到 RGB8 图像（原地）。参数已做区间钳制。
pub fn apply_in_place(img: &mut image::RgbImage, adj: &Adjustments) {
    if adj.is_identity() {
        return;
    }

    let exposure_factor = 2f32.powf(adj.exposure.clamp(-5.0, 5.0));
    let contrast = adj.contrast.clamp(-100.0, 100.0) / 100.0;
    let highlights = adj.highlights.clamp(-100.0, 100.0) / 100.0;
    let shadows = adj.shadows.clamp(-100.0, 100.0) / 100.0;
    let temp = adj.temperature.clamp(-100.0, 100.0) / 100.0;
    let tint = adj.tint.clamp(-100.0, 100.0) / 100.0;
    let saturation = adj.saturation.clamp(-100.0, 100.0) / 100.0;

    let temp_r = 1.0 + 0.30 * temp;
    let temp_b = 1.0 - 0.30 * temp;
    let tint_g = 1.0 - 0.20 * tint;

    for pixel in img.pixels_mut() {
        let mut r = pixel[0] as f32 / 255.0;
        let mut g = pixel[1] as f32 / 255.0;
        let mut b = pixel[2] as f32 / 255.0;

        // 曝光
        r *= exposure_factor;
        g *= exposure_factor;
        b *= exposure_factor;

        // 白平衡
        r *= temp_r;
        b *= temp_b;
        g *= tint_g;

        // 高光 / 阴影（基于亮度权重）
        if highlights != 0.0 || shadows != 0.0 {
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let hl_mask = (lum * lum).clamp(0.0, 1.0);
            let sh_mask = ((1.0 - lum) * (1.0 - lum)).clamp(0.0, 1.0);
            let hl_delta = highlights * hl_mask;
            let sh_delta = shadows * sh_mask;
            for v in [&mut r, &mut g, &mut b] {
                if highlights != 0.0 {
                    *v += hl_delta * if highlights > 0.0 { 1.0 - *v } else { *v };
                }
                if shadows != 0.0 {
                    *v += sh_delta * if shadows > 0.0 { 1.0 - *v } else { *v };
                }
            }
        }

        // 对比（围绕中灰的线性扩展）
        if contrast != 0.0 {
            let k = 1.0 + contrast * 0.8;
            r = (r - 0.5) * k + 0.5;
            g = (g - 0.5) * k + 0.5;
            b = (b - 0.5) * k + 0.5;
        }

        // 饱和度
        if saturation != 0.0 {
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let k = 1.0 + saturation;
            r = lum + (r - lum) * k;
            g = lum + (g - lum) * k;
            b = lum + (b - lum) * k;
        }

        pixel[0] = (r * 255.0).clamp(0.0, 255.0) as u8;
        pixel[1] = (g * 255.0).clamp(0.0, 255.0) as u8;
        pixel[2] = (b * 255.0).clamp(0.0, 255.0) as u8;
    }
}

/// 供预览/导出共用的渲染入口：按长边上限缩放后应用调整。
pub fn render(
    img: image::DynamicImage,
    adj: &Adjustments,
    max_long_edge: Option<u32>,
) -> image::RgbImage {
    let img = match max_long_edge {
        Some(max) if img.width().max(img.height()) > max => {
            img.resize(max, max, image::imageops::FilterType::Lanczos3)
        }
        _ => img,
    };
    let mut rgb = crate::preview::flatten_to_rgb(img).to_rgb8();
    apply_in_place(&mut rgb, adj);
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(v: u8) -> image::RgbImage {
        image::RgbImage::from_pixel(4, 4, image::Rgb([v, v, v]))
    }

    #[test]
    fn identity_is_noop() {
        let mut img = gray(120);
        let before = img.clone();
        apply_in_place(&mut img, &Adjustments::default());
        assert_eq!(img, before);
        assert!(Adjustments::default().is_identity());
    }

    #[test]
    fn exposure_stop_doubles() {
        let mut img = gray(64);
        let adj = Adjustments {
            exposure: 1.0,
            ..Default::default()
        };
        apply_in_place(&mut img, &adj);
        let v = img.get_pixel(0, 0)[0];
        assert!((v as i32 - 128).abs() <= 2, "64 * 2^1 ≈ 128, got {v}");
    }

    #[test]
    fn saturation_minus_100_grays_out() {
        let mut img = image::RgbImage::from_pixel(2, 2, image::Rgb([200, 50, 100]));
        let adj = Adjustments {
            saturation: -100.0,
            ..Default::default()
        };
        apply_in_place(&mut img, &adj);
        let p = img.get_pixel(0, 0);
        assert!(
            (p[0] as i32 - p[1] as i32).abs() <= 1
                && (p[1] as i32 - p[2] as i32).abs() <= 1,
            "should be gray: {p:?}"
        );
    }

    #[test]
    fn warm_temperature_raises_red_lowers_blue() {
        let mut img = gray(128);
        let adj = Adjustments {
            temperature: 100.0,
            ..Default::default()
        };
        apply_in_place(&mut img, &adj);
        let p = img.get_pixel(0, 0);
        assert!(p[0] > 128 && p[2] < 128, "warm: {p:?}");
    }

    #[test]
    fn highlights_pull_down_only_bright_pixels() {
        let adj = Adjustments {
            highlights: -100.0,
            ..Default::default()
        };
        let mut bright = gray(230);
        apply_in_place(&mut bright, &adj);
        let mut dark = gray(40);
        apply_in_place(&mut dark, &adj);
        assert!(bright.get_pixel(0, 0)[0] < 200, "bright recovered");
        let dark_after = dark.get_pixel(0, 0)[0] as i32;
        assert!((dark_after - 40).abs() <= 2, "dark nearly untouched: {dark_after}");
    }

    #[test]
    fn json_roundtrip_and_identity() {
        let adj = Adjustments {
            exposure: 1.25,
            contrast: -30.0,
            ..Default::default()
        };
        let json = adj.to_json();
        assert_eq!(Adjustments::from_json(&json), Some(adj.clone()));
        assert!(!adj.is_identity());
        assert!(Adjustments::from_json("").is_none());
    }
}
