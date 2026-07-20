use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppError,
    file_ops::{LocalUploadPath, MountId, RemoteName, RemotePath},
    koofr_api::{FileInfo, KoofrApi},
};

use super::{
    NetworkRetryPolicy, TransferDirection, TransferResult, TransferState,
    checkpoint::{SplitUploadCheckpoint, TransferCheckpoint, TransferCheckpointStore},
    emit_progress, emit_terminal,
    manager::TransferManager,
    normalize_interruption, should_retry_network,
    split_package::{
        SplitManifest, SplitPart, package_directory_name, part_file_name, validate_part_bytes,
    },
    split_part_io::{committed_bytes, hash_file, hash_file_range, upload_next_part},
    split_support::upload_support_files,
    upload::modified_millis,
    wait_for_network_retry,
};

pub struct SplitTransferRuntime<'a> {
    pub app: AppHandle,
    pub api: &'a KoofrApi,
    pub manager: &'a TransferManager,
    pub checkpoints: &'a TransferCheckpointStore,
    pub retry_policy: NetworkRetryPolicy,
}

pub struct SplitUploadRequest {
    pub transfer_id: String,
    pub mount_id: MountId,
    pub directory: RemotePath,
    pub local_path: LocalUploadPath,
    pub package_name: Option<RemoteName>,
    pub part_bytes: u64,
}

pub fn validate_split_part_bytes(part_bytes: u64) -> Result<u64, AppError> {
    validate_part_bytes(part_bytes)
}

pub async fn upload_split(
    runtime: SplitTransferRuntime<'_>,
    request: SplitUploadRequest,
) -> Result<TransferResult, AppError> {
    let metadata = tokio::fs::metadata(request.local_path.as_path()).await?;
    if metadata.len() == 0 {
        return Err(AppError::InvalidInput("empty split upload"));
    }
    let transfer_uuid = uuid::Uuid::parse_str(&request.transfer_id)
        .map_err(|_| AppError::InvalidInput("transfer id"))?;
    let package_name = match request.package_name {
        Some(name) => name,
        None => RemoteName::parse(package_directory_name(
            &request.local_path.file_name()?,
            transfer_uuid,
        ))?,
    };
    let part_bytes = validate_part_bytes(request.part_bytes)?;
    let package_path = request.directory.join(&package_name)?;
    let checkpoint = SplitUploadCheckpoint {
        transfer_id: request.transfer_id,
        owner_id: runtime.api.recovery_scope().await?,
        mount_id: request.mount_id.as_str().to_owned(),
        remote_directory: request.directory.as_str().to_owned(),
        package_path: package_path.as_str().to_owned(),
        local_path: request.local_path.as_path().to_path_buf(),
        expected_size: metadata.len(),
        modified_millis: modified_millis(&metadata)?,
        chunk_size: part_bytes,
        completed_chunks: Vec::new(),
    };
    runtime
        .checkpoints
        .insert(TransferCheckpoint::SplitUpload(checkpoint.clone()))
        .await?;
    let cancel = runtime.manager.register(&checkpoint.transfer_id)?;
    let preparation = prepare_new_package(
        &runtime,
        &cancel,
        &request.mount_id,
        &request.directory,
        &package_name,
        &package_path,
        &checkpoint,
    )
    .await;
    if let Err(error) = preparation {
        let paused = runtime.manager.was_paused(&checkpoint.transfer_id);
        runtime.manager.finish(&checkpoint.transfer_id);
        let result = normalize_interruption(Err(error), paused);
        if matches!(
            &result,
            Err(AppError::InvalidInput(_) | AppError::Conflict | AppError::Forbidden)
        ) {
            let _ = runtime.checkpoints.remove(&checkpoint.transfer_id).await;
        }
        emit_terminal(
            &runtime.app,
            &checkpoint.transfer_id,
            TransferDirection::Upload,
            0,
            &result,
        );
        return result;
    }
    run_registered(runtime, checkpoint, cancel).await
}

