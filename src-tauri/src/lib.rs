mod commands;
mod credential_manager;
mod error;
mod file_ops;
mod koofr_api;
mod local_access;
mod local_open;
mod metadata_cache;
mod settings;
mod transfer;

use credential_manager::CredentialManager;
use koofr_api::KoofrApi;
use local_access::LocalAccessManager;
use metadata_cache::MetadataCache;
use settings::SettingsStore;
use tauri::Manager;
use transfer::TransferManager;

pub struct AppState {
    api: KoofrApi,
    local_access: LocalAccessManager,
    transfers: TransferManager,
    settings: SettingsStore,
    cache: MetadataCache,
    credentials: CredentialManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let settings = SettingsStore::load(data_dir.join("settings.json"));
            let cache = MetadataCache::load(
                data_dir.join("metadata-cache.json"),
                settings.initial_cache_mode() == settings::CacheMode::Disk,
            );
            app.manage(AppState {
                api: KoofrApi::production()?,
                local_access: LocalAccessManager::default(),
                transfers: TransferManager::default(),
                settings,
                cache,
                credentials: CredentialManager::initialize()?,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_koofr,
            commands::restore_saved_login,
            commands::disconnect_koofr,
            commands::koofr_session,
            commands::get_settings,
            commands::update_settings,
            commands::clear_metadata_cache,
            commands::forget_saved_login,
            commands::select_upload_file,
            commands::select_download_location,
            commands::list_mounts,
            commands::list_files,
            commands::list_recent,
            commands::list_shared,
            commands::list_trash,
            commands::restore_trash,
            commands::empty_trash,
            commands::create_folder,
            commands::rename_entry,
            commands::move_entry,
            commands::copy_entry,
            commands::delete_entry,
            commands::upload_file,
            commands::download_file,
            commands::open_downloaded_file,
            commands::open_downloaded_folder,
            commands::cancel_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Koofr GUI");
}
