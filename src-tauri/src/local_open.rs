use std::path::Path;

use crate::error::AppError;

#[cfg(windows)]
pub fn open(path: &Path) -> Result<(), AppError> {
    use std::{os::windows::ffi::OsStrExt, ptr};

    use windows_sys::Win32::UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::SW_SHOWNORMAL,
    };

    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // ShellExecuteW opens through the user's registered Windows file association
    // without passing the path through a command shell.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(AppError::LocalOpen);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn open(_path: &Path) -> Result<(), AppError> {
    Err(AppError::LocalOpen)
}
