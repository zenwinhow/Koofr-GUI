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
mod vault;

pub use checkpoint::{RecoveryKind, ResumableTransfer, TransferCheckpointStore};
pub use download::{download, resume_download};
pub use manager::TransferManager;
pub use model::{
    NetworkRetryPolicy, NetworkRetryRequest, TransferDirection, TransferResult, TransferState,
    emit_progress, emit_terminal, normalize_interruption, should_retry_network,
    wait_for_network_retry,
};
pub use split_upload::{
    SplitTransferRuntime, SplitUploadRequest, resume_split_upload, upload_split,
    validate_split_part_bytes,
};
pub use upload::{retry_upload, upload};
pub use vault::{
    download_vault_file, resume_vault_download, resume_vault_upload, upload_vault_file,
};

pub struct ResumeOutcome {
    pub result: TransferResult,
    pub completed_path: Option<std::path::PathBuf>,
}

pub async fn resume_checkpoint(
    app: tauri::AppHandle,
    api: &crate::koofr_api::KoofrApi,
    manager: &TransferManager,
    store: &TransferCheckpointStore,
    vault: &crate::vault_core::VaultManager,
    retry_policy: NetworkRetryPolicy,
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
            let result =
                resume_download(app, api, manager, store, retry_policy, transfer_id).await?;
            Ok(ResumeOutcome {
                result,
                completed_path: Some(completed_path),
            })
        }
        checkpoint::TransferCheckpoint::VaultDownload(checkpoint) => {
            let completed_path = checkpoint.local_path.clone();
            let result =
                resume_vault_download(app, api, manager, store, vault, retry_policy, checkpoint)
                    .await?;
            Ok(ResumeOutcome {
                result,
                completed_path: Some(completed_path),
            })
        }
        checkpoint::TransferCheckpoint::VaultUpload(checkpoint) => {
            let result =
                resume_vault_upload(app, api, manager, store, vault, retry_policy, checkpoint)
                    .await?;
            Ok(ResumeOutcome {
                result,
                completed_path: None,
            })
        }
        checkpoint::TransferCheckpoint::SplitUpload(_) => {
            let result = resume_split_upload(
                SplitTransferRuntime {
                    app,
                    api,
                    manager,
                    checkpoints: store,
                    retry_policy,
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
            let result = retry_upload(app, api, manager, store, retry_policy, transfer_id).await?;
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
    let partial_path = match checkpoint {
        checkpoint::TransferCheckpoint::Download(download) => Some(download.partial_path),
        checkpoint::TransferCheckpoint::VaultDownload(download) => Some(download.partial_path),
        _ => None,
    };
    if let Some(partial_path) = partial_path {
        match tokio::fs::remove_file(partial_path).await {
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
