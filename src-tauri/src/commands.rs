use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

use crate::{
    AppState,
    error::{AppError, CommandError},
    file_ops::{
        LocalDownloadPath, LocalUploadPath, MountId, RemoteName, RemotePath,
        safe_suggested_file_name,
    },
    koofr_api::{FileInfo, LocatedFile, Mount, SessionInfo, TrashList},
    local_access::LocalFileSelection,
    local_open,
    logging::LogConfig,
    settings::{CacheMode, LogLevel},
    transfer::{self, TransferResult},
};

type CommandResult<T> = Result<T, CommandError>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashRestoreTarget {
    mount_id: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginBootstrap {
    session: SessionInfo,
    saved_email: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    cache_mode: CacheMode,
    cache_ttl_minutes: u32,
    cached_items: usize,
    cache_disk_bytes: u64,
    saved_email: Option<String>,
    download_directory: String,
    ask_download_location: bool,
    cache_directory: String,
    log_directory: String,
    log_level: LogLevel,
    log_retention_days: u32,
    log_max_file_size_mb: u32,
    log_files: usize,
    log_disk_bytes: u64,
}

#[tauri::command]
pub async fn connect_koofr(
    state: State<'_, AppState>,
    email: String,
    app_password: String,
    remember_password: bool,
) -> CommandResult<SessionInfo> {
    state.transfers.cancel_all();
    state.local_access.clear();
    let app_password = Zeroizing::new(app_password);
    let session = state
        .api
        .authenticate(&email, app_password.as_str())
        .await
        .map_err(CommandError::from)?;
    let previous_email = state.settings.remembered_email().await;
    if remember_password {
        if let Some(previous) = previous_email.filter(|previous| previous != &email) {
            state
                .credentials
                .delete(previous)
                .await
                .map_err(CommandError::from)?;
        }
        state
            .credentials
            .save(email.clone(), app_password)
            .await
            .map_err(CommandError::from)?;
        state
            .settings
            .set_remembered_email(Some(email.clone()))
            .await
            .map_err(CommandError::from)?;
    } else if let Some(previous) = previous_email {
        state
            .credentials
            .delete(previous)
            .await
            .map_err(CommandError::from)?;
        state
            .settings
            .set_remembered_email(None)
            .await
            .map_err(CommandError::from)?;
    }
    let (mode, _) = state.settings.cache_policy().await;
    let _ = state
        .cache
        .select_account(
            session
                .user_id
                .clone()
                .unwrap_or_else(|| email.to_ascii_lowercase()),
            mode,
        )
        .await;
    Ok(session)
}

#[tauri::command]
pub async fn restore_saved_login(state: State<'_, AppState>) -> CommandResult<LoginBootstrap> {
    if state.api.session_info().await.authenticated {
        return Ok(LoginBootstrap {
            session: state.api.session_info().await,
            saved_email: state.settings.remembered_email().await,
        });
    }
    let Some(email) = state.settings.remembered_email().await else {
        return Ok(LoginBootstrap {
            session: state.api.session_info().await,
            saved_email: None,
        });
    };
    let Some(password) = state
        .credentials
        .load(email.clone())
        .await
        .map_err(CommandError::from)?
    else {
        state
            .settings
            .set_remembered_email(None)
            .await
            .map_err(CommandError::from)?;
        return Ok(LoginBootstrap {
            session: state.api.session_info().await,
            saved_email: None,
        });
    };
    let session = state
        .api
        .authenticate(&email, password.as_str())
        .await
        .map_err(CommandError::from)?;
    let (mode, _) = state.settings.cache_policy().await;
    let _ = state
        .cache
        .select_account(
            session
                .user_id
                .clone()
                .unwrap_or_else(|| email.to_ascii_lowercase()),
            mode,
        )
        .await;
    Ok(LoginBootstrap {
        session,
        saved_email: Some(email),
    })
}

#[tauri::command]
pub async fn disconnect_koofr(state: State<'_, AppState>) -> CommandResult<()> {
    state.transfers.cancel_all();
    state.api.disconnect().await;
    state.local_access.clear();
    Ok(())
}

#[tauri::command]
pub async fn select_upload_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<LocalFileSelection>> {
    let Some(file_path) = app.dialog().file().blocking_pick_file() else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    LocalUploadPath::from_selected(path.clone())
        .await
        .map_err(CommandError::from)?;
    state
        .local_access
        .grant_upload(path)
        .map(Some)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn select_download_location(
    app: AppHandle,
    state: State<'_, AppState>,
    suggested_name: String,
) -> CommandResult<Option<LocalFileSelection>> {
    let suggested_name = RemoteName::parse(suggested_name).map_err(CommandError::from)?;
    let Some(file_path) = app
        .dialog()
        .file()
        .set_file_name(safe_suggested_file_name(&suggested_name))
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    LocalDownloadPath::from_selected(path.clone())
        .await
        .map_err(CommandError::from)?;
    state
        .local_access
        .grant_download(path)
        .map(Some)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn select_download_directory(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    let (initial_directory, _) = state.settings.download_policy().await;
    let Some(directory) = app
        .dialog()
        .file()
        .set_directory(initial_directory)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = directory
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    let metadata = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(|error| CommandError::from(AppError::from(error)))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(CommandError::from(AppError::InvalidInput(
            "download directory",
        )));
    }
    path.to_str()
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| CommandError::from(AppError::InvalidInput("download directory")))
}

#[tauri::command]
pub async fn select_settings_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    directory_kind: String,
) -> CommandResult<Option<String>> {
    let initial_directory = match directory_kind.as_str() {
        "cache" => state.settings.cache_directory().await,
        "logs" => state.settings.log_policy().await.0,
        _ => return Err(CommandError::from(AppError::InvalidInput("directory kind"))),
    };
    let Some(directory) = app
        .dialog()
        .file()
        .set_directory(initial_directory)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = directory
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    let metadata = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(|error| CommandError::from(AppError::from(error)))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(CommandError::from(AppError::InvalidInput(
            "settings directory",
        )));
    }
    path.to_str()
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| CommandError::from(AppError::InvalidInput("settings directory")))
}

