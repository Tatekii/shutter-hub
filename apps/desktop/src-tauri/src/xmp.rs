use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use std::path::Path;

pub const COLOR_LABELS: &[&str] = &["", "red", "yellow", "green", "blue", "purple"];

#[derive(Debug, Default, Clone)]
pub struct SidecarMeta {
    pub rating: Option<i64>,
    pub label: Option<String>,
    pub flag: Option<String>,
    /// crs 调色字段（Lightroom 互通）
    pub develop: Option<crate::develop::Adjustments>,
}

pub fn sidecar_path_for(photo_path: &Path) -> std::path::PathBuf {
    let mut p = photo_path.to_path_buf();
    let name = photo_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    p.set_file_name(format!("{name}.xmp"));
    p
}

/// 读取 XMP sidecar 中的评分/色标/旗标（兼容 Lightroom 的 xmp:Rating / xmp:Label）。
pub fn read(photo_path: &Path) -> Option<SidecarMeta> {
    let sidecar = sidecar_path_for(photo_path);
    let text = std::fs::read_to_string(&sidecar).ok()?;

    let mut meta = SidecarMeta::default();
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(true);

    let mut crs = crate::develop::Adjustments::default();
    let mut crs_seen = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                "Rating" => {
                    let v = read_text_of(&mut reader, &mut buf);
                    meta.rating = v.and_then(|s| s.trim().parse().ok());
                }
                "Label" => {
                    meta.label = read_text_of(&mut reader, &mut buf);
                }
                "Flag" => {
                    meta.flag = read_text_of(&mut reader, &mut buf);
                }
                name @ ("Exposure2012"
                | "Contrast2012"
                | "Highlights2012"
                | "Shadows2012"
                | "Temperature"
                | "Tint"
                | "Saturation") => {
                    let name = name.to_string();
                    let value = read_text_of(&mut reader, &mut buf)
                        .and_then(|s| s.trim().parse::<f32>().ok());
                    if let Some(v) = value {
                        crs_seen = true;
                        match name.as_str() {
                            "Exposure2012" => crs.exposure = v,
                            "Contrast2012" => crs.contrast = v,
                            "Highlights2012" => crs.highlights = v,
                            "Shadows2012" => crs.shadows = v,
                            "Temperature" => crs.temperature = v,
                            "Tint" => crs.tint = v,
                            "Saturation" => crs.saturation = v,
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    if crs_seen {
        meta.develop = Some(crs);
    }
    Some(meta)
}

fn read_text_of(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Option<String> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Text(t)) => {
                let s = t.xml_content(quick_xml::XmlVersion::Implicit1_0);
                return Some(s.into_owned());
            }
            Ok(Event::End(_)) | Ok(Event::Empty(_)) => return None,
            Err(_) => return None,
            _ => {}
        }
    }
}

/// 将评分写入 sidecar（xmp 命名空间，Lightroom 兼容）。
pub fn write_rating(photo_path: &Path, rating: i64) -> Result<()> {
    replace_or_insert(photo_path, "xmp", "Rating", &rating.to_string())
}

pub fn write_label(photo_path: &Path, label: &str) -> Result<()> {
    replace_or_insert(photo_path, "xmp", "Label", label)
}

pub fn write_flag(photo_path: &Path, flag: &str) -> Result<()> {
    replace_or_insert(photo_path, "shutterhub", "Flag", flag)
}

/// 将调色参数写入 Lightroom 兼容的 crs 字段（identity 时全部移除）。
pub fn write_develop(photo_path: &Path, adj: &crate::develop::Adjustments) -> Result<()> {
    let fields: [(&str, f32); 7] = [
        ("Exposure2012", adj.exposure),
        ("Contrast2012", adj.contrast),
        ("Highlights2012", adj.highlights),
        ("Shadows2012", adj.shadows),
        ("Temperature", adj.temperature),
        ("Tint", adj.tint),
        ("Saturation", adj.saturation),
    ];
    for (tag, value) in fields {
        let text = if adj.is_identity() {
            String::new()
        } else {
            format_float(value)
        };
        replace_or_insert(photo_path, "crs", tag, &text)?;
    }
    Ok(())
}

