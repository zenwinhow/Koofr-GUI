use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::AppError;

const SETTINGS_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CacheMode {
    Off,
    #[default]
    Memory,
    Disk,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    version: u8,
    cache_mode: CacheMode,
    cache_ttl_minutes: u32,
    remembered_email: Option<String>,
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            cache_mode: CacheMode::Memory,
            cache_ttl_minutes: 15,
            remembered_email: None,
        }
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    path: PathBuf,
    state: Arc<RwLock<StoredSettings>>,
    initial_cache_mode: CacheMode,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<StoredSettings>(&bytes).ok())
            .filter(|settings| settings.version == SETTINGS_VERSION)
            .unwrap_or_default();
        Self {
            path,
            initial_cache_mode: state.cache_mode,
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub const fn initial_cache_mode(&self) -> CacheMode {
        self.initial_cache_mode
    }

    pub async fn cache_policy(&self) -> (CacheMode, u32) {
        let state = self.state.read().await;
        (state.cache_mode, state.cache_ttl_minutes)
    }

    pub async fn remembered_email(&self) -> Option<String> {
        self.state.read().await.remembered_email.clone()
    }

    pub async fn update_cache(
        &self,
        cache_mode: CacheMode,
        cache_ttl_minutes: u32,
    ) -> Result<(), AppError> {
        if !(1..=1440).contains(&cache_ttl_minutes) {
            return Err(AppError::InvalidInput("cache_ttl_minutes"));
        }
        {
            let mut state = self.state.write().await;
            state.cache_mode = cache_mode;
            state.cache_ttl_minutes = cache_ttl_minutes;
        }
        self.persist().await
    }

    pub async fn set_remembered_email(&self, email: Option<String>) -> Result<(), AppError> {
        self.state.write().await.remembered_email = email;
        self.persist().await
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

#[cfg(test)]
mod tests {
    use super::{CacheMode, SettingsStore};

    #[tokio::test]
    async fn persists_non_secret_settings_without_credentials() {
        let directory =
            std::env::temp_dir().join(format!("koofr-settings-{}", uuid::Uuid::new_v4()));
        let path = directory.join("settings.json");
        let store = SettingsStore::load(path.clone());

        store
            .update_cache(CacheMode::Disk, 60)
            .await
            .expect("update cache settings");
        store
            .set_remembered_email(Some("person@example.com".to_owned()))
            .await
            .expect("store remembered email");

        let reloaded = SettingsStore::load(path.clone());
        assert_eq!(reloaded.cache_policy().await, (CacheMode::Disk, 60));
        assert_eq!(
            reloaded.remembered_email().await.as_deref(),
            Some("person@example.com")
        );
        let payload = std::fs::read_to_string(path).expect("read settings file");
        assert!(!payload.contains("password"));

        std::fs::remove_dir_all(directory).expect("remove settings directory");
    }
}
