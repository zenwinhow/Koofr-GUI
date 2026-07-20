use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

use crate::{AppState, error::AppError, koofr_api::FileInfo, settings::NetworkRetrySettings};

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
    Retrying,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkRetryPolicy {
    enabled: bool,
    max_retries: Option<u32>,
    interval_seconds: u32,
}

impl NetworkRetryPolicy {
    pub const fn new(enabled: bool, max_retries: Option<u32>, interval_seconds: u32) -> Self {
        Self {
            enabled,
            max_retries,
            interval_seconds,
        }
    }

    pub const fn enabled(self) -> bool {
        self.enabled
    }

    pub const fn interval_seconds(self) -> u32 {
        self.interval_seconds
    }
}

impl Default for NetworkRetryPolicy {
    fn default() -> Self {
        Self::new(false, Some(8), 5)
    }
}

impl From<NetworkRetrySettings> for NetworkRetryPolicy {
    fn from(settings: NetworkRetrySettings) -> Self {
        Self::new(
            settings.enabled,
            settings.max_retries,
            settings.interval_seconds,
        )
    }
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

pub fn should_retry_network<T>(
    result: &Result<T, AppError>,
    policy: NetworkRetryPolicy,
    retries_completed: u32,
) -> bool {
    let within_limit = match policy.max_retries {
        Some(limit) => retries_completed < limit,
        None => true,
    };
    policy.enabled() && within_limit && matches!(result, Err(AppError::Network(_)))
}

pub async fn wait_for_network_retry(
    app: &AppHandle,
    cancel: &CancellationToken,
    transfer_id: &str,
    direction: TransferDirection,
    retry_attempt: u32,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
    policy: NetworkRetryPolicy,
) -> Result<(), AppError> {
    let delay_seconds = policy.interval_seconds();
    let mut fields = Map::new();
    fields.insert("retryAttempt".to_owned(), Value::from(retry_attempt));
    fields.insert("delaySeconds".to_owned(), Value::from(delay_seconds));
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
    app.state::<AppState>().logger.warn(
        "transfer",
        "network_retry_scheduled",
        Some(transfer_id),
        fields,
    );
    emit_progress(
        app,
        transfer_id,
        direction,
        TransferState::Retrying,
        bytes_transferred,
        total_bytes,
    );
    tokio::select! {
        () = tokio::time::sleep(std::time::Duration::from_secs(u64::from(delay_seconds))) => Ok(()),
        () = cancel.cancelled() => Err(AppError::Cancelled),
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkRetryPolicy, normalize_interruption, should_retry_network};
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

    #[test]
    fn network_retry_policy_supports_finite_and_unlimited_retries() {
        assert!(!NetworkRetryPolicy::default().enabled());
        assert!(NetworkRetryPolicy::new(true, None, 30).enabled());
    }

    #[tokio::test]
    async fn retries_only_network_errors_and_stops_at_the_limit() {
        let network_error = reqwest::Client::new()
            .get("http://127.0.0.1:0")
            .send()
            .await
            .expect_err("closed local endpoint should fail");
        let result = Err::<(), _>(AppError::Network(network_error));
        let enabled = NetworkRetryPolicy::new(true, Some(8), 5);

        assert!(should_retry_network(&result, enabled, 0));
        assert!(!should_retry_network(&result, enabled, 8));
        assert!(!should_retry_network(
            &result,
            NetworkRetryPolicy::default(),
            0
        ));
        assert!(!should_retry_network(
            &Err::<(), _>(AppError::Forbidden),
            enabled,
            0
        ));
        assert!(should_retry_network(
            &result,
            NetworkRetryPolicy::new(true, None, 5),
            u32::MAX
        ));
    }
}