/// 紧凑数字格式：去掉多余小数位。
fn format_float(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// 确保命名空间声明存在（crs 等），插到第一个 rdf:Description 上。
fn ensure_namespace(text: &str, ns: &str) -> String {
    let url = match ns {
        "crs" => "http://ns.adobe.com/camera-raw-settings/1.0/",
        _ => return text.to_string(),
    };
    if text.contains(&format!("xmlns:{ns}=")) {
        return text.to_string();
    }
    let Some(start) = text.find("<rdf:Description") else {
        return text.to_string();
    };
    let Some(end_rel) = text[start..].find('>') else {
        return text.to_string();
    };
    let end = start + end_rel;
    let insert_at = if text[..end].ends_with('/') { end - 1 } else { end };
    let mut out = String::with_capacity(text.len() + url.len() + ns.len() + 10);
    out.push_str(&text[..insert_at]);
    out.push_str(&format!(" xmlns:{ns}=\"{url}\""));
    out.push_str(&text[insert_at..]);
    out
}

/// 字符串级替换/插入，保留 Lightroom 等工具写入的其他 XMP 内容。
/// value 为空时删除该字段。
fn replace_or_insert(photo_path: &Path, ns: &str, tag: &str, value: &str) -> Result<()> {
    let sidecar = sidecar_path_for(photo_path);

    let existing = std::fs::read_to_string(&sidecar).ok();

    let (text, created) = if let Some(text) = existing {
        (text, false)
    } else if value.is_empty() {
        return Ok(());
    } else {
        (TEMPLATE.replace("{CONTENT}", ""), true)
    };
    let text = ensure_namespace(&text, ns);

    let pattern = Regex::new(&format!(r#"<{ns}:{tag}[^>]*>[\s\S]*?</{ns}:{tag}>"#))
        .with_context(|| format!("invalid xmp regex for {ns}:{tag}"))?;

    let mut replaced = false;
    let replacement = if value.is_empty() {
        String::new()
    } else {
        format!("<{ns}:{tag}>{value}</{ns}:{tag}>")
    };
    let new_text = pattern
        .replace_all(&text, |_: &regex::Captures| {
            replaced = true;
            replacement.clone()
        })
        .into_owned();

    let new_text = if replaced || value.is_empty() {
        new_text
    } else {
        let insert = format!("\n   <{ns}:{tag}>{value}</{ns}:{tag}>");
        insert_before_description_end(&new_text, &insert)
    };

    if created || new_text != text {
        std::fs::write(&sidecar, new_text)
            .with_context(|| format!("write xmp sidecar {}", sidecar.display()))?;
    }
    Ok(())
}

fn insert_before_description_end(text: &str, insert: &str) -> String {
    const MARK: &str = "</rdf:Description>";
    if let Some(pos) = text.rfind(MARK) {
        let mut out = String::with_capacity(text.len() + insert.len());
        out.push_str(&text[..pos]);
        out.push_str(insert);
        out.push_str(&text[pos..]);
        out
    } else {
        const RDF_END: &str = "</rdf:RDF>";
        if let Some(pos) = text.rfind(RDF_END) {
            let desc = format!(
                " <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:shutterhub=\"http://shutter-hub.local/ns/\">{insert}\n  </rdf:Description>\n "
            );
            let mut out = String::with_capacity(text.len() + desc.len());
            out.push_str(&text[..pos]);
            out.push_str(&desc);
            out.push_str(&text[pos..]);
            out
        } else {
            text.to_string()
        }
    }
}

const TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Shutter Hub">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:shutterhub="http://shutter-hub.local/ns/">{CONTENT}
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shutter_hub_xmp_{tag}_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_rating() {
        let dir = tmp("roundtrip");
        let photo = dir.join("IMG_001.nef");
        std::fs::write(&photo, b"x").unwrap();

        assert!(read(&photo).is_none());
        write_rating(&photo, 4).unwrap();
        let meta = read(&photo).unwrap();
        assert_eq!(meta.rating, Some(4));

        write_rating(&photo, 1).unwrap();
        let meta = read(&photo).unwrap();
        assert_eq!(meta.rating, Some(1));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn preserves_foreign_content() {
        let dir = tmp("preserve");
        let photo = dir.join("IMG_002.nef");
        std::fs::write(&photo, b"x").unwrap();
        let sidecar = sidecar_path_for(&photo);

        let lr = r#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
   <xmp:Rating>2</xmp:Rating>
   <xmp:CreatorTool>Adobe Lightroom Classic</xmp:CreatorTool>
   <lr:hierarchicalSubject xmlns:lr="http://ns.adobe.com/lightroom/1.0/"><rdf:Bag><rdf:li>foo</rdf:li></rdf:Bag></lr:hierarchicalSubject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#;
        std::fs::write(&sidecar, lr).unwrap();

        write_label(&photo, "green").unwrap();
        let text = std::fs::read_to_string(&sidecar).unwrap();
        assert!(text.contains("Adobe Lightroom Classic"));
        assert!(text.contains("hierarchicalSubject"));
        assert!(text.contains("<xmp:Label>green</xmp:Label>"));
        assert_eq!(read(&photo).unwrap().rating, Some(2));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn develop_roundtrip_crs() {
        let dir = tmp("develop");
        let photo = dir.join("IMG_010.nef");
        std::fs::write(&photo, b"x").unwrap();

        let adj = crate::develop::Adjustments {
            exposure: 0.75,
            contrast: -20.0,
            temperature: 15.5,
            ..Default::default()
        };
        write_develop(&photo, &adj).unwrap();

        let text = std::fs::read_to_string(sidecar_path_for(&photo)).unwrap();
        assert!(text.contains("xmlns:crs="), "namespace declared: {text}");
        assert!(text.contains("<crs:Exposure2012>0.75</crs:Exposure2012>"));

        let meta = read(&photo).unwrap();
        let develop = meta.develop.expect("crs fields parsed");
        assert!((develop.exposure - 0.75).abs() < 1e-6);
        assert!((develop.contrast - -20.0).abs() < 1e-6);
        assert!((develop.temperature - 15.5).abs() < 1e-6);

        // identity → 移除全部 crs 字段
        write_develop(&photo, &crate::develop::Adjustments::default()).unwrap();
        let text = std::fs::read_to_string(sidecar_path_for(&photo)).unwrap();
        assert!(!text.contains("Exposure2012"), "cleared: {text}");
        assert!(read(&photo).unwrap().develop.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn multiline_tag_replacement() {
        let dir = tmp("multiline");
        let photo = dir.join("IMG_003.nef");
        std::fs::write(&photo, b"x").unwrap();
        let sidecar = sidecar_path_for(&photo);
        std::fs::write(
            &sidecar,
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
   <xmp:Rating>
   3
   </xmp:Rating>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#,
        )
        .unwrap();

        write_rating(&photo, 5).unwrap();
        let meta = read(&photo).unwrap();
        assert_eq!(meta.rating, Some(5));
        std::fs::remove_dir_all(&dir).ok();
    }
}
