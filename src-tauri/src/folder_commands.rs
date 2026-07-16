use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    AppState,
    error::{AppError, CommandError},
    file_ops::{MountId, RemoteName, RemotePath},
    folder_download::{self, FolderDownloadContext, FolderDownloadRequest, FolderDownloadTarget},
    local_access::LocalFileSelection,
    transfer::TransferResult,
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn select_download_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    suggested_name: String,
) -> CommandResult<Option<LocalFileSelection>> {
    let name = RemoteName::parse(suggested_name).map_err(CommandError::from)?;
    let Some(parent) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let parent = parent
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    let target = FolderDownloadTarget::from_parent(parent, &name)
        .await
        .map_err(CommandError::from)?;
    state
        .local_access
        .grant_download_directory(target.as_path().to_path_buf())
        .map(Some)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn prepare_download_folder(
    state: State<'_, AppState>,
    suggested_name: String,
    download_directory: String,
) -> CommandResult<LocalFileSelection> {
    let name = RemoteName::parse(suggested_name).map_err(CommandError::from)?;
    let target = FolderDownloadTarget::from_parent(download_directory.into(), &name)
        .await
        .map_err(CommandError::from)?;
    state
        .local_access
        .grant_download_directory(target.as_path().to_path_buf())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn download_folder(
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
        .take_download_directory(&local_path_grant)
        .map_err(CommandError::from)?;
    let completed_path = selected_path.clone();
    let target = FolderDownloadTarget::from_selected(selected_path)
        .await
        .map_err(CommandError::from)?;
    let result = folder_download::download_folder(
        FolderDownloadContext {
            app,
            api: &state.api,
            manager: &state.transfers,
        },
        FolderDownloadRequest {
            transfer_id: transfer_id.clone(),
            mount_id,
            remote_path,
            target,
        },
    )
    .await
    .map_err(CommandError::from)?;
    state
        .local_access
        .remember_download(&transfer_id, completed_path)
        .map_err(CommandError::from)?;
    Ok(result)
}
