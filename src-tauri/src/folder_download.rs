use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::StreamExt;
use tauri::AppHandle;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath, safe_suggested_file_name},
    koofr_api::KoofrApi,
    transfer::{
        NetworkRetryPolicy, TransferDirection, TransferManager, TransferResult, TransferState,
        emit_progress, emit_terminal, should_retry_network, wait_for_network_retry,
    },
};

#[cfg(test)]
mod test_support;

#[derive(Clone, Debug)]
pub struct FolderDownloadTarget(PathBuf);

impl FolderDownloadTarget {
    pub async fn from_parent(parent: PathBuf, name: &RemoteName) -> Result<Self, AppError> {
        if !parent.is_absolute() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        let metadata = tokio::fs::symlink_metadata(&parent).await?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        let preferred = safe_suggested_file_name(name);
        for index in 1_u64.. {
            let candidate = if index == 1 {
                preferred.clone()
            } else {
                numbered_name(&preferred, index)
            };
            let target = parent.join(candidate);
            if !tokio::fs::try_exists(&target).await? {
                return Self::from_selected(target).await;
            }
        }
        unreachable!()
    }

    pub async fn from_selected(path: PathBuf) -> Result<Self, AppError> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(AppError::InvalidInput("local folder download path"));
        }
        if tokio::fs::try_exists(&path).await? {
            return Err(AppError::Conflict);
        }
        let parent = path
            .parent()
            .ok_or(AppError::InvalidInput("local download parent"))?;
        let metadata = tokio::fs::symlink_metadata(parent).await?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    fn staging_path(&self) -> Result<PathBuf, AppError> {
        let name = self
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(AppError::InvalidInput("local folder download name"))?;
        Ok(self
            .0
            .with_file_name(format!(".{name}.koofr-part-{}", uuid::Uuid::new_v4())))
    }
}

pub struct FolderDownloadRequest {
    pub transfer_id: String,
    pub mount_id: MountId,
    pub remote_path: RemotePath,
    pub target: FolderDownloadTarget,
}

pub struct FolderDownloadContext<'a> {
    pub app: AppHandle,
    pub api: &'a KoofrApi,
    pub manager: &'a TransferManager,
    pub retry_policy: NetworkRetryPolicy,
}

pub async fn download_folder(
    context: FolderDownloadContext<'_>,
    request: FolderDownloadRequest,
) -> Result<TransferResult, AppError> {
    let cancel = context.manager.register(&request.transfer_id)?;
    let progress = Arc::new(AtomicU64::new(0));
    let app = context.app.clone();
    let transfer_id = request.transfer_id.clone();
    let progress_for_events = progress.clone();
    let mut executor = FolderExecutor {
        api: context.api,
        mount_id: &request.mount_id,
        target: &request.target,
        cancel: &cancel,
        progress: move |transferred, total| {
            progress_for_events.store(transferred, Ordering::Relaxed);
            emit_progress(
                &app,
                &transfer_id,
                TransferDirection::Download,
                TransferState::Running,
                transferred,
                total,
            );
        },
    };
    let mut retries_completed = 0_u32;
    let result = loop {
        let result = executor
            .run(request.remote_path.clone(), &request.transfer_id)
            .await;
        if !should_retry_network(&result, context.retry_policy, retries_completed) {
            break result;
        }
        retries_completed = retries_completed.saturating_add(1);
        if let Err(error) = wait_for_network_retry(
            &context.app,
            &cancel,
            &request.transfer_id,
            TransferDirection::Download,
            retries_completed,
            progress.load(Ordering::Relaxed),
            None,
            context.retry_policy,
        )
        .await
        {
            break Err(error);
        }
    };
    let paused = context.manager.was_paused(&request.transfer_id);
    context.manager.finish(&request.transfer_id);
    let result = match result {
        Err(AppError::Cancelled) if paused => Err(AppError::TransferPaused),
        other => other,
    };
    emit_terminal(
        &context.app,
        &request.transfer_id,
        TransferDirection::Download,
        progress.load(Ordering::Relaxed),
        &result,
    );
    result
}

struct FolderManifest {
    directories: Vec<PathBuf>,
    files: Vec<FolderManifestFile>,
    total_bytes: u64,
}

struct FolderManifestFile {
    remote_path: RemotePath,
    relative_path: PathBuf,
    size: u64,
}

struct FolderExecutor<'a, F> {
    api: &'a KoofrApi,
    mount_id: &'a MountId,
    target: &'a FolderDownloadTarget,
    cancel: &'a CancellationToken,
    progress: F,
}

