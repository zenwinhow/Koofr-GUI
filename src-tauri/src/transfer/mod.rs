mod checkpoint;
mod checkpoint_snapshot;
mod download;
mod manager;
mod model;
mod part;
mod range;
mod split_package;
#[cfg(test)]
#[path = "split_package_tests.rs"]
mod split_package_tests;
mod split_part_io;
mod split_support;
mod split_upload;
mod upload;

pub use checkpoint::{ResumableTransfer, TransferCheckpointStore};
pub use download::{download, resume_download};
pub use manager::TransferManager;
pub use model::{
    TransferDirection, TransferResult, TransferState, emit_progress, emit_terminal,
    normalize_interruption,
};
pub use split_upload::{
    SplitTransferRuntime, SplitUploadRequest, resume_split_upload, upload_split,
    validate_split_part_bytes,
};
pub use upload::{retry_upload, upload};

pub struct ResumeOutcome {
    pub result: TransferResult,
    pub completed_path: Option<std::path::PathBuf>,
}

pub async fn resume_checkpoint(
    app: tauri::AppHandle,
    api: &crate::koofr_api::KoofrApi,
    manager: &TransferManager,
    store: &TransferCheckpointStore,
    transfer_id: String,
) -> Result<ResumeOutcome, crate::error::AppError> {
    let owner_id = current_owner(api).await?;
    let checkpoint = store.get(&transfer_id).await?;
    if checkpoint.owner_id() != owner_id {
        return Err(crate::error::AppError::NotFound);
    }
    match checkpoint {
        checkpoint::TransferCheckpoint::Download(checkpoint) => {
            let completed_path = checkpoint.local_path;
            let result = resume_download(app, api, manager, store, transfer_id).await?;
            Ok(ResumeOutcome {
                result,
                completed_path: Some(completed_path),
            })
        }
        checkpoint::TransferCheckpoint::SplitUpload(_) => {
            let result = resume_split_upload(
                SplitTransferRuntime {
                    app,
                    api,
                    manager,
                    checkpoints: store,
                },
                transfer_id,
            )
            .await?;
            Ok(ResumeOutcome {
                result,
                completed_path: None,
            })
        }
        checkpoint::TransferCheckpoint::Upload(_) => {
            let result = retry_upload(app, api, manager, store, transfer_id).await?;
            Ok(ResumeOutcome {
                result,
                completed_path: None,
            })
        }
    }
}

pub async fn discard_checkpoint(
    store: &TransferCheckpointStore,
    transfer_id: &str,
    owner_id: &str,
) -> Result<bool, crate::error::AppError> {
    let checkpoint = store.get(transfer_id).await?;
    if checkpoint.owner_id() != owner_id {
        return Err(crate::error::AppError::NotFound);
    }
    if let checkpoint::TransferCheckpoint::Download(download) = checkpoint {
        match tokio::fs::remove_file(download.partial_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    store.remove(transfer_id).await
}

pub async fn current_owner(
    api: &crate::koofr_api::KoofrApi,
) -> Result<String, crate::error::AppError> {
    api.recovery_scope().await
}
