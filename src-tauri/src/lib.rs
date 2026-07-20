mod commands;
mod credential_manager;
mod error;
mod file_ops;
mod folder_commands;
mod folder_download;
mod koofr_api;
mod link_commands;
mod local_access;
mod local_open;
mod logging;
mod metadata_cache;
mod settings;
mod split_commands;
mod transfer;
mod transfer_commands;

use credential_manager::CredentialManager;
use koofr_api::KoofrApi;
use local_access::LocalAccessManager;
use logging::{AppLogger, LogConfig};
use metadata_cache::MetadataCache;
use settings::{SettingsDefaults, SettingsStore};
use tauri::Manager;
use transfer::{TransferCheckpointStore, TransferManager};

pub struct AppState {
    api: KoofrApi,
    local_access: LocalAccessManager,
    transfers: TransferManager,
    transfer_checkpoints: TransferCheckpointStore,
    settings: SettingsStore,
    cache: MetadataCache,
    credentials: CredentialManager,
    logger: AppLogger,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let defaults = SettingsDefaults {
                download_directory: app.path().download_dir()?,
                cache_directory: data_dir.join("cache"),
                log_directory: data_dir.join("logs"),
            };
            std::fs::create_dir_all(&defaults.cache_directory)?;
            std::fs::create_dir_all(&defaults.log_directory)?;
            let settings = SettingsStore::load(data_dir.join("settings.json"), defaults);
            let (log_directory, log_level, retention_days, max_file_size_mb) =
                settings.initial_log_config().clone();
            std::fs::create_dir_all(&log_directory)?;
            let logger = AppLogger::initialize(LogConfig {
                directory: log_directory,
                level: log_level,
                retention_days,
                max_file_bytes: u64::from(max_file_size_mb) * 1024 * 1024,
            })?;
            let cache = MetadataCache::load(
                settings
                    .initial_cache_directory()
                    .join("metadata-cache.json"),
                settings.initial_cache_mode() == settings::CacheMode::Disk,
            );
            app.manage(AppState {
                api: KoofrApi::production()?,
                local_access: LocalAccessManager::default(),
                transfers: TransferManager::default(),
                transfer_checkpoints: TransferCheckpointStore::load(
                    data_dir.join("transfer-checkpoints.json"),
                ),
                settings,
                cache,
                credentials: CredentialManager::initialize()?,
                logger,
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
            commands::update_download_settings,
            commands::update_logging_settings,
            commands::clear_metadata_cache,
            commands::clear_logs,
            commands::forget_saved_login,
            commands::select_upload_file,
            commands::select_download_location,
            commands::select_download_directory,
            commands::select_settings_directory,
            commands::prepare_download_location,
            folder_commands::select_download_folder,
            folder_commands::prepare_download_folder,
            commands::list_mounts,
            commands::list_files,
            commands::list_recent,
            commands::list_shared,
            commands::list_trash,
            link_commands::list_public_links,
            link_commands::create_public_link,
            link_commands::delete_public_link,
            commands::restore_trash,
            commands::empty_trash,
            commands::create_folder,
            commands::rename_entry,
            commands::move_entry,
            commands::copy_entry,
            commands::delete_entry,
            commands::upload_file,
            split_commands::upload_split_file,
            commands::download_file,
            folder_commands::download_folder,
            commands::open_downloaded_file,
            commands::open_downloaded_folder,
            commands::cancel_transfer,
            commands::pause_transfer,
            transfer_commands::list_resumable_transfers,
            transfer_commands::resume_transfer,
            transfer_commands::discard_resumable_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Koofr GUI");
}
