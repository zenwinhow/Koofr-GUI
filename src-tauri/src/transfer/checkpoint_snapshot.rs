use std::path::Path;

use crate::error::AppError;

use super::{
    TransferDirection,
    checkpoint::{RecoveryKind, ResumableTransfer, TransferCheckpoint},
};

pub(super) async fn snapshot(
    checkpoint: TransferCheckpoint,
) -> Result<ResumableTransfer, AppError> {
    match checkpoint {
        TransferCheckpoint::Download(checkpoint) => {
            let bytes_transferred =
                match tokio::fs::symlink_metadata(&checkpoint.partial_path).await {
                    Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                        metadata.len().min(checkpoint.expected_size)
                    }
                    Ok(_) => return Err(AppError::InvalidInput("partial download path")),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
                    Err(error) => return Err(AppError::Io(error)),
                };
            Ok(ResumableTransfer {
                transfer_id: checkpoint.transfer_id,
                name: file_name(&checkpoint.local_path)?,
                direction: TransferDirection::Download,
                recovery_kind: RecoveryKind::ByteResume,
                bytes_transferred,
                total_bytes: checkpoint.expected_size,
            })
        }
        TransferCheckpoint::VaultDownload(checkpoint) => {
            let encrypted_transferred =
                match tokio::fs::symlink_metadata(&checkpoint.partial_path).await {
                    Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                        metadata.len().min(checkpoint.expected_size)
                    }
                    Ok(_) => return Err(AppError::InvalidInput("vault partial download path")),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
                    Err(error) => return Err(AppError::Io(error)),
                };
            let total_bytes = super::vault::decrypted_total(checkpoint.expected_size)?;
            let bytes_transferred =
                super::vault::plaintext_progress(encrypted_transferred, total_bytes);
            Ok(ResumableTransfer {
                transfer_id: checkpoint.transfer_id,
                name: file_name(&checkpoint.local_path)?,
                direction: TransferDirection::Download,
                recovery_kind: RecoveryKind::ByteResume,
                bytes_transferred,
                total_bytes,
            })
        }
        TransferCheckpoint::VaultUpload(checkpoint) => Ok(ResumableTransfer {
            transfer_id: checkpoint.transfer_id,
            name: file_name(&checkpoint.local_path)?,
            direction: TransferDirection::Upload,
            recovery_kind: RecoveryKind::Restart,
            bytes_transferred: 0,
            total_bytes: checkpoint.expected_size,
        }),
        TransferCheckpoint::SplitUpload(checkpoint) => Ok(ResumableTransfer {
            transfer_id: checkpoint.transfer_id,
            name: file_name(&checkpoint.local_path)?,
            direction: TransferDirection::Upload,
            recovery_kind: RecoveryKind::ChunkResume,
            bytes_transferred: checkpoint
                .completed_chunks
                .iter()
                .map(|part| part.size)
                .sum(),
            total_bytes: checkpoint.expected_size,
        }),
        TransferCheckpoint::Upload(checkpoint) => Ok(ResumableTransfer {
            transfer_id: checkpoint.transfer_id,
            name: file_name(&checkpoint.local_path)?,
            direction: TransferDirection::Upload,
            recovery_kind: RecoveryKind::Restart,
            bytes_transferred: 0,
            total_bytes: checkpoint.expected_size,
        }),
    }
}

fn file_name(path: &Path) -> Result<String, AppError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or(AppError::InvalidInput("checkpoint file name"))
}
