mod models;

use std::{collections::HashSet, time::Duration};

use reqwest::{Client, Method, Response, StatusCode, Url, header};
use serde::Serialize;
use tokio::sync::RwLock;
use zeroize::Zeroize;

use crate::{
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath},
};

pub use models::{FileInfo, LocatedFile, Mount, SessionInfo, TrashItem, TrashList};
use models::{
    FileListResponse, MountListResponse, SearchResponse, SharedListResponse, TokenResponse,
    TrashListResponse,
};

const USER_AGENT: &str = concat!("Koofr-GUI/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
struct Session {
    token: String,
    user_id: Option<String>,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

pub struct KoofrApi {
    base_url: Url,
    client: Client,
    session: RwLock<Option<Session>>,
}

impl KoofrApi {
    pub fn production() -> Result<Self, AppError> {
        Self::new("https://app.koofr.net")
    }

    fn new(base_url: &str) -> Result<Self, AppError> {
        let base_url = Url::parse(base_url).map_err(|_| AppError::Initialization)?;
        if base_url.scheme() != "https" && !cfg!(test) {
            return Err(AppError::Initialization);
        }
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| AppError::Initialization)?;
        Ok(Self {
            base_url,
            client,
            session: RwLock::new(None),
        })
    }

    pub async fn authenticate(&self, email: &str, password: &str) -> Result<SessionInfo, AppError> {
        if email.trim().is_empty() || password.is_empty() {
            return Err(AppError::InvalidInput("credentials"));
        }
        self.disconnect().await;
        #[derive(Serialize)]
        struct Credentials<'a> {
            email: &'a str,
            password: &'a str,
        }

        let url = self.endpoint(&["token"])?;
        let response = self
            .client
            .post(url)
            .json(&Credentials { email, password })
            .send()
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(AppError::AuthenticationFailed);
        }
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let user_id = response
            .headers()
            .get("x-user-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = response.bytes().await?;
        let token: TokenResponse = serde_json::from_slice(&bytes).map_err(AppError::Decode)?;
        if token.token.is_empty() {
            return Err(AppError::AuthenticationFailed);
        }

        let info = SessionInfo {
            authenticated: true,
            user_id: user_id.clone(),
        };
        *self.session.write().await = Some(Session {
            token: token.token,
            user_id,
        });
        Ok(info)
    }

    pub async fn disconnect(&self) {
        *self.session.write().await = None;
    }

    pub async fn session_info(&self) -> SessionInfo {
        let session = self.session.read().await;
        SessionInfo {
            authenticated: session.is_some(),
            user_id: session.as_ref().and_then(|session| session.user_id.clone()),
        }
    }

    pub async fn list_mounts(&self) -> Result<Vec<Mount>, AppError> {
        let url = self.endpoint(&["api", "v2", "mounts"])?;
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let payload: MountListResponse = Self::decode_json(response).await?;
        Ok(payload.mounts)
    }

    pub async fn list_files(
        &self,
        mount_id: &MountId,
        path: &RemotePath,
    ) -> Result<Vec<FileInfo>, AppError> {
        let mut url = self.mount_endpoint(mount_id, &["files", "list"])?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let mut payload: FileListResponse = Self::decode_json(response).await?;
        for file in &mut payload.files {
            file.path = path
                .join(&RemoteName::parse(file.name.clone())?)?
                .as_str()
                .to_owned();
        }
        Ok(payload.files)
    }

    pub async fn list_recent(&self) -> Result<Vec<LocatedFile>, AppError> {
        let mut url = self.endpoint(&["api", "v2", "search"])?;
        url.query_pairs_mut()
            .append_pair("limit", "1000")
            .append_pair("offset", "0")
            .append_pair("sortField", "mtime")
            .append_pair("sortDir", "desc")
            .append_pair("mountId", "")
            .append_pair("path", "");
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let payload: SearchResponse = Self::decode_json(response).await?;

        Ok(payload
            .hits
            .into_iter()
            .map(|hit| {
                let mount_name = hit
                    .mount
                    .as_ref()
                    .or_else(|| payload.mounts.get(&hit.mount_id))
                    .map(|mount| mount.name.clone())
                    .unwrap_or_default();
                LocatedFile {
                    mount_id: hit.mount_id,
                    mount_name,
                    name: hit.name,
                    entry_type: hit.entry_type,
                    modified: hit.modified,
                    size: hit.size,
                    content_type: hit.content_type,
                    hash: hit.hash,
                    path: hit.path,
                    share_direction: None,
                }
            })
            .collect())
    }

    pub async fn list_shared(&self) -> Result<Vec<LocatedFile>, AppError> {
        let url = self.endpoint(&["api", "v2", "shared"])?;
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let payload: SharedListResponse = Self::decode_json(response).await?;

        Ok(payload
            .files
            .into_iter()
            .map(|file| {
                let share_direction = if file.mount.is_shared {
                    "received"
                } else {
                    "outgoing"
                };
                LocatedFile {
                    mount_id: file.mount.id,
                    mount_name: file.mount.name,
                    name: file.name,
                    entry_type: file.entry_type,
                    modified: file.modified,
                    size: file.size,
                    content_type: file.content_type,
                    hash: file.hash,
                    path: "/".to_owned(),
                    share_direction: Some(share_direction.to_owned()),
                }
            })
            .collect())
    }

    pub async fn list_trash(&self) -> Result<TrashList, AppError> {
        let mut cursor: Option<String> = None;
        let mut seen_cursors = HashSet::new();
        let mut items = Vec::new();
        let mut retention_days = 0;

        loop {
            let mut url = self.endpoint(&["api", "v2", "trash"])?;
            if let Some(current_cursor) = cursor.as_deref() {
                url.query_pairs_mut().append_pair("cursor", current_cursor);
            } else {
                url.query_pairs_mut()
                    .append_pair("pageSize", "1000")
                    .append_pair("sortField", "deleted")
                    .append_pair("sortDir", "desc");
            }
            let response = self
                .authenticated_request(Method::GET, url)
                .await?
                .send()
                .await?;
            let response = Self::expect_status(response, &[StatusCode::OK])?;
            let payload: TrashListResponse = Self::decode_json(response).await?;
            if retention_days == 0 {
                retention_days = payload.retention_days;
            }
            items.extend(payload.files.into_iter().map(|file| {
                TrashItem {
                    mount_name: payload
                        .mounts
                        .get(&file.mount_id)
                        .map(|mount| mount.name.clone())
                        .unwrap_or_default(),
                    version_id: file.id,
                    mount_id: file.mount_id,
                    path: file.path,
                    name: file.name,
                    deleted: file.deleted,
                    size: file.size,
                    content_type: file.content_type,
                }
            }));

            let Some(next_cursor) = payload.page_info.cursor else {
                break;
            };
            if !seen_cursors.insert(next_cursor.clone()) {
                break;
            }
            cursor = Some(next_cursor);
        }

        Ok(TrashList {
            items,
            retention_days,
        })
    }

    pub async fn restore_trash(&self, files: &[(MountId, RemotePath)]) -> Result<(), AppError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct TrashFile<'a> {
            mount_id: &'a str,
            path: &'a str,
        }
        #[derive(Serialize)]
        struct RestoreRequest<'a> {
            files: Vec<TrashFile<'a>>,
        }

        let request = RestoreRequest {
            files: files
                .iter()
                .map(|(mount_id, path)| TrashFile {
                    mount_id: mount_id.as_str(),
                    path: path.as_str(),
                })
                .collect(),
        };
        let url = self.endpoint(&["api", "v2", "trash", "undelete"])?;
        let response = self
            .authenticated_request(Method::POST, url)
            .await?
            .json(&request)
            .send()
            .await?;
        Self::expect_status(
            response,
            &[
                StatusCode::OK,
                StatusCode::CREATED,
                StatusCode::ACCEPTED,
                StatusCode::NO_CONTENT,
            ],
        )?;
        Ok(())
    }

    pub async fn empty_trash(&self) -> Result<(), AppError> {
        let url = self.endpoint(&["api", "v2", "trash"])?;
        let response = self
            .authenticated_request(Method::DELETE, url)
            .await?
            .send()
            .await?;
        Self::expect_status(
            response,
            &[StatusCode::OK, StatusCode::ACCEPTED, StatusCode::NO_CONTENT],
        )?;
        Ok(())
    }

    pub async fn create_folder(
        &self,
        mount_id: &MountId,
        parent: &RemotePath,
        name: &RemoteName,
    ) -> Result<(), AppError> {
        #[derive(Serialize)]
        struct Folder<'a> {
            name: &'a str,
        }
        let mut url = self.mount_endpoint(mount_id, &["files", "folder"])?;
        url.query_pairs_mut().append_pair("path", parent.as_str());
        let response = self
            .authenticated_request(Method::POST, url)
            .await?
            .json(&Folder {
                name: name.as_str(),
            })
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK, StatusCode::CREATED])?;
        Ok(())
    }

    pub async fn delete(&self, mount_id: &MountId, path: &RemotePath) -> Result<(), AppError> {
        let mut url = self.mount_endpoint(mount_id, &["files", "remove"])?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        let response = self
            .authenticated_request(Method::DELETE, url)
            .await?
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK])?;
        Ok(())
    }

    pub async fn move_to(
        &self,
        mount_id: &MountId,
        path: &RemotePath,
        destination_mount_id: &MountId,
        destination_path: &RemotePath,
    ) -> Result<(), AppError> {
        self.relocate(
            "move",
            mount_id,
            path,
            destination_mount_id,
            destination_path,
        )
        .await
    }

    pub async fn copy_to(
        &self,
        mount_id: &MountId,
        path: &RemotePath,
        destination_mount_id: &MountId,
        destination_path: &RemotePath,
    ) -> Result<(), AppError> {
        self.relocate(
            "copy",
            mount_id,
            path,
            destination_mount_id,
            destination_path,
        )
        .await
    }

    async fn relocate(
        &self,
        operation: &'static str,
        mount_id: &MountId,
        path: &RemotePath,
        destination_mount_id: &MountId,
        destination_path: &RemotePath,
    ) -> Result<(), AppError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Destination<'a> {
            to_mount_id: &'a str,
            to_path: &'a str,
        }
        let mut url = self.mount_endpoint(mount_id, &["files", operation])?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        let response = self
            .authenticated_request(Method::PUT, url)
            .await?
            .json(&Destination {
                to_mount_id: destination_mount_id.as_str(),
                to_path: destination_path.as_str(),
            })
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK])?;
        Ok(())
    }

    pub async fn upload(
        &self,
        mount_id: &MountId,
        directory: &RemotePath,
        file_name: &RemoteName,
        body: reqwest::Body,
        content_length: u64,
    ) -> Result<FileInfo, AppError> {
        let mut url = self.content_mount_endpoint(mount_id, &["files", "put"])?;
        url.query_pairs_mut()
            .append_pair("path", directory.as_str())
            .append_pair("filename", file_name.as_str())
            .append_pair("info", "true");
        let part = reqwest::multipart::Part::stream_with_length(body, content_length)
            .file_name(file_name.as_str().to_owned())
            .mime_str("application/octet-stream")?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = self
            .authenticated_request(Method::POST, url)
            .await?
            .multipart(form)
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        Self::decode_json(response).await
    }

    pub async fn download_response(
        &self,
        mount_id: &MountId,
        path: &RemotePath,
    ) -> Result<Response, AppError> {
        let mut url = self.content_mount_endpoint(mount_id, &["files", "get"])?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK])
    }

    async fn authenticated_request(
        &self,
        method: Method,
        url: Url,
    ) -> Result<reqwest::RequestBuilder, AppError> {
        let session = self.session.read().await;
        let session = session.as_ref().ok_or(AppError::NotAuthenticated)?;
        let authorization = format!("Token token={}", session.token);
        Ok(self
            .client
            .request(method, url)
            .header(header::AUTHORIZATION, authorization))
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url, AppError> {
        let mut url = self.base_url.clone();
        url.set_path("");
        let mut path = url
            .path_segments_mut()
            .map_err(|_| AppError::Initialization)?;
        path.clear();
        path.extend(segments.iter().copied());
        drop(path);
        Ok(url)
    }

    fn mount_endpoint(&self, mount_id: &MountId, tail: &[&str]) -> Result<Url, AppError> {
        let mut segments = vec!["api", "v2", "mounts", mount_id.as_str()];
        segments.extend_from_slice(tail);
        self.endpoint(&segments)
    }

    fn content_mount_endpoint(&self, mount_id: &MountId, tail: &[&str]) -> Result<Url, AppError> {
        let mut segments = vec!["content", "api", "v2", "mounts", mount_id.as_str()];
        segments.extend_from_slice(tail);
        self.endpoint(&segments)
    }

    fn expect_status(response: Response, expected: &[StatusCode]) -> Result<Response, AppError> {
        if expected.contains(&response.status()) {
            Ok(response)
        } else {
            Err(AppError::from_status(response.status()))
        }
    }

    async fn decode_json<T: serde::de::DeserializeOwned>(
        response: Response,
    ) -> Result<T, AppError> {
        let bytes = response.bytes().await?;
        serde_json::from_slice(&bytes).map_err(AppError::Decode)
    }
}

