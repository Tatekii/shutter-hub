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
    /// 裁剪 [top, left, bottom, right]，0..1，作用于旋转后的图像
    pub crop: [f32; 4],
    /// 顺时针旋转：0 | 90 | 180 | 270（u16 存度数）
    pub rotation: u16,
    /// RGB 复合曲线控制点（0..1，按 x 升序）；空 = 未使用
    pub curve: Vec<[f32; 2]>,
    /// HSL 8 波段（红橙黄绿青蓝紫品红）饱和度 -100..100
    pub hsl_sat: [f32; 8],
    /// HSL 8 波段亮度 -100..100
    pub hsl_lum: [f32; 8],
}

fn default_crop() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
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
            crop: default_crop(),
            rotation: 0,
            curve: Vec::new(),
            hsl_sat: [0.0; 8],
            hsl_lum: [0.0; 8],
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
        if !a.iter().all(|v| v.abs() < 0.001) {
            return false;
        }
        if self.rotation % 360 != 0 {

            return false;
        }
        if self.crop != default_crop() {
            return false;
        }
        if !self.curve.is_empty() && curve_lut(&self.curve).is_some() {
            return false;
        }
        if self.hsl_sat.iter().any(|v| v.abs() > 0.001) {
            return false;
        }
        if self.hsl_lum.iter().any(|v| v.abs() > 0.001) {
            return false;
        }
        true
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }

    /// 归一化并钳制的裁剪矩形 [top, left, bottom, right]。
    pub fn crop_rect(&self) -> [f32; 4] {
        let [mut t, mut l, mut b, mut r] = self.crop;
        for v in [&mut t, &mut l, &mut b, &mut r] {
            *v = v.clamp(0.0, 1.0);
        }
        if b - t < 0.02 {
            b = (t + 0.02).min(1.0);
            t = (b - 0.02).max(0.0);
        }
        if r - l < 0.02 {
            r = (l + 0.02).min(1.0);
            l = (r - 0.02).max(0.0);
        }
        [t, l, b, r]
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

/// 供预览/导出共用的渲染入口：旋转 → 裁剪 → 长边缩放 → 基础调整 → HSL → 曲线。
pub fn render(
    img: image::DynamicImage,
    adj: &Adjustments,
    max_long_edge: Option<u32>,
) -> image::RgbImage {
    // 几何：旋转（顺时针）+ 裁剪（旋转后空间）
    let img = match adj.rotation % 360 {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    };
    let [t, l, b, r] = adj.crop_rect();
    let img = if [t, l, b, r] != default_crop() {
        let w = img.width() as f32;
        let h = img.height() as f32;
        let x = (l * w).floor() as u32;
        let y = (t * h).floor() as u32;
        let cw = (((r - l) * w).floor() as u32).max(1).min(img.width() - x);
        let ch = (((b - t) * h).floor() as u32).max(1).min(img.height() - y);
        img.crop_imm(x, y, cw, ch)
    } else {
        img
    };

    let img = match max_long_edge {
        Some(max) if img.width().max(img.height()) > max => {
            img.resize(max, max, image::imageops::FilterType::Lanczos3)
        }
        _ => img,
    };
    let mut rgb = crate::preview::flatten_to_rgb(img).to_rgb8();
    apply_in_place(&mut rgb, adj);
    apply_hsl(&mut rgb, &adj.hsl_sat, &adj.hsl_lum);
    if let Some(lut) = curve_lut(&adj.curve) {
        apply_curve(&mut rgb, &lut);
    }
    rgb
}

/// 单调三次插值（Fritsch–Carlson）构建 256 项 LUT；无效果时返回 None。
pub fn curve_lut(points: &[[f32; 2]]) -> Option<[u8; 256]> {
    let mut pts: Vec<(f32, f32)> = points
        .iter()
        .map(|p| (p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0)))
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-4);
    if pts.len() < 2 {
        return None;
    }
    // 与默认直线 (y=x) 一致 → 无效果
    if pts.iter().all(|(x, y)| (x - y).abs() < 0.002) {
        return None;
    }

    let n = pts.len();
    let xs: Vec<f32> = pts.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = pts.iter().map(|p| p.1).collect();

    // 斜率
    let mut d: Vec<f32> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        d.push((ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]));
    }
    let mut m: Vec<f32> = vec![0.0; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for i in 1..n - 1 {
        if d[i - 1] * d[i] <= 0.0 {
            m[i] = 0.0;
        } else {
            m[i] = (d[i - 1] + d[i]) / 2.0;
        }
    }
    // Fritsch–Carlson 限制，保证单调
    for i in 0..n - 1 {
        if d[i].abs() < 1e-6 {
            m[i] = 0.0;
            m[i + 1] = 0.0;
            continue;
        }
        let a = m[i] / d[i];
        let b = m[i + 1] / d[i];
        let s = a * a + b * b;
        if s > 9.0 {
            let tau = 3.0 / s.sqrt();
            m[i] = tau * a * d[i];
            m[i + 1] = tau * b * d[i];
        }
    }

    let mut lut = [0u8; 256];
    for (i, item) in lut.iter_mut().enumerate() {
        let x = i as f32 / 255.0;
        let y = if x <= xs[0] {
            ys[0]
        } else if x >= xs[n - 1] {
            ys[n - 1]
        } else {
            // 定位区间
            let mut k = 0;
            while k + 2 < n && x > xs[k + 1] {
                k += 1;
            }
            let h = xs[k + 1] - xs[k];
            let t = (x - xs[k]) / h;
            let t2 = t * t;
            let t3 = t2 * t;
            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;
            h00 * ys[k] + h10 * h * m[k] + h01 * ys[k + 1] + h11 * h * m[k + 1]
        };
        *item = (y.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    Some(lut)
}

/// 应用 RGB 复合曲线。
pub fn apply_curve(img: &mut image::RgbImage, lut: &[u8; 256]) {
    for p in img.pixels_mut() {
        p[0] = lut[p[0] as usize];
        p[1] = lut[p[1] as usize];
        p[2] = lut[p[2] as usize];
    }
}

/// HSL 8 波段中心色相（红橙黄绿青蓝紫品红）。
pub const HSL_BANDS: [f32; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 280.0, 320.0];

/// HSL 波段调整（饱和度乘性、亮度加性），权重为三角窗。
pub fn apply_hsl(img: &mut image::RgbImage, sat: &[f32; 8], lum: &[f32; 8]) {
    let sat_active = sat.iter().any(|v| v.abs() > 0.001);
    let lum_active = lum.iter().any(|v| v.abs() > 0.001);
    if !sat_active && !lum_active {
        return;
    }

    for p in img.pixels_mut() {
        let (h, s, l) = rgb_to_hsl(p[0], p[1], p[2]);
        if s < 0.02 {
            continue; // 近灰像素不参与色彩调整
        }
        let mut dsat = 0.0f32;
        let mut dlum = 0.0f32;
        for (i, center) in HSL_BANDS.iter().enumerate() {
            let mut d = (h - center).abs();
            if d > 180.0 {
                d = 360.0 - d;
            }
            let w = (1.0 - d / 60.0).clamp(0.0, 1.0);
            if w > 0.0 {
                dsat += w * sat[i] / 100.0;
                dlum += w * lum[i] / 100.0;
            }
        }
        let s2 = (s * (1.0 + dsat)).clamp(0.0, 1.0);
        let l2 = (l + dlum * 0.4).clamp(0.0, 1.0);
        let [r, g, b] = hsl_to_rgb(h, s2, l2);
        p[0] = r;
        p[1] = g;
        p[2] = b;
    }
}

fn rgb_to_hsl(r8: u8, g8: u8, b8: u8) -> (f32, f32, f32) {
    let r = r8 as f32 / 255.0;
    let g = g8 as f32 / 255.0;
    let b = b8 as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0) * 60.0
    } else if max == g {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    if s < 1e-6 {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hk = h / 360.0;
    let conv = |t: f32| -> f32 {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    [
        (conv(hk + 1.0 / 3.0) * 255.0).clamp(0.0, 255.0) as u8,
        (conv(hk) * 255.0).clamp(0.0, 255.0) as u8,
        (conv(hk - 1.0 / 3.0) * 255.0).clamp(0.0, 255.0) as u8,
    ]
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

    #[test]
    fn rotation_and_crop_geometry() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(400, 200));
        // 旋转 90° → 尺寸互换
        let adj = Adjustments {
            rotation: 90,
            ..Default::default()
        };
        let rotated = render(img.clone(), &adj, None);
        assert_eq!(rotated.dimensions(), (200, 400));

        // 旋转 + 裁剪（旋转后空间）：取左上一半 → 100x200
        let adj = Adjustments {
            rotation: 90,
            crop: [0.0, 0.0, 0.5, 0.5],
            ..Default::default()
        };
        let cut = render(img, &adj, None);
        assert_eq!(cut.dimensions(), (100, 200));
    }

    #[test]
    fn curve_lut_identity_and_lift() {
        assert!(curve_lut(&[]).is_none());
        let identity = vec![[0.0, 0.0], [0.5, 0.5], [1.0, 1.0]];
        assert!(curve_lut(&identity).is_none(), "y=x treated as identity");

        let lift = vec![[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]];
        let lut = curve_lut(&lift).expect("non-identity");
        assert!(lut[128] > 128 + 20, "midtones lifted: {}", lut[128]);
        assert!(lut[255] == 255, "white preserved: {}", lut[255]);
        // 单调：不失真
        assert!(lut.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn hsl_band_adjustments() {
        // 纯红像素：只提饱和度 → 更纯（绿分量下降）
        let mut img = image::RgbImage::from_pixel(2, 2, image::Rgb([200, 40, 40]));
        let mut sat = [0.0f32; 8];
        let mut lum = [0.0f32; 8];
        sat[0] = 50.0; // 红
        apply_hsl(&mut img, &sat, &lum);
        let p = img.get_pixel(0, 0);
        assert!(p[1] < 30, "more saturated red: {p:?}");

        // 再提亮度 → 整体更亮
        let before_l = {
            let p = img.get_pixel(0, 0);
            p[0] as i32 + p[1] as i32 + p[2] as i32
        };
        lum[0] = 60.0;
        apply_hsl(&mut img, &[0.0f32; 8], &lum);
        let p2 = img.get_pixel(0, 0);
        let after_l = p2[0] as i32 + p2[1] as i32 + p2[2] as i32;
        assert!(after_l > before_l, "luminance up");

        // 灰色不受影响
        let mut gray = image::RgbImage::from_pixel(1, 1, image::Rgb([128, 128, 128]));
        let before = gray.clone();
        apply_hsl(&mut gray, &sat, &lum);
        assert_eq!(gray, before);
    }

    #[test]
    fn crop_rect_clamps_and_enforces_minimum() {
        let adj = Adjustments {
            crop: [-0.5, 0.9, 0.1, 1.2],
            ..Default::default()
        };
        let [t, l, b, r] = adj.crop_rect();
        assert!(t >= 0.0 && l >= 0.0 && b <= 1.0 && r <= 1.0);
        assert!(b - t >= 0.02 && r - l >= 0.02);
    }
}
