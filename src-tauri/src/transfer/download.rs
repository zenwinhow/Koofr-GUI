use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use futures_util::StreamExt;
use tauri::AppHandle;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppError,
    file_ops::{LocalDownloadPath, MountId, RemotePath},
    koofr_api::{FileInfo, KoofrApi},
};

use super::{
    checkpoint::{DownloadCheckpoint, TransferCheckpoint, TransferCheckpointStore},
    manager::TransferManager,
    model::{TransferDirection, TransferResult, TransferState, emit_progress, emit_terminal},
    part::{open_partial, partial_length, truncate_partial, validate_checkpoint_paths},
    range::{ResponseMode, response_mode},
};

#[allow(clippy::too_many_arguments)]
pub async fn download(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    transfer_id: String,
    mount_id: MountId,
    remote_path: RemotePath,
    local_path: LocalDownloadPath,
) -> Result<TransferResult, AppError> {
    let info = api.file_info(&mount_id, &remote_path).await?;
    let owner_id = account_owner(api).await?;
    let expected_size = expected_size(&info)?;
    let checkpoint = DownloadCheckpoint {
        transfer_id: transfer_id.clone(),
        owner_id,
        mount_id: mount_id.as_str().to_owned(),
        remote_path: remote_path.as_str().to_owned(),
        partial_path: local_path.resumable_temporary_path(&transfer_id)?,
        local_path: local_path.as_path().to_path_buf(),
        expected_size,
        remote_hash: info.hash,
        remote_modified: info.modified,
    };
    checkpoints
        .insert(TransferCheckpoint::Download(checkpoint.clone()))
        .await?;
    run_download(app, api, manager, checkpoints, checkpoint).await
}

pub async fn resume_download(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    transfer_id: String,
) -> Result<TransferResult, AppError> {
    let TransferCheckpoint::Download(checkpoint) = checkpoints.get(&transfer_id).await? else {
        return Err(AppError::InvalidInput("download checkpoint"));
    };
    run_download(app, api, manager, checkpoints, checkpoint).await
}

async fn run_download(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    checkpoints: &TransferCheckpointStore,
    mut checkpoint: DownloadCheckpoint,
) -> Result<TransferResult, AppError> {
    let transfer_id = checkpoint.transfer_id.clone();
    validate_checkpoint_paths(&checkpoint).await?;
    let cancel = manager.register(&transfer_id)?;
    let progress = Arc::new(AtomicU64::new(partial_length(&checkpoint).await?));
    let result = refresh_remote(api, checkpoints, &mut checkpoint).await;
    let result = match result {
        Ok(()) => download_inner(&app, api, &cancel, &checkpoint, progress.clone()).await,
        Err(error) => Err(error),
    };
    manager.finish(&transfer_id);
    let result = match result {
        Ok(result) => {
            checkpoints.remove(&transfer_id).await?;
            Ok(result)
        }
        Err(_) => Err(AppError::TransferPaused),
    };
    emit_terminal(
        &app,
        &transfer_id,
        TransferDirection::Download,
        progress.load(Ordering::Relaxed),
        &result,
    );
    result
}

async fn refresh_remote(
    api: &KoofrApi,
    checkpoints: &TransferCheckpointStore,
    checkpoint: &mut DownloadCheckpoint,
) -> Result<(), AppError> {
    let mount_id = MountId::parse(checkpoint.mount_id.clone())?;
    let remote_path = RemotePath::parse(checkpoint.remote_path.clone())?;
    let info = api.file_info(&mount_id, &remote_path).await?;
    let size = expected_size(&info)?;
    let hash_changed = !checkpoint.remote_hash.is_empty()
        && !info.hash.is_empty()
        && checkpoint.remote_hash != info.hash;
    if checkpoint.expected_size != size
        || checkpoint.remote_modified != info.modified
        || hash_changed
    {
        truncate_partial(checkpoint).await?;
        checkpoint.expected_size = size;
        checkpoint.remote_hash = info.hash;
        checkpoint.remote_modified = info.modified;
        checkpoints
            .insert(TransferCheckpoint::Download(checkpoint.clone()))
            .await?;
    }
    Ok(())
}

async fn download_inner(
    app: &AppHandle,
    api: &KoofrApi,
    cancel: &CancellationToken,
    checkpoint: &DownloadCheckpoint,
    progress: Arc<AtomicU64>,
) -> Result<TransferResult, AppError> {
    let mount_id = MountId::parse(checkpoint.mount_id.clone())?;
    let remote_path = RemotePath::parse(checkpoint.remote_path.clone())?;
    let offset = partial_length(checkpoint).await?;
    if offset == checkpoint.expected_size {
        let output = open_partial(checkpoint, ResponseMode::Append).await?;
        output.sync_all().await?;
        drop(output);
        tokio::fs::rename(&checkpoint.partial_path, &checkpoint.local_path).await?;
        return Ok(TransferResult {
            transfer_id: checkpoint.transfer_id.clone(),
            bytes_transferred: offset,
            file: None,
        });
    }
    let response = tokio::select! {
        result = api.download_response_from(&mount_id, &remote_path, offset) => result?,
        () = cancel.cancelled() => return Err(AppError::Cancelled),
    };
    let mode = response_mode(
        offset,
        response.status(),
        response.headers().get(reqwest::header::CONTENT_RANGE),
        checkpoint.expected_size,
    )?;
    let transferred = if mode == ResponseMode::Append {
        offset
    } else {
        0
    };
    let mut output = open_partial(checkpoint, mode).await?;
    progress.store(transferred, Ordering::Relaxed);
    emit_progress(
        app,
        &checkpoint.transfer_id,
        TransferDirection::Download,
        TransferState::Running,
        transferred,
        Some(checkpoint.expected_size),
    );
    let mut stream = response.bytes_stream();
    let mut transferred = transferred;
    loop {
        let next = tokio::select! {
            item = stream.next() => item,
            () = cancel.cancelled() => return Err(AppError::Cancelled),
        };
        match next {
            Some(Ok(chunk)) => {
                output.write_all(&chunk).await?;
                transferred = transferred.saturating_add(chunk.len() as u64);
                progress.store(transferred, Ordering::Relaxed);
                emit_progress(
                    app,
                    &checkpoint.transfer_id,
                    TransferDirection::Download,
                    TransferState::Running,
                    transferred,
                    Some(checkpoint.expected_size),
                );
            }
            Some(Err(error)) => return Err(AppError::Network(error)),
            None => break,
        }
    }
    if transferred != checkpoint.expected_size || cancel.is_cancelled() {
        return Err(AppError::IncompleteTransfer);
    }
    output.flush().await?;
    output.sync_all().await?;
    drop(output);
    tokio::fs::rename(&checkpoint.partial_path, &checkpoint.local_path).await?;
    Ok(TransferResult {
        transfer_id: checkpoint.transfer_id.clone(),
        bytes_transferred: transferred,
        file: None,
    })
}

fn expected_size(info: &FileInfo) -> Result<u64, AppError> {
    if info.entry_type != "file" {
        return Err(AppError::InvalidInput("remote download file"));
    }
    u64::try_from(info.size).map_err(|_| AppError::InvalidInput("remote file size"))
}

async fn account_owner(api: &KoofrApi) -> Result<String, AppError> {
    api.recovery_scope().await
}
