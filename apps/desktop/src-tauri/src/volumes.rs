use crate::metadata;
use crate::models::{CardPhotoDto, IngestProgress, IngestRequest, IngestResult, VolumeDto};
use anyhow::Context;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::commands::to_str;

const CARD_IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "heic", "heif",
    "nef", "cr2", "cr3", "arw", "raf", "dng", "orf", "rw2", "pef", "srw", "x3f",
];

/// 列出可移动卷（SD/TF 卡、U 盘等）。
#[tauri::command]
pub fn list_volumes() -> Result<Vec<VolumeDto>, String> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut volumes = Vec::new();
    for disk in disks.list() {
        if !disk.is_removable() {
            continue;
        }
        let mount = disk.mount_point();
        if mount.as_os_str().is_empty() {
            continue;
        }
        volumes.push(VolumeDto {
            name: disk.name().to_string_lossy().into_owned(),
            path: mount.to_string_lossy().into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        });
    }
    volumes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(volumes)
}

/// 浏览可移动卷中的照片（不入库，缩略图存临时区）。
#[tauri::command]
pub fn browse_volume(state: State<'_, AppState>, path: String) -> Result<Vec<CardPhotoDto>, String> {
    let root = PathBuf::from(path);
    let card_cache = state.cache_dir.join("card");
    std::fs::create_dir_all(&card_cache).map_err(to_str)?;

    let mut photos = Vec::new();
    for entry in walkdir::WalkDir::new(&root).follow_links(false) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let file_path = entry.path();
        let Some(ext) = file_path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !CARD_IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            continue;
        }

        let Ok(meta) = std::fs::metadata(file_path) else {
            continue;
        };
        let photo = metadata::extract(file_path);

        let thumb_path = card_thumb(file_path, &card_cache);

        photos.push(CardPhotoDto {
            path: file_path.to_string_lossy().into_owned(),
            filename: file_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size: meta.len() as i64,
            width: photo.width,
            height: photo.height,
            camera_model: photo.camera_model,
            taken_at: photo.taken_at,
            thumb_path: thumb_path.map(|p| p.to_string_lossy().into_owned()),
        });
    }
    photos.sort_by(|a, b| {
        a.taken_at
            .cmp(&b.taken_at)
            .then_with(|| a.filename.cmp(&b.filename))
    });
    Ok(photos)
}

/// 卡上文件的缩略图：按路径哈希命名，幂等生成。
fn card_thumb(file_path: &Path, card_cache: &Path) -> Option<PathBuf> {
    let key = blake3::hash(file_path.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    let out = card_cache.join(format!("{key}.jpg"));
    if out.exists() {
        return Some(out);
    }

    // 优先抽相机内嵌预览（RAW 零解码）
    if let Some(bytes) = crate::thumbs::extract_embedded_preview(file_path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let img = img.thumbnail(crate::thumbs::THUMB_SIZE, crate::thumbs::THUMB_SIZE);
            if img.save(&out).is_ok() {
                return Some(out);
            }
        }
    }

    if let Ok(img) = image::open(file_path) {
        let img = img.thumbnail(crate::thumbs::THUMB_SIZE, crate::thumbs::THUMB_SIZE);
        if img.save(&out).is_ok() {
            return Some(out);
        }
    }
    None
}

/// 从卡导入到本地库：blake3 校验拷贝，可选按日期归档。
#[tauri::command]
pub async fn ingest_files(
    app: AppHandle,
    req: IngestRequest,
) -> Result<IngestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let total = req.files.len() as u64;
        let mut result = IngestResult {
            copied: 0,
            failed: Vec::new(),
            total_bytes: 0,
        };

        std::fs::create_dir_all(&req.target).map_err(|e| format!("{e:#}"))?;

        for (i, file) in req.files.iter().enumerate() {
            let current = Path::new(file)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let _ = app.emit(
                "ingest://progress",
                IngestProgress {
                    processed: (i + 1) as u64,
                    total,
                    current: current.clone(),
                },
            );

            match copy_verified(Path::new(file), &req) {
                Ok((dest, bytes)) => {
                    result.copied += 1;
                    result.total_bytes += bytes;
                    let _ = dest;
                }
                Err(e) => {
                    result.failed.push(format!("{current}: {e:#}"));
                }
            }
        }

        let _ = app.emit("ingest://done", &result);
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn copy_verified(src: &Path, req: &IngestRequest) -> anyhow::Result<(PathBuf, u64)> {
    let dest_dir = destination_dir(src, req)?;
    std::fs::create_dir_all(&dest_dir)?;

    let filename = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .context("invalid source filename")?;
    let dest = unique_dest(&dest_dir, &filename);

    let mut hasher = blake3::Hasher::new();
    let mut input = std::fs::File::open(src)?;
    let mut output = std::fs::File::create(&dest)?;
    let mut buf = vec![0u8; 1024 * 1024];
    let mut written = 0u64;
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        output.write_all(&buf[..n])?;
        written += n as u64;
    }
    output.flush()?;
    drop(output);
    drop(input);

    let source_hash = hasher.finalize();

    // 校验目标文件与源一致
    let mut verify = blake3::Hasher::new();
    let mut check = std::fs::File::open(&dest)?;
    loop {
        let n = check.read(&mut buf)?;
        if n == 0 {
            break;
        }
        verify.update(&buf[..n]);
    }
    if verify.finalize() != source_hash {
        std::fs::remove_file(&dest).ok();
        anyhow::bail!("校验失败：目标文件哈希与源不一致");
    }

    Ok((dest, written))
}

fn destination_dir(src: &Path, req: &IngestRequest) -> anyhow::Result<PathBuf> {
    let target = PathBuf::from(&req.target);
    if !req.organize_by_date {
        return Ok(target);
    }
    let date = capture_date(src);
    match date {
        Some((y, m, d)) => Ok(target
            .join(format!("{y}"))
            .join(format!("{y}-{m:02}-{d:02}"))),
        None => Ok(target.join("未分类")),
    }
}

fn capture_date(src: &Path) -> Option<(i32, u32, u32)> {
    if let Some(meta) = metadata::extract(src).taken_at {
        let date = meta.split('T').next().unwrap_or("");
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() == 3 {
            return Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ));
        }
    }
    let modified = std::fs::metadata(src).ok()?.modified().ok()?;
    let days = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs()
        / 86400;
    let (y, m, d) = civil_from_days(days);
    Some((y, m, d))
}

fn civil_from_days(days: u64) -> (i32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

fn unique_dest(dir: &Path, filename: &str) -> PathBuf {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = Path::new(filename)
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut candidate = dir.join(filename);
    let mut n = 1;
    while candidate.exists() {
        let name = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        candidate = dir.join(name);
        n += 1;
    }
    candidate
}

/// 安全弹出可移动卷（macOS/Linux 卸载；Windows 请手动弹出）。
#[tauri::command]
pub fn eject_volume(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("diskutil")
            .args(["unmount", &path])
            .status()
            .map_err(to_str)?;
        if status.success() {
            Ok(())
        } else {
            Err("卸载失败，请检查是否有程序正在使用该设备".to_string())
        }
    }
    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("umount")
            .arg(&path)
            .status()
            .map_err(to_str)?;
        if status.success() {
            Ok(())
        } else {
            Err("卸载失败，请检查是否有程序正在使用该设备".to_string())
        }
    }
    #[cfg(target_os = "windows")]
    {
        let _ = path;
        Err("Windows 暂不支持应用内弹出，请在文件资源管理器中安全弹出该设备".to_string())
    }
}
