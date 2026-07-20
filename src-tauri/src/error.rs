use std::io;

use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("authentication is required")]
    NotAuthenticated,
    #[error("the authenticated account identity is unavailable")]
    AccountIdentityUnavailable,
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
    #[error("the interrupted transfer can be resumed or retried")]
    TransferPaused,
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
    #[error("local application data operation failed")]
    LocalData,
    #[error("secure credential store operation failed")]
    CredentialStore,
    #[error("the downloaded file could not be opened")]
    LocalOpen,
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

    pub const fn log_code(&self) -> &'static str {
        match self {
            Self::AuthenticationFailed => "authentication_failed",
            Self::NotAuthenticated => "not_authenticated",
            Self::AccountIdentityUnavailable => "account_identity_unavailable",
            Self::InvalidInput(_) => "invalid_input",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::Forbidden => "forbidden",
            Self::Cancelled => "cancelled",
            Self::TransferPaused => "transfer_paused",
            Self::IncompleteTransfer => "incomplete_transfer",
            Self::DuplicateTransfer => "duplicate_transfer",
            Self::RemoteStatus { .. } => "remote_error",
            Self::Network(_) => "network_error",
            Self::Io(_) => "local_io_error",
            Self::Decode(_) => "invalid_response",
            Self::Initialization => "initialization_error",
            Self::Dialog => "dialog_error",
            Self::LocalData => "local_data_error",
            Self::CredentialStore => "credential_store_error",
            Self::LocalOpen => "local_open_error",
        }
    }

    pub fn safe_log_fields(&self) -> Map<String, Value> {
        let mut fields = Map::new();
        match self {
            Self::RemoteStatus { status } => {
                fields.insert("httpStatus".to_owned(), Value::from(status.as_u16()));
            }
            Self::Network(error) => {
                fields.insert("isTimeout".to_owned(), Value::Bool(error.is_timeout()));
                fields.insert("isConnect".to_owned(), Value::Bool(error.is_connect()));
                fields.insert("isRequest".to_owned(), Value::Bool(error.is_request()));
                fields.insert("isBody".to_owned(), Value::Bool(error.is_body()));
            }
            Self::Io(error) => {
                fields.insert(
                    "ioKind".to_owned(),
                    Value::String(io_kind_code(error.kind()).to_owned()),
                );
                if let Some(code) = error.raw_os_error() {
                    fields.insert("osErrorCode".to_owned(), Value::from(code));
                }
            }
            Self::Decode(error) => {
                fields.insert(
                    "jsonCategory".to_owned(),
                    Value::String(format!("{:?}", error.classify()).to_ascii_lowercase()),
                );
                fields.insert("jsonLine".to_owned(), Value::from(error.line() as u64));
                fields.insert("jsonColumn".to_owned(), Value::from(error.column() as u64));
            }
            _ => {}
        }
        fields
    }
}

fn io_kind_code(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::AlreadyExists => "already_exists",
        std::io::ErrorKind::InvalidInput => "invalid_input",
        std::io::ErrorKind::InvalidData => "invalid_data",
        std::io::ErrorKind::TimedOut => "timed_out",
        std::io::ErrorKind::WriteZero => "write_zero",
        std::io::ErrorKind::UnexpectedEof => "unexpected_eof",
        std::io::ErrorKind::OutOfMemory => "out_of_memory",
        std::io::ErrorKind::StorageFull => "storage_full",
        std::io::ErrorKind::FileTooLarge => "file_too_large",
        std::io::ErrorKind::ReadOnlyFilesystem => "read_only_filesystem",
        _ => "other",
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::AuthenticationFailed => Self::new(
                "authentication_failed",
                "邮箱或应用专用密码不正确，请检查后重试。",
            ),
            AppError::NotAuthenticated => Self::new("not_authenticated", "请先连接 Koofr 账户。"),
            AppError::AccountIdentityUnavailable => Self::new(
                "account_identity_unavailable",
                "无法确认当前 Koofr 账户，请重新登录后重试。",
            ),
            AppError::InvalidInput(_) => {
                Self::new("invalid_input", "请求中包含无效的路径、名称或标识。")
            }
            AppError::Conflict | AppError::DuplicateTransfer => {
                Self::new("conflict", "目标已存在或操作正在进行。")
            }
            AppError::NotFound => Self::new("not_found", "指定的文件、文件夹或挂载点不存在。"),
            AppError::Forbidden => Self::new("forbidden", "当前账户没有执行此操作的权限。"),
            AppError::Cancelled => Self::new("cancelled", "传输已取消。"),
            AppError::TransferPaused => {
                Self::new("transfer_paused", "传输已暂停，可从传输面板继续。")
            }
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
            AppError::Decode(error) => {
                Self::new("invalid_response", "Koofr 返回了无法识别的数据。")
                    .with_diagnostic(safe_decode_diagnostic(&error))
            }
            AppError::Initialization => Self::new("initialization_error", "后端初始化失败。"),
            AppError::Dialog => Self::new("dialog_error", "无法打开本地文件选择窗口。"),
            AppError::LocalData => Self::new("local_data_error", "无法读取或保存本地应用数据。"),
            AppError::CredentialStore => Self::new(
                "credential_store_error",
                "无法访问 Windows 凭据管理器，请检查系统设置后重试。",
            ),
            AppError::LocalOpen => Self::new(
                "local_open_error",
                "无法打开下载内容，它可能已被移动、删除或没有关联的应用。",
            ),
        }
    }
}

