//! 端到端后端集成测试：建库 → 扫描 → 评分/XMP → 查询 → 去重。

use desktop_lib::commands::{
    add_photos_to_album_inner, create_album_inner, list_dupes_inner, list_similar_inner,
};
use desktop_lib::db;
use desktop_lib::indexer;
use desktop_lib::models::{PhotoQuery, ScanStats};
use desktop_lib::xmp;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 全局互斥：WAL 数据库与临时目录在测试间共享会互相干扰。
static LOCK: Mutex<()> = Mutex::new(());

struct TestCtx {
    dir: PathBuf,
    db: PathBuf,
    cache: PathBuf,
    previews: PathBuf,
}

fn setup(tag: &str) -> TestCtx {
    let base = std::env::temp_dir().join(format!("shutter_hub_it_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let photos_dir = base.join("photos");
    std::fs::create_dir_all(&photos_dir).unwrap();
    let cache = base.join("thumbs");
    std::fs::create_dir_all(&cache).unwrap();
    let previews = base.join("previews");
    std::fs::create_dir_all(&previews).unwrap();
    let db_path = base.join("test.db");
    TestCtx {
        dir: photos_dir,
        db: db_path,
        cache,
        previews,
    }
}

fn make_jpeg(path: &Path, w: u32, h: u32, seed: u8) {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x as u8).wrapping_add(seed), (y as u8).wrapping_mul(seed), seed])
    });
    img.save(path).unwrap();
}

fn scan(ctx: &TestCtx) -> ScanStats {
    let conn = db::open_db(&ctx.db).unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO folders (path) VALUES (?1)",
        [ctx.dir.to_string_lossy().into_owned()],
    )
    .unwrap();
    let folder_id: i64 = conn
        .query_row("SELECT id FROM folders WHERE path = ?1", [ctx.dir.to_string_lossy().into_owned()], |r| r.get(0))
        .unwrap();

    indexer::scan_folder(folder_id, &ctx.dir, &ctx.db, &ctx.cache, |_| {}).unwrap()
}

fn query_photos(ctx: &TestCtx, q: PhotoQuery) -> Vec<desktop_lib::models::PhotoDto> {
    let conn = db::open_db(&ctx.db).unwrap();
    let (sql, params) = build_query_sql(q);
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    desktop_lib::commands::query_photos(&conn, &sql, &refs, &ctx.cache).unwrap()
}

