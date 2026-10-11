use anyhow::{Context, Result};
use exif as exif;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub const THUMB_SIZE: u32 = 480;

/// 可从容器中提取内嵌预览的格式（TIFF 系 + 带预览的 RAW）。
const EMBEDDED_PREVIEW_EXTENSIONS: &[&str] = &[
    "tif", "tiff", "psd", "nef", "cr2", "cr3", "arw", "raf", "dng", "orf", "rw2", "pef",
    "srw", "x3f",
];

pub fn thumb_path_for(cache_dir: &Path, photo_id: i64) -> PathBuf {
    cache_dir.join(format!("{photo_id}.jpg"))
}

/// 生成缩略图：优先抽相机内嵌 JPEG 预览（RAW 零解码），失败则回退到
/// 完整解码 + 缩放。返回写入的缩略图路径。
pub fn generate(photo_id: i64, src: &Path, cache_dir: &Path) -> Result<Option<PathBuf>> {
    let out = thumb_path_for(cache_dir, photo_id);

    if is_embedded_capable(src) {
        if let Some(bytes) = extract_embedded_preview(src) {
            if scale_and_save(&bytes, &out).is_ok() {
                return Ok(Some(out));
            }
        }
    }

    match image::open(src) {
        Ok(img) => {
            let img = img.thumbnail(THUMB_SIZE, THUMB_SIZE);
            img.save(&out)?;
            Ok(Some(out))
        }
        Err(_) => Ok(None),
    }
}

fn is_embedded_capable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EMBEDDED_PREVIEW_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// 从 RAW/TIFF 容器中提取相机内嵌的 JPEG 预览图。
///
/// 通过 JPEGInterchangeFormat(0x0201)/JPEGInterchangeFormatLength(0x0202)
/// 字段定位预览数据，再按 TIFF 基址切片。
pub fn extract_embedded_preview(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let tiff_base = tiff_base_offset(&bytes)?;

    let mut cursor = Cursor::new(&bytes);
    let exif = exif::Reader::new().read_from_container(&mut cursor).ok()?;

    let offset_field = exif.get_field(exif::Tag::JPEGInterchangeFormat, exif::In::THUMBNAIL)?;
    let length_field =
        exif.get_field(exif::Tag::JPEGInterchangeFormatLength, exif::In::THUMBNAIL)?;

    let offset = match &offset_field.value {
        exif::Value::Long(v) => *v.first()? as usize,
        _ => return None,
    };
    let length = match &length_field.value {
        exif::Value::Long(v) => *v.first()? as usize,
        _ => return None,
    };

    let start = tiff_base.checked_add(offset)?;
    let end = start.checked_add(length)?;
    if length > 1024 && end <= bytes.len() {
        Some(bytes[start..end].to_vec())
    } else {
        None
    }
}

/// 计算 TIFF 结构的基址：TIFF 系文件为 0，JPEG 为 "Exif\0\0" 之后。
fn tiff_base_offset(bytes: &[u8]) -> Option<usize> {
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return Some(0);
    }
    bytes
        .windows(6)
        .position(|w| w == b"Exif\0\0")
        .map(|p| p + 6)
}

fn scale_and_save(bytes: &[u8], out: &Path) -> Result<()> {
    let img = image::load_from_memory(bytes).context("decode embedded preview")?;
    let img = img.thumbnail(THUMB_SIZE, THUMB_SIZE);
    img.save(out)?;
    Ok(())
}

/// 对已有缩略图计算感知哈希（dHash：9x8 灰度梯度 → 64 bit）。
pub fn phash_of_thumb(thumb: &Path) -> Option<i64> {
    let img = image::open(thumb).ok()?;
    let gray = img.to_luma8();
    let small = image::imageops::resize(&gray, 9, 8, image::imageops::FilterType::Triangle);
    let mut bits = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let left = small.get_pixel(x, y)[0] as i32;
            let right = small.get_pixel(x + 1, y)[0] as i32;
            if left > right {
                bits |= 1 << (y * 8 + x);
            }
        }
    }
    Some(bits as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shutter_hub_test_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn generate_thumb_for_plain_jpeg() {
        let dir = temp_dir("thumb");
        let src = dir.join("test.jpg");
        let img =
            image::RgbImage::from_fn(800, 600, |x, y| image::Rgb([x as u8, y as u8, 128]));
        img.save(&src).unwrap();

        let out = generate(1, &src, &dir).unwrap().expect("thumb generated");
        let thumb = image::open(&out).unwrap();
        assert!(thumb.width() <= THUMB_SIZE && thumb.height() <= THUMB_SIZE);
        assert_eq!(out, thumb_path_for(&dir, 1));

        let meta = crate::metadata::extract(&src);
        assert!(meta.taken_at.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsupported_file_yields_none() {
        let dir = temp_dir("none");
        let src = dir.join("not_an_image.txt");
        std::fs::write(&src, b"hello").unwrap();
        assert!(generate(2, &src, &dir).unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
