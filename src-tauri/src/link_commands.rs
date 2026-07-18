use tauri::State;

use crate::{
    AppState,
    error::CommandError,
    file_ops::{MountId, RemotePath},
    koofr_api::{PublicLink, PublicLinkKind},
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn list_public_links(
    state: State<'_, AppState>,
    mount_id: String,
) -> CommandResult<Vec<PublicLink>> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    state
        .api
        .list_public_links(&mount_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn create_public_link(
    state: State<'_, AppState>,
    mount_id: String,
    path: String,
    kind: PublicLinkKind,
) -> CommandResult<PublicLink> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let path = RemotePath::parse(path).map_err(CommandError::from)?;
    state
        .api
        .create_public_link(&mount_id, &path, kind)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn delete_public_link(
    state: State<'_, AppState>,
    mount_id: String,
    link_id: String,
    kind: PublicLinkKind,
) -> CommandResult<()> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    state
        .api
        .delete_public_link(&mount_id, &link_id, kind)
        .await
        .map_err(CommandError::from)
}
