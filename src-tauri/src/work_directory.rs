use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const LOCATOR_VERSION: u8 = 1;
const LOCATOR_FILE_NAME: &str = "work-directory.json";
const MIGRATION_DIRECTORY_PREFIX: &str = ".koofr-gui-migration-";
const MIGRATION_MARKER_PREFIX: &str = ".koofr-work-migration-";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingChange {
    source_directory: PathBuf,
    target_directory: PathBuf,
    move_existing: bool,
    migration_id: String,
    #[serde(default)]
    activated: bool,
    #[serde(default)]
    failed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredLocator {
    version: u8,
    current_directory: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending: Option<PendingChange>,
}

#[derive(Clone, Debug)]
pub struct WorkDirectoryStatus {
    pub current_directory: PathBuf,
    pub pending_directory: Option<PathBuf>,
    pub move_existing: bool,
    pub migration_failed: bool,
}

#[derive(Clone)]
pub struct WorkDirectoryStore {
    locator_path: PathBuf,
    state: Arc<Mutex<StoredLocator>>,
}

impl WorkDirectoryStore {
    pub fn initialize(default_directory: PathBuf) -> Result<Self, AppError> {
        fs::create_dir_all(&default_directory).map_err(|_| AppError::LocalData)?;
        validate_directory(&default_directory)?;
        let locator_path = locator_path_for(&default_directory)?;
        let mut locator = load_locator(&locator_path).unwrap_or_else(|| StoredLocator {
            version: LOCATOR_VERSION,
            current_directory: default_directory.clone(),
            pending: None,
        });
        if locator.version != LOCATOR_VERSION
            || validate_directory(&locator.current_directory).is_err()
        {
            locator = StoredLocator {
                version: LOCATOR_VERSION,
                current_directory: default_directory.clone(),
                pending: None,
            };
        }
        apply_pending_change(&locator_path, &mut locator)?;
        if locator.current_directory != default_directory {
            fs::create_dir_all(&default_directory).map_err(|_| AppError::LocalData)?;
            validate_directory(&default_directory)?;
        }
        persist_locator(&locator_path, &locator)?;
        Ok(Self {
            locator_path,
            state: Arc::new(Mutex::new(locator)),
        })
    }

    pub fn current_directory(&self) -> PathBuf {
        self.state().current_directory.clone()
    }

    pub fn status(&self) -> WorkDirectoryStatus {
        let state = self.state();
        WorkDirectoryStatus {
            current_directory: state.current_directory.clone(),
            pending_directory: state
                .pending
                .as_ref()
                .map(|pending| pending.target_directory.clone()),
            move_existing: state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.move_existing),
            migration_failed: state.pending.as_ref().is_some_and(|pending| pending.failed),
        }
    }

    pub fn schedule_change(
        &self,
        target_directory: PathBuf,
        move_existing: bool,
    ) -> Result<(), AppError> {
        validate_directory(&target_directory)?;
        if target_directory.parent().is_none() {
            return Err(AppError::InvalidInput("work directory"));
        }
        let mut state = self.state();
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.activated)
        {
            return Err(AppError::Conflict);
        }
        validate_distinct_non_nested_directories(&state.current_directory, &target_directory)?;
        if !directory_is_empty(&target_directory)? {
            return Err(AppError::Conflict);
        }
        state.pending = Some(PendingChange {
            source_directory: state.current_directory.clone(),
            target_directory,
            move_existing,
            migration_id: uuid::Uuid::new_v4().to_string(),
            activated: false,
            failed: false,
        });
        persist_locator(&self.locator_path, &state)
    }

    fn state(&self) -> MutexGuard<'_, StoredLocator> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn load_locator(path: &Path) -> Option<StoredLocator> {
    fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())?;
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn persist_locator(path: &Path, locator: &StoredLocator) -> Result<(), AppError> {
    let payload = serde_json::to_vec_pretty(locator).map_err(|_| AppError::LocalData)?;
    let temporary = path.with_extension("json.tmp");
    reject_unsafe_file(path)?;
    reject_unsafe_file(&temporary)?;
    fs::write(&temporary, payload).map_err(|_| AppError::LocalData)?;
    if path.exists() {
        fs::remove_file(path).map_err(|_| AppError::LocalData)?;
    }
    fs::rename(temporary, path).map_err(|_| AppError::LocalData)
}