impl<F> FolderExecutor<'_, F>
where
    F: FnMut(u64, Option<u64>),
{
    async fn run(
        &mut self,
        remote_path: RemotePath,
        transfer_id: &str,
    ) -> Result<TransferResult, AppError> {
        FolderDownloadTarget::from_selected(self.target.0.clone()).await?;
        let staging = self.target.staging_path()?;
        tokio::fs::create_dir(&staging).await?;
        let result = self.run_staged(remote_path, transfer_id, &staging).await;
        if let Err(error) = result {
            tokio::fs::remove_dir_all(&staging).await?;
            return Err(error);
        }
        result
    }

    async fn run_staged(
        &mut self,
        remote_path: RemotePath,
        transfer_id: &str,
        staging: &Path,
    ) -> Result<TransferResult, AppError> {
        let manifest = self.build_manifest(remote_path).await?;
        (self.progress)(0, Some(manifest.total_bytes));
        for directory in &manifest.directories {
            self.ensure_active()?;
            tokio::fs::create_dir_all(staging.join(directory)).await?;
        }
        let transferred = self.download_files(&manifest, staging).await?;
        self.ensure_active()?;
        tokio::fs::rename(staging, self.target.as_path()).await?;
        Ok(TransferResult {
            transfer_id: transfer_id.to_owned(),
            bytes_transferred: transferred,
            file: None,
        })
    }

    async fn build_manifest(&self, root: RemotePath) -> Result<FolderManifest, AppError> {
        let mut queue = VecDeque::from([(root, PathBuf::new())]);
        let mut directories = Vec::new();
        let mut files = Vec::new();
        let mut total_bytes = 0_u64;

        while let Some((remote_directory, relative_directory)) = queue.pop_front() {
            self.ensure_active()?;
            let mut entries = tokio::select! {
                result = self.api.list_files(self.mount_id, &remote_directory) => result?,
                () = self.cancel.cancelled() => return Err(AppError::Cancelled),
            };
            entries.sort_by(|left, right| {
                left.name
                    .to_lowercase()
                    .cmp(&right.name.to_lowercase())
                    .then_with(|| left.name.cmp(&right.name))
            });
            let mut used_names = HashSet::new();
            for entry in entries {
                let name = RemoteName::parse(entry.name)?;
                let local_name = allocate_local_segment(&name, &mut used_names);
                let relative_path = relative_directory.join(local_name);
                let remote_path = remote_directory.join(&name)?;
                match entry.entry_type.as_str() {
                    "dir" | "folder" => {
                        directories.push(relative_path.clone());
                        queue.push_back((remote_path, relative_path));
                    }
                    "file" => {
                        let size = u64::try_from(entry.size)
                            .map_err(|_| AppError::InvalidInput("remote file size"))?;
                        total_bytes = total_bytes
                            .checked_add(size)
                            .ok_or(AppError::InvalidInput("folder download size"))?;
                        files.push(FolderManifestFile {
                            remote_path,
                            relative_path,
                            size,
                        });
                    }
                    _ => return Err(AppError::InvalidInput("remote entry type")),
                }
            }
        }
        Ok(FolderManifest {
            directories,
            files,
            total_bytes,
        })
    }

    async fn download_files(
        &mut self,
        manifest: &FolderManifest,
        staging: &Path,
    ) -> Result<u64, AppError> {
        let mut transferred = 0_u64;
        for file in &manifest.files {
            self.ensure_active()?;
            let response = tokio::select! {
                result = self.api.download_response(self.mount_id, &file.remote_path) => result?,
                () = self.cancel.cancelled() => return Err(AppError::Cancelled),
            };
            if response
                .content_length()
                .is_some_and(|length| length != file.size)
            {
                return Err(AppError::IncompleteTransfer);
            }
            let output_path = staging.join(&file.relative_path);
            let mut output = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output_path)
                .await?;
            let mut stream = response.bytes_stream();
            let file_start = transferred;
            loop {
                let next = tokio::select! {
                    item = stream.next() => item,
                    () = self.cancel.cancelled() => return Err(AppError::Cancelled),
                };
                match next {
                    Some(Ok(chunk)) => {
                        output.write_all(&chunk).await?;
                        transferred = transferred
                            .checked_add(chunk.len() as u64)
                            .ok_or(AppError::IncompleteTransfer)?;
                        (self.progress)(transferred, Some(manifest.total_bytes));
                    }
                    Some(Err(error)) => return Err(AppError::Network(error)),
                    None => break,
                }
            }
            if transferred - file_start != file.size {
                return Err(AppError::IncompleteTransfer);
            }
            output.flush().await?;
            output.sync_all().await?;
        }
        Ok(transferred)
    }

    fn ensure_active(&self) -> Result<(), AppError> {
        if self.cancel.is_cancelled() {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn allocate_local_segment(name: &RemoteName, used_names: &mut HashSet<String>) -> String {
    let sanitized = safe_suggested_file_name(name);
    if used_names.insert(sanitized.to_lowercase()) {
        return sanitized;
    }

    let (stem, extension) = split_extension(&sanitized);
    for index in 2_u64.. {
        let candidate = numbered_name_parts(stem, extension, index);
        if used_names.insert(candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!()
}

fn numbered_name(name: &str, index: u64) -> String {
    let (stem, extension) = split_extension(name);
    numbered_name_parts(stem, extension, index)
}

fn numbered_name_parts(stem: &str, extension: &str, index: u64) -> String {
    let suffix = format!(" ({index})");
    let extension_budget = 255_usize.saturating_sub(suffix.len());
    let fitted_extension = truncate_utf16(extension, extension_budget);
    let stem_budget = extension_budget.saturating_sub(fitted_extension.encode_utf16().count());
    format!(
        "{}{}{}",
        truncate_utf16(stem, stem_budget),
        suffix,
        fitted_extension
    )
}

fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.').filter(|position| *position > 0) {
        Some(position) => (&name[..position], &name[position..]),
        None => (name, ""),
    }
}

fn truncate_utf16(value: &str, max_units: usize) -> String {
    let mut units = 0_usize;
    value
        .chars()
        .take_while(|character| {
            let next = units + character.len_utf16();
            if next > max_units {
                false
            } else {
                units = next;
                true
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use httpmock::MockServer;
    use serde_json::json;

    use crate::file_ops::RemoteName;

    use super::{
        FolderDownloadTarget,
        test_support::{
            authenticated_api, file_json, mock_content, mock_listing, mock_token, run_executor,
            temporary_parent,
        },
    };

    #[tokio::test]
    async fn downloads_nested_folder_and_preserves_empty_directories() {
        // Given a nested remote folder, an empty directory, and authenticated API access.
        let server = MockServer::start_async().await;
        mock_token(&server).await;
        mock_listing(
            &server,
            "/资料",
            json!({"files": [
                file_json("note.txt", "file", 5),
                file_json("子目录", "dir", 0),
                file_json("空目录", "dir", 0)
            ]}),
        )
        .await;
        mock_listing(
            &server,
            "/资料/子目录",
            json!({"files": [file_json("你好.txt", "file", 6)]}),
        )
        .await;
        mock_listing(&server, "/资料/空目录", json!({"files": []})).await;
        mock_content(&server, "/资料/note.txt", "hello").await;
        mock_content(&server, "/资料/子目录/你好.txt", "world!").await;
        let api = authenticated_api(&server).await;
        let parent = temporary_parent("nested").await;
        let target = FolderDownloadTarget::from_parent(
            parent.clone(),
            &RemoteName::parse("资料".to_owned()).expect("folder name"),
        )
        .await
        .expect("download target");

        // When the folder transfer completes.
        let result = run_executor(&api, &target, "/资料")
            .await
            .expect("folder download");

        // Then all files and empty directories exist under the final target.
        assert_eq!(result.bytes_transferred, 11);
        assert_eq!(
            tokio::fs::read(target.as_path().join("note.txt"))
                .await
                .unwrap(),
            b"hello"
        );
        assert_eq!(
            tokio::fs::read(target.as_path().join("子目录").join("你好.txt"))
                .await
                .unwrap(),
            b"world!"
        );
        assert!(target.as_path().join("空目录").is_dir());
        tokio::fs::remove_dir_all(parent)
            .await
            .expect("remove test directory");
    }

    #[tokio::test]
    async fn failed_folder_download_leaves_no_final_or_staging_tree() {
        // Given a remote file whose response is shorter than its listed size.
        let server = MockServer::start_async().await;
        mock_token(&server).await;
        mock_listing(
            &server,
            "/broken",
            json!({"files": [file_json("broken.bin", "file", 5)]}),
        )
        .await;
        mock_content(&server, "/broken/broken.bin", "bad").await;
        let api = authenticated_api(&server).await;
        let parent = temporary_parent("failure").await;
        let target = FolderDownloadTarget::from_parent(
            parent.clone(),
            &RemoteName::parse("broken".to_owned()).expect("folder name"),
        )
        .await
        .expect("download target");

        // When the folder transfer fails integrity validation.
        let result = run_executor(&api, &target, "/broken").await;

        // Then neither a final folder nor a hidden staging tree remains.
        assert!(matches!(
            result,
            Err(crate::error::AppError::IncompleteTransfer)
        ));
        assert!(!target.as_path().exists());
        assert!(
            std::fs::read_dir(&parent)
                .expect("read parent")
                .next()
                .is_none()
        );
        tokio::fs::remove_dir_all(parent)
            .await
            .expect("remove test directory");
    }
}
