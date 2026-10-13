//! RAW 解码（rawler，纯 Rust）：传感器数据 → 去马赛克 → 拍摄时白平衡
//! → 相机色彩矩阵 → sRGB 伽马。全程本地，无外部依赖。
//!
//! 说明：查看走缩略图/内嵌预览（零解码等待）；带调色的预览与导出走本模块
//! 的真实 RAW 显影（全分辨率），保证编辑建立在传感器数据之上。

use anyhow::{Context, Result};
use rawler::decoders::RawDecodeParams;
use rawler::imgop::develop::RawDevelop;
use rawler::rawsource::RawSource;
use std::path::Path;

/// rawler 可解码的 RAW 扩展名（与 thumbs 的内嵌预览列表区分）。
pub const RAW_EXTENSIONS: &[&str] = &[
    "nef", "cr2", "cr3", "arw", "raf", "dng", "orf", "rw2", "pef", "srw", "x3f",
];

pub fn is_raw(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| RAW_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// 完整显影（全分辨率）。耗时与像素数成正比，请放 spawn_blocking 调用。
pub fn develop(path: &Path) -> Result<image::DynamicImage> {
    let rawfile = RawSource::new(path)
        .with_context(|| format!("open raw {}", path.display()))?;
    let decoder = rawler::get_decoder(&rawfile)
        .with_context(|| format!("no raw decoder for {}", path.display()))?;
    let raw_params = RawDecodeParams { image_index: 0 };
    let rawimage = decoder
        .raw_image(&rawfile, &raw_params, false)
        .with_context(|| format!("decode raw {}", path.display()))?;
    let intermediate = RawDevelop::default()
        .develop_intermediate(&rawimage)
        .with_context(|| format!("develop raw {}", path.display()))?;
    intermediate
        .to_dynamic_image()
        .ok_or_else(|| anyhow::anyhow!("develop produced no image for {}", path.display()))
}

/// 显影并按长边缩放（看图预览用）。
pub fn develop_resized(path: &Path, max_long: Option<u32>) -> Result<image::DynamicImage> {
    let img = develop(path)?;
    Ok(match max_long {
        Some(max) if img.width().max(img.height()) > max => {
            img.resize(max, max, image::imageops::FilterType::Lanczos3)
        }
        _ => img,
    })
}

/// 调色预览的 RAW 基底缓存：`<id>_raw_<max>.jpg`，源文件更新时重渲染。
/// 取决于参数哈希之外，调色参数在基底之上应用，避免每次滑杆都重解码。
pub fn ensure_base(
    preview_dir: &Path,
    photo_id: i64,
    src: &Path,
    max_long: u32,
) -> Result<std::path::PathBuf> {
    let out = preview_dir.join(format!("{photo_id}_raw_{max_long}.jpg"));

    let fresh = match (
        std::fs::metadata(&out).and_then(|m| m.modified()),
        std::fs::metadata(src).and_then(|m| m.modified()),
    ) {
        (Ok(o), Ok(s)) => o >= s,
        _ => false,
    };
    if fresh {
        return Ok(out);
    }

    let img = develop_resized(src, Some(max_long))?;
    let rgb = crate::preview::flatten_to_rgb(img);
    let file = std::fs::File::create(&out)
        .with_context(|| format!("create {}", out.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 95);
    encoder.encode_image(&rgb)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_extension_detection() {
        assert!(is_raw(Path::new("/a/IMG_1.NEF")));
        assert!(is_raw(Path::new("/a/IMG_1.cr3")));
        assert!(!is_raw(Path::new("/a/IMG_1.jpg")));
        assert!(!is_raw(Path::new("/a/IMG_1.mp4")));
    }

    /// 真实 RAW 冒烟测试：需要本机样张，设置 SHUTTER_HUB_TEST_RAW 指向文件。
    /// `SHUTTER_HUB_TEST_RAW=/path/to/photo.cr2 cargo test -- --ignored raw_smoke`
    #[test]
    #[ignore = "requires a real RAW sample via SHUTTER_HUB_TEST_RAW"]
    fn raw_smoke_develop() {
        let Ok(path) = std::env::var("SHUTTER_HUB_TEST_RAW") else {
            eprintln!("SHUTTER_HUB_TEST_RAW not set, skipping");
            return;
        };
        let path = Path::new(&path);
        assert!(is_raw(path));
        let img = develop(path).expect("develop raw");
        assert!(img.width() > 100 && img.height() > 100, "dims: {}x{}", img.width(), img.height());

        // 缓存基底可重用
        let dir = std::env::temp_dir().join(format!("shutter_hub_raw_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let base = ensure_base(&dir, 1, path, 1024).expect("base");
        assert!(base.exists());
        let small = image::open(&base).unwrap();
        assert!(small.width().max(small.height()) <= 1024);

        // 重新调用走缓存（mtime 判定）
        let base2 = ensure_base(&dir, 1, path, 1024).expect("base cached");
        assert_eq!(base, base2);
        std::fs::remove_dir_all(&dir).ok();
    }
}
