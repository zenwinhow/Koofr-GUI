use tauri::WebviewWindow;
use zeroize::Zeroizing;

use crate::error::AppError;

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_CANCELLED, NO_ERROR},
    Security::Credentials::{
        CREDUI_FLAGS_ALWAYS_SHOW_UI, CREDUI_FLAGS_DO_NOT_PERSIST,
        CREDUI_FLAGS_EXCLUDE_CERTIFICATES, CREDUI_FLAGS_GENERIC_CREDENTIALS,
        CREDUI_FLAGS_KEEP_USERNAME, CREDUI_INFOW, CredUIPromptForCredentialsW,
    },
};

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Opens the Windows credential UI so the Safe Key never becomes an IPC argument
/// or React state value.
#[cfg(windows)]
pub async fn prompt_safe_key(
    window: &WebviewWindow,
    safe_box_id: &str,
    safe_box_name: &str,
    action: &str,
) -> Result<Option<Zeroizing<String>>, AppError> {
    let hwnd = window.hwnd().map_err(|_| AppError::VaultPrompt)?.0 as isize;
    let caption = wide("Koofr 私人保险箱");
    let message = wide(&format!(
        "{action}“{safe_box_name}”\nSafe Key 只会交给本机 Rust 后端。"
    ));
    let target = wide(&format!("KoofrGUI:Vault:{safe_box_id}"));
    let username_seed = safe_box_name.to_owned();

    tauri::async_runtime::spawn_blocking(move || {
        let mut username = Zeroizing::new(vec![0_u16; 514]);
        for (index, unit) in username_seed
            .encode_utf16()
            .take(username.len().saturating_sub(1))
            .enumerate()
        {
            username[index] = unit;
        }
        let mut password = Zeroizing::new(vec![0_u16; 512]);
        let mut save = 0;
        let info = CREDUI_INFOW {
            cbSize: std::mem::size_of::<CREDUI_INFOW>() as u32,
            hwndParent: hwnd as _,
            pszMessageText: message.as_ptr(),
            pszCaptionText: caption.as_ptr(),
            hbmBanner: std::ptr::null_mut(),
        };
        let flags = CREDUI_FLAGS_ALWAYS_SHOW_UI
            | CREDUI_FLAGS_DO_NOT_PERSIST
            | CREDUI_FLAGS_EXCLUDE_CERTIFICATES
            | CREDUI_FLAGS_GENERIC_CREDENTIALS
            | CREDUI_FLAGS_KEEP_USERNAME;
        // SAFETY: every pointer references a live, writable, NUL-terminated buffer for the
        // duration of the synchronous Windows credential UI call.
        let status = unsafe {
            CredUIPromptForCredentialsW(
                &info,
                target.as_ptr(),
                std::ptr::null(),
                0,
                username.as_mut_ptr(),
                username.len() as u32,
                password.as_mut_ptr(),
                password.len() as u32,
                &mut save,
                flags,
            )
        };
        if status == ERROR_CANCELLED {
            return Ok(None);
        }
        if status != NO_ERROR {
            return Err(AppError::VaultPrompt);
        }
        let length = password
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(password.len());
        let value = String::from_utf16(&password[..length]).map_err(|_| AppError::VaultPrompt)?;
        if value.is_empty() {
            return Ok(None);
        }
        Ok(Some(Zeroizing::new(value)))
    })
    .await
    .map_err(|_| AppError::VaultPrompt)?
}

#[cfg(not(windows))]
pub async fn prompt_safe_key(
    _window: &WebviewWindow,
    _safe_box_id: &str,
    _safe_box_name: &str,
    _action: &str,
) -> Result<Option<Zeroizing<String>>, AppError> {
    Err(AppError::VaultPrompt)
}
