use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{error::AppError, settings::LogLevel};

const ACTIVE_LOG_FILE: &str = "koofr-gui.jsonl";
const LOG_FILE_PREFIX: &str = "koofr-gui";
const LOG_FILE_SUFFIX: &str = ".jsonl";

#[derive(Clone, Debug)]
pub struct LogConfig {
    pub directory: PathBuf,
    pub level: LogLevel,
    pub retention_days: u32,
    pub max_file_bytes: u64,
}

#[derive(Clone)]
pub struct AppLogger {
    sender: mpsc::Sender<WriterCommand>,
    config: Arc<RwLock<LogConfig>>,
    session_id: String,
}

enum WriterCommand {
    Record(LogRecord),
    Configure(LogConfig),
    Clear(mpsc::Sender<Result<(), ()>>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogRecord {
    timestamp_ms: u128,
    level: LogLevel,
    target: &'static str,
    event: &'static str,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_id: Option<String>,
    #[serde(skip_serializing_if = "Map::is_empty")]
    fields: Map<String, Value>,
}

impl AppLogger {
    pub fn initialize(config: LogConfig) -> Result<Self, AppError> {
        validate_directory(&config.directory)?;
        let (sender, receiver) = mpsc::channel();
        let shared_config = Arc::new(RwLock::new(config.clone()));
        std::thread::Builder::new()
            .name("koofr-log-writer".to_owned())
            .spawn(move || writer_loop(receiver, config))
            .map_err(|_| AppError::Initialization)?;
        let logger = Self {
            sender,
            config: shared_config,
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        logger.info("application", "session_started", None, Map::new());
        Ok(logger)
    }

    pub fn info(
        &self,
        target: &'static str,
        event: &'static str,
        transfer_id: Option<&str>,
        fields: Map<String, Value>,
    ) {
        self.record(LogLevel::Info, target, event, transfer_id, fields);
    }

    pub fn error(
        &self,
        target: &'static str,
        event: &'static str,
        transfer_id: Option<&str>,
        error: &AppError,
        mut fields: Map<String, Value>,
    ) {
        fields.insert(
            "errorCode".to_owned(),
            Value::String(error.log_code().to_owned()),
        );
        fields.extend(error.safe_log_fields());
        self.record(LogLevel::Error, target, event, transfer_id, fields);
    }

    fn record(
        &self,
        level: LogLevel,
        target: &'static str,
        event: &'static str,
        transfer_id: Option<&str>,
        fields: Map<String, Value>,
    ) {
        let configured_level = self
            .config
            .read()
            .map(|config| config.level)
            .unwrap_or(LogLevel::Error);
        if level > configured_level {
            return;
        }
        let _ = self.sender.send(WriterCommand::Record(LogRecord {
            timestamp_ms: now_ms(),
            level,
            target,
            event,
            session_id: self.session_id.clone(),
            transfer_id: transfer_id.map(str::to_owned),
            fields,
        }));
    }

    pub async fn configure(&self, config: LogConfig) -> Result<(), AppError> {
        validate_directory_async(&config.directory).await?;
        self.sender
            .send(WriterCommand::Configure(config.clone()))
            .map_err(|_| AppError::LocalData)?;
        *self.config.write().map_err(|_| AppError::LocalData)? = config;
        Ok(())
    }

    pub async fn clear(&self) -> Result<(), AppError> {
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(WriterCommand::Clear(sender))
            .map_err(|_| AppError::LocalData)?;
        tokio::task::spawn_blocking(move || receiver.recv())
            .await
            .map_err(|_| AppError::LocalData)?
            .map_err(|_| AppError::LocalData)?
            .map_err(|_| AppError::LocalData)
    }

    pub async fn stats(&self) -> (usize, u64) {
        let directory = self
            .config
            .read()
            .map(|config| config.directory.clone())
            .unwrap_or_default();
        let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
            return (0, 0);
        };
        let mut files = 0_usize;
        let mut bytes = 0_u64;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if is_log_file(&entry.path())
                && let Ok(metadata) = entry.metadata().await
                && metadata.is_file()
            {
                files += 1;
                bytes = bytes.saturating_add(metadata.len());
            }
        }
        (files, bytes)
    }
}

fn writer_loop(receiver: mpsc::Receiver<WriterCommand>, mut config: LogConfig) {
    let _ = cleanup_expired(&config);
    for command in receiver {
        match command {
            WriterCommand::Record(record) => {
                if record.level <= config.level {
                    let _ = append_record(&config, &record);
                }
            }
            WriterCommand::Configure(next) => {
                config = next;
                let _ = cleanup_expired(&config);
            }
            WriterCommand::Clear(reply) => {
                let result = clear_log_files(&config.directory).map_err(|_| ());
                let _ = reply.send(result);
            }
        }
    }
}

fn append_record(config: &LogConfig, record: &LogRecord) -> Result<(), AppError> {
    fs::create_dir_all(&config.directory).map_err(|_| AppError::LocalData)?;
    let active = config.directory.join(ACTIVE_LOG_FILE);
    if fs::symlink_metadata(&active)
        .is_ok_and(|metadata| !metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(AppError::LocalData);
    }
    let line = serde_json::to_vec(record).map_err(|_| AppError::LocalData)?;
    let current_size = fs::metadata(&active)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current_size > 0
        && current_size.saturating_add(line.len() as u64 + 1) > config.max_file_bytes
    {
        let rotated = config.directory.join(format!(
            "{LOG_FILE_PREFIX}-{}-{}.jsonl",
            now_ms(),
            uuid::Uuid::new_v4()
        ));
        fs::rename(&active, rotated).map_err(|_| AppError::LocalData)?;
        cleanup_expired(config)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(active)
        .map_err(|_| AppError::LocalData)?;
    file.write_all(&line).map_err(|_| AppError::LocalData)?;
    file.write_all(b"\n").map_err(|_| AppError::LocalData)
}

fn cleanup_expired(config: &LogConfig) -> Result<(), AppError> {
    let retention_ms = u128::from(config.retention_days) * 24 * 60 * 60 * 1_000;
    let cutoff = now_ms().saturating_sub(retention_ms);
    for entry in fs::read_dir(&config.directory).map_err(|_| AppError::LocalData)? {
        let entry = entry.map_err(|_| AppError::LocalData)?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(ACTIVE_LOG_FILE)
            || !is_log_file(&path)
        {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis())
            .unwrap_or(now_ms());
        if modified < cutoff {
            fs::remove_file(path).map_err(|_| AppError::LocalData)?;
        }
    }
    Ok(())
}

fn clear_log_files(directory: &Path) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if is_log_file(&path) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn is_log_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(LOG_FILE_PREFIX) && name.ends_with(LOG_FILE_SUFFIX))
}

