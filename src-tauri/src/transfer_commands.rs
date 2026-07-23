use std::collections::BTreeMap;

use tauri::{AppHandle, State};

use crate::{
    AppState,
    download_history::DownloadHistoryItem,
    error::CommandError,
    transfer::{self, ResumableTransfer, TransferResult},
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn list_download_history(
    state: State<'_, AppState>,
) -> CommandResult<Vec<DownloadHistoryItem>> {
    let owner_id = transfer::current_owner(&state.api)
        .await
        .map_err(CommandError::from)?;
    let resumable = state
        .transfer_checkpoints
        .list(&owner_id)
        .await
        .map_err(CommandError::from)?
        .into_iter()
        .map(|item| (item.transfer_id, item.recovery_kind))
        .collect::<BTreeMap<_, _>>();
    state
        .download_history
        .reconcile_interrupted(&owner_id, &resumable)
        .map_err(CommandError::from)?;
    state
        .download_history
        .list(&owner_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn clear_finished_download_history(state: State<'_, AppState>) -> CommandResult<usize> {
    let owner_id = transfer::current_owner(&state.api)
        .await
        .map_err(CommandError::from)?;
    state
        .download_history
        .clear_finished(&owner_id)
        .map_err(CommandError::from)
}

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
        &state.vault,
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
    let removed =
        transfer::discard_checkpoint(&state.transfer_checkpoints, &transfer_id, &owner_id)
            .await
            .map_err(CommandError::from)?;
    if removed {
        state
            .download_history
            .remove(&owner_id, &transfer_id)
            .map_err(CommandError::from)?;
    }
    Ok(removed)
}
