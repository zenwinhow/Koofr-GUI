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
    #[serde(rename(deserialize = "type", serialize = "mountType"))]
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
    #[serde(rename(deserialize = "type", serialize = "entryType"))]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{FileInfo, Mount};

    #[test]
    fn maps_remote_type_fields_to_explicit_frontend_names() {
        let mount: Mount = serde_json::from_value(json!({
            "id": "mount_1",
            "name": "Koofr",
            "type": "device"
        }))
        .expect("decode mount");
        let file: FileInfo = serde_json::from_value(json!({
            "name": "资料",
            "type": "dir",
            "modified": 123,
            "size": 0
        }))
        .expect("decode file");

        let mount_frontend = serde_json::to_value(mount).expect("encode mount");
        let file_frontend = serde_json::to_value(file).expect("encode file");
        assert_eq!(mount_frontend["mountType"], "device");
        assert!(mount_frontend.get("type").is_none());
        assert_eq!(file_frontend["entryType"], "dir");
        assert!(file_frontend.get("type").is_none());
    }
}