fn apply_pending_change(locator_path: &Path, locator: &mut StoredLocator) -> Result<(), AppError> {
    let Some(mut pending) = locator.pending.clone() else {
        return Ok(());
    };
    if validate_directory(&pending.target_directory).is_err() {
        pending.failed = true;
        locator.pending = Some(pending);
        return Ok(());
    }
    if !pending.move_existing {
        locator.current_directory = pending.target_directory;
        locator.pending = None;
        return Ok(());
    }

    let source = pending.source_directory.clone();
    let target = pending.target_directory.clone();
    let marker = target.join(format!("{MIGRATION_MARKER_PREFIX}{}", pending.migration_id));
    if pending.activated || marker.is_file() {
        pending.activated = true;
        locator.current_directory = target.clone();
        locator.pending = Some(pending.clone());
        persist_locator(locator_path, locator)?;
        let cleanup = if source.exists() {
            validate_directory(&source)
                .map_err(|_| ())
                .and_then(|()| remove_source_directory(&source))
        } else {
            Ok(())
        };
        match cleanup {
            Ok(()) => {
                let _ = fs::remove_file(marker);
                locator.pending = None;
            }
            Err(()) => {
                pending.failed = true;
                locator.pending = Some(pending);
            }
        }
        return Ok(());
    }

    if validate_directory(&source).is_err()
        || validate_distinct_non_nested_directories(&source, &target).is_err()
    {
        pending.failed = true;
        locator.pending = Some(pending);
        return Ok(());
    }
    if !directory_is_empty(&target)? {
        pending.failed = true;
        locator.pending = Some(pending);
        return Ok(());
    }
    let parent = target
        .parent()
        .ok_or(AppError::InvalidInput("work directory"))?;
    let stage = parent.join(format!(
        "{MIGRATION_DIRECTORY_PREFIX}{}",
        pending.migration_id
    ));
    if stage.exists() {
        validate_owned_stage_directory(&stage, parent, &pending.migration_id)?;
        fs::remove_dir_all(&stage).map_err(|_| AppError::LocalData)?;
    }
    fs::create_dir(&stage).map_err(|_| AppError::LocalData)?;
    let copy_result = copy_directory_contents(&source, &stage);
    if copy_result.is_err() {
        let _ = fs::remove_dir_all(&stage);
        pending.failed = true;
        locator.pending = Some(pending);
        return Ok(());
    }
    fs::write(
        stage.join(format!("{MIGRATION_MARKER_PREFIX}{}", pending.migration_id)),
        b"Koofr-GUI work directory migration",
    )
    .map_err(|_| AppError::LocalData)?;
    fs::remove_dir(&target).map_err(|_| AppError::LocalData)?;
    fs::rename(&stage, &target).map_err(|_| AppError::LocalData)?;

    pending.activated = true;
    pending.failed = false;
    locator.current_directory = target;
    locator.pending = Some(pending.clone());
    persist_locator(locator_path, locator)?;
    match remove_source_directory(&source) {
        Ok(()) => {
            let _ = fs::remove_file(marker);
            locator.pending = None;
        }
        Err(()) => {
            pending.failed = true;
            locator.pending = Some(pending);
        }
    }
    Ok(())
}

fn validate_directory(path: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::InvalidInput("work directory"));
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| AppError::InvalidInput("work directory"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::InvalidInput("work directory"));
    }
    Ok(())
}

fn validate_distinct_non_nested_directories(current: &Path, target: &Path) -> Result<(), AppError> {
    let current =
        fs::canonicalize(current).map_err(|_| AppError::InvalidInput("work directory"))?;
    let target = fs::canonicalize(target).map_err(|_| AppError::InvalidInput("work directory"))?;
    if current == target || current.starts_with(&target) || target.starts_with(&current) {
        return Err(AppError::InvalidInput("work directory"));
    }
    Ok(())
}

