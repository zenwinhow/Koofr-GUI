use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::{AppState, error::AppError, koofr_api::FileInfo};

pub const TRANSFER_EVENT: &str = "koofr://transfer-progress";

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub file: Option<FileInfo>,
}

pub fn emit_progress(
    app: &AppHandle,
    transfer_id: &str,
    direction: TransferDirection,
    state: TransferState,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
) {
    let _ = app.emit(
        TRANSFER_EVENT,
        TransferProgress {
            transfer_id: transfer_id.to_owned(),
            direction,
            state,
            bytes_transferred,
            total_bytes,
        },
    );
}

pub fn emit_terminal(
    app: &AppHandle,
    transfer_id: &str,
    direction: TransferDirection,
    bytes_transferred: u64,
    result: &Result<TransferResult, AppError>,
) {
    let (state, bytes) = match result {
        Ok(result) => (TransferState::Completed, result.bytes_transferred),
        Err(AppError::TransferPaused) => (TransferState::Paused, bytes_transferred),
        Err(AppError::Cancelled) => (TransferState::Cancelled, bytes_transferred),
        Err(_) => (TransferState::Failed, bytes_transferred),
    };
    let mut fields = Map::new();
    fields.insert(
        "direction".to_owned(),
        Value::String(
            match direction {
                TransferDirection::Upload => "upload",
                TransferDirection::Download => "download",
            }
            .to_owned(),
        ),
    );
    fields.insert("bytesTransferred".to_owned(), Value::from(bytes));
    let logger = &app.state::<AppState>().logger;
    match result {
        Ok(_) => logger.info("transfer", "transfer_completed", Some(transfer_id), fields),
        Err(AppError::TransferPaused) => {
            logger.info("transfer", "transfer_paused", Some(transfer_id), fields)
        }
        Err(AppError::Cancelled) => {
            logger.info("transfer", "transfer_cancelled", Some(transfer_id), fields)
        }
        Err(error) => logger.error(
            "transfer",
            "transfer_failed",
            Some(transfer_id),
            error,
            fields,
        ),
    }
    emit_progress(app, transfer_id, direction, state, bytes, None);
}

pub fn normalize_interruption<T>(
    result: Result<T, AppError>,
    pause_requested: bool,
) -> Result<T, AppError> {
    match result {
        Err(AppError::Cancelled) if pause_requested => Err(AppError::TransferPaused),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_interruption;
    use crate::error::AppError;

    #[test]
    fn preserves_real_failures_instead_of_reporting_an_automatic_pause() {
        let result = normalize_interruption::<()>(
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "sensitive path omitted",
            ))),
            false,
        );

        assert!(matches!(result, Err(AppError::Io(_))));
    }

    #[test]
    fn only_an_explicit_pause_converts_cancellation_to_paused() {
        assert!(matches!(
            normalize_interruption::<()>(Err(AppError::Cancelled), true),
            Err(AppError::TransferPaused)
        ));
        assert!(matches!(
            normalize_interruption::<()>(Err(AppError::Cancelled), false),
            Err(AppError::Cancelled)
        ));
    }
}