#[tauri::command]
pub async fn prepare_download_location(
    state: State<'_, AppState>,
    suggested_name: String,
    download_directory: String,
) -> CommandResult<LocalFileSelection> {
    let name = RemoteName::parse(suggested_name).map_err(CommandError::from)?;
    let target = LocalDownloadPath::from_parent(PathBuf::from(download_directory), &name)
        .await
        .map_err(CommandError::from)?;
    state
        .local_access
        .grant_download(target.as_path().to_path_buf())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn koofr_session(state: State<'_, AppState>) -> CommandResult<SessionInfo> {
    Ok(state.api.session_info().await)
}

async fn settings_snapshot(state: &State<'_, AppState>) -> SettingsSnapshot {
    let (cache_mode, cache_ttl_minutes) = state.settings.cache_policy().await;
    let (download_directory, ask_download_location) = state.settings.download_policy().await;
    let cache_directory = state.settings.cache_directory().await;
    let (log_directory, log_level, log_retention_days, log_max_file_size_mb) =
        state.settings.log_policy().await;
    let (cached_items, cache_disk_bytes) = state.cache.stats().await;
    let (log_files, log_disk_bytes) = state.logger.stats().await;
    SettingsSnapshot {
        cache_mode,
        cache_ttl_minutes,
        cached_items,
        cache_disk_bytes,
        saved_email: state.settings.remembered_email().await,
        download_directory: download_directory.to_string_lossy().into_owned(),
        ask_download_location,
        cache_directory: cache_directory.to_string_lossy().into_owned(),
        log_directory: log_directory.to_string_lossy().into_owned(),
        log_level,
        log_retention_days,
        log_max_file_size_mb,
        log_files,
        log_disk_bytes,
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CommandResult<SettingsSnapshot> {
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    cache_mode: CacheMode,
    cache_ttl_minutes: u32,
    cache_directory: String,
) -> CommandResult<SettingsSnapshot> {
    let cache_directory = PathBuf::from(cache_directory.trim());
    state
        .settings
        .update_cache(cache_mode, cache_ttl_minutes, cache_directory.clone())
        .await
        .map_err(CommandError::from)?;
    state
        .cache
        .relocate(cache_directory.join("metadata-cache.json"), cache_mode)
        .await
        .map_err(CommandError::from)?;
    state.logger.info(
        "settings",
        "cache_settings_updated",
        None,
        serde_json::Map::new(),
    );
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn update_logging_settings(
    state: State<'_, AppState>,
    log_directory: String,
    log_level: LogLevel,
    log_retention_days: u32,
    log_max_file_size_mb: u32,
) -> CommandResult<SettingsSnapshot> {
    let log_directory = PathBuf::from(log_directory.trim());
    state
        .settings
        .update_logging(
            log_directory.clone(),
            log_level,
            log_retention_days,
            log_max_file_size_mb,
        )
        .await
        .map_err(CommandError::from)?;
    state
        .logger
        .configure(LogConfig {
            directory: log_directory,
            level: log_level,
            retention_days: log_retention_days,
            max_file_bytes: u64::from(log_max_file_size_mb) * 1024 * 1024,
        })
        .await
        .map_err(CommandError::from)?;
    state.logger.info(
        "settings",
        "logging_settings_updated",
        None,
        serde_json::Map::new(),
    );
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn update_download_settings(
    state: State<'_, AppState>,
    download_directory: String,
    ask_download_location: bool,
) -> CommandResult<SettingsSnapshot> {
    state
        .settings
        .update_download(
            PathBuf::from(download_directory.trim()),
            ask_download_location,
        )
        .await
        .map_err(CommandError::from)?;
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn clear_metadata_cache(state: State<'_, AppState>) -> CommandResult<SettingsSnapshot> {
    state.cache.clear().await.map_err(CommandError::from)?;
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn clear_logs(state: State<'_, AppState>) -> CommandResult<SettingsSnapshot> {
    state.logger.clear().await.map_err(CommandError::from)?;
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn forget_saved_login(state: State<'_, AppState>) -> CommandResult<SettingsSnapshot> {
    if let Some(email) = state.settings.remembered_email().await {
        state
            .credentials
            .delete(email)
            .await
            .map_err(CommandError::from)?;
    }
    state
        .settings
        .set_remembered_email(None)
        .await
        .map_err(CommandError::from)?;
    Ok(settings_snapshot(&state).await)
}

#[tauri::command]
pub async fn list_mounts(state: State<'_, AppState>, refresh: bool) -> CommandResult<Vec<Mount>> {
    let (mode, ttl) = state.settings.cache_policy().await;
    if !refresh && let Some(cached) = state.cache.get("mounts", mode, ttl).await {
        return Ok(cached);
    }
    let mounts = state.api.list_mounts().await.map_err(CommandError::from)?;
    let _ = state.cache.put("mounts".to_owned(), &mounts, mode).await;
    Ok(mounts)
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
    refresh: bool,
) -> CommandResult<Vec<FileInfo>> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    let cache_key = format!("files:{}:{}", mount_id.as_str(), path.as_str());
    let (mode, ttl) = state.settings.cache_policy().await;
    if !refresh && let Some(cached) = state.cache.get(&cache_key, mode, ttl).await {
        return Ok(cached);
    }
    let files = state
        .api
        .list_files(&mount_id, &path)
        .await
        .map_err(CommandError::from)?;
    let _ = state.cache.put(cache_key, &files, mode).await;
    Ok(files)
}

#[tauri::command]
pub async fn list_recent(
    state: State<'_, AppState>,
    refresh: bool,
) -> CommandResult<Vec<LocatedFile>> {
    let (mode, ttl) = state.settings.cache_policy().await;
    if !refresh && let Some(cached) = state.cache.get("recent", mode, ttl).await {
        return Ok(cached);
    }
    let files = state.api.list_recent().await.map_err(CommandError::from)?;
    let _ = state.cache.put("recent".to_owned(), &files, mode).await;
    Ok(files)
}

#[tauri::command]
pub async fn list_shared(
    state: State<'_, AppState>,
    refresh: bool,
) -> CommandResult<Vec<LocatedFile>> {
    let (mode, ttl) = state.settings.cache_policy().await;
    if !refresh && let Some(cached) = state.cache.get("shared", mode, ttl).await {
        return Ok(cached);
    }
    let files = state.api.list_shared().await.map_err(CommandError::from)?;
    let _ = state.cache.put("shared".to_owned(), &files, mode).await;
    Ok(files)
}

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>, refresh: bool) -> CommandResult<TrashList> {
    let (mode, ttl) = state.settings.cache_policy().await;
    if !refresh && let Some(cached) = state.cache.get("trash", mode, ttl).await {
        return Ok(cached);
    }
    let trash = state.api.list_trash().await.map_err(CommandError::from)?;
    let _ = state.cache.put("trash".to_owned(), &trash, mode).await;
    Ok(trash)
}

#[tauri::command]
pub async fn restore_trash(
    state: State<'_, AppState>,
    files: Vec<TrashRestoreTarget>,
) -> CommandResult<()> {
    let files = files
        .into_iter()
        .map(|file| {
            let mount_id = MountId::parse(file.mount_id)?;
            let path = RemotePath::parse(file.path)?;
            path.require_non_root()?;
            Ok((mount_id, path))
        })
        .collect::<Result<Vec<_>, AppError>>()
        .map_err(CommandError::from)?;
    state
        .api
        .restore_trash(&files)
        .await
        .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>, confirmation: String) -> CommandResult<()> {
    if confirmation != "永久删除" {
        return Err(CommandError::from(AppError::InvalidInput("confirmation")));
    }
    state.api.empty_trash().await.map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    mount_id: String,
    parent_path: String,
    name: String,
) -> CommandResult<()> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let parent = RemotePath::parse(parent_path).map_err(CommandError::from)?;
    let name = RemoteName::parse(name).map_err(CommandError::from)?;
    state
        .api
        .create_folder(&mount_id, &parent, &name)
        .await
        .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn rename_entry(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
    new_name: String,
) -> CommandResult<()> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    path.require_non_root().map_err(CommandError::from)?;
    let destination = path
        .parent()
        .and_then(|parent| parent.join(&RemoteName::parse(new_name)?))
        .map_err(CommandError::from)?;
    state
        .api
        .move_to(&mount_id, &path, &mount_id, &destination)
        .await
        .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn move_entry(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
    destination_mount_id: String,
    destination_directory: String,
) -> CommandResult<()> {
    relocate_entry(
        &state,
        true,
        mount_id,
        path,
        destination_mount_id,
        destination_directory,
    )
    .await
}

#[tauri::command]
pub async fn copy_entry(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
    destination_mount_id: String,
    destination_directory: String,
) -> CommandResult<()> {
    relocate_entry(
        &state,
        false,
        mount_id,
        path,
        destination_mount_id,
        destination_directory,
    )
    .await
}

async fn relocate_entry(
    state: &State<'_, AppState>,
    is_move: bool,
    mount_id: String,
    path: String,
    destination_mount_id: String,
    destination_directory: String,
) -> CommandResult<()> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    path.require_non_root().map_err(CommandError::from)?;
    let destination_mount_id = MountId::parse(destination_mount_id).map_err(CommandError::from)?;
    let destination_directory =
        RemotePath::parse(destination_directory).map_err(CommandError::from)?;
    let name = RemoteName::parse(path.file_name()?.to_owned()).map_err(CommandError::from)?;
    let destination = destination_directory
        .join(&name)
        .map_err(CommandError::from)?;
    let result = if is_move {
        state
            .api
            .move_to(&mount_id, &path, &destination_mount_id, &destination)
            .await
    } else {
        state
            .api
            .copy_to(&mount_id, &path, &destination_mount_id, &destination)
            .await
    };
    result.map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn delete_entry(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
) -> CommandResult<()> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    path.require_non_root().map_err(CommandError::from)?;
    state
        .api
        .delete(&mount_id, &path)
        .await
        .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn upload_file(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
    mount_id: String,
    remote_directory: String,
    local_path_grant: String,
) -> CommandResult<TransferResult> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let directory = RemotePath::parse(remote_directory).map_err(CommandError::from)?;
    let selected_path = state
        .local_access
        .take_upload(&local_path_grant)
        .map_err(CommandError::from)?;
    let local_path = LocalUploadPath::from_selected(selected_path)
        .await
        .map_err(CommandError::from)?;
    let result = transfer::upload(
        app,
        &state.api,
        &state.transfers,
        &state.transfer_checkpoints,
        transfer_id,
        mount_id,
        directory,
        local_path,
    )
    .await
    .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(result)
}

#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
    mount_id: String,
    remote_path: String,
    local_path_grant: String,
) -> CommandResult<TransferResult> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let remote_path = RemotePath::parse(remote_path).map_err(CommandError::from)?;
    let selected_path = state
        .local_access
        .take_download(&local_path_grant)
        .map_err(CommandError::from)?;
    let completed_path = selected_path.clone();
    let local_path = LocalDownloadPath::from_selected(selected_path)
        .await
        .map_err(CommandError::from)?;
    let result = transfer::download(
        app,
        &state.api,
        &state.transfers,
        &state.transfer_checkpoints,
        transfer_id.clone(),
        mount_id,
        remote_path,
        local_path,
    )
    .await
    .map_err(CommandError::from)?;
    state
        .local_access
        .remember_download(&transfer_id, completed_path)
        .map_err(CommandError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn open_downloaded_file(
    state: State<'_, AppState>,
    transfer_id: String,
) -> CommandResult<()> {
    let path = state
        .local_access
        .completed_download(&transfer_id)
        .map_err(CommandError::from)?;
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| CommandError::from(AppError::LocalOpen))?;
    if !metadata.is_file() {
        return Err(CommandError::from(AppError::LocalOpen));
    }
    local_open::open(&path).map_err(CommandError::from)
}

#[tauri::command]
pub async fn open_downloaded_folder(
    state: State<'_, AppState>,
    transfer_id: String,
) -> CommandResult<()> {
    let path = state
        .local_access
        .completed_download(&transfer_id)
        .map_err(CommandError::from)?;
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::from(AppError::LocalOpen))?;
    let metadata = tokio::fs::metadata(parent)
        .await
        .map_err(|_| CommandError::from(AppError::LocalOpen))?;
    if !metadata.is_dir() {
        return Err(CommandError::from(AppError::LocalOpen));
    }
    local_open::open(parent).map_err(CommandError::from)
}

#[tauri::command]
pub fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> CommandResult<bool> {
    state
        .transfers
        .cancel(&transfer_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn pause_transfer(state: State<'_, AppState>, transfer_id: String) -> CommandResult<bool> {
    state
        .transfers
        .pause(&transfer_id)
        .map_err(CommandError::from)
}
use std::path::PathBuf;
