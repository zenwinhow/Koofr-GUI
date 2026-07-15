use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use serde::Serialize;

use crate::error::AppError;

const MAX_PENDING_GRANTS: usize = 64;

#[derive(Debug)]
enum LocalPathGrant {
    Upload(PathBuf),
    Download(PathBuf),
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
}

impl LocalAccessManager {
    pub fn grant_upload(&self, path: PathBuf) -> Result<LocalFileSelection, AppError> {
        self.insert(path, true)
    }

    pub fn grant_download(&self, path: PathBuf) -> Result<LocalFileSelection, AppError> {
        self.insert(path, false)
    }

    fn insert(&self, path: PathBuf, is_upload: bool) -> Result<LocalFileSelection, AppError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or(AppError::InvalidInput("selected local file name"))?;
        let grant_id = uuid::Uuid::new_v4().to_string();
        let grant = if is_upload {
            LocalPathGrant::Upload(path)
        } else {
            LocalPathGrant::Download(path)
        };
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
            LocalPathGrant::Download(_) => Err(AppError::InvalidInput("upload path grant")),
        }
    }

    pub fn take_download(&self, grant_id: &str) -> Result<PathBuf, AppError> {
        match self.take(grant_id)? {
            LocalPathGrant::Download(path) => Ok(path),
            LocalPathGrant::Upload(_) => Err(AppError::InvalidInput("download path grant")),
        }
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
            .grant_upload(PathBuf::from("C:\\files\\report.txt"))
            .expect("create upload grant");
        assert_eq!(
            manager.take_upload(&selected.grant_id).expect("use grant"),
            PathBuf::from("C:\\files\\report.txt")
        );
        assert!(manager.take_upload(&selected.grant_id).is_err());
    }
}
