use std::{collections::HashSet, path::PathBuf};

use httpmock::{Method::GET, Method::POST, MockServer};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath},
    koofr_api::KoofrApi,
    transfer::TransferResult,
};

use super::{FolderDownloadTarget, FolderExecutor, allocate_local_segment};

#[test]
fn disambiguates_sanitized_windows_name_collisions() {
    let first = RemoteName::parse("report:final.txt".to_owned()).expect("first name");
    let second = RemoteName::parse("report?final.txt".to_owned()).expect("second name");
    let mut used = HashSet::new();
    let first_local = allocate_local_segment(&first, &mut used);
    let second_local = allocate_local_segment(&second, &mut used);
    assert_eq!(first_local, "report_final.txt");
    assert_eq!(second_local, "report_final (2).txt");
}

pub(super) async fn run_executor(
    api: &KoofrApi,
    target: &FolderDownloadTarget,
    remote_path: &str,
) -> Result<TransferResult, AppError> {
    let mount_id = MountId::parse("mount_1".to_owned()).expect("mount id");
    let cancel = CancellationToken::new();
    let mut executor = FolderExecutor {
        api,
        mount_id: &mount_id,
        target,
        cancel: &cancel,
        progress: |_, _| {},
    };
    executor
        .run(
            RemotePath::parse(remote_path.to_owned()).expect("remote path"),
            &uuid::Uuid::new_v4().to_string(),
        )
        .await
}

pub(super) async fn authenticated_api(server: &MockServer) -> KoofrApi {
    let api = KoofrApi::new(&server.base_url()).expect("create API client");
    api.authenticate("test@example.com", "password")
        .await
        .expect("authenticate API client");
    api
}

pub(super) async fn mock_token(server: &MockServer) {
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body(json!({"token": "test-token"}));
        })
        .await;
}

pub(super) async fn mock_listing(server: &MockServer, path: &str, body: serde_json::Value) {
    server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/api/v2/mounts/mount_1/files/list")
                .query_param("path", path)
                .header("authorization", "Token token=test-token");
            then.status(200).json_body(body);
        })
        .await;
}

pub(super) async fn mock_content(server: &MockServer, path: &str, body: &str) {
    server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/content/api/v2/mounts/mount_1/files/get")
                .query_param("path", path)
                .header("authorization", "Token token=test-token");
            then.status(200).body(body);
        })
        .await;
}

pub(super) fn file_json(name: &str, entry_type: &str, size: i64) -> serde_json::Value {
    json!({
        "name": name,
        "type": entry_type,
        "modified": 0,
        "size": size
    })
}

pub(super) async fn temporary_parent(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "koofr-gui-folder-download-{label}-{}",
        uuid::Uuid::new_v4()
    ));
    tokio::fs::create_dir(&path)
        .await
        .expect("create test directory");
    path
}
