pub mod ai;
pub mod commands;
pub mod db;
pub mod indexer;
pub mod metadata;
pub mod models;
pub mod preview;
pub mod thumbs;
pub mod video;
pub mod volumes;
pub mod xmp;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub cache_dir: PathBuf,
    pub preview_dir: PathBuf,
    pub model_dir: PathBuf,
    pub ai_busy: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?.join("thumbs");
            std::fs::create_dir_all(&cache_dir)?;
            let preview_dir = app.path().app_cache_dir()?.join("previews");
            std::fs::create_dir_all(&preview_dir)?;

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("shutter_hub.db");
            db::open_db(&db_path)?;

            // 本地 AI 模型目录（首次使用时按需下载）
            let model_dir = data_dir.join("models");
            std::fs::create_dir_all(&model_dir)?;

            app.manage(AppState {
                db_path,
                cache_dir,
                preview_dir,
                model_dir,
                ai_busy: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_folder,
            commands::rescan_folder,
            commands::list_folders,
            commands::list_photos,
            commands::get_photo,
            commands::get_viewer_preview,
            commands::get_stats,
            commands::remove_folder,
            commands::set_rating,
            commands::set_color_label,
            commands::set_flag,
            commands::list_facets,
            commands::list_albums,
            commands::create_album,
            commands::delete_album,
            commands::add_photos_to_album,
            commands::remove_photos_from_album,
            commands::list_tags,
            commands::add_tags,
            commands::remove_tag_from_photos,
            commands::delete_tag,
            commands::compute_hashes,
            commands::list_dupes,
            commands::list_similar,
            volumes::list_volumes,
            volumes::browse_volume,
            volumes::ingest_files,
            volumes::eject_volume,
            commands::ai_status,
            commands::ai_download_models,
            commands::ai_compute_embeddings,
            commands::ai_detect_faces,
            commands::ai_search,
            commands::list_faces,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
