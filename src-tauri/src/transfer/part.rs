use crate::{error::AppError, file_ops::LocalDownloadPath};

use super::{checkpoint::DownloadCheckpoint, range::ResponseMode};

pub async fn open_partial(
    checkpoint: &DownloadCheckpoint,
    mode: ResponseMode,
) -> Result<tokio::fs::File, AppError> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true);
    match mode {
        ResponseMode::Append => {
            options.append(true);
        }
        ResponseMode::Restart => {
            options.truncate(true);
        }
    }
    options
        .open(&checkpoint.partial_path)
        .await
        .map_err(Into::into)
}

pub async fn partial_length(checkpoint: &DownloadCheckpoint) -> Result<u64, AppError> {
    match tokio::fs::symlink_metadata(&checkpoint.partial_path).await {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            if metadata.len() > checkpoint.expected_size {
                truncate_partial(checkpoint).await?;
                Ok(0)
            } else {
                Ok(metadata.len())
            }
        }
        Ok(_) => Err(AppError::InvalidInput("partial download path")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

pub async fn truncate_partial(checkpoint: &DownloadCheckpoint) -> Result<(), AppError> {
    tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&checkpoint.partial_path)
        .await?;
    Ok(())
}

pub async fn validate_checkpoint_paths(checkpoint: &DownloadCheckpoint) -> Result<(), AppError> {
    let local_path = LocalDownloadPath::from_selected(checkpoint.local_path.clone()).await?;
    if local_path.resumable_temporary_path(&checkpoint.transfer_id)? != checkpoint.partial_path {
        return Err(AppError::InvalidInput("partial download path"));
    }
    let _ = partial_length(checkpoint).await?;
    Ok(())
}
