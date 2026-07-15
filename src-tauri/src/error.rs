use std::io;

use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("authentication is required")]
    NotAuthenticated,
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("the requested item already exists")]
    Conflict,
    #[error("the requested item was not found")]
    NotFound,
    #[error("the operation is not permitted")]
    Forbidden,
    #[error("the transfer was cancelled")]
    Cancelled,
    #[error("the remote transfer ended before all bytes were received")]
    IncompleteTransfer,
    #[error("the transfer identifier is already active")]
    DuplicateTransfer,
    #[error("remote service returned HTTP {status}")]
    RemoteStatus { status: StatusCode },
    #[error("network request failed")]
    Network(#[source] reqwest::Error),
    #[error("local file operation failed")]
    Io(#[source] io::Error),
    #[error("remote response could not be decoded")]
    Decode(#[source] serde_json::Error),
    #[error("application initialization failed")]
    Initialization,
    #[error("native file dialog failed")]
    Dialog,
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value)
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl AppError {
    pub fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::NotAuthenticated,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::CONFLICT => Self::Conflict,
            _ => Self::RemoteStatus { status },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::AuthenticationFailed => Self::new(
                "authentication_failed",
                "邮箱或应用专用密码不正确，请检查后重试。",
            ),
            AppError::NotAuthenticated => Self::new("not_authenticated", "请先连接 Koofr 账户。"),
            AppError::InvalidInput(_) => {
                Self::new("invalid_input", "请求中包含无效的路径、名称或标识。")
            }
            AppError::Conflict | AppError::DuplicateTransfer => {
                Self::new("conflict", "目标已存在或操作正在进行。")
            }
            AppError::NotFound => Self::new("not_found", "指定的文件、文件夹或挂载点不存在。"),
            AppError::Forbidden => Self::new("forbidden", "当前账户没有执行此操作的权限。"),
            AppError::Cancelled => Self::new("cancelled", "传输已取消。"),
            AppError::IncompleteTransfer => {
                Self::new("incomplete_transfer", "传输未完整结束，请重试。")
            }
            AppError::RemoteStatus { .. } => {
                Self::new("remote_error", "Koofr 暂时无法完成此操作。")
            }
            AppError::Network(_) => {
                Self::new("network_error", "无法连接 Koofr，请检查网络后重试。")
            }
            AppError::Io(_) => Self::new("local_io_error", "无法读取或写入所选的本地文件。"),
            AppError::Decode(_) => Self::new("invalid_response", "Koofr 返回了无法识别的数据。"),
            AppError::Initialization => Self::new("initialization_error", "后端初始化失败。"),
            AppError::Dialog => Self::new("dialog_error", "无法打开本地文件选择窗口。"),
        }
    }
}

impl CommandError {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, CommandError};

    #[test]
    fn command_errors_do_not_expose_diagnostics() {
        let diagnostic = "C:\\Users\\private\\secret.txt";
        let error = std::io::Error::other(diagnostic);
        let command = CommandError::from(AppError::Io(error));
        let serialized = serde_json::to_string(&command).expect("serialize command error");

        assert!(!serialized.contains(diagnostic));
        assert_eq!(command.code, "local_io_error");
    }
}
