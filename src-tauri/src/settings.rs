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
    #[serde(default)]
    download_directory: Option<PathBuf>,
    #[serde(default = "default_ask_download_location")]
    ask_download_location: bool,
}

const fn default_ask_download_location() -> bool {
    true
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            cache_mode: CacheMode::Memory,
            cache_ttl_minutes: 15,
            remembered_email: None,
            download_directory: None,
            ask_download_location: default_ask_download_location(),
        }
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    path: PathBuf,
    state: Arc<RwLock<StoredSettings>>,
    initial_cache_mode: CacheMode,
    default_download_directory: PathBuf,
}

impl SettingsStore {
    pub fn load(path: PathBuf, default_download_directory: PathBuf) -> Self {
        let state = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<StoredSettings>(&bytes).ok())
            .filter(|settings| settings.version == SETTINGS_VERSION)
            .unwrap_or_default();
        Self {
            path,
            initial_cache_mode: state.cache_mode,
            default_download_directory,
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

    pub async fn download_policy(&self) -> (PathBuf, bool) {
        let state = self.state.read().await;
        (
            state
                .download_directory
                .clone()
                .unwrap_or_else(|| self.default_download_directory.clone()),
            state.ask_download_location,
        )
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

    pub async fn update_download(
        &self,
        download_directory: PathBuf,
        ask_download_location: bool,
    ) -> Result<(), AppError> {
        if !download_directory.is_absolute() {
            return Err(AppError::InvalidInput("download directory"));
        }
        let metadata = tokio::fs::symlink_metadata(&download_directory).await?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput("download directory"));
        }
        {
            let mut state = self.state.write().await;
            state.download_directory = Some(download_directory);
            state.ask_download_location = ask_download_location;
        }
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
        let default_download_directory = directory.join("Downloads");
        std::fs::create_dir_all(&default_download_directory).expect("create downloads directory");
        let store = SettingsStore::load(path.clone(), default_download_directory.clone());

        store
            .update_cache(CacheMode::Disk, 60)
            .await
            .expect("update cache settings");
        store
            .set_remembered_email(Some("person@example.com".to_owned()))
            .await
            .expect("store remembered email");

        let reloaded = SettingsStore::load(path.clone(), default_download_directory);
        assert_eq!(reloaded.cache_policy().await, (CacheMode::Disk, 60));
        assert_eq!(
            reloaded.remembered_email().await.as_deref(),
            Some("person@example.com")
        );
        let payload = std::fs::read_to_string(path).expect("read settings file");
        assert!(!payload.contains("password"));

        std::fs::remove_dir_all(directory).expect("remove settings directory");
    }

    #[tokio::test]
    async fn uses_the_os_download_directory_and_prompts_by_default() {
        // Given
        let directory =
            std::env::temp_dir().join(format!("koofr-settings-{}", uuid::Uuid::new_v4()));
        let default_download_directory = directory.join("Downloads");

        // When
        let store = SettingsStore::load(
            directory.join("settings.json"),
            default_download_directory.clone(),
        );

        // Then
        assert_eq!(
            store.download_policy().await,
            (default_download_directory, true)
        );
    }

    #[tokio::test]
    async fn persists_validated_download_preferences() {
        // Given
        let directory =
            std::env::temp_dir().join(format!("koofr-settings-{}", uuid::Uuid::new_v4()));
        let default_download_directory = directory.join("Downloads");
        let custom_download_directory = directory.join("Koofr downloads");
        std::fs::create_dir_all(&custom_download_directory)
            .expect("create custom downloads directory");
        let path = directory.join("settings.json");
        let store = SettingsStore::load(path.clone(), default_download_directory.clone());

        // When
        store
            .update_download(custom_download_directory.clone(), false)
            .await
            .expect("save download settings");
        let reloaded = SettingsStore::load(path, default_download_directory);

        // Then
        assert_eq!(
            reloaded.download_policy().await,
            (custom_download_directory, false)
        );
        std::fs::remove_dir_all(directory).expect("remove settings directory");
    }

    #[tokio::test]
    async fn rejects_a_download_destination_that_is_not_a_directory() {
        // Given
        let directory =
            std::env::temp_dir().join(format!("koofr-settings-{}", uuid::Uuid::new_v4()));
        let default_download_directory = directory.join("Downloads");
        std::fs::create_dir_all(&directory).expect("create settings directory");
        let file_path = directory.join("not-a-directory.txt");
        std::fs::write(&file_path, b"not a directory").expect("create test file");
        let store =
            SettingsStore::load(directory.join("settings.json"), default_download_directory);

        // When
        let result = store.update_download(file_path, true).await;

        // Then
        assert!(result.is_err());
        std::fs::remove_dir_all(directory).expect("remove settings directory");
    }
}
