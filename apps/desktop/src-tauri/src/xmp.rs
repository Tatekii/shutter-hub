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
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
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
