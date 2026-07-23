use std::{collections::HashMap, path::Path};

use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

use crate::{
    AppState,
    crypto::VaultCipher,
    download_history::{DownloadLocalKind, NewDownloadHistoryItem},
    error::{AppError, CommandError},
    file_ops::{LocalDownloadPath, LocalUploadPath, MountId, RemoteName, RemotePath},
    koofr_api::VaultRepoCreate,
    transfer::{self, TransferResult},
    vault_core::{VaultDirectory, VaultSummary, prompt_safe_key},
};

type CommandResult<T> = Result<T, CommandError>;

#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> CommandResult<Vec<VaultSummary>> {
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    Ok(state.vault.sync_repos(repos).await)
}

#[tauri::command]
pub async fn unlock_vault(
    window: WebviewWindow,
    state: State<'_, AppState>,
    repo_id: String,
) -> CommandResult<VaultSummary> {
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    state.vault.sync_repos(repos).await;
    let repo = state
        .vault
        .repo_for_unlock(&repo_id)
        .await
        .map_err(CommandError::from)?;
    let safe_key = prompt_safe_key(&window, &repo.id, &repo.name, "请输入 Safe Key 以解锁")
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::from(AppError::Cancelled))?;
    state
        .vault
        .unlock(&repo_id, safe_key.as_str())
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn lock_vault(
    state: State<'_, AppState>,
    repo_id: String,
) -> CommandResult<Vec<VaultSummary>> {
    state
        .vault
        .lock(&repo_id)
        .await
        .map_err(CommandError::from)?;
    Ok(state.vault.summaries().await)
}