fn validate_directory(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::InvalidInput("log directory"));
    }
    fs::create_dir_all(path).map_err(|_| AppError::LocalData)?;
    let metadata = fs::symlink_metadata(path).map_err(AppError::from)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::InvalidInput("log directory"));
    }
    Ok(())
}

async fn validate_directory_async(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::InvalidInput("log directory"));
    }
    let metadata = tokio::fs::symlink_metadata(path).await?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::InvalidInput("log directory"));
    }
    Ok(())
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, time::Duration};

    use serde_json::Map;

    use super::{AppLogger, LogConfig};
    use crate::{error::AppError, settings::LogLevel};

    fn config(directory: std::path::PathBuf) -> LogConfig {
        LogConfig {
            directory,
            level: LogLevel::Info,
            retention_days: 7,
            max_file_bytes: 300,
        }
    }

    #[tokio::test]
    async fn writes_structured_redacted_errors_and_rotates_files() {
        let directory = std::env::temp_dir().join(format!("koofr-logs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("create log directory");
        let logger = AppLogger::initialize(config(directory.clone())).expect("initialize logger");
        for _ in 0..8 {
            logger.error(
                "transfer",
                "transfer_failed",
                Some("8ed2fd86-b6d8-42af-8854-f5b69ef621d5"),
                &AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    r"C:\Users\private\secret.txt",
                )),
                Map::new(),
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        let files = std::fs::read_dir(&directory)
            .expect("read logs")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert!(files.len() > 1);
        let mut payload = String::new();
        for file in files {
            payload.push_str(&std::fs::read_to_string(file).expect("read log"));
        }
        assert!(payload.contains("local_io_error"));
        assert!(payload.contains("permission_denied"));
        assert!(!payload.contains("private"));
        logger.clear().await.expect("clear logs");
        let remaining = std::fs::read_dir(&directory)
            .expect("read cleared directory")
            .map(|entry| entry.expect("entry").file_name())
            .collect::<BTreeSet<_>>();
        assert!(remaining.is_empty());
        std::fs::remove_dir_all(directory).expect("remove log directory");
    }
}
