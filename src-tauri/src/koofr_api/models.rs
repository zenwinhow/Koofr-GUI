use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub authenticated: bool,
    pub user_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mount {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub mount_type: String,
    #[serde(default)]
    pub space_total: i64,
    #[serde(default)]
    pub space_used: i64,
    #[serde(default)]
    pub online: bool,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub is_shared: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub modified: i64,
    pub size: i64,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct TokenResponse {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct MountListResponse {
    pub mounts: Vec<Mount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FileListResponse {
    pub files: Vec<FileInfo>,
}
