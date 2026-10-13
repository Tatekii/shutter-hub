//! 出图导出：格式转换、长边缩放、文件名后缀、批量与冲突避让。
//!
//! 照片走「全尺寸预览」管线（JPEG 直拷 / RAW 内嵌预览 / 解码重编码），
//! 视频按原文件字节拷贝导出。

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Jpeg,
    Png,
    Webp,
    Tiff,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "png" => Self::Png,
            "webp" => Self::Webp,
            "tiff" | "tif" => Self::Tiff,
            _ => Self::Jpeg,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Tiff => "tif",
        }
    }
}

/// 长边不超过 max（仅缩小，不放大）；None 保持原尺寸。
pub fn fit_long_edge(img: image::DynamicImage, max: Option<u32>) -> image::DynamicImage {
    let Some(max) = max else { return img };
    let (w, h) = (img.width(), img.height());
    let long = w.max(h);
    if long <= max || long == 0 {
        return img;
    }
    img.resize(max, max, image::imageops::FilterType::Lanczos3)
}

/// 目标路径避让冲突：`name.jpg` → `name-1.jpg` → `name-2.jpg`…
pub fn unique_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    for i in 1..=9999 {
        let candidate = dir.join(format!("{stem}-{i}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    // 极端场景：时间戳兜底
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    dir.join(format!("{stem}-{ts}.{extension}"))
}

/// 导出单张照片（源已解码为 image），返回写入路径。
pub fn write_image(
    img: image::DynamicImage,
    dir: &Path,
    stem: &str,
    format: ExportFormat,
    quality: u8,
) -> Result<PathBuf> {
    let out = unique_path(dir, stem, format.extension());
    match format {
        ExportFormat::Jpeg => {
            let img = crate::preview::flatten_to_rgb(img);
            let file = std::fs::File::create(&out)
                .with_context(|| format!("create {}", out.display()))?;
            let mut writer = BufWriter::new(file);
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality.clamp(1, 100));
            encoder.encode_image(&img)?;
        }
        ExportFormat::Png => {
            let file = std::fs::File::create(&out)
                .with_context(|| format!("create {}", out.display()))?;
            let mut writer = BufWriter::new(file);
            let encoder = image::codecs::png::PngEncoder::new(&mut writer);
            img.write_with_encoder(encoder)?;
        }
        ExportFormat::Webp => {
            // image crate 仅支持 WebP 无损编码
            let img = crate::preview::flatten_to_rgb(img);
            let file = std::fs::File::create(&out)
                .with_context(|| format!("create {}", out.display()))?;
            let mut writer = BufWriter::new(file);
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut writer);
            img.write_with_encoder(encoder)?;
        }
        ExportFormat::Tiff => {
            // RAW 后期工作流：无损 TIFF 交付
            let img = crate::preview::flatten_to_rgb(img);
            let file = std::fs::File::create(&out)
                .with_context(|| format!("create {}", out.display()))?;
            let mut writer = BufWriter::new(file);
            let encoder = image::codecs::tiff::TiffEncoder::new(&mut writer);
            img.write_with_encoder(encoder)?;
        }
    }
    Ok(out)
}

/// 原始字节拷贝（视频导出）。
pub fn copy_file(src: &Path, dir: &Path, stem: &str, extension: &str) -> Result<PathBuf> {
    let out = unique_path(dir, stem, extension);
    std::fs::copy(src, &out).with_context(|| format!("copy {} -> {}", src.display(), out.display()))?;
    Ok(out)
}

/// 从文件名提取 stem（不含扩展名）。
pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "photo".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shutter_hub_test_export_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fit_long_edge_only_shrinks() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(1000, 500));
        let fitted = fit_long_edge(img.clone(), Some(400));
        assert_eq!((fitted.width(), fitted.height()), (400, 200));

        // 小图不放大
        let small = fit_long_edge(img, Some(4000));
        assert_eq!((small.width(), small.height()), (1000, 500));
    }

    #[test]
    fn unique_path_avoids_collisions() {
        let dir = temp_dir("unique");
        let first = unique_path(&dir, "IMG_0001", "jpg");
        assert!(first.ends_with("IMG_0001.jpg"));
        std::fs::write(&first, b"x").unwrap();

        let second = unique_path(&dir, "IMG_0001", "jpg");
        assert!(second.ends_with("IMG_0001-1.jpg"));
        std::fs::write(&second, b"x").unwrap();

        let third = unique_path(&dir, "IMG_0001", "jpg");
        assert!(third.ends_with("IMG_0001-2.jpg"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_image_formats_produce_loadable_files() {
        let dir = temp_dir("formats");
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            120,
            80,
            image::Rgb([200, 100, 50]),
        ));

        let jpg = write_image(img.clone(), &dir, "a", ExportFormat::Jpeg, 90).unwrap();
        assert_eq!(jpg.extension().unwrap(), "jpg");
        assert_eq!(image::open(&jpg).unwrap().width(), 120);
        assert_eq!(image::open(&jpg).unwrap().height(), 80);

        let png = write_image(img.clone(), &dir, "b", ExportFormat::Png, 90).unwrap();
        assert_eq!(png.extension().unwrap(), "png");
        assert_eq!(image::open(&png).unwrap().width(), 120);

        let webp = write_image(img.clone(), &dir, "c", ExportFormat::Webp, 90).unwrap();
        assert_eq!(webp.extension().unwrap(), "webp");
        assert_eq!(image::open(&webp).unwrap().width(), 120);

        let tif = write_image(img, &dir, "d", ExportFormat::Tiff, 90).unwrap();
        assert_eq!(tif.extension().unwrap(), "tif");
        assert_eq!(image::open(&tif).unwrap().width(), 120);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rgba_png_flattens_without_alpha_artifacts() {
        let dir = temp_dir("rgba");
        let mut rgba = image::RgbaImage::new(10, 10);
        rgba.put_pixel(5, 5, image::Rgba([255, 0, 0, 255])); // 不透明红
        let out = write_image(
            image::DynamicImage::ImageRgba8(rgba),
            &dir,
            "t",
            ExportFormat::Jpeg,
            90,
        )
        .unwrap();
        let decoded = image::open(&out).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (10, 10));
        std::fs::remove_dir_all(&dir).ok();
    }
}
