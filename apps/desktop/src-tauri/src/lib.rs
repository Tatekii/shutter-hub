pub mod commands;
pub mod db;
pub mod indexer;
pub mod metadata;
pub mod models;
pub mod preview;
pub mod thumbs;
pub mod volumes;
pub mod xmp;

use std::path::PathBuf;
use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub cache_dir: PathBuf,
    pub preview_dir: PathBuf,
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

            app.manage(AppState { db_path, cache_dir, preview_dir });
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
