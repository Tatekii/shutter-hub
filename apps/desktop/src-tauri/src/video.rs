use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// 视频容器格式。缩略图/时长探测依赖系统 ffmpeg / ffprobe，
/// 未安装时优雅降级：仍可索引、去重（blake3），只是没有缩略图和时长。
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "avi", "wmv", "webm", "mts", "m2ts", "3gp", "mpg", "mpeg",
];

pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VideoInfo {
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_ms: Option<i64>,
    /// 容器 creation_time（ISO 8601）转换后的拍摄时间。
    pub taken_at: Option<String>,
}

fn bin_available(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn ffmpeg_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| bin_available("ffmpeg"))
}

pub fn ffprobe_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| bin_available("ffprobe"))
}

/// 用 ffprobe 读取视频信息；ffprobe 不可用或解析失败时返回 None。
pub fn probe(path: &Path) -> Option<VideoInfo> {
    if !ffprobe_available() {
        return None;
    }
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_probe_json(&String::from_utf8_lossy(&out.stdout))
}

/// 从 ffprobe json 提取宽高 / 时长 / creation_time（纯函数，便于测试）。
pub fn parse_probe_json(json: &str) -> Option<VideoInfo> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut info = VideoInfo::default();

    if let Some(streams) = v.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            if stream.get("codec_type").and_then(|t| t.as_str()) == Some("video") {
                info.width = stream.get("width").and_then(|w| w.as_i64());
                info.height = stream.get("height").and_then(|h| h.as_i64());
                break;
            }
        }
    }

    if let Some(dur) = v
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
    {
        if dur.is_finite() && dur >= 0.0 {
            info.duration_ms = Some((dur * 1000.0).round() as i64);
        }
    }

    if let Some(ct) = v
        .get("format")
        .and_then(|f| f.get("tags"))
        .and_then(|t| t.get("creation_time"))
        .and_then(|c| c.as_str())
    {
        info.taken_at = parse_creation_time(ct);
    }

    Some(info)
}

/// "2023-05-01T12:34:56.000000Z" → "2023-05-01T12:34:56"（与 EXIF 时间格式一致）。
fn parse_creation_time(raw: &str) -> Option<String> {
    let s: String = raw.chars().take(19).collect();
    if s.len() == 19 && s.as_bytes().get(10) == Some(&b'T') {
        Some(s)
    } else {
        None
    }
}

/// 用 ffmpeg 将视频转为 GIF（12fps / 480px 宽，调色板优化）。返回是否成功。
pub fn make_gif(src: &Path, out: &Path) -> bool {
    if !ffmpeg_available() {
        return false;
    }
    let filter = "fps=12,scale=480:-2:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse";
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(src)
        .args(["-vf", filter, "-loop", "0", "-y"])
        .arg(out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if matches!(status, Ok(s) if s.success()) && out.exists() {
        true
    } else {
        let _ = std::fs::remove_file(out);
        false
    }
}

/// 用 ffmpeg 抽取一帧作为缩略图。返回是否成功。
pub fn make_thumb(src: &Path, out: &PathBuf, max_dim: u32) -> bool {
    if !ffmpeg_available() {
        return false;
    }
    let filter = format!("scale={max_dim}:-2");
    // 先尝试 0.5s 处抽帧（避开黑场），失败再退回首帧。
    for seek in ["0.5", "0"] {
        let status = Command::new("ffmpeg")
            .args(["-v", "error", "-ss", seek, "-i"])
            .arg(src)
            .args(["-frames:v", "1", "-vf", &filter, "-q:v", "3", "-y"])
            .arg(out)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if matches!(status, Ok(s) if s.success()) && out.exists() {
            return true;
        }
    }
    let _ = std::fs::remove_file(out);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffprobe_json() {
        let json = r#"{
            "streams": [
                {"codec_type": "audio", "width": 0},
                {"codec_type": "video", "width": 1920, "height": 1080}
            ],
            "format": {
                "duration": "12.345000",
                "tags": {"creation_time": "2023-05-01T12:34:56.000000Z"}
            }
        }"#;
        let info = parse_probe_json(json).unwrap();
        assert_eq!(info.width, Some(1920));
        assert_eq!(info.height, Some(1080));
        assert_eq!(info.duration_ms, Some(12345));
        assert_eq!(info.taken_at.as_deref(), Some("2023-05-01T12:34:56"));
    }

    #[test]
    fn parses_ffprobe_json_without_video_stream() {
        let json = r#"{"streams": [], "format": {"duration": "1.0"}}"#;
        let info = parse_probe_json(json).unwrap();
        assert_eq!(info.width, None);
        assert_eq!(info.duration_ms, Some(1000));
        assert_eq!(info.taken_at, None);
    }

    #[test]
    fn video_extension_detection() {
        assert!(is_video(Path::new("/a/b/clip.MP4")));
        assert!(is_video(Path::new("/a/b/clip.mov")));
        assert!(!is_video(Path::new("/a/b/photo.jpg")));
        assert!(!is_video(Path::new("/a/b/noext")));
    }

    #[test]
    fn rejects_garbage_json() {
        assert!(parse_probe_json("not json").is_none());
    }
}
