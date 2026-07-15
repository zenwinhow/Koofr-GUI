use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrInteger {
    String(String),
    Signed(i64),
    Unsigned(u64),
}

impl StringOrInteger {
    fn into_string(self) -> String {
        match self {
            Self::String(value) => value,
            Self::Signed(value) => value.to_string(),
            Self::Unsigned(value) => value.to_string(),
        }
    }
}

fn deserialize_string_or_integer<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    StringOrInteger::deserialize(deserializer).map(StringOrInteger::into_string)
}

fn deserialize_optional_string_or_integer<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<StringOrInteger>::deserialize(deserializer)
        .map(|value| value.map(StringOrInteger::into_string))
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatedFile {
    pub mount_id: String,
    pub mount_name: String,
    pub name: String,
    pub entry_type: String,
    pub modified: i64,
    pub size: i64,
    pub content_type: String,
    pub hash: String,
    pub path: String,
    pub share_direction: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub version_id: String,
    pub mount_id: String,
    pub mount_name: String,
    pub path: String,
    pub name: String,
    pub deleted: String,
    pub size: i64,
    pub content_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashList {
    pub items: Vec<TrashItem>,
    pub retention_days: i64,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchHit {
    pub mount_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub modified: i64,
    pub size: i64,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub hash: String,
    pub path: String,
    #[serde(default)]
    pub mount: Option<Mount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchResponse {
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    pub mounts: HashMap<String, Mount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SharedRemote {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub modified: i64,
    pub size: i64,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub hash: String,
    pub mount: Mount,
}

#[derive(Debug, Deserialize)]
pub(super) struct SharedListResponse {
    pub files: Vec<SharedRemote>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrashRemote {
    #[serde(
        alias = "versionId",
        deserialize_with = "deserialize_string_or_integer"
    )]
    pub id: String,
    pub mount_id: String,
    pub path: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_string_or_integer")]
    pub deleted: String,
    pub size: i64,
    #[serde(default)]
    pub content_type: String,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct PageInfo {
    #[serde(default, deserialize_with = "deserialize_optional_string_or_integer")]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrashListResponse {
    pub files: Vec<TrashRemote>,
    #[serde(default)]
    pub mounts: HashMap<String, Mount>,
    #[serde(default)]
    pub retention_days: i64,
    #[serde(default)]
    pub page_info: PageInfo,
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
