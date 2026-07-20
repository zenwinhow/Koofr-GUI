use tauri::{AppHandle, State};

use crate::{
    AppState,
    error::CommandError,
    transfer::{self, ResumableTransfer, TransferResult},
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn list_resumable_transfers(
    state: State<'_, AppState>,
) -> CommandResult<Vec<ResumableTransfer>> {
    let owner_id = transfer::current_owner(&state.api)
        .await
        .map_err(CommandError::from)?;
    state
        .transfer_checkpoints
        .list(&owner_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn resume_transfer(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
) -> CommandResult<TransferResult> {
    let outcome = transfer::resume_checkpoint(
        app,
        &state.api,
        &state.transfers,
        &state.transfer_checkpoints,
        transfer::NetworkRetryPolicy::from(state.settings.network_retry_settings().await),
        transfer_id.clone(),
    )
    .await
    .map_err(CommandError::from)?;
    if let Some(path) = outcome.completed_path {
        state
            .local_access
            .remember_download(&transfer_id, path)
            .map_err(CommandError::from)?;
    }
    let _ = state.cache.clear().await;
    Ok(outcome.result)
}

#[tauri::command]
pub async fn discard_resumable_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> CommandResult<bool> {
    let owner_id = transfer::current_owner(&state.api)
        .await
        .map_err(CommandError::from)?;
    transfer::discard_checkpoint(&state.transfer_checkpoints, &transfer_id, &owner_id)
        .await
        .map_err(CommandError::from)
}
