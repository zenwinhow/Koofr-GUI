use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::{
    AppState,
    error::CommandError,
    file_ops::{LocalUploadPath, MountId, RemoteName, RemotePath},
    transfer::{self, SplitTransferRuntime, SplitUploadRequest, TransferResult},
};

type CommandResult<T> = Result<T, CommandError>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitUploadCommand {
    transfer_id: String,
    mount_id: String,
    remote_directory: String,
    local_path_grant: String,
    package_name: Option<String>,
    part_bytes: u64,
}

#[tauri::command]
pub async fn upload_split_file(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SplitUploadCommand,
) -> CommandResult<TransferResult> {
    let mount_id = MountId::parse(request.mount_id).map_err(CommandError::from)?;
    let directory = RemotePath::parse(request.remote_directory).map_err(CommandError::from)?;
    let package_name = request
        .package_name
        .map(RemoteName::parse)
        .transpose()
        .map_err(CommandError::from)?;
    let part_bytes =
        transfer::validate_split_part_bytes(request.part_bytes).map_err(CommandError::from)?;
    let selected_path = state
        .local_access
        .take_upload(&request.local_path_grant)
        .map_err(CommandError::from)?;
    let request = SplitUploadRequest {
        transfer_id: request.transfer_id,
        mount_id,
        directory,
        local_path: LocalUploadPath::from_selected(selected_path)
            .await
            .map_err(CommandError::from)?,
        package_name,
        part_bytes,
    };
    let result = transfer::upload_split(
        SplitTransferRuntime {
            app,
            api: &state.api,
            manager: &state.transfers,
            checkpoints: &state.transfer_checkpoints,
        },
        request,
    )
    .await
    .map_err(CommandError::from)?;
    let _ = state.cache.clear().await;
    Ok(result)
}