fn directory_is_empty(path: &Path) -> Result<bool, AppError> {
    let mut entries = fs::read_dir(path).map_err(|_| AppError::LocalData)?;
    Ok(entries.next().is_none())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<(), AppError> {
    for entry in fs::read_dir(source).map_err(|_| AppError::LocalData)? {
        let entry = entry.map_err(|_| AppError::LocalData)?;
        let source_path = entry.path();
        let file_type = entry.file_type().map_err(|_| AppError::LocalData)?;
        if file_type.is_symlink() {
            return Err(AppError::InvalidInput("work directory contents"));
        }
        let destination_path = destination.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir(&destination_path).map_err(|_| AppError::LocalData)?;
            copy_directory_contents(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|_| AppError::LocalData)?;
        } else {
            return Err(AppError::InvalidInput("work directory contents"));
        }
    }
    Ok(())
}

fn remove_source_directory(source: &Path) -> Result<(), ()> {
    let entries = fs::read_dir(source).map_err(|_| ())?;
    for entry in entries {
        let entry = entry.map_err(|_| ())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|_| ())?;
        if file_type.is_dir() && !file_type.is_symlink() {
            fs::remove_dir_all(path).map_err(|_| ())?;
        } else {
            fs::remove_file(path).map_err(|_| ())?;
        }
    }
    fs::remove_dir(source).map_err(|_| ())
}

fn locator_path_for(default_directory: &Path) -> Result<PathBuf, AppError> {
    let parent = default_directory.parent().ok_or(AppError::LocalData)?;
    let mut file_name = OsString::from(default_directory.file_name().ok_or(AppError::LocalData)?);
    file_name.push(format!(".{LOCATOR_FILE_NAME}"));
    Ok(parent.join(file_name))
}

fn validate_owned_stage_directory(
    stage: &Path,
    expected_parent: &Path,
    migration_id: &str,
) -> Result<(), AppError> {
    if stage.parent() != Some(expected_parent)
        || stage.file_name().and_then(|name| name.to_str())
            != Some(format!("{MIGRATION_DIRECTORY_PREFIX}{migration_id}").as_str())
    {
        return Err(AppError::LocalData);
    }
    let metadata = fs::symlink_metadata(stage).map_err(|_| AppError::LocalData)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::LocalData);
    }
    Ok(())
}

fn reject_unsafe_file(path: &Path) -> Result<(), AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(AppError::LocalData),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(AppError::LocalData),
    }
}

#[cfg(test)]
mod tests {
    use super::WorkDirectoryStore;

