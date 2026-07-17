use httpmock::{Method::DELETE, Method::GET, Method::POST, MockServer};
use serde_json::json;

use super::super::{KoofrApi, PublicLinkKind, Session};
use crate::file_ops::{MountId, RemotePath};

async fn authenticated_api(server: &MockServer) -> KoofrApi {
    let api = KoofrApi::new(&server.base_url()).expect("create API client");
    *api.session.write().await = Some(Session {
        token: "test-token".to_owned(),
        user_id: None,
    });
    api
}

#[tokio::test]
async fn lists_download_and_upload_links_for_a_mount() {
    // Given
    let server = MockServer::start_async().await;
    let download_mock = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/api/v2.1/mounts/mount_1/links")
                .header("authorization", "Token token=test-token");
            then.status(200).json_body(json!({
                "links": [{
                    "id": "download_1",
                    "name": "report.pdf",
                    "path": "/report.pdf",
                    "counter": 3,
                    "url": "https://app.koofr.net/links/download_1",
                    "shortUrl": "https://k00.fr/report",
                    "hasPassword": false
                }]
            }));
        })
        .await;
    let upload_mock = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/api/v2.1/mounts/mount_1/receivers")
                .header("authorization", "Token token=test-token");
            then.status(200).json_body(json!({
                "receivers": [{
                    "id": "upload_1",
                    "name": "incoming",
                    "path": "/incoming/",
                    "counter": "2",
                    "url": "https://app.koofr.net/receive/upload_1",
                    "shortUrl": "https://k00.fr/incoming",
                    "hasPassword": true
                }]
            }));
        })
        .await;
    let api = authenticated_api(&server).await;

    // When
    let links = api
        .list_public_links(&MountId::parse("mount_1".to_owned()).expect("mount id"))
        .await
        .expect("list public links");

    // Then
    download_mock.assert_async().await;
    upload_mock.assert_async().await;
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].id, "download_1");
    assert_eq!(links[0].kind, PublicLinkKind::Download);
    assert_eq!(links[1].id, "upload_1");
    assert_eq!(links[1].kind, PublicLinkKind::Upload);
    assert_eq!(links[1].counter, 2);
    assert!(links[1].has_password);
}

#[tokio::test]
async fn creates_a_receive_link_with_a_directory_path() {
    // Given
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/api/v2.1/mounts/mount_1/receivers")
                .header("authorization", "Token token=test-token")
                .json_body(json!({ "path": "/incoming/" }));
            then.status(201).json_body(json!({
                "id": "upload_1",
                "name": "incoming",
                "path": "/incoming/",
                "counter": 0,
                "url": "https://app.koofr.net/receive/upload_1",
                "shortUrl": "https://k00.fr/incoming",
                "hasPassword": false
            }));
        })
        .await;
    let api = authenticated_api(&server).await;

    // When
    let link = api
        .create_public_link(
            &MountId::parse("mount_1".to_owned()).expect("mount id"),
            &RemotePath::parse("/incoming".to_owned()).expect("remote path"),
            PublicLinkKind::Upload,
        )
        .await
        .expect("create receive link");

    // Then
    mock.assert_async().await;
    assert_eq!(link.id, "upload_1");
    assert_eq!(link.kind, PublicLinkKind::Upload);
}

#[tokio::test]
async fn revokes_the_selected_download_link() {
    // Given
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(DELETE)
                .path("/api/v2.1/mounts/mount_1/links/download_1")
                .header("authorization", "Token token=test-token");
            then.status(204);
        })
        .await;
    let api = authenticated_api(&server).await;

    // When
    api.delete_public_link(
        &MountId::parse("mount_1".to_owned()).expect("mount id"),
        "download_1",
        PublicLinkKind::Download,
    )
    .await
    .expect("delete public link");

    // Then
    mock.assert_async().await;
}
