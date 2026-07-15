mod commands;
mod error;
mod file_ops;
mod koofr_api;
mod local_access;
mod transfer;

use koofr_api::KoofrApi;
use local_access::LocalAccessManager;
use transfer::TransferManager;

pub struct AppState {
    api: KoofrApi,
    local_access: LocalAccessManager,
    transfers: TransferManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let api = KoofrApi::production().expect("failed to initialize the Koofr API client");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            api,
            local_access: LocalAccessManager::default(),
            transfers: TransferManager::default(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_koofr,
            commands::disconnect_koofr,
            commands::koofr_session,
            commands::select_upload_file,
            commands::select_download_location,
            commands::list_mounts,
            commands::list_files,
            commands::create_folder,
            commands::rename_entry,
            commands::move_entry,
            commands::copy_entry,
            commands::delete_entry,
            commands::upload_file,
            commands::download_file,
            commands::cancel_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Koofr GUI");
}
