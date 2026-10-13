use chrono::NaiveDateTime;
use exif as exif;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct PhotoMeta {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub iso: Option<i64>,
    pub taken_at: Option<String>,
}

pub fn read_exif(path: &Path) -> Option<exif::Exif> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    exif::Reader::new().read_from_container(&mut reader).ok()
}

pub fn extract(path: &Path) -> PhotoMeta {
    let mut meta = PhotoMeta::default();
    let Some(exif) = read_exif(path) else {
        return meta;
    };

    meta.width = exif_int(&exif, exif::Tag::PixelXDimension);
    meta.height = exif_int(&exif, exif::Tag::PixelYDimension);
    meta.camera_make = exif_text(&exif, exif::Tag::Make);
    meta.camera_model = exif_text(&exif, exif::Tag::Model);
    meta.lens_model = exif_text(&exif, exif::Tag::LensModel);
    meta.focal_length = exif_float(&exif, exif::Tag::FocalLength);
    meta.aperture = exif_float(&exif, exif::Tag::FNumber);
    meta.shutter_speed = exif_shutter(&exif);
    meta.iso = exif_iso(&exif);

    if let Some(v) = exif_text(&exif, exif::Tag::DateTimeOriginal) {
        if let Ok(dt) = NaiveDateTime::parse_from_str(v.trim(), "%Y:%m:%d %H:%M:%S") {
            meta.taken_at = Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
    }

    meta
}

fn exif_text(exif: &exif::Exif, tag: exif::Tag) -> Option<String> {
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| clean_exif_text(f.display_value().to_string()))
        .filter(|s| !s.is_empty())
}

/// display_value() 会给 Ascii 字段加引号（如 `"SONY"`），去掉引号与空白。
fn clean_exif_text(s: String) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_quotes_from_exif_text() {
        assert_eq!(clean_exif_text("\"SONY\"".into()), "SONY");
        assert_eq!(clean_exif_text("\"ILCE-6700\"".into()), "ILCE-6700");
        assert_eq!(clean_exif_text("  Canon  ".into()), "Canon");
        assert_eq!(
            clean_exif_text("\"NIKON CORPORATION\"".into()),
            "NIKON CORPORATION"
        );
    }
}

fn exif_int(exif: &exif::Exif, tag: exif::Tag) -> Option<i64> {
    let field = exif.get_field(tag, exif::In::PRIMARY)?;
    match &field.value {
        exif::Value::Short(v) => v.first().map(|&x| x as i64),
        exif::Value::Long(v) => v.first().map(|&x| x as i64),
        exif::Value::Ascii(v) => v
            .first()
            .and_then(|b| String::from_utf8_lossy(b).trim().parse().ok()),
        _ => None,
    }
}

fn exif_float(exif: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    let field = exif.get_field(tag, exif::In::PRIMARY)?;
    match &field.value {
        exif::Value::Rational(v) => v.first().map(|r| r.num as f64 / r.denom as f64),
        exif::Value::SRational(v) => v.first().map(|r| r.num as f64 / r.denom as f64),
        exif::Value::Short(v) => v.first().map(|&x| x as f64),
        exif::Value::Long(v) => v.first().map(|&x| x as f64),
        exif::Value::Ascii(v) => v
            .first()
            .and_then(|b| String::from_utf8_lossy(b).trim().parse().ok()),
        _ => None,
    }
}

fn exif_iso(exif: &exif::Exif) -> Option<i64> {
    exif_int(exif, exif::Tag::PhotographicSensitivity)
        .or_else(|| exif_int(exif, exif::Tag::ISOSpeed))
}

fn exif_shutter(exif: &exif::Exif) -> Option<String> {
    let field = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)?;
    let exif::Value::Rational(v) = &field.value else {
        return None;
    };
    let r = v.first()?;
    if r.num == 0 {
        return None;
    }
    let secs = r.num as f64 / r.denom as f64;
    if secs >= 1.0 {
        Some(format!("{secs:.1}s"))
    } else {
        Some(format!("1/{}", (r.denom as f64 / r.num as f64).round()))
    }
}
