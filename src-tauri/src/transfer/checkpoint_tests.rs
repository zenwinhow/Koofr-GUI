use std::path::PathBuf;

use super::{
    DownloadCheckpoint, RecoveryKind, TransferCheckpoint, TransferCheckpointStore, UploadCheckpoint,
};

#[tokio::test]
async fn persists_download_offsets_and_upload_restart_semantics() {
    let directory =
        std::env::temp_dir().join(format!("koofr-transfer-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("create transfer test directory");
    let state_path = directory.join("transfers.json");
    let partial_path = directory.join("large.bin.koofr-part");
    std::fs::write(&partial_path, vec![7_u8; 32]).expect("create partial download");

    let store = TransferCheckpointStore::load(state_path.clone());
    store
        .insert(TransferCheckpoint::Download(DownloadCheckpoint {
            transfer_id: uuid::Uuid::new_v4().to_string(),
            owner_id: "user-1".to_owned(),
            mount_id: "mount_1".to_owned(),
            remote_path: "/large.bin".to_owned(),
            local_path: directory.join("large.bin"),
            partial_path,
            expected_size: 128,
            remote_hash: "hash".to_owned(),
            remote_modified: 42,
        }))
        .await
        .expect("save download checkpoint");
    store
        .insert(TransferCheckpoint::Upload(UploadCheckpoint {
            transfer_id: uuid::Uuid::new_v4().to_string(),
            owner_id: "user-1".to_owned(),
            mount_id: "mount_1".to_owned(),
            remote_directory: "/".to_owned(),
            local_path: PathBuf::from(r"C:\files\large.bin"),
            expected_size: 256,
            modified_millis: 84,
        }))
        .await
        .expect("save upload checkpoint");

    let reloaded = TransferCheckpointStore::load(state_path);
    let snapshots = reloaded
        .list("user-1")
        .await
        .expect("list persisted checkpoints");

    assert_eq!(snapshots.len(), 2);
    assert!(snapshots.iter().any(|snapshot| {
        snapshot.recovery_kind == RecoveryKind::ByteResume
            && snapshot.bytes_transferred == 32
            && snapshot.total_bytes == 128
    }));
    assert!(
        reloaded
            .list("another-user")
            .await
            .expect("list another account")
            .is_empty()
    );
    assert!(snapshots.iter().any(|snapshot| {
        snapshot.recovery_kind == RecoveryKind::Restart
            && snapshot.bytes_transferred == 0
            && snapshot.total_bytes == 256
    }));

    std::fs::remove_dir_all(directory).expect("remove transfer test directory");
}
