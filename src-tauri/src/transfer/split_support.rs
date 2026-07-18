use crate::{
    error::AppError,
    file_ops::{MountId, RemoteName, RemotePath},
    koofr_api::{FileInfo, KoofrApi},
};

use super::split_package::{
    MANIFEST_NAME, README_NAME, RESTORE_CMD_NAME, RESTORE_SH_NAME, SHA256SUMS_NAME, SplitManifest,
    restore_cmd, restore_sh,
};

pub(super) async fn upload_support_files(
    api: &KoofrApi,
    mount_id: &MountId,
    package_path: &RemotePath,
    manifest: &SplitManifest,
) -> Result<(), AppError> {
    let existing = api.list_files(mount_id, package_path).await?;
    let target = RemoteTarget {
        api,
        mount_id,
        directory: package_path,
        existing: &existing,
    };
    target
        .upload_if_missing(RESTORE_CMD_NAME, restore_cmd().as_bytes().to_vec())
        .await?;
    target
        .upload_if_missing(RESTORE_SH_NAME, restore_sh().as_bytes().to_vec())
        .await?;
    target
        .upload_if_missing(README_NAME, manifest.readme().into_bytes())
        .await?;
    target
        .upload_if_missing(SHA256SUMS_NAME, manifest.sha256sums().into_bytes())
        .await?;
    let payload = serde_json::to_vec_pretty(manifest).map_err(|_| AppError::LocalData)?;
    target.upload_if_missing(MANIFEST_NAME, payload).await?;
    Ok(())
}

struct RemoteTarget<'a> {
    api: &'a KoofrApi,
    mount_id: &'a MountId,
    directory: &'a RemotePath,
    existing: &'a [FileInfo],
}

impl RemoteTarget<'_> {
    async fn upload_if_missing(&self, name: &str, bytes: Vec<u8>) -> Result<(), AppError> {
        let length = u64::try_from(bytes.len()).map_err(|_| AppError::LocalData)?;
        let remote_name = RemoteName::parse(name.to_owned())?;
        if let Some(file) = self.existing.iter().find(|file| file.name == name) {
            if file.entry_type == "file" && u64::try_from(file.size).ok() == Some(length) {
                let path = self.directory.join(&remote_name)?;
                let existing = self.api.download_response(self.mount_id, &path).await?;
                if existing.bytes().await?.as_ref() == bytes.as_slice() {
                    return Ok(());
                }
            }
            return Err(AppError::Conflict);
        }
        self.api
            .upload(
                self.mount_id,
                self.directory,
                &remote_name,
                reqwest::Body::from(bytes),
                length,
            )
            .await?;
        Ok(())
    }
}
