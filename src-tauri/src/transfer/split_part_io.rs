use std::{
    io::{BufReader, Read, Seek},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::TryStreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

use crate::{
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath},
};

use super::{
    TransferDirection, TransferState,
    checkpoint::{SplitUploadCheckpoint, TransferCheckpoint},
    emit_progress,
    split_package::{SplitPart, encode_sha256, part_file_name},
    split_upload::SplitTransferRuntime,
};

pub(super) async fn upload_next_part(
    runtime: &SplitTransferRuntime<'_>,
    cancel: &CancellationToken,
    mount_id: &MountId,
    package_path: &RemotePath,
    checkpoint: &mut SplitUploadCheckpoint,
) -> Result<(), AppError> {
    let committed = committed_bytes(checkpoint);
    let size = (checkpoint.expected_size - committed).min(checkpoint.chunk_size);
    let index = u32::try_from(checkpoint.completed_chunks.len())
        .map_err(|_| AppError::InvalidInput("split part index"))?;
    let mut file = tokio::fs::File::open(&checkpoint.local_path).await?;
    file.seek(std::io::SeekFrom::Start(committed)).await?;

    let hasher = Arc::new(Mutex::new(Sha256::new()));
    let streamed = Arc::new(AtomicU64::new(0));
    let stream_hasher = Arc::clone(&hasher);
    let stream_progress = Arc::clone(&streamed);
    let app = runtime.app.clone();
    let transfer_id = checkpoint.transfer_id.clone();
    let total = checkpoint.expected_size;
    let stream = ReaderStream::new(file.take(size)).inspect_ok(move |chunk| {
        match stream_hasher.lock() {
            Ok(mut digest) => digest.update(chunk),
            Err(poisoned) => poisoned.into_inner().update(chunk),
        }
        let part_bytes = stream_progress
            .fetch_add(chunk.len() as u64, Ordering::Relaxed)
            .saturating_add(chunk.len() as u64);
        emit_progress(
            &app,
            &transfer_id,
            TransferDirection::Upload,
            TransferState::Running,
            committed.saturating_add(part_bytes),
            Some(total),
        );
    });
    let name = RemoteName::parse(part_file_name(index))?;
    tokio::select! {
        result = runtime.api.upload(
            mount_id,
            package_path,
            &name,
            reqwest::Body::wrap_stream(stream),
            size,
        ) => { result?; }
        () = cancel.cancelled() => return Err(AppError::Cancelled),
    }

    let digest = match hasher.lock() {
        Ok(digest) => digest.clone().finalize(),
        Err(poisoned) => poisoned.into_inner().clone().finalize(),
    };
    checkpoint
        .completed_chunks
        .push(SplitPart::new(index, size, encode_sha256(&digest)));
    runtime
        .checkpoints
        .insert(TransferCheckpoint::SplitUpload(checkpoint.clone()))
        .await
}

pub(super) async fn hash_file(
    path: PathBuf,
    cancel: CancellationToken,
) -> Result<String, AppError> {
    hash_range(path, 0, None, cancel).await
}

pub(super) async fn hash_file_range(
    path: PathBuf,
    offset: u64,
    size: u64,
    cancel: CancellationToken,
) -> Result<String, AppError> {
    hash_range(path, offset, Some(size), cancel).await
}

async fn hash_range(
    path: PathBuf,
    offset: u64,
    size: Option<u64>,
    cancel: CancellationToken,
) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(path)?;
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut remaining = size.unwrap_or(u64::MAX);
        while remaining > 0 {
            if cancel.is_cancelled() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "split hashing cancelled",
                ));
            }
            let limit = usize::try_from(remaining.min(buffer.len() as u64))
                .map_err(|_| std::io::Error::other("invalid hash range"))?;
            let read = reader.read(&mut buffer[..limit])?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            remaining = remaining.saturating_sub(read as u64);
        }
        if size.is_some() && remaining != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "split source changed while hashing",
            ));
        }
        Ok::<_, std::io::Error>(encode_sha256(&hasher.finalize()))
    })
    .await
    .map_err(|_| AppError::LocalData)?
    .map_err(AppError::Io)
}

pub(super) fn committed_bytes(checkpoint: &SplitUploadCheckpoint) -> u64 {
    checkpoint
        .completed_chunks
        .iter()
        .map(|part| part.size)
        .sum()
}
