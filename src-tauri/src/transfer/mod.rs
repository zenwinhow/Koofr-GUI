use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::{StreamExt, TryStreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio_util::{io::ReaderStream, sync::CancellationToken};

use crate::{
    error::AppError,
    file_ops::{LocalDownloadPath, LocalUploadPath, MountId, RemoteName, RemotePath},
    koofr_api::{FileInfo, KoofrApi},
};

pub const TRANSFER_EVENT: &str = "koofr://transfer-progress";

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub file: Option<FileInfo>,
}

#[derive(Default)]
pub struct TransferManager {
    active: Mutex<HashMap<String, CancellationToken>>,
}

impl TransferManager {
    pub fn register(&self, transfer_id: &str) -> Result<CancellationToken, AppError> {
        validate_transfer_id(transfer_id)?;
        let mut active = self.active.lock().expect("transfer registry poisoned");
        if active.contains_key(transfer_id) {
            return Err(AppError::DuplicateTransfer);
        }
        let token = CancellationToken::new();
        active.insert(transfer_id.to_owned(), token.clone());
        Ok(token)
    }

    pub fn finish(&self, transfer_id: &str) {
        self.active
            .lock()
            .expect("transfer registry poisoned")
            .remove(transfer_id);
    }

    pub fn cancel(&self, transfer_id: &str) -> Result<bool, AppError> {
        validate_transfer_id(transfer_id)?;
        let token = self
            .active
            .lock()
            .expect("transfer registry poisoned")
            .get(transfer_id)
            .cloned();
        if let Some(token) = token {
            token.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cancel_all(&self) {
        for token in self
            .active
            .lock()
            .expect("transfer registry poisoned")
            .values()
        {
            token.cancel();
        }
    }
}

pub async fn upload(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
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
    manager.finish(&transfer_id);
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
        let transferred = previous.saturating_add(chunk.len() as u64);
        emit_progress(
            &app_for_stream,
            &id_for_stream,
            TransferDirection::Upload,
            TransferState::Running,
            transferred,
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

pub async fn download(
    app: AppHandle,
    api: &KoofrApi,
    manager: &TransferManager,
    transfer_id: String,
    mount_id: MountId,
    remote_path: RemotePath,
    local_path: LocalDownloadPath,
) -> Result<TransferResult, AppError> {
    let cancel = manager.register(&transfer_id)?;
    let progress = Arc::new(AtomicU64::new(0));
    let result = download_inner(
        &app,
        api,
        &transfer_id,
        &cancel,
        mount_id,
        remote_path,
        local_path,
        progress.clone(),
    )
    .await;
    manager.finish(&transfer_id);
    emit_terminal(
        &app,
        &transfer_id,
        TransferDirection::Download,
        progress.load(Ordering::Relaxed),
        &result,
    );
    result
}

#[allow(clippy::too_many_arguments)]
async fn download_inner(
    app: &AppHandle,
    api: &KoofrApi,
    transfer_id: &str,
    cancel: &CancellationToken,
    mount_id: MountId,
    remote_path: RemotePath,
    local_path: LocalDownloadPath,
    progress: Arc<AtomicU64>,
) -> Result<TransferResult, AppError> {
    let response = tokio::select! {
        result = api.download_response(&mount_id, &remote_path) => result?,
        () = cancel.cancelled() => return Err(AppError::Cancelled),
    };
    let total = response.content_length();
    let temporary_path = local_path.temporary_path()?;
    let mut output = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .await?;
    let mut stream = response.bytes_stream();
    let mut transferred = 0_u64;
    emit_progress(
        app,
        transfer_id,
        TransferDirection::Download,
        TransferState::Running,
        0,
        total,
    );

    let transfer_result = async {
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
                        transfer_id,
                        TransferDirection::Download,
                        TransferState::Running,
                        transferred,
                        total,
                    );
                }
                Some(Err(error)) => return Err(AppError::Network(error)),
                None => break,
            }
        }
        if total.is_some_and(|expected| expected != transferred) {
            return Err(AppError::IncompleteTransfer);
        }
        if cancel.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        output.flush().await?;
        output.sync_all().await?;
        drop(output);
        tokio::fs::rename(&temporary_path, local_path.as_path()).await?;
        Ok(TransferResult {
            transfer_id: transfer_id.to_owned(),
            bytes_transferred: transferred,
            file: None,
        })
    }
    .await;

    if transfer_result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    transfer_result
}

fn validate_transfer_id(transfer_id: &str) -> Result<(), AppError> {
    let parsed =
        uuid::Uuid::parse_str(transfer_id).map_err(|_| AppError::InvalidInput("transfer id"))?;
    if parsed.is_nil() {
        return Err(AppError::InvalidInput("transfer id"));
    }
    Ok(())
}

pub(crate) fn emit_progress(
    app: &AppHandle,
    transfer_id: &str,
    direction: TransferDirection,
    state: TransferState,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
) {
    let _ = app.emit(
        TRANSFER_EVENT,
        TransferProgress {
            transfer_id: transfer_id.to_owned(),
            direction,
            state,
            bytes_transferred,
            total_bytes,
        },
    );
}

pub(crate) fn emit_terminal(
    app: &AppHandle,
    transfer_id: &str,
    direction: TransferDirection,
    bytes_transferred: u64,
    result: &Result<TransferResult, AppError>,
) {
    let (state, bytes) = match result {
        Ok(result) => (TransferState::Completed, result.bytes_transferred),
        Err(AppError::Cancelled) => (TransferState::Cancelled, bytes_transferred),
        Err(_) => (TransferState::Failed, bytes_transferred),
    };
    emit_progress(app, transfer_id, direction, state, bytes, None);
}

#[cfg(test)]
mod tests {
    use super::TransferManager;

    #[test]
    fn transfer_ids_are_unique_and_cancellable() {
        let manager = TransferManager::default();
        let id = uuid::Uuid::new_v4().to_string();
        let token = manager.register(&id).expect("register transfer");
        assert!(manager.register(&id).is_err());
        assert!(manager.cancel(&id).expect("cancel transfer"));
        assert!(token.is_cancelled());
        manager.finish(&id);
        assert!(!manager.cancel(&id).expect("cancel missing transfer"));

        let first = manager
            .register(&uuid::Uuid::new_v4().to_string())
            .expect("register first transfer");
        let second = manager
            .register(&uuid::Uuid::new_v4().to_string())
            .expect("register second transfer");
        manager.cancel_all();
        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
    }
}