pub async fn resume_split_upload(
    runtime: SplitTransferRuntime<'_>,
    transfer_id: String,
) -> Result<TransferResult, AppError> {
    let TransferCheckpoint::SplitUpload(checkpoint) = runtime.checkpoints.get(&transfer_id).await?
    else {
        return Err(AppError::InvalidInput("split upload checkpoint"));
    };
    let metadata = tokio::fs::metadata(&checkpoint.local_path).await?;
    if metadata.len() != checkpoint.expected_size
        || modified_millis(&metadata)? != checkpoint.modified_millis
    {
        return Err(AppError::Conflict);
    }
    validate_part_bytes(checkpoint.chunk_size)?;
    run(runtime, checkpoint).await
}

async fn run(
    runtime: SplitTransferRuntime<'_>,
    checkpoint: SplitUploadCheckpoint,
) -> Result<TransferResult, AppError> {
    let cancel = runtime.manager.register(&checkpoint.transfer_id)?;
    run_registered(runtime, checkpoint, cancel).await
}

async fn run_registered(
    runtime: SplitTransferRuntime<'_>,
    mut checkpoint: SplitUploadCheckpoint,
    cancel: CancellationToken,
) -> Result<TransferResult, AppError> {
    let mut retries_completed = 0_u32;
    let result = loop {
        let result = run_inner(&runtime, &cancel, &mut checkpoint).await;
        if !should_retry_network(&result, runtime.retry_policy, retries_completed) {
            break result;
        }
        retries_completed = retries_completed.saturating_add(1);
        if let Err(error) = wait_for_network_retry(
            &runtime.app,
            &cancel,
            &checkpoint.transfer_id,
            TransferDirection::Upload,
            retries_completed,
            committed_bytes(&checkpoint),
            Some(checkpoint.expected_size),
            runtime.retry_policy,
        )
        .await
        {
            break Err(error);
        }
    };
    let paused = runtime.manager.was_paused(&checkpoint.transfer_id);
    runtime.manager.finish(&checkpoint.transfer_id);
    let committed = checkpoint
        .completed_chunks
        .iter()
        .map(|part| part.size)
        .sum();
    let result = match result {
        Ok(result) => {
            runtime.checkpoints.remove(&checkpoint.transfer_id).await?;
            Ok(result)
        }
        Err(AppError::InvalidInput(reason)) => Err(AppError::InvalidInput(reason)),
        Err(AppError::Conflict) => Err(AppError::Conflict),
        Err(AppError::Forbidden) => Err(AppError::Forbidden),
        other => normalize_interruption(other, paused),
    };
    emit_terminal(
        &runtime.app,
        &checkpoint.transfer_id,
        TransferDirection::Upload,
        committed,
        &result,
    );
    result
}

async fn prepare_new_package(
    runtime: &SplitTransferRuntime<'_>,
    cancel: &CancellationToken,
    mount_id: &MountId,
    directory: &RemotePath,
    package_name: &RemoteName,
    package_path: &RemotePath,
    checkpoint: &SplitUploadCheckpoint,
) -> Result<(), AppError> {
    let mut retries_completed = 0_u32;
    let mut creation_attempted = false;
    loop {
        let result = prepare_new_package_once(
            runtime.api,
            mount_id,
            directory,
            package_name,
            package_path,
            &mut creation_attempted,
        )
        .await;
        if !should_retry_network(&result, runtime.retry_policy, retries_completed) {
            return result;
        }
        retries_completed = retries_completed.saturating_add(1);
        wait_for_network_retry(
            &runtime.app,
            cancel,
            &checkpoint.transfer_id,
            TransferDirection::Upload,
            retries_completed,
            0,
            Some(checkpoint.expected_size),
            runtime.retry_policy,
        )
        .await?;
    }
}

async fn prepare_new_package_once(
    api: &KoofrApi,
    mount_id: &MountId,
    directory: &RemotePath,
    package_name: &RemoteName,
    package_path: &RemotePath,
    creation_attempted: &mut bool,
) -> Result<(), AppError> {
    match api.file_info(mount_id, package_path).await {
        Ok(info) if *creation_attempted && info.entry_type == "dir" => Ok(()),
        Ok(_) => Err(AppError::Conflict),
        Err(AppError::NotFound) => {
            *creation_attempted = true;
            api.create_folder(mount_id, directory, package_name).await
        }
        Err(error) => Err(error),
    }
}

