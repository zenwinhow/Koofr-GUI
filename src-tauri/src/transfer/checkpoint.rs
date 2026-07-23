use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::AppError;

use super::{TransferDirection, checkpoint_snapshot::snapshot, split_package::SplitPart};

const CHECKPOINT_VERSION: u8 = 2;
const MAX_CHECKPOINTS: usize = 128;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryKind {
    ByteResume,
    ChunkResume,
    Restart,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCheckpoint {
    pub transfer_id: String,
    pub owner_id: String,
    pub mount_id: String,
    pub remote_path: String,
    pub local_path: PathBuf,
    pub partial_path: PathBuf,
    pub expected_size: u64,
    pub remote_hash: String,
    pub remote_modified: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDownloadCheckpoint {
    pub transfer_id: String,
    pub owner_id: String,
    pub repo_id: String,
    pub mount_id: String,
    pub remote_path: String,
    pub local_path: PathBuf,
    pub partial_path: PathBuf,
    pub expected_size: u64,
    pub remote_hash: String,
    pub remote_modified: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultUploadCheckpoint {
    pub transfer_id: String,
    pub owner_id: String,
    pub repo_id: String,
    pub mount_id: String,
    pub remote_directory: String,
    pub local_path: PathBuf,
    pub expected_size: u64,
    pub modified_millis: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadCheckpoint {
    pub transfer_id: String,
    pub owner_id: String,
    pub mount_id: String,
    pub remote_directory: String,
    pub local_path: PathBuf,
    pub expected_size: u64,
    pub modified_millis: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitUploadCheckpoint {
    pub transfer_id: String,
    pub owner_id: String,
    pub mount_id: String,
    pub remote_directory: String,
    pub package_path: String,
    pub local_path: PathBuf,
    pub expected_size: u64,
    pub modified_millis: u128,
    pub chunk_size: u64,
    pub completed_chunks: Vec<SplitPart>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "direction", content = "checkpoint", rename_all = "snake_case")]
pub enum TransferCheckpoint {
    Download(DownloadCheckpoint),
    VaultDownload(VaultDownloadCheckpoint),
    VaultUpload(VaultUploadCheckpoint),
    SplitUpload(SplitUploadCheckpoint),
    Upload(UploadCheckpoint),
}

impl TransferCheckpoint {
    fn transfer_id(&self) -> &str {
        match self {
            Self::Download(checkpoint) => &checkpoint.transfer_id,
            Self::VaultDownload(checkpoint) => &checkpoint.transfer_id,
            Self::VaultUpload(checkpoint) => &checkpoint.transfer_id,
            Self::SplitUpload(checkpoint) => &checkpoint.transfer_id,
            Self::Upload(checkpoint) => &checkpoint.transfer_id,
        }
    }

    pub fn owner_id(&self) -> &str {
        match self {
            Self::Download(checkpoint) => &checkpoint.owner_id,
            Self::VaultDownload(checkpoint) => &checkpoint.owner_id,
            Self::VaultUpload(checkpoint) => &checkpoint.owner_id,
            Self::SplitUpload(checkpoint) => &checkpoint.owner_id,
            Self::Upload(checkpoint) => &checkpoint.owner_id,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableTransfer {
    pub transfer_id: String,
    pub name: String,
    pub direction: TransferDirection,
    pub recovery_kind: RecoveryKind,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredCheckpoints {
    version: u8,
    checkpoints: BTreeMap<String, TransferCheckpoint>,
}

impl Default for StoredCheckpoints {
    fn default() -> Self {
        Self {
            version: CHECKPOINT_VERSION,
            checkpoints: BTreeMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct TransferCheckpointStore {
    path: PathBuf,
    state: Arc<RwLock<StoredCheckpoints>>,
}

impl TransferCheckpointStore {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<StoredCheckpoints>(&bytes).ok())
            .filter(|stored| stored.version == CHECKPOINT_VERSION)
            .unwrap_or_default();
        Self {
            path,
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub async fn insert(&self, checkpoint: TransferCheckpoint) -> Result<(), AppError> {
        let transfer_id = checkpoint.transfer_id().to_owned();
        validate_transfer_id(&transfer_id)?;
        {
            let mut state = self.state.write().await;
            if state.checkpoints.len() >= MAX_CHECKPOINTS
                && !state.checkpoints.contains_key(&transfer_id)
            {
                return Err(AppError::LocalData);
            }
            state.checkpoints.insert(transfer_id, checkpoint);
        }
        self.persist().await
    }

    pub async fn get(&self, transfer_id: &str) -> Result<TransferCheckpoint, AppError> {
        validate_transfer_id(transfer_id)?;
        self.state
            .read()
            .await
            .checkpoints
            .get(transfer_id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    pub async fn remove(&self, transfer_id: &str) -> Result<bool, AppError> {
        validate_transfer_id(transfer_id)?;
        let removed = self
            .state
            .write()
            .await
            .checkpoints
            .remove(transfer_id)
            .is_some();
        if removed {
            self.persist().await?;
        }
        Ok(removed)
    }

    pub async fn list(&self, owner_id: &str) -> Result<Vec<ResumableTransfer>, AppError> {
        if owner_id.is_empty() || owner_id.len() > 256 || owner_id.contains('\0') {
            return Err(AppError::InvalidInput("checkpoint owner"));
        }
        let checkpoints = self
            .state
            .read()
            .await
            .checkpoints
            .values()
            .filter(|checkpoint| checkpoint.owner_id() == owner_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut snapshots = Vec::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            snapshots.push(snapshot(checkpoint).await?);
        }
        Ok(snapshots)
    }

    async fn persist(&self) -> Result<(), AppError> {
        let payload = serde_json::to_vec_pretty(&*self.state.read().await)
            .map_err(|_| AppError::LocalData)?;
        let parent = self.path.parent().ok_or(AppError::LocalData)?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| AppError::LocalData)?;
        let temporary = self.path.with_extension("json.tmp");
        tokio::fs::write(&temporary, payload)
            .await
            .map_err(|_| AppError::LocalData)?;
        if tokio::fs::try_exists(&self.path).await.unwrap_or(false) {
            tokio::fs::remove_file(&self.path)
                .await
                .map_err(|_| AppError::LocalData)?;
        }
        tokio::fs::rename(temporary, &self.path)
            .await
            .map_err(|_| AppError::LocalData)
    }
}

fn validate_transfer_id(transfer_id: &str) -> Result<(), AppError> {
    let parsed =
        uuid::Uuid::parse_str(transfer_id).map_err(|_| AppError::InvalidInput("transfer id"))?;
    if parsed.is_nil() {
        return Err(AppError::InvalidInput("transfer id"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