#[cfg(test)]
mod tests {
    use httpmock::{Method::DELETE, Method::GET, Method::POST, Method::PUT, MockServer};
    use serde_json::json;

    use super::{KoofrApi, Session};
    use crate::{
        error::AppError,
        file_ops::{MountId, RemotePath},
    };

    #[tokio::test]
    async fn exchanges_an_app_password_for_an_in_memory_token() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/token").json_body(json!({
                    "email": "person@example.com",
                    "password": "app-password"
                }));
                then.status(200)
                    .header("x-user-id", "user-1")
                    .json_body(json!({ "token": "session-token" }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");

        let session = api
            .authenticate("person@example.com", "app-password")
            .await
            .expect("authenticate");

        mock.assert_async().await;
        assert!(session.authenticated);
        assert_eq!(session.user_id.as_deref(), Some("user-1"));
        let current_session = api.session.read().await;
        assert_eq!(
            current_session
                .as_ref()
                .map(|session| session.token.as_str()),
            Some("session-token")
        );
    }

    #[tokio::test]
    async fn reports_invalid_credentials_without_creating_a_session() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/token");
                then.status(401);
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");

        let error = api
            .authenticate("person@example.com", "wrong-password")
            .await
            .expect_err("reject invalid credentials");

        mock.assert_async().await;
        assert!(matches!(error, AppError::AuthenticationFailed));
        assert!(!api.session_info().await.authenticated);
    }

    #[tokio::test]
    async fn maps_mount_list_request_and_response() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/api/v2/mounts")
                    .header("authorization", "Token token=test-token");
                then.status(200).json_body(json!({
                    "mounts": [{
                        "id": "mount_1",
                        "name": "Koofr",
                        "type": "device",
                        "spaceTotal": 1000,
                        "spaceUsed": 250,
                        "online": true,
                        "isPrimary": true,
                        "isShared": false
                    }]
                }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        let mounts = api.list_mounts().await.expect("list mounts");

        mock.assert_async().await;
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].id, "mount_1");
        assert!(mounts[0].is_primary);
        assert_eq!(mounts[0].space_used, 250);
    }

    #[tokio::test]
    async fn maps_list_files_request_and_response() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/api/v2/mounts/mount_1/files/list")
                    .query_param("path", "/资料")
                    .header("authorization", "Token token=test-token");
                then.status(200).json_body(json!({
                    "files": [{
                        "name": "计划.txt",
                        "type": "file",
                        "modified": 123,
                        "size": 42,
                        "contentType": "text/plain",
                        "hash": "test-hash"
                    }]
                }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        let files = api
            .list_files(
                &MountId::parse("mount_1".to_owned()).expect("mount id"),
                &RemotePath::parse("/资料".to_owned()).expect("remote path"),
            )
            .await
            .expect("list files");

        mock.assert_async().await;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "/资料/计划.txt");
    }

    #[tokio::test]
    async fn maps_recent_search_request_and_locations() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/api/v2/search")
                    .query_param("limit", "1000")
                    .query_param("offset", "0")
                    .query_param("sortField", "mtime")
                    .query_param("sortDir", "desc")
                    .query_param("mountId", "")
                    .query_param("path", "")
                    .header("authorization", "Token token=test-token");
                then.status(200).json_body(json!({
                    "hits": [{
                        "mountId": "mount_1",
                        "name": "recent.txt",
                        "type": "file",
                        "modified": 123,
                        "size": 42,
                        "contentType": "text/plain",
                        "hash": "hash",
                        "path": "/docs/recent.txt"
                    }],
                    "mounts": {
                        "mount_1": { "id": "mount_1", "name": "Koofr", "type": "device" }
                    }
                }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        let files = api.list_recent().await.expect("list recent files");

        mock.assert_async().await;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].mount_id, "mount_1");
        assert_eq!(files[0].mount_name, "Koofr");
        assert_eq!(files[0].path, "/docs/recent.txt");
    }

    #[tokio::test]
    async fn maps_shared_items_and_direction() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/api/v2/shared")
                    .header("authorization", "Token token=test-token");
                then.status(200).json_body(json!({
                    "files": [{
                        "name": "Team folder",
                        "type": "dir",
                        "modified": 123,
                        "size": 0,
                        "mount": {
                            "id": "shared_1",
                            "name": "Team folder",
                            "type": "shared",
                            "isShared": true
                        }
                    }]
                }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        let files = api.list_shared().await.expect("list shared files");

        mock.assert_async().await;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "/");
        assert_eq!(files[0].share_direction.as_deref(), Some("received"));
    }

    #[tokio::test]
    async fn maps_trash_list_request_and_response() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/api/v2/trash")
                    .query_param("pageSize", "1000")
                    .query_param("sortField", "deleted")
                    .query_param("sortDir", "desc")
                    .header("authorization", "Token token=test-token");
                then.status(200).json_body(json!({
                    "files": [{
                        "id": 1,
                        "mountId": "mount_1",
                        "path": "/deleted.txt",
                        "name": "deleted.txt",
                        "deleted": 1784109600000_i64,
                        "size": 42,
                        "contentType": "text/plain"
                    }],
                    "mounts": {
                        "mount_1": { "id": "mount_1", "name": "Koofr", "type": "device" }
                    },
                    "retentionDays": 30,
                    "pageInfo": { "cursor": null }
                }));
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        let trash = api.list_trash().await.expect("list trash");

        mock.assert_async().await;
        assert_eq!(trash.retention_days, 30);
        assert_eq!(trash.items[0].version_id, "1");
        assert_eq!(trash.items[0].mount_name, "Koofr");
    }

    #[tokio::test]
    async fn restores_selected_trash_paths() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/api/v2/trash/undelete")
                    .header("authorization", "Token token=test-token")
                    .json_body(json!({
                        "files": [{ "mountId": "mount_1", "path": "/deleted.txt" }]
                    }));
                then.status(202);
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        api.restore_trash(&[(
            MountId::parse("mount_1".to_owned()).expect("mount id"),
            RemotePath::parse("/deleted.txt".to_owned()).expect("remote path"),
        )])
        .await
        .expect("restore trash");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn permanently_empties_trash() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(DELETE)
                    .path("/api/v2/trash")
                    .header("authorization", "Token token=test-token");
                then.status(204);
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        api.empty_trash().await.expect("empty trash");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn requires_a_session_before_requests() {
        let server = MockServer::start_async().await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        let result = api.list_mounts().await;
        assert!(matches!(
            result,
            Err(crate::error::AppError::NotAuthenticated)
        ));
    }

    #[tokio::test]
    async fn maps_move_request_without_concatenating_unchecked_paths() {
        let server = MockServer::start_async().await;
        let mock = server
            .mock_async(|when, then| {
                when.method(PUT)
                    .path("/api/v2/mounts/mount_1/files/move")
                    .query_param("path", "/source.txt")
                    .header("authorization", "Token token=test-token")
                    .json_body(json!({
                        "toMountId": "mount_2",
                        "toPath": "/target/source.txt"
                    }));
                then.status(200);
            })
            .await;
        let api = KoofrApi::new(&server.base_url()).expect("create API client");
        *api.session.write().await = Some(Session {
            token: "test-token".to_owned(),
            user_id: None,
        });

        api.move_to(
            &MountId::parse("mount_1".to_owned()).expect("source mount"),
            &RemotePath::parse("/source.txt".to_owned()).expect("source path"),
            &MountId::parse("mount_2".to_owned()).expect("destination mount"),
            &RemotePath::parse("/target/source.txt".to_owned()).expect("destination path"),
        )
        .await
        .expect("move entry");

        mock.assert_async().await;
    }
}