impl CommandError {
    const fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            code,
            message,
            diagnostic: None,
        }
    }

    fn with_diagnostic(mut self, diagnostic: String) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }
}

fn safe_decode_diagnostic(error: &serde_json::Error) -> String {
    let message = error.to_string();
    let reason = extract_safe_field(&message, "missing field `")
        .map(|field| format!("缺少字段 `{field}`"))
        .or_else(|| {
            extract_safe_field(&message, "unknown field `")
                .map(|field| format!("包含未知字段 `{field}`"))
        })
        .or_else(|| type_mismatch_reason(&message))
        .unwrap_or_else(|| match error.classify() {
            serde_json::error::Category::Io => "读取 JSON 响应失败".to_owned(),
            serde_json::error::Category::Syntax => "JSON 语法无效".to_owned(),
            serde_json::error::Category::Data => "JSON 字段结构不匹配".to_owned(),
            serde_json::error::Category::Eof => "JSON 响应不完整".to_owned(),
        });

    format!(
        "{reason}（第 {} 行，第 {} 列）",
        error.line(),
        error.column()
    )
}

fn extract_safe_field(message: &str, prefix: &str) -> Option<String> {
    let value = message.split_once(prefix)?.1.split('`').next()?;
    (!value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'))
    .then(|| value.to_owned())
}

fn type_mismatch_reason(message: &str) -> Option<String> {
    let invalid = message.strip_prefix("invalid type: ")?;
    let received = [
        "null",
        "boolean",
        "integer",
        "floating point",
        "string",
        "map",
        "sequence",
        "unit",
        "byte array",
        "character",
    ]
    .into_iter()
    .find(|kind| invalid.starts_with(kind))
    .unwrap_or("未知类型");
    let expected = invalid
        .split_once(", expected ")
        .map(|(_, expected)| expected.split(" at line ").next().unwrap_or(expected))
        .filter(|expected| {
            !expected.is_empty()
                && expected.len() <= 96
                && expected.chars().all(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, ' ' | '_' | '-' | '[' | ']' | '(' | ')')
                })
        });

    Some(match expected {
        Some(expected) => format!("字段类型不匹配：收到 {received}，预期 {expected}"),
        None => format!("字段类型不匹配：收到 {received}"),
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{AppError, CommandError};

    #[test]
    fn command_errors_do_not_expose_diagnostics() {
        let diagnostic = "C:\\Users\\private\\secret.txt";
        let error = std::io::Error::other(diagnostic);
        let command = CommandError::from(AppError::Io(error));
        let serialized = serde_json::to_string(&command).expect("serialize command error");

        assert!(!serialized.contains(diagnostic));
        assert_eq!(command.code, "local_io_error");
        assert!(command.diagnostic.is_none());
    }

    #[test]
    fn decode_errors_report_missing_fields_without_response_values() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct RequiredId {
            id: String,
        }

        let private_value = "private-file-name.txt";
        let error = serde_json::from_str::<RequiredId>(&format!(r#"{{"name":"{private_value}"}}"#))
            .expect_err("missing id should fail");
        let command = CommandError::from(AppError::Decode(error));
        let diagnostic = command.diagnostic.expect("decode diagnostic");

        assert!(diagnostic.contains("缺少字段 `id`"));
        assert!(!diagnostic.contains(private_value));
    }

    #[test]
    fn decode_errors_redact_invalid_string_values() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct RequiredSize {
            size: i64,
        }

        let private_value = r"C:\Users\private\secret.txt";
        let payload = serde_json::json!({ "size": private_value });
        let error =
            serde_json::from_value::<RequiredSize>(payload).expect_err("string size should fail");
        let command = CommandError::from(AppError::Decode(error));
        let diagnostic = command.diagnostic.expect("decode diagnostic");

        assert!(diagnostic.contains("字段类型不匹配：收到 string"));
        assert!(!diagnostic.contains(private_value));
    }
}
