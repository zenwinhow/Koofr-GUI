mod native_prompt;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::Serialize;
use tokio::sync::RwLock;

use crate::{
    crypto::VaultCipher,
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath},
    koofr_api::{KoofrApi, VaultRepo},
};

pub use native_prompt::prompt_safe_key;

const ROOT_HANDLE: &str = "root";
const DEFAULT_AUTO_LOCK: Duration = Duration::from_secs(60 * 60);
const MAX_HANDLES_PER_REPO: usize = 4096;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSummary {
    pub id: String,
    pub name: String,
    pub locked: bool,
    pub added: i64,
    pub auto_lock_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultBreadcrumb {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntry {
    pub id: String,
    pub name: String,
    pub entry_type: String,
    pub modified: i64,
    pub size: i64,
    pub content_type: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDirectory {
    pub repo_id: String,
    pub repo_name: String,
    pub directory_id: String,
    pub breadcrumbs: Vec<VaultBreadcrumb>,
    pub entries: Vec<VaultEntry>,
}

#[derive(Clone)]
pub struct VaultManager {
    state: Arc<RwLock<VaultState>>,
    auto_lock_after: Duration,
}

#[derive(Default)]
struct VaultState {
    repos: HashMap<String, ManagedRepo>,
}

struct ManagedRepo {
    metadata: VaultRepo,
    unlocked: Option<UnlockedRepo>,
}

struct UnlockedRepo {
    cipher: Arc<VaultCipher>,
    handles: HashMap<String, VaultHandle>,
    last_activity: Instant,
}

#[derive(Clone)]
struct VaultHandle {
    encrypted_path: RemotePath,
    entry_type: HandleType,
    breadcrumbs: Vec<VaultBreadcrumb>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum HandleType {
    File,
    Directory,
}

impl Default for VaultManager {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(VaultState::default())),
            auto_lock_after: DEFAULT_AUTO_LOCK,
        }
    }
}

impl VaultManager {
    pub async fn reset(&self) {
        *self.state.write().await = VaultState::default();
    }

    pub async fn sync_repos(&self, repos: Vec<VaultRepo>) -> Vec<VaultSummary> {
        let mut state = self.state.write().await;
        prune_expired(&mut state, self.auto_lock_after);
        let mut previous = std::mem::take(&mut state.repos);
        for repo in repos {
            let unlocked = previous
                .remove(&repo.id)
                .filter(|old| same_crypto_identity(&old.metadata, &repo))
                .and_then(|old| old.unlocked);
            state.repos.insert(
                repo.id.clone(),
                ManagedRepo {
                    metadata: repo,
                    unlocked,
                },
            );
        }
        summaries(&state, self.auto_lock_after)
    }

    pub async fn summaries(&self) -> Vec<VaultSummary> {
        let mut state = self.state.write().await;
        prune_expired(&mut state, self.auto_lock_after);
        summaries(&state, self.auto_lock_after)
    }

    pub async fn repo_for_unlock(&self, repo_id: &str) -> Result<VaultRepo, AppError> {
        validate_repo_id(repo_id)?;
        self.state
            .read()
            .await
            .repos
            .get(repo_id)
            .map(|repo| repo.metadata.clone())
            .ok_or(AppError::NotFound)
    }

    pub async fn unlock(&self, repo_id: &str, safe_key: &str) -> Result<VaultSummary, AppError> {
        validate_repo_id(repo_id)?;
        let repo = self.repo_for_unlock(repo_id).await?;
        let cipher = VaultCipher::new(safe_key, repo.salt.as_deref());
        if !cipher.validates(&repo.password_validator, &repo.password_validator_encrypted)? {
            return Err(AppError::VaultInvalidKey);
        }
        let mut handles = HashMap::new();
        handles.insert(
            ROOT_HANDLE.to_owned(),
            VaultHandle {
                encrypted_path: RemotePath::parse("/".to_owned())?,
                entry_type: HandleType::Directory,
                breadcrumbs: vec![VaultBreadcrumb {
                    id: ROOT_HANDLE.to_owned(),
                    name: repo.name.clone(),
                }],
            },
        );
        let mut state = self.state.write().await;
        let managed = state.repos.get_mut(repo_id).ok_or(AppError::NotFound)?;
        managed.unlocked = Some(UnlockedRepo {
            cipher: Arc::new(cipher),
            handles,
            last_activity: Instant::now(),
        });
        Ok(summary(managed, self.auto_lock_after))
    }

    pub async fn lock(&self, repo_id: &str) -> Result<(), AppError> {
        validate_repo_id(repo_id)?;
        let mut state = self.state.write().await;
        let repo = state.repos.get_mut(repo_id).ok_or(AppError::NotFound)?;
        repo.unlocked = None;
        Ok(())
    }

    pub async fn lock_expired(&self) {
        let mut state = self.state.write().await;
        prune_expired(&mut state, self.auto_lock_after);
    }

    pub async fn list_directory(
        &self,
        api: &KoofrApi,
        repo_id: &str,
        directory_id: &str,
    ) -> Result<VaultDirectory, AppError> {
        let (repo, cipher, parent) = self
            .operation_snapshot(repo_id, directory_id, Some(HandleType::Directory))
            .await?;
        let mount_id = MountId::parse(repo.mount_id.clone())?;
        let remote_directory = mount_path(&repo.path, &parent.encrypted_path)?;
        let files = api.list_files(&mount_id, &remote_directory).await?;
        let mut entries = Vec::with_capacity(files.len());
        let mut new_handles = Vec::with_capacity(files.len());
        for file in files {
            let name = cipher.decrypt_name(&file.name)?;
            RemoteName::parse(name.clone())?;
            let encrypted_name = RemoteName::parse(file.name)?;
            let encrypted_path = parent.encrypted_path.join(&encrypted_name)?;
            let entry_type = if file.entry_type == "dir" {
                HandleType::Directory
            } else if file.entry_type == "file" {
                HandleType::File
            } else {
                return Err(AppError::VaultCrypto);
            };
            let id = uuid::Uuid::new_v4().to_string();
            let mut breadcrumbs = parent.breadcrumbs.clone();
            if entry_type == HandleType::Directory {
                breadcrumbs.push(VaultBreadcrumb {
                    id: id.clone(),
                    name: name.clone(),
                });
            }
            let size = if entry_type == HandleType::File {
                VaultCipher::decrypted_size(file.size)?
            } else {
                0
            };
            entries.push(VaultEntry {
                id: id.clone(),
                name,
                entry_type: file.entry_type,
                modified: file.modified,
                size,
                content_type: file.content_type,
            });
            new_handles.push((
                id,
                VaultHandle {
                    encrypted_path,
                    entry_type,
                    breadcrumbs,
                },
            ));
        }
        entries.sort_by(|left, right| {
            let left_dir = left.entry_type == "dir";
            let right_dir = right.entry_type == "dir";
            right_dir
                .cmp(&left_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        self.install_handles(repo_id, new_handles).await?;
        Ok(VaultDirectory {
            repo_id: repo.id,
            repo_name: repo.name,
            directory_id: directory_id.to_owned(),
            breadcrumbs: parent.breadcrumbs,
            entries,
        })
    }

    pub async fn create_directory(
        &self,
        api: &KoofrApi,
        repo_id: &str,
        parent_id: &str,
        name: String,
    ) -> Result<(), AppError> {
        let name = RemoteName::parse(name)?;
        let (repo, cipher, parent) = self
            .operation_snapshot(repo_id, parent_id, Some(HandleType::Directory))
            .await?;
        let encrypted_name = RemoteName::parse(cipher.encrypt_name(name.as_str()))?;
        let mount_id = MountId::parse(repo.mount_id)?;
        let remote_parent = mount_path(&repo.path, &parent.encrypted_path)?;
        api.create_folder(&mount_id, &remote_parent, &encrypted_name)
            .await
    }

    pub async fn rename(
        &self,
        api: &KoofrApi,
        repo_id: &str,
        entry_id: &str,
        new_name: String,
    ) -> Result<(), AppError> {
        let new_name = RemoteName::parse(new_name)?;
        let (repo, cipher, entry) = self.operation_snapshot(repo_id, entry_id, None).await?;
        if entry.encrypted_path.as_str() == "/" {
            return Err(AppError::InvalidInput("vault root rename"));
        }
        let encrypted_name = RemoteName::parse(cipher.encrypt_name(new_name.as_str()))?;
        let parent = entry.encrypted_path.parent()?;
        let destination_relative = parent.join(&encrypted_name)?;
        let mount_id = MountId::parse(repo.mount_id)?;
        let source = mount_path(&repo.path, &entry.encrypted_path)?;
        let destination = mount_path(&repo.path, &destination_relative)?;
        api.move_to(&mount_id, &source, &mount_id, &destination)
            .await
    }

    pub async fn relocate(
        &self,
        api: &KoofrApi,
        repo_id: &str,
        entry_id: &str,
        destination_id: &str,
        is_move: bool,
    ) -> Result<(), AppError> {
        let (repo, _cipher, entry) = self.operation_snapshot(repo_id, entry_id, None).await?;
        let (_, _, destination) = self
            .operation_snapshot(repo_id, destination_id, Some(HandleType::Directory))
            .await?;
        if entry.encrypted_path.as_str() == "/" {
            return Err(AppError::InvalidInput("vault root relocate"));
        }
        let name = RemoteName::parse(entry.encrypted_path.file_name()?.to_owned())?;
        let destination_relative = destination.encrypted_path.join(&name)?;
        if entry.entry_type == HandleType::Directory
            && destination_relative
                .as_str()
                .starts_with(&format!("{}/", entry.encrypted_path.as_str()))
        {
            return Err(AppError::InvalidInput("vault move into self"));
        }
        let mount_id = MountId::parse(repo.mount_id)?;
        let source = mount_path(&repo.path, &entry.encrypted_path)?;
        let target = mount_path(&repo.path, &destination_relative)?;
        if is_move {
            api.move_to(&mount_id, &source, &mount_id, &target).await
        } else {
            api.copy_to(&mount_id, &source, &mount_id, &target).await
        }
    }

    pub async fn delete(
        &self,
        api: &KoofrApi,
        repo_id: &str,
        entry_ids: &[String],
    ) -> Result<(), AppError> {
        if entry_ids.is_empty() || entry_ids.len() > 256 {
            return Err(AppError::InvalidInput("vault delete selection"));
        }
        for entry_id in entry_ids {
            let (repo, _, entry) = self.operation_snapshot(repo_id, entry_id, None).await?;
            if entry.encrypted_path.as_str() == "/" {
                return Err(AppError::InvalidInput("vault root delete"));
            }
            let mount_id = MountId::parse(repo.mount_id)?;
            let remote_path = mount_path(&repo.path, &entry.encrypted_path)?;
            api.delete(&mount_id, &remote_path).await?;
        }
        Ok(())
    }

    pub async fn operation_target(
        &self,
        repo_id: &str,
        entry_id: &str,
        expected: Option<&str>,
    ) -> Result<(VaultRepo, Arc<VaultCipher>, RemotePath), AppError> {
        let expected = match expected {
            Some("file") => Some(HandleType::File),
            Some("dir") => Some(HandleType::Directory),
            Some(_) => return Err(AppError::InvalidInput("vault entry type")),
            None => None,
        };
        let (repo, cipher, handle) = self.operation_snapshot(repo_id, entry_id, expected).await?;
        let path = mount_path(&repo.path, &handle.encrypted_path)?;
        Ok((repo, cipher, path))
    }

    pub async fn resume_download_target(
        &self,
        repo_id: &str,
        mount_id: &str,
        remote_path: &str,
    ) -> Result<Arc<VaultCipher>, AppError> {
        validate_repo_id(repo_id)?;
        let mount_id = MountId::parse(mount_id.to_owned())?;
        let remote_path = RemotePath::parse(remote_path.to_owned())?;
        let mut state = self.state.write().await;
        prune_expired(&mut state, self.auto_lock_after);
        let repo = state.repos.get_mut(repo_id).ok_or(AppError::NotFound)?;
        if repo.metadata.mount_id != mount_id.as_str() {
            return Err(AppError::InvalidInput("vault checkpoint mount"));
        }
        let base = RemotePath::parse(repo.metadata.path.clone())?;
        let inside = remote_path.as_str() == base.as_str()
            || (base.as_str() == "/" && remote_path.as_str().starts_with('/'))
            || remote_path
                .as_str()
                .strip_prefix(base.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'));
        if !inside {
            return Err(AppError::InvalidInput("vault checkpoint path"));
        }
        let unlocked = repo.unlocked.as_mut().ok_or(AppError::VaultLocked)?;
        unlocked.last_activity = Instant::now();
        Ok(unlocked.cipher.clone())
    }

    async fn operation_snapshot(
        &self,
        repo_id: &str,
        handle_id: &str,
        expected_type: Option<HandleType>,
    ) -> Result<(VaultRepo, Arc<VaultCipher>, VaultHandle), AppError> {
        validate_repo_id(repo_id)?;
        validate_handle_id(handle_id)?;
        let mut state = self.state.write().await;
        prune_expired(&mut state, self.auto_lock_after);
        let repo = state.repos.get_mut(repo_id).ok_or(AppError::NotFound)?;
        let unlocked = repo.unlocked.as_mut().ok_or(AppError::VaultLocked)?;
        let handle = unlocked
            .handles
            .get(handle_id)
            .cloned()
            .ok_or(AppError::NotFound)?;
        if expected_type.is_some_and(|expected| handle.entry_type != expected) {
            return Err(AppError::InvalidInput("vault entry type"));
        }
        unlocked.last_activity = Instant::now();
        Ok((repo.metadata.clone(), unlocked.cipher.clone(), handle))
    }

    async fn install_handles(
        &self,
        repo_id: &str,
        handles: Vec<(String, VaultHandle)>,
    ) -> Result<(), AppError> {
        let mut state = self.state.write().await;
        let repo = state.repos.get_mut(repo_id).ok_or(AppError::NotFound)?;
        let unlocked = repo.unlocked.as_mut().ok_or(AppError::VaultLocked)?;
        if unlocked.handles.len().saturating_add(handles.len()) > MAX_HANDLES_PER_REPO {
            unlocked.handles.retain(|id, _| id == ROOT_HANDLE);
        }
        unlocked.handles.extend(handles);
        unlocked.last_activity = Instant::now();
        Ok(())
    }
}

fn same_crypto_identity(left: &VaultRepo, right: &VaultRepo) -> bool {
    left.mount_id == right.mount_id
        && left.path == right.path
        && left.salt == right.salt
        && left.password_validator == right.password_validator
        && left.password_validator_encrypted == right.password_validator_encrypted
}

fn prune_expired(state: &mut VaultState, after: Duration) {
    let now = Instant::now();
    for repo in state.repos.values_mut() {
        let expired = repo
            .unlocked
            .as_ref()
            .is_some_and(|unlocked| now.duration_since(unlocked.last_activity) >= after);
        if expired {
            repo.unlocked = None;
        }
    }
}

fn summaries(state: &VaultState, after: Duration) -> Vec<VaultSummary> {
    let mut values = state
        .repos
        .values()
        .map(|repo| summary(repo, after))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.added
            .cmp(&right.added)
            .then_with(|| left.name.cmp(&right.name))
    });
    values
}

fn summary(repo: &ManagedRepo, after: Duration) -> VaultSummary {
    VaultSummary {
        id: repo.metadata.id.clone(),
        name: repo.metadata.name.clone(),
        locked: repo.unlocked.is_none(),
        added: repo.metadata.added,
        auto_lock_seconds: after.as_secs(),
    }
}

fn mount_path(base: &str, relative: &RemotePath) -> Result<RemotePath, AppError> {
    let base = RemotePath::parse(base.to_owned())?;
    if relative.as_str() == "/" {
        return Ok(base);
    }
    let joined = if base.as_str() == "/" {
        relative.as_str().to_owned()
    } else {
        format!("{}{}", base.as_str(), relative.as_str())
    };
    RemotePath::parse(joined)
}

fn validate_repo_id(value: &str) -> Result<(), AppError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        Ok(())
    } else {
        Err(AppError::InvalidInput("vault repo id"))
    }
}

fn validate_handle_id(value: &str) -> Result<(), AppError> {
    if value == ROOT_HANDLE || uuid::Uuid::parse_str(value).is_ok() {
        Ok(())
    } else {
        Err(AppError::InvalidInput("vault handle"))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{VaultManager, mount_path};
    use crate::file_ops::RemotePath;
    use crate::{error::AppError, koofr_api::VaultRepo};

    fn test_repo() -> VaultRepo {
        VaultRepo {
            id: "repo-1".to_owned(),
            name: "Safe Box".to_owned(),
            mount_id: "mount-1".to_owned(),
            path: "/Safe Box".to_owned(),
            salt: None,
            password_validator: "508ddd3f-f18e-4514-932b-b2c1f0c8b291".to_owned(),
            password_validator_encrypted: "v2:UkNMT05FAAA-YjvGKKxTpiFekFYVMNO2UnG2u-Z16MMHAB-ipQYycVTmPSNk0mbnYeZrZ2I-Kh0lTmh4Kt2UxhdYWEXd9YQvyODrWMWWHZaLhL7e".to_owned(),
            added: 1,
        }
    }

    #[test]
    fn joins_opaque_relative_paths_under_the_registered_safe_box() {
        let relative = RemotePath::parse("/encrypted/child".to_owned()).expect("relative path");
        assert_eq!(
            mount_path("/Safe Box", &relative)
                .expect("mount path")
                .as_str(),
            "/Safe Box/encrypted/child"
        );
        assert_eq!(
            mount_path("/", &relative)
                .expect("root mount path")
                .as_str(),
            "/encrypted/child"
        );
    }

    #[tokio::test]
    async fn unlocks_only_with_a_valid_safe_key_and_locks_explicitly() {
        let manager = VaultManager::default();
        manager.sync_repos(vec![test_repo()]).await;
        assert!(matches!(
            manager.unlock("repo-1", "wrong").await,
            Err(AppError::VaultInvalidKey)
        ));
        let unlocked = manager
            .unlock("repo-1", "testpassword")
            .await
            .expect("unlock valid key");
        assert!(!unlocked.locked);
        manager.lock("repo-1").await.expect("lock");
        assert!(manager.summaries().await[0].locked);
    }

    #[tokio::test]
    async fn expires_idle_unlocked_sessions() {
        let manager = VaultManager {
            auto_lock_after: Duration::from_millis(1),
            ..VaultManager::default()
        };
        manager.sync_repos(vec![test_repo()]).await;
        manager
            .unlock("repo-1", "testpassword")
            .await
            .expect("unlock valid key");
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(manager.summaries().await[0].locked);
    }
}