async fn run_inner(
    runtime: &SplitTransferRuntime<'_>,
    cancel: &CancellationToken,
    checkpoint: &mut SplitUploadCheckpoint,
) -> Result<TransferResult, AppError> {
    let mount_id = MountId::parse(checkpoint.mount_id.clone())?;
    let package_path = RemotePath::parse(checkpoint.package_path.clone())?;
    ensure_package(runtime.api, &mount_id, &package_path).await?;
    reconcile_parts(runtime, cancel, &mount_id, &package_path, checkpoint).await?;
    emit_progress(
        &runtime.app,
        &checkpoint.transfer_id,
        TransferDirection::Upload,
        TransferState::Running,
        committed_bytes(checkpoint),
        Some(checkpoint.expected_size),
    );

    while committed_bytes(checkpoint) < checkpoint.expected_size {
        upload_next_part(runtime, cancel, &mount_id, &package_path, checkpoint).await?;
    }
    let full_hash = hash_file(checkpoint.local_path.clone(), cancel.clone()).await?;
    let manifest = SplitManifest::new(
        file_name(&checkpoint.local_path)?,
        checkpoint.expected_size,
        checkpoint.chunk_size,
        full_hash,
        checkpoint.completed_chunks.clone(),
    )?;
    upload_support_files(runtime.api, &mount_id, &package_path, &manifest).await?;
    Ok(TransferResult {
        transfer_id: checkpoint.transfer_id.clone(),
        bytes_transferred: checkpoint.expected_size,
        file: Some(FileInfo {
            name: manifest.file_name,
            entry_type: "split_package".to_owned(),
            modified: 0,
            size: i64::try_from(checkpoint.expected_size).unwrap_or(i64::MAX),
            content_type: "application/octet-stream".to_owned(),
            hash: manifest.file_sha256,
            path: checkpoint.package_path.clone(),
        }),
    })
}

async fn ensure_package(
    api: &KoofrApi,
    mount_id: &MountId,
    package_path: &RemotePath,
) -> Result<(), AppError> {
    match api.file_info(mount_id, package_path).await {
        Ok(info) if info.entry_type == "dir" => Ok(()),
        Ok(_) => Err(AppError::Conflict),
        Err(AppError::NotFound) => {
            let parent = package_path.parent()?;
            let name = RemoteName::parse(package_path.file_name()?.to_owned())?;
            api.create_folder(mount_id, &parent, &name).await
        }
        Err(error) => Err(error),
    }
}

async fn reconcile_parts(
    runtime: &SplitTransferRuntime<'_>,
    cancel: &CancellationToken,
    mount_id: &MountId,
    package_path: &RemotePath,
    checkpoint: &mut SplitUploadCheckpoint,
) -> Result<(), AppError> {
    let remote = runtime.api.list_files(mount_id, package_path).await?;
    let mut parts = Vec::new();
    let mut covered = 0_u64;
    while covered < checkpoint.expected_size {
        let index =
            u32::try_from(parts.len()).map_err(|_| AppError::InvalidInput("split part index"))?;
        let expected = (checkpoint.expected_size - covered).min(checkpoint.chunk_size);
        let name = part_file_name(index);
        let Some(file) = remote.iter().find(|file| file.name == name) else {
            break;
        };
        if file.entry_type != "file" || u64::try_from(file.size).ok() != Some(expected) {
            return Err(AppError::Conflict);
        }
        let saved_hash = checkpoint
            .completed_chunks
            .iter()
            .find(|part| part.index == index)
            .map(|part| part.sha256.clone());
        let sha256 = match saved_hash {
            Some(hash) => hash,
            None => {
                hash_file_range(
                    checkpoint.local_path.clone(),
                    covered,
                    expected,
                    cancel.clone(),
                )
                .await?
            }
        };
        parts.push(SplitPart::new(index, expected, sha256));
        covered += expected;
    }
    checkpoint.completed_chunks = parts;
    runtime
        .checkpoints
        .insert(TransferCheckpoint::SplitUpload(checkpoint.clone()))
        .await
}

fn file_name(path: &std::path::Path) -> Result<String, AppError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or(AppError::InvalidInput("local file name"))
}
