use anyhow::{Context, Result};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// 看图模式的全尺寸预览：与缩略图（480px）不同，预览不下采样，
/// 保证 1:1 检查时像素级可用。预览文件缓存在 `<app_cache>/previews/`，
/// 内容优先复用原图字节（JPEG 直拷、RAW 抽相机内嵌 JPEG 预览），
/// 仅当源文件比缓存新时才重新生成。

pub fn path_for(preview_dir: &Path, photo_id: i64) -> PathBuf {
    preview_dir.join(format!("{photo_id}.jpg"))
}

/// 确保存在最新的全尺寸预览，返回其路径。
pub fn ensure(preview_dir: &Path, photo_id: i64, src: &Path) -> Result<Option<PathBuf>> {
    let out = path_for(preview_dir, photo_id);

    if let (Ok(out_meta), Ok(src_meta)) = (std::fs::metadata(&out), std::fs::metadata(src)) {
        if let (Ok(out_mtime), Ok(src_mtime)) = (out_meta.modified(), src_meta.modified()) {
            if out_mtime >= src_mtime {
                return Ok(Some(out));
            }
        }
    }

    // RAW/TIFF 系：直接抽相机内嵌的全尺寸 JPEG 预览，零解码等待。
    if crate::thumbs::is_embedded_capable(src) {
        if let Some(bytes) = crate::thumbs::extract_embedded_preview(src) {
            std::fs::write(&out, &bytes)
                .with_context(|| format!("write preview {}", out.display()))?;
            return Ok(Some(out));
        }
    }

    // JPEG：字节级拷贝，不重编码、不损失质量。
    if is_plain_jpeg(src) {
        std::fs::copy(src, &out).with_context(|| format!("copy preview {}", out.display()))?;
        return Ok(Some(out));
    }

    // 其余格式（PNG / WebP / TIFF 等）：解码后重编码为 JPEG。
    match image::open(src) {
        Ok(img) => {
            let img = flatten_to_rgb(img);
            let file = std::fs::File::create(&out)
                .with_context(|| format!("create preview {}", out.display()))?;
            let mut writer = BufWriter::new(file);
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 90);
            encoder.encode_image(&img)?;
            Ok(Some(out))
        }
        Err(_) => Ok(None),
    }
}

/// RGBA8（带透明通道）合成到白底，其余色彩类型转为 RGB8；
/// JPEG 编码前置处理，避免编码失败或透明变黑。
pub fn flatten_to_rgb(img: image::DynamicImage) -> image::DynamicImage {
    match img {
        image::DynamicImage::ImageRgba8(buf) => {
            let mut white = image::RgbImage::from_pixel(
                buf.width(),
                buf.height(),
                image::Rgb([255, 255, 255]),
            );
            for (x, y, p) in buf.enumerate_pixels() {
                let a = p[3] as f32 / 255.0;
                let px = white.get_pixel_mut(x, y);
                px[0] = (p[0] as f32 * a + 255.0 * (1.0 - a)) as u8;
                px[1] = (p[1] as f32 * a + 255.0 * (1.0 - a)) as u8;
                px[2] = (p[2] as f32 * a + 255.0 * (1.0 - a)) as u8;
            }
            image::DynamicImage::ImageRgb8(white)
        }
        other => image::DynamicImage::ImageRgb8(other.into_rgb8()),
    }
}

fn is_plain_jpeg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shutter_hub_test_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_jpeg(path: &Path, w: u32, h: u32, seed: u8) {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x as u8).wrapping_add(seed), (y as u8).wrapping_mul(seed), seed])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn plain_jpeg_is_copied_verbatim_and_reused() {
        let dir = temp_dir("preview_jpeg");
        let src = dir.join("photo.jpg");
        make_jpeg(&src, 400, 300, 7);

        let out = ensure(&dir, 1, &src).unwrap().expect("preview generated");
        assert_eq!(out, path_for(&dir, 1));
        assert_eq!(std::fs::read(&out).unwrap(), std::fs::read(&src).unwrap());

        // 源未变化时直接复用缓存，不重新生成
        let out2 = ensure(&dir, 1, &src).unwrap().expect("preview reused");
        assert_eq!(out2, out);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn png_is_reencoded_to_full_size_jpeg() {
        let dir = temp_dir("preview_png");
        let src = dir.join("photo.png");
        image::RgbaImage::from_fn(64, 48, |x, y| image::Rgba([x as u8, y as u8, 128, 255]))
            .save(&src)
            .unwrap();

        let out = ensure(&dir, 2, &src).unwrap().expect("preview generated");
        let decoded = image::open(&out).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 48));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_preview_is_regenerated_when_source_is_newer() {
        let dir = temp_dir("preview_stale");
        let src = dir.join("photo.jpg");
        make_jpeg(&src, 100, 100, 1);
        let out = ensure(&dir, 3, &src).unwrap().expect("preview generated");
        let first = std::fs::read(&out).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));
        make_jpeg(&src, 100, 100, 2);
        ensure(&dir, 3, &src).unwrap().expect("preview regenerated");
        let second = std::fs::read(&out).unwrap();
        assert_ne!(first, second);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsupported_file_yields_none() {
        let dir = temp_dir("preview_none");
        let src = dir.join("not_an_image.txt");
        std::fs::write(&src, b"hello").unwrap();
        assert!(ensure(&dir, 4, &src).unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