/// 与 library.rs 中 build_photo_query 等价的查询构建（测试侧实现）。
fn build_query_sql(q: PhotoQuery) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = String::from(
        "SELECT id, folder_id, path, filename, size, width, height, camera_make, camera_model,
         lens_model, focal_length, aperture, shutter_speed, iso, taken_at, has_thumb,
         rating, color_label, flag, media_type, duration_ms FROM photos",
    );
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(qs) = &q.q {
        clauses.push("id IN (SELECT rowid FROM photos_fts WHERE photos_fts MATCH ?)".to_string());
        params.push(Box::new(qs.clone()));
    }
    if let Some(v) = q.min_rating {
        clauses.push("rating >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = &q.flag {
        clauses.push("flag = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &q.media_type {
        clauses.push("media_type = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = q.album_id {
        clauses.push("id IN (SELECT photo_id FROM album_photos WHERE album_id = ?)".to_string());
        params.push(Box::new(v));
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY filename ASC");
    (sql, params)
}

fn setup_state(ctx: &TestCtx) -> desktop_lib::AppState {
    desktop_lib::AppState {
        db_path: ctx.db.clone(),
        cache_dir: ctx.cache.clone(),
        preview_dir: ctx.previews.clone(),
        model_dir: ctx.dir.parent().unwrap().join("models"),
        ai_busy: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
}

#[test]
fn scan_index_fts_and_rating_flow() {
    let _guard = LOCK.lock().unwrap();
    let ctx = setup("scan_flow");

    make_jpeg(&ctx.dir.join("IMG_0001.jpg"), 640, 480, 10);
    make_jpeg(&ctx.dir.join("IMG_0002.jpg"), 640, 480, 20);
    let dup_target = ctx.dir.join("IMG_0003.jpg");
    std::fs::copy(&ctx.dir.join("IMG_0001.jpg"), &dup_target).unwrap();

    let stats = scan(&ctx);
    assert_eq!(stats.added, 3, "should index 3 photos");

    // 重复文件内容一致
    let a = std::fs::read(ctx.dir.join("IMG_0001.jpg")).unwrap();
    let b = std::fs::read(&dup_target).unwrap();
    assert_eq!(a, b);

    // FTS 搜索文件名
    let hits = query_photos(
        &ctx,
        PhotoQuery {
            q: Some("\"IMG_0003\"*".into()),
            ..Default::default()
        },
    );
    assert_eq!(hits.len(), 1);
    assert!(hits[0].filename.contains("IMG_0003"));

    // 评分 + XMP sidecar 回写
    let id = hits[0].id;
    let state = setup_state(&ctx);
    desktop_lib::commands::set_rating_inner(&state, id, 4).unwrap();

    let sidecar_meta = xmp::read(&dup_target).unwrap();
    assert_eq!(sidecar_meta.rating, Some(4), "rating written to XMP sidecar");

    // 重写文件以更新 mtime，触发重新索引并读回 XMP 评分
    touch(&dup_target);
    let stats2 = scan(&ctx);
    assert_eq!(stats2.added, 0, "no new photos on rescan");
    assert_eq!(stats2.updated + stats2.skipped, 3);

    let by_rating = query_photos(
        &ctx,
        PhotoQuery {
            min_rating: Some(4),
            ..Default::default()
        },
    );
    assert_eq!(by_rating.len(), 1);
    assert_eq!(by_rating[0].rating, 4);

    // 旗标
    desktop_lib::commands::set_flag_inner(&state, id, "pick".into()).unwrap();
    let picks = query_photos(
        &ctx,
        PhotoQuery {
            flag: Some("pick".into()),
            ..Default::default()
        },
    );
    assert_eq!(picks.len(), 1);

    // 相册
    let album = create_album_inner(&state, "测试相册".into()).unwrap();
    let photo_ids = query_photos(&ctx, PhotoQuery::default()).iter().map(|p| p.id).collect();
    add_photos_to_album_inner(&state, album.id, photo_ids).unwrap();
    let in_album = query_photos(
        &ctx,
        PhotoQuery {
            album_id: Some(album.id),
            ..Default::default()
        },
    );
    assert_eq!(in_album.len(), 3);

    std::fs::remove_dir_all(ctx.dir.parent().unwrap()).ok();
}

#[test]
fn duplicate_detection_by_content_hash() {
    let _guard = LOCK.lock().unwrap();
    let ctx = setup("dupes");

    make_jpeg(&ctx.dir.join("A.jpg"), 320, 240, 42);
    std::fs::copy(ctx.dir.join("A.jpg"), ctx.dir.join("B.jpg")).unwrap();
    make_jpeg(&ctx.dir.join("C.jpg"), 320, 240, 43);

    scan(&ctx);

    let conn = db::open_db(&ctx.db).unwrap();
    // 模拟 compute_hashes 的核心逻辑
    let pending: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, path FROM photos WHERE content_hash IS NULL")
            .unwrap();
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        rows.collect::<Result<Vec<_>, _>>().unwrap()
    };
    assert_eq!(pending.len(), 3);
    for (id, path) in &pending {
        let mut hasher = blake3::Hasher::new();
        let mut f = std::fs::File::open(path).unwrap();
        std::io::copy(&mut f, &mut hasher).unwrap();
        conn.execute(
            "UPDATE photos SET content_hash = ?1 WHERE id = ?2",
            rusqlite::params![hasher.finalize().to_hex().to_string(), id],
        )
        .unwrap();
    }

    let state = setup_state(&ctx);
    let groups = list_dupes_inner(&state).unwrap();
    assert_eq!(groups.len(), 1, "exactly one duplicate group");
    assert_eq!(groups[0].photos.len(), 2);

    // 相似检测（A/C 内容接近，dHash 应命中）
    let similar = list_similar_inner(&state).unwrap();
    assert!(
        !similar.is_empty(),
        "A and C should be detected as similar"
    );

    std::fs::remove_dir_all(ctx.dir.parent().unwrap()).ok();
}

#[test]
fn video_files_are_indexed_without_ffmpeg() {
    let _guard = LOCK.lock().unwrap();
    let ctx = setup("video");

    // 无效容器内容：无 ffmpeg 环境下仍应完成索引（仅无缩略图/时长）
    std::fs::write(ctx.dir.join("CLIP_0001.mp4"), b"not a real video").unwrap();
    make_jpeg(&ctx.dir.join("IMG_0001.jpg"), 320, 240, 5);

    let stats = scan(&ctx);
    assert_eq!(stats.added, 2, "both video and photo indexed");

    let conn = db::open_db(&ctx.db).unwrap();
    let (media_type, has_thumb): (String, i64) = conn
        .query_row(
            "SELECT media_type, has_thumb FROM photos WHERE filename = 'CLIP_0001.mp4'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(media_type, "video");
    assert_eq!(has_thumb, 0, "undecodable video → no thumbnail");

    // 按媒体类型过滤
    let videos = query_photos(
        &ctx,
        PhotoQuery {
            media_type: Some("video".into()),
            ..Default::default()
        },
    );
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0].media_type, "video");

    std::fs::remove_dir_all(ctx.dir.parent().unwrap()).ok();
}

fn touch(path: &Path) {
    let bytes = std::fs::read(path).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn viewer_preview_is_full_size_and_reused() {
    let _guard = LOCK.lock().unwrap();
    let ctx = setup("viewer");

    make_jpeg(&ctx.dir.join("photo.jpg"), 800, 600, 9);
    scan(&ctx);

    let conn = db::open_db(&ctx.db).unwrap();
    let (id, path): (i64, String) = conn
        .query_row("SELECT id, path FROM photos", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();

    let state = setup_state(&ctx);
    let out = desktop_lib::commands::get_viewer_preview_inner(&state, id)
        .unwrap()
        .expect("viewer preview generated");
    let img = image::open(&out).unwrap();
    assert_eq!((img.width(), img.height()), (800, 600), "full-size, no downscale");

    // 字节级拷贝：预览应与原 JPEG 完全一致
    assert_eq!(std::fs::read(&out).unwrap(), std::fs::read(&path).unwrap());

    // 再次请求应复用缓存
    let out2 = desktop_lib::commands::get_viewer_preview_inner(&state, id)
        .unwrap()
        .expect("preview reused");
    assert_eq!(out, out2);

    std::fs::remove_dir_all(ctx.dir.parent().unwrap()).ok();
}