#[tauri::command]
pub async fn list_vault_files(
    state: State<'_, AppState>,
    repo_id: String,
    directory_id: String,
) -> CommandResult<VaultDirectory> {
    state
        .vault
        .list_directory(&state.api, &repo_id, &directory_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn create_vault_folder(
    state: State<'_, AppState>,
    repo_id: String,
    parent_id: String,
    name: String,
) -> CommandResult<()> {
    state
        .vault
        .create_directory(&state.api, &repo_id, &parent_id, name)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn rename_vault_entry(
    state: State<'_, AppState>,
    repo_id: String,
    entry_id: String,
    new_name: String,
) -> CommandResult<()> {
    state
        .vault
        .rename(&state.api, &repo_id, &entry_id, new_name)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn relocate_vault_entry(
    state: State<'_, AppState>,
    repo_id: String,
    entry_id: String,
    destination_id: String,
    is_move: bool,
) -> CommandResult<()> {
    state
        .vault
        .relocate(&state.api, &repo_id, &entry_id, &destination_id, is_move)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn delete_vault_entries(
    state: State<'_, AppState>,
    repo_id: String,
    entry_ids: Vec<String>,
) -> CommandResult<()> {
    state
        .vault
        .delete(&state.api, &repo_id, &entry_ids)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn upload_vault_file(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
    repo_id: String,
    parent_id: String,
    local_path_grant: String,
) -> CommandResult<TransferResult> {
    let selected = state
        .local_access
        .take_upload(&local_path_grant)
        .map_err(CommandError::from)?;
    let local_path = LocalUploadPath::from_selected(selected)
        .await
        .map_err(CommandError::from)?;
    let (repo, cipher, remote_directory) = state
        .vault
        .operation_target(&repo_id, &parent_id, Some("dir"))
        .await
        .map_err(CommandError::from)?;
    let mount_id = MountId::parse(repo.mount_id.clone()).map_err(CommandError::from)?;
    let retry_policy =
        transfer::NetworkRetryPolicy::from(state.settings.network_retry_settings().await);
    transfer::upload_vault_file(
        app,
        &state.api,
        &state.transfers,
        &state.transfer_checkpoints,
        retry_policy,
        transfer_id,
        repo.id,
        mount_id,
        remote_directory,
        cipher,
        local_path,
    )
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn download_vault_file(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
    repo_id: String,
    entry_id: String,
    display_name: String,
    local_path_grant: String,
) -> CommandResult<TransferResult> {
    let display_name = RemoteName::parse(display_name).map_err(CommandError::from)?;
    let selected = state
        .local_access
        .take_download(&local_path_grant)
        .map_err(CommandError::from)?;
    let completed_path = selected.clone();
    let local_path = LocalDownloadPath::from_selected(selected)
        .await
        .map_err(CommandError::from)?;
    let (repo, cipher, remote_path) = state
        .vault
        .operation_target(&repo_id, &entry_id, Some("file"))
        .await
        .map_err(CommandError::from)?;
    let mount_id = MountId::parse(repo.mount_id).map_err(CommandError::from)?;
    let owner_id = transfer::current_owner(&state.api)
        .await
        .map_err(CommandError::from)?;
    state
        .download_history
        .start(NewDownloadHistoryItem {
            transfer_id: &transfer_id,
            owner_id: &owner_id,
            name: display_name.as_str(),
            remote_path: &format!("vault:{}", repo.id),
            local_path: &completed_path,
            local_kind: DownloadLocalKind::File,
            recovery_kind: Some(transfer::RecoveryKind::ByteResume),
        })
        .map_err(CommandError::from)?;
    let retry_policy =
        transfer::NetworkRetryPolicy::from(state.settings.network_retry_settings().await);
    let result = transfer::download_vault_file(
        app,
        &state.api,
        &state.transfers,
        &state.transfer_checkpoints,
        retry_policy,
        transfer_id.clone(),
        owner_id,
        repo.id,
        mount_id,
        remote_path,
        cipher,
        local_path,
    )
    .await
    .map_err(CommandError::from)?;
    state
        .local_access
        .remember_download(&transfer_id, completed_path)
        .map_err(CommandError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn create_vault(
    window: WebviewWindow,
    state: State<'_, AppState>,
    mount_id: String,
    parent_path: String,
    name: String,
) -> CommandResult<VaultSummary> {
    let mount_id = MountId::parse(mount_id).map_err(CommandError::from)?;
    let parent_path = RemotePath::parse(parent_path).map_err(CommandError::from)?;
    let name = RemoteName::parse(name).map_err(CommandError::from)?;
    let prompt_id = uuid::Uuid::new_v4().to_string();
    let first = prompt_safe_key(
        &window,
        &prompt_id,
        name.as_str(),
        "请设置至少 8 个字符的 Safe Key",
    )
    .await
    .map_err(CommandError::from)?
    .ok_or_else(|| CommandError::from(AppError::Cancelled))?;
    if first.chars().count() < 8 {
        return Err(CommandError::from(AppError::InvalidInput(
            "vault Safe Key length",
        )));
    }
    let second = prompt_safe_key(
        &window,
        &prompt_id,
        name.as_str(),
        "请再次输入 Safe Key 进行确认",
    )
    .await
    .map_err(CommandError::from)?
    .ok_or_else(|| CommandError::from(AppError::Cancelled))?;
    if first.as_str() != second.as_str() {
        return Err(CommandError::from(AppError::VaultInvalidKey));
    }
    let salt = vault_crypto::random_password::random_password(128)
        .map_err(|_| CommandError::from(AppError::VaultCrypto))?;
    let cipher = VaultCipher::new(first.as_str(), Some(&salt));
    let (password_validator, password_validator_encrypted) =
        cipher.generate_validator().map_err(CommandError::from)?;
    let repo_path = parent_path.join(&name).map_err(CommandError::from)?;
    let created_folder = match state
        .api
        .create_folder(&mount_id, &parent_path, &name)
        .await
    {
        Ok(()) => true,
        Err(AppError::Conflict) => false,
        Err(error) => return Err(CommandError::from(error)),
    };
    let created = state
        .api
        .create_vault_repo(&VaultRepoCreate {
            mount_id: mount_id.as_str().to_owned(),
            path: repo_path.as_str().to_owned(),
            salt: Some(salt),
            password_validator,
            password_validator_encrypted,
        })
        .await;
    let created = match created {
        Ok(created) => created,
        Err(error) => {
            if created_folder {
                let _ = state.api.delete(&mount_id, &repo_path).await;
            }
            return Err(CommandError::from(error));
        }
    };
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    state.vault.sync_repos(repos).await;
    state
        .vault
        .unlock(&created.id, first.as_str())
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn remove_vault(
    window: WebviewWindow,
    state: State<'_, AppState>,
    repo_id: String,
    confirmation: String,
) -> CommandResult<Vec<VaultSummary>> {
    if confirmation != "移除保险箱" {
        return Err(CommandError::from(AppError::InvalidInput(
            "vault remove confirmation",
        )));
    }
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    state.vault.sync_repos(repos).await;
    let repo = state
        .vault
        .repo_for_unlock(&repo_id)
        .await
        .map_err(CommandError::from)?;
    let safe_key = prompt_safe_key(
        &window,
        &repo.id,
        &repo.name,
        "请输入 Safe Key 以移除保险箱注册",
    )
    .await
    .map_err(CommandError::from)?
    .ok_or_else(|| CommandError::from(AppError::Cancelled))?;
    state
        .vault
        .unlock(&repo.id, safe_key.as_str())
        .await
        .map_err(CommandError::from)?;
    state
        .api
        .remove_vault_repo(&repo.id)
        .await
        .map_err(CommandError::from)?;
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    Ok(state.vault.sync_repos(repos).await)
}

#[tauri::command]
pub async fn export_vault_rclone_config(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    repo_id: String,
) -> CommandResult<bool> {
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    state.vault.sync_repos(repos).await;
    let repo = state
        .vault
        .repo_for_unlock(&repo_id)
        .await
        .map_err(CommandError::from)?;
    let safe_key = prompt_safe_key(
        &window,
        &repo.id,
        &repo.name,
        "请输入 Safe Key 以导出 rclone 配置",
    )
    .await
    .map_err(CommandError::from)?
    .ok_or_else(|| CommandError::from(AppError::Cancelled))?;
    let cipher = VaultCipher::new(safe_key.as_str(), repo.salt.as_deref());
    if !cipher
        .validates(&repo.password_validator, &repo.password_validator_encrypted)
        .map_err(CommandError::from)?
    {
        return Err(CommandError::from(AppError::VaultInvalidKey));
    }
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("rclone 配置", &["conf"])
        .set_file_name(format!("{}.conf", config_section_name(&repo.name)))
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = file_path
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    validate_sensitive_output(&path)
        .await
        .map_err(CommandError::from)?;
    let password = vault_crypto::rclone_obscure::obscure(safe_key.as_str())
        .map_err(|_| CommandError::from(AppError::VaultCrypto))?;
    let mut config = format!(
        "[{}]\ntype=crypt\nremote=koofr:{}\npassword={password}\n",
        config_section_name(&repo.name),
        repo.path
    );
    if let Some(salt) = repo.salt.as_deref() {
        let password2 = vault_crypto::rclone_obscure::obscure(salt)
            .map_err(|_| CommandError::from(AppError::VaultCrypto))?;
        config.push_str(&format!("password2={password2}\n"));
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(path).await.map_err(AppError::from)?;
    use tokio::io::AsyncWriteExt;
    file.write_all(config.as_bytes())
        .await
        .map_err(AppError::from)?;
    file.flush().await.map_err(AppError::from)?;
    file.sync_all().await.map_err(AppError::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn import_vault_rclone_config(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Vec<VaultSummary>> {
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("rclone 配置", &["conf"])
        .blocking_pick_file()
    else {
        return Ok(state.vault.summaries().await);
    };
    let path = file_path
        .into_path()
        .map_err(|_| CommandError::from(AppError::Dialog))?;
    let metadata = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(AppError::from)
        .map_err(CommandError::from)?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > 64 * 1024
    {
        return Err(CommandError::from(AppError::InvalidInput(
            "rclone config file",
        )));
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(AppError::from)
        .map_err(CommandError::from)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| CommandError::from(AppError::InvalidInput("rclone config encoding")))?;
    let parsed = parse_rclone_config(text).map_err(CommandError::from)?;
    let safe_key = Zeroizing::new(
        vault_crypto::rclone_obscure::reveal(&parsed.password)
            .map_err(|_| CommandError::from(AppError::VaultCrypto))?,
    );
    let salt = parsed
        .password2
        .as_deref()
        .map(vault_crypto::rclone_obscure::reveal)
        .transpose()
        .map_err(|_| CommandError::from(AppError::VaultCrypto))?;
    let remote_path = RemotePath::parse(parsed.remote_path).map_err(CommandError::from)?;
    let mounts = state.api.list_mounts().await.map_err(CommandError::from)?;
    let mount = mounts
        .iter()
        .find(|mount| mount.is_primary && mount.online)
        .or_else(|| mounts.iter().find(|mount| mount.is_primary))
        .ok_or_else(|| CommandError::from(AppError::NotFound))?;
    let mount_id = MountId::parse(mount.id.clone()).map_err(CommandError::from)?;
    let info = state
        .api
        .file_info(&mount_id, &remote_path)
        .await
        .map_err(CommandError::from)?;
    if info.entry_type != "dir" {
        return Err(CommandError::from(AppError::InvalidInput(
            "vault import directory",
        )));
    }
    let cipher = VaultCipher::new(safe_key.as_str(), salt.as_deref());
    let (password_validator, password_validator_encrypted) =
        cipher.generate_validator().map_err(CommandError::from)?;
    let created = state
        .api
        .create_vault_repo(&VaultRepoCreate {
            mount_id: mount.id.clone(),
            path: remote_path.as_str().to_owned(),
            salt,
            password_validator,
            password_validator_encrypted,
        })
        .await
        .map_err(CommandError::from)?;
    let repos = state
        .api
        .list_vault_repos()
        .await
        .map_err(CommandError::from)?;
    state.vault.sync_repos(repos).await;
    state
        .vault
        .unlock(&created.id, safe_key.as_str())
        .await
        .map_err(CommandError::from)?;
    Ok(state.vault.summaries().await)
}

struct ParsedRcloneConfig {
    remote_path: String,
    password: String,
    password2: Option<String>,
}

fn parse_rclone_config(value: &str) -> Result<ParsedRcloneConfig, AppError> {
    let mut sections: Vec<HashMap<String, String>> = Vec::new();
    let mut current: Option<HashMap<String, String>> = None;
    for raw_line in value.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(HashMap::new());
            continue;
        }
        let (key, val) = line
            .split_once('=')
            .ok_or(AppError::InvalidInput("rclone config line"))?;
        let section = current
            .as_mut()
            .ok_or(AppError::InvalidInput("rclone config section"))?;
        section.insert(key.trim().to_ascii_lowercase(), val.trim().to_owned());
    }
    if let Some(section) = current {
        sections.push(section);
    }
    let mut crypt = sections
        .into_iter()
        .filter(|section| section.get("type").is_some_and(|value| value == "crypt"));
    let section = crypt
        .next()
        .ok_or(AppError::InvalidInput("rclone crypt section"))?;
    if crypt.next().is_some() {
        return Err(AppError::InvalidInput("rclone crypt section count"));
    }
    let remote = section
        .get("remote")
        .ok_or(AppError::InvalidInput("rclone remote"))?;
    let remote_path = remote
        .strip_prefix("koofr:")
        .ok_or(AppError::InvalidInput("rclone Koofr remote"))?;
    let remote_path = if remote_path.is_empty() {
        "/".to_owned()
    } else if remote_path.starts_with('/') {
        remote_path.to_owned()
    } else {
        format!("/{remote_path}")
    };
    let password = section
        .get("password")
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or(AppError::InvalidInput("rclone password"))?;
    Ok(ParsedRcloneConfig {
        remote_path,
        password,
        password2: section.get("password2").cloned(),
    })
}

fn config_section_name(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "vault".to_owned()
    } else {
        slug.chars().take(64).collect()
    }
}

async fn validate_sensitive_output(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(AppError::InvalidInput("rclone config output"));
    }
    let parent = path
        .parent()
        .ok_or(AppError::InvalidInput("rclone config parent"))?;
    let metadata = tokio::fs::symlink_metadata(parent).await?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::InvalidInput("rclone config parent"));
    }
    if tokio::fs::try_exists(path).await? {
        return Err(AppError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_rclone_config;

    #[test]
    fn parses_exactly_one_koofr_crypt_section() {
        let parsed = parse_rclone_config(
            "[vault]\ntype=crypt\nremote=koofr:/Safe Box\npassword=obscured\npassword2=salt\n",
        )
        .expect("parse config");
        assert_eq!(parsed.remote_path, "/Safe Box");
        assert_eq!(parsed.password, "obscured");
        assert_eq!(parsed.password2.as_deref(), Some("salt"));
    }

    #[test]
    fn rejects_non_koofr_and_ambiguous_configs() {
        assert!(parse_rclone_config("[vault]\ntype=crypt\nremote=local:/x\npassword=x\n").is_err());
        assert!(
            parse_rclone_config(
                "[a]\ntype=crypt\nremote=koofr:/a\npassword=x\n[b]\ntype=crypt\nremote=koofr:/b\npassword=y\n"
            )
            .is_err()
        );
    }
}
