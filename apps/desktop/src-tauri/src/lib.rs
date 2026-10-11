mod commands;
mod db;
mod indexer;
mod metadata;
mod models;
mod thumbs;

use std::path::PathBuf;
use tauri::Manager;

pub struct AppState {
    pub db_path: PathBuf,
    pub cache_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?.join("thumbs");
            std::fs::create_dir_all(&cache_dir)?;

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("shutter_hub.db");
            db::open_db(&db_path)?;

            app.manage(AppState { db_path, cache_dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_folder,
            commands::rescan_folder,
            commands::list_folders,
            commands::list_photos,
            commands::get_stats,
            commands::remove_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