    fn directory(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "koofr-work-directory-{name}-{}",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn switches_to_an_empty_directory_without_moving_existing_files() {
        let default = directory("default");
        let target = directory("target");
        std::fs::create_dir_all(&default).expect("create default");
        std::fs::create_dir_all(&target).expect("create target");
        std::fs::write(default.join("settings.json"), b"settings").expect("write settings");

        let store = WorkDirectoryStore::initialize(default.clone()).expect("initialize");
        store
            .schedule_change(target.clone(), false)
            .expect("schedule change");
        let reloaded = WorkDirectoryStore::initialize(default.clone()).expect("reload");

        assert_eq!(reloaded.current_directory(), target);
        assert!(default.join("settings.json").is_file());
        std::fs::remove_dir_all(&default).expect("remove default");
        std::fs::remove_dir_all(target).expect("remove target");
        let locator = super::locator_path_for(&default).expect("locator path");
        std::fs::remove_file(locator).expect("remove locator");
    }

    #[test]
    fn moves_the_complete_work_directory_on_the_next_start() {
        let default = directory("move-default");
        let target = directory("move-target");
        std::fs::create_dir_all(default.join("cache")).expect("create cache");
        std::fs::create_dir_all(default.join("logs")).expect("create logs");
        std::fs::create_dir_all(default.join("future-data").join("nested"))
            .expect("create unknown nested data");
        std::fs::create_dir_all(&target).expect("create target");
        std::fs::write(default.join("settings.json"), b"settings").expect("write settings");
        std::fs::write(
            default.join("transfer-checkpoints.json"),
            b"transfer checkpoints",
        )
        .expect("write checkpoints");
        std::fs::write(default.join("download-history.json"), b"download history")
            .expect("write history");
        std::fs::write(default.join("cache").join("metadata-cache.json"), b"cache")
            .expect("write cache");
        std::fs::write(default.join("logs").join("koofr-gui.jsonl"), b"log").expect("write log");
        std::fs::write(
            default
                .join("future-data")
                .join("nested")
                .join("unknown.bin"),
            b"unknown future data",
        )
        .expect("write unknown data");

        let store = WorkDirectoryStore::initialize(default.clone()).expect("initialize");
        store
            .schedule_change(target.clone(), true)
            .expect("schedule migration");
        let reloaded = WorkDirectoryStore::initialize(default.clone()).expect("reload");

        assert_eq!(reloaded.current_directory(), target);
        assert!(target.join("settings.json").is_file());
        assert!(target.join("transfer-checkpoints.json").is_file());
        assert!(target.join("download-history.json").is_file());
        assert!(target.join("cache").join("metadata-cache.json").is_file());
        assert!(target.join("logs").join("koofr-gui.jsonl").is_file());
        assert!(
            target
                .join("future-data")
                .join("nested")
                .join("unknown.bin")
                .is_file()
        );
        assert!(!default.join("settings.json").exists());
        assert!(
            std::fs::read_dir(&default)
                .expect("read empty default")
                .next()
                .is_none()
        );
        let locator = super::locator_path_for(&default).expect("locator path");
        std::fs::remove_dir(&default).expect("remove default");
        std::fs::remove_file(locator).expect("remove locator");
        std::fs::remove_dir_all(target).expect("remove target");
    }

    #[test]
    fn can_move_a_custom_work_directory_back_to_the_default_location() {
        let default = directory("round-trip-default");
        let custom = directory("round-trip-custom");
        std::fs::create_dir_all(&default).expect("create default");
        std::fs::create_dir_all(&custom).expect("create custom");
        std::fs::write(default.join("settings.json"), b"settings").expect("write settings");

        let store = WorkDirectoryStore::initialize(default.clone()).expect("initialize");
        store
            .schedule_change(custom.clone(), true)
            .expect("schedule custom directory");
        let custom_store =
            WorkDirectoryStore::initialize(default.clone()).expect("activate custom directory");
        custom_store
            .schedule_change(default.clone(), true)
            .expect("schedule default directory");
        let default_store =
            WorkDirectoryStore::initialize(default.clone()).expect("restore default directory");

        assert_eq!(default_store.current_directory(), default);
        assert!(default.join("settings.json").is_file());
        assert!(!custom.exists());
        let locator = super::locator_path_for(&default).expect("locator path");
        std::fs::remove_dir_all(default).expect("remove default");
        std::fs::remove_file(locator).expect("remove locator");
    }

    #[test]
    fn rejects_non_empty_and_nested_targets() {
        let default = directory("reject-default");
        let non_empty = directory("reject-target");
        let nested = default.join("nested");
        std::fs::create_dir_all(&default).expect("create default");
        std::fs::create_dir_all(&non_empty).expect("create target");
        std::fs::create_dir_all(&nested).expect("create nested");
        std::fs::write(non_empty.join("existing.txt"), b"existing").expect("write existing");
        let store = WorkDirectoryStore::initialize(default.clone()).expect("initialize");

        assert!(store.schedule_change(non_empty.clone(), true).is_err());
        assert!(store.schedule_change(nested, true).is_err());
        std::fs::remove_dir_all(&default).expect("remove default");
        std::fs::remove_dir_all(non_empty).expect("remove target");
        let locator = super::locator_path_for(&default).expect("locator path");
        std::fs::remove_file(locator).expect("remove locator");
    }
}
