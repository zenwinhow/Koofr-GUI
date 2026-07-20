use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::UNIX_EPOCH,
};

use futures_util::TryStreamExt;
use tauri::AppHandle;
use tokio_util::{io::ReaderStream, sync::CancellationToken};

use crate::{
    error::AppError,
    file_ops::{LocalUploadPath, MountId, RemoteName, RemotePath},
    koofr_api::KoofrApi,
};

use super::{
    checkpoint::{TransferCheckpoint, TransferCheckpointStore, UploadCheckpoint},
    manager::TransferManager,
    model::{
        TransferDirection, TransferResult, TransferState, emit_progress, emit_terminal,
        normalize_interruption,
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn upload(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    transfer_id: String,
    mount_id: MountId,
    directory: RemotePath,
    local_path: LocalUploadPath,
) -> Result<TransferResult, AppError> {
    let metadata = tokio::fs::metadata(local_path.as_path()).await?;
    let owner_id = account_owner(api).await?;
    checkpoints
        .insert(TransferCheckpoint::Upload(UploadCheckpoint {
            transfer_id: transfer_id.clone(),
            owner_id,
            mount_id: mount_id.as_str().to_owned(),
            remote_directory: directory.as_str().to_owned(),
            local_path: local_path.as_path().to_path_buf(),
            expected_size: metadata.len(),
            modified_millis: modified_millis(&metadata)?,
        }))
        .await?;
    run_upload(
        app,
        api,
        manager,
        checkpoints,
        transfer_id,
        mount_id,
        directory,
        local_path,
    )
    .await
}

pub async fn retry_upload(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    transfer_id: String,
) -> Result<TransferResult, AppError> {
    let TransferCheckpoint::Upload(checkpoint) = checkpoints.get(&transfer_id).await? else {
        return Err(AppError::InvalidInput("upload checkpoint"));
    };
    let local_path = LocalUploadPath::from_selected(checkpoint.local_path).await?;
    let metadata = tokio::fs::metadata(local_path.as_path()).await?;
    if metadata.len() != checkpoint.expected_size
        || modified_millis(&metadata)? != checkpoint.modified_millis
    {
        return Err(AppError::Conflict);
    }
    run_upload(
        app,
        api,
        manager,
        checkpoints,
        transfer_id,
        MountId::parse(checkpoint.mount_id)?,
        RemotePath::parse(checkpoint.remote_directory)?,
        local_path,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_upload(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    transfer_id: String,
    mount_id: MountId,
    directory: RemotePath,
    local_path: LocalUploadPath,
) -> Result<TransferResult, AppError> {
    let cancel = manager.register(&transfer_id)?;
    let progress = Arc::new(AtomicU64::new(0));
    let result = upload_inner(
        &app,
        api,
        &transfer_id,
        &cancel,
        mount_id,
        directory,
        local_path,
        progress.clone(),
    )
    .await;
    let paused = manager.was_paused(&transfer_id);
    manager.finish(&transfer_id);
    let result = match result {
        Ok(result) => {
            checkpoints.remove(&transfer_id).await?;
            Ok(result)
        }
        other => normalize_interruption(other, paused),
    };
    emit_terminal(
        &app,
        &transfer_id,
        TransferDirection::Upload,
        progress.load(Ordering::Relaxed),
        &result,
    );
    result
}

#[allow(clippy::too_many_arguments)]
async fn upload_inner(
    app: &AppHandle,
    api: &KoofrApi,
    transfer_id: &str,
    cancel: &CancellationToken,
    mount_id: MountId,
    directory: RemotePath,
    local_path: LocalUploadPath,
    progress: Arc<AtomicU64>,
) -> Result<TransferResult, AppError> {
    let file_name = RemoteName::parse(local_path.file_name()?)?;
    let file = tokio::fs::File::open(local_path.as_path()).await?;
    let total = file.metadata().await?.len();
    let app_for_stream = app.clone();
    let id_for_stream = transfer_id.to_owned();
    emit_progress(
        app,
        transfer_id,
        TransferDirection::Upload,
        TransferState::Running,
        0,
        Some(total),
    );
    let stream = ReaderStream::new(file).inspect_ok(move |chunk| {
        let previous = progress.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        emit_progress(
            &app_for_stream,
            &id_for_stream,
            TransferDirection::Upload,
            TransferState::Running,
            previous.saturating_add(chunk.len() as u64),
            Some(total),
        );
    });
    let body = reqwest::Body::wrap_stream(stream);
    let file_info = tokio::select! {
        result = api.upload(&mount_id, &directory, &file_name, body, total) => result?,
        () = cancel.cancelled() => return Err(AppError::Cancelled),
    };
    Ok(TransferResult {
        transfer_id: transfer_id.to_owned(),
        bytes_transferred: total,
        file: Some(file_info),
    })
}

pub(super) fn modified_millis(metadata: &std::fs::Metadata) -> Result<u128, AppError> {
    metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|_| AppError::InvalidInput("local file modification time"))
}

async fn account_owner(api: &KoofrApi) -> Result<String, AppError> {
    api.recovery_scope().await
}
