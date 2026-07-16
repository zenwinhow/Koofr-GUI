use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use serde::Serialize;

use crate::error::AppError;

const MAX_PENDING_GRANTS: usize = 64;
const MAX_COMPLETED_DOWNLOADS: usize = 100;

#[derive(Debug)]
enum LocalPathGrant {
    Upload(PathBuf),
    Download(PathBuf),
    DownloadDirectory(PathBuf),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileSelection {
    pub grant_id: String,
    pub file_name: String,
}

#[derive(Default)]
pub struct LocalAccessManager {
    grants: Mutex<HashMap<String, LocalPathGrant>>,
    completed_downloads: Mutex<HashMap<String, PathBuf>>,
}

impl LocalAccessManager {
    pub fn grant_upload(&self, path: PathBuf) -> Result<LocalFileSelection, AppError> {
        self.insert(LocalPathGrant::Upload(path))
    }

    pub fn grant_download(&self, path: PathBuf) -> Result<LocalFileSelection, AppError> {
        self.insert(LocalPathGrant::Download(path))
    }

    pub fn grant_download_directory(&self, path: PathBuf) -> Result<LocalFileSelection, AppError> {
        self.insert(LocalPathGrant::DownloadDirectory(path))
    }

    fn insert(&self, grant: LocalPathGrant) -> Result<LocalFileSelection, AppError> {
        let path = match &grant {
            LocalPathGrant::Upload(path)
            | LocalPathGrant::Download(path)
            | LocalPathGrant::DownloadDirectory(path) => path,
        };
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or(AppError::InvalidInput("selected local file name"))?;
        let grant_id = uuid::Uuid::new_v4().to_string();
        let mut grants = self.grants.lock().expect("local path grant store poisoned");
        if grants.len() >= MAX_PENDING_GRANTS {
            grants.clear();
        }
        grants.insert(grant_id.clone(), grant);
        Ok(LocalFileSelection {
            grant_id,
            file_name,
        })
    }

    pub fn take_upload(&self, grant_id: &str) -> Result<PathBuf, AppError> {
        match self.take(grant_id)? {
            LocalPathGrant::Upload(path) => Ok(path),
            LocalPathGrant::Download(_) | LocalPathGrant::DownloadDirectory(_) => {
                Err(AppError::InvalidInput("upload path grant"))
            }
        }
    }

    pub fn take_download(&self, grant_id: &str) -> Result<PathBuf, AppError> {
        match self.take(grant_id)? {
            LocalPathGrant::Download(path) => Ok(path),
            LocalPathGrant::Upload(_) | LocalPathGrant::DownloadDirectory(_) => {
                Err(AppError::InvalidInput("download path grant"))
            }
        }
    }

    pub fn take_download_directory(&self, grant_id: &str) -> Result<PathBuf, AppError> {
        match self.take(grant_id)? {
            LocalPathGrant::DownloadDirectory(path) => Ok(path),
            LocalPathGrant::Upload(_) | LocalPathGrant::Download(_) => {
                Err(AppError::InvalidInput("download directory path grant"))
            }
        }
    }

    pub fn remember_download(&self, transfer_id: &str, path: PathBuf) -> Result<(), AppError> {
        validate_grant_id(transfer_id)?;
        let mut downloads = self
            .completed_downloads
            .lock()
            .expect("completed download store poisoned");
        if downloads.len() >= MAX_COMPLETED_DOWNLOADS {
            downloads.clear();
        }
        downloads.insert(transfer_id.to_owned(), path);
        Ok(())
    }

    pub fn completed_download(&self, transfer_id: &str) -> Result<PathBuf, AppError> {
        validate_grant_id(transfer_id)?;
        self.completed_downloads
            .lock()
            .expect("completed download store poisoned")
            .get(transfer_id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    fn take(&self, grant_id: &str) -> Result<LocalPathGrant, AppError> {
        validate_grant_id(grant_id)?;
        self.grants
            .lock()
            .expect("local path grant store poisoned")
            .remove(grant_id)
            .ok_or(AppError::InvalidInput("expired local path grant"))
    }

    pub fn clear(&self) {
        self.grants
            .lock()
            .expect("local path grant store poisoned")
            .clear();
        self.completed_downloads
            .lock()
            .expect("completed download store poisoned")
            .clear();
    }
}

fn validate_grant_id(grant_id: &str) -> Result<(), AppError> {
    let parsed = uuid::Uuid::parse_str(grant_id)
        .map_err(|_| AppError::InvalidInput("local path grant id"))?;
    if parsed.is_nil() {
        return Err(AppError::InvalidInput("local path grant id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::LocalAccessManager;

    #[test]
    fn grants_are_one_shot_and_direction_scoped() {
        let manager = LocalAccessManager::default();
        let selected = manager
            .grant_upload(PathBuf::from("C:\\files\\report.txt"))
            .expect("create upload grant");
        assert!(manager.take_download(&selected.grant_id).is_err());
        assert!(manager.take_upload(&selected.grant_id).is_err());

        let selected = manager
            .grant_download_directory(PathBuf::from("C:\\files\\reports"))
            .expect("create directory grant");
        assert!(manager.take_download(&selected.grant_id).is_err());
        assert!(manager.take_download_directory(&selected.grant_id).is_err());

        let selected = manager
            .grant_upload(PathBuf::from("C:\\files\\report.txt"))
            .expect("create upload grant");
        assert_eq!(
            manager.take_upload(&selected.grant_id).expect("use grant"),
            PathBuf::from("C:\\files\\report.txt")
        );
        assert!(manager.take_upload(&selected.grant_id).is_err());
    }
}
