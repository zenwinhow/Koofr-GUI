use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{error::AppError, koofr_api::FileInfo};

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
    emit_progress(app, transfer_id, direction, state, bytes, None);
}
