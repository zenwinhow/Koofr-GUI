use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{error::AppError, settings::CacheMode};

const CACHE_VERSION: u8 = 1;
const MAX_ENTRIES: usize = 500;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    saved_at_ms: u64,
    value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredCache {
    version: u8,
    account_key: Option<String>,
    entries: HashMap<String, CacheEntry>,
}

impl Default for StoredCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            account_key: None,
            entries: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct MetadataCache {
    path: Arc<RwLock<PathBuf>>,
    state: Arc<RwLock<StoredCache>>,
}

impl MetadataCache {
    pub fn load(path: PathBuf, load_disk: bool) -> Self {
        let state = load_disk
            .then(|| {
                std::fs::symlink_metadata(&path)
                    .ok()
                    .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
                    .and_then(|_| std::fs::read(&path).ok())
            })
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<StoredCache>(&bytes).ok())
            .filter(|cache| cache.version == CACHE_VERSION)
            .unwrap_or_default();
        Self {
            path: Arc::new(RwLock::new(path)),
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub async fn select_account(
        &self,
        account_key: String,
        mode: CacheMode,
    ) -> Result<(), AppError> {
        let changed = {
            let mut state = self.state.write().await;
            if state.account_key.as_deref() == Some(account_key.as_str()) {
                false
            } else {
                state.account_key = Some(account_key);
                state.entries.clear();
                true
            }
        };
        if changed && mode == CacheMode::Disk {
            self.persist().await?;
        }
        Ok(())
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        key: &str,
        mode: CacheMode,
        ttl_minutes: u32,
    ) -> Option<T> {
        if mode == CacheMode::Off {
            return None;
        }
        let now = now_ms();
        let ttl = Duration::from_secs(u64::from(ttl_minutes) * 60).as_millis() as u64;
        let value = {
            let state = self.state.read().await;
            let entry = state.entries.get(key)?;
            (now.saturating_sub(entry.saved_at_ms) <= ttl).then(|| entry.value.clone())?
        };
        serde_json::from_value(value).ok()
    }

    pub async fn put<T: Serialize>(
        &self,
        key: String,
        value: &T,
        mode: CacheMode,
    ) -> Result<(), AppError> {
        if mode == CacheMode::Off {
            return Ok(());
        }
        let value = serde_json::to_value(value).map_err(|_| AppError::LocalData)?;
        {
            let mut state = self.state.write().await;
            if state.entries.len() >= MAX_ENTRIES
                && !state.entries.contains_key(&key)
                && let Some(oldest) = state
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.saved_at_ms)
                    .map(|(key, _)| key.clone())
            {
                state.entries.remove(&oldest);
            }
            state.entries.insert(
                key,
                CacheEntry {
                    saved_at_ms: now_ms(),
                    value,
                },
            );
        }
        if mode == CacheMode::Disk {
            self.persist().await?;
        }
        Ok(())
    }

    pub async fn clear(&self) -> Result<(), AppError> {
        self.state.write().await.entries.clear();
        self.remove_disk_file().await
    }

    pub async fn apply_mode(&self, mode: CacheMode) -> Result<(), AppError> {
        match mode {
            CacheMode::Off => self.clear().await,
            CacheMode::Memory => self.remove_disk_file().await,
            CacheMode::Disk => self.persist().await,
        }
    }

    pub async fn stats(&self) -> (usize, u64) {
        let entries = self.state.read().await.entries.len();
        let path = self.path.read().await.clone();
        let bytes = tokio::fs::metadata(path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        (entries, bytes)
    }

    async fn persist(&self) -> Result<(), AppError> {
        let payload =
            serde_json::to_vec(&*self.state.read().await).map_err(|_| AppError::LocalData)?;
        let path = self.path.read().await.clone();
        let parent = path.parent().ok_or(AppError::LocalData)?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| AppError::LocalData)?;
        let temporary = path.with_extension("json.tmp");
        reject_unsafe_file(&path).await?;
        reject_unsafe_file(&temporary).await?;
        tokio::fs::write(&temporary, payload)
            .await
            .map_err(|_| AppError::LocalData)?;
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|_| AppError::LocalData)?;
        }
        tokio::fs::rename(temporary, path)
            .await
            .map_err(|_| AppError::LocalData)
    }

    async fn remove_disk_file(&self) -> Result<(), AppError> {
        let path = self.path.read().await.clone();
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(path)
                .await
                .map_err(|_| AppError::LocalData)?;
        }
        Ok(())
    }
}

async fn reject_unsafe_file(path: &std::path::Path) -> Result<(), AppError> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(AppError::LocalData),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(error)),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::MetadataCache;
    use crate::settings::CacheMode;

    #[tokio::test]
    async fn persists_disk_entries_and_isolates_accounts() {
        let directory = std::env::temp_dir().join(format!("koofr-cache-{}", uuid::Uuid::new_v4()));
        let path = directory.join("metadata-cache.json");
        let cache = MetadataCache::load(path.clone(), true);
        cache
            .select_account("account-1".to_owned(), CacheMode::Disk)
            .await
            .expect("select first account");
        cache
            .put("files:/".to_owned(), &vec!["one", "two"], CacheMode::Disk)
            .await
            .expect("cache files");

        let reloaded = MetadataCache::load(path, true);
        let files: Vec<String> = reloaded
            .get("files:/", CacheMode::Disk, 15)
            .await
            .expect("read persisted cache");
        assert_eq!(files, ["one", "two"]);

        reloaded
            .select_account("account-2".to_owned(), CacheMode::Memory)
            .await
            .expect("select second account");
        assert!(
            reloaded
                .get::<Vec<String>>("files:/", CacheMode::Memory, 15)
                .await
                .is_none()
        );

        std::fs::remove_dir_all(directory).expect("remove cache directory");
    }
}
