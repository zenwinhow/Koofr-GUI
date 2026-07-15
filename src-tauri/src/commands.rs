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
    koofr_api::{FileInfo, Mount, SessionInfo},
    local_access::LocalFileSelection,
    transfer::{self, TransferResult},
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn connect_koofr(
    state: State<'_, AppState>,
    email: String,
    app_password: String,
) -> CommandResult<SessionInfo> {
    state.transfers.cancel_all();
    state.local_access.clear();
    let app_password = Zeroizing::new(app_password);
    state
        .api
        .authenticate(&email, app_password.as_str())
        .await
        .map_err(Into::into)
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
pub async fn koofr_session(state: State<'_, AppState>) -> CommandResult<SessionInfo> {
    Ok(state.api.session_info().await)
}

#[tauri::command]
pub async fn list_mounts(state: State<'_, AppState>) -> CommandResult<Vec<Mount>> {
    state.api.list_mounts().await.map_err(Into::into)
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
) -> CommandResult<Vec<FileInfo>> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    state
        .api
        .list_files(&mount_id, &path)
        .await
        .map_err(Into::into)
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
        .map_err(Into::into)
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
        .map_err(Into::into)
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
    result.map_err(Into::into)
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
    state.api.delete(&mount_id, &path).await.map_err(Into::into)
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
    transfer::upload(
        app,
        &state.api,
        &state.transfers,
        transfer_id,
        mount_id,
        directory,
        local_path,
    )
    .await
    .map_err(Into::into)
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
    remote_path.require_non_root().map_err(CommandError::from)?;
    let selected_path = state
        .local_access
        .take_download(&local_path_grant)
        .map_err(CommandError::from)?;
    let local_path = LocalDownloadPath::from_selected(selected_path)
        .await
        .map_err(CommandError::from)?;
    transfer::download(
        app,
        &state.api,
        &state.transfers,
        transfer_id,
        mount_id,
        remote_path,
        local_path,
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> CommandResult<bool> {
    state
        .transfers
        .cancel(&transfer_id)
        .map_err(CommandError::from)
}
